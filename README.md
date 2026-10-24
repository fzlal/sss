# Super Slime Soccer — Rust remake

An open-source, cross-platform remake of the 2011 game **Super Slime Soccer**,
written in Rust with [macroquad](https://macroquad.rs/). It recreates the
original arcade football plus the volleyball and basketball variants, the full
slime roster, arenas, abilities, mana system and championship run.

The original game's artwork is included in this repository and in release
builds. It is **not** MIT-licensed: copyright remains with the original
creators, no license is granted, and publishing builds containing it may
require permission. See [ASSETS.md](ASSETS.md) and
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) before publishing or
redistributing anything.

## Features

- **Three sports** with separate physics and scoring:
  - **Soccer** — goals, crossbars, net rebounds and own goals.
  - **Volleyball** — centre net, floor scoring and a serve toss.
  - **Basketball** — rims, backboards and downward hoop sensors.
- **35 slimes**, each with its own stats and special ability (rockets,
  boomerangs, black holes, clones, trees, weather, teleports and more).
- **11 arenas**, from the classic pitch to the moon, disco, underwater and a
  runner track, each with its own ambient particle effect.
- **Three mana modes** — Bonus, Offensive and Round — plus per-ability mana
  costs and cooldowns.
- **Quick match** against the CPU, local two-player, or a three-round
  **championship** that ends in a cup (a loss ends the run).
- **Three AI difficulties** with reaction time, prediction, jumping skill,
  mistake rate and ability usage tuned separately.
- **English and Persian** interfaces, with right-to-left text shaping and
  bidi handling; English is the default and the language can be switched at
  runtime under **Settings → Language**.
- Pause, screen shake, pixel HUD, match statistics and persistent
  settings/progress.
