# Re-enter the installed MSVC environment for client-only work. Building the
# native libraries remains an explicit prerequisite, not a per-test operation.
$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
$native = Join-Path $repository 'work/hotr-build/native'
foreach ($name in @('libcrypto.lib','sqlcipher.lib','sqlite3.c','sqlite3.h')) {
    if (-not (Test-Path -LiteralPath (Join-Path $native $name) -PathType Leaf)) {
        throw 'Native prerequisites missing. Run .cargo/prepare-native.ps1.'
    }
}
$outputs = @('libcrypto.lib','sqlcipher.lib') | ForEach-Object { Get-Item -LiteralPath (Join-Path $native $_) }
$oldestOutput = ($outputs | Sort-Object LastWriteTimeUtc | Select-Object -First 1).LastWriteTimeUtc
foreach ($name in @('xtask/native-builder/build.rs','xtask/native-builder/Cargo.toml','xtask/native-builder/Cargo.lock','work/hotr-build/native/sqlite3.c','.cargo/config.toml','.cargo/prepare-native.ps1')) {
    if ((Get-Item -LiteralPath (Join-Path $repository $name)).LastWriteTimeUtc -gt $oldestOutput) {
        throw "Native input changed: $name. Run .cargo/prepare-native.ps1."
    }
}
foreach ($item in @($native, (Join-Path $repository 'work/hotr-tool-cache/cargo'), (Join-Path $repository 'work/hotr-build/tmp'))) {
    for ($ancestor = $item; $ancestor -and $ancestor.Length -ge $repository.Length; $ancestor = Split-Path $ancestor -Parent) {
        if ((Get-Item -LiteralPath $ancestor).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Native path contains a reparse point' }
    }
}
$env:CARGO_HOME = Join-Path $repository 'work/hotr-tool-cache/cargo'
$env:TEMP = Join-Path $repository 'work/hotr-build/tmp'
$env:TMP = $env:TEMP
$env:LC_ALL = 'C'
$env:LANG = 'C'
$env:CARGO_BUILD_JOBS = '4'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$installation = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $installation) { throw 'MSVC x64 Build Tools are required' }
Import-Module (Join-Path $installation 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $installation -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
if ((Get-Item -LiteralPath (Get-Command cl.exe).Source).LastWriteTimeUtc -gt $oldestOutput) {
    throw 'Native compiler changed. Run .cargo/prepare-native.ps1.'
}
Write-Host 'Reusing prepared native libraries; runtime encryption and integrity checks remain required.'
