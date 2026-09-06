# Read-only installation and capability inventory. Never print secret values,
# process command lines, chat history, or user profile contents.
param([ValidateSet('metadata','hermes','hermes-help','desktop','build','evidence')][string]$Section = 'metadata')
$ErrorActionPreference = 'Stop'
$hermesSource = Join-Path $env:LOCALAPPDATA 'hermes/hermes-agent'
$programs = Join-Path $env:LOCALAPPDATA 'Programs'
if ($Section -eq 'evidence') {
    $evidenceRoot = Join-Path $PSScriptRoot '../work/hotr-evidence'
    $nativeGate = Get-ChildItem -LiteralPath $evidenceRoot -Directory -Filter 'HOTR-03-*' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($nativeGate -and (Test-Path -LiteralPath (Join-Path $nativeGate.FullName 'manifest.json'))) {
        Get-Content -LiteralPath (Join-Path $nativeGate.FullName 'manifest.json') -Raw | ConvertFrom-Json | Select-Object prompt,result,failure,product_sha256,commands | ConvertTo-Json -Depth 5
        Get-Content -LiteralPath (Join-Path $nativeGate.FullName 'tests.txt') -Tail 100 -ErrorAction SilentlyContinue
    }
    $gate = Get-ChildItem -LiteralPath $evidenceRoot -Directory -Filter 'HOTR-12-LAMPREY-*' | Where-Object Name -NotMatch 'SMOKE|PREFLIGHT' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $manifest = Get-Content -LiteralPath (Join-Path $gate.FullName 'manifest.json') -Raw | ConvertFrom-Json
    $manifest | Select-Object prompt,result,scope,product_sha256,runner_sha256,commands | ConvertTo-Json -Depth 6
    Get-Content -LiteralPath (Join-Path $gate.FullName 'installed-lamprey-smoke.txt') -Tail 38
    Get-ChildItem -LiteralPath (Join-Path $evidenceRoot 'HOTR-compatibility-budget-20260906') -Filter 'attempt-*.json' | ForEach-Object { Get-Content -LiteralPath $_.FullName }
    exit 0
}
if ($Section -eq 'hermes-help') {
    node (Join-Path $PSScriptRoot 'inspect-hermes-cli.cjs')
    exit $LASTEXITCODE
}
if ($Section -eq 'desktop') {
    node (Join-Path $PSScriptRoot 'inspect-electron.cjs')
    exit $LASTEXITCODE
}
if ($Section -eq 'build') {
    Get-Content (Join-Path $PSScriptRoot '../xtask/src/main.rs') -TotalCount 180
    Get-Content (Join-Path $PSScriptRoot '../docs/DEVLOG.md') -Tail 75
    Get-Content (Join-Path $PSScriptRoot '../PLANNING/HOME-ON-THE-RANGE-PSPR.md') -TotalCount 75
    $testRoot = Join-Path $PSScriptRoot '../work/hotr-tests'
    Get-ChildItem -LiteralPath $testRoot -Filter 'initialization-timeout.mcp.stderr.txt' -Recurse -File | Sort-Object LastWriteTime -Descending | Select-Object -First 3 | ForEach-Object {
        [pscustomobject]@{Path=$_.FullName;Created=$_.CreationTimeUtc;Modified=$_.LastWriteTimeUtc;Bytes=$_.Length} | ConvertTo-Json
        Get-Content -LiteralPath $_.FullName
    }
    $registry = Join-Path $PSScriptRoot '../work/hotr-tool-cache/cargo/registry/src'
    Get-ChildItem -LiteralPath $registry -Directory | ForEach-Object { Get-ChildItem -LiteralPath $_.FullName -Directory -Filter 'rmcp-*' } | ForEach-Object { rg -n 'fn serve_with_ct|fn serve_server|initialize.*timeout|struct RunningService' (Join-Path $_.FullName 'src') }
    Get-Process cargo,cl,link,Lamprey -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,CPU,StartTime | ConvertTo-Json
    exit 0
}
if ($Section -eq 'hermes') {
    Get-ChildItem -LiteralPath (Join-Path $env:LOCALAPPDATA 'hermes/bin') -Force | Select-Object Name,Length | ConvertTo-Json
    rg --files --hidden --no-ignore (Join-Path $env:LOCALAPPDATA 'hermes') -g 'python.exe' -g 'pyvenv.cfg' | Select-Object -First 20
    rg --files (Join-Path $hermesSource 'hermes_cli') -g '*chat*' -g '*mcp*'
    rg -n -A 6 '^def cmd_mcp|^def cmd_chat' (Join-Path $hermesSource 'hermes_cli/main.py')
    Get-Content (Join-Path $hermesSource 'hermes_cli/main.py') | Select-Object -Skip 3190 -First 85
    rg -n -A 8 -- '"--query"|"--toolsets"|"--max-turns"|"--quiet"|"--json"|"--provider"' (Join-Path $hermesSource 'cli.py') | Select-Object -First 160
    Get-ChildItem -LiteralPath (Join-Path $env:LOCALAPPDATA 'hermes/runtime') -Force | Select-Object Name,Mode | ConvertTo-Json
    rg --files --hidden --no-ignore (Join-Path $env:LOCALAPPDATA 'hermes/runtime') -g 'python.exe' -g 'pyvenv.cfg' -g 'hermes.exe' | Select-Object -First 15
    rg -n -A 10 -- '--query|--toolsets|--max-turns|--quiet|--json|--provider' (Join-Path $hermesSource 'hermes_cli/subcommands/_shared.py') | Select-Object -First 125
    rg -n 'def _test|def test|def mcp_command|class MCPServerTask|def call_tool|async def call_tool' (Join-Path $hermesSource 'hermes_cli/mcp_config.py') (Join-Path $hermesSource 'tools/mcp_tool.py')
    exit 0
}
Get-Command hermes, opencode, qwen, gemini, code, cursor, ollama, docker -ErrorAction SilentlyContinue |
    Select-Object Name, Source | ConvertTo-Json
