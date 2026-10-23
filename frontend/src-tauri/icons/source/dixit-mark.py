#!/usr/bin/env python3
"""Dixit mark: a Roman inscriptional capital D with a triangular interpunct (D·), V-cut.

The outline is drawn with lines and cubic Béziers. The V-cut is computed: every boundary sample is
joined to its point on the medial axis (shrinking-ball method) and each facet takes its tone from
its edge normal against a light from the upper left (walls facing the light are in shade, as in a
real groove). Python stdlib only.

From `frontend/`:
    python3 src-tauri/icons/source/dixit-mark.py          # SVG sources + UI mark paths (stdout)
    python3 src-tauri/icons/source/dixit-mark.py --build  # also every raster: needs rsvg-convert, iconutil, pnpm

Outputs next to this file: dixit.svg (app icon, 1024, macOS grid), dixit-small.svg (16/32 px hint:
bigger letter, softer cut, no grain), tray-template.svg (menu-bar template), dixit-1024.png.
The UI mark (src/components/dixit/DixitMark.tsx) pastes the three paths printed on stdout.
"""
import math
import random
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ICONS = HERE.parent                     # src-tauri/icons
FRONTEND = ICONS.parent.parent          # frontend/
KAPPA = 0.5523

# ---------- geometry ----------

def bez(p0, p1, p2, p3, t):
    u = 1 - t
    return (u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
            u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1])


def point(seg, t):
    if seg[0] == 'L':
        (x0, y0), (x1, y1) = seg[1], seg[2]
        return (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t)
    return bez(*seg[1:], t)


def seg_len(seg):
    pts = [point(seg, i / 32) for i in range(33)]
    return sum(math.dist(pts[i], pts[i + 1]) for i in range(32))


def sample(contour, step):
    """Closed contour (list of segments) to points, last point not repeated."""
    pts = []
    for seg in contour:
        n = max(2, math.ceil(seg_len(seg) / step))
        pts += [point(seg, i / n) for i in range(n)]
    return pts


def path_d(contour, ox=0.0, oy=0.0):
    f = lambda p: f"{p[0] - ox:.2f} {p[1] - oy:.2f}".replace('.00', '')
    d = f"M{f(contour[0][1])}"
    for seg in contour:
        d += f"L{f(seg[2])}" if seg[0] == 'L' else f"C{f(seg[2])} {f(seg[3])} {f(seg[4])}"
    return d + "Z"


