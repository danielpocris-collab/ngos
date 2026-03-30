param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("wasmtime", "wasm-tools")]
    [string]$Tool,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Arguments
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

switch ($Tool) {
    "wasmtime" {
        $exe = Join-Path $scriptRoot "wasmtime-v43.0.0-x86_64-windows\\wasmtime.exe"
    }
    "wasm-tools" {
        $exe = Join-Path $scriptRoot "wasm-tools-v1.245.1\\wasm-tools-1.245.1-x86_64-windows\\wasm-tools.exe"
    }
}

if (-not (Test-Path $exe)) {
    throw "Missing local Wasm tool: $exe"
}

& $exe @Arguments
$exitCode = $LASTEXITCODE
if ($exitCode -ne 0) {
    exit $exitCode
}
