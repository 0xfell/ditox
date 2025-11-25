# Build Ditox Installer
# This script:
# 1. Reads version from Cargo.toml
# 2. Builds release binary
# 3. Compiles the Inno Setup installer (passing version dynamically)
#
# Usage:
#   .\build-installer.ps1           # Full build
#   .\build-installer.ps1 -SkipBuild # Skip cargo build (if already built)
#   .\build-installer.ps1 -Quiet     # Minimal output

param(
    [switch]$SkipBuild,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$StartTime = Get-Date

# Paths
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$CargoToml = Join-Path $ProjectRoot "Cargo.toml"
$SetupIss = Join-Path $ProjectRoot "ditox-gui\installer\setup.iss"
$OutputDir = Join-Path $ProjectRoot "target\installer"

function Write-Step($message) {
    if (-not $Quiet) {
        Write-Host "`n[$((Get-Date).ToString('HH:mm:ss'))] $message" -ForegroundColor Cyan
    }
}

function Write-Success($message) {
    if (-not $Quiet) {
        Write-Host $message -ForegroundColor Green
    }
}

function Get-InnoSetupPath {
    # Try standard location
    $StandardPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
    if (Test-Path $StandardPath) {
        return $StandardPath
    }
    
    # Try registry
    try {
        $RegPath = Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1" -Name "InstallLocation" -ErrorAction SilentlyContinue
        if ($RegPath) {
            $ExePath = Join-Path $RegPath.InstallLocation "ISCC.exe"
            if (Test-Path $ExePath) {
                return $ExePath
            }
        }
    } catch {}

    return $null
}

# Banner
if (-not $Quiet) {
    Write-Host ""
    Write-Host "  ____  _ _            " -ForegroundColor Magenta
    Write-Host " |  _ \(_) |_ _____  __" -ForegroundColor Magenta
    Write-Host " | | | | | __/ _ \ \/ /" -ForegroundColor Magenta
    Write-Host " | |_| | | || (_) >  < " -ForegroundColor Magenta
    Write-Host " |____/|_|\__\___/_/\_\" -ForegroundColor Magenta
    Write-Host ""
    Write-Host " Windows Installer Builder" -ForegroundColor White
    Write-Host ""
}

# Step 0: Check prerequisites
$InnoSetup = Get-InnoSetupPath
if (-not $InnoSetup) {
    Write-Host "`nInno Setup not found!" -ForegroundColor Red
    Write-Host "Please install Inno Setup from: https://jrsoftware.org/isdl.php" -ForegroundColor Yellow
    exit 1
}

# Step 1: Extract version from Cargo.toml
Write-Step "Reading version from Cargo.toml..."
$CargoContent = Get-Content $CargoToml -Raw
if ($CargoContent -match 'version\s*=\s*"([^"]+)"') {
    $Version = $Matches[1]
    Write-Success "  Version: $Version"
} else {
    Write-Error "Could not find version in Cargo.toml"
    exit 1
}

# Step 2: Build release binary
if (-not $SkipBuild) {
    Write-Step "Building release binary..."
    
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "Cargo not found in PATH"
        exit 1
    }

    Push-Location $ProjectRoot
    try {
        if ($Quiet) {
            cargo build --release -p ditox-gui 2>&1 | Out-Null
        } else {
            cargo build --release -p ditox-gui
        }
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Cargo build failed"
            exit 1
        }
        $ExePath = Join-Path $ProjectRoot "target\release\ditox-gui.exe"
        if (Test-Path $ExePath) {
            $ExeSize = [math]::Round((Get-Item $ExePath).Length / 1MB, 2)
            Write-Success "  Built: ditox-gui.exe ($ExeSize MB)"
        }
    } finally {
        Pop-Location
    }
} else {
    Write-Step "Skipping cargo build (using existing binary)"
}

# Step 3: Compile installer
Write-Step "Compiling installer with Inno Setup..."
# Pass version to Inno Setup via command line
$InnoArgs = @("/Qp", "/DMyAppVersion=$Version", $SetupIss)

if ($Quiet) {
    & $InnoSetup $InnoArgs 2>&1 | Out-Null
} else {
    & $InnoSetup $InnoArgs
}

if ($LASTEXITCODE -ne 0) {
    Write-Error "Inno Setup compilation failed"
    exit 1
}

# Final output
$InstallerName = "ditox-setup-$Version.exe"
$InstallerPath = Join-Path $OutputDir $InstallerName
$Duration = [math]::Round(((Get-Date) - $StartTime).TotalSeconds, 1)

if (Test-Path $InstallerPath) {
    $InstallerSize = [math]::Round((Get-Item $InstallerPath).Length / 1MB, 2)

    if ($Quiet) {
        Write-Host $InstallerPath
    } else {
        Write-Host ""
        Write-Host "  ========================================" -ForegroundColor Green
        Write-Host "  BUILD SUCCESSFUL" -ForegroundColor Green
        Write-Host "  ========================================" -ForegroundColor Green
        Write-Host ""
        Write-Host "  Version:    $Version" -ForegroundColor White
        Write-Host "  Size:       $InstallerSize MB" -ForegroundColor White
        Write-Host "  Time:       $Duration seconds" -ForegroundColor White
        Write-Host ""
        Write-Host "  Output:" -ForegroundColor White
        Write-Host "  $InstallerPath" -ForegroundColor Yellow
        Write-Host ""
    }
} else {
    Write-Error "Installer not found at expected location: $InstallerPath"
    exit 1
}
