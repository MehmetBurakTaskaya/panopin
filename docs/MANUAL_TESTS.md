# Manual tests

Checks that automated tests cannot cover: real focus and paste, global shortcuts, multiple monitors,
DPI, sleep/resume, sounds. Each phase adds or ticks items. Record the date, Windows build and result
next to each item when it is run.

## Live run, 2026-10-30

The installed release build (v1.0.0, per-user NSIS install, Windows 11 25H2 build 26200, one
monitor at 100 %, Turkish Q) was driven by scripts: simulated keys sent only to a test window or to
Panopin, UI Automation to read and press controls, the database to check results, and the
clipboard put back afterwards. 90 checks passed in the end: install 0.9.9, find 1.0.0 on a local
update server, "Update and restart", data kept, uninstall (14 + 5), and Phases 1–6 (71). Items
below marked "(live 2026-10-30)" were covered; notes say when only part of an item was.

The run found four bugs, all fixed and checked again:

- Settings and the Library opened blank from the palette, L, the tray and shortcuts (D-070).
- Win+Alt+C focused the search box only the first time; afterwards typed letters reached the
  shelf's letter keys, and S took a card down (the search box now takes the focus as it appears).
- Item ids could be given again after a delete for good, showing an old picture (D-071).
- Pictures stayed in the webview's disk cache after their items were deleted (D-072).

The same scripts ran again on a 1.0.0 build that includes the macOS work (commit fd9a49c),
installed over the previous one with the data kept: Phases 1–6 (53), the extra checks (21, now
also the card menu naming Ctrl+C, Ctrl+Enter and Delete) and a watched folder (4: a new picture
there becomes an image card, a text file does not, the settings come back unchanged). 78 checks
passed. In the first pass another window took the focus during three shelf sections; those
sections passed when run again. Installing, updating and uninstalling were not repeated.

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
- [x] Clicking outside the board (desktop, taskbar, another window) closes the shelf. (live 2026-10-30: beside the
      board; the taskbar and other windows not)
- [ ] A tray click while the shelf is open closes it and does not reopen it.
- [ ] Settings → Dil: switching to English updates Settings, the shelf and the tray menu at once, and
      survives a restart.
- [x] Starting Panopin a second time does not start a copy; the shelf (or the open Settings window)
      comes forward. (live 2026-10-30)
- [x] Win+Alt+C opens the shelf too (search focus arrives in Phase 6). (live 2026-10-30)
- [ ] With Windows "Animation effects" off, the shelf appears and disappears without sliding.
- [x] Alt+F4 on the shelf slides it away instead of quitting. (live 2026-10-30)

## Global shortcuts and keyboard layouts (Phase 1–3)

- [ ] On Turkish Q: Win+Alt+V toggles the shelf, Win+Alt+C opens it with search focused, Win+Alt+S starts a snip.
- [x] With the app running, AltGr+Q/E/T still type @ € ₺ on Turkish Q, and AltGr+F/E/T on Turkish F.
      (live 2026-10-30: Turkish Q; Turkish F not checked)
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
- [x] Copy an image in a browser ("Copy image", PNG path): images +1. (live 2026-10-30: a "PNG"
      clipboard format as browsers write it)
- [ ] Win+Shift+S capture: images +1 exactly once (clipboard and auto-saved file merge).
- [ ] Win+PrtScn: images +1; the file in the Screenshots folder is unchanged and never moved.
- [ ] The Screenshots folder is found although Pictures is redirected to OneDrive and localised.
- [ ] Copies from KeePass, KeePassXC, Bitwarden and 1Password are not captured.
- [x] Copying files in Explorer captures nothing. (live 2026-10-30: a file list on the clipboard)
- [ ] Tray → Pause capture → 5 minutes: icon greys out, copies are ignored, capture resumes by
      itself after 5 minutes. "Until I resume" survives a restart.
