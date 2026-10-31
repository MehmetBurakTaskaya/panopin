# Architecture

Status: approved with Phase 0 (2026-10-29) and refined as phases land. The reasoning
behind each technical choice lives in [DECISIONS.md](DECISIONS.md) (referenced as `D-xxx`). The
product name is **Panopin**.

## 1. Principles

- **Rust is the single source of truth.** Data, rules (admission, identity, retention, search,
  networking) and every OS integration live in Rust. The webview renders state and owns the
  keyboard model only.
- **Event-driven, zero idle work.** No polling loops. Idle threads block on OS events or channels.
  The only timers are the retention sweep (once a day) and the optional update check (every 6 h).
- **Local only.** The webview never talks to the network. Rust makes exactly two kinds of
  requests, both opt-in: link previews and update checks.
- **Clipboard content never reaches logs.** Log lines carry ids, sizes, formats and timings only.
- **Platform code behind traits**, selected with `#[cfg(target_os)]`, so macOS (Phase 8) plugs in
  without touching core logic.

## 2. Process and windows

One Tauri 2 process. WebView2 on Windows, WKWebView on macOS.

| Window | Kind | Lifetime |
| --- | --- | --- |
| `shelf` | Transparent, undecorated, always-on-top, skip-taskbar overlay covering the **work area of the monitor under the cursor** | Created hidden at startup for an instant first open; shown and hidden afterwards |
| `library` | Normal resizable window, Mica on Windows 11 | Created on demand, destroyed on close |
| `settings` | Normal window, Mica | On demand |
| `onboarding` | Normal window, first run only | On demand |

The shelf, its search box, the preview, context menus, the command palette and toasts are
**layers inside the one `shelf` webview** (D-004). One React tree means one keyboard model, one
Esc stack (edit → preview → search → shelf) and card-to-preview animations with no cross-window
syncing. Clicking anywhere outside the board hits the overlay and closes the shelf, which is the
specified behaviour. Preview and palette components are reused inside the Library window.

## 3. Rust crate layout

```
src-tauri/src
  main.rs, lib.rs        app setup, plugin registration, single-instance hand-off
  commands.rs            Tauri IPC commands (thin; delegates to services)
  editing.rs             preview edits: text bodies, image edit layers, thumbnails (D-050)
  events.rs              typed events pushed to the webviews
  settings/              typed settings (serde defaults) stored in SQLite
  capture/
    clipboard_win.rs     message-only window + AddClipboardFormatListener (dedicated thread)
    clipboard_mac.rs     NSPasteboard changeCount, compared twice a second (D-073)
    folder_watcher.rs    ReadDirectoryChangesW on a dedicated thread (D-008)
    folder_watcher_mac.rs kqueue on the folder; the screenshot mark on the Desktop (D-073)
    admission.rs         privacy markers, excluded apps, pause, self-copy, whitespace rule
  store/
    mod.rs               open the database (WAL) and apply migrations
    library.rs           Library data, folders, tags, notes, Trash (D-053)
    search.rs            the FTS5 index: folding, indexing, queries (D-017, D-059)
    migrations/          0001_init.sql, ... applied via PRAGMA user_version (D-012)
    models.rs            Item, Folder, Tag, ... (exported to TS with ts-rs, D-013)
    identity.rs          text and image identity hashes (D-016)
    edits.rs             edited text bodies and image edit state (D-052)
    retention.rs         expiry to Trash, Trash auto-empty, "clear history" transaction
    search.rs            FTS5 index + Turkish folding (D-017)
    undo.rs              session undo stack (32 steps, batches = one step)
  media/
    thumbnails.rs        decode once, longest side 512 px (D-025)
    annotations.rs       non-destructive crop + markup layer (JSON)
    flatten.rs           full-resolution render only on copy/export/save
    video.rs             recording poster, length and size from the shell (D-058)
    protocol.rs          custom URI scheme, serves by item id only (D-014)
    video.rs             poster frame + duration via the shell (D-024)
  ocr/        mod.rs (queue), engine_win.rs (Windows.Media.Ocr, D-056), engine_mac.rs (Vision)
  links/      mod.rs (queue, storage), guard.rs (network guard, D-010), page.rs (D-057)
  look.rs                Mica support and the Windows accent colour (D-060)
  edge.rs                edge opening: low-level mouse hook while enabled (D-022)
  snip.rs                the snip shortcut and its crop editor (D-062)
  transfer/
    mod.rs               what a copy, paste or drag carries (D-045)
    staging.rs           neutrally named files handed to other apps
    clipboard_win.rs     clipboard writer with the own-copy marker
    paste_win.rs         UIPI and password-field checks, SendInput (D-046)
    drag_win.rs          shell data object + SHDoDragDrop (D-023)
    folder_win.rs        the common item dialog for exports, imports and watched folders
    folder_mac.rs        NSOpenPanel for the same (D-073)
    clipboard_mac.rs     pasteboard writer with the own-copy type (D-073)
    paste_mac.rs         Accessibility permission, password-field check, CGEvent Cmd+V
    queue.rs             paste one by one: the order and its state (D-084)
    queue_win.rs         clipboard owner window with delayed rendering
    queue_mac.rs         pasteboard item with a data provider
  focus.rs               remember/restore the previous foreground window, Alt mask key
  system_state.rs        full-screen / presentation / focus-session detection (D-021)
  hotkeys.rs             global shortcut registry, recorder suspend/restore
  tray.rs                tray icon, menu, counts tooltip; a picture per state and DPI (D-067)
  updates.rs             update checks, their schedule and installs (D-065, D-066)
  platform/
    theme.rs             accent colour, light/dark, high contrast, animation setting
    edge.rs              optional edge-dwell opener (D-022)
```

