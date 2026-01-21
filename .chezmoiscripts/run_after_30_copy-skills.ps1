#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$ChezmoiSourceDir = if ($env:CHEZMOI_SOURCE_DIR) {
    $env:CHEZMOI_SOURCE_DIR
} else {
    chezmoi source-path
}

$SkillsSrc = Join-Path $ChezmoiSourceDir ".chezmoitemplates\skills"

if (-not (Test-Path $SkillsSrc -PathType Container)) {
    Write-Error "Skills source directory not found: $SkillsSrc"
    exit 1
}

$Destinations = @(
    Join-Path $env:USERPROFILE ".claude\skills"
    Join-Path $env:USERPROFILE ".codex\skills"
    Join-Path $env:USERPROFILE ".config\opencode\skills"
)

foreach ($dest in $Destinations) {
    if (-not (Test-Path $dest)) {
        New-Item -ItemType Directory -Path $dest -Force | Out-Null
    }
    Copy-Item -Path "$SkillsSrc\*" -Destination $dest -Recurse -Force
}
