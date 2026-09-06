param([ValidateSet('lamprey-preflight', 'lamprey-smoke', 'lamprey-acceptance', 'hermes-preflight', 'hermes-acceptance', 'native', 'imports', 'lifecycle', 'embedding', 'hybrid', 'evaluation', 'viewer', 'inspect')][string]$Mode = 'lamprey-acceptance', [ValidateSet('metadata','hermes','hermes-help','desktop','build','evidence')][string]$Section = 'metadata')
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
if ($Mode -eq 'viewer') {
    $env:HOTR_PLAYWRIGHT_MODULE = Join-Path $env:USERPROFILE '.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright'
    $env:HOTR_CHROME_EXE = Join-Path $env:ProgramFiles 'Google/Chrome/Application/chrome.exe'
    if (-not (Test-Path -LiteralPath $env:HOTR_PLAYWRIGHT_MODULE -PathType Container) -or -not (Test-Path -LiteralPath $env:HOTR_CHROME_EXE -PathType Leaf)) { throw 'Installed Chrome and Playwright are required for the actual browser gate.' }
    cargo xtask verify --prompt HOTR-18
    exit $LASTEXITCODE
}
if ($Mode -eq 'evaluation') {
    cargo xtask verify --prompt HOTR-17
    exit $LASTEXITCODE
}
if ($Mode -eq 'hybrid') {
    cargo xtask verify --prompt HOTR-16
    exit $LASTEXITCODE
}
if ($Mode -eq 'embedding') {
    cargo xtask verify --prompt HOTR-15
} elseif ($Mode -eq 'lifecycle') {
    cargo xtask verify --prompt HOTR-14
} elseif ($Mode -eq 'imports') {
    cargo xtask verify --prompt HOTR-13
} elseif ($Mode -eq 'native') {
    cargo xtask verify --prompt HOTR-03
} else {
    cargo xtask $Mode
}
exit $LASTEXITCODE
