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

## Phase 1: skeleton

Automated run on the development PC (2026-10-29, Windows 11 25H2 build 26200, one 1920×1080 monitor
at 100 %, Turkish Q, debug build) with simulated key presses and screen captures:

- [x] Win+Alt+V opens the empty cork shelf on the monitor under the cursor, and Panopin takes focus.
- [x] The overlay is transparent around the board (no black background).
- [x] Esc slides the shelf away and focus returns to the previous app.
- [x] Open → close → open within about 100 ms leaves the shelf open (the stale close is ignored).
- [x] The UI follows the Turkish system language.
- [x] `%APPDATA%\Panopin\data.db` is created in WAL mode; the log holds only lifecycle lines.

Still to check by hand:

- [ ] Tray icon: left click toggles the shelf; the menu shows "Rafı göster", "Ayarlar…", "Çıkış".
- [ ] Clicking outside the board (desktop, taskbar, another window) closes the shelf.
- [ ] A tray click while the shelf is open closes it and does not reopen it.
- [ ] Settings → Dil: switching to English updates Settings, the shelf and the tray menu at once, and
      survives a restart.
- [ ] Starting Panopin a second time does not start a copy; the shelf (or the open Settings window)
      comes forward.
- [ ] Win+Alt+C opens the shelf too (search focus arrives in Phase 6).
- [ ] With Windows "Animation effects" off, the shelf appears and disappears without sliding.
- [ ] Alt+F4 on the shelf slides it away instead of quitting.

## Global shortcuts and keyboard layouts (Phase 1–3)

- [ ] On Turkish Q: Win+Alt+V toggles the shelf, Win+Alt+C opens it with search focused, Win+Alt+S starts a snip.
- [ ] With the app running, AltGr+Q/E/T still type @ € ₺ on Turkish Q, and AltGr+F/E/T on Turkish F.
- [ ] After Win+Alt+V and Enter, Notepad's menu bar and Word's KeyTips are not left active.
- [ ] Recorder: a combination already used by another app shows a warning and is not taken over.
- [ ] Letter shortcuts inside the shelf (P, S, F, Shift+I/T/L) work on Turkish Q and Turkish F.

## Capture (Phase 2)

The tray tooltip shows the item counts per kind; use it to see what was captured.

- [x] First run: the welcome window lists the sources, says text is collected, and capture starts
      only after "Start"; the window then closes (2026-10-29, Windows 11 25H2).
Automated run on the development PC (2026-10-29, Windows 11 25H2, debug build): a script set the
clipboard through .NET and the database was checked afterwards; the test items were then deleted.

- [x] A text with Turkish characters is captured once; copying it again (also with a trailing CRLF)
      moves the same card forward and keeps the newest bytes.
- [x] A whole-URL copy becomes a link; a sentence containing a URL stays text.
- [x] Whitespace-only text and an Explorer-style file copy capture nothing.
- [x] Copies carrying `ExcludeClipboardContentFromMonitorProcessing` or
      `CanIncludeInClipboardHistory = 0` (what password managers set) are not captured.
- [x] A copied bitmap is captured once (DIB path) with `original.png` and `thumb.jpg`; copying it
      again adds nothing. The source app is recorded.

Still to check by hand:

- [ ] Copy a sentence in Notepad: texts +1. Copy it again: the count stays the same.
- [ ] Copy an image in a browser ("Copy image", PNG path): images +1.
- [ ] Win+Shift+S capture: images +1 exactly once (clipboard and auto-saved file merge).
- [ ] Win+PrtScn: images +1; the file in the Screenshots folder is unchanged and never moved.
- [ ] The Screenshots folder is found although Pictures is redirected to OneDrive and localised.
- [ ] Copies from KeePass, KeePassXC, Bitwarden and 1Password are not captured.
- [ ] Copying files in Explorer captures nothing.
- [ ] Tray → Pause capture → 5 minutes: icon greys out, copies are ignored, capture resumes by
      itself after 5 minutes. "Until I resume" survives a restart.
