// Accessibility helpers for the macOS live test (tests/macos/live.sh). System Events cannot see
// into Panopin's web views; the Accessibility API can.
//   texts <pid> <window>          texts and control names in that window, one per line
//   press <pid> <window> <name>   presses the control with that name in the window
//   focus <pid>                   focused window title | focused element role | subrole
//   menu <pid> <item start>       opens the menu bar icon's menu and chooses the item
//   icons <pid>                   how many menu bar icons the app has
//   shown <pid>                   titles of the app's windows on screen
import AppKit
import ApplicationServices

let args = Array(CommandLine.arguments.dropFirst())
let controlRoles: Set<String> = [
    "AXButton", "AXCheckBox", "AXRadioButton", "AXTab", "AXLink", "AXPopUpButton", "AXMenuButton",
]

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

func value(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var result: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, name as CFString, &result) == .success else { return nil }
    return result
}

func text(_ element: AXUIElement, _ name: String) -> String { value(element, name) as? String ?? "" }

func element(_ element: AXUIElement, _ name: String) -> AXUIElement? {
    guard let result = value(element, name), CFGetTypeID(result) == AXUIElementGetTypeID() else { return nil }
    return (result as! AXUIElement)
}

func children(_ element: AXUIElement, _ name: String = kAXChildrenAttribute) -> [AXUIElement] {
    value(element, name) as? [AXUIElement] ?? []
}

func label(_ element: AXUIElement) -> String {
    for name in [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute] {
        let found = text(element, name)
        if !found.isEmpty { return found }
    }
    return ""
}

// Depth first; stops at the first element for which visit returns true.
@discardableResult
func walk(_ element: AXUIElement, depth: Int = 0, _ visit: (AXUIElement, String) -> Bool) -> Bool {
    if depth > 80 { return false }
    if visit(element, text(element, kAXRoleAttribute)) { return true }
    for child in children(element) where walk(child, depth: depth + 1, visit) { return true }
    return false
}

func application(_ pid: String) -> AXUIElement {
    guard let number = pid_t(pid) else { fail("bad pid \(pid)") }
    return AXUIElementCreateApplication(number)
}

func window(_ app: AXUIElement, _ wanted: String) -> AXUIElement {
    let windows = children(app, kAXWindowsAttribute)
    if let exact = windows.first(where: { text($0, kAXTitleAttribute) == wanted }) { return exact }
    if let near = windows.first(where: { text($0, kAXTitleAttribute).contains(wanted) }) { return near }
    fail("no window named \(wanted)")
}

func press(_ element: AXUIElement) { AXUIElementPerformAction(element, kAXPressAction as CFString) }

func postKey(_ code: CGKeyCode) {
    CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true)?.post(tap: .cghidEventTap)
    CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false)?.post(tap: .cghidEventTap)
}

switch args.first {
case "texts" where args.count == 3:
    walk(window(application(args[1]), args[2])) { element, role in
        if role == "AXStaticText" {
            let found = text(element, kAXValueAttribute)
            if !found.isEmpty { print(found) }
        } else if role == "AXPopUpButton" {
            print("[\(label(element))] \(text(element, kAXValueAttribute))")
        } else if controlRoles.contains(role) {
            print("[\(label(element))]")
        }
        return false
    }
case "press" where args.count == 4:
    let found = walk(window(application(args[1]), args[2])) { element, role in
        guard controlRoles.contains(role), label(element) == args[3] else { return false }
        press(element)
        return true
    }
    if !found { fail("no control named \(args[3])") }
case "focus" where args.count == 2:
    let app = application(args[1])
    let title = element(app, kAXFocusedWindowAttribute).map { text($0, kAXTitleAttribute) } ?? "-"
    let focused = element(app, kAXFocusedUIElementAttribute)
    let role = focused.map { text($0, kAXRoleAttribute) } ?? "-"
    let subrole = focused.map { text($0, kAXSubroleAttribute) } ?? "-"
    print("\(title)|\(role)|\(subrole)")
case "menu" where args.count == 3:
    let app = application(args[1])
    guard let extras = element(app, "AXExtrasMenuBar"), let item = children(extras).first else {
        fail("no menu bar icon")
    }
    var origin = CGPoint.zero
    var size = CGSize.zero
    if let position = value(item, kAXPositionAttribute) { AXValueGetValue(position as! AXValue, .cgPoint, &origin) }
    if let extent = value(item, kAXSizeAttribute) { AXValueGetValue(extent as! AXValue, .cgSize, &size) }
    // A left click opens the shelf; the menu comes with a right click.
    let point = CGPoint(x: origin.x + size.width / 2, y: origin.y + size.height / 2)
    for type in [CGEventType.rightMouseDown, .rightMouseUp] {
        CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .right)?
            .post(tap: .cghidEventTap)
        usleep(80_000)
    }
    usleep(700_000)
    let choose: (AXUIElement, String) -> Bool = { element, role in
        guard role == "AXMenuItem", label(element).hasPrefix(args[2]) else { return false }
        press(element)
        return true
    }
    if !walk(item, choose) && !walk(app, choose) {
        postKey(53)
        fail("no menu item \(args[2])")
    }
case "icons" where args.count == 2:
    print(element(application(args[1]), "AXExtrasMenuBar").map { children($0).count } ?? 0)
case "shown" where args.count == 2:
    guard let pid = pid_t(args[1]) else { fail("bad pid \(args[1])") }
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
    for info in list where (info[kCGWindowOwnerPID as String] as? pid_t) == pid {
        print(info[kCGWindowName as String] as? String ?? "?")
    }
default:
    fail("usage: axtool texts|press|focus|menu|shown ...")
}
