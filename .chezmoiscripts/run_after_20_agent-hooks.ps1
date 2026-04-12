# Download agent_hooks and claude_statusline binaries from GitHub releases

$ErrorActionPreference = "Stop"

$REPO = "waki285/dotfiles-tools"
$HOOKS_DIR = Join-Path $env:USERPROFILE ".claude\hooks"
$OPENCODE_PLUGIN_DIR = Join-Path $env:USERPROFILE ".config\opencode\plugin"
$AGENT_HOOKS_BINARY_NAME = "agent_hooks.exe"
$STATUSLINE_BINARY_NAME = "claude_statusline.exe"
$AGENT_HOOKS_VERSION_FILE = Join-Path $HOOKS_DIR ".agent_hooks_version"
$STATUSLINE_VERSION_FILE = Join-Path $HOOKS_DIR ".claude_statusline_version"

function Get-LatestVersion {
    param(
        [Parameter(Mandatory = $true)][string]$Prefix,
        [Parameter(Mandatory = $true)]$Releases
    )

    $escapedPrefix = [regex]::Escape($Prefix)
    foreach ($release in $releases) {
        if ($release.tag_name -match "^${escapedPrefix}-(v.+)$") {
            return $Matches[1]
        }
    }
    return $null
}

function Download-Asset {
    param(
        [Parameter(Mandatory = $true)][string]$Url,
        [Parameter(Mandatory = $true)][string]$OutFile,
        [Parameter(Mandatory = $true)][string]$Label
    )

    Write-Host "Downloading $Label from $Url..."
    try {
        Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing
    } catch {
        Write-Error "Failed to download $Label`: $_"
        exit 1
    }
}

$releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases" -UseBasicParsing
$AGENT_HOOKS_VERSION = Get-LatestVersion -Prefix "agent_hooks" -Releases $releases
$STATUSLINE_VERSION = Get-LatestVersion -Prefix "claude_statusline" -Releases $releases

if (-not $AGENT_HOOKS_VERSION) {
    Write-Error "Could not determine latest agent_hooks version"
    exit 1
}
if (-not $STATUSLINE_VERSION) {
    Write-Error "Could not determine latest claude_statusline version"
    exit 1
}

$ARCH = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

switch ($ARCH) {
    "X64" {
        $AGENT_HOOKS_ASSET = "agent_hooks-windows-x86_64.exe"
        $OPENCODE_ASSET = "agent_hooks_opencode-windows-x86_64.node"
        $STATUSLINE_ASSET = "claude_statusline-windows-x86_64.exe"
    }
    "Arm64" {
        $AGENT_HOOKS_ASSET = "agent_hooks-windows-arm64.exe"
        $OPENCODE_ASSET = "agent_hooks_opencode-windows-arm64.node"
        $STATUSLINE_ASSET = "claude_statusline-windows-arm64.exe"
    }
    default {
        Write-Error "Unsupported architecture: $ARCH"
        exit 1
    }
}

$AGENT_HOOKS_DOWNLOAD_URL = "https://github.com/$REPO/releases/download/agent_hooks-$AGENT_HOOKS_VERSION/$AGENT_HOOKS_ASSET"
$OPENCODE_DOWNLOAD_URL = "https://github.com/$REPO/releases/download/agent_hooks-$AGENT_HOOKS_VERSION/$OPENCODE_ASSET"
$STATUSLINE_DOWNLOAD_URL = "https://github.com/$REPO/releases/download/claude_statusline-$STATUSLINE_VERSION/$STATUSLINE_ASSET"

$AGENT_HOOKS_TARGET_PATH = Join-Path $HOOKS_DIR $AGENT_HOOKS_BINARY_NAME
$STATUSLINE_TARGET_PATH = Join-Path $HOOKS_DIR $STATUSLINE_BINARY_NAME
$OPENCODE_TARGET_PATH = Join-Path $OPENCODE_PLUGIN_DIR "agent_hooks.node"

if (-not (Test-Path $HOOKS_DIR)) {
    New-Item -ItemType Directory -Path $HOOKS_DIR -Force | Out-Null
}
if (-not (Test-Path $OPENCODE_PLUGIN_DIR)) {
    New-Item -ItemType Directory -Path $OPENCODE_PLUGIN_DIR -Force | Out-Null
}

$needAgentHooksDownload = $true
if ((Test-Path $AGENT_HOOKS_TARGET_PATH) -and (Test-Path $OPENCODE_TARGET_PATH) -and (Test-Path $AGENT_HOOKS_VERSION_FILE)) {
    $installedAgentHooksVersion = (Get-Content $AGENT_HOOKS_VERSION_FILE -Raw).Trim()
    if ($installedAgentHooksVersion -eq $AGENT_HOOKS_VERSION) {
        $needAgentHooksDownload = $false
    }
}

$needStatuslineDownload = $true
if ((Test-Path $STATUSLINE_TARGET_PATH) -and (Test-Path $STATUSLINE_VERSION_FILE)) {
    $installedStatuslineVersion = (Get-Content $STATUSLINE_VERSION_FILE -Raw).Trim()
    if ($installedStatuslineVersion -eq $STATUSLINE_VERSION) {
        $needStatuslineDownload = $false
    }
}

if (-not $needAgentHooksDownload -and -not $needStatuslineDownload) {
    Write-Host "agent_hooks $AGENT_HOOKS_VERSION and claude_statusline $STATUSLINE_VERSION are already installed, skipping download"
    exit 0
}

if ($needAgentHooksDownload) {
    Download-Asset -Url $AGENT_HOOKS_DOWNLOAD_URL -OutFile $AGENT_HOOKS_TARGET_PATH -Label "$AGENT_HOOKS_ASSET $AGENT_HOOKS_VERSION"
    Write-Host "Successfully installed $AGENT_HOOKS_BINARY_NAME $AGENT_HOOKS_VERSION to $AGENT_HOOKS_TARGET_PATH"

    Download-Asset -Url $OPENCODE_DOWNLOAD_URL -OutFile $OPENCODE_TARGET_PATH -Label "$OPENCODE_ASSET $AGENT_HOOKS_VERSION"
    Write-Host "Successfully installed agent_hooks.node $AGENT_HOOKS_VERSION to $OPENCODE_TARGET_PATH"

    $AGENT_HOOKS_VERSION | Out-File -FilePath $AGENT_HOOKS_VERSION_FILE -Encoding UTF8 -NoNewline
}

if ($needStatuslineDownload) {
    Download-Asset -Url $STATUSLINE_DOWNLOAD_URL -OutFile $STATUSLINE_TARGET_PATH -Label "$STATUSLINE_ASSET $STATUSLINE_VERSION"
    Write-Host "Successfully installed $STATUSLINE_BINARY_NAME $STATUSLINE_VERSION to $STATUSLINE_TARGET_PATH"

    $STATUSLINE_VERSION | Out-File -FilePath $STATUSLINE_VERSION_FILE -Encoding UTF8 -NoNewline
}

Write-Host "Installation complete: agent_hooks=$AGENT_HOOKS_VERSION, claude_statusline=$STATUSLINE_VERSION"
