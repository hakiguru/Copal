#!/usr/bin/env python3
"""Writes the small test models in this folder (our own, public domain / CC0).

orientation-F.stl: a flat letter F, 30 x 40 x 3 mm. It looks different when mirrored in X or Y,
so slicing it in two slicers and comparing a layer shows if both write the image the same way.
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
