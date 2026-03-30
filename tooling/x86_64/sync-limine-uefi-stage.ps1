param(
    [Parameter(Mandatory = $true)]
    [string]$StagePath,
    [Parameter(Mandatory = $true)]
    [string]$EspRoot,
    [switch]$Prune
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-NormalizedFullPath {
    param([Parameter(Mandatory = $true)][string]$Path)
    if ($Path.StartsWith("\\?\")) {
        return $Path.TrimEnd('\') + '\'
    }
    return [System.IO.Path]::GetFullPath($Path)
}

function Join-NativePath {
    param(
        [Parameter(Mandatory = $true)][string]$Base,
        [Parameter(Mandatory = $true)][string]$Relative
    )

    $trimmedBase = $Base.TrimEnd('\')
    $trimmedRelative = ($Relative -replace '/', '\').TrimStart('\')
    if ([string]::IsNullOrWhiteSpace($trimmedRelative)) {
        return $trimmedBase + '\'
    }
    return $trimmedBase + '\' + $trimmedRelative
}

function Get-RelativeStageEntries {
    param([Parameter(Mandatory = $true)][string]$Root)

    $rootFull = Get-NormalizedFullPath -Path $Root
    $prefix = $rootFull.TrimEnd('\') + '\'
    $items = Get-ChildItem -LiteralPath $rootFull -Recurse -Force
    foreach ($item in $items) {
        $full = Get-NormalizedFullPath -Path $item.FullName
        $relative = $full.Substring($prefix.Length).Replace('\', '/')
        [PSCustomObject]@{
            FullName = $full
            Relative = $relative
            IsDirectory = $item.PSIsContainer
        }
    }
}

function Get-TopLevelRelativeComponent {
    param([Parameter(Mandatory = $true)][string]$Relative)

    $normalized = ($Relative -replace '\\', '/').Trim('/')
    if ([string]::IsNullOrWhiteSpace($normalized)) {
        return ""
    }
    $separatorIndex = $normalized.IndexOf('/')
    if ($separatorIndex -lt 0) {
        return $normalized
    }
    return $normalized.Substring(0, $separatorIndex)
}

function Get-PruneAnchors {
    param([Parameter(Mandatory = $true)][string]$StageRoot)

    $anchors = New-Object System.Collections.Generic.List[string]
    $rootEntries = @(Get-ChildItem -LiteralPath $StageRoot -Force)
    foreach ($entry in $rootEntries) {
        if (-not $entry.PSIsContainer) {
            $anchors.Add($entry.Name.Replace('\', '/'))
            continue
        }

        if ($entry.Name -ieq "EFI") {
            $efiChildren = @(Get-ChildItem -LiteralPath $entry.FullName -Force)
            foreach ($efiChild in $efiChildren) {
                $anchors.Add(("EFI/" + $efiChild.Name).Replace('\', '/'))
            }
            continue
        }

        $anchors.Add($entry.Name.Replace('\', '/'))
    }

    return $anchors
}

function Ensure-Directory {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (!(Test-Path -LiteralPath $Path)) {
        [System.IO.Directory]::CreateDirectory($Path) | Out-Null
    }
}

function Test-EspWritable {
    param([Parameter(Mandatory = $true)][string]$Root)

    $probeDir = Join-NativePath -Base $Root -Relative "EFI\BOOT"
    $probeFile = Join-NativePath -Base $probeDir -Relative ".ngos-write-probe.tmp"
    try {
        Ensure-Directory -Path $probeDir
        [System.IO.File]::WriteAllText($probeFile, "ngos-write-probe")
        return $true
    }
    catch [System.UnauthorizedAccessException] {
        return $false
    }
    finally {
        if (Test-Path -LiteralPath $probeFile) {
            Remove-Item -LiteralPath $probeFile -Force -ErrorAction SilentlyContinue
        }
    }
}

function Get-FileHashHex {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

$stageFull = Get-NormalizedFullPath -Path $StagePath
$espFull = Get-NormalizedFullPath -Path $EspRoot

if (!(Test-Path -LiteralPath $stageFull)) {
    throw "Stage directory not found: $stageFull"
}
if (!(Test-Path -LiteralPath $espFull)) {
    throw "ESP root not found: $espFull"
}
if (-not (Test-EspWritable -Root $espFull)) {
    throw "ESP root is not writable from this PowerShell session: $espFull"
}

$stageEntries = @(Get-RelativeStageEntries -Root $stageFull)

foreach ($entry in $stageEntries | Where-Object { $_.IsDirectory }) {
    Ensure-Directory -Path (Join-NativePath -Base $espFull -Relative $entry.Relative)
}

foreach ($entry in $stageEntries | Where-Object { -not $_.IsDirectory }) {
    $targetPath = Join-NativePath -Base $espFull -Relative $entry.Relative
    $targetDir = Split-Path -Parent $targetPath
    Ensure-Directory -Path $targetDir
    Copy-Item -LiteralPath $entry.FullName -Destination $targetPath -Force
}

if ($Prune) {
    $stageRelativeSet = @{}
    foreach ($entry in $stageEntries) {
        $stageRelativeSet[$entry.Relative] = $true
    }

    $pruneAnchors = @(Get-PruneAnchors -StageRoot $stageFull)
    foreach ($anchor in $pruneAnchors) {
        $anchorPath = Join-NativePath -Base $espFull -Relative $anchor
        if (!(Test-Path -LiteralPath $anchorPath)) {
            continue
        }

        $anchorItem = Get-Item -LiteralPath $anchorPath
        if (-not $anchorItem.PSIsContainer) {
            if (-not $stageRelativeSet.ContainsKey($anchor)) {
                Remove-Item -LiteralPath $anchorPath -Force
            }
            continue
        }

        $ownedEntries = @(Get-RelativeStageEntries -Root $anchorPath | Sort-Object Relative -Descending)
        foreach ($entry in $ownedEntries) {
            if ([string]::IsNullOrWhiteSpace($entry.Relative)) {
                continue
            }

            $relativeFromEsp = ($anchor + "/" + $entry.Relative.Replace('\', '/')).Trim('/')
            if (-not $stageRelativeSet.ContainsKey($relativeFromEsp)) {
                Remove-Item -LiteralPath $entry.FullName -Force -Recurse:$entry.IsDirectory
            }
        }

        if (-not $stageRelativeSet.ContainsKey($anchor)) {
            $remainingChildren = @(Get-ChildItem -LiteralPath $anchorPath -Force)
            if ($remainingChildren.Count -eq 0) {
                Remove-Item -LiteralPath $anchorPath -Force
            }
        }
    }
}

foreach ($entry in $stageEntries | Where-Object { -not $_.IsDirectory }) {
    $targetPath = Join-NativePath -Base $espFull -Relative $entry.Relative
    if (!(Test-Path -LiteralPath $targetPath)) {
        throw "Missing file after sync: $($entry.Relative)"
    }
    $sourceHash = Get-FileHashHex -Path $entry.FullName
    $targetHash = Get-FileHashHex -Path $targetPath
    if ($sourceHash -ne $targetHash) {
        throw "File mismatch after sync: $($entry.Relative) source=$sourceHash target=$targetHash"
    }
}

Write-Host "Stage synchronized successfully."
Write-Host "Stage: $stageFull"
Write-Host "ESP:   $espFull"
