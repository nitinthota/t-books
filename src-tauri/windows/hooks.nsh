; T Books NSIS hooks.
; Uninstall removes the app. It must never delete %LOCALAPPDATA%\T-Books.
;
; One file for the office: T-Books-Setup.exe. Wizard pages are inside that exe.
;
; Dependent software (never a second installer the user launches):
; - Visual C++ runtime DLLs are copied next to t-books.exe (bundleVCRuntime).
; - Tauri embedBootstrapper ships Microsoft’s WebView2 bootstrapper inside Setup.
;   Tauri runs that helper if WebView2 is missing (Evergreen may then download).
; - This POSTINSTALL pass is only a silent retry when registry pv is still empty.
;   It does not Abort Setup if the PC is offline. Do not tell users to run the
;   temp bootstrapper themselves.

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
  StrCpy $0 ""
  SetRegView 64
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" 0 tbooks_webview2_ok
  ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" 0 tbooks_webview2_ok
  DetailPrint "WebView2 still missing after Setup's embedded helper. Retrying Microsoft Evergreen download..."
  nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -Command "$$ProgressPreference=''SilentlyContinue''; Invoke-WebRequest -UseBasicParsing -Uri ''https://go.microsoft.com/fwlink/p/?LinkId=2124703'' -OutFile (Join-Path $$env:TEMP ''TBooksWebView2Setup.exe'')"'
  IfFileExists "$TEMP\TBooksWebView2Setup.exe" 0 tbooks_webview2_ok
  nsExec::ExecToLog '"$TEMP\TBooksWebView2Setup.exe" /silent /install'
  tbooks_webview2_ok:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Intentionally empty: leave %LOCALAPPDATA%\T-Books (books, backups, logs).
!macroend