Platform traits: `ClipboardBackend`, `WatcherBackend`, `OcrBackend`, `PasteBackend`,
`FocusBackend`, `SystemStateBackend`.

## 4. Threads and data flow

```
 clipboard thread ─┐                        ┌─> OCR queue (low priority thread)
 folder watchers ──┼─> capture worker ─> store ─┼─> link enricher (async, opt-in)
 import / manual ──┘   (admission rules,         └─> event "items:changed" ─> webviews + tray
                        decode, hash, thumbs)
 IPC commands (pin, rename, tag, take down, undo, clear history, ...) ─> store
 retention thread (start + every 6 h) ─> store
```

- One **capture worker** handles clipboard and file events one at a time, so captures never race.
- One SQLite connection behind a mutex is shared by all writers (D-035). Decoding, hashing and
  thumbnails run outside the lock; statements and transactions inside it are short.
- After each change the worker emits `items:changed` with the counts per kind; the tray tooltip
  shows them and windows refetch only what they display.

## 5. Capture pipeline (Windows)

1. `WM_CLIPBOARDUPDATE` arrives on the clipboard thread.
2. Read `GetClipboardSequenceNumber()`; drop the event if it is 0, equals the last self-write, or
   capture is paused or the source is disabled.
3. Owner and foreground checks: `GetClipboardOwner` and `GetForegroundWindow` → process image
   name; drop if on the excluded list (KeePass, KeePassXC, 1Password, Bitwarden + user entries).
4. `OpenClipboard` with a short bounded retry (another process may hold it).
5. Privacy markers: if `Clipboard Viewer Ignore`, `ExcludeClipboardContentFromMonitorProcessing`,
   `CanIncludeInClipboardHistory` = 0 or `CanUploadToCloudClipboard` = 0 is present, drop the
   whole copy. Our own marker format also drops it (self-copy).
6. Read the best format: registered `PNG` > `CF_DIBV5` > `CF_DIB` for images (D-015),
   `CF_UNICODETEXT` for text. `CF_HDROP` is ignored in v1.
