# Download claude_hooks binary from GitHub releases
# Version is fetched from the latest release (source of truth: Cargo.toml)

$ErrorActionPreference = "Stop"

$REPO = "waki285/dotfiles"
$HOOKS_DIR = Join-Path $env:USERPROFILE ".claude\hooks"
$BINARY_NAME = "claude_hooks.exe"
$VERSION_FILE = Join-Path $HOOKS_DIR ".claude_hooks_version"

# Get latest version from GitHub API
function Get-LatestVersion {
    $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases" -UseBasicParsing
    foreach ($release in $releases) {
        if ($release.tag_name -match "^claude_hooks-(v.+)$") {
            return $Matches[1]
        }
    }
    return $null
}

$VERSION = Get-LatestVersion

if (-not $VERSION) {
    Write-Error "Could not determine latest version"
    exit 1
}

# Detect architecture
$ARCH = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

switch ($ARCH) {
    "X64" {
        $ASSET_NAME = "claude_hooks-windows-x86_64.exe"
    }
    "Arm64" {
        $ASSET_NAME = "claude_hooks-windows-arm64.exe"
    }
    default {
        Write-Error "Unsupported architecture: $ARCH"
        exit 1
    }
}

$DOWNLOAD_URL = "https://github.com/$REPO/releases/download/claude_hooks-$VERSION/$ASSET_NAME"
$TARGET_PATH = Join-Path $HOOKS_DIR $BINARY_NAME

# Check if already installed with correct version
if ((Test-Path $TARGET_PATH) -and (Test-Path $VERSION_FILE)) {
    $INSTALLED_VERSION = (Get-Content $VERSION_FILE -Raw).Trim()
    if ($INSTALLED_VERSION -eq $VERSION) {
        Write-Host "claude_hooks $VERSION is already installed, skipping download"
        exit 0
    }
}

# Create hooks directory if it doesn't exist
if (-not (Test-Path $HOOKS_DIR)) {
    New-Item -ItemType Directory -Path $HOOKS_DIR -Force | Out-Null
}

# Download the binary
Write-Host "Downloading $ASSET_NAME $VERSION from $DOWNLOAD_URL..."
try {
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $TARGET_PATH -UseBasicParsing
} catch {
    Write-Error "Failed to download: $_"
    exit 1
}

# Save version file
$VERSION | Out-File -FilePath $VERSION_FILE -Encoding UTF8 -NoNewline

Write-Host "Successfully installed $BINARY_NAME $VERSION to $TARGET_PATH"
