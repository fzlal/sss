# Packaging

The zip/tar/installer files are built from an already compiled release binary;
the CI workflow handles compilation and calls these scripts. They can also be
run locally.

## Common requirements

- Rust stable with the target toolchain installed.
- A release binary:
  - Linux: `cargo build --release --locked`
  - Windows: `cargo build --release --locked`
  - macOS universal: build `aarch64-apple-darwin` and `x86_64-apple-darwin`,
    then combine them with `lipo -create`.

The version passed to every script is the version from `Cargo.toml` without a
leading `v`.

## Linux

```sh
FORMATS=tar,deb,rpm,appimage \
  packaging/linux/package.sh 0.1.0 x86_64 \
  target/release/slime-soccer dist
```

Produced files:

- `slime-soccer-VERSION-linux-ARCH.tar.gz`
- `slime-soccer_VERSION_amd64.deb` (or `_arm64.deb`)
- `slime-soccer-VERSION-1.x86_64.rpm` (or `.aarch64.rpm`)
- `slime-soccer-VERSION-linux-ARCH.AppImage`

Requirements: `dpkg-deb`, `rpmbuild`, `file` and `curl` for the formats you
enable, plus `desktop-file-validate` optionally. The AppImage step downloads
the official architecture-specific `appimagetool` from the AppImage project.
The AppImage contains the executable, icons and documentation but relies on
the host's OpenGL, X11 and ALSA libraries; it is not a fully bundled runtime.
Set `FORMATS` to skip formats whose tools are not installed.

## Windows

```powershell
./packaging/windows/package.ps1 -Version 0.1.0 -Arch x86_64 `
  -Binary target/x86_64-pc-windows-msvc/release/slime-soccer.exe `
  -Dist dist
```

Produced files:

- `slime-soccer-VERSION-windows-ARCH.zip` — portable build with a
  `portable.txt` marker that keeps `save.json` next to the executable
- `slime-soccer-VERSION-windows-ARCH-setup.exe` (requires NSIS `makensis`)

Use `-SkipInstaller` to produce only the portable zip (the ARM64 CI job does
this because NSIS is an x86 tool). The portable zip must be extracted to a
writable folder. The installer deliberately omits `portable.txt`, is per-user,
installs under `%LOCALAPPDATA%\Programs\Slime Soccer`, and does not require
administrator rights; it stores saves in `%APPDATA%\slime-soccer`.

## macOS

```sh
packaging/macos/package.sh 0.1.0 target/universal/slime-soccer dist
```

Produced files:

- `slime-soccer-VERSION-macos-universal.dmg`
- `slime-soccer-VERSION-macos-universal.tar.gz`

Requirements: `sips`, `iconutil`, `codesign`, `hdiutil` and `lipo` from the
Xcode command line tools. The app is ad-hoc signed (`codesign -s -`) so it can
run on Apple Silicon; it is not notarized. Set `CODESIGN_IDENTITY` to a
Developer ID identity for a signed local build, but notarization still needs
Apple credentials and is outside this script.

## Icons

`packaging/icons/generate.py` redraws the application icon from scratch with
Pillow and writes the PNG sizes used by Linux and macOS plus the Windows ICO.
The generated files are committed; rerun the script only when the artwork
changes:

```sh
python3 packaging/icons/generate.py
```

The icon is an original drawing for this remake and is not part of the
proprietary asset pack.

## CI

`.github/workflows/ci.yml` runs formatting, clippy and the test suite on Linux,
Windows and macOS for pushes to `main` and pull requests.

`.github/workflows/release.yml` builds the complete matrix and uploads each
platform's files as artifacts. Pushing a tag such as `v0.1.0` (matching
`Cargo.toml`) downloads all artifacts, creates `SHA256SUMS.txt` and publishes
a GitHub Release. `workflow_dispatch` performs the same builds without
creating a release.
