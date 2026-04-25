; Ditox Clipboard Manager - Inno Setup Installer Script
; Download Inno Setup from: https://jrsoftware.org/isdl.php

#define MyAppName "Ditox Clipboard Manager"
#ifndef MyAppVersion
  #define MyAppVersion "0.3.1"
#endif
#define MyAppPublisher "0xfell"
#define MyAppURL "https://github.com/0xfell/ditox"
#define MyAppExeName "ditox-gui.exe"

[Setup]
; Unique application ID - DO NOT CHANGE after first release
AppId={{A3E8F2B1-5C4D-4E6F-8A9B-1C2D3E4F5A6B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Ditox
DefaultGroupName={#MyAppName}
; No admin required - installs to user's AppData
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
; Output settings
OutputDir=..\..\target\installer
OutputBaseFilename=ditox-setup-{#MyAppVersion}
; Compression
Compression=lzma2/ultra64
SolidCompression=yes
; Visual
SetupIconFile=ditox.ico
UninstallDisplayIcon={app}\ditox.ico
WizardStyle=modern
WizardSizePercent=100
; Windows version
MinVersion=10.0
; UPGRADE SUPPORT - key settings
UsePreviousAppDir=yes
UsePreviousGroup=yes
UsePreviousTasks=yes
; Uninstall info
UninstallDisplayName={#MyAppName}
; Close running app before upgrade
CloseApplications=yes
CloseApplicationsFilter=*.exe
RestartApplications=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startupicon"; Description: "Run Ditox when Windows starts"; GroupDescription: "Startup:"

[Files]
; Main executable
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; Icon file
Source: "ditox.ico"; DestDir: "{app}"; Flags: ignoreversion
; License
Source: "License.rtf"; DestDir: "{app}"; DestName: "License.rtf"; Flags: ignoreversion

[Icons]
; Start Menu
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\ditox.ico"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
; Desktop (optional)
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\ditox.ico"; Tasks: desktopicon

[Registry]
; Run on startup (optional)
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Ditox"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: startupicon

[Run]
; Option to launch after install
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
// Check if app is running
function IsAppRunning(): Boolean;
var
  ResultCode: Integer;
begin
  // Use tasklist to check if process exists
  Exec('cmd', '/c tasklist /FI "IMAGENAME eq ditox-gui.exe" | find /i "ditox-gui.exe" >nul', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := (ResultCode = 0);
end;

// Close Ditox if running before install/upgrade
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;

  if IsAppRunning() then
  begin
    // Kill the process silently for upgrades
    Exec('taskkill', '/f /im ditox-gui.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Sleep(1000); // Wait for process to fully close
  end;
end;

// Close app before uninstall
function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  Exec('taskkill', '/f /im ditox-gui.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Sleep(500);
end;

// Show "upgrade" message if already installed
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  NeedsRestart := False;
end;
