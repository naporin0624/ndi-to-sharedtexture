# Fetch the Spout2 SDK source into vendor/Spout2, preserving the upstream
# layout. SpoutDX.h uses relative includes (e.g. ../../SpoutGL/SpoutCommon.h),
# so build.rs expects:
#   vendor/Spout2/SpoutDirectX/SpoutDX/
#   vendor/Spout2/SpoutGL/
#
# Usage (PowerShell, from the repo root):
#   ./scripts/fetch-spout2.ps1

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (Test-Path "vendor\Spout2\SpoutGL\SpoutDX.h" -PathType Leaf) {
    Write-Host "vendor/Spout2 already present; skipping."
    exit 0
}

$tmp = "_spout2_tmp"
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
git clone --depth 1 https://github.com/leadedge/Spout2.git $tmp

New-Item -ItemType Directory -Force "vendor\Spout2" | Out-Null
Copy-Item -Recurse -Force "$tmp\SPOUTSDK\SpoutDirectX" "vendor\Spout2\SpoutDirectX"
Copy-Item -Recurse -Force "$tmp\SPOUTSDK\SpoutGL"       "vendor\Spout2\SpoutGL"
Remove-Item -Recurse -Force $tmp

Write-Host "Spout2 SDK fetched into vendor/Spout2"
