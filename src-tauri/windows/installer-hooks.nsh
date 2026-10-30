; Additions to Tauri's NSIS installer and uninstaller (D-006, D-066). Nothing here runs during an
; update.

; Opening the installer installs at once: it starts itself again in Tauri's passive mode (/P),
; which shows only the progress bar, and Panopin opens when it is done (/R). Silent installs never
; reach this point, and the updater already passes /P.
!define MUI_CUSTOMFUNCTION_GUIINIT PanopinOneClick
Function PanopinOneClick
  ClearErrors
  ${GetOptions} $CMDLINE "/P" $R0
  ${If} ${Errors}
    ${GetParameters} $R0
    Exec '"$EXEPATH" /P /R $R0'
    Quit
  ${EndIf}
FunctionEnd

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
