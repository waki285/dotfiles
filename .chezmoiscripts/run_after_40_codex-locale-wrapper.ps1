#Requires -Version 5.1
$ErrorActionPreference = "Stop"

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Content
    )

    $dir = Split-Path -Parent $Path
    if ($dir -and -not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }

    [System.IO.File]::WriteAllText($Path, $Content, [System.Text.UTF8Encoding]::new($false))
}

function Set-CodexWrapperBlock {
    param([Parameter(Mandatory = $true)][string]$Path)

    $content = if (Test-Path $Path) { Get-Content $Path -Raw } else { "" }
    $newline = if ($content -match "`r`n") { "`r`n" } else { "`r`n" }
    $pattern = '(?ms)^# >>> chezmoi codex locale wrapper >>>\r?\n.*?^# <<< chezmoi codex locale wrapper <<<\r?\n?'

    $block = @'
# >>> chezmoi codex locale wrapper >>>
# Codex injects C.UTF-8 into Windows PowerShell sessions; GNU sed rejects that locale.
function global:sed {
    $sedCommand = Get-Command sed.exe -All |
        Where-Object CommandType -eq 'Application' |
        Select-Object -First 1

    if (-not $sedCommand) {
        throw 'sed executable not found'
    }

    $restore = [ordered]@{}

    if ($env:LANG -eq 'C.UTF-8') {
        $restore['LANG'] = $env:LANG
        $env:LANG = 'C'
    }

    Get-ChildItem Env: |
        Where-Object { $_.Name -like 'LC_*' -and $_.Value -eq 'C.UTF-8' } |
        ForEach-Object {
            $restore[$_.Name] = $_.Value
            Set-Item -Path ("Env:" + $_.Name) -Value 'C'
        }

    try {
        if ($MyInvocation.ExpectingInput) {
            $input | & $sedCommand.Source @args
        } else {
            & $sedCommand.Source @args
        }
    } finally {
        foreach ($entry in $restore.GetEnumerator()) {
            Set-Item -Path ("Env:" + $entry.Key) -Value $entry.Value
        }
    }
}
# <<< chezmoi codex locale wrapper <<<
'@

    $normalizedBlock = ($block.TrimEnd("`r", "`n") -replace "`r?`n", $newline) + $newline
    $updated = [regex]::Replace($content, $pattern, "")
    $updated = $updated.TrimEnd("`r", "`n")

    if ($updated.Length -gt 0) {
        $updated += $newline + $newline
    }

    $updated += $normalizedBlock
    Write-Utf8NoBom -Path $Path -Content $updated
}

Set-CodexWrapperBlock -Path $PROFILE
