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
identifier-based folders; the uninstaller offers to remove both (done in Phase 7, D-066).

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
`/locales/*.json` (English and Turkish at first, twelve languages since D-074) are the single string
source for TypeScript (small `t()` helper with `Intl.PluralRules`) and Rust (tray, notifications).
A CI script fails on missing or unused keys.

## D-028 Installer and updates — Accepted
NSIS only (no MSI), `installMode: currentUser` (no admin), WebView2 `downloadBootstrapper`.
`createUpdaterArtifacts: true`; tauri-action v1 uploads `latest.json` (`uploadUpdaterJson`) to a
**draft** release. Because GitHub's "latest release" excludes drafts, clients see an update only
after the draft is published. The signing key exists only in GitHub Secrets. (Phase 7: the
workflow writes `latest.json` itself, D-065.)

## D-029 Portable ZIP — Accepted
The portable build keeps data in `%APPDATA%` like the installed one. A "data next to the exe" mode
is in the backlog. The portable build learns about updates but never installs them (D-066).

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

## D-053 Library data and window — Accepted
Folders nest, and deleting one takes its subfolders but never their items. A tag name is unique
by a key Rust folds from it: the four Turkish i forms (İ, I, ı, i) are one letter and case does not
count, so "İş" and "IŞ" are one tag; the pickers match names the same way. Items join folders and
tags through `item_folders` and `item_tags` (migration 0005). An item is saved when it is pinned,
named, in a folder or tagged; that rule is one SQL expression shared by retention, taking down and
the Library. The window gets every item once, Trash included, with texts cut to 400 characters, and
filters, sorts and counts on its side with pure, tested functions; names sort with the collation of
the UI language. The grid and the list are virtualised with `@tanstack/react-virtual` 3.14.20
(MIT), pinned exactly because a newer release came out the same day. While idle the window renders
nothing. View, sort, panel and column widths are settings. Windows 11 (build 22000 or later, read
from the registry) gets Mica; Windows 10 gets the opaque window colour.