7. `CloseClipboard`, read the sequence number again; drop if it changed.
8. Hand the raw payload to the capture worker; nothing is logged except kind and size.

Admission rules (pure, unit-tested): whitespace-only text is rejected but accepted text is kept
byte-for-byte; a trimmed text that is exactly one http(s) URL becomes a **link**; identity is
computed per D-016 and an existing item is bumped (body replaced, `last_copied_at` updated,
title/pin/folders/tags/note kept). Content already on the clipboard when a source is enabled is
ignored: the listener only reacts to later updates.

Screenshot folders: resolved through the Known Folder API (D-007), watched non-recursively.
A file is admitted once its size is stable and it can be opened without sharing violations; it
is **copied** into managed storage, never moved or modified. A Win+Shift+S capture usually
arrives twice (clipboard + auto-saved file); pixel identity merges them into one card.

## 6. Data model (SQLite, WAL)

```
items        id, kind(image|text|link|video), identity BLOB, created_at, last_copied_at,
             title, note, pinned, trashed_at, source(clipboard|folder|import|manual),
             source_app, source_path, text_body, url, width, height, byte_size,
             duration_ms, ocr_state, ocr_text, link_meta JSON, edit_layer JSON
             UNIQUE(kind, identity)
folders      id, parent_id (deleting a folder deletes its subfolders), name, created_at
item_folders item_id, folder_id
tags         id, name, name_key UNIQUE   -- name_key: Turkish i forms unified, lower case
item_tags    item_id, tag_id
settings     key PRIMARY KEY, value JSON
search_fts   FTS5(title, body, note, tags, link_title, domain, ocr,
                  + accent-folded twins) -- contentless, rowid = items.id
```

- **Saved** = pinned OR has a title OR in a folder OR has a tag. Saved items never expire.
- Retention moves unsaved items to Trash when `last_copied_at` is older than the per-kind
  limit (D-018); Trash empties after its own limit. "Clear history" counts affected items,
  asks, then runs one transaction that re-checks "still unsaved" at commit time.
- Files under `%APPDATA%\Panopin\` (D-006): `data.db`, `media/<id>/` (original, thumbnail,
  `edit.json`), `cache/` (link images, favicons), `staging/` (exports and drags; cleaned on exit
  and on next start). Writes are atomic (temp file + rename).

## 7. Search

FTS5 over pre-folded text written by Rust. Folding (search only, never identity): NFC, the four
Turkish i forms unify (İ, I, ı, i → i), then lower-case; an optional accent-folded column maps
ç ş ğ ö ü â î û. Queries become prefix terms (`term*`) with `prefix='2 3'`. Hidden rows are
filtered out with a `kind IN (...)` join. Target: under 50 ms at 10 000+ items, enforced by a
benchmark test.

## 8. Shelf, focus and paste

Open sequence:
1. Hotkey arrives (WM_HOTKEY grants foreground rights). Record the foreground HWND and PID
   **before** showing anything.
2. If Alt is still held, inject a neutral "mask" keystroke so the previous app does not enter
   menu mode on Alt release (to be verified, see MANUAL_TESTS).
3. Size the overlay to the work area of the cursor's monitor, show it, `SetForegroundWindow`.
4. The webview slides the board in with CSS (350–450 ms ease-out, none when Windows animations
   are off). Keys work at once; pointer events are enabled when the board settles.
5. Close: slide out (~200 ms), then hide. Every open/close increments a generation counter and
   stale callbacks are ignored.

Enter copies, hides, and restores the recorded window. Ctrl+Enter additionally: verify the
window still exists and is the same process; check its integrity level against ours (UIPI);
check the focused element with UI Automation and refuse password fields; release any held
modifiers; `SendInput` Ctrl+V. Every refusal shows the reason.

## 9. Frontend

- React + TypeScript (strict) + Vite, CSS Modules with design tokens, Zustand stores per window.
- `src/keyboard/` holds the **pure keyboard model** (rows, selection, ranges, layers, Esc stack),
  tested with scenario tests in vitest; React components only dispatch to it.
- `src/edit/` holds the edit layer model, the geometry shared with Rust, the Canvas 2D drawing and
  the undo history; the preview and its editors live in `src/windows/shelf/preview/`.
- `src/library/` holds the Library's sections, filters and sorting; `src/windows/library/` the
  window (sidebar, grid and list, details); the name, folder and tag pickers and the command
  palette in `src/components/pickers/` serve the shelf, the Library and Settings.
- `src/search/` marks matched words the way Rust folds them; `src/look.ts` applies Mica, the
  accent colour and the animation setting in every window.
- Memoised cards so unchanged cards do not re-render; the Library grid is virtualised
  (`@tanstack/react-virtual`), the shelf rows are capped at 300 cards instead (D-044).
- Sounds: the Web Audio synthesis module proven in `docs/concept/mockup/sound.js`.
- i18n: `/locales/en.json` and `tr.json` are the only string sources, used by TS and by Rust
  (tray, notifications). CI fails on missing or unused keys.

## 10. IPC and security

- Commands are thin and typed; shared types are generated from Rust with ts-rs.
- Tauri capabilities per window: the shelf and library get only the commands they use; no
  shell or filesystem plugin access from the webview.
- CSP: `default-src 'self'`; images and media only from `'self'`, `data:` and the media scheme;
  no remote origins anywhere.
- Media protocol resolves `<scheme>://item/<id>/<variant>` through the database. Paths never come
  from the webview, so traversal is impossible by construction.
