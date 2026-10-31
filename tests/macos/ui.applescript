-- System Events helpers for the macOS live test (tests/macos/live.sh); tests/macos/ax.swift
-- reaches into Panopin's web views.
--   front                        bundle identifier of the app in front
--   key <code> [modifiers]       presses a key; modifiers: command, option command,
--                                control command, shift command, shift
--   type <text>                  types the text
--   close <window>               presses the close button of that Panopin window
--   dock                         names of the Dock's items
--   textedit                     the text of TextEdit's front document
-- Words such as "action" mean something inside System Events, so the request is read first.

property appId : "com.mehmetburaktaskaya.panopin"

on run argv
	set todo to item 1 of argv
	if todo is "front" then
		tell application "System Events" to return bundle identifier of first process whose frontmost is true
	else if todo is "key" then
		set keyNumber to (item 2 of argv) as integer
		set mods to ""
		if (count of argv) > 2 then set mods to item 3 of argv
		pressKey(keyNumber, mods)
		return ""
	else if todo is "type" then
		set typed to item 2 of argv
		tell application "System Events" to keystroke typed
		return ""
	else if todo is "close" then
		closeWindow(item 2 of argv)
		return ""
	else if todo is "dock" then
		tell application "System Events" to tell process "Dock" to set names to name of every UI element of list 1
		set AppleScript's text item delimiters to linefeed
		return names as text
	else if todo is "textedit" then
		tell application "System Events" to tell process "TextEdit" to return value of text area 1 of scroll area 1 of front window
	end if
	error "unknown request " & todo
end run

on pressKey(keyNumber, mods)
	tell application "System Events"
		if mods is "command" then
			key code keyNumber using {command down}
		else if mods is "option command" then
			key code keyNumber using {option down, command down}
		else if mods is "control command" then
			key code keyNumber using {control down, command down}
		else if mods is "shift command" then
			key code keyNumber using {shift down, command down}
		else if mods is "shift" then
			key code keyNumber using {shift down}
		else
			key code keyNumber
		end if
	end tell
end pressKey

on closeWindow(wanted)
	tell application "System Events"
		tell (first process whose bundle identifier is appId)
			repeat with w in every window
				if name of w contains wanted then
					click (first button of w whose subrole is "AXCloseButton")
					return
				end if
			end repeat
		end tell
	end tell
	error "no window named " & wanted
end closeWindow
