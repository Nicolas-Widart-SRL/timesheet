# Timesheet

Native desktop app (Rust, egui) for monthly timesheet entry on a single project, built from the
mockup in `A · Monthly entry (single project)-html/`. Runs on macOS and Windows.

- Weekdays only, at most 8 h per day, three task types (Frontend, Backend, Support).
- Exports two PDFs per month: **Detailed report** (every line, grouped by week) and
  **Daily summary** (hours per type per weekday).
- Consultant, client and project names are set in Settings (click the avatar).

## Build and run

Install Rust from <https://rustup.rs>, then:

```sh
cargo run --release          # run
cargo build --release        # binary in target/release/timesheet(.exe)
cargo test                   # unit tests
cargo test -- --ignored      # writes sample October PDFs to target/
```

Build the Windows `.exe` on a Windows machine with the same commands (MSVC toolchain).
Release builds on Windows open without a console window.

### macOS app

`cargo build --release` produces a bare binary. To get a `Timesheet.app` bundle:

```sh
cargo install cargo-bundle   # once
cargo bundle --release       # target/release/bundle/osx/Timesheet.app and dmg/Timesheet.dmg
```

Drag `Timesheet.app` to `/Applications`. Bundle settings are in `[package.metadata.bundle]` in `Cargo.toml`.

The app is not signed. It opens normally on the Mac that built it. On another Mac, open it once with
right-click › **Open**, or System Settings › Privacy & Security › **Open Anyway**.

## CI and releases

`.github/workflows/ci.yml` builds and tests on macOS and Windows for every push to `main` and every pull request.

To publish a release, set `version` in `Cargo.toml`, commit, then push a matching tag:

```sh
git tag v0.1.0 && git push origin v0.1.0
```

The workflow then attaches `Timesheet-v0.1.0-macos-universal.dmg` (Apple silicon and Intel) and
`Timesheet-v0.1.0-windows-x64.exe` to a new GitHub release. It fails if the tag does not match the `Cargo.toml` version.

## Data

All data is saved automatically after each change, as JSON:

- macOS: `~/Library/Application Support/Timesheet/timesheet.json`
- Windows: `%APPDATA%\Timesheet\data\timesheet.json`

If the file cannot be read, it is renamed to `timesheet.corrupt-<timestamp>.json` (never overwritten)
and the app starts empty with a warning.

## Fonts

Instrument Sans and JetBrains Mono are embedded in the binary (SIL Open Font License, see `assets/fonts/`).
