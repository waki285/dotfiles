#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$File = Join-Path $env:USERPROFILE ".claude.json"
$ItemName = "context7-api-key"
$KeyName = "CONTEXT7_API_KEY"
$Url = "https://mcp.context7.com/mcp"

$ApiKey = bw get password $ItemName
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to get password from Bitwarden"
    exit 1
}

if (-not (Test-Path $File)) {
    Set-Content -Path $File -Value "{}" -Encoding UTF8
}

$content = Get-Content -Path $File -Raw -Encoding UTF8
try {
    $json = $content | ConvertFrom-Json
} catch {
    Write-Error "Error: $File is not valid JSON. Fix it first."
    exit 1
}

# Ensure mcpServers exists
if (-not $json.PSObject.Properties["mcpServers"]) {
    $json | Add-Member -NotePropertyName "mcpServers" -NotePropertyValue ([PSCustomObject]@{})
}

# Set hasCompletedOnboarding
if ($json.PSObject.Properties["hasCompletedOnboarding"]) {
    $json.hasCompletedOnboarding = $true
} else {
    $json | Add-Member -NotePropertyName "hasCompletedOnboarding" -NotePropertyValue $true
}

# Set context7 MCP server
$context7Config = [PSCustomObject]@{
    type = "http"
    url = $Url
    headers = [PSCustomObject]@{
        $KeyName = $ApiKey
    }
}
$json.mcpServers | Add-Member -NotePropertyName "context7" -NotePropertyValue $context7Config -Force

# Write back to file
$json | ConvertTo-Json -Depth 10 | Set-Content -Path $File -Encoding UTF8 -NoNewline

# Set file permissions (readable only by current user)
$acl = Get-Acl $File
$acl.SetAccessRuleProtection($true, $false)
$rule = New-Object System.Security.AccessControl.FileSystemAccessRule(
    [System.Security.Principal.WindowsIdentity]::GetCurrent().Name,
    "FullControl",
    "Allow"
)
$acl.SetAccessRule($rule)
Set-Acl -Path $File -AclObject $acl
