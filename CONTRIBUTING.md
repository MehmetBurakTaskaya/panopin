# Contributing to Panopin

Issues and pull requests are welcome. For larger changes, please open an issue first, so we can
agree on the approach before you spend time on it. The design is described in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), and the reasons behind it in
[docs/DECISIONS.md](docs/DECISIONS.md).

## Code

- Building and the checks CI runs are described in the README under
  [Build from source](README.md#build-from-source). Keep them green.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`,
  `fix:`, `docs:`, …).
- Every text the user sees lives in the files under `locales/`, never in code. A new text goes into
  `locales/en.json` and into every other locale file; until it is translated, the English text may
  stand in a translation, which `npm run check:i18n` accepts.
- Only dependencies under the licences listed in `src-tauri/deny.toml` can be used.

## Translations

Panopin speaks English, German, Spanish, French, Brazilian Portuguese, Turkish, Russian, Japanese,
Korean and Simplified Chinese. English is the source; all other languages were translated by
machine and are waiting for native speakers to review them. If you speak one of them, your
corrections are the most useful contribution there is: a single fixed word helps.

### The files

Each language is one file in `locales/`, named by its language tag: `de.json`, `pt-BR.json`,
`zh-CN.json` and so on. A file is a flat list of keys and texts in the order of `en.json`. The two
entries at the top describe the file itself:

```json
{
  "@language": "Deutsch",
  "@status": "needs review",
  "app.name": "Panopin",
  …
}
```

`"@status"` is `"needs review"` until a native speaker has gone through the whole file, and
`"reviewed"` after that. While it says `"needs review"`, Settings → General shows a short note
under the language choice.

The same files are used by the windows and by the menu bar or tray menu, so a change shows up
everywhere.

### Reviewing a language

1. Choose the language in Panopin: Settings → General → Language, or the language list in the
   first-run window. Walk through the shelf, its card menu, the preview, the Library, Settings and
   the first-run window, and note what reads wrong, sounds unnatural, or is cut off.
2. Fix the texts in the language's file. You can do this in GitHub's web editor: open the file,
   press the pencil icon, and GitHub will guide you to a pull request.
3. If you went through the whole file, change `"@status"` to `"reviewed"`. If you only fixed some
   texts, leave it as it is; partial fixes are just as welcome.
4. In the pull request, say which language you reviewed and, if you like, how you want to be
   credited.

### Rules for texts

- **Keep the placeholders.** Words in braces, such as `{count}`, `{name}` or `{version}`, are filled
  in by the app. Keep each of them exactly; you may move them within the sentence.
- **Key names** come from the `key.*` entries at the end of the file: the app writes shortcuts on
  Windows with them (Ctrl, Shift, Enter, Space, Esc, Backspace, Delete). Use the names that Windows
  and keyboards use in your language (German: Strg, Umschalt, Entf), and write keys the same way in
  your texts, so a hint such as "Strg+Eingabe" matches the menus. Alt, Win and Tab stay as they are,
  and so do the Mac symbols ⌘ ⌥ ⇧ ⌃ ⌫. Single-letter shortcuts (P for pin, S for take down …) are
  keys on the keyboard and stay the same letter in every language.
- **Keep names**: Panopin, Windows, macOS, Microsoft Edge WebView2, GitHub, SmartScreen, Snipping
  Tool, Vision. For system settings ("System Settings → Privacy & Security → Accessibility"), use
  the names the system shows in your language.
- **`.mac` keys** are the macOS version of the key without `.mac`. Use Mac words there: ⌘ instead
  of Ctrl, "this Mac" instead of "this PC", and the names of macOS settings.
- **Keep it short.** Many texts are labels on buttons, menus and cards with little room. Aim for
  no more than about a third longer than the English, and check the result in the app.
- **Use the same word for the same thing** throughout a file: shelf, card, pin, tape, take down,
  Library and Trash in particular.
- **Numbers**: there is one text for "1 item" and one for "{count} items". If your language has more
  forms, phrase the `{count}` text so it is right for any number (for example "Items: {count}").
- Follow your language's punctuation and quotation marks, and keep the "→" in settings paths.

### Checking

```powershell
npm run check:i18n
```

This makes sure every file has the same keys and placeholders as English and a valid `"@status"`.
To look at your texts in the app, run `npm run tauri dev` (see [Build from source](README.md#build-from-source)),
choose the language and check the shelf, the Library, Settings and the first-run window for text
that is cut off or runs over. Screenshots in the pull request help.

### Adding a language

1. Copy `locales/en.json` to `locales/<tag>.json`, set `"@language"` to the language's own name and
   `"@status"` to `"needs review"`, and translate the texts.
2. Add the language to `LANGUAGES` in `src/i18n/index.ts`, and to the Rust side: the `Language`
   setting in `src-tauri/src/settings.rs` and `Lang` in `src-tauri/src/i18n.rs` (its file and the
   language tags that choose it).
3. Run `npm run check:i18n`, `npm test` and, in `src-tauri/`, `cargo test`.
