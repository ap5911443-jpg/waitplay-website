# Builds dist\windows\WaitPlay-<version>-windows-x64-setup.exe (a self-extracting installer made with the IExpress tool that ships with
# Windows, so no extra software is needed) and WaitPlay-<version>-windows-x64.zip (the same files, portable) plus SHA256SUMS-windows.txt.
#
#   cargo build --release --workspace
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build_installer.ps1
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$ver = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$bin = Join-Path $root 'target\release'
$out = Join-Path $root 'dist\windows'
$stage = Join-Path $out 'stage'
Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null

foreach ($f in 'waitplay.exe', 'waitplay-bridge.exe', 'waitplay-hook.exe') {
    if (-not (Test-Path (Join-Path $bin $f))) { throw "missing $bin\$f - run: cargo build --release --workspace" }
    Copy-Item (Join-Path $bin $f) $stage
}
foreach ($f in 'install.ps1', 'install.cmd', 'uninstall.ps1', 'uninstall.cmd') { Copy-Item (Join-Path $PSScriptRoot $f) $stage }
Set-Content -Path (Join-Path $stage 'version.txt') -Value $ver -NoNewline

$name = "WaitPlay-$ver-windows-x64"
$setup = Join-Path $out "$name-setup.exe"
$files = 'waitplay.exe', 'waitplay-bridge.exe', 'waitplay-hook.exe', 'install.ps1', 'install.cmd', 'uninstall.ps1', 'uninstall.cmd', 'version.txt'
$sed = @"
[Version]
Class=IEXPRESS
SEDVersion=3
[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=1
HideExtractAnimation=1
UseLongFileName=1
InsideCompressed=0
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=
DisplayLicense=
FinishMessage=
TargetName=$setup
FriendlyName=Vibe Play $ver
AppLaunched=cmd /c install.cmd
PostInstallCmd=<None>
AdminQuietInstCmd=
UserQuietInstCmd=
SourceFiles=SourceFiles
[SourceFiles]
SourceFiles0=$stage\
[SourceFiles0]
$(($files | ForEach-Object { "%FILE$([array]::IndexOf($files, $_))%=" }) -join "`r`n")
[Strings]
$(($files | ForEach-Object { "FILE$([array]::IndexOf($files, $_))=$_" }) -join "`r`n")
"@
$sedPath = Join-Path $out 'setup.sed'
Set-Content -Path $sedPath -Value $sed -Encoding ASCII
$p = Start-Process -FilePath "$env:WINDIR\System32\iexpress.exe" -ArgumentList '/N', '/Q', $sedPath -Wait -PassThru
if (-not (Test-Path $setup)) { throw "IExpress did not produce $setup (exit $($p.ExitCode))" }

Compress-Archive -Path (Join-Path $stage '*') -DestinationPath (Join-Path $out "$name.zip") -Force
Remove-Item $stage, $sedPath -Recurse -Force
Push-Location $out
Get-ChildItem "$name*" | ForEach-Object { "$((Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower())  $($_.Name)" } | Set-Content SHA256SUMS-windows.txt -Encoding ASCII
Pop-Location
Get-ChildItem $out | Format-Table Name, Length
