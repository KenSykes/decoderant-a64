<#
.SYNOPSIS
Measures what the formatter adds to a binary.

.DESCRIPTION
Builds examples/size_probe twice with the `size` profile (opt-level z, LTO, abort,
stripped): once without the `formatter` feature and once with it. The difference
between the two executables is the formatter's cost, including the code and tables
it pulls in from the rest of the crate. Pass -Features to add other features to
both builds, and -Symbols to keep symbol information (the .pdb on Windows).

Output: target/size/examples/{without,with}-formatter/size_probe[.exe].

Works in Windows PowerShell 5.1 and PowerShell 7 (pwsh, any OS).

.EXAMPLE
.\scripts\formatter-size.ps1
#>
param(
    [string]$Features = '',
    [switch]$Symbols
)

Set-Location (Split-Path -Parent $PSScriptRoot)
# Keeps the symbol table (a .pdb next to the exe on MSVC); the sizes below are then
# of the unstripped binaries.
if ($Symbols) { $env:CARGO_PROFILE_SIZE_STRIP = 'none' }

function Build-Probe([string]$Name, [string]$FeatureList) {
    Write-Host "==> $Name (features: $FeatureList)" -ForegroundColor Cyan
    cargo build -q --profile size --example size_probe --no-default-features --features $FeatureList
    if ($LASTEXITCODE -ne 0) { throw "build failed: $Name" }
    $windows = $IsWindows -or $env:OS -eq 'Windows_NT'
    $exe = if ($windows) { 'size_probe.exe' } else { 'size_probe' }
    # Each build gets its own folder: the exe names its .pdb by absolute path, so the
    # pair has to stay together under the name the linker gave it.
    $dir = "target/size/examples/$Name"
    New-Item -ItemType Directory -Force $dir | Out-Null
    Copy-Item "target/size/examples/$exe" "$dir/$exe" -Force
    if ($Symbols -and $windows) { Copy-Item 'target/size/examples/size_probe.pdb' "$dir/size_probe.pdb" -Force }
    Write-Host "    $dir/$exe"
    return (Get-Item "$dir/$exe").Length
}

$base = 'std'
if ($Features) { $base = "std,$Features" }
$without = Build-Probe 'without-formatter' $base
$with = Build-Probe 'with-formatter' "$base,formatter"

Write-Host ""
Write-Host ("without formatter: {0,10:N0} bytes" -f $without)
Write-Host ("with formatter:    {0,10:N0} bytes" -f $with)
Write-Host ("formatter adds:    {0,10:N0} bytes ({1:N1}%)" -f ($with - $without), (100.0 * ($with - $without) / $without))