- [ ] Capture keeps working after sleep and resume.
- [ ] Content already on the clipboard when a source is enabled is not captured.
- [x] `%APPDATA%\Panopin\media\<id>\` holds `original.*` and `thumb.*` for each image. (live 2026-10-30)

## Shelf, focus and paste (Phase 3)

Checked in a browser with the real shelf components, sample items and a mocked backend
(2026-10-29): three rows with counts, tape and pins, tilt, 1–9 badges on the active row, arrow
moves between rows, P pins with the pin animation, S takes a card down (the paper falls and the
next card is selected), Backspace brings it back, Delete asks over the board and Enter confirms,
F shows pinned items only, Shift+I hides the Images row, the right-click menu, the paste notice,
the empty board, dark theme, small and large cards, and the board at the bottom edge. Everything
below needs the real app on Windows.

The board:

- [x] Win+Alt+V shows Images, Texts and Links with their counts; the first image is selected. (live 2026-10-30)
- [ ] ←/→, ↑/↓ (keeps the column, skips empty rows), Tab, Ctrl+←/→, Alt+←/→ and 1–9 move as
      expected; Shift+arrows and Ctrl+A select several cards.
- [ ] A click copies, shows "Kopyalandı" and the board stays open; Ctrl+click and Shift+click select.
- [ ] Right click and Shift+F10 open the menu with every action; Esc closes the menu first.
- [x] P pins and unpins; S takes down (a pinned card stays in the database, a temporary one goes to
      Trash); Backspace undoes; Delete asks first and Backspace undoes it too. (live 2026-10-30; S on
      a pinned card not checked)
- [ ] F shows pinned items only; Shift+I/T/L hide rows; hiding the last row closes the shelf.
- [ ] Settings → Shelf: card size S/M/L and "Slides in from: Bottom" apply at once.

Copy, focus and paste:

- [x] Enter copies, closes the board and focus returns to the previous app; Ctrl+V pastes there. (live 2026-10-30)
- [ ] Ctrl+Enter pastes into Notepad, Word, Chrome, Edge and VS Code.
- [x] Three texts selected with Shift+→: they are pasted in selection order, one per line. (live 2026-10-30)
- [ ] One image: pastes into Paint and Word at full resolution; a transparent PNG keeps its
      transparency where the app supports PNG.
- [ ] Two images (Ctrl+click), Ctrl+C, paste in Explorer: two files named
      `Panopin-<date>-<n>.<ext>`; Paint pastes the first image.
- [x] Copies made from the shelf are not captured again and do not move their card. (live 2026-10-30)
- [x] Ctrl+Enter after closing the previous window: the board says nothing was pasted. (live 2026-10-30)
- [ ] Ctrl+Enter into an elevated (administrator) window explains why, and Ctrl+V there works.
- [x] Ctrl+Enter into a password field (a browser login form): refused, the board reopens with the
      reason. (live 2026-10-30: a Windows password box; a browser form not checked)
- [ ] After closing the shelf with Win+Alt+V in Notepad, releasing Alt does not activate the menu
      bar; same for Word's KeyTips.
- [ ] Clicking the taskbar while the board is open closes it without moving focus elsewhere.
- [x] Starting Panopin.exe again from Explorer while it runs brings the shelf up with focus. (live 2026-10-30)

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
- [x] Text: E or a click edits, saved about 300 ms after typing; Ctrl+Enter or Esc finishes; the
      card updates; copying the new text later moves the same card forward. (live 2026-10-30: E,
      typing and Esc)
- [ ] Taking down the last card while the preview is open leaves the empty preview with "Undo".

## Library (Phase 5)

Checked in a browser with the real Library and shelf components and a mocked backend
(2026-10-29): sections and their counts; arrows, Home/End, Shift ranges and Ctrl+A in the grid and
in the list; P; Alt+P, Shift+P and Ctrl+Shift+T in the Library and on the shelf; Delete with its
question, Backspace undo, R and permanent delete in Trash; Ctrl+N (the note editor opens and the
text saves); Ctrl+Shift+N, F2 and deleting a folder, with the focus back on the tree; deleting a
tag; sorting the list by a column; the sidebar width kept in settings; a card dropped on a folder;
the details saving a title; L and Ctrl+Shift+E on the shelf. No renders while idle. Automated:
folders, tags (Turkish i forms), the saved rule, Trash and notes in Rust; filters, sorting and the
keyboard model in vitest. Everything below needs the real app.

- [x] Tray → Open Library and L on the shelf (the shelf closes): one window, brought forward if open.
      (live 2026-10-30: L; the tray not checked)
- [ ] Mica on Windows 11; an opaque window on Windows 10; light, dark and high contrast.
- [ ] After a restart the window keeps grid or list, sorting, panel widths and column widths.
- [ ] Sections: Recent shows only unsaved items; Pinned; Unsorted (saved, without folder or tag);
      nested folders and tags with counts; Trash.
- [ ] Ctrl+0–3 kind filter, F pinned only, tag filters (all must match), date and site filters.
- [ ] With 10 000 items the grid and the list scroll smoothly and typing in the details stays quick.
- [ ] Enter and Ctrl+C copy and the window stays open; Space preview; E, M and C editors.
- [ ] P, Alt+P (several items one after another; Esc stops), Shift+P (Ctrl+N makes a folder) and
      Ctrl+Shift+T (new and existing tags; "İş" and "iş" are one tag), in the Library and on the shelf.
- [ ] Details: title and note save on their own; when a save fails the typed text stays.
- [x] Ctrl+N note: pinned, the editor opens, it never expires; in a folder view it joins the folder.
      (live 2026-10-30: outside a folder)
- [ ] Ctrl+Shift+N folder, F2 rename, Delete (asks; the items stay); renaming a tag to an existing
      name shows an error; deleting a tag removes it from every item.
- [ ] A card dropped on a folder joins it and keeps its pin; Alt+drag to Explorer gives .txt, .url
      and .png files.
- [ ] Delete → Trash; Backspace and Ctrl+Z bring it back; in Trash R restores, Delete asks and
      deletes for good, Empty Trash asks.
- [ ] Ctrl+Shift+E in the Library and on the shelf: the folder dialog, files named
      Panopin-<date>-<n>, nothing overwritten; the shelf stays open behind the dialog.
- [ ] F6 and Shift+F6 cycle sidebar, items and details; Tab moves between controls; Narrator reads
      the tree, the items and the selection.

## Search, OCR, links, recordings, palette and Settings (Phase 6)

Checked on the development PC (2026-10-29): Windows OCR read a sample picture with Turkish text
word for word; real link previews came back for example.com, a GitHub repository and a YouTube
video through the network guard; a Windows sample MP4 gave its poster, length and frame size.
Checked in a browser with the real components and a mocked backend: the shelf search (box, counts,
chips, marks, Enter to the results, Esc), the Library search with relevance sorting, the palette
on the shelf, in the Library and in Settings, and every Settings tab. Automated: search folding
and the 10 000-item timing, the network guard (SSRF), page parsing, OCR and link queues, range
answers for recordings, shortcut recording and the Settings flows. Everything below needs the
real app.

- [ ] Shift+S and Win+Alt+C open the search box below the board; "istanbul" finds "İstanbul",
      "ışık" finds "IŞIK"; with "Ignore accents" on, "cicek" finds "çiçek"; hidden rows stay out.
      (live 2026-10-30: Win+Alt+C and "istanbul"; the rest not yet)
- [ ] Ctrl+F in the Library searches with the section and filters; "Relevance" sorts while searching.
- [ ] Ten minutes after copying screenshots with text, the text is found by search; Ctrl+Shift+C
      copies it; a pixelated area of an edited image is not found.
      (live 2026-10-30: read within seconds, found and copied; the pixelated area not checked)
- [ ] With link previews on: a news page, a GitHub page and a YouTube video get title, site,
      picture and icon on the card and in the preview; Ctrl+O opens the browser; "Refresh preview".
      With them off, nothing goes out (check with a network monitor if possible).
- [x] Links to localhost, 192.168.x.x, http:// or with ?token= get no preview. (live 2026-10-30:
      http://localhost; the rest by the Rust network-guard tests)
- [ ] Screen recordings on: a Snipping Tool recording appears in the image row with poster and
      length; Space plays it, seeking works; Ctrl+C pastes it as a file in Explorer; drag and export.
- [ ] "Import…" adds pictures and MP4 files; the originals are untouched.
- [ ] Win+Alt+S opens the Snipping Tool; the snip opens on the shelf in the crop editor, with focus.
- [ ] Settings → Shortcuts: record Win+Alt+L for the Library and use it; Esc cancels; Backspace
      turns it off; keys another app holds are refused; Restore the defaults.
- [ ] Copy the last image and pause/resume through their shortcuts (the tray icon changes).
- [ ] Ctrl+K on the shelf, in the preview, in the Library and in Settings; each command runs on the
      selection of the moment.
      (live 2026-10-30: the shelf and Settings palettes; the preview and the Library not yet)
- [ ] Settings → General: Start with Windows (sign out and in), edge opening (top and bottom edge,
      not during a full-screen video), accent-free search.
- [ ] Settings → Appearance: theme, Windows accent colour (change it in Windows; Panopin follows),
      card size, Mica on and off while the Library is open, animations off.
- [ ] Settings → Sources and History: watched folders, retention, Clear history counts, asks and
      moves only unsaved items. Privacy: excluded apps, pause, link previews, OCR language hint.
- [ ] Settings → About: version, links, third-party licences.
- [x] Idle CPU stays near 0 % with OCR finished and previews off; Task Manager memory is noted.
      (2026-10-30, release build: 0.2 s of CPU per minute, about 120 MB private working set)

## Appearance and accessibility (Phase 3–5)

- [ ] Mica on Windows 11; opaque fallback on Windows 10 22H2.
- [ ] High contrast themes keep cards, focus rings and text readable.
- [ ] Narrator reads card types, titles and actions in English and Turkish.

## Release, updates and website (Phase 7)

Checked on the development PC (2026-10-30): a local release build signed with the update key;
the Settings update states, the tray's tab request and the release-note text in automated tests;
the website and the embed in a browser (desktop, 375 px phone emulation, dark mode, a host page
with conflicting styles). Everything below needs a Windows session.

- [x] The installer runs without administrator rights, installs to `%LOCALAPPDATA%\Panopin`, adds
      a Start menu entry and speaks Turkish on a Turkish Windows; SmartScreen steps match the
      README. (live 2026-10-30, silent install: the installer's pages and SmartScreen not seen)
- [x] Opened as a double-click opens it, the installer installs without a single click: only a
      progress bar shows, a running Panopin is closed, and Panopin starts when it is done; items
      and settings stay. (live 2026-10-30: over the installed copy and after an uninstall; the
      SmartScreen prompt of a downloaded copy not seen)
- [ ] The tray icon reads well at 100 %, 125 %, 150 % and 200 % scaling on a light and a dark
      taskbar, in the normal, paused and update states.
- [ ] Settings → About → Licence texts opens `THIRD_PARTY_NOTICES.txt`.
- [x] Updates off: no request to github.com (network monitor). On: a newer published release puts
      a dot on the tray icon and "Update to x…" in its menu; Settings shows the notes; "Update and
      restart" installs it and Panopin comes back with every item, pin, folder and setting.
      (live 2026-10-30: 0.9.9 found 1.0.0 on a local server, showed the notes, updated, restarted and
      kept every item and setting, the database moving from schema 7 to 8; the tray dot and the
      network monitor not checked)
- [ ] The portable ZIP runs from any folder and its Licence texts button works. (Since 1.3.0 it
      updates itself; see "Updates that install by themselves".)
- [x] Uninstalling without "Delete the application data" keeps `%APPDATA%\Panopin`; with it, the
      folder is gone. The "Start with Windows" entry is removed both times. (live 2026-10-30: without
      it; ticking it would have deleted the development data, so not checked)
- [x] Tag `v1.0.0`: the draft release has `Panopin-Setup-x64.exe`, `Panopin-Portable-x64.zip`,
      `latest.json` and `SHA256SUMS.txt`, and the hashes match the downloads. (2026-10-30: also
      the update signature in `latest.json` checked against the app's public key, and the
      release's installer opened like a double-click over the installed copy: no click needed,
      Panopin started again, data kept)
- [ ] The website on Windows offers "Download for Windows", on a phone "See all downloads"; the
      language switch remembers its choice. The embed shows both languages on
      mehmetburaktaskaya.com.

## macOS (Phase 8)

CI builds the app on `macos-latest`, runs the Rust tests there (the folder watcher and the
screenshot mark on a real file system among them) and checks the code with clippy. macOS 13 or
later. **Mac live test, 2026-10-30** (Actions → Mac live test, `tests/macos/live.sh`, D-073): the
app built for the runner's chip, installed from its `.dmg` and driven by script on macOS 14.8,
15.7 and 26.6 (Apple silicon) and 15.7 (Intel); all 47 checks passed on each of the four.
The runs for 1.2.0 missed one copy now and then, each time a different one (a PNG and the first
text on macOS 26, a TIFF on macOS 15 Intel): a copy read between an app clearing the pasteboard
and writing to it looked empty and was never read again. With the listener reading such a
change again, and the script waiting for the shelf's and the Library's texts, 48 checks passed
on each of the four (commit 2648861).
The script presses keys through System Events, so the shortcuts are synthetic, and it grants
Accessibility and Desktop access through the privacy database instead of macOS's prompts; since
D-078 it binds them to the app's designated requirement, as System Settings does. Items
below marked "(live Mac)" were covered; notes say when only part of one was. On a real Mac (a
friend's, 2026-10-30) the first try stopped at macOS's "could not verify" dialog, the step the
README now spells out (D-075); nothing further has been tried there yet. To try a test build, start Actions → Mac test build → Run workflow and
download the `.dmg` from the run's artifacts (kept for 14 days).

- [x] The `.dmg` opens; Panopin starts after "Open Anyway" in System Settings → Privacy &
      Security. The menu bar shows the pushpin, tinted for light and dark menu bars; there is no
      Dock icon. (live Mac: installed from the `.dmg` by script, so the "Open Anyway" step was not
      seen; the code signature verifies; Launch Services lists Panopin as a menu bar app, the
      Dock has no icon for it, and the icon shows in light and dark screenshots)
- [x] Copied text, links and pictures (TextEdit, Safari, Preview, Cmd+Ctrl+Shift+4) become
      cards. Copies a password manager marks as concealed or transient are not kept; Panopin's
      own copies are not captured again. The source app shows in the details. (live Mac: text
      with Turkish letters, a link, a PNG and a TIFF, which is kept as PNG; concealed, transient
      and 1Password copies skipped; the source app is TextEdit; Safari and Preview not tried)
- [x] Cmd+Shift+3 and Cmd+Shift+4 screenshots on the Desktop reach the shelf; other pictures saved
      to the Desktop do not. After choosing another folder in the Screenshot app's Options and
      restarting Panopin, screenshots there are kept. A folder added in Settings takes every
      picture. (live Mac: Cmd+Shift+3 and the added folder; Cmd+Shift+4 and another screenshot
      folder not tried)
- [x] Cmd+Option+V opens the shelf on the current Space and over a full-screen app;
      Cmd+Option+C opens it with search; Cmd+Option+S starts the system's capture and opens the
      result in the crop editor. (live Mac: the shelf takes the keyboard while TextEdit stays in
      front, also over full-screen TextEdit; Cmd+Option+S starts the capture, which was then
      cancelled)
- [x] Enter pastes into the previous app. The first time, macOS asks for Accessibility
      permission; without it the item stays on the clipboard and the shelf says how to allow it.
      With it, Cmd+V reaches TextEdit, Notes, Safari and Terminal; a password field is never
      pasted into. (live Mac: Enter copies and returns to TextEdit, Cmd+Enter pastes into it with
      Accessibility allowed; the first request, Notes, Safari, Terminal and password fields not
      tried)
- [x] Words in a screenshot are found by search (Vision), in Turkish and English. (live Mac: Vision
      read "Kestane" and a number in a copied picture, and search found the picture; Turkish not
      tried)
- [x] No text names Windows, Ctrl or "this PC": Settings says "Open at login", "Same as macOS"
      and "Use the macOS accent colour"; hints and menus show ⌘Enter, ⌥⌘V, ⇧⌘T and ⌘⌫; edge
      opening is shown as not available. (live Mac: every Settings tab, the Library, the shelf's
      hints and the first-run window read through Accessibility; the Library named Ctrl+F before
      1.1.0)
- [x] ⌘⌫ moves the selected cards to Trash on the shelf and in the Library; ⌫ alone undoes. (live
      Mac: on the shelf, after its question; the Library not tried)
- [ ] With "Use the macOS accent colour" on, the accent follows System Settings → Appearance
      after the window is opened again. (live Mac: inconclusive; the script changed the setting
      without the notice System Settings sends, and the accent stayed blue)
- [x] Export… (Library and shelf), Import… and Settings → Sources → Add folder open the macOS
      panel above the window that asked; Import offers only pictures and MP4 files; Cancel
      changes nothing. (live Mac: Add folder and Cancel; Export and Import not tried)
- [x] Settings → Start at login adds and removes the login item. (live Mac: through the setting
      and a restart)
- [ ] An update from `latest.json` installs the universal app and Panopin restarts.
- [x] Every build is signed with one certificate, so its designated requirement names the
      certificate, not the build (D-078). (live Mac, 2026-10-30, runs 41095683679 and
      41098617458 on a test branch with these changes, macOS 14.8, 15.8, 15.7 Intel and 26.6, with
      a certificate made for each run: `codesign` signed with it although nothing on the runner
      trusts it, and the requirement was
      `identifier "com.mehmetburaktaskaya.panopin" and certificate root = H"…"`; 48 of 48 in the
      live test with the permissions bound to it)
- [x] An update keeps Accessibility and Desktop access: after Panopin updated itself to a bundle
      signed again with the same certificate, ⌘Enter still pasted into TextEdit and a ⇧⌘3
      screenshot on the Desktop still became a card. (live Mac, same runs: 10 of 10 in the update
      step on each runner)
- [x] The privacy database holds to the requirement: a test app given Accessibility in one build
      keeps it in a second build signed with the same certificate, loses it in an ad-hoc signed
      build, and of two ad-hoc builds only the first has it; a permission stored without a
      requirement, as the live test stored it before, covers any build. (live Mac, same runs:
      10 of 10 in `tests/macos/permission.sh` on each runner)
- [ ] On a real Mac: allow Accessibility, let Panopin update itself, and ⌘Enter still pastes; the
      first update from 1.3.0, which is ad-hoc signed, asks once more and later ones do not.
- [x] A build signed with the project's certificate from the repository secrets (Actions → Mac
      test build) has the designated requirement in `src-tauri/macos-requirement.txt`.
      (2026-10-30, run 41119078075 on 5d53db4: the universal app and its `.dmg` signed with
      "Panopin", SHA-1 C853290F…3ECC, which `security find-identity` lists as not trusted; the
      requirement matched the file exactly)

## Languages (D-074)

Checked on 2026-10-30 in a browser (Chromium on Windows 11, display at 125 %) with the real
windows and a mocked backend, in all twelve languages, with English as the baseline. A script looked
at every visible text and picker: Settings at 820×640 and at its smallest size, 640×480, on every
tab; the first-run window; the Library; the shelf, its card menu and the empty board. Screenshots
of each window were then read by eye for the font and the line breaks.

- [x] No text is cut off or runs over its box in any language, German lengths included (the row
      tags, the Settings sidebar, the hint line under the board, the Library toolbar, which wraps
      to a third row in French and Spanish).
- [x] Russian shows in the regular UI font, which covers Cyrillic; Chinese, Japanese and Korean
      show their own Windows UI fonts (Microsoft YaHei UI, Yu Gothic UI, Malgun Gothic) with no
      missing glyphs, and the page's `lang` follows the language.
- [x] Korean wraps between words, not inside them. (fixed during this check)
- [x] German, French, Spanish and Italian write keys with their keyboards' names in the card menu,
      the hints, the Library and Settings → Shortcuts: Strg+Eingabe, Strg+Umschalt+T, Entf;
      Ctrl+Maj+C, Suppr, Échap; Ctrl+Intro, Mayús, Supr; Ctrl+Invio, Maiusc, Canc. Other languages
      keep Ctrl, Shift, Enter.
- [x] Arabic mirrors every window: the shelf's rows start at the right (labels and their tape
      stripe on the right, card 1 the rightmost, number badges at the cards' left corners),
      Settings' tabs and the Library's sidebar sit on the right, toggles and the volume slider
      mirror, and the card menu opens leftwards from the card. Arabic letters stay joined (no
      letter spacing) in Segoe UI, numbers and dates use Western digits.
- [x] Arabic keys follow the screen: Left moves to the card on the left, Right back, Ctrl+Right to
      the first card; moving to the last card scrolls the row left until it is whole, and the
      wheel scrolls towards the row's end.
- [x] Copied text keeps its own direction: Latin and Turkish cards, Library names and sizes
      (1920×1080) read left to right in the Arabic window, cut at their end.
- [x] Settings → General and the first-run window list the twelve languages by their own names;
      Settings shows the review note under the choice for every language except English.
- [x] In the installed app, "System default" follows the Windows display language, and choosing a
      language in Settings changes the open windows at once. (live 2026-10-30, the 1.2.0 release
      installer over 1.0.0 with the data kept: on Turkish Windows "System default" shows Turkish;
      German, Russian, Arabic, Japanese, Korean and Chinese retitle and redraw Settings, the shelf
      and an open Library, and the review note follows; 43 of 44 checks, the 44th, "Suchen
      (Strg+F)" in the Library's search box, is a placeholder UI Automation does not read and shows
      in the window capture; the setting was put back. The tray menu was not read.)
- [ ] The installer shows its pages in the Windows display language when it is one of the twelve.
- [x] With Mac texts in Arabic, ⌘ shortcuts in the hint line and the card menu are isolated, so
      they keep their order. (browser with a Mac user agent; not on a real Mac)
- [ ] On a Mac, the panels macOS draws (Open, Save, permission prompts) follow the language.

## Updates that install by themselves (D-076, D-077)

Checked on 2026-10-30 on the development PC (Windows 11 25H2 build 26200) with test builds of this
code as 1.2.8 and 1.2.9, pointed at a local update server and signed with the update key. Scripts
sent no keys and opened no windows; the release 1.2.0 was installed again afterwards.

- [x] The installed copy finds a newer version 30 s after start and, with the shelf closed and no
      Panopin window in front, installs it a minute later without any window: Panopin starts again
      as the new version, and items, folders, tags and settings stay. (live 2026-10-30: 1.2.8 →
      1.2.9; the installer ran from the updater's folder without a window, fetched as
      `Panopin-Update-x64.exe`, and the new version logged its start 2 s after the install began)
- [x] The portable copy does the same from the signed ZIP: `Panopin.exe` is replaced, the old one
      waits as `Panopin.old`, Panopin starts again and its next start removes `Panopin.old`; the
      licence texts stay and no `.new` file is left. (live 2026-10-30: 1.2.8 → 1.2.9 in 91 s; it
      fetched `latest.json` and the ZIP, not the installer)
- [ ] While the shelf is open or a Panopin window is in front, the update waits, and it installs
      about a minute after. (not run: it needs the keyboard and the focus)
- [ ] "Update and restart" installs a waiting update at once, also while it downloads by itself.
- [ ] An update whose signature does not match, or whose signed version differs from the one in
      `latest.json`, is refused and nothing changes.
- [x] On a Mac in Applications, an update installs by itself and Panopin starts again from the new
      bundle (Actions → Mac live test, its update step). (2026-10-30, run 41083660234 on 2faeacc:
      6 of 6 on macOS 14, 15, 15 Intel and 26, next to 48 of 48 in the live test; on GitHub's
      runners, not on a real Mac. With the certificate of D-078, runs 41095683679 and
      41098617458: 10 of 10, the permissions kept)
- [x] The release's own installer installs over the previous release with the data kept, and the
      new version's first update check against GitHub works. (2026-10-30: the 1.3.0 draft's
      installer over 1.2.0 on the development PC; the draft's checksums and all five update
      signatures, each signed for 1.3.0, checked against the app's public key)
- [ ] On a Mac whose Applications folder the user may not change, the update waits for "Update and
      restart", which asks for an administrator's password.
- [x] Publishing a release turns the older ones into drafts: their files answer 404 and the
      release list shows only the newest. (2026-10-30: v1.0.0 and v1.1.0 turned into drafts by hand;
      the workflow runs with the next release)
- [x] `npm run stats` prints the downloads per release and file. (2026-10-30)

## Search note and key list (D-080)

Checked on 2026-10-31 in the browser preview (`preview/shelf.html`, `preview/settings.html`, mocked
IPC, Turkish, light theme). The browser pane was hidden for part of the check, which stalls the
preview's timers. Dark theme, Arabic and the Mac's key names were therefore not looked at, and
nothing ran in the real app.

- [x] Shift+S pins the search note under the board, with a gap. Typing shows the matches in the
      rows, the count in red and the labels per row. Rows with no match fade. (preview 2026-10-31)
- [ ] Win+Alt+C opens the shelf with the search note under the board, not on it, also on a monitor
      where the board is tall.
- [x] ? and F1 pin the key note over the screen in three columns. Other keys do nothing while it is
      up (P pinned nothing). Esc takes it down and the board stays open. (preview 2026-10-31: ?
      sent from script, because the test browser does not type it as a key)
- [ ] ? opens the note on a Turkish Q keyboard (Shift+*) and on a German one (Shift+ß).
- [x] Settings → Shortcuts lists the shelf's and the Library's keys under the global shortcuts.
      (preview 2026-10-31)
- [ ] The key note and the search note in dark mode, in Arabic (right to left), and on a Mac with
      ⌘ and ⌥.

## Installing over a copy that antivirus holds (D-081)

On the development PC, which runs McAfee: its framework host keeps panopin.exe open even while
Panopin is closed.

- [x] The 1.4.0 draft built before this fix stopped with "Panopin could not be closed", when the
      owner double-clicked it with Panopin running and also with Panopin closed.
      (2026-10-31)
- [x] A build of the new hook closed a running Panopin in 250 ms and set the program aside.
      Tauri's check then listed no process, and the restore path put the program back.
      (2026-10-31)
- [x] An update (`/S /UPDATE /R`) installs over a running copy and starts Panopin again.
      (2026-10-31, the 1.4.1 draft over a running 1.3.0: done in 3 s. Panopin 1.4.1 started at
      once; the items, the Run entry and the folder were as before, with no set-aside file left)
- [x] The 1.4.1 installer opened by hand, which restarts itself with `/P /R`, installs over a
      running copy. Panopin starts again and the items stay. (2026-10-31, over a running 1.4.1)
- [ ] Uninstalling with Panopin running works, and nothing is left in the folder but what the
      antivirus still holds. (not run: it would remove the owner's copy; the hook it uses is the
      one tested above)

## Accents, code font, Library marks and folder totals (D-082, D-083)

Covered by unit tests on 2026-10-31 (Rust index and folding, TypeScript highlighting, code check,
folder totals). The items below need the real app.

- [ ] With "Search without accents" on, "cafe" finds "Café crème" on the shelf and in the Library,
      and "елка" finds "Ёлка"; with it off, neither does.
- [ ] After the update, the index rebuilds once in the background, and search keeps working.
- [ ] A copied piece of code shows in the monospace font with its indentation, on its card and in
      the Library. A sentence with a semicolon does not.
- [ ] A Library search marks its words in the tiles and in the list.
- [ ] A folded folder with subfolders counts their items too; unfolded, it shows its own count.

## Full round for v1.0.0

Run every unchecked item in this file on the installed v1.0.0 build, in this order: capture,
shelf, preview and editing, Library, search/OCR/links/recordings/Settings, appearance and
accessibility, release. Add the date, Windows build and result next to each item; a failure gets a
GitHub issue and blocks publishing the draft until it is fixed or accepted as a known issue in the
release notes.
