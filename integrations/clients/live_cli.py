"""Bounded driver for actual installed CLIs; stdin/stdout carry synthetic test data.

This is an acceptance driver, not an alternative MCP client. Each application's
own MCP implementation launches the real HOTR bridge and executes its tools.
"""
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

TOOLS = ["hotr_health", "hotr_search", "hotr_get", "hotr_create", "hotr_revise"]
MAX_OUTPUT = 8 * 1024 * 1024


def checked(path, root):
    candidate = Path(path).resolve(strict=True)
    assert candidate.is_relative_to(root), "client path outside owned profile"
    return candidate


def bounded_process(args, cwd, env, prompt, reserve_prompt, claude=False, preflight_only=False):
    process = subprocess.Popen(args, cwd=cwd, env=env, stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               creationflags=subprocess.CREATE_NO_WINDOW)
    events = queue.Queue(maxsize=64)

    def pump(stream, label):
        while chunk := stream.read1(4096):
            events.put((label, chunk))
        events.put((label, None))

    for label, stream in [("stdout", process.stdout), ("stderr", process.stderr)]:
        threading.Thread(target=pump, args=(stream, label), daemon=True).start()
    output = {"stdout": bytearray(), "stderr": bytearray()}
    start = time.monotonic()
    def send(value):
        process.stdin.write((json.dumps(value) + "\n").encode())
        process.stdin.flush()

    def control(identifier, subtype):
        send({"type": "control_request", "request_id": identifier,
              "request": {"subtype": subtype}})

    ready = not claude
    started_prompt = False
    pending_lines = bytearray()
    next_status = None
    status_number = 0
    ended = set()
    failure = None
    try:
        if claude:
            control("hotr-init", "initialize")
        else:
            assert not preflight_only, "preflight-only is a Claude control check"
            reserve_prompt()
            started_prompt = True
            process.stdin.write(prompt.encode())
            process.stdin.close()
        while len(ended) < 2:
            if time.monotonic() - start > 180:
                failure = "installed client exceeded 180 seconds"
                break
            if not ready and time.monotonic() - start > 35:
                failure = "Claude MCP preflight exceeded 35 seconds; no prompt sent"
                break
            if next_status is not None and time.monotonic() >= next_status:
                status_number += 1
                control(f"hotr-status-{status_number}", "mcp_status")
                next_status = None
            try:
                label, chunk = events.get(timeout=0.1)
            except queue.Empty:
                continue
            if chunk is None:
                ended.add(label)
            else:
                output[label].extend(chunk)
                if sum(map(len, output.values())) > MAX_OUTPUT:
                    failure = "installed client output exceeded bound"
                    break
                if claude and not ready and label == "stdout":
                    pending_lines.extend(chunk)
                    while b"\n" in pending_lines:
                        line, _, tail = pending_lines.partition(b"\n")
                        pending_lines = bytearray(tail)
                        row = json.loads(line)
                        if row.get("type") != "control_response":
                            continue
                        response = row.get("response", {})
                        identifier = response.get("request_id", "")
                        if response.get("subtype") == "error":
                            failure = "Claude rejected MCP preflight; no prompt sent"
                            break
                        if identifier == "hotr-init":
                            next_status = time.monotonic()
                        elif identifier.startswith("hotr-status-"):
                            servers = response.get("response", {}).get("mcpServers", [])
                            if len(servers) == 1 and servers[0].get("name") == "hotr":
                                server = servers[0]
                                names = {tool["name"] for tool in server.get("tools", [])}
                                ready = server.get("status") == "connected" and names == set(TOOLS)
                                if server.get("status") in {"failed", "disabled", "needs-auth"}:
                                    failure = "Claude MCP connection failed; no prompt sent"
                                    break
                            if ready:
                                if not preflight_only:
                                    reserve_prompt()
                                    started_prompt = True
                                    send({"type": "user", "session_id": "",
                                          "message": {"role": "user", "content": prompt},
                                          "parent_tool_use_id": None})
                                process.stdin.close()
                            else:
                                next_status = time.monotonic() + 0.25
                if failure:
                    break
        if not ready and failure is None:
            failure = "Claude ended before MCP readiness; no prompt sent"
        if failure and process.poll() is None:
            process.kill()
        code = process.wait(timeout=10)
        decoded = {k: bytes(v).decode("utf-8", errors="replace") for k, v in output.items()}
        decoded["failure"] = failure
        decoded["mcp_ready"] = ready
        decoded["model_prompt_sent"] = started_prompt
        return (124 if failure else code), decoded, time.monotonic() - start
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)


