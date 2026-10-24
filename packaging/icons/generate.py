#!/usr/bin/env python3
"""Generate the MIT-licensed project icon set.

Only the PNG/ICO files are needed at build time; this script documents how
they were produced and can regenerate them:

    python3 -m pip install pillow
    python3 packaging/icons/generate.py
"""

from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

OUT = Path(__file__).resolve().parent
SCALE = 3
SIZE = 1024
CANVAS = SIZE * SCALE


def sc(value: float) -> int:
    return int(round(value * SCALE))


def point(x: float, y: float) -> tuple[int, int]:
    return sc(x), sc(y)


def box(x0: float, y0: float, x1: float, y1: float) -> tuple[int, int, int, int]:
    return sc(x0), sc(y0), sc(x1), sc(y1)


def gradient_background() -> Image.Image:
    top = (18, 32, 52)
    bottom = (6, 12, 22)
    image = Image.new("RGB", (CANVAS, CANVAS), bottom)
    draw = ImageDraw.Draw(image)
    for y in range(CANVAS):
        t = y / (CANVAS - 1)
        color = tuple(int(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
        draw.line([(0, y), (CANVAS, y)], fill=color)

    mask = Image.new("L", (CANVAS, CANVAS), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        box(28, 28, 996, 996), radius=sc(220), fill=255
    )
    rounded = Image.new("RGB", (CANVAS, CANVAS), (0, 0, 0))
    rounded.paste(image, (0, 0), mask)

    glow = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.ellipse(box(80, 110, 944, 920), fill=(46, 110, 80, 105))
    glow = glow.filter(ImageFilter.GaussianBlur(sc(70)))
    rounded = Image.alpha_composite(rounded.convert("RGBA"), glow)
    rounded.putalpha(mask)
    return rounded


def slime_layer() -> Image.Image:
    layer = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)

    shadow = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    sd.ellipse(box(130, 846, 894, 974), fill=(0, 0, 0, 135))
    shadow = shadow.filter(ImageFilter.GaussianBlur(sc(28)))
    layer = Image.alpha_composite(layer, shadow)
    draw = ImageDraw.Draw(layer)

    cx, base, rx, ry = 512.0, 902.0, 382.0, 436.0
    outline: list[tuple[int, int]] = []
    for index in range(181):
        theta = math.pi + math.pi * index / 180.0
        outline.append(point(cx + rx * math.cos(theta), base + ry * math.sin(theta)))
    outline += [point(cx + rx, base), point(cx - rx, base)]

    draw.polygon(outline, fill=(103, 190, 58, 255))
    draw.line(outline[:-2] + [outline[0]], fill=(44, 108, 34, 255), width=sc(10), joint="curve")

    highlight = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    hd = ImageDraw.Draw(highlight)
    hd.ellipse(box(214, 480, 570, 676), fill=(167, 238, 103, 120))
    highlight = highlight.filter(ImageFilter.GaussianBlur(sc(24)))
    layer = Image.alpha_composite(layer, highlight)
    draw = ImageDraw.Draw(layer)

    for eye_x in (398, 626):
        draw.ellipse(box(eye_x - 58, 628, eye_x + 58, 772), fill=(250, 253, 244, 255))
        draw.ellipse(
            box(eye_x - 15, 666, eye_x + 39, 733), fill=(29, 34, 48, 255)
        )
        draw.ellipse(box(eye_x + 11, 676, eye_x + 25, 692), fill=(255, 255, 255, 255))

    draw.arc(box(448, 764, 578, 846), start=18, end=162, fill=(35, 88, 30, 255), width=sc(15))

    draw.ellipse(box(752, 752, 916, 916), fill=(244, 247, 250, 255), outline=(35, 41, 56, 255), width=sc(8))
    pentagon = []
    for index in range(5):
        angle = -math.pi / 2 + 2 * math.pi * index / 5
        pentagon.append(point(834 + 58 * math.cos(angle), 834 + 58 * math.sin(angle)))
    draw.polygon(pentagon, fill=(30, 36, 50, 255))
    for angle in (-0.25, 2.1, 4.4):
        x = 834 + 116 * math.cos(angle)
        y = 834 + 116 * math.sin(angle)
        draw.ellipse(box(x - 28, y - 24, x + 28, y + 24), fill=(30, 36, 50, 255))
    draw.ellipse(box(782, 784, 832, 821), fill=(255, 255, 255, 210))
    return layer


def main() -> None:
    image = gradient_background()
    image = Image.alpha_composite(image, slime_layer())
    image = image.resize((SIZE, SIZE), Image.Resampling.LANCZOS)

    OUT.mkdir(parents=True, exist_ok=True)
    image.save(OUT / "slime-soccer-1024.png", optimize=True)
    image.resize((512, 512), Image.Resampling.LANCZOS).save(
        OUT / "slime-soccer-512.png", optimize=True
    )
    image.resize((256, 256), Image.Resampling.LANCZOS).save(
        OUT / "slime-soccer-256.png", optimize=True
    )

    ico_source = image.resize((256, 256), Image.Resampling.LANCZOS)
    ico_source.save(
        OUT / "slime-soccer.ico",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )


if __name__ == "__main__":
    main()
