$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $root

function Assert-ProjectPath([string]$Path) {
    $absolute = [IO.Path]::GetFullPath($Path)
    if (-not $absolute.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Build path escaped project' }
    $item = $absolute
    while ($item -and $item.Length -ge $root.Length) {
        if (Test-Path -LiteralPath $item) {
            if ((Get-Item -LiteralPath $item).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Build path contains a reparse point' }
        }
        $item = Split-Path $item -Parent
    }
}

$download = Join-Path $root 'work/hotr-tool-cache/downloads'
$native = Join-Path $root 'work/hotr-build/native'
$cache = Join-Path $root 'work/hotr-tool-cache/cargo'
$temp = Join-Path $root 'work/hotr-build/tmp'
$perl = Join-Path $root 'work/hotr-tool-cache/native-perl-5.42.3.1'
foreach ($path in @($download, $native, $cache, $temp, $perl)) {
    Assert-ProjectPath $path
    New-Item -ItemType Directory -Path $path -Force | Out-Null
}

function Get-PinnedArchive([string]$Name, [string]$Url, [string]$Hash) {
    $path = Join-Path $download $Name
    if (-not (Test-Path -LiteralPath $path)) { Invoke-WebRequest $Url -OutFile $path -TimeoutSec 180 }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Hash) { throw "Checksum mismatch: $Name; retained for inspection" }
    return $path
}

$sqlArchive = Get-PinnedArchive 'sqlcipher-4.18.0.tar.gz' 'https://github.com/sqlcipher/sqlcipher/archive/refs/tags/v4.18.0.tar.gz' '1df02d1b346fa27feaf2da2cb2c0d8209e788248e461ec288718aa5d3e9643e5'
$source = Join-Path $download 'sqlcipher-4.18.0'
if (-not (Test-Path -LiteralPath $source)) {
    tar -xf $sqlArchive -C $download
    if ($LASTEXITCODE -ne 0) { throw 'SQLCipher source extraction failed' }
}
Assert-ProjectPath $source
$perlArchive = Get-PinnedArchive 'strawberry-perl-5.42.3.1-64bit-portable.zip' 'https://github.com/StrawberryPerl/Perl-Dist-Strawberry/releases/download/SP_54231_64bit/strawberry-perl-5.42.3.1-64bit-portable.zip' '6a081a811781c30aca51dbc036afd93092af91e3297901f02c17043795a10690'
if (-not (Test-Path -LiteralPath (Join-Path $perl 'perl/bin/perl.exe'))) {
    tar -xf $perlArchive -C $perl 'perl'
    if ($LASTEXITCODE -ne 0) { throw 'Native Perl extraction failed' }
}

$env:CARGO_HOME = $cache
$env:TEMP = $temp
$env:TMP = $temp
$env:LC_ALL = 'C'
$env:LANG = 'C'
$env:CARGO_BUILD_JOBS = '4'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw 'MSVC x64 Build Tools are required' }
Import-Module (Join-Path $vs 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
Push-Location -LiteralPath $native
try {
    # Upstream tools manage their own generated files ONLY in this checked directory.
    & nmake /nologo /f (Join-Path $source 'Makefile.msc') "TOP=$source" 'NO_TCL=1' sqlite3.c
    if ($LASTEXITCODE -ne 0) { throw 'SQLCipher source generation failed' }
} finally { Pop-Location }
& cargo build --manifest-path (Join-Path $root 'xtask/native-builder/Cargo.toml') --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Native library compilation failed' }
Write-Host 'Native libraries ready. Use cargo build/test --release --locked in this shell.'