def main(request):
    repository = Path(__file__).resolve().parents[2]
    root = (repository / "work/hotr-client-profiles").resolve(strict=True)
    profile = checked(request["profile"], root)
    assert (profile / "SYNTHETIC-ONLY").read_text().startswith("HOTR-12;"), "unowned profile"
    workspace = checked(request["workspace"], root)
    # Keep provider credentials out of Codex and out of unrelated subprocesses.
    keep = {"SYSTEMROOT", "WINDIR", "COMSPEC", "PATH", "PATHEXT", "USERPROFILE",
            "APPDATA", "LOCALAPPDATA", "PROGRAMDATA", "PROGRAMFILES", "PROGRAMFILES(X86)",
            "TEMP", "TMP", "NUMBER_OF_PROCESSORS", "PROCESSOR_ARCHITECTURE"}
    env = {k: v for k, v in os.environ.items() if k.upper() in keep}
    env.update({"NO_COLOR": "1", "DO_NOT_TRACK": "1"})
    app = request["app"]
    if app == "codex":
        env["CODEX_HOME"] = str(profile)
        args = [os.environ["HOTR_CODEX_EXE"], "exec", "--strict-config", "--ephemeral",
                "--skip-git-repo-check", "--sandbox", "read-only", "--color", "never", "--json", "-"]
    elif app == "claude":
        assert os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com").rstrip("/") == "https://api.anthropic.com", "configured provider override needs a matching adapter"
        key = os.environ.get("ANTHROPIC_API_KEY")
        assert key, "existing Anthropic API credential unavailable"
        env.update({"ANTHROPIC_API_KEY": key, "CLAUDE_CONFIG_DIR": str(profile),
                    "CLAUDE_CODE_SUBPROCESS_ENV_SCRUB": "1", "DISABLE_AUTOUPDATER": "1",
                    "ENABLE_TOOL_SEARCH": "false", "MCP_CONNECTION_NONBLOCKING": "0",
                    "MCP_CONNECT_TIMEOUT_MS": "15000", "MCP_TIMEOUT": "15000",
                    "MCP_TOOL_TIMEOUT": "15000"})
        # Bare mode uses only the explicitly supplied existing API credential.
        args = [os.environ["HOTR_CLAUDE_EXE"], "--bare", "--print", "--output-format", "stream-json",
                "--input-format", "stream-json",
                "--verbose", "--no-session-persistence", "--no-chrome", "--disable-slash-commands",
                "--strict-mcp-config", "--mcp-config", str(profile / request["mcp_config"]),
                "--tools", "", "--allowedTools", "mcp__hotr__*", "--permission-mode", "dontAsk",
                "--max-budget-usd", "1", "--effort", "low"]
    else:
        raise ValueError("unsupported acceptance client")
    version = subprocess.run([args[0], "--version"], env=env, cwd=workspace,
                             capture_output=True, timeout=15, check=True).stdout.decode().strip()
    if app == "codex":
        preflight = subprocess.run([args[0], "mcp", "list", "--json"],
                                   cwd=workspace, env=env, capture_output=True, timeout=20)
        if preflight.returncode:
            return {"app": app, "version": version, "exit_code": preflight.returncode,
                    "phase": "configuration", "calls": [],
                    "stderr": preflight.stderr.decode(errors="replace")}
    # Durable milestone-wide budget includes failed attempts. No automatic replay.
    ledger = repository / "work/hotr-evidence/HOTR-M1-client-budget.jsonl"
    def reserve_prompt():
        used = len(ledger.read_text().splitlines()) if ledger.exists() else 0
        assert used < 12, "M1's twelve-prompt budget is exhausted"
        with ledger.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps({"app": app, "profile": str(profile.relative_to(root)),
                                     "started_unix": time.time(), "attempt": used + 1}) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
    code, output, elapsed = bounded_process(
        args, workspace, env, request["prompt"], reserve_prompt,
        claude=app == "claude", preflight_only=request.get("preflight_only", False))
    # Parse the application's real event stream; final prose alone never proves a tool call.
    rows, unparsed = [], []
    for line in output["stdout"].splitlines():
        if line.strip():
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                unparsed.append(line)
    calls = []
    final = None
    for row in rows:
        item = row.get("item", {})
        if row.get("type") == "item.completed" and item.get("type") == "mcp_tool_call":
            calls.append(item)
        if row.get("type") == "assistant":
            for item in row.get("message", {}).get("content", []):
                if item.get("type") == "tool_use":
                    calls.append(item)
        if row.get("type") in {"result", "turn.completed", "turn.failed"}:
            final = row
    # Return data to the native fixture in memory. It retains only sanitized metadata.
    return {"app": app, "version": version, "exit_code": code, "elapsed_seconds": elapsed,
            "calls": calls, "final": final, "events": rows, "stderr": output["stderr"],
            "driver_failure": output["failure"], "unparsed_stdout": unparsed,
            "mcp_preflight_ready": output["mcp_ready"],
            "model_prompt_sent": output["model_prompt_sent"],
            "executable_sha256": hashlib.sha256(Path(args[0]).read_bytes()).hexdigest()}


if __name__ == "__main__":
    try:
        raw = sys.stdin.buffer.read(131073)
        assert len(raw) <= 131072, "driver input exceeded bound"
        print(json.dumps(main(json.loads(raw))))
    except Exception as exc:
        # No request, environment, authentication file, or raw exception is printed.
        print(json.dumps({"driver_error": type(exc).__name__}), file=sys.stderr)
        sys.exit(1)
