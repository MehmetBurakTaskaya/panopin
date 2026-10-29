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

## D-023 Drag-out — Accepted (revised in Phase 3)
No plugin. A card that moves 6 px while pressed starts a native drag: the shell builds the data
object (`SHCreateDataObject` with the item IDs of neutrally named copies in `staging/`, plus
`CF_UNICODETEXT` for texts and links) and `SHDoDragDrop` runs it with a small drop source of ours.
The overlay covers the screen, so it ignores the pointer during the drag and the app beneath gets
the drop; letting go over the board cancels instead of dropping into what lies under it. A drop
closes the shelf without taking focus back. (`tauri-plugin-drag` was the first plan; it has open
Windows issues and is not needed.)

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

## D-035 Storage access — Accepted (revised in Phase 2)
One SQLite connection behind a mutex serves every writer. Capture events are handled one at a time
by a single worker thread, and decoding, hashing and thumbnails run outside the lock, so the lock is
held only for short statements. A separate writer thread added complexity without a benefit.

## D-036 Text wins over a rendered picture — Accepted
Many apps put both text and a picture of it on the clipboard (spreadsheets, documents). A copy that
carries non-whitespace text is stored as text or link only; its picture is not even read. A copy
whose text is empty or whitespace-only, with an image, is stored as an image. A disabled text or
link source does not fall back to the picture.

## D-037 Copying again — Accepted
Copying the same text, link or picture again moves the existing card to the front and takes it out
of Trash; a text keeps the newest bytes. Title, pin and note never change. A link's identity is its
URL without outer whitespace.

## D-038 Pause — Accepted
The pause is stored in settings (`pausedUntil`, Unix ms; the largest value means "until resumed"),
so it survives a restart. Timed pauses end through a sleeping timer thread; a timer whose pause was
replaced does nothing. While paused, the tray icon is greyed out with a pause badge.

## D-039 First-run gate — Accepted
Nothing is captured until the first-run window has been confirmed (`onboarded`). It lists the
sources, says plainly that text is collected, states that everything stays on the PC and that
Panopin is not a password vault, and asks separately about link previews (off by default).

## D-040 Reading the clipboard — Accepted
On `WM_CLIPBOARDUPDATE` the listener checks the sequence number, records the owner and foreground
executables, opens the clipboard (up to five short retries), checks the privacy markers before
reading any content, then reads text (UTF-16 up to the first NUL) or an image (`PNG`, then
`CF_DIBV5`, then `CF_DIB`). The change is dropped if the sequence number moved meanwhile. Copies made
by Panopin itself will carry the `Panopin.OwnCopy` format (Phase 3) and are skipped. After sleep, the
listener re-registers on resume.

## D-041 Screenshot files — Accepted
A folder event starts a bounded wait (at most about 10 s) until the file size holds still and the
file can be opened exclusively, then the bytes are copied into managed storage. Files already
imported with the same size and time are skipped. The Screenshots known folder is created if it does
not exist yet, so the first screenshot is not missed. Extra watched folders belong to the
screenshots source; their settings UI arrives in Phase 6.

## D-042 Retention schedule — Accepted
Retention and Trash emptying run at start and then every 6 hours from a sleeping thread. "Clear
history" is exposed as two commands (count with a cutoff, then confirm); its Settings UI arrives in
Phase 6.

## D-043 Taking items off the shelf — Accepted
Items carry a `shelved` flag (migration 0003). S on a saved item (pinned or named) clears it, so the
item leaves the shelf but stays in the Library; S on a temporary item moves it to Trash. Delete asks
first and moves any item to Trash, pin included. Copying an item again puts it back on the shelf.
Backspace undoes the last 32 take-downs or deletions of the session; a batch is one step. Each row
shows its 300 most recent items.

## D-044 Keyboard model and paging — Accepted
The model is a pure reducer (`src/keyboard/shelfModel.ts`) and the key table a pure function
(`shelfKeys.ts`); both are covered by scenario tests. A page is the number of cards that fit side by
side in the active row, at most 9. The page that holds the cursor is scrolled to the left edge, so
the 1–9 badges are always on screen. Digits also match the physical digit row (layouts that need
Shift for digits). Rows are not virtualised: lazy thumbnails keep 300 cards per row cheap, and
virtualisation arrives with the Library grid.

## D-045 What a copy puts on the clipboard — Accepted
Texts and links are joined with CRLF in selection order; a link goes back as its URL without outer
whitespace, a text exactly as it was last copied. One image: PNG (the stored original when it is
PNG) and a 32-bit `CF_DIB` laid on white, since DIB readers ignore alpha. Several images: a
`CF_HDROP` list of copies in `staging/` named `Panopin-<date>-<n>.<ext>` in their original format,
and the first image as PNG and DIB. Every copy carries `Panopin.OwnCopy`, so it is not captured
again and does not move its card. Windows clipboard history treats it like any other copy.

