# Backlog

Ideas and out-of-scope items. Nothing here is planned for v1.

## Out of scope for v1

- Cloud sync and accounts
- Telemetry
- AI / LLM features
- Semantic (CLIP-like) image search; check the model licence before adding
- Video trimming
- Rich text (HTML/RTF) storage
- Capturing Explorer file copies (`CF_HDROP`)
- Microsoft Store distribution
- Phone sync

## Later

- Windows ARM64 build
- winget and Scoop packages

## Not on macOS yet (D-073)

- Screen recordings (macOS saves them as `.mov`)
- Dragging cards out of the shelf and the Library
- The export and import dialogs
- Opening the shelf from the screen edge
- Window materials and the system accent colour
- Quiet sounds while a full-screen app is in front
- A signed and notarised `.dmg` (needs an Apple Developer account)

## Ideas from Phase 0

- Portable mode that keeps data next to the executable (see D-029).
- Detect a manually enabled Do Not Disturb if Windows ever offers a public API for it (see D-021).

## Ideas from Phase 3

- A thumbnail under the pointer while dragging a card (the drag now shows the system cursor only).
- Neighbouring cards sliding over when a card is taken down, instead of closing the gap at once.
- Code-looking text in a monospace font on text cards.

## Ideas from Phase 4

- Zoom while cropping, for pixel-exact frames on large screenshots.
- Arrow keys nudging the selected mark by one pixel.
- More text styles in the markup text tool (size steps beyond three, a background box).
- Flattening only the cropped part of very large pictures, to speed up copies.

## Ideas from Phase 5

- Folder counts that include their subfolders.
- Remembering which folders are folded in the sidebar.
- Ordering folders by hand.

## Ideas from Phase 6

- Trimming screen recordings (out of scope for v1).
- A small "text" tag on image cards found by their OCR text, as in the concept.
- Marking matched words in the Library's tiles and list.
- Choosing the OCR language per image.
- The command palette inside the markup, crop and text editors.

## Ideas from Phase 7

- winget and Scoop manifests (planned after v1.0.0).
- The SignPath Foundation application once v1.0.0 is published (docs/SIGNING.md).
- A portable build that can replace itself in its own folder.
- An ARM64 build.
- A short demo GIF for the README and the website (the placeholders are in place).
