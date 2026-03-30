param(
    [string]$PortName = "COM1",
    [int[]]$BaudRates = @(38400, 115200, 57600, 19200, 9600),
    [int]$WaitPerBaudSeconds = 20,
    [int]$CaptureAfterFirstByteSeconds = 5,
    [string]$OutputDirectory = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($WaitPerBaudSeconds -le 0) {
    throw "WaitPerBaudSeconds must be greater than zero."
}
if ($CaptureAfterFirstByteSeconds -le 0) {
    throw "CaptureAfterFirstByteSeconds must be greater than zero."
}
if ($BaudRates.Count -eq 0) {
    throw "BaudRates must contain at least one baud rate."
}

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$CaptureScript = Join-Path $PSScriptRoot "capture-hardware-serial.ps1"

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $RepoRoot "target\hardware\serial-scan-$($PortName.ToLowerInvariant())"
}

$outputFull = [System.IO.Path]::GetFullPath($OutputDirectory)
if (!(Test-Path -LiteralPath $outputFull)) {
    New-Item -ItemType Directory -Path $outputFull | Out-Null
}

$results = New-Object System.Collections.Generic.List[object]

foreach ($baudRate in $BaudRates) {
    $logPath = Join-Path $outputFull ("baud-" + $baudRate + ".log")
    Write-Host "Probe serial port $PortName at baud $baudRate. Boot the target hardware now if needed."

    & $CaptureScript `
        -PortName $PortName `
        -BaudRate $baudRate `
        -DurationSeconds $CaptureAfterFirstByteSeconds `
        -WaitForFirstByteSeconds $WaitPerBaudSeconds `
        -LogPath $logPath

    $bytesRead = 0
    if (Test-Path -LiteralPath $logPath) {
        $bytesRead = (Get-Item -LiteralPath $logPath).Length
    }

    $result = [PSCustomObject]@{
        PortName = $PortName
        BaudRate = $baudRate
        BytesRead = $bytesRead
        LogPath = $logPath
        HasData = ($bytesRead -gt 0)
    }
    $results.Add($result) | Out-Null

    if ($bytesRead -gt 0) {
        Write-Host "Serial data detected on $PortName at baud $baudRate."
    }
}

$summaryPath = Join-Path $outputFull "summary.txt"
$summaryLines = New-Object System.Collections.Generic.List[string]
foreach ($result in $results) {
    $summaryLines.Add(("port={0} baud={1} bytes={2} log={3}" -f $result.PortName, $result.BaudRate, $result.BytesRead, $result.LogPath)) | Out-Null
}
[System.IO.File]::WriteAllLines($summaryPath, $summaryLines, [System.Text.Encoding]::ASCII)

$hits = @($results | Where-Object { $_.HasData })
if ($hits.Count -eq 0) {
    throw "No serial data detected on $PortName for baud rates: $($BaudRates -join ', '). Summary: $summaryPath"
}

Write-Host "Hardware serial scan complete."
foreach ($hit in $hits) {
    Write-Host ("DATA port={0} baud={1} bytes={2} log={3}" -f $hit.PortName, $hit.BaudRate, $hit.BytesRead, $hit.LogPath)
}
Write-Host "Summary: $summaryPath"
