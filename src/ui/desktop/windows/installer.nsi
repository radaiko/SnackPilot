; SnackPilot per-user installer (08-gpui-desktop "Shipping matrix").
;
; Built by tools/devops/package-windows.sh, which passes VERSION, BUILD, SOURCE
; (the directory holding snackpilot-desktop.exe), OUT and ICON. Per-user and
; admin-free: everything lands under %LOCALAPPDATA%, so no elevation prompt and
; no machine-wide state. The app's own data lives outside the install directory
; (%LOCALAPPDATA%\dev.radaiko.gourmetclient), so an upgrade or uninstall never
; touches a signed-in user's cache or preferences.
;
; This installer is UNSIGNED: SmartScreen warns until it has reputation. The
; release notes say so; nothing here can change that.

Unicode true

!include "MUI2.nsh"
!include "FileFunc.nsh"

!ifndef VERSION
  !error "VERSION is required (-DVERSION=x.y.z)"
!endif
!ifndef BUILD
  !define BUILD "0"
!endif
!ifndef SOURCE
  !error "SOURCE is required (-DSOURCE=<dir with snackpilot-desktop.exe>)"
!endif
!ifndef OUT
  !error "OUT is required (-DOUT=<setup.exe path>)"
!endif
!ifndef ICON
  !define ICON "SnackPilot.ico"
!endif

!define APP_NAME "SnackPilot"
!define APP_EXE "snackpilot-desktop.exe"
!define APP_ID "dev.radaiko.gourmetclient"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\SnackPilot"

Name "${APP_NAME} ${VERSION}"
BrandingText "${APP_NAME} ${VERSION} (unsigned build)"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\SnackPilot"
InstallDirRegKey HKCU "Software\SnackPilot" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ShowInstDetails show

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"
VIAddVersionKey "FileVersion" "${VERSION}.${BUILD}"
VIAddVersionKey "LegalCopyright" "Aiko Radlingmayr"

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "SnackPilot starten"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "English"

Section "SnackPilot" SEC_APP
  SetOutPath "$INSTDIR"
  SetOverwrite on
  File "${SOURCE}\${APP_EXE}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0

  WriteRegStr HKCU "Software\SnackPilot" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "Aiko Radlingmayr"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

Section "Uninstall"
  ; Only what the installer put there. %LOCALAPPDATA%\${APP_ID} is the app's own
  ; data directory and stays, exactly like an app dragged to the Trash.
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "Software\SnackPilot"
SectionEnd
