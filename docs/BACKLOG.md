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

## Languages (D-074)

- Native speakers' reviews of the machine translations (CONTRIBUTING.md)
- Text in images read in the UI language too: Windows OCR reads Turkish or English today

## Not on macOS yet (D-073)

- Screen recordings (macOS saves them as `.mov`)
- Dragging cards out of the shelf and the Library
- Opening the shelf from the screen edge
- Window materials (vibrancy) behind the Library and Settings
- Following accent colour changes while a window is open
- Quiet sounds while a full-screen app is in front
- A signed and notarised `.dmg` (needs an Apple Developer account)

## Ideas from Phase 0

- Portable mode that keeps data next to the executable (see D-029).
- Detect a manually enabled Do Not Disturb if Windows ever offers a public API for it (see D-021).

## Ideas from Phase 3

- A thumbnail under the pointer while dragging a card (the drag now shows the system cursor only).
- Neighbouring cards sliding over when a card is taken down, instead of closing the gap at once.

## Ideas from Phase 4

- Zoom while cropping, for pixel-exact frames on large screenshots.
- Arrow keys nudging the selected mark by one pixel.
- More text styles in the markup text tool (size steps beyond three, a background box).
- Flattening only the cropped part of very large pictures, to speed up copies.

## Ideas from Phase 5

- Remembering which folders are folded in the sidebar.
- Ordering folders by hand.

## Ideas from Phase 6

- Trimming screen recordings (out of scope for v1).
- A small "text" tag on image cards found by their OCR text, as in the concept.
- Choosing the OCR language per image.
- The command palette inside the markup, crop and text editors.

## Ideas from Phase 7

- winget and Scoop manifests (planned after v1.0.0).
- The SignPath Foundation application once v1.0.0 is published (docs/SIGNING.md).
- An ARM64 build.

## Left from the 1.6.0 review (D-085)

- Showing and hiding the overlay costs about 0.8 s of CPU per opening in WebView2's processes,
  whatever the board shows: the page is redrawn and the full-screen transparent surface rebuilt.
  Keeping the window shown off screen, or sizing it to the board, needs its own design.
- Copying a large picture prepares its bitmap before the clipboard gets anything; delayed rendering
  (as the paste queue does) would hand it out at once. Shift+Enter prepares every item up front.
- Recordings in watched folders hold the capture worker while they settle; folder events could get
  their own thread.
- The preview decodes the full original; decoding at the panel's size would be quicker.
- Timed pauses and the update checks count time with `thread::sleep`, which stops while the PC
  sleeps.
- A retried update downloads the installer again.
- The screen-edge dwell opens the shelf from the mouse hook's thread.
- A text's character count and preview could be stored when it is written, so huge texts are not
  read on every list.
