# Changelog

All notable changes to Panopin are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.0] - 2026-10-30

### Added

- **Eight more languages.** German, Spanish, French, Brazilian Portuguese, Russian, Japanese, Korean
  and Simplified Chinese join English and Turkish. Choose the language in Settings or in the
  first-run window; by default Panopin follows the system and falls back to English. The
  translations were made by machine and are waiting for native speakers to review them;
  `CONTRIBUTING.md` explains how to help. The Windows installer and the panels macOS draws for
  Panopin follow the same language, and on Windows keys carry their keyboard names, such as Strg
  in German or Maj in French.

### Fixed

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

[Unreleased]: https://github.com/MehmetBurakTaskaya/panopin/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/MehmetBurakTaskaya/panopin/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/MehmetBurakTaskaya/panopin/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/MehmetBurakTaskaya/panopin/releases/tag/v1.0.0
