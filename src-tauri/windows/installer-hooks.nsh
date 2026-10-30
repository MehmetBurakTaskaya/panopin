; Additions to Tauri's NSIS uninstaller (D-006, D-066). Nothing here runs during an update.

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ; "Start with Windows" writes this entry; it must not outlive the app.
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Panopin"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Panopin"
  ${EndIf}
  ; Panopin keeps the user's items in %APPDATA%\Panopin, outside the identifier folders Tauri
  ; removes. They go only when the user ticks "Delete the application data".
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RmDir /r "$APPDATA\Panopin"
  ${EndIf}
!macroend
