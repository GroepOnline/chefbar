#!/usr/bin/env python3
"""ChefBar 2.0 launcher — thin entry that loads the chefbar package."""

from __future__ import annotations

import sys
from pathlib import Path

# Support both repo layout and ~/.local/share/chefbar install layout.
_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

from chefbar.__main__ import main  # noqa: E402

if __name__ == "__main__":
    raise SystemExit(main())
