# Changelog

All notable changes to Panopin are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Paste one by one.** Choose several cards and press Shift+Enter: each paste, in any app, takes
  the next one, in the order you chose them. Ctrl+V, Shift+Insert and the Paste menu all count.
  When the last one is pasted it stays on the clipboard; copying anything else ends the queue
  sooner. Shift+Enter works in the Library too.
- **Code looks like code.** A copied text that looks like code shows in a monospace font, with its
  indentation, on the shelf and in the Library.
- A Library search marks the words it found in titles and texts, as the shelf does.
- A folded folder in the Library counts the items in its subfolders too.

### Changed

- **Search without accents works in every language.** "cafe" finds "café", "senor" finds "señor",
  and Greek, Russian and Arabic lose their accents the same way. Turn it on in Settings or with the
  box under the search. The search index is rebuilt once after the update.

## [1.4.1] - 2026-10-31

### Added

- **Every key in one place.** Settings → Shortcuts lists the keys of the shelf and of the Library.
  On the shelf, ? or F1 pins the shelf's keys on a note, and the line of keys along the board ends
  with "Keys ?".

### Changed

- **Search is a note pinned under the board.** Shift+S and Win+Alt+C write the search on a note.
  It counts the matches in each row and shows the keys that leave it.
- **On a Mac, updates keep the permissions you gave Panopin.** Every version is now signed with the
  same certificate, so macOS knows each update as the same app. After this one update, allow
  Panopin under Privacy & Security → Accessibility once more; macOS may also ask once more before
  Panopin reads the Desktop. From then on ⌘Enter keeps pasting after every update.

### Fixed

- When the shelf opened with the search (Win+Alt+C), the search box could land on top of the board.
- **Installing over Panopin works with antivirus software that keeps the program open**, as
  McAfee does. The installer, the one updates run included, stopped with "Panopin could not be
  closed", even with Panopin closed. Uninstalling works there too.

## [1.3.0] - 2026-10-30

### Changed

- **Panopin keeps itself up to date.** It looks for a new version at start and every 6 hours and
  installs it by itself while you are not using it: it closes, updates in place and starts again,
  and your items and settings stay. There is no switch for it any more; "Update and restart" in
  Settings → Updates installs a waiting version at once. On Windows the update runs without a
  window. On a Mac it works while Panopin is in Applications, and macOS may ask again for
  Accessibility afterwards.
- The portable copy updates itself too.
- Settings → Privacy and the first-run window say what goes online: the update check, which sends
  nothing about you or your copies, and link previews if you turn them on.

### Security

- An update must carry its version inside its signature, so an altered update list cannot offer an
  older release as a new one.

## [1.2.0] - 2026-10-30

### Added

- **Ten more languages.** German, Spanish, French, Italian, Brazilian Portuguese, Russian, Arabic,
  Japanese, Korean and Simplified Chinese join English and Turkish. Choose the language in Settings
  or in the first-run window; by default Panopin follows the system and falls back to English.
  Arabic lays the windows out from right to left. The translations were made by machine and are waiting for native speakers to review them;
  `CONTRIBUTING.md` explains how to help. The Windows installer and the panels macOS draws for
  Panopin follow the same language, and on Windows keys carry their keyboard names, such as Strg
  in German or Maj in French.

### Fixed

- On a Mac, a copy could be missed when Panopin looked at the clipboard while an app was still
  writing to it.
- Text copied in a right-to-left script, such as Arabic or Hebrew, now reads right to left on its
  card, in the preview and in the Library.
- The link preview choice in the first-run window had no room at its sides.

## [1.1.0] - 2026-10-30

### Added

- **macOS.** Panopin for macOS 13 or later, on Apple silicon and Intel: a menu bar app without a
  Dock icon that keeps copies and screenshots as on Windows. ⌥⌘V slides in the shelf, which takes
  the keyboard while the app you were in stays in front; ⌘Enter pastes there once Panopin is
  allowed under Accessibility. ⌥⌘C opens search, ⌥⌘S starts the system's screen capture, Vision
  reads the text in images, and texts, key names and the accent colour follow macOS. Screen
  recordings, dragging cards out and opening the shelf from the screen edge are not on the Mac yet
  (`docs/DECISIONS.md`, D-073).

### Fixed

- The Library names Mac keys on a Mac (⌘F, ⌘N and the others).

## [1.0.0] - 2026-10-30

The first release, for Windows 10 22H2 and Windows 11 (x64).

### Added

- **Capture.** Texts, links and images copied anywhere are saved silently, along with screenshots
  from the Screenshots folder, Snipping Tool recordings and extra watched folders. Copies that
  password managers and Windows mark as private are skipped, so are excluded apps; capture can be
  paused for 5 minutes, an hour or until resumed.
- **Shelf.** Win+Alt+V slides in a cork board with Images, Texts and Links rows. Arrows and 1–9
  choose, Enter copies and returns to the previous app, Ctrl+Enter pastes there, cards can be
  dragged out as files. Pinned cards get a pushpin, the rest are taped and expire on their own
  schedule; S takes a card down, Delete moves it to Trash, Backspace undoes.
- **Preview and editing.** Space previews any card. Texts can be edited; images can be marked up
  (pen, highlighter, shapes, arrows, text, pixelate) and cropped, with the original always kept.
- **Library.** A window with every item: folders, tags, names, notes, Trash, grid and list views,
  export, drag to folders, and the same keys as the shelf.
- **Search.** Turkish-aware full-text search (İ/ı, optional accent-free matching) on the shelf
  (Shift+S, Win+Alt+C) and in the Library (Ctrl+F), including text found in images.
- **Text in images.** Windows' own OCR reads every image in the background; Ctrl+Shift+C copies
  the text.
- **Link previews**, off until turned on: title, site, picture and icon, fetched by the app's core
  with strict network rules.
- **Screen snip.** Win+Alt+S opens the Snipping Tool and brings the snip to the shelf, ready to crop.
- **Command palette.** Ctrl+K lists every action with its shortcut on the shelf, in the Library and
  in Settings.
- **Settings.** General, Sources, History, Shortcuts (with a recorder for six global shortcuts),
  Appearance (theme, Windows accent colour, Mica, card size, animations), Privacy, Updates and
  About with third-party licences.
- **Updates**, off until turned on: a check at start and every 6 hours, a mark on the tray icon,
  and "Update and restart" when you choose. Packages are verified with the project's update key.
- **Installer and portable ZIP.** A per-user installer that needs no administrator rights and no
  clicks: opened, it installs and starts Panopin, in English or Turkish. And a portable ZIP.

[Unreleased]: https://github.com/MehmetBurakTaskaya/panopin/compare/v1.4.1...HEAD
[1.4.1]: https://github.com/MehmetBurakTaskaya/panopin/compare/v1.3.0...v1.4.1
[1.3.0]: https://github.com/MehmetBurakTaskaya/panopin/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/MehmetBurakTaskaya/panopin/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/MehmetBurakTaskaya/panopin/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/MehmetBurakTaskaya/panopin/releases/tag/v1.0.0
