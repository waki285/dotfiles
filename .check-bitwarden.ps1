#Requires -Version 5.1
$ErrorActionPreference = "Stop"

# Check if bw command exists
$bwPath = Get-Command bw -ErrorAction SilentlyContinue
if (-not $bwPath) {
    Write-Error "Bitwarden CLI (bw) is not installed. Install it with: winget install Bitwarden.CLI"
    exit 1
}

# Check if logged in and unlocked
try {
    $statusJson = bw status 2>$null
    $status = ($statusJson | ConvertFrom-Json).status
} catch {
    Write-Error "Could not determine Bitwarden status."
    exit 1
}

switch ($status) {
    "unlocked" {
        exit 0
    }
    "locked" {
        Write-Error "Bitwarden is locked. Run: bw unlock"
        exit 1
    }
    "unauthenticated" {
        Write-Error "Bitwarden is not logged in. Run: bw login"
        exit 1
    }
    default {
        Write-Error "Unknown Bitwarden status: $status"
        exit 1
    }
}
