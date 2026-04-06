# Build Chatminal for Windows and produce a .zip
# Usage: pwsh scripts/release/build-windows.ps1 [-SkipBuild]
param(
    [switch]$SkipBuild
)
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path "$PSScriptRoot\..\.."
$TargetDir = "$RepoRoot\target\release"
$AssetDir = "$RepoRoot\apps\desktop\assets\windows"
$DistDir = "$RepoRoot\dist"

# Read version from workspace Cargo.toml
$VersionLine = Get-Content "$RepoRoot\Cargo.toml" | Where-Object { $_ -match '^version\s*=' } | Select-Object -First 1
$Version = if ($env:CHATMINAL_VERSION) { $env:CHATMINAL_VERSION }
           elseif ($VersionLine -match '"([\d.]+)"') { $Matches[1] }
           else { "0.0.0" }

$ArchiveName = "Chatminal-$Version-windows-x86_64.zip"
$StagingDir = "$RepoRoot\target\windows-staging\chatminal-$Version"

Write-Host "==> Building Chatminal $Version for Windows"

# ── 1. Bootstrap vendor deps (only if not already done by CI step) ────────────
$ZlibSentinel = "$RepoRoot\vendor\terminal-deps\freetype\zlib\adler32.c"
if (-not (Test-Path $ZlibSentinel)) {
    Write-Host "==> Bootstrapping vendor deps via Git Bash"
    bash "$RepoRoot/scripts/bootstrap-terminal-vendor-deps.sh"
    if ($LASTEXITCODE -ne 0) { throw "bootstrap failed" }

    # Generate zconf.h for MSVC: raw zlib git repo does not include it
    $ZlibDir = "$RepoRoot\vendor\terminal-deps\freetype\zlib"
    $ZconfH = "$ZlibDir\zconf.h"
    if ((Test-Path "$ZlibDir\CMakeLists.txt") -and (-not (Test-Path $ZconfH))) {
        Write-Host "==> Generating zconf.h via cmake"
        $GenDir = "$ZlibDir\_cmake_gen"
        cmake -S $ZlibDir -B $GenDir -G "NMake Makefiles" -DCMAKE_BUILD_TYPE=Release 2>&1 | Out-Null
        if (Test-Path "$GenDir\zconf.h") {
            Copy-Item "$GenDir\zconf.h" $ZconfH
        }
        Remove-Item -Recurse -Force $GenDir -ErrorAction SilentlyContinue
    }
}

# ── 2. Cargo release build ────────────────────────────────────────────────────
if (-not $SkipBuild) {
    cargo build --release --manifest-path "$RepoRoot\apps\desktop\Cargo.toml"
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

# ── 2. Stage files ────────────────────────────────────────────────────────────
Write-Host "==> Staging files"
if (Test-Path $StagingDir) { Remove-Item -Recurse -Force $StagingDir }
New-Item -ItemType Directory -Force $StagingDir | Out-Null

# Main binary
Copy-Item "$TargetDir\desktop.exe" "$StagingDir\chatminal.exe"

# ANGLE (OpenGL ES via DirectX)
foreach ($dll in @("libEGL.dll", "libGLESv2.dll")) {
    $src = "$AssetDir\angle\$dll"
    if (Test-Path $src) { Copy-Item $src "$StagingDir\$dll" }
}

# ConPTY support
foreach ($f in @("conpty.dll", "OpenConsole.exe")) {
    $src = "$AssetDir\conhost\$f"
    if (Test-Path $src) { Copy-Item $src "$StagingDir\$f" }
}

# Mesa software renderer (fallback for systems without GPU/DirectX support)
$MesaDir = "$StagingDir\mesa"
New-Item -ItemType Directory -Force $MesaDir | Out-Null
$MesaSrc = "$AssetDir\mesa\opengl32.dll"
if (Test-Path $MesaSrc) { Copy-Item $MesaSrc "$MesaDir\opengl32.dll" }

# ── 3. Create .zip ────────────────────────────────────────────────────────────
Write-Host "==> Creating $ArchiveName"
New-Item -ItemType Directory -Force $DistDir | Out-Null
$ZipPath = "$DistDir\$ArchiveName"
if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }

Compress-Archive -Path "$StagingDir\*" -DestinationPath $ZipPath

Remove-Item -Recurse -Force (Split-Path $StagingDir)
Write-Host "==> Done: $ZipPath"
