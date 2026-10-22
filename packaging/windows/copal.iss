; Inno Setup script for the Copal installer (Copal-Setup-<version>.exe).
; Built by .github/workflows/release.yml:
;   ISCC.exe /DAppVersion=0.0.2 /DBinDir=<folder with the .exe files> /DOutDir=<output> copal.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef BinDir
  #define BinDir "..\..\target\release"
#endif
#ifndef OutDir
  #define OutDir "..\..\dist"
#endif

[Setup]
AppId={{6F2C3A1E-5B7D-4C8E-9A0F-2D1B3C4E5F60}
AppName=Copal
AppVersion={#AppVersion}
AppVerName=Copal {#AppVersion}
AppPublisher=hakiguru
AppPublisherURL=https://github.com/hakiguru/Copal
AppSupportURL=https://github.com/hakiguru/Copal/issues
AppUpdatesURL=https://github.com/hakiguru/Copal/releases
DefaultDirName={autopf}\Copal
DefaultGroupName=Copal
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\copal.exe
UninstallDisplayName=Copal
OutputDir={#OutDir}
OutputBaseFilename=Copal-Setup-{#AppVersion}
SetupIconFile=..\..\assets\copal.ico
LicenseFile=..\..\LICENSE
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Install for everyone (asks for admin) or just for me (no admin): the user chooses.
PrivilegesRequiredOverridesAllowed=dialog
ChangesEnvironment=yes
ChangesAssociations=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[CustomMessages]
english.AddToPath=Add copal-cli to PATH (use Copal from any terminal)
russian.AddToPath=Добавить copal-cli в PATH (Copal из любого терминала)
english.Associate=Open .copal project files with Copal
russian.Associate=Открывать проекты .copal в Copal

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "associate"; Description: "{cm:Associate}"
Name: "addtopath"; Description: "{cm:AddToPath}"; Flags: unchecked

[Files]
Source: "{#BinDir}\copal.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\copal-lite.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\copal-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"
Source: "..\..\README.md"; DestDir: "{app}"

[Icons]
Name: "{autoprograms}\Copal"; Filename: "{app}\copal.exe"
Name: "{autoprograms}\Copal Lite"; Filename: "{app}\copal-lite.exe"
Name: "{autodesktop}\Copal"; Filename: "{app}\copal.exe"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\.copal"; ValueType: string; ValueName: ""; ValueData: "Copal.Project"; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Copal.Project"; ValueType: string; ValueName: ""; ValueData: "Copal project"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Copal.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\copal.exe,0"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Copal.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\copal.exe"" ""%1"""; Tasks: associate
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Check: NeedsAddPath(ExpandConstant('{app}')); Tasks: addtopath

[Run]
Filename: "{app}\copal.exe"; Description: "{cm:LaunchProgram,Copal}"; Flags: nowait postinstall skipifsilent

[Code]
function NeedsAddPath(Dir: string): Boolean;
var
  Paths: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Paths) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Paths) + ';') = 0;
end;
