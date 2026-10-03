# Signs and verifies a Windows executable using the code-signing certificate
# supplied through the WINDOWS_CERTIFICATE and WINDOWS_CERTIFICATE_PASSWORD
# environment variables (base64-encoded PFX and its password).
#
# Usage: ./packaging/windows/sign.ps1 -Target path\to\PacketSmith.exe

param(
    [Parameter(Mandatory = $true)]
    [string]$Target
)

$ErrorActionPreference = "Stop"

if (-not $env:WINDOWS_CERTIFICATE) {
    throw "WINDOWS_CERTIFICATE is not set; configure the signing secret or skip signing"
}
if (-not (Test-Path $Target)) {
    throw "signing target not found: $Target"
}

$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName |
    Select-Object -Last 1
if (-not $signtool) {
    throw "signtool.exe not found; install the Windows SDK"
}

$temporaryDirectory = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$certificatePath = Join-Path $temporaryDirectory "packetsmith-certificate.pfx"
[IO.File]::WriteAllBytes($certificatePath, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))

& $signtool.FullName sign /fd SHA256 /f $certificatePath /p $env:WINDOWS_CERTIFICATE_PASSWORD /tr http://timestamp.digicert.com /td SHA256 $Target
if ($LASTEXITCODE -ne 0) {
    throw "signtool sign failed for $Target"
}

& $signtool.FullName verify /pa $Target
if ($LASTEXITCODE -ne 0) {
    throw "signtool verify failed for $Target"
}

Remove-Item -Force $certificatePath
Write-Host "Signed and verified $Target"
