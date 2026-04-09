param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Limine preboot executable-path rejection serial log not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$ansiPattern = [string]([char]27) + "\[[0-9;=]*[A-Za-z]"
$sanitized = [regex]::Replace($content, $ansiPattern, "")
$requiredPatterns = @(
    'limine: Loading executable .*does-not-exist',
    'PANIC: limine: Failed to open executable with path',
    'boot\(\):/kernel/does-not-exist',
    'Is the path correct\?'
)

$missing = @()
foreach ($pattern in $requiredPatterns) {
    if (-not [regex]::IsMatch($sanitized, $pattern)) {
        $missing += $pattern
    }
}

if ($sanitized.Contains("ngos/x86_64: stage0 entered")) {
    throw "Log shows ngos stage0 entry; expected Limine-side executable rejection before ngos handoff."
}

if ($missing.Count -ne 0) {
    throw ("Missing Limine preboot executable-path rejection markers: " + ($missing -join " | "))
}

Write-Host "Limine preboot executable-path rejection markers verified."
Write-Host "Log: $LogPath"
