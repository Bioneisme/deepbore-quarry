#!/usr/bin/env python3
"""Draws the mod's textures: original art, standard library only.

    python tools/make_textures.py

Writes content/assets/deepbore_quarry/textures/{block,item,gui}/*.png and icon.png.
"""
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TEX = ROOT / "content" / "assets" / "deepbore_quarry" / "textures"

CLEAR = (0, 0, 0, 0)
EDGE = (28, 30, 36, 255)
DARK = (58, 62, 70, 255)
STEEL = (92, 98, 108, 255)
LIGHT = (136, 142, 152, 255)
SHINE = (180, 186, 194, 255)
AMBER = (214, 158, 46, 255)
AMBER_DARK = (140, 96, 24, 255)
DIAMOND = (92, 226, 222, 255)


def write_png(path, width, height, pixels):
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw += bytes(pixels[y][x])

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def canvas(width, height, color=CLEAR):
    return [[color] * width for _ in range(height)]


def rect(p, x, y, w, h, color):
    for j in range(y, y + h):
        for i in range(x, x + w):
            p[j][i] = color


def noise(x, y):
    return ((x * 73856093) ^ (y * 19349663)) >> 3 & 3


def plate():
    """A riveted steel plate, the block's every face starts from."""
    p = canvas(16, 16, STEEL)
    for y in range(16):
        for x in range(16):
            if noise(x, y) == 0:
                p[y][x] = (86, 92, 102, 255)
    rect(p, 0, 0, 16, 1, LIGHT)
    rect(p, 0, 0, 1, 16, LIGHT)
    rect(p, 0, 15, 16, 1, EDGE)
    rect(p, 15, 0, 1, 16, EDGE)
    for x, y in [(2, 2), (13, 2), (2, 13), (13, 13)]:
        p[y][x] = SHINE
        p[y + 1][x] = DARK
    return p


