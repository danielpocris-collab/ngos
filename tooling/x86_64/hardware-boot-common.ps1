Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

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

function Ensure-Directory {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (!(Test-Path -LiteralPath $Path)) {
        [System.IO.Directory]::CreateDirectory($Path) | Out-Null
    }
}

function Test-IsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
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

function Invoke-ScriptElevated {
    param(
        [Parameter(Mandatory = $true)][string]$ScriptPath,
        [Parameter(Mandatory = $true)][string[]]$ArgumentList
    )

    $process = Start-Process `
        -FilePath "powershell.exe" `
        -Verb RunAs `
        -ArgumentList $ArgumentList `
        -Wait `
        -PassThru

    if ($process.ExitCode -ne 0) {
        throw "Elevated script failed with exit code $($process.ExitCode): $ScriptPath"
    }
}
