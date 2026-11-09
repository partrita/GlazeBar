#ifndef BundleDir
  #error BundleDir is required
#endif
#ifndef OutputDir
  #error OutputDir is required
#endif
#ifndef Arch
  #define Arch "arm64"
#endif
#ifndef TestProfile
  #define TestProfile ""
#endif
#if TestProfile != ""
  #define ProductId "GlazeBar-Setup-Isolated-Test"
  #define FileSuffix "-test"
#else
  #define ProductId "GlazeBar-GlazeWM-Community"
  #define FileSuffix ""
#endif

#ifndef AppVersion
  #define AppVersion "0.0.11"
#endif

[Setup]
AppId={#ProductId}
AppName=GlazeBar + GlazeWM
AppVersion={#AppVersion}
AppPublisher=deepthough
AppPublisherURL=https://github.com/partrita/GlazeBar
AppSupportURL=https://github.com/partrita/GlazeBar/issues
DefaultDirName={localappdata}\Programs\GlazeBar-GlazeWM
DefaultGroupName=GlazeBar + GlazeWM
PrivilegesRequired=lowest
#if Arch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible and not arm64
ArchitecturesInstallIn64BitMode=x64compatible
#endif
MinVersion=10.0.22000
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableWelcomePage=no
LicenseFile={#BundleDir}\LICENSE
InfoBeforeFile=Setup-Info.txt
OutputDir={#OutputDir}
OutputBaseFilename=GlazeBar-GlazeWM-Setup-{#AppVersion}-{#Arch}{#FileSuffix}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
SetupLogging=yes
CloseApplications=no
RestartApplications=no
UninstallDisplayName=GlazeBar + GlazeWM (community setup)
UninstallDisplayIcon={app}\bundle\payload\GlazeBar.exe

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "{cm:AutoStart}"; Flags: unchecked

[CustomMessages]
korean.AutoStart=Windows 로그인 시 GlazeBar + GlazeWM 자동 시작
english.AutoStart=Start GlazeBar + GlazeWM when I sign in
korean.ThemePageTitle=기본 테마 선택
korean.ThemePageDesc=GlazeBar와 GlazeWM에 적용할 첫 컬러 테마를 선택하세요.
korean.ThemePageSub=언제든지 GlazeBar 설정 또는 단축키(Alt+M)를 통해 테마를 변경할 수 있습니다.
english.ThemePageTitle=Select Default Theme
english.ThemePageDesc=Choose the initial color scheme for GlazeBar and GlazeWM.
english.ThemePageSub=You can change themes anytime from GlazeBar settings or using shortcut (Alt+M).

[Files]
Source: "{#BundleDir}\*"; DestDir: "{app}\bundle"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "Setup-Bridge.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "Setup-Info.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "GlazeBar-Setup.iss"; DestDir: "{app}\setup-source"; Flags: ignoreversion
Source: "Build-Setup.ps1"; DestDir: "{app}\setup-source"; Flags: ignoreversion
Source: "Test-SetupStartup.ps1"; DestDir: "{app}\setup-source"; Flags: ignoreversion
Source: "Setup-Bridge.ps1"; DestDir: "{app}\setup-source"; Flags: ignoreversion
Source: "Setup-Info.txt"; DestDir: "{app}\setup-source"; Flags: ignoreversion
Source: "BUILDING.md"; DestDir: "{app}\setup-source"; Flags: ignoreversion
Source: "..\licenses\INNO-SETUP-LICENSE.txt"; DestDir: "{app}\setup-source"; Flags: ignoreversion

[Icons]
#if TestProfile == ""
Name: "{group}\GlazeBar + GlazeWM"; Filename: "{code:GetProfileRoot}\.glzr\glazewm\wm\glazewm.exe"; Parameters: "start"; WorkingDir: "{code:GetProfileRoot}\.glzr\glazewm\wm"
Name: "{group}\Installation and licenses"; Filename: "{app}\Setup-Info.txt"
Name: "{group}\Uninstall GlazeBar bundle"; Filename: "{uninstallexe}"
#endif

[Run]
#if TestProfile == ""
Filename: "{code:GetProfileRoot}\\.glzr\\glazewm\\wm\\glazewm.exe"; Parameters: "start"; WorkingDir: "{code:GetProfileRoot}\\.glzr\\glazewm\\wm"; Description: "{cm:LaunchProgram,GlazeBar + GlazeWM}"; Flags: nowait postinstall skipifsilent
#endif

[Code]
var
  StagingRoot: String;
  PayloadInstalled, SetupCompleted: Boolean;
  ThemePage: TInputOptionWizardPage;

procedure ExitProcess(ExitCode: Cardinal);
  external 'ExitProcess@kernel32.dll stdcall';
function GetProfileRoot(Param: String): String;
begin
  Result := GetEnv('USERPROFILE');
end;

function InitializeSetup: Boolean;
var
  StatePath, ProfileRoot: String;
  I: Integer;
begin
  Result := False;
  ProfileRoot := GetProfileRoot('');
  if (ProfileRoot = '') or not DirExists(ProfileRoot) then begin
    SuppressibleMsgBox('Windows user profile could not be found.', mbError, MB_OK, IDOK);
    exit;
  end;
  StatePath := AddBackslash(ProfileRoot) + '.glzr\glazewm\community-install.json';
  { Exercise the production startup path without installing anything. }
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/CHECKSTARTUP') = 0 then begin
      Log('GlazeBar startup check passed: ' + StatePath);
      exit;
    end;
#if TestProfile != ""
  StatePath := '{#TestProfile}\.glzr\glazewm\community-install.json';
#endif
  Result := True;
end;

function GetPowershellExe: String;
begin
  Result := ExpandConstant('{sysnative}\WindowsPowerShell\v1.0\powershell.exe');
  if not FileExists(Result) then
    Result := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
end;

function ReadLogContent(LogFile: String): String;
var
  Lines: TArrayOfString;
  I, StartIdx: Integer;
begin
  Result := '';
  if FileExists(LogFile) then begin
    if LoadStringsFromFile(LogFile, Lines) then begin
      StartIdx := 0;
      if GetArrayLength(Lines) > 10 then
        StartIdx := GetArrayLength(Lines) - 10;
      for I := StartIdx to GetArrayLength(Lines) - 1 do begin
        if Result <> '' then Result := Result + #13#10;
        Result := Result + Lines[I];
      end;
    end;
  end;
end;

function GetSelectedTheme: String;
begin
  Result := 'tokyo-night';
  if ThemePage <> nil then begin
    case ThemePage.SelectedValueIndex of
      0: Result := 'tokyo-night';
      1: Result := 'catppuccin';
      2: Result := 'everforest';
      3: Result := 'rose-pine';
      4: Result := 'monokai';
      5: Result := 'dracula';
    end;
  end;
end;

procedure InitializeWizard;
begin
  ThemePage := CreateInputOptionPage(wpSelectTasks,
    ExpandConstant('{cm:ThemePageTitle}'),
    ExpandConstant('{cm:ThemePageDesc}'),
    ExpandConstant('{cm:ThemePageSub}'),
    True, False);

  ThemePage.Add('Tokyo Night (도쿄의 밤하늘 테마, 권장)');
  ThemePage.Add('Catppuccin Mocha (부드러운 파스텔 톤)');
  ThemePage.Add('Everforest (편안하고 자연스러운 그린/포레스트)');
  ThemePage.Add('Rosé Pine (우아한 소나무와 장미빛)');
  ThemePage.Add('Monokai (선명하고 또렷한 클래식 코딩 테마)');
  ThemePage.Add('Dracula (깊은 보라색과 네온 대비)');

  ThemePage.SelectedValueIndex := 0;
end;

function RunBridgeAt(BasePath, Action: String): Boolean;
var
  Code: Integer;
  Args: String;
begin
  Args := '-NoProfile -NonInteractive -ExecutionPolicy RemoteSigned -File "' +
    BasePath + '\Setup-Bridge.ps1' + '" -Action ' + Action +
    ' -Bundle "' + BasePath + '\bundle' + '"';
  if Action = 'Install' then begin
    if WizardIsTaskSelected('autostart') then
      Args := Args + ' -AutoStart';
    Args := Args + ' -Theme ' + GetSelectedTheme;
  end;
#if TestProfile != ""
  Args := Args + ' -TestProfile "{#TestProfile}"';
#endif
  Result := Exec(GetPowershellExe,
    Args, BasePath, SW_HIDE, ewWaitUntilTerminated, Code);
  Result := Result and (Code = 0);
end;

function UnquoteStr(const S: String): String;
begin
  if (Length(S) >= 2) and (S[1] = '"') and (S[Length(S)] = '"') then
    Result := Copy(S, 2, Length(S) - 2)
  else
    Result := S;
end;

function GetPreviousUninstallString: String;
var
  S: String;
begin
  Result := '';
  if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#ProductId}_is1', 'UninstallString', S) then
    Result := S;
end;

function UninstallPreviousVersion: Boolean;
var
  UninstStr, UninstExe: String;
  ResultCode: Integer;
begin
  Result := True;
  UninstStr := GetPreviousUninstallString;
  if UninstStr <> '' then begin
    UninstExe := UnquoteStr(UninstStr);
    if FileExists(UninstExe) then begin
      Log('Previous installation detected: ' + UninstExe + '. Uninstalling previous version...');
      if Exec(UninstExe, '/SILENT /NORESTART /SUPPRESSMSGBOXES', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then begin
        Result := (ResultCode = 0);
        Log('Previous version uninstaller finished with code: ' + IntToStr(ResultCode));
      end else begin
        Log('Failed to execute previous version uninstaller.');
        Result := False;
      end;
    end;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  InstallLog, LogSummary: String;
begin
  Result := '';
  if PayloadInstalled then exit;
  ExtractTemporaryFiles('*');
  StagingRoot := ExpandConstant('{tmp}\') + '{app}';
  RunBridgeAt(StagingRoot, 'StopProcesses');
  UninstallPreviousVersion;
  if not RunBridgeAt(StagingRoot, 'Install') then begin
    InstallLog := ExpandConstant('{localappdata}\GlazeBarSetup\install.log');
    LogSummary := ReadLogContent(InstallLog);
    if LogSummary <> '' then
      Result := 'Installation failed:' + #13#10#13#10 + LogSummary
    else
      Result := 'Installation failed. PowerShell returned an error before logging could begin.' + #13#10 +
                'Path: ' + StagingRoot + '\Setup-Bridge.ps1';
  end else
    PayloadInstalled := True;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssDone then SetupCompleted := True;
end;

procedure DeinitializeSetup;
begin
  if PayloadInstalled and not SetupCompleted then
    if not RunBridgeAt(StagingRoot, 'Uninstall') then
      SuppressibleMsgBox('Installation was interrupted and settings could not be restored. Keep the community backup and check the uninstall log.', mbError, MB_OK, IDOK);
end;
function InitializeUninstall: Boolean;
begin
  Result := RunBridgeAt(ExpandConstant('{app}'), 'CheckUninstall');
  if not Result then
    SuppressibleMsgBox('Restore could not finish. Close GlazeWM and GlazeBar and retry. The uninstaller and backups have been retained. Details: ' +
      ExpandConstant('{localappdata}\GlazeBarSetup\uninstall.log'), mbError, MB_OK, IDOK);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    if not RunBridgeAt(ExpandConstant('{app}'), 'Uninstall') then begin
      SuppressibleMsgBox('Settings could not be restored. Setup files were retained. Check the uninstall log before retrying.', mbError, MB_OK, IDOK);
      ExitProcess(1);
    end;
end;
