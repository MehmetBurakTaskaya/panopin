// Pasteboard and picture helpers for the macOS live test (tests/macos/live.sh).
//   render <text> <out.png>   draws the text in black on white and saves it as a PNG
//   png <file>                puts the PNG on the pasteboard
//   tiff <file>               puts the picture on the pasteboard as TIFF only
//   marked <type> <text>      puts the text on the pasteboard together with a marker type
//   accent-changed            tells running apps the accent colour changed, as System Settings does
import AppKit

let args = Array(CommandLine.arguments.dropFirst())
let board = NSPasteboard.general

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

switch args.first {
case "render" where args.count == 3:
    let size = NSSize(width: 720, height: 220)
    guard let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: Int(size.width), pixelsHigh: Int(size.height),
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)
    else { fail("no bitmap") }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    NSColor.white.setFill()
    NSRect(origin: .zero, size: size).fill()
    let attributes: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: 56, weight: .semibold),
        .foregroundColor: NSColor.black,
    ]
    (args[1] as NSString).draw(at: NSPoint(x: 32, y: 80), withAttributes: attributes)
    NSGraphicsContext.restoreGraphicsState()
    guard let data = rep.representation(using: .png, properties: [:]) else { fail("no PNG") }
    do { try data.write(to: URL(fileURLWithPath: args[2])) } catch { fail("\(error)") }
case "png" where args.count == 2:
    guard let data = FileManager.default.contents(atPath: args[1]) else { fail("cannot read \(args[1])") }
    board.clearContents()
    board.setData(data, forType: .png)
case "tiff" where args.count == 2:
    guard let image = NSImage(contentsOfFile: args[1]), let tiff = image.tiffRepresentation
    else { fail("cannot read \(args[1])") }
    board.clearContents()
    board.setData(tiff, forType: .tiff)
case "marked" where args.count == 3:
    let marker = NSPasteboard.PasteboardType(args[1])
    board.clearContents()
    board.declareTypes([.string, marker], owner: nil)
    board.setString(args[2], forType: .string)
    board.setData(Data(), forType: marker)
case "accent-changed" where args.count == 1:
    DistributedNotificationCenter.default().postNotificationName(
        NSNotification.Name("AppleColorPreferencesChangedNotification"), object: nil, userInfo: nil,
        deliverImmediately: true)
default:
    fail("usage: pbtool render <text> <out.png> | png <file> | tiff <file> | marked <type> <text> | accent-changed")
}
