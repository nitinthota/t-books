; T Books NSIS hooks.
; Uninstall removes the app. It must never delete %LOCALAPPDATA%\T-Books.
;
; Dependent software:
; - Visual C++ runtime DLLs are copied next to t-books.exe (bundleVCRuntime).
; - Tauri downloadBootstrapper fetches Evergreen WebView2 when it is missing.
;   This POSTINSTALL pass retries that download if the registry still has no pv.

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
  StrCpy $0 ""
  SetRegView 64
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" 0 tbooks_webview2_ok
  ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" 0 tbooks_webview2_ok
  DetailPrint "WebView2 missing. Downloading Microsoft Evergreen runtime..."
  nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -Command "$$ProgressPreference=''SilentlyContinue''; Invoke-WebRequest -UseBasicParsing -Uri ''https://go.microsoft.com/fwlink/p/?LinkId=2124703'' -OutFile (Join-Path $$env:TEMP ''TBooksWebView2Setup.exe'')"'
  nsExec::ExecToLog '"$TEMP\TBooksWebView2Setup.exe" /silent /install'
  tbooks_webview2_ok:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Intentionally empty: leave %LOCALAPPDATA%\T-Books (books, backups, logs).
!macroend
