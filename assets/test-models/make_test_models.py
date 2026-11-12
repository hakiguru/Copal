#!/usr/bin/env python3
"""Writes the small test models in this folder (our own, public domain / CC0).

orientation-F.stl: a flat letter F, 30 x 40 x 3 mm. It looks different when mirrored in X or Y,
so slicing it in two slicers and comparing a layer shows if both write the image the same way.

case.stl: a small suitcase, 90 x 28 x 62 mm, with a handle, latches, corner guards and ribs.
A bulky part with thin details: the test model for orientation, supports, hollowing and drain holes.
"""
import struct


def box(x0, y0, z0, x1, y1, z1):
    c = [(x, y, z) for z in (z0, z1) for y in (y0, y1) for x in (x0, x1)]
    # Corner index: bit 0 = x, bit 1 = y, bit 2 = z. Faces wound counter-clockwise from outside.
    quads = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    tris = []
    for a, b, cc, d in quads:
        tris += [(c[a], c[b], c[cc]), (c[a], c[cc], c[d])]
    return tris


def write_stl(path, tris):
    with open(path, "wb") as f:
        f.write(b"Copal test model".ljust(80, b"\0"))
        f.write(struct.pack("<I", len(tris)))
        for t in tris:
            f.write(struct.pack("<3f", 0, 0, 0))
            for v in t:
                f.write(struct.pack("<3f", *v))
            f.write(b"\0\0")


if __name__ == "__main__":
    h = 3.0
    letter_f = box(0, 0, 0, 6, 40, h) + box(6, 34, 0, 24, 40, h) + box(6, 17, 0, 16, 23, h)
    write_stl("orientation-F.stl", letter_f)

    # The suitcase: X is its length, Y its thickness, Z its height. Parts overlap a little, so
    # the union is one solid.
    case = box(-45, -14, 0, 45, 14, 62)
    # The seam between the two shells, and two ribs on each big face.
    case += box(-45.6, -1, 0.5, 45.6, 1, 62.6)
    for y0, y1 in ((-14.8, -13.5), (13.5, 14.8)):
        for x in (-30, 27):
            case += box(x, y0, 3, x + 3, y1, 59)
    # Corner guards.
    for sx in (-1, 1):
        for sz in (0, 1):
            x0, x1 = sorted((sx * 46, sx * 38))
            z0, z1 = (-1, 7) if sz == 0 else (55, 63)
            case += box(x0, -15, z0, x1, 15, z1)
    # The handle: two posts and a bar over them.
    case += box(-14, -3, 61.5, -10, 3, 69) + box(10, -3, 61.5, 14, 3, 69)
    case += box(-16, -3.5, 68, 16, 3.5, 72)
    # Two latches that stand out from the top.
    for x in (-28, 24):
        case += box(x, -6, 61.5, x + 5, 6, 64) + box(x + 1, 5, 58, x + 4, 15.5, 63)
    write_stl("case.stl", case)
