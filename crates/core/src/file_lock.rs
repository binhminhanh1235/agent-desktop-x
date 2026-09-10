use std::fs::File;
use std::path::Path;
use std::time::Duration;

use crate::{AdapterError, Deadline, ErrorCode};

/// A private state-file lock held until this guard is dropped.
///
/// Releases by closing rather than unlocking: a duplicated descriptor shares
/// the lock, so unlocking would release it for the other holder too.
pub struct FileLock {
    _file: File,
    #[cfg(unix)]
    contention_count: u64,
}

impl FileLock {
    pub fn acquire(path: &Path, deadline: Deadline, purpose: &str) -> Result<Self, AdapterError> {
        let file = crate::private_file::open_private_lock(path, true).map_err(io_error)?;
        lock_file(file, deadline, purpose, path)
    }

    pub(crate) fn is_held(path: &Path) -> bool {
        let Ok(file) = crate::private_file::open_private_lock(path, false) else {
            return false;
        };
        match file.try_lock() {
            Ok(()) => {
                let _ = file.unlock();
                false
            }
            Err(std::fs::TryLockError::WouldBlock) => true,
            Err(std::fs::TryLockError::Error(_)) => true,
        }
    }

    #[cfg(unix)]
    pub(crate) fn contention_count(&self) -> u64 {
        self.contention_count
    }

    #[cfg(unix)]
    pub(crate) fn duplicate_inheritable(&self) -> Result<std::os::fd::OwnedFd, AdapterError> {
        use std::os::fd::{AsRawFd, FromRawFd};

        let duplicated = unsafe { libc::fcntl(self._file.as_raw_fd(), libc::F_DUPFD, 3) };
        if duplicated < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(duplicated) })
    }

    #[cfg(unix)]
    pub(crate) fn adopt_inherited(
        raw_fd: std::os::fd::RawFd,
        canonical_path: &Path,
        deadline: Deadline,
        purpose: &str,
    ) -> Result<Self, AdapterError> {
        use std::os::fd::{FromRawFd, OwnedFd};
        use std::os::unix::fs::MetadataExt;

        let duplicated = unsafe { libc::fcntl(raw_fd, libc::F_DUPFD_CLOEXEC, 3) };
        if duplicated < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        let original_flags = unsafe { libc::fcntl(raw_fd, libc::F_GETFD) };
        if original_flags < 0
            || unsafe { libc::fcntl(raw_fd, libc::F_SETFD, original_flags | libc::FD_CLOEXEC) } != 0
        {
            unsafe { libc::close(duplicated) };
            return Err(io_error(std::io::Error::last_os_error()));
        }
        let inherited = File::from(unsafe { OwnedFd::from_raw_fd(duplicated) });
        let inherited_meta =
            crate::private_file::validate_private_regular(&inherited).map_err(io_error)?;
        let canonical =
            crate::private_file::open_private_lock(canonical_path, false).map_err(io_error)?;
        let canonical_meta = canonical.metadata().map_err(io_error)?;
        if inherited_meta.dev() != canonical_meta.dev()
            || inherited_meta.ino() != canonical_meta.ino()
        {
            return Err(AdapterError::new(
                ErrorCode::PolicyDenied,
                "Inherited interaction lease does not identify the canonical lock file",
            ));
        }
        lock_file(inherited, deadline, purpose, canonical_path)
    }
}

/// Drops a contended lock without unlocking: shared file descriptions need drop,
/// not unlock. On the adopt path, shared descriptions exist and must be left alone.
fn lock_file(
    file: File,
    deadline: Deadline,
    purpose: &str,
    path: &Path,
) -> Result<FileLock, AdapterError> {
    // Opening and validating a private lock file is part of the caller's budget,
    // but a slow filesystem must not turn an otherwise-free, non-blocking lock
    // into a false timeout. Always make one immediate try_lock attempt. The
    // deadline governs waiting/retries only after real contention is observed.
    let mut contention_count = match file.try_lock() {
        Ok(()) => {
            return Ok(FileLock {
                _file: file,
                #[cfg(unix)]
                contention_count: 0,
            });
        }
        Err(std::fs::TryLockError::WouldBlock) => 1_u64,
        Err(std::fs::TryLockError::Error(error)) => {
            return Err(AdapterError::new(
                ErrorCode::Internal,
                format!("Failed to acquire {purpose}: {error}"),
            ));
        }
    };

    loop {
        if deadline.is_expired() {
            return Err(lock_timeout(deadline, purpose, path, contention_count));
        }
        let remaining = deadline.remaining();
        if remaining.is_zero() {
            return Err(lock_timeout(deadline, purpose, path, contention_count));
        }
        std::thread::sleep(remaining.min(Duration::from_millis(10)));
        match file.try_lock() {
            Ok(()) => {
                return Ok(FileLock {
                    _file: file,
                    #[cfg(unix)]
                    contention_count,
                });
            }
            Err(std::fs::TryLockError::WouldBlock) => {
                contention_count = contention_count.saturating_add(1);
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(AdapterError::new(
                    ErrorCode::Internal,
                    format!("Failed to acquire {purpose}: {error}"),
                ));
            }
        }
    }
}

fn lock_timeout(
    deadline: Deadline,
    purpose: &str,
    path: &Path,
    contention_count: u64,
) -> AdapterError {
    deadline.timeout_error().with_details(serde_json::json!({
        "kind": "lock_timeout",
        "purpose": purpose,
        "path": path.display().to_string(),
        "contention_count": contention_count,
    }))
}

fn io_error(error: std::io::Error) -> AdapterError {
    AdapterError::new(ErrorCode::Internal, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!(
                "agent-desktop-file-lock-{name}-{}-{}",
                std::process::id(),
                crate::refs::new_snapshot_id()
            ))
            .join("state.lock")
    }

    #[test]
    fn uncontended_lock_gets_one_nonblocking_attempt_at_deadline() {
        let path = lock_path("first-attempt");
        let file = crate::private_file::open_private_lock(&path, true).unwrap();
        let deadline = Deadline::after(0).unwrap();

        let lock = lock_file(file, deadline, "test lock", &path).unwrap();
        drop(lock);

        let _ = std::fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }

    #[test]
    fn contended_lock_at_deadline_still_times_out_without_waiting() {
        let path = lock_path("contended-deadline");
        let held = crate::private_file::open_private_lock(&path, true).unwrap();
        held.try_lock().unwrap();
        let contender = crate::private_file::open_private_lock(&path, false).unwrap();
        let deadline = Deadline::after(0).unwrap();

        let error = match lock_file(contender, deadline, "test lock", &path) {
            Ok(_) => panic!("contended lock unexpectedly acquired"),
            Err(error) => error,
        };
        assert_eq!(error.code, ErrorCode::Timeout);

        drop(held);
        let _ = std::fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}
