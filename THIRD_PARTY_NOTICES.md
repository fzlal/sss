# Third-party notices

This project is an unofficial fan remake. It is not affiliated with or endorsed
by the original creators or rights holders of **Super Slime Soccer**.

## Source code

Copyright (c) 2026 The slime-soccer contributors.

The Rust source code, build scripts, packaging files and the icon in
`packaging/icons/` are licensed under the MIT License. The full text is in
[LICENSE](LICENSE).

## Original Super Slime Soccer artwork

Copyright (c) 2011 the original creators of Super Slime Soccer.
**All rights reserved.**

Files covered by this notice:

- everything under `assets/original/backgrounds/`
- everything under `assets/original/sprites/`
- any extracted data or font atlas derived from the original game
- the original game's `data.ini` if present in a working copy

These files are **not** part of the MIT-licensed Software and **no license is
granted** for them. They are included in this repository and in release builds
to develop and preserve a fan remake of the original game. No permission to
redistribute, publish, sell or use them in a released build has been granted by
the rights holder. Anyone who publishes this repository or builds containing
these files is responsible for obtaining the necessary permission. If you are a
rights holder and want the material removed, contact the repository owner.

The name **Super Slime Soccer** and all associated original artwork remain the
property of their respective rights holders. See [ASSETS.md](ASSETS.md) and
[assets/original/COPYRIGHT.md](assets/original/COPYRIGHT.md).

## Fonts — SIL Open Font License 1.1

The following font files are distributed under the SIL Open Font License 1.1.
The complete license texts are stored next to the fonts:

| File | Copyright | License text |
| --- | --- | --- |
| `assets/fonts/PressStart2P-Regular.ttf` | Copyright 2012 The Press Start 2P Project Authors | `assets/fonts/OFL-PressStart2P.txt` |
| `assets/fonts/VT323-Regular.ttf` | Copyright 2011 The VT323 Project Authors | `assets/fonts/OFL-VT323.txt` |
| `assets/fonts/IBMPlexSansArabic-Regular.ttf` | Copyright (c) 2017 IBM Corp., Reserved Font Name "Plex" | `assets/fonts/OFL-IBMPlexSansArabic.txt` |

The fonts are embedded in the binary for the game's interface and Persian
localization; they are not derived from the original game artwork.

## Audio

All music and sound effects are synthesized at runtime by `src/audio.rs`. No
third-party audio files are bundled.

## Rust dependencies

The game links third-party Rust crates listed in `Cargo.lock` (macroquad,
serde, postcard, dirs and their transitive dependencies). Each crate remains
under its own license, typically MIT or Apache-2.0. Run `cargo metadata` or
inspect the packaged crates for the exact license of each dependency.

## System libraries

Release builds dynamically use platform graphics, input and audio libraries
(for example OpenGL, X11, ALSA, CoreAudio and the Windows multimedia APIs).
Those libraries are part of the operating system or distribution and are not
redistributed by this project.
