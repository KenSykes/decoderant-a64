<#
.SYNOPSIS
Runs the crate's checks and tests, and says what it ran and what it skipped.

.DESCRIPTION
With no options: format, clippy, every test with all features, the docs, and the
generated-files check (when the pinned spec is unpacked). Every step runs even when
an earlier one fails; the exit code is 1 if any failed. A step that cannot run here
is listed with the reason.

Works in Windows PowerShell 5.1 and PowerShell 7 (pwsh, any OS).

.PARAMETER Sweep
Also the 2^32-word sweeps (minutes).

.PARAMETER Fingerprint
Also compare the behaviour fingerprint with this file (the first run writes it;
about 10 minutes).

.PARAMETER Exhaustive
Also the exhaustive oracle comparison (hours).

.PARAMETER All
-Sweep and -Exhaustive.

.EXAMPLE
.\scripts\test.ps1

.EXAMPLE
.\scripts\test.ps1 -Sweep -Fingerprint before.txt
#>
param(
    [switch]$Sweep,
    [string]$Fingerprint,
    [switch]$Exhaustive,
    [switch]$All
)

Set-Location (Split-Path -Parent $PSScriptRoot)
if ($All) { $Sweep = $true; $Exhaustive = $true }

$results = New-Object System.Collections.Generic.List[string]
$script:failures = 0

# Runs a command (an executable and its arguments) as one step.
function Invoke-Step([string]$Name, [string[]]$Command, [hashtable]$Env = @{}) {
    Write-Host ""
    Write-Host "==> $Name" -ForegroundColor Cyan
    Write-Host "    $($Command -join ' ')"
    $saved = @{}
    foreach ($k in $Env.Keys) {
        $saved[$k] = [Environment]::GetEnvironmentVariable($k)
        [Environment]::SetEnvironmentVariable($k, $Env[$k])
    }
    $exe = $Command[0]
    $rest = @($Command | Select-Object -Skip 1)
    $ok = $false
    try {
        if (-not (Get-Command $exe -ErrorAction SilentlyContinue)) {
            Write-Host "    $exe is not installed or not on PATH" -ForegroundColor Red
        } else {
            $global:LASTEXITCODE = 0
            & $exe @rest
            $ok = ($LASTEXITCODE -eq 0)
        }
    } catch {
        Write-Host "    $_" -ForegroundColor Red
    } finally {
        foreach ($k in $saved.Keys) { [Environment]::SetEnvironmentVariable($k, $saved[$k]) }
    }
    if ($ok) {
        $results.Add("  ok      $Name")
    } else {
        $results.Add("  FAILED  $Name")
        $script:failures++
    }
}

function Skip-Step([string]$Name, [string]$Reason) {
    Write-Host ""
    Write-Host "==> ${Name}: skipped ($Reason)" -ForegroundColor Yellow
    $results.Add("  skipped ${Name}: $Reason")
}

Invoke-Step "format" @('cargo', 'fmt', '--check')
Invoke-Step "clippy" @('cargo', 'clippy', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings')
Invoke-Step "tests" @('cargo', 'test', '--workspace', '--all-features', '--no-fail-fast')
Invoke-Step "docs" @('cargo', 'doc', '--no-deps', '--all-features') @{ RUSTDOCFLAGS = '-D warnings' }

# The generated tables must match the pinned spec, when it is here.
$release = ((Get-Content 'spec/REVISION') |
    Where-Object { $_ -like 'release *' }) -replace '^release ', ''
$candidates = @("spec/data/$release")
if ($env:AARCHMRS_DATA_DIR) { $candidates = @("$env:AARCHMRS_DATA_DIR/$release") + $candidates }
$spec = $candidates | Where-Object { Test-Path "$_/Instructions.json" } | Select-Object -First 1
if ($spec) {
    Invoke-Step "generated files up to date" @(
        'cargo', 'run', '-q', '-p', 'decoderant-a64-gen', '--release', '--', '--spec', $spec, '--check')
} else {
    Skip-Step "generated files check" "spec $release not found (fetch it with spec/fetch.cmake)"
}

if ($Sweep) {
    Invoke-Step "sweeps (2^32 words)" @(
        'cargo', 'test', '--release', '--all-features', '--test', 'sweep', '--', '--ignored')
}
if ($Fingerprint) {
    Invoke-Step "fingerprint ($Fingerprint)" @(
        'cargo', 'test', '--release', '--all-features', '--test', 'fingerprint', '--', '--ignored') `
        @{ DECODERANT_A64_FINGERPRINT = $Fingerprint }
}
if ($Exhaustive) {
    Invoke-Step "exhaustive oracle comparison" @(
        'cargo', 'test', '--release', '--all-features', '--test', 'oracles', '--', '--ignored')
}

Write-Host ""
Write-Host "==> summary" -ForegroundColor Cyan
$results | ForEach-Object { Write-Host $_ }
if ($script:failures -gt 0) { exit 1 }
exit 0
