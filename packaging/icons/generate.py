#!/usr/bin/env python3
"""Generate the placeholder PacketSmith application icon set.

The icon is a rounded-square gradient tile from the Aurora theme palette with
a white "PS" monogram. It renders at 1024x1024 with analytic anti-aliasing and
emits the PNG, ICO (Windows), and ICNS (macOS) files committed under
packaging/icons/.

Run from any directory:

    python3 packaging/icons/generate.py

Replace the committed assets with final brand artwork when it is available.
"""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

MASTER = 1024
OUT_DIR = Path(__file__).resolve().parent

# Aurora theme accent palette (see crates/packetsmith-app/src/shell/theme.rs).
ACCENT_TOP = (0x63, 0x8C, 0xFF)
ACCENT_BOTTOM = (0x36, 0x5A, 0xC8)
INK = (0xFF, 0xFF, 0xFF)

TILE_INSET = 64.0
TILE_RADIUS = 210.0
STROKE_WIDTH = 92.0

PNG_SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)
ICO_SIZES = (16, 24, 32, 48, 64, 128, 256)
ICNS_SIZES = {
    "icp4": 16,
    "icp5": 32,
    "icp6": 64,
    "ic07": 128,
    "ic08": 256,
    "ic09": 512,
    "ic10": 1024,
    "ic11": 32,
    "ic12": 64,
    "ic13": 256,
    "ic14": 512,
}


def cubic(p0, p1, p2, p3, steps=28):
    """Sample a cubic Bezier curve into points, including both endpoints."""
    points = []
    for index in range(steps + 1):
        t = index / steps
        one_t = 1.0 - t
        x = (
            one_t**3 * p0[0]
            + 3 * one_t**2 * t * p1[0]
            + 3 * one_t * t**2 * p2[0]
            + t**3 * p3[0]
        )
        y = (
            one_t**3 * p0[1]
            + 3 * one_t**2 * t * p1[1]
            + 3 * one_t * t**2 * p2[1]
            + t**3 * p3[1]
        )
        points.append((x, y))
    return points


def polyline(*chains):
    points = []
    for chain in chains:
        points.extend(chain if points and points[-1] != chain[0] else chain)
    return points


def monogram_strokes():
    """Centerline strokes for a geometric "PS" monogram, in pixel space."""
    strokes = []

    # P stem.
    strokes.append([(300.0, 352.0), (300.0, 672.0)])

    # P bowl: out around the right and back into the stem.
    bowl = cubic((300.0, 352.0), (444.0, 352.0), (492.0, 394.0), (492.0, 452.0))
    bowl += cubic((492.0, 452.0), (492.0, 510.0), (444.0, 552.0), (300.0, 552.0))[1:]
    strokes.append(bowl)

    # S: upper hook, middle diagonal, lower hook.
    s_curve = cubic((744.0, 398.0), (716.0, 352.0), (628.0, 336.0), (596.0, 382.0))
    s_curve += cubic((596.0, 382.0), (566.0, 424.0), (620.0, 466.0), (668.0, 494.0))[1:]
    s_curve += cubic((668.0, 494.0), (716.0, 522.0), (732.0, 566.0), (706.0, 612.0))[1:]
    s_curve += cubic((706.0, 612.0), (672.0, 672.0), (576.0, 678.0), (548.0, 626.0))[1:]
    strokes.append(s_curve)

    return strokes


def rounded_rect_distance(x, y):
    half = MASTER / 2.0 - TILE_INSET
    center = MASTER / 2.0
    qx = abs(x - center) - (half - TILE_RADIUS)
    qy = abs(y - center) - (half - TILE_RADIUS)
    outside = math.hypot(max(qx, 0.0), max(qy, 0.0))
    inside = min(max(qx, qy), 0.0)
    return outside + inside - TILE_RADIUS


def segment_distance(px, py, x0, y0, x1, y1):
    dx = x1 - x0
    dy = y1 - y0
    length_sq = dx * dx + dy * dy
    if length_sq == 0.0:
        return math.hypot(px - x0, py - y0)
    t = ((px - x0) * dx + (py - y0) * dy) / length_sq
    t = max(0.0, min(1.0, t))
    return math.hypot(px - (x0 + t * dx), py - (y0 + t * dy))


