param([Parameter(Mandatory=$true)][string]$Challenge)
$ErrorActionPreference = 'Stop'
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$testRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../work/hotr-tests')) + [IO.Path]::DirectorySeparatorChar
$challengePath = [IO.Path]::GetFullPath($Challenge)
if (-not $challengePath.StartsWith($testRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Challenge outside synthetic root' }
$runDirectory = [IO.Path]::GetDirectoryName($challengePath)
if (-not (Test-Path -LiteralPath (Join-Path $runDirectory 'SYNTHETIC-ONLY') -PathType Leaf)) { throw 'Missing synthetic marker' }
$challengeData = Get-Content -LiteralPath $challengePath -Raw | ConvertFrom-Json
if (-not $identity.IsAuthenticated -or $identity.User.Value -eq $challengeData.owner_sid) { throw 'A different authenticated Windows account is required' }
foreach ($name in @('directory', 'database', 'marker', 'receipt')) {
    $candidate = [IO.Path]::GetFullPath($challengeData.$name)
    if (-not $candidate.StartsWith($runDirectory + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Probe path outside marked run' }
}
if ($challengeData.pipe -notmatch '^\\\\\.\\pipe\\hotr-owner-v1-[0-9a-f]{64}$') { throw 'Unexpected pipe endpoint' }
function Test-AccessDenied([scriptblock]$Operation) {
    try { & $Operation; return @{ denied=$false; win32_error=0 } }
    catch {
        $exception = $_.Exception.GetBaseException()
        $code = $exception.HResult -band 65535
        return @{ denied=($code -eq 5); win32_error=$code; exception=$exception.GetType().FullName }
    }
}
$result = [ordered]@{ sid=$identity.User.Value; authenticated=$identity.IsAuthenticated; utc=[DateTime]::UtcNow.ToString('o') }
$result.directory = Test-AccessDenied { $null = [IO.Directory]::GetFileSystemEntries($challengeData.directory) }
foreach ($name in @('database', 'marker')) {
    $filePath = $challengeData.$name
    $result[$name] = Test-AccessDenied {
        $stream = [IO.File]::Open($filePath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
        $stream.Dispose()
    }
}
$result.pipe = Test-AccessDenied {
    $pipeName = $challengeData.pipe.Substring(9)
    $pipe = [IO.Pipes.NamedPipeClientStream]::new('.', $pipeName, [IO.Pipes.PipeDirection]::InOut)
    try { $pipe.Connect(1000) } finally { $pipe.Dispose() }
}
$json = $result | ConvertTo-Json -Depth 5
# Exclusive temporary receipt, then non-overwriting atomic publication in this
# new marked synthetic run. No vault data or existing file is modified.
$temporary = $challengeData.receipt + '.pending'
$stream = [IO.File]::Open($temporary, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
try {
    $bytes = [Text.Encoding]::UTF8.GetBytes($json)
    $stream.Write($bytes, 0, $bytes.Length)
    $stream.Flush($true)
} finally { $stream.Dispose() }
if ([IO.File]::Exists($challengeData.receipt)) { throw 'Receipt already exists' }
[IO.File]::Move($temporary, $challengeData.receipt)
Write-Output $json
foreach ($name in @('directory', 'database', 'marker', 'pipe')) {
    if (-not $result[$name].denied) { throw "Boundary failed for $name" }
}
