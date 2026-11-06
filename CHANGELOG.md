# Changelog

All notable changes to this project are documented here.

## [3.5.2] - 2026-11-06

### Added

- GUI **scarlet** theme (neon red ink)

## [3.5.1] - 2026-11-06

### Fixed

- Frosted labels and the settings cog sit on a light brush-stroke wash so 0% tint stays readable on a light wallpaper
- Long job-status and toast lines wrap under that wash instead of overflowing
- Windows uses Mica while focused (acrylic fallback on older Windows); DWM fills an inactive window with a solid theme color

### Changed

- Installing-update toast shows the same progress bar as job rows and cannot be dismissed until success or failure
- `torrenter update` shows the CLI byte bar while the asset downloads
- Release binaries use the HTTP Range transfer (gzip/brotli off, exact 206, merge via tmp) into a file beside the running exe, not dest/`.resume` (still a pinned GitHub first hop + `SHA256SUMS`)

## [3.5.0] - 2026-11-06

### Added

- GUI settings cog: theme swatches and a tint slider for the frosted window
- macOS/Windows GUI uses a translucent window with OS backdrop blur; phosphor / amber / ice still set the ink

### Changed

- Theme and tint live in the cog; the auto-update slider stays on the header beside it
- `glass_tint` is stored in `config.json` (40–210, default 60% of that range)
- Native fullscreen (macOS/Windows) locks tint at 100% without saving that override

## [3.4.3] - 2026-11-05

### Fixed

- Linux GUI **restart now** after an in-app update spawns a new process (and ignores a `(deleted)` `/proc/self/exe` path) instead of `exec` into the old Wayland/GL session

## [3.4.2] - 2026-11-05

### Fixed

- Updater retries GitHub 502/503/504 (and connect/read timeouts) when fetching `SHA256SUMS` or the binary
- Linux GUI: set Wayland/X11 app id `torrenter`, skip AccessKit, and do not vsync-block so compositor pings are answered (Wayland client decorations stay via winit’s defaults)

## [3.4.1] - 2026-11-05

### Changed

- GUI auto-update control is a plain label plus a track-and-knob slider (same check-on-enable behavior)
- Job rows: copy icon after the title copies that job’s magnet or HTTP URL
- Waiting/starting job bar ping-pongs left and right with eased motion instead of wrapping with a jump

## [3.4.0] - 2026-11-05

### Added

- GUI job list is stored in `jobs.json` (beside `config.json`) and restored on the next launch in the last live state (queued / downloading / paused / seeding), still capped at 4 running jobs
- Startup checks each row’s own dest: missing payload on a >0% magnet, or a live HTTP job without usable `.resume` parts, stays stopped until Restart or Clear

### Changed

- Closing the GUI still drops live torrent sessions; share-mode rows that were seeding start seeding again after the restore probe
- Title click still opens that row’s dest (not the folder used for new START jobs)

### Fixed

- Restored paused magnets pause librqbit before the control loop (in-session pause also reaches `session.pause` instead of blocking on `wait_while_paused` first)
- `jobs.json` is replaced by rename, not unlink-then-rename; a leftover `.tmp` / `.prev` is recovered if the live file is missing
- Restored in-progress rows no longer flash a full bar (`ProgressHandle` was writing total=1 over saved size); the bar slides until the engine reports real progress (~50ms cadence, not an unbounded repaint)

## [3.3.0] - 2026-10-31

### Added

- GUI theme (phosphor / amber / ice) is stored in `config.json` and restored on the next launch; missing or unknown values stay phosphor

## [3.2.3] - 2026-10-27

### Changed

- macOS `.app.zip` packaging is a feature-gated Rust binary (`macos-app`) instead of `scripts/macos-app.sh`
- In-app updates restamp `torrenter.app` `Info.plist` and ad-hoc codesign after replacing the inner binary; leftover `.old` / `.update` / `.sha256sums` files are removed on the next launch

## [3.2.2] - 2026-10-27

### Changed

- GUI toasts: duplicate-job / validation errors auto-dismiss after 5 seconds (click still dismisses). In-progress update notes and dest/update fatals stay until replaced or clicked.
- GUI **Stop** marks the row stopped immediately so **clear** (and **restart**) appear for queued or incomplete jobs, not only after a finished download.

## [3.2.1] - 2026-10-27

### Added

- GUI job search: filter downloading and finished rows by title, URL, folder, mode, or info-hash (`Esc` clears)
- Dock / taskbar icon is a squircle with transparent corners (and is embedded in the Windows GUI `.exe` so pinning keeps the glyph)
- macOS GitHub Releases also attach `torrenter-gui-*-apple-darwin.app.zip` (`torrenter.app` for Applications / Launchpad). The updater still uses the naked GUI Mach-O.

### Changed

- README Install documents the `.app.zip` drag-to-Applications path, macOS quarantine / Open Anyway, and Windows SmartScreen **Run anyway**

## [3.2.0] - 2026-10-26

### Added

- Download folder picker: default `<OS Downloads>/torrenter`, optional `download_dir` in config
- CLI menu `download:` path and **6. Change download folder**; `--dir` on `http` / `magnet` for a one-shot override
- GUI dest row under the URL field; restart confirm popup (mode + folder + START); click a job title to reveal the folder
- Bottom-right toasts for update checks and folder/IO errors

### Changed

- Output no longer uses `./downloads`; magnets isolate under `<dest>/<torrent-name>/` (spaces → `_`), falling back to the info-hash when the name is missing or collides
- Idle magnet rows use **restart** (glyphs are last-mode indicators); HTTP stopped jobs use the same popup minus seed glyphs

