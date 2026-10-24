# Super Slime Soccer

A cross-platform Rust remake of the 2011 arcade game, with soccer, volleyball
and basketball, 35 slimes, 11 arenas, championships, local multiplayer and
online/LAN play.

## Downloads

| Platform | Files |
| --- | --- |
| Linux x86_64 / aarch64 | portable `.tar.gz`, `.deb`, `.rpm`, `.AppImage` |
| Windows x86_64 / i686 | portable `.zip` and per-user setup `.exe` |
| Windows ARM64 | portable `.zip` |
| macOS Intel + Apple Silicon | universal `.dmg` and `.tar.gz` |

Linux builds target glibc 2.35 or newer. Use the portable tarball or AppImage
if your distribution is older.

## Install notes

- **Linux:** install the `.deb` or `.rpm` with your package manager, or make the
  AppImage executable and run it. The game needs working OpenGL and audio
  libraries.
- **Windows:** run the setup executable, or extract the zip to a writable
  folder. The zip is a fully portable build: its `portable.txt` marker makes
  the game keep `save.json` next to the executable, so settings and progress
  travel with it. Remove `portable.txt` to use
  `%APPDATA%\slime-soccer\save.json` instead. The installer omits the marker
  and always uses `%APPDATA%`. Neither package is code-signed, so SmartScreen
  may ask for confirmation.
- **macOS:** open the `.dmg`, drag the app to Applications, then launch it.
  The app is ad-hoc signed but not notarized, so Gatekeeper may require
  right-clicking the app and choosing **Open** the first time.
- **Online play:** the host needs UDP port 43210 (or `--port`) reachable.
  LAN discovery uses UDP broadcast; use **JOIN BY IP** when broadcast is
  blocked. See `ONLINE_PLAY.md` in the repository for details.
- All players must run the same protocol version (the same release).

## Verify the download

Use the attached `SHA256SUMS.txt`:

```sh
sha256sum -c SHA256SUMS.txt
```

On Windows:

```powershell
Get-FileHash .\slime-soccer-*-windows-*.zip -Algorithm SHA256
```

## Assets and licensing

The source code is MIT licensed. The original **Super Slime Soccer** artwork
(Copyright (c) 2011 its original creators, all rights reserved) is embedded in
these binaries and is **not** MIT licensed. No license to redistribute it is
granted; publishing builds containing it may require permission from the rights
holder. See `THIRD_PARTY_NOTICES.md` and `ASSETS.md` in the repository. Fonts
are under the SIL Open Font License 1.1, and the audio is generated at runtime.

This is a fan remake and is not affiliated with or endorsed by the original
game's creators.
