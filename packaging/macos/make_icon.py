#!/usr/bin/env python3
"""Draws the Vibe Play app icon (rounded square, play triangle) into a .iconset folder. Standard library only.

usage: make_icon.py OUT.iconset
"""
import math
import struct
import sys
import zlib
from pathlib import Path


def sd_round_box(px, py, half, radius):
    qx, qy = abs(px) - half + radius, abs(py) - half + radius
    return math.hypot(max(qx, 0.0), max(qy, 0.0)) + min(max(qx, qy), 0.0) - radius


def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))


def sd_triangle(px, py, pts, radius):
    # rounded convex triangle: distance to the polygon, minus the corner radius (the polygon is pre-shrunk by the caller)
    d = min(seg_dist(px, py, *pts[i], *pts[(i + 1) % 3]) for i in range(3))
    signs = [(pts[(i + 1) % 3][0] - pts[i][0]) * (py - pts[i][1]) - (pts[(i + 1) % 3][1] - pts[i][1]) * (px - pts[i][0]) for i in range(3)]
    inside = all(v >= 0 for v in signs) or all(v <= 0 for v in signs)
    return (-d if inside else d) - radius


def render(size):
    half = size / 2.0
    margin = size * 0.0977  # Apple's icon grid: the artwork is ~80% of the canvas
    box = half - margin
    corner = box * 0.46
    # triangle (pointing right), optically centred, with rounded corners
    r = box * 0.07
    cx = size * 0.015
    pts = [(-box * 0.30 + cx, -box * 0.42), (-box * 0.30 + cx, box * 0.42), (box * 0.46 + cx, 0.0)]
    # shrink towards the centroid by r so the final rounding keeps the intended outline
    gx, gy = sum(p[0] for p in pts) / 3, sum(p[1] for p in pts) / 3
    pts = [(gx + (x - gx) * 0.9, gy + (y - gy) * 0.9) for x, y in pts]
    rows = bytearray()
    for y in range(size):
        rows.append(0)
        py = y + 0.5 - half
        for x in range(size):
            px = x + 0.5 - half
            a_bg = min(1.0, max(0.0, 0.5 - sd_round_box(px, py, box, corner)))
            if a_bg <= 0.0:
                rows += b"\x00\x00\x00\x00"
                continue
            t = (py + box) / (2 * box)  # top -> bottom
            br = int(124 + (59 - 124) * t)
            bg = int(131 + (63 - 131) * t)
            bb = int(255 + (216 - 255) * t)
            a_fg = min(1.0, max(0.0, 0.5 - sd_triangle(px, py, pts, r)))
            rr = br + (255 - br) * a_fg
            gg = bg + (255 - bg) * a_fg
            bl = bb + (255 - bb) * a_fg
            rows += bytes((int(rr), int(gg), int(bl), int(a_bg * 255)))
    return bytes(rows)


def png(size):
    raw = render(size)

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for base in (16, 32, 128, 256, 512):
        (out / f"icon_{base}x{base}.png").write_bytes(png(base))
        (out / f"icon_{base}x{base}@2x.png").write_bytes(png(base * 2))


if __name__ == "__main__":
    main()
