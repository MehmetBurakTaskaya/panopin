-- System Events helpers for the macOS live test (tests/macos/live.sh); tests/macos/ax.swift
-- reaches into Panopin's web views.
--   front                        bundle identifier of the app in front
--   key <code> [modifiers]       presses a key; modifiers: command, option command,
--                                control command, shift command
--   type <text>                  types the text
--   close <window>               presses the close button of that Panopin window
--   dock                         names of the Dock's items
--   statusitems                  how many menu bar icons Panopin has
--   textedit                     the text of TextEdit's front document

property appId : "com.mehmetburaktaskaya.panopin"

on run argv
	set action to item 1 of argv
	tell application "System Events"
		if action is "front" then
			return bundle identifier of first process whose frontmost is true
		else if action is "key" then
			set mods to ""
			if (count of argv) > 2 then set mods to item 3 of argv
			set code to (item 2 of argv) as integer
			if mods is "command" then
				key code code using {command down}
			else if mods is "option command" then
				key code code using {option down, command down}
			else if mods is "control command" then
				key code code using {control down, command down}
			else if mods is "shift command" then
				key code code using {shift down, command down}
			else
				key code code
			end if
			return ""
		else if action is "type" then
			keystroke (item 2 of argv)
			return ""
		else if action is "close" then
			tell (first process whose bundle identifier is appId)
				repeat with w in every window
					if name of w contains (item 2 of argv) then
						click (first button of w whose subrole is "AXCloseButton")
						return ""
					end if
				end repeat
			end tell
			error "no window named " & (item 2 of argv)
		else if action is "dock" then
			tell process "Dock" to set names to name of every UI element of list 1
			set AppleScript's text item delimiters to linefeed
			return names as text
		else if action is "statusitems" then
			tell (first process whose bundle identifier is appId) to return count of menu bar items of menu bar 2
		else if action is "textedit" then
			tell process "TextEdit" to return value of text area 1 of scroll area 1 of front window
		end if
	end tell
	error "unknown action " & action
end run
