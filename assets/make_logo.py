"""Draws the Ambra logo (16x16 pixel crystal) and the wordmark (crystal + AMBRA).

Usage: python3 make_logo.py [output_dir]
Needs Pillow. All art is drawn here by hand, pixel by pixel.
"""
import sys
from PIL import Image

# Palette: saturated amber, from deep shadow to white glint.
P = {
    'o': (112, 32, 4),     # outline
    'D': (176, 58, 6),     # deep shadow
    'd': (214, 88, 10),    # shadow
    'M': (240, 122, 18),   # mid tone
    'L': (252, 158, 38),   # light
    'l': (255, 190, 72),   # lighter
    'H': (255, 222, 128),  # highlight
    'W': (255, 251, 228),  # glint
}

CRYSTAL = [
    "................",
    ".......oo.......",
    "......oHlo......",
    ".....oHWlLo.....",
    "....oHHWlLMo....",
    "...olHHHlLMdo...",
    "...oLLLlMMdDo...",
    "...oLLLlMMdDo...",
    "...oLLLlMMdDo...",
    "...oLLLlMMdDo...",
    "...oLLLlMMdDo...",
    "...oLLLlMMdDo...",
    "....oMLlMdDo....",
    ".....oMdDDo.....",
    "......odDo......",
    ".......oo.......",
]

# Chunky 9-px-tall letters ('#' = filled).
FONT = {
    'A': [".####.", "##..##", "##..##", "##..##", "######", "##..##", "##..##", "##..##", "##..##"],
    'M': ["##...##", "###.###", "#######", "##.#.##", "##...##", "##...##", "##...##", "##...##", "##...##"],
    'B': ["#####.", "##..##", "##..##", "##..##", "#####.", "##..##", "##..##", "##..##", "#####."],
    'R': ["#####.", "##..##", "##..##", "##..##", "#####.", "##.##.", "##..##", "##..##", "##..##"],
}
# Vertical gradient for letters, top to bottom.
ROW_SHADES = ['H', 'l', 'l', 'L', 'L', 'M', 'M', 'd', 'D']


def put(img, x, y, key):
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), P[key] + (255,))


def draw_grid(img, grid, ox=0, oy=0):
    for y, row in enumerate(grid):
        assert len(row) == len(grid[0]), f"row {y} has wrong width"
        for x, c in enumerate(row):
            if c != '.':
                put(img, ox + x, oy + y, c)


def draw_word(img, text, ox, oy):
    """Draws text with a shaded fill, glint, 1px outline and a drop shadow."""
    fill = {}
    x = ox
    for ch in text:
        glyph = FONT[ch]
        for gy, row in enumerate(glyph):
            for gx, c in enumerate(row):
                if c == '#':
                    fill[(x + gx, oy + gy)] = ROW_SHADES[gy]
        fill[(x, oy + (1 if glyph[0][0] == '.' else 0))] = 'W'  # glint on the top-left pixel
        x += len(glyph[0]) + 1
    outline = set()
    for (px, py) in fill:
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                if (px + dx, py + dy) not in fill:
                    outline.add((px + dx, py + dy))
    shadow = {(px + 1, py + 1) for (px, py) in outline} - outline - set(fill)
    for (px, py) in shadow:
        put(img, px, py, 'D')
    for (px, py) in outline:
        put(img, px, py, 'o')
    for (px, py), k in fill.items():
        put(img, px, py, k)
    return x


def sparkle(img, cx, cy, size):
    """Draws a 4-point star glint. size 1 = small (3x3 plus), size 2 = big (5x5 plus)."""
    put(img, cx, cy, 'W')
    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        if size == 1:
            put(img, cx + dx, cy + dy, 'L')
        else:
            put(img, cx + dx, cy + dy, 'H')
            put(img, cx + 2 * dx, cy + 2 * dy, 'L')


def draw_crystal(img, ox=0, oy=0):
    draw_grid(img, CRYSTAL, ox, oy)
    sparkle(img, ox + 2, oy + 2, 2)    # big glint, top left
    sparkle(img, ox + 13, oy + 12, 1)  # small glint, bottom right


def word_width(text):
    return sum(len(FONT[c][0]) + 1 for c in text) - 1


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else '.'

    icon = Image.new('RGBA', (16, 16), (0, 0, 0, 0))
    draw_crystal(icon)
    icon.save(f'{out}/logo-16.png')
    for s in (256, 512):
        icon.resize((s, s), Image.NEAREST).save(f'{out}/logo-{s}.png')

    text = 'AMBRA'
    w = 16 + 2 + word_width(text) + 3
    mark = Image.new('RGBA', (w, 16), (0, 0, 0, 0))
    draw_crystal(mark)
    end = draw_word(mark, text, 16 + 2, 3)
    sparkle(mark, end - 2, 2, 1)  # glint on the last letter
    mark.save(f'{out}/wordmark-1x.png')
    mark.resize((w * 8, 16 * 8), Image.NEAREST).save(f'{out}/wordmark.png')


if __name__ == '__main__':
    main()