- **Online and LAN play**: host from the menu or run a headless dedicated
  server, join by direct IP or automatic LAN discovery, with a
  host-authoritative 120 Hz match and a live ping/loss readout. See
  [Online and LAN play](#online-and-lan-play).

## Building

You need a Rust toolchain (2021 edition; developed with Rust 1.98) and Cargo.

```sh
cargo run --release
```

The release binary is written to `target/release/slime-soccer`.

> **Asset requirement:** the artwork is embedded with `include_bytes!`, so the
> repository must contain `assets/original/` to compile. See
> [ASSETS.md](ASSETS.md) for provenance and for how to import or replace it.

### Platform notes

- **Linux** — needs an OpenGL-capable X11/Wayland session and the ALSA runtime.
  On Debian/Ubuntu, `libasound2`, `libx11-6` and `libgl1` are usually enough.
  `build.rs` links directly against `libasound.so.2` when the development
  symlink from `libasound2-dev` is missing, so a plain desktop install can
  build without extra `-dev` packages.
- **Windows** — build with the MSVC toolchain.
- **macOS** — builds with the default Apple toolchain; the app is not sandboxed.
- **WebAssembly** — not wired up yet. The simulation core has no graphics
  dependencies, so a browser build is a possible follow-up once the asset
  loading and input layers are adapted.

## Builds and releases

Every `v*.*.*` tag builds release artifacts for desktop platforms and
publishes a GitHub Release; `workflow_dispatch` builds the same artifacts
without publishing. The tag must match the version in `Cargo.toml` (`v0.1.0`
for version `0.1.0`). To cut a release:

```sh
git tag v0.1.0
git push origin main --tags
```

| Target | Packages |
| --- | --- |
| Linux x86_64 / aarch64 | `.tar.gz`, `.deb` (`amd64`/`arm64`), `.rpm` (`x86_64`/`aarch64`), `.AppImage` |
| Windows x86_64 / i686 | portable `.zip` and an NSIS setup `.exe` |
| Windows ARM64 | portable `.zip` |
| macOS universal (Intel + Apple Silicon) | `.dmg` and `.tar.gz` |

Linux builds run on Ubuntu 22.04 (x86_64 and ARM) and require glibc 2.35 or
newer. macOS builds are universal and ad-hoc signed but **not notarized**;
Windows builds have no Authenticode certificate. Gatekeeper/SmartScreen
therefore shows a warning on first launch. Adding signing certificates is
possible later without changing the packaging layout.

The build matrix uses GitHub-hosted `windows-11-arm` for Windows ARM64 and
Arm64 Linux runners, which require a public repository or a plan that provides
those runners; remove the corresponding matrix entry if they are unavailable.
Release artifacts are attached with a `SHA256SUMS.txt` checksum file.

The release workflow packages the original artwork because it is embedded in
the executable. Read the copyright notice above before publishing a release.

## Controls

| Action | Player 1 | Player 2 |
| --- | --- | --- |
| Move | Left / Right arrows | A / D |
| Jump | Up arrow | W |
| Ability | Space | Left Ctrl |

Menus: arrow keys or WASD to navigate, Enter/Space to confirm, Escape to go
back. The mouse works in all menus. Bindings can be changed under
**Settings → Control Bindings**.

The interface starts in English. Switch it to Persian under
**Settings → Language** (confirm the row, or press Left/Right on it); the
choice is stored in the save file and restored on the next launch.

In a match: **Esc** pauses, **Q** (while paused) abandons to the menu, **M**
toggles mute, **F11** toggles fullscreen. In an online match, **Q** leaves to
the online menu and the server continues the match while you have the local
pause open.

## Rules

- **Soccer:** first to the chosen score wins. The ball bounces off walls,
  crossbars and slimes; own goals award the point to the opponent.
- **Volleyball:** keep the ball off your floor. The ball starts as a toss on
  the serving side and must be hit over the net; the floor on a side awards the
  point to the other side.
- **Basketball:** put the ball down through the opponent's hoop. Rims and
  backboards are solid; a basket in your own hoop awards the point to the
  opponent.
- **Mana:** abilities cost mana, and pushing into the opponent's half charges
  it faster. Bonus regenerates everywhere at three times the rate in the
  opponent's half, Offensive charges only in the opponent's half, and Round
  regenerates slowly, faster in the opponent's half, and refills after every
  point.
- **Championship:** choose a slime, sport, arena and difficulty, then beat
  three opponents in a row. A single loss ends the run; three wins earn a cup.

All match settings (sport, arena, both slimes, score target, mana mode,
difficulty, local/CPU opponent) are chosen in the setup screen and remembered
for the next session.

## Online and LAN play

Online play is available from **ONLINE** in the main menu:

- **HOST GAME** opens the setup screen with online-host labels. Pick the sport,
  arena, your slime, score target and mana mode, then **HOST MATCH**. The lobby
  shows the address to share; the joiner picks their own slime, and the match
  starts when they connect.
- **LAN GAMES** lists hosts found automatically on the local network over UDP
  broadcast. Use **JOIN BY IP** when broadcast is blocked or the host uses a
  custom port.
- **JOIN BY IP** accepts `host:port` or a bare host/IP (`43210` is the default
  port), then asks for the joiner's slime.

The same modes are available from the command line:

```sh
# Headless dedicated server for two remote players
slime-soccer --server --port 43210 --sport soccer --play-to 6 --name "Friday Cup"

# Open the game directly in a host lobby, or connect to a host
slime-soccer --host --port 43210
slime-soccer --connect 192.168.1.20:43210 --name CHALLENGER

slime-soccer --help
```

The host (or `--server` process) runs the authoritative simulation at 120 Hz;
clients send input at 60 Hz and receive full snapshots at 30 Hz, interpolated
for rendering. Input redundancy hides occasional packet loss. In an online
match **Esc** pauses locally (the server keeps running), **Q** while paused
leaves to the online menu, and the HUD shows ping and snapshot loss.

**Current limits:** plain UDP with no encryption or authentication, no rollback
prediction (input latency follows the round trip), and no NAT traversal or
relay. LAN play needs no configuration; internet play requires forwarding the
UDP port. All players must run the same build. Details, message formats and the
roadmap are in [ONLINE_PLAY.md](ONLINE_PLAY.md).

## Saving

Settings, controller bindings and career progress are stored as JSON in the
platform data directory:

- Linux: `~/.local/share/slime-soccer/save.json`
- macOS: `~/Library/Application Support/slime-soccer/save.json`
- Windows: `%APPDATA%\slime-soccer\save.json`

On Windows, the portable zip includes a `portable.txt` marker: while that file
sits next to `slime-soccer.exe`, the game writes `save.json` in the same folder
so settings and progress travel with the extracted copy. Remove `portable.txt`
to use `%APPDATA%\slime-soccer\save.json` again. The NSIS installer does not
include the marker. Extract the zip to a writable folder, since the game cannot
save next to an executable in a read-only location.

A corrupt or unreadable save falls back to defaults. Delete the file to reset
all progress and settings.

## Project layout

```
src/
  game/        deterministic simulation, AI, characters, arenas, abilities
  render/      macroquad renderer and virtual 560x400 canvas
  ui/          title, online, setup, roster, arena, settings, controls, results
  net/         UDP protocol, host server, client, LAN discovery and CLI flags
  i18n.rs      English/Persian localization, Arabic shaping and bidi
  save.rs      JSON persistence
  audio.rs     synthesized chiptune music and sound effects
  assets.rs    embedded texture and font loading
  app.rs       main loop, fixed timestep, pause, local and online sessions
tests/
  gameplay.rs  physics, scoring, abilities, determinism, arena safety
  ai.rs        AI balance and no-stall checks
  network.rs   real UDP loopback session: join, snapshots, movement, disconnect
  ui_flow.rs   casual results and championship progression
tools/
  import_original_assets.py
packaging/
  linux/       tar.gz, deb, rpm and AppImage packaging
  macos/       universal .app and .dmg packaging
  windows/     portable zip and NSIS installer
.github/
  workflows/   CI and tag-triggered release builds
THIRD_PARTY_NOTICES.md   code, artwork, font and dependency notices
```

## Simulation and determinism

The match runs at a fixed 120 Hz timestep independent of rendering. Inputs are
edge-triggered `InputFrame`s; the simulation owns its seedable RNG and contains
no graphics calls. Given the same setup, seed and input sequence, two runs
produce identical matches.

### Online play

The online session reuses the same fixed-step simulation on the host at
120 Hz, sends input frames from clients at 60 Hz with redundancy, and
broadcasts full postcard-encoded snapshots at 30 Hz. The host is
authoritative for physics, score and phase; clients interpolate between
snapshots. The wire protocol, LAN discovery, limitations and roadmap are
documented in [ONLINE_PLAY.md](ONLINE_PLAY.md).

## Testing

```sh
cargo test
```

The suite covers core physics, all three sports' scoring, walls and arenas,
ability activation, mana modes, deterministic replay, AI balance/no-stall
behavior, UI career flow, save round-tripping, audio buffer sanity, packet
serialization and a real UDP host/client loopback session.

## Licensing and credits

- **Source code:** MIT, see [LICENSE](LICENSE).
- **Original Super Slime Soccer artwork:** Copyright (c) 2011 the original
  creators, all rights reserved. It is **not** MIT, no license is granted, and
  publishing it may require permission. It is included in this repository and
  in the release artifacts because the binary embeds it. See
  [ASSETS.md](ASSETS.md) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
  Replace it with your own art to make a fully redistributable build.
- **Press Start 2P / VT323 / IBM Plex Sans Arabic fonts:** SIL Open Font
  License 1.1; license texts are in `assets/fonts/`.
- **Audio:** generated procedurally by the code.

This is a fan remake and is not affiliated with or endorsed by the creator of
the original game.
