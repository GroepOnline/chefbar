"""CLI entry: `python -m chefbar` or via apps/chefbar/chefbar.py."""

from __future__ import annotations

import argparse
import logging
import os
import sys
from pathlib import Path

LOG_DIR = Path.home() / ".local/share/chefbar"
LOG_DIR.mkdir(parents=True, exist_ok=True)


def _setup_logging() -> None:
    logging.basicConfig(
        filename=str(LOG_DIR / "chefbar.log"),
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
    )
    # Also mirror warnings to stderr when interactive / show-panel.
    if sys.stderr.isatty() or "--show-panel" in sys.argv:
        console = logging.StreamHandler()
        console.setLevel(logging.INFO)
        console.setFormatter(logging.Formatter("%(levelname)s %(message)s"))
        logging.getLogger().addHandler(console)


def main(argv: list[str] | None = None) -> int:
    _setup_logging()
    parser = argparse.ArgumentParser(description="ChefBar 2.0 — ChefGroep OS tray panel")
    parser.add_argument(
        "--show-panel",
        action="store_true",
        help="Testmodus: open alleen het mission-control panel (geen tray)",
    )
    parser.add_argument(
        "--bar",
        action="store_true",
        help="Open de command-bar (via de lopende tray, anders standalone)",
    )
    parser.add_argument(
        "--version",
        action="store_true",
        help="Print versie en exit",
    )
    args = parser.parse_args(argv)

    if args.version:
        from . import __version__

        print(f"chefbar {__version__}")
        return 0

    if args.bar:
        # Snelle route: vraag de lopende tray de bar te openen (geen GTK nodig).
        from . import ipc

        if ipc.send_command("bar"):
            return 0

    # Ensure DISPLAY for GTK.
    if not os.environ.get("DISPLAY") and not os.environ.get("WAYLAND_DISPLAY"):
        logging.error("Geen DISPLAY/WAYLAND_DISPLAY · ChefBar heeft een graphical session nodig")
        return 1

    # Force X11 (XWayland): Wayland toplevels cannot be positioned, so the
    # panel could not anchor under the tray icon. Via XWayland kan
    # window.move() wel, waardoor het panel als bar-menu rechtsboven opent.
    if os.environ.get("XDG_SESSION_TYPE") == "wayland" and os.environ.get("DISPLAY"):
        os.environ.setdefault("GDK_BACKEND", "x11")

    if args.bar:
        from .bar import run_bar_only

        run_bar_only()
        return 0

    if args.show_panel:
        from .panel import run_panel_only

        run_panel_only()
        return 0

    from .indicator import run

    run()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
