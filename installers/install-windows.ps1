# Warden Installation Script for Windows
# Installs Warden CLI to Program Files or user local bin directory
#
# Usage:
#   powershell -Command "& { $(irm https://raw.githubusercontent.com/sergiogswv/warden/installers/install-windows.ps1) }"
# or with elevated privileges:
#   powershell -Command "& { $(irm https://raw.githubusercontent.com/sergiogswv/warden/installers/install-windows.ps1) }" -AsAdmin

param(
    [string]$InstallDir = "$env:LOCALAPPDATA\warden\bin",
    [string]$Version = "latest"
)

# Enable error handling
$ErrorActionPreference = "Stop"

# Colors for output
function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host "→ $Message" -ForegroundColor Cyan
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

# Header
Write-Host "┌──────────────────────────────────────┐" -ForegroundColor Blue
Write-Host "│  Warden Installation Script (Win)   │" -ForegroundColor Blue
Write-Host "└──────────────────────────────────────┘" -ForegroundColor Blue
Write-Host ""

# System information
Write-Info "System Information:"
Write-Host "  • OS: Windows $([System.Environment]::OSVersion.VersionString)"
Write-Host "  • Architecture: $([System.Environment]::Is64BitProcess ? 'x64' : 'x86')"
Write-Host "  • Install Directory: $InstallDir"
Write-Host ""

# Create installation directory if it doesn't exist
if (-not (Test-Path $InstallDir)) {
    try {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        Write-Success "Created installation directory"
    }
    catch {
        Write-Error "Failed to create installation directory: $_"
        exit 1
    }
}

# Download Warden binary
$GithubRepo = "sergiogswv/warden"
$BinaryName = "warden-windows-x64.exe"
$DownloadUrl = "https://github.com/$GithubRepo/releases/download/$Version/$BinaryName"

Write-Info "Downloading Warden $Version..."

try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $DownloadUrl -OutFile "$InstallDir\warden.exe" -UseBasicParsing
    Write-Success "Successfully downloaded Warden"
}
catch {
    Write-Error "Failed to download Warden: $_"
    exit 1
}

# Verify installation
$WardePath = "$InstallDir\warden.exe"
if (Test-Path $WardePath) {
    Write-Success "Warden installed to: $WardePath"
    Write-Host ""

    # Add to PATH if needed
    $PathVar = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($PathVar -notlike "*$InstallDir*") {
        try {
            [Environment]::SetEnvironmentVariable("Path", "$PathVar;$InstallDir", "User")
            Write-Success "Added Warden to user PATH"
        }
        catch {
            Write-Info "Could not automatically add to PATH. Please add manually:"
            Write-Host "  $InstallDir"
        }
    }

    Write-Host ""
    Write-Success "Warden is ready to use!"
    Write-Host "  Try: warden --help"
    Write-Host ""
    Write-Info "Note: You may need to restart PowerShell/CMD for PATH changes to take effect"
}
else {
    Write-Error "Installation verification failed"
    exit 1
}
