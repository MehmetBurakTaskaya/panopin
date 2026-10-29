# Decisions

Technical defaults chosen during development, newest last. Product decisions (name, visual
concept, default shortcuts) are recorded under "Product decisions".
Status values: **Accepted**, **Proposed** (under discussion), **Open** (needs a spike).

## Product decisions

| Topic | Decision | Date |
| --- | --- | --- |
| Product name | **Panopin** (pano + pin) | 2026-10-29 |
| Default global shortcuts | Shelf **Win+Alt+V**, shelf with search **Win+Alt+C**, screen snip **Win+Alt+S** (D-020) | 2026-10-29 |
| Visual concept | Cork board, paper tape and pushpins; mockup in `docs/concept/mockup/` | 2026-10-29 |

## D-001 Documentation language — Accepted
Repository docs, code and comments are in English for an international open-source audience.
READMEs ship in English and Turkish.

## D-002 Toolchain and versions (verified 2026-10-29) — Accepted
Checked against crates.io, npm, GitHub releases and nodejs.org. Re-verify at the start of Phase 1.
- `tauri` 2.12.5, `tauri-build` 2.7.4, `@tauri-apps/cli` and `@tauri-apps/api` 2.12.4.
- Plugins: global-shortcut 2.4.1, single-instance 2.5.5 (Rust only), autostart 2.7.0,
  updater 2.14.1, process 2.4.0, dialog 2.8.4, opener 2.7.2, notification 2.8.0, log 2.10.1,
  window-state 2.5.0.
- `windows` 0.62.2, the line Tauri itself uses (a 0.100 line is being prepared; do not mix).
- `rusqlite` 0.41.1 with `bundled` (SQLite 3.54.1; the bundled build enables FTS5).
- `tauri-apps/tauri-action@v1`. Node.js 26 LTS. Rust stable, MSVC target.

## D-003 Package manager — Accepted
npm, because it ships with Node and needs no extra install.

## D-004 Shelf as one transparent overlay window — Accepted on Windows 11, Open on Windows 10
Phase 1 result (Windows 11 25H2, build 26200, 1920×1080 at 100 %): the overlay opens on the cursor's
monitor, takes focus from the hotkey, shows no black background, and Esc hands focus back to the
previous app. Windows 10 22H2 and multi-monitor/DPI setups are still untested.
The shelf window covers the work area of the cursor's monitor; board, search, preview, palette and
menus are layers in one webview. Gives a single keyboard model and Esc stack, and "click outside
closes" for free. Risks: open Tauri issues about black backgrounds on transparent windows, and
black areas when toggling ignore-cursor-events on Windows 10. Fallback: a strip-sized shelf window
plus a separate centred preview window.

## D-005 Window materials — Accepted
Mica for Library, Settings and Onboarding through Tauri's native `windowEffects` (Windows 11).
On Windows 10 Mica is a silent no-op, so those windows use an opaque themed background there. No
direct `window-vibrancy` dependency. Acrylic is limited to places that never resize (decided in
Phase 4) because it lags during resize.