def letter(H, cx, cy, shift=0.0, **k):
    """Outer contour, counter and interpunct of D·, cap height H, block centred on (cx, cy).
    Proportions are fractions of H; the defaults are the app icon's."""
    g = lambda name, default: k.get(name, default) * H
    S, A, B = g('S', 0.215), g('A', 0.13), g('B', 0.235)          # stem, arms, bowl at its thickest
    W = 0.90 * H                                                  # stem left to bowl right
    Ls, Ts, Br = g('Ls', 0.10), g('Ts', 0.018), g('Br', 0.15)     # serif reach, tip, bracket
    gap, side = g('gap', 0.085), g('side', 0.15)                  # interpunct
    yT, yB = cy - H / 2, cy + H / 2
    x0 = cx - (Ls + W + gap + side) / 2 + Ls + shift
    x1, xR = x0 + S, x0 + W
    xa = x0 + 0.40 * W            # the outer bowl leaves the top line
    xi = xa - 0.02 * W            # the counter leaves the arm
    ymI = cy - 0.02 * H           # counter extreme a touch higher: stress below the middle
    kx = ky = 0.60                # outer fullness
    ix, iy = 0.58, 0.62           # counter fullness
    f = 0.006 * H                 # counter corners nearly sharp: a clean mitre instead of a fan of tones
    cR = xR - B
    outer = [
        ('L', (x0 - Ls, yT), (xa, yT)),
        ('C', (xa, yT), (xa + kx * (xR - xa), yT), (xR, cy - ky * (cy - yT)), (xR, cy)),
        ('C', (xR, cy), (xR, cy + ky * (yB - cy)), (xa + kx * (xR - xa), yB), (xa, yB)),
        ('L', (xa, yB), (x0 - Ls, yB)),
        ('L', (x0 - Ls, yB), (x0 - Ls, yB - Ts)),
        ('C', (x0 - Ls, yB - Ts), (x0 - Ls * 0.30, yB - Ts), (x0, yB - Ts - Br * 0.40), (x0, yB - Ts - Br)),
        ('L', (x0, yB - Ts - Br), (x0, yT + Ts + Br)),
        ('C', (x0, yT + Ts + Br), (x0, yT + Ts + Br * 0.40), (x0 - Ls * 0.30, yT + Ts), (x0 - Ls, yT + Ts)),
        ('L', (x0 - Ls, yT + Ts), (x0 - Ls, yT)),
    ]
    counter = [
        ('L', (x1, yT + A + f), (x1, yB - A - f)),
        ('C', (x1, yB - A - f), (x1, yB - A - f + KAPPA * f), (x1 + f - KAPPA * f, yB - A), (x1 + f, yB - A)),
        ('L', (x1 + f, yB - A), (xi, yB - A)),
        ('C', (xi, yB - A), (xi + ix * (cR - xi), yB - A), (cR, ymI + iy * (yB - A - ymI)), (cR, ymI)),
        ('C', (cR, ymI), (cR, ymI - iy * (ymI - yT - A)), (xi + ix * (cR - xi), yT + A), (xi, yT + A)),
        ('L', (xi, yT + A), (x1 + f, yT + A)),
        ('C', (x1 + f, yT + A), (x1 + f - KAPPA * f, yT + A), (x1, yT + A + f - KAPPA * f), (x1, yT + A + f)),
    ]
    # interpunct: triangle, apex down, centred on the middle of the capital
    tx, th = xR + gap + side / 2, side * math.sqrt(3) / 2
    a, b, c = (tx - side / 2, cy - th / 3), (tx + side / 2, cy - th / 3), (tx, cy + 2 * th / 3)
    dot = [('L', a, b), ('L', b, c), ('L', c, a)]
    return outer, counter, dot


def bbox(*contours):
    pts = [p for c in contours for seg in c for p in seg[1:]]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)

# ---------- the V-cut ----------

LIGHT = (-math.sqrt(0.5), -math.sqrt(0.5))   # towards the light: upper left (y down)


def inside_ring(p, ring):
    x, y = p
    c = False
    for i in range(len(ring)):
        (x1, y1), (x2, y2) = ring[i], ring[i - 1]
        if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1) + x1:
            c = not c
    return c


def facets(contours, step):
    """[(quad, tone 0..1)] along each contour; tone 0 = wall facing the light (shade)."""
    rings = [sample(c, step) for c in contours]
    every = [p for r in rings for p in r]
    inside = lambda p: sum(inside_ring(p, r) for r in rings) % 2 == 1
    out = []
    for r in rings:
        n = len(r)
        nrm = []
        for i in range(n):
            tx, ty = r[(i + 1) % n][0] - r[i - 1][0], r[(i + 1) % n][1] - r[i - 1][1]
            ln = math.hypot(tx, ty) or 1
            nrm.append((-ty / ln, tx / ln))
        m = n // 3
        if not inside((r[m][0] + nrm[m][0] * 1e-2 * step, r[m][1] + nrm[m][1] * 1e-2 * step)):
            nrm = [(-x, -y) for x, y in nrm]            # normals point into the letter
        spine = []
        for p, (nx, ny) in zip(r, nrm):                 # shrinking ball: largest inscribed circle at p
            best = 1e9
            for q in every:
                dx, dy = q[0] - p[0], q[1] - p[1]
                den = 2 * (dx * nx + dy * ny)
                if den > 1e-9:
                    best = min(best, (dx * dx + dy * dy) / den)
            spine.append((p[0] + nx * best, p[1] + ny * best))
        for i in range(n):
            j = (i + 1) % n
            nx, ny = nrm[i][0] + nrm[j][0], nrm[i][1] + nrm[j][1]
            ln = math.hypot(nx, ny) or 1
            v = (nx * LIGHT[0] + ny * LIGHT[1]) / ln    # = -(outward normal . light)
            out.append(([r[i], r[j], spine[j], spine[i]], (v + 1) / 2))
    return out