foreach ($name in @('Lamprey', 'Qwen', 'Chatbox', 'Grok Bot', '@openworkdesktop', '@opencode-aidesktop')) {
    $directory = Join-Path $programs $name
    if (Test-Path -LiteralPath $directory) {
        Get-ChildItem -LiteralPath $directory -File -Filter '*.exe' | ForEach-Object {
            [pscustomobject]@{Application=$name; Path=$_.FullName; Version=$_.VersionInfo.ProductVersion; Bytes=$_.Length}
        } | ConvertTo-Json
        $resources = Join-Path $directory 'resources'
        if (Test-Path -LiteralPath $resources) {
            Get-ChildItem -LiteralPath $resources | Select-Object Name, Length | ConvertTo-Json
        }
    }
}
$package = Join-Path $hermesSource 'pyproject.toml'
if (Test-Path -LiteralPath $package) {
    rg -n '^version|^requires-python|hermes =|mcp' $package
    Get-ChildItem -LiteralPath (Join-Path $hermesSource '.venv/Scripts') -Filter 'python*.exe' -ErrorAction SilentlyContinue | Select-Object Name,FullName | ConvertTo-Json
    rg --files (Join-Path $hermesSource 'tools') -g '*mcp*'
}
try {
    $models = Invoke-RestMethod -Uri 'http://127.0.0.1:11434/api/tags' -TimeoutSec 5
    $models.models | Select-Object name,size,digest,details | ConvertTo-Json -Depth 4
} catch { 'Ollama model listing unavailable: ' + $_.Exception.GetType().Name }
$extensions = Join-Path $env:USERPROFILE '.vscode/extensions'
if (Test-Path -LiteralPath $extensions) {
    Get-ChildItem -LiteralPath $extensions -Directory | Where-Object Name -Match 'continue|claude-dev|copilot|codex' | ForEach-Object {
        $manifest = Get-Content -LiteralPath (Join-Path $_.FullName 'package.json') -Raw | ConvertFrom-Json
        [pscustomobject]@{Name=$manifest.name; Version=$manifest.version; ExtensionPath=$_.FullName; Main=$manifest.main}
    } | ConvertTo-Json
}
Get-Process cargo,cl,link,Lamprey,hermes,Qwen,ollama,unsloth -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,CPU,StartTime | ConvertTo-Json
exit 0