- `net_guard`: HTTPS only, no credentials in URLs, no sensitive query keys, custom DNS resolver
  that drops loopback/private/link-local/reserved ranges (re-checked on every redirect), 2 MB and
  5 s limits, HTML parsed never executed.

## 11. Budgets

| Metric | Target |
| --- | --- |
| Installer (NSIS, x64) | < 15 MB (v1.0.0: 3.3 MB) |
| Idle CPU | ≈ 0 % (no polling; v1.0.0: 0.2 s of CPU per minute with WebView2) |
| Shelf open, hotkey → first frame | < 100 ms with the pre-created window |
| Search, 10 000 items | < 50 ms |
| Memory | Published in the README (v1.0.0: about 120 MB idle, 110 MB of it WebView2) |

## 12. Release and website

- `release.yml` (tag `v*`): version and notices checks, `tauri build` builds and signs the
  installer, a draft release gets `Panopin-Setup-x64.exe` and `Panopin-Portable-x64.zip`, then
  `latest.json` (with the update signature) and `SHA256SUMS.txt`, and while `PANOPIN_MACOS` is
  `true` the macOS `.dmg` and update archive (D-068). Started by hand: `mac-test.yml` builds a test
  `.dmg`; `mac-live.yml` drives the app on GitHub's Mac runners with `tests/macos/` (D-073). Panopin's page lives on mehmetburaktaskaya.com; `site/`
  holds its material (D-069).
- Scripts: `icons.mjs` draws every icon from one SVG (D-067); `third-party.mjs` writes the
  licence list and `THIRD_PARTY_NOTICES.txt`, and CI checks that both are current (D-064);
  `release-notes.mjs` and `latest-json.mjs` serve the release workflow.
- `src-tauri/windows/installer-hooks.nsh` makes the installer install at once and adds to the
  uninstaller (D-066).

## 13. Open questions and spikes

| Topic | Plan |
| --- | --- |
| Transparent overlay on Win10/Win11 (known black-background issues) | Spike in Phase 1; fallback: strip window + separate preview window |
| File drag-out from the overlay | Resolved in Phase 3 without a plugin (D-023) |
| Alt-release menu activation after Win+Alt hotkeys | Mask keystroke (D-046); manual test in Word and Notepad |
| Do Not Disturb detection | No public API; partial signals only (D-021) |
| Acrylic placement | Decide in Phase 4 once the preview layer exists |