def runs(fs, keep):
    """Merge consecutive facets whose tone passes `keep` (a function of the tone) into strips."""
    strips, cur, cur_k = [], [], None
    for quad, t in fs:
        k = keep(t)
        if k is not None and cur and k == cur_k and math.dist(cur[-1][1], quad[0]) < 1e-6:
            cur.append(quad)
            continue
        if cur:
            strips.append((cur, cur_k))
        cur, cur_k = ([quad], k) if k is not None else ([], None)
    if cur:
        strips.append((cur, cur_k))
    return [([q[0] for q in s] + [s[-1][1], s[-1][2]] + [q[3] for q in reversed(s)], k) for s, k in strips]


def rdp(pts, eps):
    if len(pts) < 3:
        return pts
    (ax, ay), (bx, by) = pts[0], pts[-1]
    dx, dy = bx - ax, by - ay
    ln = math.hypot(dx, dy) or 1e-9
    dist = [abs(dy * x - dx * y + bx * ay - by * ax) / ln for x, y in pts[1:-1]]
    i = max(range(len(dist)), key=dist.__getitem__) + 1
    if dist[i - 1] > eps:
        return rdp(pts[:i + 1], eps)[:-1] + rdp(pts[i:], eps)
    return [pts[0], pts[-1]]


def poly_d(pts, ox=0.0, oy=0.0):
    return 'M' + 'L'.join(f"{x - ox:.1f} {y - oy:.1f}" for x, y in pts) + 'Z'

# ---------- app icon ----------

def hexrgb(h):
    return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))


def mix(c1, c2, t):
    return '#%02x%02x%02x' % tuple(round(c1[k] + (c2[k] - c1[k]) * t) for k in range(3))


def squircle(c=512, r=412, n=5.0, steps=720):
    """macOS icon body: superellipse, 824 wide on the 1024 canvas."""
    pts = []
    for i in range(steps):
        a = 2 * math.pi * i / steps
        x, y = math.cos(a), math.sin(a)
        pts.append((c + r * math.copysign(abs(x) ** (2 / n), x), c + r * math.copysign(abs(y) ** (2 / n), y)))
    return poly_d(pts)


def cut(contours, step, dark, light, levels=96):
    d = []
    for pts, k in runs(facets(contours, step), lambda t: round(t * levels)):
        col = mix(dark, light, k / levels)
        d.append(f'<path d="{poly_d(pts)}" fill="{col}" stroke="{col}" stroke-width="1.2" stroke-linejoin="round"/>')
    return '\n    '.join(d)


