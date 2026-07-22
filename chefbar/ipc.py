"""Mini-IPC over een Unix socket: `chefbar --bar` praat met de lopende tray.

De hotkey start een tweede chefbar-proces; dat proces stuurt alleen een
commando ("bar" / "panel") naar de tray en stopt weer. Zo opent de bar in
<100 ms in het proces dat de cache en de watcher al warm heeft.
"""

from __future__ import annotations

import logging
import os
import socket
import threading
from pathlib import Path
from typing import Callable

log = logging.getLogger("chefbar.ipc")

SOCKET_PATH = Path(
    os.environ.get("CHEFBAR_SOCKET", str(Path.home() / ".local/share/chefbar/chefbar.sock"))
)


def send_command(command: str, timeout: float = 2.0) -> bool:
    """Stuur een commando naar de lopende tray; False als die er niet is."""
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
            sock.settimeout(timeout)
            sock.connect(str(SOCKET_PATH))
            sock.sendall(command.encode() + b"\n")
            return sock.recv(16).strip() == b"ok"
    except (OSError, TimeoutError):
        return False


class IpcServer:
    """Luistert op de socket en dispatcht commando's naar de GTK main loop.

    `dispatch` is GLib.idle_add (geïnjecteerd door de tray) zodat deze module
    geen gi-import nodig heeft; het hotkey-clientproces blijft daardoor licht.
    """

    def __init__(
        self,
        handlers: dict[str, Callable[[], None]],
        dispatch: Callable[[Callable[[], None]], object],
    ) -> None:
        self.handlers = handlers
        self.dispatch = dispatch
        self._sock: socket.socket | None = None

    def start(self) -> None:
        SOCKET_PATH.parent.mkdir(parents=True, exist_ok=True)
        SOCKET_PATH.unlink(missing_ok=True)
        self._sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self._sock.bind(str(SOCKET_PATH))
        os.chmod(SOCKET_PATH, 0o600)
        self._sock.listen(4)
        threading.Thread(target=self._loop, daemon=True).start()
        log.info("IPC luistert op %s", SOCKET_PATH)

    def _loop(self) -> None:
        assert self._sock is not None
        while True:
            try:
                conn, _addr = self._sock.accept()
            except OSError:
                return
            with conn:
                try:
                    command = conn.recv(64).decode().strip()
                    handler = self.handlers.get(command)
                    if handler is not None:
                        self.dispatch(handler)
                        conn.sendall(b"ok\n")
                    else:
                        conn.sendall(b"unknown\n")
                except OSError:
                    continue
