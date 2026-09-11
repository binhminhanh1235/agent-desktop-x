#!/usr/bin/env python3
import argparse
import json
import subprocess
import sys
import time
from typing import List, Optional, Tuple

PROTOCOL_VERSION = "2025-11-25"
RESTART_CYCLES = 10


class McpSession:
    def __init__(self, binary: str):
        self.proc = subprocess.Popen(
            [binary, "--mcp"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self.next_id = 0
        self.call(
            "initialize",
            {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "aro-p1b-macos-lifecycle", "version": "1"},
            },
        )
        self.notify("notifications/initialized")

    def request(self, payload: dict) -> dict:
        assert self.proc.stdin is not None
        assert self.proc.stdout is not None
        self.proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            stderr = ""
            if self.proc.stderr is not None:
                stderr = self.proc.stderr.read()
            raise RuntimeError(f"MCP process closed unexpectedly: {stderr}")
        return json.loads(line)

    def call(self, method: str, params: Optional[dict] = None) -> dict:
        self.next_id += 1
        payload = {"jsonrpc": "2.0", "id": self.next_id, "method": method}
        if params is not None:
            payload["params"] = params
        response = self.request(payload)
        if "error" in response:
            raise RuntimeError(f"MCP protocol error: {response['error']}")
        return response

    def notify(self, method: str, params: Optional[dict] = None) -> None:
        payload = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            payload["params"] = params
        assert self.proc.stdin is not None
        self.proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        self.proc.stdin.flush()

    def tool(self, name: str, arguments: dict) -> dict:
        response = self.call(
            "tools/call", {"name": name, "arguments": arguments}
        )
        return response["result"]

    def close(self) -> None:
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait(timeout=5)


def pids_for(app: str) -> List[int]:
    result = subprocess.run(
        ["pgrep", "-x", app], capture_output=True, text=True, check=False
    )
    if result.returncode not in (0, 1):
        raise RuntimeError(f"pgrep failed for {app}: {result.stderr}")
    return [int(value) for value in result.stdout.split()]


def wait_for_exit(app: str, timeout: float = 10.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not pids_for(app):
            return
        time.sleep(0.05)
    raise RuntimeError(f"{app} did not exit before timeout")


def wait_for_new_pid(app: str, old_pid: int, timeout: float = 10.0) -> int:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        pids = pids_for(app)
        if len(pids) == 1 and pids[0] != old_pid:
            return pids[0]
        time.sleep(0.05)
    raise RuntimeError(f"{app} did not relaunch with a new PID before timeout")


def wait_for_external_window(binary: str, app: str, expected_pid: int, timeout: float = 10.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            [binary, "list-windows", "--app", app],
            capture_output=True,
            text=True,
            check=False,
        )
        try:
            payload = json.loads(result.stdout)
        except json.JSONDecodeError:
            payload = {}
        windows = payload.get("data", []) if payload.get("ok") else []
        if any(int(window.get("pid", -1)) == expected_pid for window in windows):
            return
        time.sleep(0.1)
    raise RuntimeError(
        f"fresh CLI process did not observe {app} window for PID {expected_pid}"
    )


def structured_success(result: dict, label: str) -> dict:
    if result.get("isError"):
        text = result.get("content", [{}])[0].get("text", "")
        raise RuntimeError(f"{label} failed: {text}")
    structured = result.get("structuredContent")
    if not isinstance(structured, dict):
        raise RuntimeError(f"{label} omitted structuredContent")
    return structured


def observe_view(mcp: McpSession, app: str) -> Tuple[str, int]:
    structured = structured_success(
        mcp.tool(
            "desktop.observe",
            {"command": "list-windows", "args": {"app": app}, "view": {}},
        ),
        "list-windows view",
    )
    view_id = structured["view"]["view_id"]
    windows = structured.get("result", [])
    if len(windows) != 1:
        raise RuntimeError(f"expected exactly one {app} window, got {len(windows)}")
    return view_id, int(windows[0]["pid"])


def semantic_find(mcp: McpSession, app: str) -> Tuple[str, str]:
    structured = structured_success(
        mcp.tool(
            "desktop.observe",
            {
                "command": "find",
                "args": {
                    "app": app,
                    "role": "button",
                    "name": "primary-button",
                    "first": True,
                },
            },
        ),
        "semantic find",
    )
    result = structured["result"]
    matches = result.get("matches", [])
    if len(matches) != 1:
        raise RuntimeError(f"expected one primary-button match, got {len(matches)}")
    return matches[0]["ref_id"], result["snapshot_id"]


def assert_old_view_stale(mcp: McpSession, old_view: str, cycle: int) -> None:
    sentinel = f"ARO-P1B-LIFECYCLE-{cycle}"
    subprocess.run(["pbcopy"], input=sentinel, text=True, check=True)
    result = mcp.tool(
        "desktop.execute",
        {
            "expected_view_id": old_view,
            "steps": [{"command": "clipboard-clear", "args": {}}],
        },
    )
    text = result.get("content", [{}])[0].get("text", "")
    if "VIEW_STALE" not in text:
        raise RuntimeError(f"old view was not rejected as VIEW_STALE: {text}")
    clipboard = subprocess.run(
        ["pbpaste"], capture_output=True, text=True, check=True
    ).stdout
    if clipboard != sentinel:
        raise RuntimeError("stale-view mutation modified the clipboard")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", required=True)
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--app", required=True)
    parser.add_argument("--cycles", type=int, default=RESTART_CYCLES)
    args = parser.parse_args()

    initial = pids_for(args.app)
    if len(initial) != 1:
        raise RuntimeError(f"expected one owned {args.app} process, got {initial}")

    mcp = McpSession(args.bin)
    try:
        old_ref, old_snapshot = semantic_find(mcp, args.app)
        old_view, observed_pid = observe_view(mcp, args.app)
        if observed_pid != initial[0]:
            raise RuntimeError(
                f"initial MCP PID {observed_pid} != owned fixture PID {initial[0]}"
            )

        for cycle in range(1, args.cycles + 1):
            old_pid = observed_pid
            subprocess.run(["pkill", "-x", args.app], check=True)
            wait_for_exit(args.app)
            subprocess.run(["open", args.fixture], check=True)
            new_pid = wait_for_new_pid(args.app, old_pid)
            wait_for_external_window(args.bin, args.app, new_pid)

            assert_old_view_stale(mcp, old_view, cycle)

            new_view, observed_pid = observe_view(mcp, args.app)
            if observed_pid != new_pid:
                raise RuntimeError(
                    f"cycle {cycle}: persistent MCP observed stale PID {observed_pid}; expected {new_pid}"
                )

            new_ref, new_snapshot = semantic_find(mcp, args.app)
            if new_snapshot == old_snapshot:
                raise RuntimeError(
                    f"cycle {cycle}: snapshot generation did not change across relaunch"
                )
            if new_ref == old_ref:
                raise RuntimeError(
                    f"cycle {cycle}: element ref did not change across relaunch"
                )

            print(
                f"cycle {cycle}/{args.cycles}: old_pid={old_pid} new_pid={new_pid} "
                f"VIEW_STALE + same-MCP recovery PASS"
            )
            old_view = new_view
            old_ref = new_ref
            old_snapshot = new_snapshot

        print(
            f"P1B macOS persistent lifecycle refresh PASS ({args.cycles} restart cycles)"
        )
        return 0
    finally:
        mcp.close()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:
        print(f"P1B macOS persistent lifecycle refresh FAIL: {error}", file=sys.stderr)
        sys.exit(1)
