# Download agent_hooks binaries from GitHub releases
# Version is fetched from the latest release (source of truth: Cargo.toml)

$ErrorActionPreference = "Stop"

$REPO = "waki285/dotfiles-tools"
$HOOKS_DIR = Join-Path $env:USERPROFILE ".claude\hooks"
$OPENCODE_PLUGIN_DIR = Join-Path $env:USERPROFILE ".config\opencode\plugin"
$BINARY_NAME = "agent_hooks_claude.exe"
$VERSION_FILE = Join-Path $HOOKS_DIR ".agent_hooks_version"

# Get latest version from GitHub API
function Get-LatestVersion {
    $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases" -UseBasicParsing
    foreach ($release in $releases) {
        if ($release.tag_name -match "^agent_hooks-(v.+)$") {
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
        $CLAUDE_ASSET = "agent_hooks_claude-windows-x86_64.exe"
        $OPENCODE_ASSET = "agent_hooks_opencode-windows-x86_64.node"
    }
    "Arm64" {
        $CLAUDE_ASSET = "agent_hooks_claude-windows-arm64.exe"
        $OPENCODE_ASSET = "agent_hooks_opencode-windows-arm64.node"
    }
    default {
        Write-Error "Unsupported architecture: $ARCH"
        exit 1
    }
}

$CLAUDE_DOWNLOAD_URL = "https://github.com/$REPO/releases/download/agent_hooks-$VERSION/$CLAUDE_ASSET"
$OPENCODE_DOWNLOAD_URL = "https://github.com/$REPO/releases/download/agent_hooks-$VERSION/$OPENCODE_ASSET"
$CLAUDE_TARGET_PATH = Join-Path $HOOKS_DIR $BINARY_NAME
$OPENCODE_TARGET_PATH = Join-Path $OPENCODE_PLUGIN_DIR "agent_hooks.node"

# Check if already installed with correct version
if ((Test-Path $CLAUDE_TARGET_PATH) -and (Test-Path $VERSION_FILE)) {
    $INSTALLED_VERSION = (Get-Content $VERSION_FILE -Raw).Trim()
    if ($INSTALLED_VERSION -eq $VERSION) {
        Write-Host "agent_hooks $VERSION is already installed, skipping download"
        exit 0
    }
}

# Create directories if they don't exist
if (-not (Test-Path $HOOKS_DIR)) {
    New-Item -ItemType Directory -Path $HOOKS_DIR -Force | Out-Null
}
if (-not (Test-Path $OPENCODE_PLUGIN_DIR)) {
    New-Item -ItemType Directory -Path $OPENCODE_PLUGIN_DIR -Force | Out-Null
}

# Download the Claude CLI binary
Write-Host "Downloading $CLAUDE_ASSET $VERSION from $CLAUDE_DOWNLOAD_URL..."
try {
    Invoke-WebRequest -Uri $CLAUDE_DOWNLOAD_URL -OutFile $CLAUDE_TARGET_PATH -UseBasicParsing
} catch {
    Write-Error "Failed to download Claude CLI: $_"
    exit 1
}

Write-Host "Successfully installed $BINARY_NAME $VERSION to $CLAUDE_TARGET_PATH"

# Download the OpenCode NAPI binary
Write-Host "Downloading $OPENCODE_ASSET $VERSION from $OPENCODE_DOWNLOAD_URL..."
try {
    Invoke-WebRequest -Uri $OPENCODE_DOWNLOAD_URL -OutFile $OPENCODE_TARGET_PATH -UseBasicParsing
} catch {
    Write-Error "Failed to download OpenCode NAPI: $_"
    exit 1
}

Write-Host "Successfully installed agent_hooks.node $VERSION to $OPENCODE_TARGET_PATH"

# Save version file
$VERSION | Out-File -FilePath $VERSION_FILE -Encoding UTF8 -NoNewline

Write-Host "agent_hooks $VERSION installation complete"
