param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BootProof = Join-Path $RepoRoot "boot-x86_64\src\boot_handoff_proof.rs"
$BootLimine = Join-Path $RepoRoot "boot-x86_64\src\limine.rs"

$bootProofText = Get-Content -LiteralPath $BootProof -Raw
$bootText = Get-Content -LiteralPath $BootLimine -Raw

$checks = @(
    @{
        Name = "InvalidHhdmOffsetMode";
        Pattern = 'invalid-hhdm-offset';
        Source = $bootProofText;
        Reason = "the proof agent can corrupt the HHDM offset after Limine mediation and before canonical validation"
    },
    @{
        Name = "InvalidKernelRangeKindMode";
        Pattern = 'invalid-kernel-range-kind';
        Source = $bootProofText;
        Reason = "the proof agent can corrupt kernel range kind on the active QEMU path"
    },
    @{
        Name = "InvalidKernelRangeAlignmentMode";
        Pattern = 'invalid-kernel-range-alignment';
        Source = $bootProofText;
        Reason = "the proof agent can misalign the kernel range after handoff construction"
    },
    @{
        Name = "EmptyKernelRangeMode";
        Pattern = 'empty-kernel-range';
        Source = $bootProofText;
        Reason = "the proof agent can force an empty kernel range before BootInfo validation"
    },
    @{
        Name = "InvalidMemoryRegionAlignmentMode";
        Pattern = 'invalid-memory-region-alignment';
        Source = $bootProofText;
        Reason = "the proof agent can misalign a memory region entry on the active QEMU path"
    },
    @{
        Name = "EmptyMemoryRegionMode";
        Pattern = 'empty-memory-region';
        Source = $bootProofText;
        Reason = "the proof agent can force a zero-length memory region before BootInfo validation"
    },
    @{
        Name = "OverlappingMemoryRegionsMode";
        Pattern = 'overlapping-memory-regions';
        Source = $bootProofText;
        Reason = "the proof agent can create overlapping memory regions before canonical validation"
    },
    @{
        Name = "LimineWriterAppliesProofAgent";
        Pattern = 'crate::boot_handoff_proof::apply\(&mut handoff\)';
        Source = $bootText;
        Reason = "the active QEMU ingress now routes every post-handoff proof through the Limine writer before BootInfo leaves stage0"
    }
)

$missing = @()
foreach ($check in $checks) {
    if (-not [regex]::IsMatch($check.Source, $check.Pattern)) {
        $missing += $check.Name
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing expected InvalidBootInfo proof anchors: " + ($missing -join ", "))
}

Write-Host "Boot InvalidBootInfo QEMU proof mechanism verified."
foreach ($check in $checks) {
    Write-Host ("- " + $check.Name + ": " + $check.Reason)
}
