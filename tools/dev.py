"""Start and stop Scriberig's local API and web development services."""
import os
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
import webbrowser
from pathlib import Path

from dev_toolchain import cargo_command
ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "playground" / "dev-app-runtime"
DATABASE = Path(
    os.environ.get(
        "SCRIBE_DATABASE_PATH",
        str(RUNTIME / "scriberig.sqlite"),
    )
).expanduser()
API_PORT = str(int(os.environ.get("SCRIBE_API_PORT", "7337")))
WEB_PORT = str(int(os.environ.get("SCRIBE_WEB_PORT", "5173")))
API_URL = f"http://127.0.0.1:{API_PORT}/api/health"
WEB_URL = f"http://127.0.0.1:{WEB_PORT}"
SERVICES = ("api", "web")
def service_paths(name: str) -> dict[str, Path]:
    """Return PID and output paths for one development service.
    Args:
        name: Service name, either API or web.
    Returns:
        Mapping of process marker and log paths.
    """
    # Store runtime files below one ignored, purpose-named directory.
    return {
        "pid": RUNTIME / f"{name}.pid",
        "output": RUNTIME / f"{name}.log",
        "error": RUNTIME / f"{name}-error.log",
    }


def read_pid(name: str) -> int | None:
    """Read a saved PID marker when it contains a valid process identifier.

    Args:
        name: Service name.

    Returns:
        A positive PID, or None for an absent or invalid marker.
    """
    # Ignore stale marker contents so startup can repair them.
    marker = service_paths(name)["pid"]
    try:
        pid = int(marker.read_text(encoding="utf-8").strip())
    except (OSError, ValueError):
        return None
    return pid if pid > 0 else None


