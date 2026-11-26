# Removes Vibe Play for the current user: stops it, removes the files, the Start menu shortcut, the "Launch at login" entry and the
# "Apps & features" entry. Your settings and web sign-ins stay unless you pass -RemoveData (see docs/PRIVACY.md).
param([switch]$RemoveData)
$ErrorActionPreference = 'Continue'
$dest = $PSScriptRoot

Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($dest, [StringComparison]::OrdinalIgnoreCase) } | Stop-Process -Force

$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$value = (Get-ItemProperty $run -Name WaitPlay -ErrorAction SilentlyContinue).WaitPlay
if ($value -and $value.IndexOf($dest, [StringComparison]::OrdinalIgnoreCase) -ge 0) { Remove-ItemProperty $run -Name WaitPlay -ErrorAction SilentlyContinue }

Remove-Item (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\WaitPlay.lnk') -ErrorAction SilentlyContinue
Remove-Item 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\WaitPlay' -Recurse -ErrorAction SilentlyContinue
if ($RemoveData) {
    Remove-Item (Join-Path $env:LOCALAPPDATA 'WaitPlay') -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $env:TEMP 'waitplay') -Recurse -Force -ErrorAction SilentlyContinue
}

# this script lives in the folder it deletes: remove the folder from a detached process after we exit
Start-Process cmd.exe -ArgumentList '/c', "ping -n 3 127.0.0.1 >nul & rmdir /s /q `"$dest`"" -WindowStyle Hidden
