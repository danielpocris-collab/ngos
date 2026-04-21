param(
    [switch]$Release
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$PostHandoffScript = Join-Path $PSScriptRoot "prove-qemu-boot-refusal-post-handoff-corruption.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-boot-open-families-state.ps1"

foreach ($mode in @("too-many-memory-regions", "invalid-hhdm-offset")) {
    & $PostHandoffScript -Release:$Release -Mode $mode
}
& $VerifyScript

Write-Host "Boot open families proof completed."
