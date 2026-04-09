param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath,
    [ValidateSet("missing-base-revision", "unsupported-base-revision")]
    [string]$Mode = "missing-base-revision"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Limine preboot base-revision rejection serial log not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$ansiPattern = [string]([char]27) + "\[[0-9;=]*[A-Za-z]"
$sanitized = [regex]::Replace($content, $ansiPattern, "")

$requiredPatterns = switch ($Mode) {
    "missing-base-revision" {
        @(
            'limine: Loading executable .*ngos-boot-x86_64',
            'PANIC: limine: Base revision 0 is no longer supported',
            'minimum: 4'
        )
    }
    "unsupported-base-revision" {
        @(
            'limine: Loading executable .*ngos-boot-x86_64',
            'PANIC: limine: Requested base revision .* is too new',
            'maximum supported: 6'
        )
    }
}

$missing = @()
foreach ($pattern in $requiredPatterns) {
    if (-not [regex]::IsMatch($sanitized, $pattern)) {
        $missing += $pattern
    }
}

if ($sanitized.Contains("ngos/x86_64: stage0 entered")) {
    throw "Log shows ngos stage0 entry; expected Limine-side base-revision rejection before ngos handoff."
}

if ($missing.Count -ne 0) {
    throw ("Missing Limine preboot base-revision rejection markers: " + ($missing -join " | "))
}

Write-Host "Limine preboot base-revision rejection markers verified."
Write-Host "Mode: $Mode"
Write-Host "Log: $LogPath"
