[Setup]
AppName=T Books
AppVersion=1.0
DefaultDirName={pf}\TBooks
DefaultGroupName=T Books
OutputDir=installer\output
OutputBaseFilename=T-Books-Setup
Compression=lzma
SolidCompression=yes

[Files]
; Try common tauri bundle locations and fallback to dist\*
Source: "src-tauri\target\release\bundle\windows\*"; DestDir: "{app}"; Flags: recursesubdirs
Source: "src-tauri\target\release\bundle\installer\*"; DestDir: "{app}"; Flags: recursesubdirs
Source: "dist\*"; DestDir: "{app}"; Flags: recursesubdirs

[Icons]
Name: "{group}\T Books"; Filename: "{app}\TBooks.exe"
Name: "{commondesktop}\T Books"; Filename: "{app}\TBooks.exe"

[Run]
Filename: "{app}\TBooks.exe"; Description: "Launch T Books"; Flags: nowait postinstall skipifsilent