## D-054 Library keys, dialogs and dragging — Accepted
F6 and Shift+F6 move between the sidebar, the items and the details; Tab stays ordinary control
navigation. The items use the shelf's keys where they mean the same thing: Enter and Ctrl+C copy
and leave the window open; P, Alt+P, Shift+P, Ctrl+Shift+T, Space, E, M, C, Delete and Backspace
work as on the shelf. In Trash, R restores and Delete deletes for good after a question. The name,
folder and tag pickers are shared with the shelf, and every dialog hands the focus back to where it
was opened. Cards are dragged onto sidebar folders with HTML drag and drop (the window turns off
Tauri's file-drop handler for this) and keep their pin; Alt+drag hands them out as files.

## D-055 Notes and export — Accepted
Ctrl+N writes a new text note: pinned, so it never expires, with the source "manual" and an identity
of its own, so empty notes never collide; in a folder view it joins that folder. Export asks for a
folder with the Windows folder dialog and writes texts as UTF-8 `.txt`, links as `.url` Internet
shortcuts and images as `.png` (edited images flattened), named like staged files
(`Panopin-<date>-<n>.<ext>`) and never overwriting a file. Exporting from the shelf keeps the board
open while the dialog has the focus and for 600 ms after it, because a late focus event would
otherwise close it.

## D-056 Text in images — Accepted
`Windows.Media.Ocr` reads every image once, newest first, on a thread of its own at background
priority (CPU, disk and memory). It reads in Turkish when that OCR language is installed, else
English, else the user's own languages; Settings shows which, and how to add one. An edited image
is read as it looks now, so a pixelated area stays unreadable; editing clears the old text and the
image is read again. A failed read is not retried. Ctrl+Shift+C copies the text, reading the image
on the spot when the queue has not reached it yet. The text goes into the search index and never
into the logs. On the development PC a test picture with Turkish text was read word for word in
60 ms.

## D-057 Link previews — Accepted
Off until the user turns them on. Each copied link is visited once, one at a time, through the
network guard of D-010 (tested in `links/guard.rs`, including names that resolve to this machine).
The page is only tokenised with `html5gum` 0.8.4 (MIT), never run; the `<head>` gives the title,
description, site name, picture and icon (Open Graph first, then Twitter cards, then `<title>`),
and the character set comes from the header or a `<meta>` (`encoding_rs`). YouTube videos ask
oEmbed for title, channel and picture. Pictures are decoded with limits (8192 px a side, 256 MB)
and re-encoded — at most 512 px, JPEG or PNG; icons 64 px PNG — into the item's media folder, so no
remote bytes are ever served; the webview gets them by item id. A link is waiting, done, failed or
refused (never visited); "Refresh preview" sends it back to waiting. Ctrl+O opens only http and
https links, with the default browser.

## D-058 Screen recordings — Accepted
The Snipping Tool's `Videos\Screen Recordings` (created when missing) and the extra watched folders
are watched for MP4 files once the source is on; a recording may take a minute to be written. Its
identity is the SHA-256 of its bytes, read in chunks, and Panopin keeps its own copy, so the
original is never touched. The poster frame comes from the shell thumbnail provider and the length
and frame size from the property system (D-024): a 4.1 s sample gave its poster, length and size.
On the shelf recordings sit in the image row with a play mark and their length; the preview plays
them from Panopin's copy, answered in ranges of at most 2 MB, so long recordings never load whole
(`media-src` allows only the media scheme). They are copied, dragged and exported as files.
"Import…" in the Library takes pictures and recordings.

## D-059 Search — Accepted
Rust folds the title (the user's and the page's), the body (text or link address, the first
200 000 characters) and the rest (note, tags, site, domain, text in images) into the FTS5 table,
in a plain and an accent-free copy (D-017). Every word typed is a prefix and all must match. The
shelf's search (Shift+S or its global shortcut) shows one box below the board; the rows show only
the matching cards, newest first, with counts per kind, the matched words marked and a chip for
accent-free search. Enter or the arrows go to the results, Esc clears and closes. The Library's
search (Ctrl+F) works with the section and the filters and sorts by relevance (bm25, titles weigh
most) unless another order is chosen. The index is rebuilt in batches after an upgrade. With
10 500 items in a debug build (SQLite always built optimised), the slowest query, a single letter,
took 8.7 ms and the others under 3 ms; the test fails above 50 ms.

## D-060 Look and system settings — Accepted
Library and Settings are created transparent wherever Mica is possible, so the setting can switch
Mica on and off while they are open; the page paints its own background when it is off. The
Windows accent colour comes from `UISettings` (`AccentDark1` on light surfaces, `AccentLight2` on
dark ones) and follows changes at once. Animations are either off, whatever Windows says, or
follow Windows' animation effects. "Start with Windows" uses `tauri-plugin-autostart` 2.7.0 (the
user's Run key); the setting is the truth and the entry follows it at every start. Edge opening is
a low-level mouse hook on a thread of its own, only while the setting is on. Update checks are a
setting now; the checking itself arrives with the updater in Phase 7, and Settings says so.

## D-061 Shortcut recorder and global shortcuts — Accepted
All six global shortcuts work: shelf, shelf with search, snip, Library, copy the last image, pause
or resume capture. While Settings records one, Panopin releases all of its own; saving,
cancelling (Esc), switching windows or a refusal brings them back. Keys are named by their position
(`KeyboardEvent.code`), so the layout does not matter, and a global shortcut needs Win, Ctrl or
Alt. Backspace or Delete turns a shortcut off. The same keys twice are refused, and so are keys
another app holds: the old shortcut then stays. Ctrl+Alt without Win is saved with a warning,
because AltGr sends it (D-020).

## D-062 Screen snip — Accepted
The snip shortcut opens the Windows Snipping Tool overlay (`ms-screenclip:`). The next picture it
puts on the clipboard within two minutes opens on the shelf, under the cursor, in the crop editor.
No shortcut gave Panopin the right to come forward at that moment, so it sends a neutral key first
(D-046's 0xE8), which Windows accepts as the latest input; edge opening does the same.

## D-063 Command palette — Accepted
Ctrl+K opens the palette on the shelf (with or without the preview), in the Library and in
Settings. It is built when it opens, so its commands act on that moment's selection, and it lists
the actions that apply, each with its current shortcut. Every typed word must appear in a command's
name (Turkish-aware). The markup, crop and text editors keep their own keys, so the palette is not
offered inside an editor.

## D-064 Third-party list — Accepted
`npm run third-party` lists every npm production package and every crate of the Windows and macOS
builds with its licence (`npm query .prod`, `cargo tree`) into
`src/windows/settings/third-party.json`, which Settings → About shows, and writes
`THIRD_PARTY_NOTICES.txt` with every licence text the packages ship (the standard text when one
ships none). CI fails when either file is out of date. Run it after changing dependencies.

## D-065 Updates — Accepted
`tauri-plugin-updater` 2.14.1 (Apache-2.0 OR MIT), called from Rust only, so the webviews get no
updater permission. Checking is off until the user turns it on; then Panopin checks 30 s after
start, which leaves time for the network after signing in, and every 6 hours; a failed check is
tried again after an hour. The scheduler sleeps on a condition variable until the next check or a
change of the setting, so there is no polling. "Check now" in Settings and "Check for updates" in
the tray run a check because the user asked. A version on offer puts a blue dot on the tray icon
and "Update to x…" in its menu, and Settings shows its notes (the CHANGELOG section, shown as
plain text). Nothing installs until "Update and restart": the installer is downloaded, its
signature is checked against the public key in `tauri.conf.json`, and it runs in passive mode
(`/P /UPDATE /R`), closing Panopin, updating it in place and starting it again. `%APPDATA%\Panopin`
is never touched. `release.yml` writes `latest.json` with github.com download addresses rather
than the API asset addresses tauri-action v1 would use, because unauthenticated API requests are
limited to 60 an hour per address. The key pair was made with `tauri signer generate` (CLI 2.12.4)
without a password; the private key stays out of the repository (docs/SIGNING.md). Since 1.3.0
checking is always on and updates install by themselves (D-076).

## D-066 Installed and portable copies, uninstalling — Accepted
An installed copy has Tauri's `uninstall.exe` next to `Panopin.exe`; the portable ZIP and
development runs have none, so their update button opens the release page instead of replacing
files. The installer works per user (`%LOCALAPPDATA%\Panopin`), speaks English or Turkish after
the Windows language, and puts `LICENSE.txt` and `THIRD_PARTY_NOTICES.txt` next to the app, which
Settings → About opens. Opening it installs at once: a hook in
`src-tauri/windows/installer-hooks.nsh` starts it again in Tauri's passive mode (`/P /R`), which
shows only a progress bar, closes a running Panopin and starts Panopin when it is done; there is
no folder to choose. Silent installs and the updater's own runs (which pass `/P`) are unchanged.
The same file makes the uninstaller remove the "Start with Windows" entry and, only when "Delete
the application data" is ticked, `%APPDATA%\Panopin` as well; Tauri itself removes only its
identifier folders. Neither happens during an update. Since 1.3.0 the portable copy updates
itself as well (D-076).

## D-067 Icon — Accepted
A note pinned to a cork tile in a wooden frame, candidate B of three (A: the note alone, C: a round
cork badge), chosen by the owner on 2026-10-30. One SVG, `src-tauri/icons/app-icon.svg`, is the
source of every size: `npm run icons` runs `tauri icon` for the app icon and draws the four tray
states (normal; paused: greyed with a pause badge; update on offer: a blue dot; both) at 16, 20,
24 and 32 px. Panopin uses the picture that matches Windows' small-icon size, so the tray never
shows a shrunken one; at 16 px the pin, the note and both badges stay readable on light and dark
taskbars.

## D-068 Release pipeline — Accepted
A `v*` tag runs `release.yml` on `windows-latest`. It first checks that the tag matches the version
in `package.json`, `tauri.conf.json` and `Cargo.toml`, that the third-party notices are current and
that the signing key secret exists. `tauri build` builds and signs the NSIS installer, and the
workflow creates a **draft** release whose notes are the CHANGELOG section plus download notes,
with two files whose names never change, so `releases/latest/download/<file>` links keep
working: `Panopin-Setup-x64.exe` and `Panopin-Portable-x64.zip` (`Panopin.exe`, `LICENSE.txt`,
`THIRD_PARTY_NOTICES.txt`). A last job adds `latest.json`, which carries the installer's update
signature, and `SHA256SUMS.txt`; the release page shows no `.sig` file and no second copy.
Publishing the draft is a manual step. While the repository variable `PANOPIN_MACOS` is `true`
(from 1.1.0, after the Mac live test passed, D-073), a macOS job adds the universal
`Panopin-macOS.dmg` and `Panopin-macOS.app.tar.gz`, the archive updates install, whose signature
also goes into `latest.json`; the download notes then name the Mac files. Code signing waits for SignPath Foundation (docs/SIGNING.md). `CHANGELOG.md` follows Keep a
Changelog and versions follow SemVer. From 1.3.0 a release also carries `Panopin-Update-x64.exe`
(D-077), `latest.json` has a signed entry for the portable ZIP (D-076), and publishing one hides
the older releases (D-076).

## D-069 Website and embed — Accepted
Panopin has no website of its own: on 2026-10-30 the owner chose a page among the projects on
mehmetburaktaskaya.com over GitHub Pages. `site/` holds the material for that page.
`site/embed/download-section.html` is a self-contained section: styles scoped under
`.panopin-embed`, an inline picture, no script; docs/WEBSITE.md explains how to add it to the Astro
site. The screenshots in `site/assets/` show the real shelf components with sample items, rendered
in a browser, so no personal data appears. `site/index.html` is a one-page draft in English and
Turkish, with system fonts and no requests to other sites, whose texts the project page can reuse.

## D-070 New windows are built off the main thread — Accepted
The first live test of the release build (2026-10-30) found Settings blank when the shelf's palette
opened it: the window appeared, but its webview never did. The palette, L, the tray menu and the
global shortcuts run as synchronous commands or event handlers on the main thread, where WebView2
cannot finish creating a webview (a documented Tauri limitation on Windows); the Library is opened
the same way. New windows are now built from a short-lived thread that hands the work to the event
loop, and a window that is already open is still brought forward at once.

## D-071 Item ids are never reused — Accepted
Items had `INTEGER PRIMARY KEY` ids, which SQLite hands out again once the newest item is deleted
for good. Media addresses (`/original/<id>`, `/thumb/<id>?v=`) are cached by the webview for a
year, so a new picture could have shown the picture deleted before it. Migration 0008 adds an
`item_ids` counter: every insert takes `last + 1`, and a trigger keeps the counter at the highest
id ever used. Found while checking the live test results on 2026-10-30; a test covers it.

## D-072 Media stays out of the webview's disk cache — Accepted
Media answers were marked cacheable for a year, so WebView2 kept its own copies of every
thumbnail and picture shown (31 MB on the development PC), and those copies outlived items deleted
for good or a cleared history. Media is now answered with `Cache-Control: no-store`; pictures that
are on screen stay in memory, and reading a file again from `media/` is cheap.

## D-073 macOS (Phase 8) — Accepted
macOS 13 or later, one universal app for Apple silicon and Intel, built and checked by CI on
`macos-latest`. `mac-live.yml`, started by hand, installs it from its `.dmg` on GitHub's Mac
runners (macOS 14, 15 and 26 on Apple silicon, macOS 15 on Intel) and drives it as a user would
with `tests/macos/live.sh`: System Events presses keys, a small Accessibility tool reads and
presses controls inside the web views, `sqlite3` checks what was kept, and screenshots stay with
the run. The runners let the shell use Accessibility and screen capture; the script grants Panopin
Accessibility and Desktop access the same way, through the privacy database. Nobody has tried it on
a real Mac yet. The bindings are the `objc2` framework crates 0.3.2 (Zlib, Apache-2.0 or MIT)
with only the classes in use; Vision needs no main thread.

| Topic | Windows | macOS |
| --- | --- | --- |
| Clipboard changes | `WM_CLIPBOARDUPDATE`, no polling | macOS announces no pasteboard change: a thread compares `NSPasteboard.changeCount` every 500 ms and reads the pasteboard only when it moved. This is the one timer that runs while idle |
| Privacy markers | `ExcludeClipboardContentFromMonitorProcessing`, … | `org.nspasteboard.ConcealedType`, `TransientType`, `AutoGeneratedType`, `com.agilebits.onepassword` |
| Own copies | `Panopin.OwnCopy` format | `com.mehmetburaktaskaya.panopin.own-copy` type |
| Source app | clipboard owner, foreground window | the app in front (macOS names no owner); excluded apps match with or without ".exe" or ".app", so the default list works on both |
| Pictures | PNG, DIB | PNG; TIFF is decoded (the `image` crate's TIFF support, macOS only) and stored as PNG |
| Screenshot folder | the Screenshots known folder, `ReadDirectoryChangesW` | the folder chosen in the Screenshot app (`com.apple.screencapture` `location`), else the Desktop. A thread waits in `kevent` on the folder and lists it when its entries change. That folder is usually the Desktop, so only pictures with macOS's screenshot mark (`com.apple.metadata:kMDItemIsScreenCapture`) count there, whatever the language of their names; folders the user adds take every picture |
| Text in images | Windows.Media.Ocr | Vision `VNRecognizeTextRequest`, accurate, Turkish when the system lists it, else English |
| Copying | text, PNG, DIB, file list | text, PNG, file URLs |
| Folder and file dialogs | common item dialog | `NSOpenPanel` on the main thread, its message carrying the title; imports limited to the known extensions through `UTType` (`objc2-uniform-type-identifiers`) |
| Paste into the previous app | `SendInput` Ctrl+V | the app is brought back (`NSRunningApplication`) and Cmd+V is posted with `CGEvent`. macOS asks for Accessibility permission the first time; without it the item stays on the clipboard and the shelf says how to allow it. A focused `AXSecureTextField` is never pasted into |
| Global shortcuts | Win+Alt+V/C/S | the same accelerators become Cmd+Option+V/C/S. Cmd+Option+V is also Finder's "Move Item Here", which does not reach Finder while Panopin runs; the owner kept it on 2026-10-30, and Settings can change it |
| In-app keys | Ctrl+… | Cmd+… (the key tables already accept either); ⌘⌫ also moves to Trash, as in Finder, since a Mac's Delete key needs fn |
| Texts | — | strings that name Windows, its keys or "this PC" have a `key.mac` variant in the locale files, which the webviews use on a Mac; menus and hints name keys as macOS does (⌥⌘V, ⇧⌘T, ⌫); the AltGr warning is off; excluded apps start as KeePassXC, 1Password and Bitwarden |
| Accent colour | `UISettings`, followed live | `NSColor.controlAccentColor` in sRGB, a little darker for light and lighter for dark surfaces; read again when a window opens |
| Shelf and the keyboard | the shelf takes the foreground and gives it back | since macOS 14 an app in the background may only ask to come forward, so the shelf is a non-activating panel: at start its window takes `PanopinShelfPanel`, an `NSPanel` without fields of its own (only when a panel is no larger than a window, since Tauri keeps its state right behind the window's fields), with `NonactivatingPanel`. It takes the keyboard while the app in front stays active, and Enter and Cmd+Enter return there |
| Shelf over full-screen apps | topmost overlay | the panel also gets `FullScreenAuxiliary` and `IgnoresCycle`; Tauri only lets it join every Space |
| Tray | notification area icon | menu bar template icon that macOS tints; no Dock icon (accessory app). The policy is set before the event loop starts: set in setup, the launch put the default back |
| Screen snip | Snipping Tool overlay | `screencapture -i -c` |
| Start at login | Run key | LaunchAgent (autostart plugin) |
| Distribution | NSIS installer, portable ZIP | universal `.dmg`, ad-hoc signed (`signingIdentity` `-`, which Apple silicon needs for apps from the internet) but not notarised: "System Settings → Privacy & Security → Open Anyway". The ad-hoc signature changed with every build, so macOS asked for Accessibility again after an update; since D-078 every CI build is signed with one self-signed certificate instead. `mac-test.yml`, started by hand, builds a test `.dmg` as a workflow artifact; releases carry the `.dmg` and the update archive while the `PANOPIN_MACOS` variable is `true` (D-068), from 1.1.0 on |

Not on macOS yet (BACKLOG): screen recordings, drag-out (a card dragged on a Mac stays where it
is), edge opening (Settings shows it as unavailable), Mica-like materials and quiet sounds during
full-screen apps.

## D-074 Twelve languages — Accepted
Panopin speaks English (the source), German, Spanish, French, Italian, Brazilian Portuguese,
Turkish, Russian, Arabic, Japanese, Korean and Simplified Chinese. Each is one flat file in `locales/`, named by its
language tag and read by the webviews (every file comes with the code, so `t()` stays synchronous)
and by the Rust side (tray, window titles), as in D-027. The first two entries describe the file:
`"@language"`, its own name, and `"@status"`: `"source"` for English, `"needs review"` for a machine
translation until a native speaker has gone through it, then `"reviewed"`. While the language in
use needs review, Settings → General says so under the language choice; CONTRIBUTING.md tells
translators how to review a language or add one. The language is chosen in Settings and in the
first-run window, where it applies at once. "System default" takes the first of the system's
preferred languages Panopin speaks: any Portuguese gets Brazilian Portuguese, Chinese only in its
simplified script (Traditional Chinese falls through to the next language), and English when none
fits. The page's `lang` follows the language: it gives Turkish its casing and, for Chinese,
Japanese and Korean, a UI font with that language's forms of the Han characters (`tokens.css`).
Korean wraps at spaces (`word-break: keep-all`) instead of between syllables; copied text keeps
its own wrapping. Arabic is written right to left: `dir` follows the language too, so the
windows mirror. Layout rules use the start and end sides instead of left and right; the shelf's
rows start at the right, the card menu opens leftwards, the Library's sidebar and resizers swap
sides, and the left and right arrows keep pointing where they move (right goes back towards the
start). Letter spacing is off for Arabic, whose letters join, and its numbers and dates keep
Western digits (`ar-u-nu-latn`), like its texts. Pictures, the markup canvas and
playback controls do not mirror. Copied text, link titles, names and notes take their direction
from their own first letters (`dir="auto"`) in every language, so English stays left to right in
an Arabic window and Arabic right to left in an English one. Inside right-to-left text, shortcuts and image
sizes are wrapped in Unicode isolates, so ⌘Enter and 1920×1080 keep their order. On Windows the code writes key names from the `key.*` entries,
so each language can use its keyboard's names (German Strg, Umschalt, Entf; French Maj, Suppr,
Échap; Spanish Mayús, Supr, Intro; Italian Maiusc, Invio, Canc), and the texts that mention keys
use the same names. Win, Alt and the Mac symbols are the same everywhere. The Windows installer
speaks the same twelve languages, and the macOS app lists them in `CFBundleLocalizations`
(`src-tauri/Info.plist`), so the panels macOS draws for it follow.
`npm run check:i18n` compares every file with English (keys, placeholders, status) and checks
that the app's two language lists name every file.

## D-075 The Mac app stays unnotarised for now — Accepted
The owner decided on 2026-10-30 not to join the Apple Developer Program (99 USD a year) for now.
The Mac app stays ad-hoc signed and unnotarised, so macOS blocks its first opening until the user
chooses Open Anyway; the README and the release notes give the steps, and the README also gives
the Terminal command that removes the quarantine flag. docs/SIGNING.md lists what signing and
notarisation would take. Since D-078 the app is signed with the project's own self-signed
certificate instead of ad hoc; it is still unnotarised.

## D-076 Updates are always on and install by themselves — Accepted
The owner decided on 2026-10-30 that every copy keeps itself up to date: no switch, and no update
left for the user to install. This changes D-065 and D-066.
- Checking is always on: 30 s after start, every 6 hours, an hour after a failed check. The
  "Check for updates" switch is gone and the stored `updates.autoCheck` value is ignored.
  Settings → Updates, Settings → Privacy and the first-run window say that Panopin goes online to
  update itself and never sends anything about the user or their copies.
- A version found by any check installs by itself once Panopin is out of use: the shelf is closed
  and no Panopin window has the keyboard. While an update waits, a thread looks once a minute
  (usually just once, so there is no standing poll) and looks again after the download; if Panopin
  came into use meanwhile, the download is left unused and tried later. "Update and restart"
  installs at once, also while the update downloads by itself.
- The installed Windows copy runs the installer silently (`installMode` `quiet`, so
  `/S /UPDATE /R`): no window opens over other apps; the installer closes Panopin, updates it in
  place and starts it again.
- The portable copy updates too. It is a release build with `THIRD_PARTY_NOTICES.txt` next to it
  and no uninstaller, and it asks for the `windows-x86_64-portable` entry of `latest.json`: the ZIP,
  which `release.yml` signs with the update key for that version (`tauri signer sign
  --app-version`). After the signature check the new `Panopin.exe` is written in full next to the
  old one; the running program, which Windows lets one rename but not overwrite, becomes
  `Panopin.old`, the new one takes its name, the licence texts follow and Panopin restarts; the next
  start removes `Panopin.old`. The `zip` crate (MIT), already in the build through the updater,
  unpacks it with its flate2 feature.
- A Mac app bundle that macOS has not translocated replaces itself with the update archive and
  restarts. Ad-hoc signed up to 1.3.0 (D-075), it had to be allowed again under Accessibility
  after an update; signed with one certificate since D-078, it keeps the permission. Where the
  user may not change the bundle or its folder, the updater would ask for an
  administrator's password, so there the update never installs by itself: it waits for "Update and
  restart". One that runs translocated (opened where it was downloaded) cannot replace itself, and
  its Updates tab asks the user to move it to Applications; development runs only point to the
  release page. `tests/macos/update.sh` (in the Mac live test) serves a local test build an update
  signed with a key made for the run and checks that it installs by itself and restarts.
- `requireSignedVersion` is on: the version signed into an update must match the version
  `latest.json` announces, so an altered `latest.json` cannot offer an older signed release as a
  new one. `tauri build` (CLI 2.12.4) already signs with the version.
- Copies before 1.3.0 keep their behaviour: with checks off they never look; with checks on they
  offer 1.3.0 and install it when the user presses "Update and restart". From 1.3.0 on, every copy
  updates by itself.
- Only the newest release stays downloadable: when a release is published, `hide-old-releases.yml`
  turns every older published release back into a draft. Drafts keep their files and download
  counts but only the owner sees them; publishing one again brings it back. The source of every
  version stays in the repository.

## D-077 Download numbers — Accepted
The owner asked on 2026-10-30 how many people download and use Panopin. Panopin itself sends
nothing (no ID, no usage counting); the numbers are the download counts GitHub keeps for each
release file. `npm run stats`, or Actions → Download numbers, which shows the same table on its
summary page, lists them per release: people's downloads (the Windows installer, the portable ZIP,
the Mac `.dmg`), updates (`Panopin-Update-x64.exe`, the installer under the name updates fetch, and
`Panopin-macOS.app.tar.gz`) and reads of `latest.json`. Every running copy reads `latest.json` at
start and every 6 hours, 2 to 4 times a day, so its reads per day divided by 4 and by 2 bound the
copies in use. These are downloads, not people: repeated downloads and the owner's own tests count,
and copies before 1.3.0 with checks off are not seen. The workflow has write access only because
GitHub lists draft releases to writers alone. The release page now also shows
`Panopin-Update-x64.exe` (this changes D-068).

## D-078 One signing certificate for every Mac build — Accepted
macOS keeps a privacy permission together with the designated requirement of the build it was
given to, and lets a later build use it only while that build satisfies the requirement (Apple
TN3127). An ad-hoc signature's requirement is that build's code hash (`cdhash H"…"`), so since
copies update themselves (D-076) every update made macOS see Panopin as a new app: ⌘Enter stopped
pasting until Panopin was allowed under Accessibility again, and the Desktop could ask again.
Every Mac build from CI is now signed with one self-signed code signing certificate (CN Panopin,
O Mehmet Burak Taşkaya, RSA 3072, valid until 2056, made with `scripts/mac-signing.sh new`). The
designated requirement then names the certificate instead of the build, `identifier
"com.mehmetburaktaskaya.panopin" and certificate root = H"c853290f…"` (in full in
`src-tauri/macos-requirement.txt`), and every later build signed with it keeps the permissions.
Nothing on a Mac has to trust the certificate: a requirement that names a certificate's hash
checks no trust, and `codesign` signs with an identity that `security find-identity` lists as
`CSSMERR_TP_NOT_TRUSTED`. No Apple account is involved; the app stays unnotarised and Gatekeeper
blocks its first opening as before (D-075).
- The certificate and its key exist only as the repository secrets `MACOS_CERTIFICATE` (the
  `.p12`, base64) and `MACOS_CERTIFICATE_PASSWORD`, and in the owner's offline backup.
  `release.yml` and `mac-test.yml` import them into a keychain of the run's own
  (`scripts/mac-signing.sh import`) and hand the identity to `tauri build` as
  `APPLE_SIGNING_IDENTITY`, which signs the binary, the app and the `.dmg`. Tauri's own import
  (`APPLE_CERTIFICATE`) takes only certificates named like Apple's, such as "Developer ID
  Application: …" (CLI 2.12.4). A release stops if the app's designated requirement differs from
  `src-tauri/macos-requirement.txt`.
- `signingIdentity` stays `-` in `tauri.macos.conf.json`, so a build on a Mac without the
  certificate is signed ad hoc as before.
- `mac-live.yml` signs its build with a certificate made for the run. `live.sh` and `update.sh`
  give Accessibility and Desktop access bound to the installed build's designated requirement,
  as System Settings stores them; until now the test stored them for any code with Panopin's
  bundle id, which no update could lose. After its update the app still pastes with ⌘Enter and
  still takes a screenshot from the Desktop. `permission.sh` shows that the runner's privacy
  database holds to the requirement: a second build with the same certificate keeps Accessibility,
  an ad-hoc build does not, and neither does a second ad-hoc build after a first. Runs
  41095683679 and 41098617458 passed all of this on macOS 14.8, 15.8 and 26.6 (Apple silicon) and
  15.7 (Intel).
- Copies up to 1.3.0 are ad-hoc signed: after the update to the first certificate-signed version,
  Panopin has to be allowed under Accessibility once more, and the Desktop may ask once; from then
  on updates keep both.
- A lost or replaced certificate costs users that one permission again and nothing else; the
  update key (D-065) is the one that must never be lost. A new certificate needs a new
  `macos-requirement.txt` in the same commit.
This changes D-073, D-075 and D-076.

## D-079 Actions pinned to commits — Accepted
Every `uses:` in `.github/workflows/` names the full commit SHA of a release, with the release in a
comment (`actions/checkout@3d3c42e… # v7.0.1`), so a moved or taken-over tag cannot change a step
that runs while the update key or the signing certificate is in reach (D-065, D-078). The owner
agreed on 2026-10-30. The pins are the commits the major tags pointed at that day:
actions/checkout v7.0.1, actions/setup-node v7.1.0, actions/upload-artifact v7.0.2,
Swatinem/rust-cache v2.9.3 and EmbarkStudios/cargo-deny-action v2.1.1, a Docker action built from
the Dockerfile at that commit, whose base image is pinned by digest; it runs only in CI, which has
no secrets. Nothing updates the pins by itself: for a new release, look up its tag's commit (`gh api
repos/<owner>/<repo>/git/ref/tags/<tag>`, then `git/tags/<sha>` when the tag is annotated) and
change the SHA and the comment together.