## D-006 Data location — Accepted
User data lives in `%APPDATA%\Panopin\` (`data.db`, `media/`, `cache/`, `staging/`).
Framework-managed state (WebView2 profile, window-state file) stays in Tauri's
identifier-based folders; the uninstaller offers to remove both (Phase 7).

## D-007 Screenshot folders via Known Folders — Accepted
The usual path `%USERPROFILE%\Pictures\Screenshots` is wrong on many machines. On the development PC,
Pictures is redirected to OneDrive and localised: `…\OneDrive\Resimler\Ekran Görüntüleri`. The app
resolves `FOLDERID_Screenshots` with `SHGetKnownFolderPath`, and the Videos known folder for screen
recordings (exact subfolder name verified in Phase 2).

## D-008 Own folder watcher instead of `notify` — Accepted
`notify` is licensed CC0-1.0, which is not on the project's licence allow-list. Watching up to four folders
non-recursively is small: `ReadDirectoryChangesW` (overlapped) on one thread on Windows, FSEvents on
macOS in Phase 8. If CC0-1.0 is added to the allow-list later, this can be revisited.

## D-009 HTTP and TLS — Accepted
`reqwest` with `native-tls` (SChannel on Windows, Security.framework on macOS) for link previews,
and `tauri-plugin-updater` with `default-features = false` + `native-tls`. Smaller binary and the
OS trust store. (The rustls/ring/aws-lc licences would also pass; this is about size.)

## D-010 Network guard — Accepted
Custom DNS resolver that returns only public unicast addresses (drops 127/8, 10/8, 172.16/12,
192.168/16, 169.254/16, 100.64/10, 0/8, multicast, ::1, fc00::/7, fe80::/10, mapped and other
reserved ranges); connections go only to vetted addresses, so DNS rebinding is covered. The redirect
policy re-validates every hop (https, no credentials, no sensitive query keys such as token, key,
secret, sig, password, auth, session, code), at most 5 hops, 2 MB and 5 s limits.

## D-011 Settings storage — Accepted
Typed Rust struct with serde defaults, stored as JSON in the `settings` table of the same database:
one file to back up, transactional, migrations cover it.

## D-012 Migrations — Accepted
Numbered SQL files embedded with `include_str!`, applied in a transaction and tracked with
`PRAGMA user_version`. No migration crate. Every migration has a test that runs on a fresh DB and on
the previous schema.

## D-013 Shared IPC types — Accepted
`ts-rs` (MIT) exports Rust models to TypeScript during `cargo test`; CI fails if the generated files
differ from the committed ones. Introduced in Phase 2 with the item model; Phase 1's only shared type
(settings) is mirrored by hand in `src/ipc.ts`.

## D-014 Media protocol — Accepted
A custom URI scheme serves `item/<id>/<variant>` resolved through the database. The webview never
supplies a path, so path traversal is impossible by construction; responses come only from the
managed folder.

## D-015 Image identity and clipboard formats — Accepted
Format priority: registered `PNG`, then `CF_DIBV5`, then `CF_DIB`. A 32-bpp DIB whose alpha channel
is entirely zero is treated as opaque. Identity = SHA-256 over width, height and RGBA8 pixels
after decoding, so the same pixels from PNG, DIB or a saved file deduplicate.

## D-016 Text identity — Accepted
The identity key is the SHA-256 of the normalised text: NFC via `unicode-normalization` (never
NFKC), CRLF/CR → LF, outer newlines ignored unless the text is only newlines. Spaces, tabs, case
and invisible characters stay significant. Stored bodies are never normalised. Table-driven tests
cover the edge cases (line endings, Turkish İ/I, ligatures, trailing spaces).

## D-017 Turkish-aware search folding — Accepted
Index-only folding: NFC, İ/I/ı/i → i, lower-case; a second column also folds ç ş ğ ö ü â î û for the
optional accent-insensitive mode, so the setting toggles without reindexing. FTS5 `unicode61` with
`remove_diacritics 0` (folding is ours, deterministic); prefix queries with `prefix='2 3'`. The
concept mockup implements the same folding.

## D-018 Retention clock — Accepted
Expiry counts from `last_copied_at`, so copying an item again restarts its clock. Saved items
(pinned, titled, in a folder or tagged) never expire.

## D-019 In-app letter shortcuts — Accepted
Matched on `KeyboardEvent.key` with Turkish I-folding (İ, I, ı, i are the same key for shortcuts),
falling back to `KeyboardEvent.code` when the layout produces non-Latin letters. This keeps P, S, F,
Shift+I and others working on Turkish Q and Turkish F.

## D-020 Global shortcuts — Accepted
RegisterHotKey via the global-shortcut plugin. Research on 2026-10-29 (Microsoft's shortcut list,
Office, Chrome/Edge/Firefox, VS Code, Windows Terminal, the AltGr layers of 218 built-in layouts
and a RegisterHotKey probe on Windows 11 25H2):
- Every Ctrl+Alt+letter or digit produces an AltGr character on at least 31 layouts (Turkish Q:
  A æ, E €, I i, Q @, S ß, T ₺, digits 0–5 and 7–9), so none is safe as a default.
- Alt+Shift+key collides with the default input-language toggle; Ctrl+Shift+Space breaks Word,
  Excel, VS Code and Windows Terminal; Win+Shift+V is taken by Windows.
- Win+Alt+V, Win+Alt+C and Win+Alt+S are free and involve no Ctrl, so AltGr cannot trigger them.
  Win+Alt+F is already registered on the development PC.
Defaults stay rebindable; error 1409 (already registered) shows a warning and never steals the
combination. Microsoft reserves Win-key combinations, so future Windows releases may claim one.

## D-021 Do Not Disturb — Accepted (with a documented limitation)
Windows has no public API that reports manual Do Not Disturb. Sounds are muted when
`SHQueryUserNotificationState` reports busy, full-screen Direct3D or presentation mode, and when
`FocusSessionManager.IsFocusActive` is true where supported. A manually enabled Do Not Disturb may
go undetected; this is stated in the README. Undocumented WNF state is not used.

## D-022 Edge opening — Accepted
A low-level mouse hook is installed only while the (default-off) feature is enabled; a 300 ms
dwell timer starts on entering the edge band and is suppressed when a full-screen app is active.

## D-023 Drag-out — Open (spike in Phase 3)
Text uses HTML5 drag from the webview. Files (images, videos, exports) use `tauri-plugin-drag`
2.1.1 (MIT OR Apache-2.0), which has open Windows issues (a DoDragDrop hang, long-path and network
path crashes). Fallback: our own OLE drag source with the `windows` crate.

## D-024 Video posters — Accepted
Poster frame from the Windows shell thumbnail provider (`IShellItemImageFactory`), duration from the
property system. No ffmpeg. AVFoundation on macOS in Phase 8.

## D-025 Thumbnails — Accepted
Decoded once with the `image` crate, longest side 512 px (2× the largest card). JPEG quality 85 for
opaque images, PNG when alpha is used.

## D-026 Sounds — Accepted
Web Audio synthesis (no audio files), ported from `docs/concept/mockup/sound.js`: default level
0.3, a compressor on the master bus, a per-sound minimum gap and at most three voices per 160 ms.

## D-027 Localisation — Accepted
`/locales/en.json` and `tr.json` are the single string source for TypeScript (small `t()` helper
with `Intl.PluralRules`) and Rust (tray, notifications). A CI script fails on missing or unused keys.

## D-028 Installer and updates — Accepted
NSIS only (no MSI), `installMode: currentUser` (no admin), WebView2 `downloadBootstrapper`.
`createUpdaterArtifacts: true`; tauri-action v1 uploads `latest.json` (`uploadUpdaterJson`) to a
**draft** release. Because GitHub's "latest release" excludes drafts, clients see an update only
after the draft is published. The signing key exists only in GitHub Secrets.

## D-029 Portable ZIP — Accepted
The portable build keeps data in `%APPDATA%` like the installed one. A "data next to the exe" mode
is in the backlog. Whether the portable build shows the updater is decided in Phase 7.

## D-030 Frontend toolchain (Phase 1) — Accepted
React 19.3, Vite 8.3, Vitest 5.0, ESLint 10 with typescript-eslint 8.72, Zustand 5. TypeScript stays
on **6.0.x**: typescript-eslint supports TypeScript below 6.1, so TypeScript 7 (the new native
compiler) waits until the lint toolchain supports it. Node.js 26.13 LTS, Rust 1.99.1.

## D-031 Build directory on long paths — Accepted
MSVC build tools can fail when paths exceed 260 characters. When the checkout path is long, local
builds set `CARGO_TARGET_DIR` to a short folder (for example `%LOCALAPPDATA%\panopin-build`). CI
checkouts are short and need nothing special.

## D-032 Licence checks — Accepted
`cargo deny check` enforces the licence allow-list (MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0,
MPL-2.0) for Rust crates exactly. `npm run check:licenses`
checks every production npm package (the code that ships in the app); dev-only tooling is never
distributed and is not checked.

## D-033 Phase 1 settings window — Accepted
Phase 1 ships a small Settings window: language (applies instantly in every window and the tray) and
a read-only list of shortcuts. The full tabbed Settings window arrives in Phase 6.

## D-034 Tray click after a focus-loss close — Accepted
Clicking the tray icon first moves focus away from the shelf, which closes it. A tray click within
400 ms of such a close is treated as the same gesture, so the shelf does not immediately reopen.

## D-035 Phase 1 storage access — Accepted
A single SQLite connection behind a mutex serves Phase 1 (settings only). The dedicated writer
thread from ARCHITECTURE §4 arrives with capture in Phase 2.
