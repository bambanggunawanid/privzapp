#!/usr/bin/env python3
"""Derive all PrivZapp brand icons from the master logo.

Source of truth: app/brand/logo-master.png — the owner-provided lettermark
(amber bolt-P on a navy->purple rounded tile, on a black matte).

Outputs:
  app/pwa/icon-512.png            tile cropped, transparent rounded corners
  app/pwa/icon-192.png            "
  app/pwa/icon-maskable-512.png   full-bleed (corners gradient-filled,
                                  mark safely inside the 80% zone)
  app/pwa/apple-touch-icon.png    180px full-bleed
  app/assets/logo.png             256px rounded — favicon + in-app brand

Stdlib only: a minimal PNG codec (8-bit RGB/RGBA, non-interlaced) plus a
fractional box-average downscaler. Rerun after replacing the master:
    python3 scripts/gen-icons.py
"""

import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MASTER = ROOT / "app" / "brand" / "logo-master.png"

# Anything at or below this luminance counts as the black matte.
MATTE_MAX = 28


# ---- minimal PNG codec ----------------------------------------------------

def read_png(path):
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, w, h, depth, ctype, idat = 8, 0, 0, 0, 0, b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        tag = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        if tag == b"IHDR":
            w, h, depth, ctype, _c, _f, interlace = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and ctype in (2, 6) and interlace == 0, (
                "gen-icons only reads 8-bit RGB/RGBA non-interlaced PNGs"
            )
        elif tag == b"IDAT":
            idat += body
        elif tag == b"IEND":
            break
        pos += 12 + length
    ch = 3 if ctype == 2 else 4
    raw = zlib.decompress(idat)
    stride = w * ch
    out = bytearray(h * stride)
    prev = bytearray(stride)
    pos = 0
    for y in range(h):
        filt = raw[pos]
        line = bytearray(raw[pos + 1 : pos + 1 + stride])
        pos += 1 + stride
        if filt == 1:  # Sub
            for i in range(ch, stride):
                line[i] = (line[i] + line[i - ch]) & 0xFF
        elif filt == 2:  # Up
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif filt == 3:  # Average
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 0xFF
        elif filt == 4:  # Paeth
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                b = prev[i]
                c = prev[i - ch] if i >= ch else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pred) & 0xFF
        out[y * stride : (y + 1) * stride] = line
        prev = line
    # normalize to RGBA rows of tuples
    px = [[None] * w for _ in range(h)]
    for y in range(h):
        base = y * stride
        for x in range(w):
            o = base + x * ch
            r, g, b = out[o], out[o + 1], out[o + 2]
            a = out[o + 3] if ch == 4 else 255
            px[y][x] = (r, g, b, a)
    return w, h, px


def write_png(path, w, h, rows):
    def chunk(tag, body):
        c = struct.pack(">I", len(body)) + tag + body
        return c + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    raw = b"".join(bytes([0]) + bytes(v for p in row for v in p) for row in rows)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(png)
    print(f"{path.relative_to(ROOT)}  {w}x{h}  {len(png)} bytes")


# ---- geometry -------------------------------------------------------------

def is_matte(p):
    return max(p[0], p[1], p[2]) <= MATTE_MAX


def tile_bounds(w, h, px):
    xs = [x for y in range(0, h, 4) for x in range(w) if not is_matte(px[y][x])]
    ys = [y for y in range(h) if any(not is_matte(px[y][x]) for x in range(0, w, 4))]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    # Corner radius: at the tile's top edge the non-matte run is inset by r
    # on each side (rounded-rect geometry).
    probe = y0 + 2
    run = [x for x in range(x0, x1 + 1) if not is_matte(px[probe][x])]
    r = ((x1 - x0) - (run[-1] - run[0])) / 2 if run else 0
    return x0, y0, x1, y1, max(r, 0)