- [ ] Capture keeps working after sleep and resume.
- [ ] Content already on the clipboard when a source is enabled is not captured.
- [ ] `%APPDATA%\Panopin\media\<id>\` holds `original.*` and `thumb.*` for each image.

## Shelf, focus and paste (Phase 3)

Checked in a browser with the real shelf components, sample items and a mocked backend
(2026-10-29): three rows with counts, tape and pins, tilt, 1–9 badges on the active row, arrow
moves between rows, P pins with the pin animation, S takes a card down (the paper falls and the
next card is selected), Backspace brings it back, Delete asks over the board and Enter confirms,
F shows pinned items only, Shift+I hides the Images row, the right-click menu, the paste notice,
the empty board, dark theme, small and large cards, and the board at the bottom edge. Everything
below needs the real app on Windows.

The board:

- [ ] Win+Alt+V shows Images, Texts and Links with their counts; the first image is selected.
- [ ] ←/→, ↑/↓ (keeps the column, skips empty rows), Tab, Ctrl+←/→, Alt+←/→ and 1–9 move as
      expected; Shift+arrows and Ctrl+A select several cards.
- [ ] A click copies, shows "Kopyalandı" and the board stays open; Ctrl+click and Shift+click select.
- [ ] Right click and Shift+F10 open the menu with every action; Esc closes the menu first.
- [ ] P pins and unpins; S takes down (a pinned card stays in the database, a temporary one goes to
      Trash); Backspace undoes; Delete asks first and Backspace undoes it too.
- [ ] F shows pinned items only; Shift+I/T/L hide rows; hiding the last row closes the shelf.
- [ ] Settings → Shelf: card size S/M/L and "Slides in from: Bottom" apply at once.

Copy, focus and paste:

- [ ] Enter copies, closes the board and focus returns to the previous app; Ctrl+V pastes there.
- [ ] Ctrl+Enter pastes into Notepad, Word, Chrome, Edge and VS Code.
- [ ] Three texts selected with Shift+→: they are pasted in selection order, one per line.
- [ ] One image: pastes into Paint and Word at full resolution; a transparent PNG keeps its
      transparency where the app supports PNG.
- [ ] Two images (Ctrl+click), Ctrl+C, paste in Explorer: two files named
      `Panopin-<date>-<n>.<ext>`; Paint pastes the first image.
- [ ] Copies made from the shelf are not captured again and do not move their card.
- [ ] Ctrl+Enter after closing the previous window: the board says nothing was pasted.
- [ ] Ctrl+Enter into an elevated (administrator) window explains why, and Ctrl+V there works.
- [ ] Ctrl+Enter into a password field (a browser login form): refused, the board reopens with the
      reason.
- [ ] After closing the shelf with Win+Alt+V in Notepad, releasing Alt does not activate the menu
      bar; same for Word's KeyTips.
- [ ] Clicking the taskbar while the board is open closes it without moving focus elsewhere.
- [ ] Starting Panopin.exe again from Explorer while it runs brings the shelf up with focus.

Drag:

- [ ] Drag a text card into Notepad: the text is inserted.
- [ ] Drag an image card into Explorer: a file appears; into a chat app: it is attached.
- [ ] Letting go over the board itself cancels; nothing is dropped into the app underneath.

Windows and sound:

- [ ] Two monitors at 100 % and 150 %: the shelf opens on the cursor's monitor and renders sharp.
- [ ] With Windows "Animation effects" off, the shelf appears without sliding and cards do not move.
- [ ] Rapid open/close never leaves the shelf hidden while it should be open.
- [ ] Sounds (open, close, copy, pin, unpin, drop, back) are short and quiet; Settings turns them
      off and sets the volume.
- [ ] Sounds are silent while a full-screen video, game or presentation is active.
- [ ] Do Not Disturb on: note whether sounds stay silent (known limitation, D-021).
- [ ] No black background on the transparent overlay (Windows 10 22H2 and Windows 11).

## Preview and editing (Phase 4)

Checked in a browser with the real preview components, generated pictures and a mocked backend
(2026-10-29): Space opens the first image fitted, with Mark up and Crop; M, then a rectangle, an
arrow and a pixelated area are stored in image pixels; Ctrl+Z and Ctrl+Shift+Z; C, dragging a
corner handle, Enter applies the crop and Esc saves; the view then shows the cropped picture with
its marks; ↓ shows the first text, E edits it and the text (with Turkish characters) is saved
300 ms after typing; ↓ shows the link card and Esc closes the preview. Automated: the Rust renderer
is checked pixel by pixel (strokes, crop, pixel averages, highlighter, text bitmaps, arrow heads),
and a 4K flatten takes about 0.45 s in a release build. Everything below needs the real app.

- [ ] Space opens and closes the preview; ←/→ and ↑/↓ move through the cards; a double click opens it.
- [ ] Image: the wheel zooms from 100 % to 400 %, dragging pans, a double click resets.
- [ ] Drawing on a 4K and an 8K screenshot stays smooth, and fast pen strokes keep their shape.
- [ ] Pen, highlighter (see-through, no darker overlaps), rectangle and ellipse (Shift: square,
      circle), arrow and line (Shift: 45°), text (Enter or Esc commits, double click edits it
      again), pixelate.
- [ ] Select (V): moving, the 8 handles (2 on lines and arrows), Delete; colour and thickness
      restyle the selected mark.
- [ ] Ctrl+Z, Ctrl+Shift+Z, Ctrl+S; Esc saves and leaves the editor.
- [ ] Crop (C): 8 handles, Enter applies, Esc cancels. "Revert to original" brings back the
      untouched picture and its thumbnail.
- [ ] Enter or Ctrl+C on an edited image pastes the edited picture at full resolution into Paint
      and Word; the card thumbnail shows the edit; dragging it out gives the edited picture too.
- [ ] Text: E or a click edits, saved about 300 ms after typing; Ctrl+Enter or Esc finishes; the
      card updates; copying the new text later moves the same card forward.
- [ ] Taking down the last card while the preview is open leaves the empty preview with "Undo".

## Appearance and accessibility (Phase 3–5)

- [ ] Mica on Windows 11; opaque fallback on Windows 10 22H2.
- [ ] High contrast themes keep cards, focus rings and text readable.
- [ ] Narrator reads card types, titles and actions in English and Turkish.

## Updates and data safety (Phase 7)

- [ ] Updating from the previous release keeps all data and settings.
- [ ] The installer runs without administrator rights; SmartScreen steps match the README.