def render_master():
    pixels = bytearray(MASTER * MASTER * 4)

    # Background tile with a diagonal gradient.
    for y in range(MASTER):
        for x in range(MASTER):
            coverage = max(0.0, min(1.0, 0.5 - rounded_rect_distance(x + 0.5, y + 0.5)))
            if coverage <= 0.0:
                continue
            t = ((x + y) - 2.0 * TILE_INSET) / (2.0 * (MASTER - 2.0 * TILE_INSET))
            t = max(0.0, min(1.0, t))
            r = ACCENT_TOP[0] + (ACCENT_BOTTOM[0] - ACCENT_TOP[0]) * t
            g = ACCENT_TOP[1] + (ACCENT_BOTTOM[1] - ACCENT_TOP[1]) * t
            b = ACCENT_TOP[2] + (ACCENT_BOTTOM[2] - ACCENT_TOP[2]) * t
            offset = (y * MASTER + x) * 4
            pixels[offset] = int(r + 0.5)
            pixels[offset + 1] = int(g + 0.5)
            pixels[offset + 2] = int(b + 0.5)
            pixels[offset + 3] = int(coverage * 255.0 + 0.5)

    # Monogram strokes composited over the tile.
    radius = STROKE_WIDTH / 2.0
    for stroke in monogram_strokes():
        for index in range(len(stroke) - 1):
            x0, y0 = stroke[index]
            x1, y1 = stroke[index + 1]
            min_x = max(0, int(min(x0, x1) - radius - 2))
            max_x = min(MASTER - 1, int(max(x0, x1) + radius + 2))
            min_y = max(0, int(min(y0, y1) - radius - 2))
            max_y = min(MASTER - 1, int(max(y0, y1) + radius + 2))
            for y in range(min_y, max_y + 1):
                py = y + 0.5
                row = y * MASTER
                for x in range(min_x, max_x + 1):
                    distance = segment_distance(x + 0.5, py, x0, y0, x1, y1)
                    coverage = max(0.0, min(1.0, radius + 0.5 - distance))
                    if coverage <= 0.0:
                        continue
                    offset = (row + x) * 4
                    inverse = 1.0 - coverage
                    pixels[offset] = int(
                        pixels[offset] * inverse + INK[0] * coverage + 0.5
                    )
                    pixels[offset + 1] = int(
                        pixels[offset + 1] * inverse + INK[1] * coverage + 0.5
                    )
                    pixels[offset + 2] = int(
                        pixels[offset + 2] * inverse + INK[2] * coverage + 0.5
                    )
                    pixels[offset + 3] = max(
                        pixels[offset + 3], int(coverage * 255.0 + 0.5)
                    )

    return pixels


def downsample(pixels, source_size, target_size):
    if target_size == source_size:
        return pixels
    factor = source_size // target_size
    output = bytearray(target_size * target_size * 4)
    for y in range(target_size):
        for x in range(target_size):
            r = g = b = a = 0
            for dy in range(factor):
                source_row = (y * factor + dy) * source_size
                for dx in range(factor):
                    offset = (source_row + x * factor + dx) * 4
                    r += pixels[offset]
                    g += pixels[offset + 1]
                    b += pixels[offset + 2]
                    a += pixels[offset + 3]
            samples = factor * factor
            offset = (y * target_size + x) * 4
            output[offset] = r // samples
            output[offset + 1] = g // samples
            output[offset + 2] = b // samples
            output[offset + 3] = a // samples
    return output


def encode_png(width, height, pixels):
    def chunk(tag, payload):
        return (
            struct.pack(">I", len(payload))
            + tag
            + payload
            + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
        )

    stride = width * 4
    raw = b"".join(
        b"\x00" + bytes(pixels[y * stride : (y + 1) * stride]) for y in range(height)
    )
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def write_ico(path, images):
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries = []
    payloads = []
    for size, data in images:
        entries.append(
            struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset)
        )
        payloads.append(data)
        offset += len(data)
    path.write_bytes(header + b"".join(entries) + b"".join(payloads))


def write_icns(path, images):
    entries = b""
    for tag, data in images:
        entries += tag.encode("ascii") + struct.pack(">I", len(data) + 8) + data
    payload = b"icns" + struct.pack(">I", len(entries) + 8) + entries
    path.write_bytes(payload)


def main():
    master = render_master()
    rendered = {
        size: encode_png(size, size, downsample(master, MASTER, size))
        for size in PNG_SIZES
    }

    for size, data in rendered.items():
        (OUT_DIR / f"app-icon-{size}.png").write_bytes(data)
    # Primary source image used by tooling that expects a single PNG.
    (OUT_DIR / "app-icon.png").write_bytes(rendered[1024])

    write_ico(
        OUT_DIR / "app-icon.ico",
        [(size, rendered[size]) for size in ICO_SIZES],
    )

    write_icns(
        OUT_DIR / "AppIcon.icns",
        [(tag, rendered[size]) for tag, size in ICNS_SIZES.items()],
    )

    print(f"Wrote PNG, ICO, and ICNS assets to {OUT_DIR}")


if __name__ == "__main__":
    main()
