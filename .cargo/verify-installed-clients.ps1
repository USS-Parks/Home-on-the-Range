param([ValidateSet('lamprey-preflight', 'lamprey-smoke', 'lamprey-acceptance', 'hermes-preflight', 'hermes-acceptance', 'native', 'imports', 'lifecycle', 'inspect')][string]$Mode = 'lamprey-acceptance', [ValidateSet('metadata','hermes','hermes-help','desktop','build','evidence')][string]$Section = 'metadata')
$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $repository
if ($Mode -eq 'inspect') {
    & (Join-Path $PSScriptRoot 'inspect-installed-clients.ps1') -Section $Section
    exit $LASTEXITCODE
}
. (Join-Path $PSScriptRoot 'enter-prepared-native.ps1')
$env:HOTR_RUN_LAMPREY = '1'
$env:HOTR_BOUNDED_LAMPREY = '1'
$env:HOTR_BOUNDED_HERMES = '1'
$env:HOTR_LAMPREY_EXE = Join-Path $env:LOCALAPPDATA 'Programs/Lamprey/Lamprey.exe'
$env:HOTR_LAMPREY_SOURCE = Join-Path $env:USERPROFILE 'Documents/Claude/Lamprey Harness'
if ($Mode -eq 'lifecycle') {
    cargo xtask verify --prompt HOTR-14
} elseif ($Mode -eq 'imports') {
    cargo xtask verify --prompt HOTR-13
} elseif ($Mode -eq 'native') {
    cargo xtask verify --prompt HOTR-03
} else {
    cargo xtask $Mode
}
exit $LASTEXITCODE