def process_command_line(pid: int) -> str:
    """Read a process command line using the operating system's process view.

    Args:
        pid: Process identifier to inspect.

    Returns:
        Lowercase command line, or an empty string when the process is absent.
    """
    # Inspect command identity before reusing or stopping a saved PID.
    if sys.platform == "linux":
        try:
            value = Path(f"/proc/{pid}/cmdline").read_bytes()
        except OSError:
            return ""
        return value.replace(b"\0", b" ").decode(errors="replace").lower()

    if os.name == "nt":
        query = (
            "(Get-CimInstance Win32_Process -Filter "
            f"'ProcessId = {pid}').CommandLine"
        )
        try:
            result = subprocess.run(
                ["powershell.exe", "-NoProfile", "-Command", query],
                capture_output=True,
                check=False,
                text=True,
                timeout=5,
            )
        except (OSError, subprocess.TimeoutExpired):
            return ""
        return result.stdout.lower()

    try:
        result = subprocess.run(
            ["ps", "-p", str(pid), "-o", "command="],
            capture_output=True,
            check=False,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return ""
    return result.stdout.lower()


def process_matches(pid: int, name: str) -> bool:
    """Check whether a PID marker still belongs to the requested service.

    Args:
        pid: Process identifier to validate.
        name: Service name.

    Returns:
        True only when its command line identifies the expected service.
    """
    # Avoid acting on an unrelated process after operating-system PID reuse.
    command_line = process_command_line(pid)
    if name == "api":
        return "scriberig" in command_line and "serve" in command_line
    return "@scriberig/app" in command_line or "vite" in command_line


def launch(name: str, command: str, arguments: list[str]) -> int:
    """Launch one service in the background and save its PID.

    Args:
        name: Service name.
        command: Executable to launch.
        arguments: Arguments passed to the executable.

    Returns:
        PID of the newly launched process.

    Raises:
        OSError: If the executable cannot be started.
    """
    # Redirect output to ignored log files and detach from the Make process.
    files = service_paths(name)
    if os.name == "nt" and command.endswith(".cmd"):
        command_line = subprocess.list2cmdline([command, *arguments])
        command = os.environ.get("COMSPEC", "cmd.exe")
        arguments = ["/d", "/s", "/c", command_line]

    flags = 0
    if os.name == "nt":
        flags = subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP
    with files["output"].open("wb") as output, files["error"].open("wb") as error:
        child = subprocess.Popen(
            [command, *arguments],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=output,
            stderr=error,
            close_fds=os.name != "nt",
            creationflags=flags,
            start_new_session=os.name != "nt",
        )
    files["pid"].write_text(f"{child.pid}\n", encoding="utf-8")
    return child.pid


def ensure_service(name: str, command: str, arguments: list[str]) -> tuple[int, bool]:
    """Reuse a running service or launch it when its marker is stale.

    Args:
        name: Service name.
        command: Executable to launch.
        arguments: Arguments passed to the executable.

    Returns:
        Tuple containing the process ID and whether this call launched it.
    """
    # Make repeated starts reuse the same managed service processes.
    marker = service_paths(name)["pid"]
    pid = read_pid(name)
    if pid is not None and process_matches(pid, name):
        return pid, False
    marker.unlink(missing_ok=True)
    return launch(name, command, arguments), True


def api_is_ready() -> bool:
    """Check whether the API health route reports a ready database.

    Returns:
        True when the local API returns its ready status.
    """
    # Bound the request so filtered local traffic cannot hang startup.
    try:
        with urllib.request.urlopen(API_URL, timeout=1.2) as response:
            return b'"status":"ready"' in response.read()
    except (OSError, urllib.error.URLError, TimeoutError):
        return False


def web_is_ready() -> bool:
    """Check whether Vite is serving the local application.

    Returns:
        True when the web URL responds successfully.
    """
    # Verify the page before opening a browser window.
    try:
        with urllib.request.urlopen(WEB_URL, timeout=1.2) as response:
            return 200 <= response.status < 300
    except (OSError, urllib.error.URLError, TimeoutError):
        return False


def wait_until_ready(api_log: Path, api_started_before: bool) -> tuple[bool, bool, bool]:
    """Wait for the API startup log and web page to become ready.

    Args:
        api_log: Path to the API output log.
        api_started_before: Whether an API process was already managed.

    Returns:
        Tuple containing API startup, API health, and web readiness.
    """
    # Allow time for a clean Cargo build while bounding startup to ten minutes.
    api_started = False
    api_health = False
    api_health_checked = False
    web_ready = False
    for _ in range(600):
        try:
            api_started = api_started_before or (
                DATABASE.is_file()
                and "Scriberig API listening" in api_log.read_text(encoding="utf-8")
            )
        except OSError:
            api_started = api_started_before
        if api_started and not api_health_checked:
            api_health = api_is_ready()
            api_health_checked = True
        web_ready = web_is_ready()
        if api_started and web_ready:
            return api_started, api_health, web_ready
        time.sleep(1)
    return api_started, api_health, web_ready


def stop_process(pid: int) -> None:
    """Stop a saved process and its subprocess tree.

    Args:
        pid: Process identifier to stop.

    Returns:
        Nothing.
    """
    # Use process groups on Unix and taskkill's tree option on Windows.
    if os.name == "nt":
        subprocess.run(
            ["taskkill.exe", "/PID", str(pid), "/T", "/F"],
            capture_output=True,
            check=False,
        )
        return
    try:
        os.killpg(pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    except OSError:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            pass


def stop() -> None:
    """Stop both managed services and remove their markers.

    Returns:
        Nothing.
    """
    # Stop only validated Scriberig processes, leaving unrelated services alone.
    for name in SERVICES:
        marker = service_paths(name)["pid"]
        pid = read_pid(name)
        if pid is not None and process_matches(pid, name):
            stop_process(pid)
        marker.unlink(missing_ok=True)
    (RUNTIME / "browser-opened").unlink(missing_ok=True)
    print("Stopped the Scriberig development processes.")


def start() -> None:
    """Start or reuse the API and web application, then open the browser.

    Returns:
        Nothing.

    Raises:
        RuntimeError: If either service fails to become ready.
    """
    # Store the database, service logs, and process markers together.
    RUNTIME.mkdir(parents=True, exist_ok=True)
    DATABASE.parent.mkdir(parents=True, exist_ok=True)
    project_roots = os.environ.get("SCRIBERIG_PROJECT_ROOT")
    project_root_arguments = []
    if project_roots:
        project_root_arguments = ["--project-root", project_roots]
    api_pid, api_new = ensure_service(
        "api",
        cargo_command(),
        [
            "run",
            "-p",
            "scriberig",
            "--",
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            API_PORT,
            "--db",
            str(DATABASE),
            *project_root_arguments,
        ],
    )
    pnpm = "pnpm.cmd" if os.name == "nt" else "pnpm"
    web_pid, web_new = ensure_service(
        "web",
        pnpm,
        [
            "--dir",
            "web",
            "--filter",
            "@scriberig/app",
            "dev",
            "--host",
            "127.0.0.1",
            "--port",
            WEB_PORT,
        ],
    )
    del api_pid, web_pid
    if api_new or web_new:
        (RUNTIME / "browser-opened").unlink(missing_ok=True)

    ready, api_health, web_ready = wait_until_ready(
        service_paths("api")["output"], not api_new
    )
    if not ready or not web_ready:
        for name, pid, started in (
            ("api", read_pid("api"), api_new),
            ("web", read_pid("web"), web_new),
        ):
            if started and pid is not None:
                stop_process(pid)
                service_paths(name)["pid"].unlink(missing_ok=True)
        raise RuntimeError("Development services did not start; inspect playground/dev-app-runtime logs.")

    browser_marker = RUNTIME / "browser-opened"
    if not browser_marker.exists() and not os.environ.get("SCRIBERIG_NO_BROWSER"):
        if not webbrowser.open(WEB_URL, new=1):
            print("Could not open the browser automatically; visit " + WEB_URL)
        browser_marker.write_text("opened\n", encoding="utf-8")
    print("Scriberig is open at " + WEB_URL)
    print("API health: " + API_URL)
    print("Database: " + str(DATABASE))
    if not api_health:
        print("Warning: API started, but its health route is unreachable.")
    print("Run make dev-stop to stop the development processes.")


def main(arguments: list[str]) -> int:
    """Run the selected Makefile development command.

    Args:
        arguments: Command-line arguments after the script name.

    Returns:
        Process exit status.
    """
    # Keep start and stop available through one portable Python entry point.
    command = arguments[1] if len(arguments) > 1 else "help"
    if command == "start":
        start()
        return 0
    if command == "stop":
        stop()
        return 0
    print("Usage: python tools/dev.py <start|stop>")
    return 0 if command == "help" else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv))
    except (OSError, RuntimeError) as error:
        print(f"Development command failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