## D-046 Focus and paste — Accepted
The foreground window (HWND and process id) is recorded before the shelf is shown. Esc, Enter, a
click beside the board and the shelf shortcut give focus back to it; losing focus to another window
does not. Ctrl+Enter: the window must still exist in the same process; a window of a higher
integrity level (an app run as administrator) cannot receive input from Panopin (UIPI), so the item
is copied and the shelf explains that Ctrl+V has to be pressed there; a process that cannot be
queried counts as higher. Otherwise the item is copied, focus goes back, Panopin waits up to 600 ms
for the window to be in front, asks UI Automation whether the focused element of that process is a
password field (refused if so), releases held modifiers and sends Ctrl+V with `SendInput`. A refusal
after the board has gone reopens it with the reason. While Alt is held when the shelf opens or is
closed by its shortcut, the unassigned key 0xE8 is sent so the Alt release does not activate a menu
bar.

## D-047 Shelf session state — Accepted
Hidden rows (Shift+I/T/L) and the pinned-only filter (F) last until Panopin quits, like the board
itself. Hiding the last visible row closes the shelf and the next opening shows all rows again.
The context menu and the Delete confirmation are layers inside the shelf webview rather than native
menus, so focus never leaves the overlay and one key handler serves every layer.

## D-048 Opening from a second launch — Accepted
Every launch first calls `AllowSetForegroundWindow(ASFW_ANY)`, so a second launch can pass its
right to take the foreground to the running instance, which then brings the shelf up with focus. A focus-loss close counts only after the
shelf has had focus once, and an opening that fails half-way is undone so the next shortcut works.

## D-049 Shelf look and sounds settings — Accepted
Card size (small, medium, large; medium by default) switches size tokens on `<html data-size>`.
The shelf slides in from the top (default) or the bottom of the screen. Sounds are on at level 30
of 100. They stay silent for an opening when `SHQueryUserNotificationState` reports busy,
full-screen Direct3D or presentation mode, or a focus session is active (D-021). WebView2 allows
audio without a prior click in Tauri's default configuration, so the opening sound plays at once.

## D-050 Image edit layer and flattening — Accepted
The original pixels never change. A crop and the marks live in `media/<id>/edit.json` (version 1,
original image pixels); "Revert to original" deletes it. Rust validates every layer (sizes,
colours, counts) before using it. The full-resolution picture is drawn only when it is needed —
copying, dragging and saving, which redraws the thumbnail — with `tiny-skia` 0.12 (BSD-3-Clause):
pixelation first, on the original pixels, then the marks in order, then the crop. The webview draws
the same geometry with Canvas 2D: round caps and joins for strokes, mitred rectangles, the
highlighter as one shape at 35 % opacity (no darker overlaps), and one shared arrow-head formula.
Text marks carry a PNG of their glyphs drawn by the webview at image resolution, so the copied
picture shows exactly the text that was on screen and Rust needs no font engine. A 3840×2160
picture with 40 strokes, a pixelated area and a crop flattens in about 0.45 s (release build).
Thumbnails are versioned in their URL (`?v=`), so a redrawn one is never served from the cache.

## D-051 Preview and editors — Accepted
The preview is a layer in the shelf webview and shows the card under the shelf cursor: the arrows
move through the cards and the layer stays open when the kind changes. Space toggles it, a double
click opens it, E edits a text, M marks up and C crops an image, Esc closes the editor first, then
the preview, then the board. Images fit the panel without being enlarged; the wheel zooms from
100 % to 400 % of that size, dragging pans, a double click resets. The original is decoded once into
an `ImageBitmap`; one canvas holds the picture and its pixelated areas, a second one the marks,
redrawn once per animation frame while drawing, with every coalesced pointer sample kept. Tool keys
are V B H R O A D T, and X for pixelate. Stroke widths are picked on
screen and stored in image pixels; pixelation blocks are 1.2 % of the picture's longer side (at
least 10 px), coarse enough that small text cannot be read back. Edits save when leaving the editor,
the picture or the preview; copying, pasting and dragging wait for saves still in flight.

## D-052 Edited texts — Accepted
A text saves about 300 ms after typing stops, and when editing ends or another card is shown. Its
identity follows the new text, so copying that text again finds the same card. When another card
already holds that text, the edited card gets an identity of its own and both stay, because merging
them could lose a name or a pin.