def box_resize(px, x0, y0, x1, y1, size):
    """Fractional box-average of the source region into size×size."""
    sw, sh = x1 - x0, y1 - y0
    out = []
    for oy in range(size):
        sy0, sy1 = y0 + sh * oy / size, y0 + sh * (oy + 1) / size
        row = []
        for ox in range(size):
            sx0, sx1 = x0 + sw * ox / size, x0 + sw * (ox + 1) / size
            acc = [0.0, 0.0, 0.0, 0.0]
            area = 0.0
            for yy in range(int(sy0), min(int(sy1) + 1, y1)):
                cy = min(sy1, yy + 1) - max(sy0, yy)
                if cy <= 0:
                    continue
                for xx in range(int(sx0), min(int(sx1) + 1, x1)):
                    cx = min(sx1, xx + 1) - max(sx0, xx)
                    if cx <= 0:
                        continue
                    wgt = cx * cy
                    p = px[yy][xx]
                    acc[0] += p[0] * wgt
                    acc[1] += p[1] * wgt
                    acc[2] += p[2] * wgt
                    acc[3] += p[3] * wgt
                    area += wgt
            row.append(tuple(round(c / area) for c in acc))
        out.append(row)
    return out


def round_corners(rows, radius_frac):
    """Alpha-out everything beyond a rounded-rect, antialiased."""
    size = len(rows)
    r = size * radius_frac
    ss = 3
    for y in range(size):
        for x in range(size):
            hits = 0
            for sy in range(ss):
                for sx in range(ss):
                    fx, fy = x + (sx + 0.5) / ss, y + (sy + 0.5) / ss
                    cx = min(max(fx, r), size - r)
                    cy = min(max(fy, r), size - r)
                    if (fx - cx) ** 2 + (fy - cy) ** 2 <= r * r:
                        hits += 1
            if hits < ss * ss:
                p = rows[y][x]
                rows[y][x] = (p[0], p[1], p[2], round(p[3] * hits / (ss * ss)))
    return rows


def fill_corners(rows, radius_frac):
    """Full-bleed: replace matte corners by sampling the gradient just
    inside the rounded corner, so color runs edge to edge (maskable)."""
    size = len(rows)
    r = size * radius_frac
    centers = [(r, r), (size - r, r), (r, size - r), (size - r, size - r)]
    for y in range(size):
        for x in range(size):
            fx, fy = x + 0.5, y + 0.5
            cx = min(max(fx, r), size - r)
            cy = min(max(fy, r), size - r)
            d2 = (fx - cx) ** 2 + (fy - cy) ** 2
            if d2 > (r - 2) ** 2 and (cx, cy) in centers and d2 > 0:
                d = d2 ** 0.5
                t = (r - 3) / d
                sx = min(size - 1, max(0, int(cx + (fx - cx) * t)))
                sy = min(size - 1, max(0, int(cy + (fy - cy) * t)))
                p = rows[sy][sx]
                rows[y][x] = (p[0], p[1], p[2], 255)
    return rows


