# Manual tests

Checks that automated tests cannot cover: real focus and paste, global shortcuts, multiple monitors,
DPI, sleep/resume, sounds. Each phase adds or ticks items. Record the date, Windows build and result
next to each item when it is run.

## Phase 0: concept mockup

Run `python -m http.server 4317 --directory docs/concept` and open `http://localhost:4317/mockup/`.

- [ ] Light and dark themes both read well (cork, wood frame, paper, tape colours).
- [ ] Three filled rows; taped and pinned cards; number badges on the active row.
- [ ] P pins (pin drops in, tape peels off, "tok" sound), P again unpins (pop + rustle).
- [ ] S takes a card down (tape tears, paper falls); Backspace puts it back.
- [ ] Click copies (jiggle + "Kopyalandı"); Enter copies and closes the board.
- [ ] Shift+S search: "istanbul" finds "İstanbul", "ışık" finds "IŞIK", "cicek" finds "çiçek".
- [ ] Empty state and the reduced-motion option.

## Global shortcuts and keyboard layouts (Phase 1–3)

- [ ] On Turkish Q: Win+Alt+V toggles the shelf, Win+Alt+C opens it with search focused, Win+Alt+S starts a snip.
- [ ] With the app running, AltGr+Q/E/T still type @ € ₺ on Turkish Q, and AltGr+F/E/T on Turkish F.
- [ ] After Win+Alt+V and Enter, Notepad's menu bar and Word's KeyTips are not left active.
- [ ] Recorder: a combination already used by another app shows a warning and is not taken over.
- [ ] Letter shortcuts inside the shelf (P, S, F, Shift+I/T/L) work on Turkish Q and Turkish F.

## Capture (Phase 2)

- [ ] Win+Shift+S capture appears on the shelf exactly once (clipboard and auto-saved file merge).
- [ ] Win+PrtScn file appears; the original file is unchanged and never moved.
- [ ] The screenshot folder resolves correctly when Pictures is redirected to OneDrive and localised.
- [ ] Copies from KeePass, KeePassXC, Bitwarden and 1Password are not captured.
- [ ] Pause for 5 minutes, 1 hour and until resumed; the tray icon changes while paused.
- [ ] Capture keeps working after sleep and resume.
- [ ] Content already on the clipboard when a source is enabled is not captured.

## Shelf, focus and paste (Phase 3)

- [ ] Enter and Ctrl+Enter paste correctly into Notepad, Word, Chrome, Edge and VS Code.
- [ ] Ctrl+Enter into an elevated (administrator) window explains why it cannot paste.
- [ ] Ctrl+Enter refuses to paste into a password field.
- [ ] Two monitors at 100 % and 150 %: the shelf opens on the cursor's monitor and renders sharp.
- [ ] With Windows "Animation effects" off, the shelf appears without sliding.
- [ ] Rapid open/close never leaves the shelf hidden while it should be open.
- [ ] Drag text into Notepad; drag an image as a file into Explorer and into a chat app.
- [ ] Sounds are silent while a full-screen video, game or presentation is active.
- [ ] Do Not Disturb on: note whether sounds stay silent (known limitation, D-021).
- [ ] No black background on the transparent overlay (Windows 10 22H2 and Windows 11).

## Appearance and accessibility (Phase 3–5)

- [ ] Mica on Windows 11; opaque fallback on Windows 10 22H2.
- [ ] High contrast themes keep cards, focus rings and text readable.
- [ ] Narrator reads card types, titles and actions in English and Turkish.

## Updates and data safety (Phase 7)

- [ ] Updating from the previous release keeps all data and settings.
- [ ] The installer runs without administrator rights; SmartScreen steps match the README.
