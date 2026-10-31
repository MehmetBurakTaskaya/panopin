<p align="center">
  <img src="site/assets/panopin.svg" width="112" height="112" alt="Panopin icon: a note pinned to a cork tile">
</p>

<h1 align="center">Panopin</h1>

<p align="center">
  <strong>Everything you copy, pinned to a cork board.</strong><br>
  A fast, fully local clipboard and screenshot shelf for Windows and macOS.
</p>

<p align="center">
  <a href="https://github.com/MehmetBurakTaskaya/panopin/releases/latest">Download</a> ·
  <a href="README.tr.md">Türkçe</a> ·
  <a href="CHANGELOG.md">Changelog</a>
</p>

![Panopin at work: the cork board with its Images, Texts and Links rows over a mail draft. A search for "room" finds a screenshot by the text in it and a copied text, and Ctrl+Enter pastes the text into the draft. Then the board goes through its 12 languages.](site/assets/demo.gif)

Copy a text, a link or an image, or take a screenshot: Panopin keeps it, quietly. Press
**Win+Alt+V** and a cork board slides down with three rows — Images, Texts and Links. Choose with
the keyboard, press **Enter**, and the item is on your clipboard, the board is gone and you are
back where you were. Pin what you want to keep; the rest expires on its own schedule. A Library
window holds everything, with folders, tags, notes and search.

## Features

- **A cork board of your own.** Copies are taped to the board, the ones you keep get a pushpin,
  and small paper-and-cork sounds go with it (they can be turned off).
- **Native and light.** Built with Tauri and Rust. The installer is about 3.4 MB
  and Panopin uses almost no CPU while idle: the clipboard is followed through Windows events, never
  polled.
- **Respects privacy.** Copies that password managers and Windows mark as private are never
  saved. Excluded apps, and a pause for 5 minutes, an hour or until you resume.
- **Windows OCR built in.** Text inside screenshots becomes searchable in Turkish and English with
  the OCR that comes with Windows' language packs — no model to download.
- **Works with your screenshots.** Snipping Tool snips (Win+Shift+S), the Screenshots folder and
  screen recordings arrive on the board; Win+Alt+S snips straight into the crop editor.
- **Preview and edit.** Space previews a card. Edit texts; mark up images with a pen,
  highlighter, shapes, arrows, text and pixelation; crop. The original is always kept.
- **Fluent design.** Mica, your Windows accent colour, light and dark themes, per-monitor DPI and
  multiple monitors.
- **Made for Turkish too.** Search that knows İ and ı, optional accent-free matching, and default
  shortcuts that never block AltGr characters such as ₺, @ and €.
- **Twelve languages.** English, German, Spanish, French, Italian, Brazilian Portuguese, Turkish,
  Russian, Arabic (right to left), Japanese, Korean and Simplified Chinese. Panopin follows the
  system language; Settings can change it.
- **Keyboard first.** Every action has a key. On the shelf, ? shows them all; Settings →
  Shortcuts lists them, and Ctrl+K opens a command palette.
- **Easy to install.** A per-user installer that needs no administrator rights, and a portable
  ZIP.

## Install

