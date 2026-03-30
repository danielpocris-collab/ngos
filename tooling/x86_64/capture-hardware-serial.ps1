param(
    [Parameter(Mandatory = $true)]
    [string]$PortName,
    [string]$LogPath = "",
    [int]$BaudRate = 38400,
    [int]$DurationSeconds = 20,
    [int]$WaitForFirstByteSeconds = 0,
    [switch]$Append,
    [switch]$RequireData
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not ("System.IO.Ports.SerialPort" -as [type])) {
    throw "System.IO.Ports.SerialPort type is not available in this PowerShell runtime."
}

if ($DurationSeconds -le 0) {
    throw "DurationSeconds must be greater than zero."
}
if ($WaitForFirstByteSeconds -lt 0) {
    throw "WaitForFirstByteSeconds must be zero or greater."
}

$availablePorts = [System.IO.Ports.SerialPort]::GetPortNames() | Sort-Object
if ($availablePorts -notcontains $PortName) {
    throw "Serial port not found: $PortName. Available ports: $($availablePorts -join ', ')"
}

if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $LogPath = Join-Path $RepoRoot "target\hardware\serial-$($PortName.ToLowerInvariant()).log"
}

$logFull = [System.IO.Path]::GetFullPath($LogPath)
$logDir = Split-Path -Parent $logFull
if (!(Test-Path -LiteralPath $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

$serial = [System.IO.Ports.SerialPort]::new($PortName, $BaudRate, [System.IO.Ports.Parity]::None, 8, [System.IO.Ports.StopBits]::One)
$serial.ReadTimeout = 200
$serial.NewLine = "`n"
$serial.DtrEnable = $true
$serial.RtsEnable = $true

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$captureStopwatch = [System.Diagnostics.Stopwatch]::new()
$lines = New-Object System.Collections.Generic.List[string]
$bytesRead = 0
$firstByteObserved = $false

try {
    $serial.Open()
    $buffer = New-Object byte[] 1024
    while ($true) {
        if ($firstByteObserved) {
            if ($captureStopwatch.Elapsed.TotalSeconds -ge $DurationSeconds) {
                break
            }
        }
        elseif ($WaitForFirstByteSeconds -eq 0) {
            if ($stopwatch.Elapsed.TotalSeconds -ge $DurationSeconds) {
                break
            }
        }
        elseif ($stopwatch.Elapsed.TotalSeconds -ge $WaitForFirstByteSeconds) {
            break
        }

        try {
            $count = $serial.Read($buffer, 0, $buffer.Length)
            if ($count -gt 0) {
                if (-not $firstByteObserved) {
                    $firstByteObserved = $true
                    $captureStopwatch.Start()
                }
                $bytesRead += $count
                $text = [System.Text.Encoding]::ASCII.GetString($buffer, 0, $count)
                $lines.Add($text)
            }
        }
        catch [System.TimeoutException] {
        }
    }
}
finally {
    if ($serial.IsOpen) {
        $serial.Close()
    }
    $stopwatch.Stop()
    $captureStopwatch.Stop()
}

if ($RequireData -and $bytesRead -eq 0) {
    if ($WaitForFirstByteSeconds -gt 0) {
        throw "No serial data captured from $PortName within $WaitForFirstByteSeconds seconds."
    }
    throw "No serial data captured from $PortName in $DurationSeconds seconds."
}

$mode = if ($Append) { [System.IO.FileMode]::Append } else { [System.IO.FileMode]::Create }
$stream = [System.IO.File]::Open($logFull, $mode, [System.IO.FileAccess]::Write, [System.IO.FileShare]::Read)
try {
    $writer = New-Object System.IO.StreamWriter($stream, [System.Text.Encoding]::ASCII)
    try {
        foreach ($chunk in $lines) {
            $writer.Write($chunk)
        }
        $writer.Flush()
    }
    finally {
        $writer.Dispose()
    }
}
finally {
    $stream.Dispose()
}

Write-Host "Hardware serial capture complete."
Write-Host "Port: $PortName"
Write-Host "Bytes: $bytesRead"
if ($firstByteObserved) {
    Write-Host "Capture after first byte: $DurationSeconds seconds"
} elseif ($WaitForFirstByteSeconds -gt 0) {
    Write-Host "Waited for first byte: $WaitForFirstByteSeconds seconds"
}
Write-Host "Log:  $logFull"
