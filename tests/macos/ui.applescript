-- UI helpers for the macOS live test (tests/macos/live.sh), all through System Events.
--   front                        bundle identifier of the app in front
--   key <code> [modifiers]       presses a key; modifiers: command, option command,
--                                control command, shift command
--   type <text>                  types the text
--   focus                        role and subrole of Panopin's focused element
--   windows                      names of Panopin's windows, one per line
--   texts <window>               texts and control names in that Panopin window, web view included
--   click <window> <name>        presses the button, tab or checkbox with that name in the window
--   close <window>               presses the window's close button
--   menu <item start>            opens the menu bar icon's menu and chooses the item
--   dock                         names of the Dock's items
--   statusitems                  how many menu bar icons Panopin has
--   textedit                     the text of TextEdit's front document

property appId : "com.mehmetburaktaskaya.panopin"
property controlRoles : {"AXButton", "AXCheckBox", "AXRadioButton", "AXTab", "AXLink", "AXPopUpButton", "AXMenuButton"}

on run argv
	set action to item 1 of argv
	with timeout of 300 seconds
		if action is "front" then
			tell application "System Events" to return bundle identifier of first process whose frontmost is true
		else if action is "key" then
			if (count of argv) > 2 then
				pressKey(item 2 of argv as integer, item 3 of argv)
			else
				pressKey(item 2 of argv as integer, "")
			end if
			return ""
		else if action is "type" then
			tell application "System Events" to keystroke (item 2 of argv)
			return ""
		else if action is "focus" then
			tell application "System Events"
				tell (first process whose bundle identifier is appId)
					set f to value of attribute "AXFocusedUIElement"
					set s to subrole of f
					if s is missing value then set s to ""
					return (role of f) & " " & s
				end tell
			end tell
		else if action is "windows" then
			tell application "System Events" to tell (first process whose bundle identifier is appId) to set names to name of every window
			return joined(names)
		else if action is "texts" then
			return windowTexts(item 2 of argv)
		else if action is "click" then
			return pressNamed(item 2 of argv, item 3 of argv)
		else if action is "close" then
			set w to findWindow(item 2 of argv)
			tell application "System Events" to click (first button of w whose subrole is "AXCloseButton")
			return ""
		else if action is "menu" then
			return chooseMenu(item 2 of argv)
		else if action is "dock" then
			tell application "System Events" to tell process "Dock" to set names to name of every UI element of list 1
			return joined(names)
		else if action is "statusitems" then
			tell application "System Events" to tell (first process whose bundle identifier is appId) to return count of menu bar items of menu bar 2
		else if action is "textedit" then
			tell application "System Events" to tell process "TextEdit" to return value of text area 1 of scroll area 1 of front window
		end if
	end timeout
	error "unknown action " & action
end run

on joined(items_)
	set AppleScript's text item delimiters to linefeed
	return items_ as text
end joined

on pressKey(code, mods)
	tell application "System Events"
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
	end tell
end pressKey

-- The Panopin window with exactly that name, else the first whose name contains it.
on findWindow(wanted)
	tell application "System Events"
		tell (first process whose bundle identifier is appId)
			repeat with w in every window
				if name of w is wanted then return w
			end repeat
			repeat with w in every window
				if name of w contains wanted then return w
			end repeat
		end tell
	end tell
	error "no window named " & wanted
end findWindow

on labelOf(e)
	tell application "System Events"
		set n to name of e
		if n is missing value or n is "" then set n to description of e
	end tell
	return n
end labelOf

on windowTexts(wanted)
	set w to findWindow(wanted)
	set out to {}
	with timeout of 300 seconds
		tell application "System Events"
			repeat with e in (entire contents of w)
				try
					set r to role of e
					if r is "AXStaticText" then
						set end of out to (value of e as text)
					else if controlRoles contains r then
						set end of out to "[" & (my labelOf(e) as text) & "]"
					end if
				end try
			end repeat
		end tell
	end timeout
	return joined(out)
end windowTexts

on pressNamed(wanted, label)
	set w to findWindow(wanted)
	with timeout of 300 seconds
		tell application "System Events"
			repeat with e in (entire contents of w)
				try
					if controlRoles contains (role of e) then
						if (my labelOf(e) as text) is label then
							perform action "AXPress" of e
							return "pressed"
						end if
					end if
				end try
			end repeat
		end tell
	end timeout
	error "no control named " & label
end pressNamed

on chooseMenu(start)
	tell application "System Events"
		tell (first process whose bundle identifier is appId)
			click menu bar item 1 of menu bar 2
			delay 0.6
			repeat with m in every menu item of menu 1 of menu bar item 1 of menu bar 2
				if name of m starts with start then
					click m
					return "chosen"
				end if
			end repeat
		end tell
		key code 53
	end tell
	error "no menu item " & start
end chooseMenu