1. Download **Panopin-Setup-x64.exe** from the
   [latest release](https://github.com/MehmetBurakTaskaya/panopin/releases/latest) and open it.
   Panopin installs right away for your user only, with no administrator rights and no pages to
   click through, and starts when it is done.
2. Panopin is not code-signed yet (certificates cost money; see [docs/SIGNING.md](docs/SIGNING.md)),
   so Windows SmartScreen may say "Windows protected your PC". Choose **More info → Run anyway**.
   Each release lists the SHA-256 of its files in `SHA256SUMS.txt` if you want to check your
   download.
3. Requirements: Windows 10 22H2 or Windows 11, x64, and Microsoft Edge WebView2, which Windows 11
   already has (on Windows 10 the installer fetches it when it is missing).

**Portable:** unzip **Panopin-Portable-x64.zip** anywhere and run `Panopin.exe`. Your data still
lives in `%APPDATA%\Panopin`, and the portable copy keeps itself up to date like the installed one.

**Updates:** Panopin keeps itself up to date. It looks for a new version on GitHub at start and
every 6 hours and installs it by itself while you are not using it: it closes, updates in place and
starts again, and your items and settings stay. "Update and restart" in Settings → Updates installs
a waiting version at once. Only the newest release can be downloaded.

**macOS:** download **Panopin-macOS.dmg** (macOS 13 or later, Apple silicon and Intel), open it and
drag Panopin into Applications, where it can keep itself up to date. Panopin is not notarised by
Apple, so macOS blocks its first opening:

1. Open Panopin. When macOS says it could not verify Panopin, close the warning with **Done** or
   **Cancel**, whichever your macOS shows (not Move to Trash).
2. Open **System Settings → Privacy & Security**, scroll down and choose **Open Anyway** next to
   Panopin. The button stays there for about an hour after you tried to open it.
3. Choose **Open Anyway** again and enter your password. From then on Panopin opens like any app.

Or, in Terminal: `xattr -dr com.apple.quarantine /Applications/Panopin.app`.

Panopin lives in the menu bar. macOS asks once before Panopin may read the Desktop, where screenshots are
saved; ⌘Enter pastes for you once Panopin is allowed under **Privacy & Security → Accessibility**.
Updates keep both permissions. On a Mac the shortcuts are ⌥⌘V, ⌥⌘C and ⌥⌘S.

## Shortcuts

Global shortcuts can be changed in Settings → Shortcuts.

| Action | Default |
| --- | --- |
| Open or close the shelf | Win+Alt+V |
| Open the shelf with search | Win+Alt+C |
| Snip the screen onto the shelf | Win+Alt+S |
| Library, copy the last image, pause capture | not set |

On the shelf:

| Key | Action |
| --- | --- |
| Arrow keys, 1–9 | Move between cards and rows, pick a card in the row |
| Enter | Copy, close the shelf and go back to your app |
| Ctrl+Enter | Paste into the app you came from |
| Shift+Enter | Paste several cards one by one: each paste takes the next |
| Ctrl+C | Copy and keep the shelf open |
| Space | Preview |
| P · Alt+P · Shift+P · Ctrl+Shift+T | Pin · name · folder · tags |
| S | Take off the shelf (unsaved items go to Trash) |
| Delete · Backspace | Move to Trash · undo |
| E · M · C | Edit a text · mark up an image · crop an image |
| Shift+S | Search |
| Shift+I · Shift+T · Shift+L | Hide or show images · texts · links |
| F | Pinned only |
| Ctrl+K | Command palette |
| Ctrl+O · Ctrl+Shift+C | Open a link · copy the text in an image |
| L | Open the Library |
| ? · F1 | Show every key on a note |
| Esc | Close the preview, then the shelf |

## Privacy

- Everything stays on your computer: no account, no cloud, no telemetry, no analytics and no
  crash reports.
- Panopin goes online only to keep itself up to date, and for **link previews** if you turn them
  on in Settings. The update check reads one small file from GitHub at start and every 6 hours and
  downloads a new version when there is one. It sends nothing about you, your computer or your
  copies; GitHub sees the request like any download. Both are made by the app's Rust core, never by
  its web view. Link previews never visit local or private network addresses, plain `http` links or
  links that carry passwords or tokens, and they load no scripts.
- The only numbers the developer sees are the download counts GitHub keeps for every release file.
  Panopin has no ID, no usage counting and no other reporting.
- Copied content is never written to log files.
- Your data is not encrypted. **Panopin is not a password vault.**
- Data lives in `%APPDATA%\Panopin` (database and media). The uninstaller asks whether to remove
  it.
- Sounds stay quiet during presentations, full-screen games and Focus sessions. A Do Not Disturb
  that you turn on by hand may go unnoticed, because Windows offers no public way to read it.

## Resource use

Measured with the v1.0.0 release build on a Windows 11 25H2 laptop (AMD Ryzen 9 7940HS,
16 GB), one minute after start, with the shelf closed. Memory is the private working set,
as Task Manager shows it.

| | |
| --- | --- |
| Installer | 3.3 MB |
| CPU while idle | 0.2 s of CPU time per minute, about 0.02 % of the processor |
| Memory while idle | about 120 MB: 10 MB for Panopin and 110 MB for its six WebView2 processes |

## Build from source

You need Windows 10 or 11, [Rust](https://rustup.rs) stable with the MSVC toolchain, Node.js 22.12
or later, and the Visual Studio Build Tools with the C++ workload.

```powershell
npm install
npm run tauri dev      # run in development
npm run tauri build    # build the installer
```

Checks run in CI on every push: `npm run typecheck`, `npm run lint`, `npm test`,
`npm run check:i18n`, `npm run check:licenses`, `npm run check:third-party`, and in `src-tauri/`
`cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and
`cargo deny check`.

If your checkout path is long, set `CARGO_TARGET_DIR` to a short folder first, for example
`%LOCALAPPDATA%\panopin-build`.

## Contributing

Issues and pull requests are welcome; for larger changes, please open an issue first.
[CONTRIBUTING.md](CONTRIBUTING.md) explains how. Translations especially: every language except
English was translated by machine and is waiting for native speakers to review it.

## Licence

[MIT](LICENSE) © 2026 Mehmet Burak Taşkaya. Panopin is built on open-source packages listed with
their licences in [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt).

---

<p align="center">Made by Mehmet Burak Taşkaya · <a href="https://mehmetburaktaskaya.com">mehmetburaktaskaya.com</a></p>
