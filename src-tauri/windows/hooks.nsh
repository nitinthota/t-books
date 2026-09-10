; T Books NSIS hooks.
; Uninstall removes the app. It must never delete %LOCALAPPDATA%\T-Books.

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Intentionally empty: leave %LOCALAPPDATA%\T-Books (books, backups, logs).
!macroend
