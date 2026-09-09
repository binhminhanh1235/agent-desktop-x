mod catalog;
mod protocol;
mod skills;

use agent_desktop_core::{
    AppError, PlatformAdapter, commands::batch::BatchCommand, context::CommandContext,
};
use serde_json::Value;
use std::ffi::OsStr;
use std::io::{self, BufRead, BufWriter, Write};
use std::process::ExitCode;

pub(crate) fn requested() -> bool {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    let mcp = arguments
        .iter()
        .any(|argument| argument == OsStr::new("--mcp"));
    let help_or_version = arguments.iter().any(|argument| {
        matches!(
            argument.to_str(),
            Some("-h" | "--help" | "-V" | "--version")
        )
    });
    mcp && !help_or_version
}

pub(crate) fn run() -> ExitCode {
    #[cfg(target_os = "windows")]
    let _ = agent_desktop_core::install_private_file_ops(Box::new(
        agent_desktop_windows::WindowsPrivateFile,
    ));

    if let Err(error) = agent_desktop_core::validate_state_root_env() {
        eprintln!("agent-desktop MCP: {error}");
        return ExitCode::FAILURE;
    }

    crate::init_tracing(false);

    #[cfg(target_os = "windows")]
    if let Err(error) = agent_desktop_windows::ensure_owned_process_mta_and_dpi() {
        eprintln!("agent-desktop MCP: {error}");
        return ExitCode::FAILURE;
    }

    let adapter = crate::build_adapter();
    let headed = headed_requested();
    match serve(&adapter, headed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("agent-desktop MCP I/O error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn serve(adapter: &dyn PlatformAdapter, headed: bool) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(_)) => Some(protocol::invalid_request()),
            Ok(request) => protocol::handle(request, adapter, headed),
            Err(_) => Some(protocol::parse_error()),
        };
        if let Some(response) = response {
            serde_json::to_writer(&mut writer, &response).map_err(io::Error::other)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }
    Ok(())
}

fn headed_requested() -> bool {
    std::env::args_os().any(|argument| argument == OsStr::new("--headed"))
        || std::env::var("AGENT_DESKTOP_MCP_HEADED")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

pub(super) fn invoke_tool(
    tool_name: &str,
    arguments: Value,
    adapter: &dyn PlatformAdapter,
    headed: bool,
) -> Result<Value, AppError> {
    let command = catalog::command_for_tool(tool_name)
        .ok_or_else(|| AppError::invalid_input(format!("Unknown MCP tool '{tool_name}'")))?;
    let command = crate::batch::parse_command(BatchCommand {
        command,
        session: None,
        args: arguments,
    })?;
    let context = CommandContext::default().with_headed(headed);
    crate::execute_with_adapter(command, adapter, &context)
}
