#Requires -Version 5.1
<#
.SYNOPSIS
    Build SnapDock release and sign the resulting executable.

.DESCRIPTION
    1. Builds the x86_64-pc-windows-gnu release binary with cargo.
    2. Copies WebView2Loader.dll next to the binary.
    3. Calls sign-release.ps1 to Authenticode-sign the executable.

.PARAMETER CertificatePath
    Path to the .pfx / .p12 code-signing certificate.

.PARAMETER CertificatePassword
    Password for the .pfx file. If omitted, the script prompts securely.

.PARAMETER TimestampServer
    RFC 3161 timestamp server. Default: http://timestamp.digicert.com

.EXAMPLE
    .\build-and-sign.ps1 -CertificatePath "C:\certs\snapdock.pfx"
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$CertificatePath,

    [Parameter(Mandatory = $false)]
    [Security.SecureString]$CertificatePassword,

    [Parameter(Mandatory = $false)]
    [string]$TimestampServer = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Definition
$rustTarget = "x86_64-pc-windows-gnu"
$releaseDir = Join-Path $root "src-tauri" "target" $rustTarget "release"
$exe = Join-Path $releaseDir "snapdock.exe"

# Ensure gcc is on PATH for the GNU target
$gccPath = "C:\mingw64_extract\mingw64\bin"
if (Test-Path -LiteralPath $gccPath) {
    $env:PATH = "$gccPath;$env:PATH"
    Write-Host "Prepended MinGW to PATH."
}

# 1. Build release binary
Write-Host "Building release binary (target=$rustTarget) ..."
Push-Location (Join-Path $root "src-tauri")
try {
    cargo build --target $rustTarget --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}
finally {
    Pop-Location
}

# 2. Copy WebView2Loader.dll if it exists in the project
$loaderSource = Join-Path $root "src-tauri" "WebView2Loader.dll"
$loaderDest = Join-Path $releaseDir "WebView2Loader.dll"
if (Test-Path -LiteralPath $loaderSource) {
    Copy-Item -LiteralPath $loaderSource -Destination $loaderDest -Force
    Write-Host "Copied WebView2Loader.dll to release directory."
}
else {
    Write-Warning "WebView2Loader.dll not found at $loaderSource. The portable build will need it."
}

# 3. Sign the executable
$signArgs = @{
    CertificatePath = $CertificatePath
    TimestampServer = $TimestampServer
    ExecutablePath = $exe
}
if ($CertificatePassword) {
    $signArgs.CertificatePassword = $CertificatePassword
}

& (Join-Path $root "sign-release.ps1") @signArgs
if ($LASTEXITCODE -ne 0) { throw "Signing failed" }

Write-Host "Done. Signed executable: $exe"
