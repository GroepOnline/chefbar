#!/usr/bin/env python3
"""Deterministische generator voor de ChefBar tray/status-PNG's.

Rendert de verticale CG-statuslijn (de enige Signaal-signature,
`.ulpi/design/DESIGN.md` + `chefbar-tray.md`) als RGBA-PNG's in
`chefbar/icons/`. Pure stdlib (struct + zlib), geen Pillow of
netwerkfetch: tweemaal draaien geeft byte-identieke output.

Status is nooit alleen kleur: elke state heeft een eigen vorm
(outline, gevuld segment, dot-badge, !-badge, gestreepte lijn).

Gebruik:
    python3 build_icons.py            # schrijf alle iconen
    python3 build_icons.py --check    # verifieer zonder te schrijven
"""

from __future__ import annotations

import struct
import sys
import zlib
from pathlib import Path

ICONS_DIR = Path(__file__).resolve().parent / "chefbar" / "icons"

# Supersampling voor rustige randen op 22px.
SS = 4

# Kleurcontract (DESIGN.md). Lichte glyph voor de donkere GNOME-balk;
# statuskleuren uit de donkere ladder zodat ze op de balk leesbaar zijn.
LINE = (244, 244, 246, 255)        # --text donker thema
MUTED = (156, 156, 157, 255)       # --text-muted donker thema
BRAND = (255, 107, 82, 255)        # --brand donker thema
SUCCESS = (89, 212, 153, 255)      # --success donker thema
WARNING = (255, 197, 51, 255)      # --warning donker thema
ERROR = (255, 97, 97, 255)         # --error donker thema
WHITE = (255, 255, 255, 255)


