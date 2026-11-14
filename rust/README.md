# ⚓ C02 Naval — native Windows version (Rust)

The full C02 Naval game, rewritten in Rust as a native Windows 11 x64 application (a single `.exe`, no browser, no installation).

## ⬇️ Download

**[Latest Windows version (releases page)](https://github.com/DRFRrobin/C02-Naval/releases)**: download `C02-Naval-windows-x64.exe` and run it.
The executable isn't signed: if Windows SmartScreen shows a warning, click "More info" then "Run anyway".

## Automatic updates (mandatory)

On every launch, the game checks the [GitHub releases](https://github.com/DRFRrobin/C02-Naval/releases) (tags ending in `-windows`, `.exe` file attached).

- No newer version: the game starts normally.
- Newer version: the update is **mandatory**. The game downloads it, replaces its own `.exe` and restarts, with no way to play the old version.
- No Internet (or GitHub not responding): the game starts with the current version.
- If the replacement fails (for example in a write-protected folder such as `Program Files`), a link to the releases page is shown.

To publish an update: raise `version` in `Cargo.toml`, build, then create a release tagged `vX.Y.Z-windows` with the `C02-Naval-windows-x64.exe` file attached.

## Features (same as the web version)

- **Login**: local accounts with username + password (PBKDF2-SHA-256, 120,000 iterations), profile photo (cropped to 160 × 160), guest mode.
- **Dashboard**: top bar (avatar, level, XP bar, rank), tabs Play / Statistics / Settings / Log out.
- **4 modes**: random opponent (bot after 45 s), game against the tactical bot, join with a code, host a private room (6-character code, copy the code or the invite link).
- **Placement**: preview then confirm, drag and drop, rotation (R key or right click), random, clear, Enter / Del shortcuts.
- **Game**: shot, **Radar** (2 × 2), **Cross strike** (5 squares), **Shield**, upgrade choice after each sunk ship, log, last enemy shot highlighted.
- **Career statistics**, XP and levels (Cabin boy → Admiral), XP halved against the bot.
- **Settings**: synthesized sound effects (no audio file), French / English language, change username / password / photo, delete the account.
- Keeps the screen awake during an online game; minimizing the window = "away" for the opponent (2 min).

## Compatible with the web version

Networking reproduces PeerJS 1.5 exactly: matchmaking through the public PeerJS service, a direct WebRTC link, BinaryPack messages split into 16,300-byte chunks. **A PC player can face a browser player** (private room, join, random opponent). The "Copy link" button gives the website's invite link.

> Compatibility was tested with the `index.html` of commit `2c707c6` (both directions: Rust host ↔ web client and web host ↔ Rust client).

## Data

Accounts and settings: `%APPDATA%\C02 Naval\` (one JSON file per account, same format as the web version).
The `C02NAVAL_DATA` environment variable changes this folder (portable version on a USB stick, for example).
`C02NAVAL_DEBUG=1` writes a network log (`net.log`) to this folder.
`C02NAVAL_NO_UPDATE=1` disables the update check (development).

## Building

Requirements: [Rust](https://rustup.rs) (stable, `x86_64-pc-windows-msvc` toolchain) and the Visual Studio C++ build tools.

```bash
cd rust
cargo build --release
```

The executable: `rust/target/release/c02-naval.exe` (icon built in, no console window).

Tests: `cargo test --release` (BinaryPack format, PBKDF2, valid random fleet, the bot finishes a game).

## Code layout

| File | Role |
|---|---|
| `src/main.rs` | Window, icon |
| `src/app.rs` | State and logic (accounts, matchmaking, placement, game, timers) |
| `src/ui.rs` | Screens (egui) |
| `src/theme.rs` | Colors, fonts (Segoe UI, Bahnschrift), custom widgets |
| `src/game.rs` | Rules, shot resolution, tactical bot |
| `src/net.rs` | PeerJS signaling + WebRTC (`webrtc` crate) |
| `src/binarypack.rs` | PeerJS BinaryPack format |
| `src/accounts.rs` | Accounts, passwords, photos, settings |
| `src/sound.rs` | Sound synthesis (cpal) |
| `src/i18n.rs` | French / English text |
| `src/update.rs` | Mandatory update from GitHub releases |
| `tools/` | Test scripts (window capture, simulated clicks) |
