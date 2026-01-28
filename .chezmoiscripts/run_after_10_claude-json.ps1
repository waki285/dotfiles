#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$File = Join-Path $env:USERPROFILE ".claude.json"

# context7 settings
$Context7ItemName = "context7-api-key"
$Context7KeyName = "CONTEXT7_API_KEY"
$Context7Url = "https://mcp.context7.com/mcp"

# searxng settings
$SearxngUrl = "http://127.0.0.1:8080"

$Context7ApiKey = bw get password $Context7ItemName
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
    url = $Context7Url
    headers = [PSCustomObject]@{
        $Context7KeyName = $Context7ApiKey
    }
}
$json.mcpServers | Add-Member -NotePropertyName "context7" -NotePropertyValue $context7Config -Force

# Set searxng MCP server
$searxngConfig = [PSCustomObject]@{
    type = "stdio"
    command = "npx"
    args = @("-y", "mcp-searxng")
    env = [PSCustomObject]@{
        SEARXNG_URL = $SearxngUrl
    }
}
$json.mcpServers | Add-Member -NotePropertyName "searxng" -NotePropertyValue $searxngConfig -Force

# Write back to file
$json | ConvertTo-Json -Depth 10 | Set-Content -Path $File -Encoding UTF8 -NoNewline

# Set file permissions (readable only by current user)
# Use icacls to avoid requiring SeSecurityPrivilege in some environments

# Disable inheritance and remove inherited ACEs
& icacls $File /inheritance:r | Out-Null

# Remove common principals if present (best-effort; ignore errors)
& icacls $File /remove "Users" "Authenticated Users" "Everyone" "BUILTIN\Users" "NT AUTHORITY\Authenticated Users" "NT AUTHORITY\Everyone" 2>$null | Out-Null

# Grant only the current user full control (replace existing explicit grants)
$me = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
& icacls $File /grant:r "${me}:(F)" | Out-Null