!include "MUI2.nsh"

!ifndef APP_VERSION
    !define APP_VERSION "0.1.0"
!endif
!ifndef APP_ARCH
    !define APP_ARCH "x86_64"
!endif
!ifndef STAGE
    !error "STAGE must point at the packaging staging directory"
!endif
!ifndef OUT_FILE
    !error "OUT_FILE must point at the output installer"
!endif
!ifndef ICON_PATH
    !define ICON_PATH "..\icons\slime-soccer.ico"
!endif

Unicode true
Name "Super Slime Soccer"
OutFile "${OUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\Slime Soccer"
InstallDirRegKey HKCU "Software\SlimeSoccer" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 32
ShowInstDetails show
ShowUninstDetails show
BrandingText "Super Slime Soccer ${APP_VERSION} (${APP_ARCH})"

!define MUI_ABORTWARNING
!define MUI_ICON "${ICON_PATH}"
!define MUI_UNICON "${ICON_PATH}"
!define MUI_WELCOMEPAGE_TITLE "Super Slime Soccer"
!define MUI_WELCOMEPAGE_TEXT "This installs the fan remake of Super Slime Soccer for Windows (${APP_ARCH}).$\r$\n$\r$\nThe game includes original artwork that is not MIT licensed; see THIRD_PARTY_NOTICES.md after installation."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${STAGE}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\slime-soccer.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Super Slime Soccer"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Super Slime Soccer (required)" SEC_MAIN
    SectionIn RO
    SetOutPath "$INSTDIR"
    File "${STAGE}\slime-soccer.exe"
    File "${STAGE}\README.md"
    File "${STAGE}\LICENSE"
    File "${STAGE}\ASSETS.md"
    File "${STAGE}\THIRD_PARTY_NOTICES.md"

    WriteUninstaller "$INSTDIR\uninstall.exe"

    CreateShortcut "$DESKTOP\Super Slime Soccer.lnk" "$INSTDIR\slime-soccer.exe"
    CreateDirectory "$SMPROGRAMS\Super Slime Soccer"
    CreateShortcut "$SMPROGRAMS\Super Slime Soccer\Super Slime Soccer.lnk" "$INSTDIR\slime-soccer.exe"
    CreateShortcut "$SMPROGRAMS\Super Slime Soccer\Uninstall.lnk" "$INSTDIR\uninstall.exe"

    WriteRegStr HKCU "Software\SlimeSoccer" "InstallDir" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer" "DisplayName" "Super Slime Soccer"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer" "DisplayVersion" "${APP_VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer" "DisplayIcon" "$INSTDIR\slime-soccer.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer" "Publisher" "Super Slime Soccer contributors"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer" "NoRepair" 1
SectionEnd

Section "Uninstall"
    Delete "$DESKTOP\Super Slime Soccer.lnk"
    Delete "$SMPROGRAMS\Super Slime Soccer\Super Slime Soccer.lnk"
    Delete "$SMPROGRAMS\Super Slime Soccer\Uninstall.lnk"
    RMDir "$SMPROGRAMS\Super Slime Soccer"

    Delete "$INSTDIR\slime-soccer.exe"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\ASSETS.md"
    Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SlimeSoccer"
    DeleteRegKey HKCU "Software\SlimeSoccer"
SectionEnd