## [3.1.0] - 2026-10-25

### Added

- Magnet **share** mode (default): stay connected after 100% and keep uploading until Stop; incoming TCP listen on a random high port
- Magnet **leech** mode: never upload; stop the session at 100%
- CLI menu `seed: on/off` and **5. Toggle seed**; GUI halo/horns glyphs on the URL row and on idle magnet rows (Restart)
- Seeding progress shows **U:** upload size and speed; window/process must stay open to share

### Changed

- `torrenter magnet` in share mode does not exit when the download finishes (`2` or Ctrl-C to stop)

## [3.0.3] - 2026-10-25

### Changed

- GUI header: magnet/HTTP swap on the URL row, auto-update beside theme, pointer cursor on clickable controls, theme swatches that only hit the color squares
- Finished job rows use a brighter accent than in-progress rows; **clear** on inactive rows
- Manual update check shows **no updates found** for about 1.5s when already up to date

## [3.0.2] - 2026-10-24

### Added

- Native GUI (`torrenter-gui`) with phosphor / amber / ice themes, no HTML/CSS
- Release assets now ship CLI and GUI binaries per OS
- `danted4/torrenter vX.Y.Z` on the CLI banner, `--version`, and GUI header
- Opt-in auto-update from GitHub Releases (`--auto-update`, GUI checkbox, or menu item 3)
- Release `SHA256SUMS`; the updater verifies SHA-256, size, host, and timeouts before replacing the binary
- GUI jobs run concurrently (cap of 4) and are keyed by id

### Fixed

- HTTP pause now drops the Range body instead of sleeping on a live connection
- Stop is honored during HTTP metadata and merge
- Magnets isolate output under `./downloads/<info-hash>/` instead of overwriting a shared tree
- Update asset URLs must be `github.com/danted4/torrenter/releases/download/<tag>/<name>` with tag matching the release; cached foreign or mismatched-tag URLs are ignored
- Same magnet cannot run twice at once (GUI refuse + in-process folder lock)
- CLI startup auto-update is gated to an interactive TTY
- GitHub API 429 / cached ETag skip instead of failing the GUI
- Job snapshot mutex poison, pause toggle races, per-frame theme apply, UTC job dates

## [3.0.1] - 2026-10-24

### Fixed

- Magnet downloads raise the process file-descriptor limit and allow overwrite so large torrents can resume instead of failing with "too many open files"

## [3.0.0] - 2026-10-24

### Changed

- Rewrote the CLI in **Rust** (replaces Node.js / WebTorrent)
- Magnet downloads now use **librqbit**; HTTP still uses 4-part Range resume
- Added `torrenter http <url>` and `torrenter magnet <uri>` subcommands
- Renamed the project from nodeWT / torrent-http-download-cli to **torrenter**
- TLS via rustls for Linux, macOS, and Windows without OpenSSL
- HTTP resume state lives in `downloads/.resume/` (not `.nodewt`)
- HTTP resume truncates oversized parts, caps Range bodies, and merges exact byte counts
- Filenames cannot escape `./downloads`; torrent sessions stop cleanly with in-memory DHT only

### Added

- Unit and integration tests for validation, formatters, CLI parsing, HTTP multipart/resume/fallback
- GitHub Actions CI on pushes and PRs to `main` (fmt, clippy, tests on Linux/macOS/Windows)
- Release workflow: `v*.*.*` tags on `main` publish Linux, macOS, and Windows binaries

## [2.2.0] - 2026-08-29

### Changed

- Renamed repository to **torrent-http-download-cli** to reflect **both** download modes equally (torrent + multipart HTTP)
- README repositioned: dual-mode tool, not WebTorrent-only; added "Two download modes" comparison table
- Updated `package.json` description and keywords (`http-download`, `direct-download`, `parallel-download`, `byte-range`)

## [2.1.0] - 2026-08-29

### Changed

- Renamed repository to **webtorrent-cli-client** (formerly **nodeWT**) for discoverability
- Rewrote README with SEO keywords, table of contents, and clearer usage docs
- Fixed `package.json` name (was incorrectly `webtorrent`), version, repository metadata, and keywords
- Added `LICENSE`, `CHANGELOG.md`, and `.nvmrc`

## [2.0.0] - 2026-08-23

### Added

- Direct HTTP(S) URL downloads with **4 parallel byte-range parts** when the server supports ranges
- Resume support for interrupted multipart downloads (`.downloads/.nodewt/`)
- Automatic fallback to single-stream download when ranges are unsupported

## [1.0.2] - 2025-11-08

### Changed

- Node.js 22 LTS support and dependency updates

## [1.0.1] - 2024-08-28

### Fixed

- Process exit and CLI prompt reliability

## [1.0.0] - 2023-10-09

### Added

- Initial WebTorrent magnet-link CLI with progress bar, peer count, speed, and ETA

[2.2.0]: https://github.com/danted4/torrent-http-download-cli/releases/tag/v2.2.0
[2.1.0]: https://github.com/danted4/torrent-http-download-cli/releases/tag/v2.1.0
[2.0.0]: https://github.com/danted4/torrent-http-download-cli/releases/tag/v2.0.0
[1.0.2]: https://github.com/danted4/torrent-http-download-cli/releases/tag/v1.0.2
[1.0.1]: https://github.com/danted4/torrent-http-download-cli/releases/tag/v1.0.1
[1.0.0]: https://github.com/danted4/torrent-http-download-cli/releases/tag/1.0.0
