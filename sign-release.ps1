#Requires -Version 5.1
<#
.SYNOPSIS
    Sign the SnapDock release executable with an Authenticode certificate.

.DESCRIPTION
    Uses osslsigncode (installed via winget) to sign the MinGW release build.
    The signed binary is written back to the same path, then verified.

.PARAMETER CertificatePath
    Path to the .pfx / .p12 code-signing certificate.

.PARAMETER CertificatePassword
    Password for the .pfx file. If omitted, the script prompts securely.

.PARAMETER TimestampServer
    RFC 3161 timestamp server. Default: http://timestamp.digicert.com

.PARAMETER ExecutablePath
    Path to the executable to sign.
    Default: .\src-tauri\target\x86_64-pc-windows-gnu\release\snapdock.exe

.EXAMPLE
    .\sign-release.ps1 -CertificatePath "C:\certs\snapdock.pfx"
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$CertificatePath,

    [Parameter(Mandatory = $false)]
    [Security.SecureString]$CertificatePassword,

    [Parameter(Mandatory = $false)]
    [string]$TimestampServer = "http://timestamp.digicert.com",

    [Parameter(Mandatory = $false)]
    [string]$ExecutablePath = ".\src-tauri\target\x86_64-pc-windows-gnu\release\snapdock.exe"
)

$ErrorActionPreference = "Stop"

# Resolve absolute paths
$cert = Resolve-Path -LiteralPath $CertificatePath
$exe = Resolve-Path -LiteralPath $ExecutablePath
$osslsigncode = "C:\Users\Administrator\AppData\Local\Microsoft\WinGet\Packages\MichalTrojnara.osslsigncode_Microsoft.Winget.Source_8wekyb3d8bbwe\bin\osslsigncode.exe"

if (-not (Test-Path -LiteralPath $osslsigncode)) {
    Write-Error "osslsigncode not found at '$osslsigncode'. Install with: winget install --id MichalTrojnara.osslsigncode"
}

if (-not (Test-Path -LiteralPath $cert)) {
    Write-Error "Certificate not found: $cert"
}

if (-not (Test-Path -LiteralPath $exe)) {
    Write-Error "Executable not found: $exe"
}

# Prompt for password if not provided
if (-not $CertificatePassword) {
    $CertificatePassword = Read-Host -Prompt "Enter PFX password" -AsSecureString
}
$plainPassword = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
    [Runtime.InteropServices.Marshal]::SecureStringToBSTR($CertificatePassword)
)

$tmpExe = "$exe.signed.tmp"
if (Test-Path -LiteralPath $tmpExe) { Remove-Item -LiteralPath $tmpExe -Force }

try {
    Write-Host "Signing $exe ..."
    & $osslsigncode sign `
        -pkcs12 $cert `
        -pass $plainPassword `
        -n "SnapDock" `
        -i "https://snapdock.app" `
        -t $TimestampServer `
        -h sha256 `
        -in $exe `
        -out $tmpExe

    if ($LASTEXITCODE -ne 0) {
        throw "osslsigncode failed with exit code $LASTEXITCODE"
    }

    Move-Item -LiteralPath $tmpExe -Destination $exe -Force
    Write-Host "Signature applied. Verifying ..."

    & $osslsigncode verify -in $exe
    if ($LASTEXITCODE -ne 0) {
        throw "Signature verification failed with exit code $LASTEXITCODE"
    }

    Write-Host "OK: $exe is signed and verified."
}
finally {
    if (Test-Path -LiteralPath $tmpExe) { Remove-Item -LiteralPath $tmpExe -Force }
    if ($plainPassword) { $plainPassword = $null }
}
