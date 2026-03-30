param(
    [string]$PortName = "COM1",
    [int]$BaudRate = 38400
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not ("System.IO.Ports.SerialPort" -as [type])) {
    throw "System.IO.Ports.SerialPort type is not available in this PowerShell runtime."
}

$availablePorts = @([System.IO.Ports.SerialPort]::GetPortNames() | Sort-Object)
if ($availablePorts -notcontains $PortName) {
    throw "Serial port not found: $PortName. Available ports: $($availablePorts -join ', ')"
}

$serial = [System.IO.Ports.SerialPort]::new($PortName, $BaudRate, [System.IO.Ports.Parity]::None, 8, [System.IO.Ports.StopBits]::One)
$serial.ReadTimeout = 200
$serial.DtrEnable = $true
$serial.RtsEnable = $true
$serial.Handshake = [System.IO.Ports.Handshake]::None

try {
    $serial.Open()
    Start-Sleep -Milliseconds 200

    Write-Host "Hardware serial port inspection complete."
    Write-Host "Port: $PortName"
    Write-Host "BaudRate: $BaudRate"
    Write-Host "IsOpen: $($serial.IsOpen)"
    Write-Host "CtsHolding: $($serial.CtsHolding)"
    Write-Host "DsrHolding: $($serial.DsrHolding)"
    Write-Host "CDHolding: $($serial.CDHolding)"
    Write-Host "BytesToRead: $($serial.BytesToRead)"
    Write-Host "BytesToWrite: $($serial.BytesToWrite)"
}
finally {
    if ($serial.IsOpen) {
        $serial.Close()
    }
}
