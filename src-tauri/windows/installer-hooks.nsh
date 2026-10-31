; Additions to Tauri's NSIS installer and uninstaller (D-006, D-066, D-081).

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

; Antivirus software may keep panopin.exe open even while Panopin is closed; McAfee's framework
; host does. Before installing and before uninstalling, Tauri asks Restart Manager to close every
; process that holds the program, and Restart Manager does nothing at all when one of them is a
; protected service, so the installer stopped with "could not close Panopin" (D-081). Panopin's
; own processes are therefore closed first, through Restart Manager by process, and the old
; program is set aside under another name, so Tauri's check finds nobody holding it. The set-aside
; file goes when the work is done, or at Panopin's next start if the antivirus still holds it.
Var PanopinProgram
Var PanopinAside

!macro PanopinCloseAndSetAside
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  Push $R2
  Push $R3
  Push $R4
  Push $R5
  Push $R6
  Push $R7
  Push $R8
  Push $R9
  StrCpy $PanopinProgram "$INSTDIR\${MAINBINARYNAME}.exe"
  StrCpy $PanopinAside ""

  ; Who holds the program: applications are closed, services and critical processes are not.
  !insertmacro RestartManager_StartSession $R9
  ${If} $R9 != ""
    !insertmacro RestartManager_RegisterFile $R9 "$PanopinProgram"
    System::Call 'RSTRTMGR::RmGetList(i R9, *i .r1, *i .r2, p 0, *i .r3) i .r0'
    ${If} $0 = 234 ; ERROR_MORE_DATA
      ; RM_PROCESS_INFO is 668 bytes; its first 12 are the RM_UNIQUE_PROCESS, its type is at 652.
      IntOp $R3 $1 * 668
      System::Alloc $R3
      Pop $R7
      StrCpy $2 $1
      System::Call 'RSTRTMGR::RmGetList(i R9, *i .r1, *i r2r2, p R7, *i .r3) i .r0'
      ${If} $0 = 0
        IntOp $R3 $2 * 12
        System::Alloc $R3
        Pop $R6
        StrCpy $R5 0
        StrCpy $R4 0
        ${While} $R4 < $2
          IntOp $R3 $R4 * 668
          IntPtrOp $R3 $R7 + $R3
          IntPtrOp $R2 $R3 + 652
          System::Call '*$R2(i .r4)'
          ${If} $4 = ${RmMainWindow}
          ${OrIf} $4 = ${RmOtherWindow}
          ${OrIf} $4 = ${RmConsole}
            System::Call '*$R3(i .r5, i .r6, i .r7)'
            IntOp $R2 $R5 * 12
            IntPtrOp $R2 $R6 + $R2
            System::Call '*$R2(i r5, i r6, i r7)'
            IntOp $R5 $R5 + 1
          ${EndIf}
          IntOp $R4 $R4 + 1
        ${EndWhile}
        ${If} $R5 > 0
          ; A session of their own: asked by process, Restart Manager closes them as Windows
          ; closes apps at sign-out, and Panopin ends at once.
          !insertmacro RestartManager_StartSession $R8
          ${If} $R8 != ""
            System::Call 'RSTRTMGR::RmRegisterResources(i R8, i 0, p 0, i R5, p R6, i 0, p 0) i .r0'
            ${If} $0 = 0
              System::Call 'RSTRTMGR::RmShutdown(i R8, i ${RmForceShutdown}, p 0) i .r0'
            ${EndIf}
            !insertmacro RestartManager_EndSession $R8
          ${EndIf}
        ${EndIf}
        System::Free $R6
      ${EndIf}
      System::Free $R7
    ${EndIf}
    !insertmacro RestartManager_EndSession $R9
  ${EndIf}

  ; A Panopin that is still running stops here.
  nsis_tauri_utils::FindProcess "${MAINBINARYNAME}.exe"
  Pop $0
  ${If} $0 = 0
    nsis_tauri_utils::KillProcess "${MAINBINARYNAME}.exe"
    Pop $0
    Sleep 500
  ${EndIf}

  ; Windows lets a program that someone holds be renamed, though not replaced or deleted.
  ${If} ${FileExists} "$PanopinProgram"
    System::Call 'kernel32::GetTickCount() i .r0'
    StrCpy $1 "$INSTDIR\${MAINBINARYNAME}.$0.old"
    ClearErrors
    Rename "$PanopinProgram" "$1"
    ${IfNot} ${Errors}
      StrCpy $PanopinAside "$1"
    ${EndIf}
  ${EndIf}

  Pop $R9
  Pop $R8
  Pop $R7
  Pop $R6
  Pop $R5
  Pop $R4
  Pop $R3
  Pop $R2
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend

!macro PanopinDropAside
  ${If} $PanopinAside != ""
    ; Held by the antivirus, it goes once the antivirus lets go, or at Panopin's next start.
    Delete "$PanopinAside"
  ${EndIf}
!macroend

; A failed install or uninstall puts the old program back.
!macro PanopinRestoreAside
  ${If} $PanopinAside != ""
  ${AndIfNot} ${FileExists} "$PanopinProgram"
    Rename "$PanopinAside" "$PanopinProgram"
  ${EndIf}
!macroend

Function .onInstFailed
  !insertmacro PanopinRestoreAside
FunctionEnd

Function un.onUninstFailed
  !insertmacro PanopinRestoreAside
FunctionEnd

!macro NSIS_HOOK_PREINSTALL
  !insertmacro PanopinCloseAndSetAside
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro PanopinDropAside
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro PanopinCloseAndSetAside
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro PanopinDropAside
  ; Tauri removed the folder before the set-aside program was gone.
  RMDir "$INSTDIR"
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
