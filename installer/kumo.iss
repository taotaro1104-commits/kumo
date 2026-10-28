; Kumo のインストーラー（Inno Setup 6）。scripts/installer.ps1 から `ISCC /DAppVersion=... /DSourceDir=...` で作る。
; 既定は管理者権限なしの利用者単位（%LOCALAPPDATA%\Programs\Kumo）。画面で「すべてのユーザー」を選ぶと Program Files へ入れる。

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\dist\stage"
#endif

[Setup]
; アプリを一意に決める ID（変えると別のアプリとして入る）
AppId={{6F0B5A52-9C3E-4C58-8B1D-3A7E2D1F4C90}
AppName=Kumo
AppVersion={#AppVersion}
AppPublisher=Kumo
AppPublisherURL=https://github.com/taotaro1104-commits/kumo
DefaultDirName={autopf}\Kumo
DefaultGroupName=Kumo
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=kumo-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\kumo.exe
UninstallDisplayName=Kumo
VersionInfoVersion={#AppVersion}
LicenseFile={#SourceDir}\LICENSE.txt

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\kumo.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Kumo"; Filename: "{app}\kumo.exe"
Name: "{autodesktop}\Kumo"; Filename: "{app}\kumo.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\kumo.exe"; Description: "{cm:LaunchProgram,Kumo}"; Flags: nowait postinstall skipifsilent

; アンインストールしても、履歴・設定・パスワード（%LOCALAPPDATA%\Kumo）は消さない。
