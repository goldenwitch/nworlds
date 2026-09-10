param(
    [Parameter(Mandatory = $true)]
    [string]$ManifestPath,

    [Parameter(Mandatory = $true)]
    [string]$BinaryName
)

$ErrorActionPreference = "Stop"

function Write-Stderr([object]$Value) {
    [Console]::Error.WriteLine([string]$Value)
}

$manifest = (Resolve-Path $ManifestPath).Path
$cargo = (Get-Command cargo).Source

try {
    $metadata = (& $cargo metadata --no-deps --format-version 1 --manifest-path $manifest --offline | Out-String | ConvertFrom-Json)
    $targetDirectory = $metadata.target_directory
    $buildArguments = @(
        "build",
        "--quiet",
        "--locked",
        "--offline",
        "--manifest-path", $manifest,
        "--bin", $BinaryName
    )
    $buildOutput = & $cargo @buildArguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        $buildOutput | ForEach-Object { Write-Stderr $_ }
        exit $LASTEXITCODE
    }

    $builtBinary = Join-Path $targetDirectory "debug\$BinaryName.exe"
    if (-not (Test-Path $builtBinary)) {
        throw "Cargo did not produce the expected MCP binary: $builtBinary"
    }

    $runtimeDirectory = Join-Path ([IO.Path]::GetTempPath()) "caravanofseasons-mcp"
    New-Item -ItemType Directory -Force -Path $runtimeDirectory | Out-Null
    Get-ChildItem -Path $runtimeDirectory -Filter "$BinaryName-*.exe" -File -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue

    $runtimeBinary = Join-Path $runtimeDirectory "$BinaryName-$([Guid]::NewGuid()).exe"
    Copy-Item $builtBinary $runtimeBinary

    try {
        & $runtimeBinary
        exit $LASTEXITCODE
    }
    finally {
        Remove-Item $runtimeBinary -Force -ErrorAction SilentlyContinue
    }
}
catch {
    Write-Stderr $_
    exit 1
}