class Canvas:
    """RGBA-raster met simpele primitieven op SS×-resolutie."""

    def __init__(self, size: int) -> None:
        self.size = size
        self.n = size * SS
        self.px: list[list[tuple[int, int, int, int]]] = [
            [(0, 0, 0, 0)] * self.n for _ in range(self.n)
        ]

    def rect(self, x0: float, y0: float, x1: float, y1: float, color) -> None:
        xa, ya = int(round(x0 * SS)), int(round(y0 * SS))
        xb, yb = int(round(x1 * SS)), int(round(y1 * SS))
        for y in range(max(0, ya), min(self.n, yb)):
            row = self.px[y]
            for x in range(max(0, xa), min(self.n, xb)):
                row[x] = color

    def disk(self, cx: float, cy: float, r: float, color) -> None:
        cxs, cys, rs = cx * SS, cy * SS, r * SS
        r2 = rs * rs
        for y in range(max(0, int(cys - rs) - 1), min(self.n, int(cys + rs) + 2)):
            row = self.px[y]
            for x in range(max(0, int(cxs - rs) - 1), min(self.n, int(cxs + rs) + 2)):
                if (x + 0.5 - cxs) ** 2 + (y + 0.5 - cys) ** 2 <= r2:
                    row[x] = color

    def downsample(self) -> bytes:
        """Middel SS×SS blokken; premultiplied middeling voor nette randen."""
        out = bytearray()
        for by in range(self.size):
            out.append(0)  # PNG-filter: None
            for bx in range(self.size):
                rs = gs = bs = as_ = 0
                for sy in range(by * SS, (by + 1) * SS):
                    row = self.px[sy]
                    for sx in range(bx * SS, (bx + 1) * SS):
                        r, g, b, a = row[sx]
                        rs += r * a
                        gs += g * a
                        bs += b * a
                        as_ += a
                if as_ == 0:
                    out += b"\x00\x00\x00\x00"
                else:
                    out += bytes(
                        (rs // as_, gs // as_, bs // as_, as_ // (SS * SS))
                    )
        return bytes(out)


def _chunk(tag: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + tag
        + payload
        + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
    )


# -- de CG-statuslijn --------------------------------------------------------

def _line_geometry(s: int) -> tuple[float, float, float, float]:
    """(x0, x1, y0, y1) van de verticale statuslijn, gecentreerd."""
    lw = max(4.0, s * 0.20)
    x0 = (s - lw) / 2
    m = s * 0.14
    return x0, x0 + lw, m, s - m


def draw_stil(c: Canvas) -> None:
    """Lijn als outline: er is verbinding, er gebeurt niks."""
    s = c.size
    x0, x1, y0, y1 = _line_geometry(s)
    th = max(1.2, s / 14)
    c.rect(x0, y0, x1, y1, LINE)
    c.rect(x0 + th, y0 + th, x1 - th, y1 - th, (0, 0, 0, 0))


def draw_bezig(c: Canvas) -> None:
    """Outline-lijn met gevuld middensegment: er wordt gewerkt."""
    s = c.size
    x0, x1, y0, y1 = _line_geometry(s)
    th = max(1.2, s / 14)
    c.rect(x0, y0, x1, y1, LINE)
    c.rect(x0 + th, y0 + th, x1 - th, y1 - th, (0, 0, 0, 0))
    h = y1 - y0
    c.rect(x0, y0 + h * 0.38, x1, y1 - h * 0.20, LINE)


def draw_hulp(c: Canvas) -> None:
    """Gevulde lijn met brand-dot rechtsboven: even jou nodig."""
    s = c.size
    x0, x1, y0, y1 = _line_geometry(s)
    c.rect(x0, y0, x1, y1, LINE)
    r = max(2.6, s * 0.16)
    c.disk(s - r - 1, r + 1, r, BRAND)


def draw_fout(c: Canvas) -> None:
    """Gevulde lijn met !-badge rechtsonder: een dienst hapert."""
    s = c.size
    x0, x1, y0, y1 = _line_geometry(s)
    c.rect(x0, y0, x1, y1, LINE)
    r = max(4.0, s * 0.24)
    cx, cy = s - r - 0.5, s - r - 0.5
    c.disk(cx, cy, r, ERROR)
    bw = max(1.0, r * 0.30)
    c.rect(cx - bw / 2, cy - r * 0.62, cx + bw / 2, cy + r * 0.10, WHITE)
    c.rect(cx - bw / 2, cy + r * 0.32, cx + bw / 2, cy + r * 0.32 + bw, WHITE)


def draw_offline(c: Canvas) -> None:
    """Gestreepte, gedempte lijn: geen verbinding."""
    s = c.size
    x0, x1, y0, y1 = _line_geometry(s)
    h = y1 - y0
    seg, gap = h * 0.22, h * 0.17
    y = y0
    while y < y1 - 0.5:
        c.rect(x0, y, x1, min(y + seg, y1), MUTED)
        y += seg + gap


def draw_status(c: Canvas, kind: str) -> None:
    """Mini-statuslijn voor ok/warn/down (joep-notify e.d.).

    Vorm verschilt per status: ok = volle lijn, warn = lijn + dot
    (uitroep-vorm), down = gebroken lijn. Kleur bevestigt alleen.
    """
    s = c.size
    x0, x1, y0, y1 = _line_geometry(s)
    h = y1 - y0
    if kind == "ok":
        c.rect(x0, y0, x1, y1, SUCCESS)
    elif kind == "warn":
        c.rect(x0, y0, x1, y1 - h * 0.34, WARNING)
        r = (x1 - x0) / 2
        c.disk((x0 + x1) / 2, y1 - r, r, WARNING)
    else:  # down
        c.rect(x0, y0, x1, y0 + h * 0.40, ERROR)
        c.rect(x0, y1 - h * 0.40, x1, y1, ERROR)


TRAY_STATES = {
    "stil": draw_stil,
    "bezig": draw_bezig,
    "hulp": draw_hulp,
    "fout": draw_fout,
    "offline": draw_offline,
}
STATUS_KINDS = ("ok", "warn", "down")


def targets() -> dict[str, tuple]:
    """name -> (size, draw-callable). Bestandsnamen zijn het contract."""
    out: dict[str, tuple] = {}
    for state, fn in TRAY_STATES.items():
        out[f"tray-{state}.png"] = (22, fn)
        out[f"tray-{state}-32.png"] = (32, fn)
        out[f"tray-{state}-48.png"] = (48, fn)
    for kind in STATUS_KINDS:
        fn = (lambda c, k=kind: draw_status(c, k))
        out[f"{kind}.png"] = (22, fn)
        out[f"{kind}-22.png"] = (22, fn)
        out[f"{kind}-32.png"] = (32, fn)
        out[f"{kind}-48.png"] = (48, fn)
    return out


def render(name: str, size: int, fn) -> bytes:
    canvas = Canvas(size)
    fn(canvas)
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", ihdr)
        + _chunk(b"IDAT", zlib.compress(canvas.downsample(), 9))
        + _chunk(b"IEND", b"")
    )


def main(argv: list[str]) -> int:
    check = "--check" in argv
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    stale = 0
    for name, (size, fn) in sorted(targets().items()):
        data = render(name, size, fn)
        path = ICONS_DIR / name
        if check:
            if not path.exists() or path.read_bytes() != data:
                print(f"STALE {name}")
                stale += 1
            continue
        path.write_bytes(data)
        print(f"{name}: {size}x{size} · {len(data)} bytes")
    if check:
        print("OK: alle iconen actueel" if not stale else f"{stale} iconen verouderd")
        return 1 if stale else 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
