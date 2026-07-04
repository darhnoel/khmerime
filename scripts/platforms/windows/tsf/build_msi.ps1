param(
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Require-Command {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string]$InstallHint
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is required. $InstallHint"
    }
}

function Resolve-WixCommand {
    $pathCommand = Get-Command "wix" -ErrorAction SilentlyContinue
    if ($pathCommand) {
        return $pathCommand.Source
    }

    $dotnetToolPath = Join-Path $env:USERPROFILE ".dotnet\tools\wix.exe"
    if (Test-Path $dotnetToolPath) {
        return $dotnetToolPath
    }

    throw "wix is required. Install WiX Toolset with: dotnet tool install --global wix"
}

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw "Windows MSI packaging must run on Windows."
}

Require-Command -Name "cargo" -InstallHint "Install Rust from https://rustup.rs/."
Require-Command -Name "rustup" -InstallHint "Install Rust from https://rustup.rs/."
$wixCommand = Resolve-WixCommand

$installedTargets = & rustup target list --installed
if ($LASTEXITCODE -ne 0) {
    throw "Failed to inspect installed Rust targets."
}
if ($installedTargets -notcontains $Target) {
    throw "Rust target $Target is not installed. Run: rustup target add $Target"
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..\..\..\..")
$cargoToml = Join-Path $repoRoot "Cargo.toml"
$wxsPath = Join-Path $repoRoot "packaging\windows\wix\KhmerIME.wxs"
$targetDir = Join-Path $repoRoot "target\windows-tsf-msi"
$stagingDir = Join-Path $targetDir "staging"
$distDir = Join-Path $repoRoot "dist\windows"

$artifactVersion = $env:KHMERIME_PACKAGE_VERSION
if (-not $artifactVersion) {
    $versionLine = Select-String -Path $cargoToml -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $versionLine) {
        throw "Could not read package version from $cargoToml."
    }
    $artifactVersion = $versionLine.Matches[0].Groups[1].Value
}

$productVersion = $env:KHMERIME_WINDOWS_PRODUCT_VERSION
if (-not $productVersion) {
    $productVersion = ($artifactVersion -split "[-+]")[0]
}
if ($productVersion -notmatch '^\d+\.\d+\.\d+(\.\d+)?$') {
    throw "Windows ProductVersion must be numeric, got '$productVersion'. Set KHMERIME_WINDOWS_PRODUCT_VERSION."
}

Write-Host "[khmerime] building Windows TSF DLL for $Target..."
Write-Host "[khmerime] artifact version: $artifactVersion"
Write-Host "[khmerime] Windows ProductVersion: $productVersion"
Push-Location $repoRoot
try {
    & cargo build -p khmerime_windows_tsf --release --target $Target --target-dir $targetDir
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed."
    }
}
finally {
    Pop-Location
}

$builtDll = Join-Path $targetDir "$Target\release\khmerime_windows_tsf.dll"
if (-not (Test-Path $builtDll)) {
    throw "Expected TSF DLL was not produced: $builtDll"
}

New-Item -ItemType Directory -Force $stagingDir | Out-Null
New-Item -ItemType Directory -Force $distDir | Out-Null

$stagedDll = Join-Path $stagingDir "khmerime_windows_tsf.dll"
Copy-Item -Force $builtDll $stagedDll

$msiPath = Join-Path $distDir "KhmerIME-$artifactVersion-windows-x64.msi"
$wixIntermediate = Join-Path $targetDir "wix"
New-Item -ItemType Directory -Force $wixIntermediate | Out-Null

Write-Host "[khmerime] building MSI: $msiPath"
& $wixCommand build `
    -arch x64 `
    -d "ProductVersion=$productVersion" `
    -d "KhmerImeDll=$stagedDll" `
    -intermediateFolder $wixIntermediate `
    -out $msiPath `
    $wxsPath

if ($LASTEXITCODE -ne 0) {
    throw "WiX MSI build failed."
}
if (-not (Test-Path $msiPath)) {
    throw "WiX completed without producing expected MSI: $msiPath"
}

Write-Host "[khmerime] Windows MSI written to: $msiPath"