def icon_svg(H, shift, bronze, minium, stone, grain, lip, proportions):
    outer, counter, dot = letter(H, 512, 512, shift, **proportions)
    body = squircle()
    ld, dd = path_d(outer) + path_d(counter), path_d(dot)
    rnd = random.Random(7)
    specks = []
    for _ in range(grain):      # peperino: dark grains and a few pale ones
        x, y, r = rnd.uniform(100, 924), rnd.uniform(100, 924), rnd.choice([1.2, 1.6, 2.0, 2.6, 3.4])
        if rnd.random() < 0.55:
            specks.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r}" fill="#141512" fill-opacity="{rnd.uniform(.18, .38):.2f}"/>')
        else:
            specks.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r * 0.8:.1f}" fill="#c9cbbf" fill-opacity="{rnd.uniform(.05, .12):.2f}"/>')
    edge = lambda d: (f'<path d="{d}" fill-rule="evenodd" fill="#000000" fill-opacity="0.28" transform="translate(0 -3)"/>'
                      f'<path d="{d}" fill-rule="evenodd" fill="#ffffff" fill-opacity="0.10" transform="translate(0 3)"/>') if lip else ''
    bd, bl = map(hexrgb, bronze)
    md, ml = map(hexrgb, minium)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
  <!-- Dixit app icon, generated by dixit-mark.py: Roman capital D and triangular interpunct, V-cut in
       bronze and minium into peperino. macOS grid: 824 superellipse body, transparent margins. -->
  <defs>
    <linearGradient id="stone" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="{stone[0]}"/>
      <stop offset="1" stop-color="{stone[1]}"/>
    </linearGradient>
    <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.16"/>
      <stop offset="0.12" stop-color="#ffffff" stop-opacity="0"/>
      <stop offset="0.88" stop-color="#000000" stop-opacity="0"/>
      <stop offset="1" stop-color="#000000" stop-opacity="0.22"/>
    </linearGradient>
    <clipPath id="body"><path d="{body}"/></clipPath>
    <filter id="shadow" x="-10%" y="-10%" width="120%" height="125%">
      <feDropShadow dx="0" dy="10" stdDeviation="12" flood-color="#000000" flood-opacity="0.30"/>
    </filter>
  </defs>
  <path d="{body}" fill="url(#stone)" filter="url(#shadow)"/>
  <g clip-path="url(#body)">{''.join(specks)}</g>
  <path d="{body}" fill="none" stroke="url(#rim)" stroke-width="6" clip-path="url(#body)"/>
  {edge(ld)}
  <path d="{ld}" fill-rule="evenodd" fill="{mix(bd, bl, 0.5)}"/>
  <g>
    {cut([outer, counter], 3.0, bd, bl)}
  </g>
  {edge(dd)}
  <path d="{dd}" fill="{mix(md, ml, 0.5)}"/>
  <g>
    {cut([dot], 2.0, md, ml)}
  </g>
</svg>
'''

# ---------- menu bar and UI ----------

TRAY = dict(S=0.25, A=0.17, B=0.27, Ls=0.10, Ts=0.03, Br=0.13, side=0.21, gap=0.09)   # heavier for 18 pt
UI = dict(S=0.23, A=0.15, B=0.25, side=0.19, gap=0.09)


def tray_svg(size=44, H=31.0):
    outer, counter, dot = letter(H, 0, 0, **TRAY)
    x0, y0, x1, y1 = bbox(outer, dot)
    ox, oy = (x0 + x1 - size) / 2, (y0 + y1 - size) / 2
    d = path_d(outer, ox, oy) + path_d(counter, ox, oy) + path_d(dot, ox, oy)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" width="{size}" height="{size}">
  <!-- macOS menu-bar template: black + alpha only, the system tints it. Dixit mark D·, generated by dixit-mark.py. -->
  <path fill="#000000" fill-rule="evenodd" d="{d}"/>
</svg>
'''


