param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BootMain = Join-Path $RepoRoot "boot-x86_64\src\main.rs"
$BootLimine = Join-Path $RepoRoot "boot-x86_64\src\limine.rs"
$BootProof = Join-Path $RepoRoot "boot-x86_64\src\boot_handoff_proof.rs"
$PlatformLib = Join-Path $RepoRoot "platform-x86_64\src\lib.rs"
$PlatformLimine = Join-Path $RepoRoot "platform-x86_64\src\limine.rs"

$bootMainText = Get-Content -LiteralPath $BootMain -Raw
$bootLimineText = Get-Content -LiteralPath $BootLimine -Raw
$bootProofText = Get-Content -LiteralPath $BootProof -Raw
$platformText = Get-Content -LiteralPath $PlatformLib -Raw
$platformLimineText = Get-Content -LiteralPath $PlatformLimine -Raw

$checks = @(
    @{
        Name = "Stage0CallsLimineWriteBootInfoDirectly";
        Pattern = 'crate::limine::write_boot_info\(boot_info\.as_mut_ptr\(\), kernel_image_len\)';
        Source = $bootMainText;
        Reason = "stage0 builds boot state by calling the Limine writer directly"
    },
    @{
        Name = "Stage0ConsumesBootInfoAfterLimineWriter";
        Pattern = 'initialize_early_boot_state\(unsafe \{ boot_info\.assume_init\(\) \}, kernel_image_len\)';
        Source = $bootMainText;
        Reason = "stage0 still consumes the mediated BootInfo directly after the Limine writer returns"
    },
    @{
        Name = "LimineWriterBuildsLoaderDefinedSnapshot";
        Pattern = 'LimineBootSnapshot\s*\{';
        Source = $bootLimineText;
        Reason = "the active QEMU ingress now materializes a LoaderDefinedBootHandoff snapshot inside the Limine writer"
    },
    @{
        Name = "LimineWriterUsesPlatformLimineBuffers";
        Pattern = 'build_loader_defined_handoff\(snapshot, kernel_image_len\)';
        Source = $bootLimineText;
        Reason = "the active QEMU path now routes through platform-owned LoaderDefinedBootHandoff mediation"
    },
    @{
        Name = "LimineWriterAppliesBootHandoffProofMutation";
        Pattern = 'crate::boot_handoff_proof::apply\(&mut handoff\)';
        Source = $bootLimineText;
        Reason = "the Limine writer exposes a repo-owned post-handoff corruption hook before canonical validation"
    },
    @{
        Name = "BootHandoffProofParsesCommandLineSelector";
        Pattern = 'ngos\.boot\.handoff_corrupt=';
        Source = $bootProofText;
        Reason = "the new proof agent is activated from the real Limine command line on the QEMU path"
    },
    @{
        Name = "BootHandoffProofCanInflateMemoryRegionsBeyondBootCapacity";
        Pattern = 'MAX_PROOF_MEMORY_REGIONS: usize = 257';
        Source = $bootProofText;
        Reason = "the proof agent owns extra region storage so the real QEMU handoff can exceed the boot contract capacity"
    },
    @{
        Name = "LoaderDefinedHandoffValidationRemainsCanonical";
        Pattern = 'pub fn as_boot_info\(&self\) -> Result<BootInfo';
        Source = $platformText;
        Reason = "canonical BootInfo validation still flows through LoaderDefinedBootHandoff::as_boot_info"
    },
    @{
        Name = "PlatformLimineStillOwnsInitialHandoffMediation";
        Pattern = 'pub fn build_loader_defined_handoff';
        Source = $platformLimineText;
        Reason = "platform-x86_64 remains the owner of the raw Limine-to-handoff mediation"
    }
)

$missing = @()
foreach ($check in $checks) {
    $matched = [regex]::IsMatch($check.Source, $check.Pattern)
    if ($check.ContainsKey("ExpectAbsent")) {
        if ($matched) {
            $missing += $check.Name
        }
    } elseif (-not $matched) {
        $missing += $check.Name
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing expected post-handoff corruption surface anchors: " + ($missing -join ", "))
}

Write-Host "QEMU boot post-handoff corruption surface verified."
foreach ($check in $checks) {
    Write-Host ("- " + $check.Name + ": " + $check.Reason)
}