def side():
    p = plate()
    for x in range(1, 15):
        c = AMBER if (x // 2) % 2 == 0 else EDGE
        rect(p, x, 6, 1, 4, c)
    rect(p, 1, 5, 14, 1, DARK)
    rect(p, 1, 10, 14, 1, DARK)
    return p


def top():
    p = plate()
    rect(p, 4, 4, 8, 8, DARK)
    rect(p, 5, 5, 6, 6, EDGE)
    rect(p, 7, 5, 2, 6, AMBER_DARK)
    rect(p, 5, 7, 6, 2, AMBER_DARK)
    rect(p, 7, 7, 2, 2, AMBER)
    return p


def front():
    """The side towards the player who placed it: a panel with a diamond lamp."""
    p = plate()
    rect(p, 3, 3, 10, 5, EDGE)
    rect(p, 4, 4, 2, 3, DIAMOND)
    for x in range(7, 12, 2):
        rect(p, x, 4, 1, 3, STEEL)
    rect(p, 3, 9, 10, 4, DARK)
    for y in range(10, 13, 2):
        rect(p, 4, y, 8, 1, EDGE)
    return p


def back():
    """The side towards the pit: the bore's mouth."""
    p = plate()
    rect(p, 3, 3, 10, 10, EDGE)
    rect(p, 5, 5, 6, 6, DARK)
    rect(p, 7, 3, 2, 10, AMBER_DARK)
    rect(p, 3, 7, 10, 2, AMBER_DARK)
    rect(p, 7, 7, 2, 2, DIAMOND)
    return p


def frame():
    """The gantry's beams, tiled along their length."""
    p = canvas(16, 16, AMBER)
    for y in range(16):
        for x in range(16):
            if (x + y) % 8 < 4:
                p[y][x] = AMBER_DARK
    rect(p, 0, 0, 16, 2, LIGHT)
    rect(p, 0, 14, 16, 2, DARK)
    return p


def drill():
    """The shaft and the head: a spiral that reads as turning."""
    p = canvas(16, 16, STEEL)
    for y in range(16):
        for x in range(16):
            if (x + y) % 6 < 2:
                p[y][x] = DIAMOND if (x + y) % 6 == 0 else EDGE
    return p


def upgrade(glyph, color, dark):
    """An upgrade: a circuit card with its glyph in the middle."""
    p = canvas(16, 16)
    rect(p, 2, 2, 12, 12, EDGE)
    rect(p, 3, 3, 10, 10, DARK)
    rect(p, 3, 3, 10, 1, STEEL)
    for x in (4, 7, 10):
        rect(p, x, 14, 2, 1, AMBER)
    for y, row in enumerate(glyph):
        for x, c in enumerate(row):
            if c == "#":
                p[4 + y][4 + x] = color
            elif c == "+":
                p[4 + y][4 + x] = dark
    return p


GLYPHS = {
    "area_upgrade": (
        ["##++++##", "#++++++#", "++++++++", "++++++++", "++++++++", "++++++++", "#++++++#", "##++++##"],
        (110, 220, 110, 255),
        (40, 96, 48, 255),
    ),
    "speed_upgrade": (
        ["#...#...", "##..##..", "+##.+##.", "..##..##", "..##..##", "+##.+##.", "##..##..", "#...#..."],
        (240, 200, 70, 255),
        (140, 100, 24, 255),
    ),
    "fluids_upgrade": (
        ["...##...", "...##...", "..####..", "..#+##..", ".#+####.", ".#+####.", ".######.", "..####.."],
        (84, 150, 240, 255),
        (190, 220, 255, 255),
    ),
    "chunk_upgrade": (
        ["..####..", ".#++++#.", "#++##++#", "#+#..#+#", "#+#..#+#", "#++##++#", ".#++++#.", "..####.."],
        (190, 110, 240, 255),
        (70, 150, 110, 255),
    ),
}


def slot(p, x, y):
    """A slot's 18 pixel frame, at the item's corner minus one."""
    rect(p, x - 1, y - 1, 18, 18, (198, 198, 198, 255))
    rect(p, x - 1, y - 1, 17, 1, (55, 55, 55, 255))
    rect(p, x - 1, y - 1, 1, 17, (55, 55, 55, 255))
    rect(p, x, y + 16, 17, 1, (255, 255, 255, 255))
    rect(p, x + 16, y, 1, 17, (255, 255, 255, 255))
    rect(p, x, y, 16, 16, (139, 139, 139, 255))


# Where the screen's slots are: glue.rs has the same numbers.
FUEL_SLOT = (17, 35)
UPGRADE_SLOTS = [(62, 35), (80, 35), (98, 35), (116, 35)]


def panel():
    """The screen, 176 by 166: vanilla's grey with the slots drawn in."""
    p = canvas(176, 166, (198, 198, 198, 255))
    rect(p, 0, 0, 176, 2, (255, 255, 255, 255))
    rect(p, 0, 0, 2, 166, (255, 255, 255, 255))
    rect(p, 0, 164, 176, 2, (85, 85, 85, 255))
    rect(p, 174, 0, 2, 166, (85, 85, 85, 255))
    slot(p, *FUEL_SLOT)
    for at in UPGRADE_SLOTS:
        slot(p, *at)
    for row in range(3):
        for col in range(9):
            slot(p, 8 + col * 18, 84 + row * 18)
    for col in range(9):
        slot(p, 8 + col * 18, 142)
    return p


def icon():
    """The catalog's picture: a gantry over a stepped pit."""
    n = 64
    p = canvas(n, n, (120, 170, 220, 255))
    for y in range(26, n):
        rect(p, 0, y, n, 1, (112, 84, 58, 255) if y < 34 else (108, 108, 112, 255))
    rect(p, 0, 26, n, 2, (92, 150, 70, 255))
    for i, (x, w) in enumerate([(10, 44), (15, 34), (20, 24), (25, 14)]):
        rect(p, x, 26 + i * 8, w, 8, (38, 36, 40, 255))
    rect(p, 8, 8, 48, 3, AMBER)
    rect(p, 8, 8, 3, 20, AMBER)
    rect(p, 53, 8, 3, 20, AMBER)
    rect(p, 28, 6, 8, 7, STEEL)
    rect(p, 31, 13, 2, 40, LIGHT)
    rect(p, 29, 53, 6, 3, DIAMOND)
    rect(p, 30, 56, 4, 2, DIAMOND)
    rect(p, 31, 58, 2, 2, DIAMOND)
    return [[p[y // 4][x // 4] for x in range(256)] for y in range(256)]


def main():
    for name, art in [
        ("quarry_side", side),
        ("quarry_top", top),
        ("quarry_front", front),
        ("quarry_back", back),
        ("frame", frame),
        ("drill", drill),
    ]:
        write_png(TEX / "block" / f"{name}.png", 16, 16, art())
    for name, (glyph, color, dark) in GLYPHS.items():
        write_png(TEX / "item" / f"{name}.png", 16, 16, upgrade(glyph, color, dark))
    write_png(TEX / "gui" / "quarry.png", 176, 166, panel())
    write_png(ROOT / "icon.png", 256, 256, icon())


if __name__ == "__main__":
    main()
