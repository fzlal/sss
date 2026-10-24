# Assets

This project deliberately separates code from artwork so the source can be
published under MIT without relicensing the original game's art.

## Copyright notice

Copyright (c) 2011 the original creators of **Super Slime Soccer**.
**All rights reserved.**

The files under `assets/original/` are **not** part of the MIT-licensed
Software. No license or permission to redistribute them is granted by this
project. They are included in this repository and in release artifacts for the
development and preservation of this fan remake. Anyone publishing this
repository or builds containing these files is responsible for obtaining
permission from the rights holder. See
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and
[assets/original/COPYRIGHT.md](assets/original/COPYRIGHT.md).

## `assets/original/` — original Super Slime Soccer art (NOT MIT)

The PNG sprites, backgrounds, the extracted font atlas and the imported
`data.ini` in `assets/original/` come from **Super Slime Soccer** (2011) and
remain the property of its original creator. They are included here so the
remake can be compared faithfully with the original game.

- They are **not** covered by the repository's MIT license.
- No permission to redistribute, publish, sell or use them in a released build
  has been granted by the rights holder. Publishing binaries or sources that
  contain them may require permission.
- Release packages include this directory's content (embedded in the binary)
  together with the copyright notices; if you need a fully redistributable
  build, replace the artwork first.
- The code in `src/` is written against the filenames listed in
  `src/assets.rs`.

`tools/import_original_assets.py` rebuilds `assets/original/` from an extracted
copy of the original game's `data/` folder. It intentionally imports only the
art needed by the remake, converts everything to PNG, and generates a font
atlas plus glyph metrics. Pillow is required:

```sh
python3 -m pip install pillow
python3 tools/import_original_assets.py /path/to/sss/data
```

The font atlas in `assets/original/sprites/font_atlas*.png` is derived from the
original `font.gif` and is therefore under the same restriction as the rest of
`assets/original/`.

## Fonts — SIL Open Font License 1.1

The interface and pixel font files in `assets/fonts/` are third-party fonts
distributed under the SIL Open Font License 1.1:

- **Press Start 2P** — Copyright 2012 The Press Start 2P Project Authors.
  License: `assets/fonts/OFL-PressStart2P.txt`.
- **VT323** — Copyright 2011 The VT323 Project Authors.
  License: `assets/fonts/OFL-VT323.txt`.
- **IBM Plex Sans Arabic** — Copyright © 2017 IBM Corp., Reserved Font Name
  "Plex". License: `assets/fonts/OFL-IBMPlexSansArabic.txt`. The Regular
  weight is embedded in the binary and used only to render the Persian
  localization.

The Latin pixel fonts are not used as embedded replacements for the original
game's font; they are used for the remake's own menus and HUD. The Persian
font is a separate localization typeface and is not derived from the original
artwork.

## Audio

All music and sound effects are synthesized at runtime by `src/audio.rs`. No
audio files ship with the project, and the generated audio is covered by the
same MIT license as the code.

## Replacing the original art

To make the project fully redistributable:

1. Create replacement PNGs using the same file names and reasonable
   dimensions (backgrounds are drawn at 560x400; slimes can be any size and are
   anchored on their feet).
2. Replace the files in `assets/original/` or update the match tables in
   `src/assets.rs` to point at a new directory.
3. Update the copyright notices and remove `assets/original/` from any
   published source or binary archive once the replacement art is in place.

The simulation, menus, AI and audio do not depend on the original artwork.
