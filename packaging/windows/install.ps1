# Installs Vibe Play for the current user (no administrator rights): copies the three executables to
# %LOCALAPPDATA%\Programs\WaitPlay, adds a Start menu shortcut and an "Apps & features" entry, and starts the app.
# Test overrides: WAITPLAY_INSTALL_DIR (destination), WAITPLAY_NO_START=1 (do not launch).
$ErrorActionPreference = 'Stop'
$ver = (Get-Content (Join-Path $PSScriptRoot 'version.txt')).Trim()
$dest = if ($env:WAITPLAY_INSTALL_DIR) { $env:WAITPLAY_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\WaitPlay' }

# an update: stop the running copies that live in the destination (and only those)
Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($dest, [StringComparison]::OrdinalIgnoreCase) } | Stop-Process -Force
Start-Sleep -Milliseconds 500

New-Item -ItemType Directory -Force $dest | Out-Null
foreach ($f in 'waitplay.exe', 'waitplay-bridge.exe', 'waitplay-hook.exe', 'uninstall.ps1', 'uninstall.cmd', 'version.txt') {
    Copy-Item (Join-Path $PSScriptRoot $f) $dest -Force
}

$programs = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
# earlier versions named the shortcut WaitPlay; the Start menu and search show the product name
Remove-Item (Join-Path $programs 'WaitPlay.lnk') -ErrorAction SilentlyContinue
$shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path $programs 'Vibe Play.lnk'))
$shortcut.TargetPath = Join-Path $dest 'waitplay.exe'
$shortcut.WorkingDirectory = $dest
$shortcut.Description = 'Vibe Play'
$shortcut.IconLocation = (Join-Path $dest 'waitplay.exe') + ',0'
$shortcut.Save()

$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\WaitPlay'
New-Item -Force $key | Out-Null
Set-ItemProperty $key DisplayName 'Vibe Play'
Set-ItemProperty $key DisplayVersion $ver
Set-ItemProperty $key Publisher 'Vibe Play'
Set-ItemProperty $key InstallLocation $dest
Set-ItemProperty $key DisplayIcon (Join-Path $dest 'waitplay.exe')
Set-ItemProperty $key UninstallString ('cmd /c "' + (Join-Path $dest 'uninstall.cmd') + '"')
Set-ItemProperty $key NoModify 1 -Type DWord
Set-ItemProperty $key NoRepair 1 -Type DWord

if (-not $env:WAITPLAY_NO_START) { Start-Process (Join-Path $dest 'waitplay.exe') }