def maskable(px, x0, y0, x1, y1, radius_frac, size, content=0.70):
    """Maskable icon: bilinear gradient synthesized from the tile's corner
    colors fills the whole square; the tile itself sits centered at
    `content` scale (feathered edges) so every mark tip survives a
    circular mask (safe-zone radius is 40% of the canvas)."""
    r = int((x1 - x0) * radius_frac)
    c00, c10 = px[y0 + r][x0 + r], px[y0 + r][x1 - r]
    c01, c11 = px[y1 - r][x0 + r], px[y1 - r][x1 - r]
    rows = []
    for y in range(size):
        ty = y / (size - 1)
        row = []
        for x in range(size):
            tx = x / (size - 1)
            top = [c00[i] + (c10[i] - c00[i]) * tx for i in range(3)]
            bot = [c01[i] + (c11[i] - c01[i]) * tx for i in range(3)]
            row.append(tuple(round(top[i] + (bot[i] - top[i]) * ty) for i in range(3)) + (255,))
        rows.append(row)

    inner_size = round(size * content)
    inner = box_resize(px, x0, y0, x1 + 1, y1 + 1, inner_size)
    # Feather on signed distance to the rounded-rect boundary so the
    # tile's baked-in edge shadow dissolves into the gradient with no
    # visible arcs at the corners.
    ir = inner_size * radius_frac
    feather = max(4, inner_size // 10)
    for y in range(inner_size):
        for x in range(inner_size):
            fx, fy = x + 0.5, y + 0.5
            cx = min(max(fx, ir), inner_size - ir)
            cy = min(max(fy, ir), inner_size - ir)
            d_corner = ((fx - cx) ** 2 + (fy - cy) ** 2) ** 0.5
            if d_corner > 0:
                inside = ir - d_corner
            else:
                inside = min(fx, fy, inner_size - fx, inner_size - fy)
            a = min(max(inside / feather, 0.0), 1.0)
            p = inner[y][x]
            inner[y][x] = (p[0], p[1], p[2], round(p[3] * a))
    off = (size - inner_size) // 2
    for y in range(inner_size):
        for x in range(inner_size):
            p = inner[y][x]
            a = p[3] / 255
            if a > 0:
                b = rows[y + off][x + off]
                rows[y + off][x + off] = tuple(
                    round(b[i] * (1 - a) + p[i] * a) for i in range(3)
                ) + (255,)
    return rows


def og_image(px, x0, y0, x1, y1, radius_frac, w=1200, h=630):
    """Open Graph / Twitter card: the tile centered on a gradient canvas
    synthesized from its own corner colors."""
    r = int((x1 - x0) * radius_frac)
    c00, c10 = px[y0 + r][x0 + r], px[y0 + r][x1 - r]
    c01, c11 = px[y1 - r][x0 + r], px[y1 - r][x1 - r]
    rows = []
    for y in range(h):
        ty = y / (h - 1)
        row = []
        for x in range(w):
            tx = x / (w - 1)
            top = [c00[i] + (c10[i] - c00[i]) * tx for i in range(3)]
            bot = [c01[i] + (c11[i] - c01[i]) * tx for i in range(3)]
            row.append(tuple(round(top[i] + (bot[i] - top[i]) * ty) for i in range(3)) + (255,))
        rows.append(row)

    tile = round(h * 0.78)
    inner = box_resize(px, x0, y0, x1 + 1, y1 + 1, tile)
    ir = tile * radius_frac
    feather = max(4, tile // 10)
    ox, oy = (w - tile) // 2, (h - tile) // 2
    for y in range(tile):
        for x in range(tile):
            fx, fy = x + 0.5, y + 0.5
            cx = min(max(fx, ir), tile - ir)
            cy = min(max(fy, ir), tile - ir)
            d_corner = ((fx - cx) ** 2 + (fy - cy) ** 2) ** 0.5
            inside = ir - d_corner if d_corner > 0 else min(fx, fy, tile - fx, tile - fy)
            a = min(max(inside / feather, 0.0), 1.0)
            p = inner[y][x]
            if a > 0:
                b = rows[y + oy][x + ox]
                rows[y + oy][x + ox] = tuple(
                    round(b[i] * (1 - a) + p[i] * a) for i in range(3)
                ) + (255,)
    return rows


def main():
    w, h, px = read_png(MASTER)
    x0, y0, x1, y1, r = tile_bounds(w, h, px)
    tile_w = x1 - x0
    radius_frac = r / tile_w
    print(f"master {w}x{h}, tile ({x0},{y0})-({x1},{y1}), corner r={r:.0f}px ({radius_frac:.2f})")

    pwa = ROOT / "app" / "pwa"
    for size, path in [(512, pwa / "icon-512.png"), (192, pwa / "icon-192.png"),
                       (256, ROOT / "app" / "assets" / "logo.png")]:
        rows = box_resize(px, x0, y0, x1 + 1, y1 + 1, size)
        write_png(path, size, size, round_corners(rows, radius_frac))

    write_png(pwa / "icon-maskable-512.png", 512, 512,
              maskable(px, x0, y0, x1, y1, radius_frac, 512))
    write_png(pwa / "og-image.png", 1200, 630,
              og_image(px, x0, y0, x1, y1, radius_frac))
    # iOS masks with a rounded square only, so full-bleed is safe there.
    rows = box_resize(px, x0, y0, x1 + 1, y1 + 1, 180)
    write_png(pwa / "apple-touch-icon.png", 180, 180, fill_corners(rows, radius_frac))


if __name__ == "__main__":
    main()
