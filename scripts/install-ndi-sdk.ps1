# Silent-install the NDI 6 SDK. Mirrors what .github/workflows/windows-smoke.yml does.
# Usage (PowerShell, from the repo root):
#   ./scripts/install-ndi-sdk.ps1

$ErrorActionPreference = "Stop"

$url = "https://downloads.ndi.tv/SDK/NDI_SDK/NDI%206%20SDK.exe"
$exe = Join-Path $env:TEMP "NDI_SDK.exe"

Write-Host "Downloading NDI 6 SDK from $url ..."
Invoke-WebRequest -Uri $url -OutFile $exe
$size = (Get-Item $exe).Length
Write-Host ("Downloaded: {0:N1} MB" -f ($size / 1MB))
if ($size -lt 1MB) {
    throw "Downloaded installer looks wrong (<1MB) - URL may have changed."
}

Write-Host "Running silent installer..."
$p = Start-Process -FilePath $exe `
    -ArgumentList "/VERYSILENT","/SUPPRESSMSGBOXES","/NORESTART" -PassThru
if (-not $p.WaitForExit(600000)) {
    $p.Kill()
    throw "Installer did not finish within 10 minutes."
}
Write-Host "Installer exit code: $($p.ExitCode)"

$sdk = "C:\Program Files\NDI\NDI 6 SDK"
$lib = Join-Path $sdk "Lib\x64\Processing.NDI.Lib.x64.lib"
if (-not (Test-Path $lib)) {
    throw "NDI SDK import library not found at $lib"
}
Write-Host "OK: SDK installed at $sdk"
