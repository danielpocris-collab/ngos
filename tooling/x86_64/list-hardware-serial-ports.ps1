Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not ("System.IO.Ports.SerialPort" -as [type])) {
    throw "System.IO.Ports.SerialPort type is not available in this PowerShell runtime."
}

$ports = @([System.IO.Ports.SerialPort]::GetPortNames() | Sort-Object)
if ($ports.Length -eq 0) {
    Write-Host "No serial ports available."
    exit 0
}

foreach ($port in $ports) {
    Write-Host $port
}
