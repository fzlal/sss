#!/usr/bin/env python3
"""Copy an extracted Super Slime Soccer data folder into assets/original.

Usage: python3 tools/import_original_assets.py ~/Downloads/sss/data

The imported sprites and backgrounds remain the property of their original
creator. They are kept outside the MIT-licensed source and are intended as a
local development asset pack that can be replaced later.
"""
from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "original"


def normalize(name: str) -> str:
    name = name.lower().replace(" ", "_")
    name = name.replace("(2)", "").replace("__", "_")
    return name.strip("_")


def main(src: Path) -> None:
    (OUT / "backgrounds").mkdir(parents=True, exist_ok=True)
    (OUT / "sprites").mkdir(parents=True, exist_ok=True)

    for folder in ("backgrounds", "sprites"):
        for path in sorted((src / folder).iterdir()):
            if path.suffix.lower() not in {".png", ".gif"}:
                continue
            if path.suffix.lower() == ".gif":
                continue
            target = OUT / folder / normalize(path.stem)
            target = target.with_suffix(".png")
            Image.open(path).convert("RGBA").save(target)
            print("imported", target.relative_to(ROOT))

    font = Image.open(src / "sprites" / "font.gif")
    frames = []
    metrics = []
    for i in range(font.n_frames):
        font.seek(i)
        frame = font.convert("RGBA")
        px = frame.load()
        for y in range(frame.height):
            for x in range(frame.width):
                r, g, b, a = px[x, y]
                if r > 200 and g < 60 and b < 60:
                    px[x, y] = (r, g, b, 0)
        bbox = frame.getbbox()
        metrics.append(
            {
                "char": chr(33 + i) if i < 90 else " ",
                "index": i,
                "bbox": list(bbox) if bbox else None,
            }
        )
        frames.append(frame)
    atlas = Image.new("RGBA", (font.width * font.n_frames, font.height), (0, 0, 0, 0))
    for i, frame in enumerate(frames):
        atlas.alpha_composite(frame, (i * font.width, 0))
    atlas.save(OUT / "sprites" / "font_atlas.png")
    (OUT / "sprites" / "font_metrics.json").write_text(json.dumps(metrics, indent=1))

    ini = src / "data.ini"
    if ini.exists():
        shutil.copyfile(ini, OUT / "data.ini")
    print("font atlas:", atlas.size)


if __name__ == "__main__":
    main(Path(sys.argv[1] if len(sys.argv) > 1 else "~/Downloads/sss/data").expanduser())