def ui_mark(H=100.0):
    """Silhouette, mid and lit facets for DixitMark.tsx (viewBox 0 0 128 100, currentColor)."""
    outer, counter, dot = letter(H, 0, 0, **UI)
    x0, y0, x1, y1 = bbox(outer, dot)
    sil = path_d(outer, x0, y0) + path_d(counter, x0, y0) + path_d(dot, x0, y0)
    fs = facets([outer, counter], 0.8) + facets([dot], 0.5)
    band = lambda t: 2 if t >= 0.62 else 1 if t >= 0.38 else None
    mid, lit = '', ''
    for pts, k in runs(fs, band):
        d = poly_d(rdp(pts[:len(pts) // 2], 0.12) + rdp(pts[len(pts) // 2:], 0.12), x0, y0)
        if k == 2:
            lit += d
        else:
            mid += d
    return (x1 - x0, y1 - y0), sil, mid, lit

# ---------- rasters ----------

def run(*cmd, cwd=None):
    subprocess.run(cmd, check=True, cwd=cwd)


def svg_png(svg, size, out):
    run('rsvg-convert', '-w', str(size), '-h', str(size), str(svg), '-o', str(out))


def write_ico(pngs, out):
    """ICO with PNG-compressed entries (Vista+), stdlib only."""
    blobs = [(s, Path(p).read_bytes()) for s, p in pngs]
    head = struct.pack('<HHH', 0, 1, len(blobs))
    offset = 6 + 16 * len(blobs)
    entries, data = b'', b''
    for s, b in blobs:
        entries += struct.pack('<BBBBHHII', s % 256, s % 256, 0, 0, 1, 32, len(b), offset + len(data))
        data += b
    Path(out).write_bytes(head + entries + data)


def build():
    main, small, tray = HERE / 'dixit.svg', HERE / 'dixit-small.svg', HERE / 'tray-template.svg'
    master = HERE / 'dixit-1024.png'
    svg_png(main, 1024, master)
    svg_png(tray, 36, ICONS / 'tray-template.png')  # tray-icon draws it at 18pt: 36px @2x stays pixel-sharp
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        # Tauri's own set (Windows/Linux/Store sizes, .ico); only the files this repo already ships.
        run('pnpm', 'exec', 'tauri', 'icon', str(master), '-o', str(tmp / 'tauri'), cwd=FRONTEND)
        for f in (tmp / 'tauri').iterdir():
            if f.is_file() and (ICONS / f.name).exists():
                shutil.copy2(f, ICONS / f.name)
        # macOS .icns with the hinted small sizes, also kept as loose iconset files.
        iconset = tmp / 'dixit.iconset'
        iconset.mkdir()
        for name, size, src in [('16x16', 16, small), ('16x16@2x', 32, small), ('32x32', 32, small),
                                ('32x32@2x', 64, main), ('128x128', 128, main), ('128x128@2x', 256, main),
                                ('256x256', 256, main), ('256x256@2x', 512, main), ('512x512', 512, main),
                                ('512x512@2x', 1024, main)]:
            svg_png(src, size, iconset / f'icon_{name}.png')
            shutil.copy2(iconset / f'icon_{name}.png', ICONS / f'icon_{name}.png')
        run('iconutil', '-c', 'icns', str(iconset), '-o', str(ICONS / 'icon.icns'))
        shutil.copy2(ICONS / 'icon.icns', ICONS / 'app_icon.icns')
        shutil.copy2(ICONS / 'icon.ico', ICONS / 'app_icon.ico')
        # Web copies
        for s, src in [(16, small), (32, small), (48, main), (64, main)]:
            svg_png(src, s, tmp / f'fav{s}.png')
        write_ico([(s, tmp / f'fav{s}.png') for s in (16, 32, 48, 64)], FRONTEND / 'src' / 'app' / 'favicon.ico')


def main():
    # The interpunct is bronze like the letter: minium is reserved for "Dixit is recording".
    bronze, small_bronze = ('#6f4f1f', '#e2bf7e'), ('#a17b3b', '#ecc987')
    (HERE / 'dixit.svg').write_text(icon_svg(440, 16, bronze, bronze, ('#4a4c45', '#2b2d29'), 520, True, {}))
    (HERE / 'dixit-small.svg').write_text(icon_svg(530, 10, small_bronze, small_bronze, ('#4f5149', '#2c2e2a'),
                                                    0, False, dict(side=0.2, gap=0.11)))
    (HERE / 'tray-template.svg').write_text(tray_svg())
    (w, h), sil, mid, lit = ui_mark()
    print(f'UI mark viewBox 0 0 {w:.0f} {h:.0f}\nSILHOUETTE {sil}\nMID {mid}\nLIT {lit}')
    if '--build' in sys.argv:
        build()


if __name__ == '__main__':
    main()
