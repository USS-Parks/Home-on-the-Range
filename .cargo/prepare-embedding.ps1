[CmdletBinding()]
param(
    [switch]$Install
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# This is deliberately an explicit, project-local installation. It never calls
# `ollama pull`, starts Ollama, changes an Ollama profile, or accepts a model
# name, registry, cache directory, proxy, or redirect from the caller.
if (-not $Install) {
    throw 'Refusing to download a model without -Install. This script only writes work/hotr-models.'
}

$Repository = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot -Parent)).TrimEnd([IO.Path]::DirectorySeparatorChar)
$ModelRoot = Join-Path $Repository 'work/hotr-models'
$ManifestPath = Join-Path $ModelRoot 'manifests/registry.ollama.ai/library/nomic-embed-text/v1.5'
$BlobsRoot = Join-Path $ModelRoot 'blobs'
$StagingRoot = Join-Path $ModelRoot 'staging'

$RegistryHost = 'registry.ollama.ai'
$ModelName = 'nomic-embed-text:v1.5'
$ManifestDigest = '0a109f422b47e3a30ba2b10eca18548e944e8a23073ee3f3e947efcf3c45e59f'
$ModelBlobDigest = '970aa74c0a90ef7482477cf803618e776e173c007bf957f635f1015bfcfef0e6'
$ModelBlobBytes = [int64]274290656
$MaxModelBytes = [int64]1GB
$MaxGeneratedBytes = [int64]20GB
$MinFreeBytes = [int64]25GB
$MaxManifestBytes = [int64]4MB
$RequestTimeout = [TimeSpan]::FromSeconds(90)
$script:ProvenBlobHost = $null

function Assert-WithinRepository([string]$Path) {
    $absolute = [IO.Path]::GetFullPath($Path)
    $prefix = $Repository + [IO.Path]::DirectorySeparatorChar
    if (-not $absolute.Equals($Repository, [StringComparison]::OrdinalIgnoreCase) -and -not $absolute.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes repository: $Path"
    }
    return $absolute
}

function Assert-NoReparsePoint([string]$Path) {
    $absolute = Assert-WithinRepository $Path
    for ($item = $absolute; $item; $item = Split-Path $item -Parent) {
        if (Test-Path -LiteralPath $item) {
            $attributes = (Get-Item -LiteralPath $item -Force).Attributes
            if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Project path contains a reparse point: $item"
            }
        }
        if ($item.Equals($Repository, [StringComparison]::OrdinalIgnoreCase)) { break }
    }
    for ($item = Split-Path $Repository -Parent; $item; $item = Split-Path $item -Parent) {
        if (Test-Path -LiteralPath $item) {
            $attributes = (Get-Item -LiteralPath $item -Force).Attributes
            if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Repository ancestor contains a reparse point: $item"
            }
        }
        $parent = Split-Path $item -Parent
        if (-not $parent -or $parent.Equals($item, [StringComparison]::OrdinalIgnoreCase)) { break }
    }
    return $absolute
}

function New-SafeDirectory([string]$Path) {
    $absolute = Assert-NoReparsePoint $Path
    if (Test-Path -LiteralPath $absolute) {
        if (-not (Test-Path -LiteralPath $absolute -PathType Container)) {
            throw "Expected directory, found another object: $absolute"
        }
        return $absolute
    }
    $parent = Split-Path $absolute -Parent
    if ($parent) { New-SafeDirectory $parent | Out-Null }
    [IO.Directory]::CreateDirectory($absolute) | Out-Null
    return (Assert-NoReparsePoint $absolute)
}

function Get-SafeDirectoryBytes([IO.DirectoryInfo]$Directory) {
    if (($Directory.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Generated-state tree contains a reparse point: $($Directory.FullName)"
    }
    [int64]$total = 0
    try {
        foreach ($entry in $Directory.EnumerateFileSystemInfos()) {
            # FileSystemInfo supplies the enumerator's cached metadata. Do not
            # reopen every cached file while accounting for the work budget.
            if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Generated-state tree contains a reparse point: $($entry.FullName)"
            }
            if ($entry -is [IO.DirectoryInfo]) {
                $total += Get-SafeDirectoryBytes $entry
            } elseif ($entry -is [IO.FileInfo]) {
                $total += [int64]$entry.Length
            } else {
                throw "Generated-state tree contains an unsupported entry: $($entry.FullName)"
            }
            if ($total -gt $MaxGeneratedBytes) { return $total }
        }
    } catch {
        throw "Unable to safely inventory generated state below $($Directory.FullName): $($_.Exception.Message)"
    }
    return $total
}

function Get-SafeTreeBytes([string]$Path) {
    $absolute = Assert-NoReparsePoint $Path
    $directory = [IO.DirectoryInfo]::new($absolute)
    if ($directory.Exists) { return Get-SafeDirectoryBytes $directory }
    $file = [IO.FileInfo]::new($absolute)
    if ($file.Exists) {
        if (($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Generated-state tree contains a reparse point: $absolute" }
        return [int64]$file.Length
    }
    return [int64]0
}

function Assert-ResourceBudget([int64]$AdditionalBytes) {
    $work = Join-Path $Repository 'work'
    $used = Get-SafeTreeBytes $work
    if ($used -gt ($MaxGeneratedBytes - $AdditionalBytes)) {
        throw "Generated-state budget exceeded: work uses $used bytes; limit is $MaxGeneratedBytes bytes"
    }
    $drive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($Repository))
    if ($drive.AvailableFreeSpace -lt ($MinFreeBytes + $AdditionalBytes)) {
        throw "Insufficient free space: need at least $MinFreeBytes bytes free after the bounded download"
    }
}

function Assert-PublicHttpsUri([uri]$Uri) {
    if (-not $Uri.IsAbsoluteUri -or $Uri.Scheme -ne 'https' -or $Uri.UserInfo -or ($Uri.Port -ne -1 -and $Uri.Port -ne 443)) {
        throw 'Only ordinary non-credentialed HTTPS URLs are allowed'
    }
    $addresses = [Net.Dns]::GetHostAddresses($Uri.DnsSafeHost)
    if ($addresses.Count -eq 0) { throw "No address resolved for $($Uri.DnsSafeHost)" }
    foreach ($address in $addresses) {
        if ([Net.IPAddress]::IsLoopback($address) -or $address.Equals([Net.IPAddress]::Any) -or $address.Equals([Net.IPAddress]::IPv6Any) -or
            $address.Equals([Net.IPAddress]::IPv6None) -or $address.IsIPv6LinkLocal -or $address.IsIPv6SiteLocal -or $address.IsIPv6Multicast) {
            throw "Refusing a private or special address for $($Uri.DnsSafeHost)"
        }
        if ($address.AddressFamily -eq [Net.Sockets.AddressFamily]::InterNetworkV6) {
            $v6 = $address.GetAddressBytes()
            if (($v6[0] -band 0xfe) -eq 0xfc) { throw "Refusing a private or special address for $($Uri.DnsSafeHost)" }
        }
        $checkedAddress = if ($address.IsIPv4MappedToIPv6) { $address.MapToIPv4() } else { $address }
        if ($checkedAddress.AddressFamily -eq [Net.Sockets.AddressFamily]::InterNetwork) {
            $octets = $checkedAddress.GetAddressBytes()
            if ($octets[0] -eq 10 -or $octets[0] -eq 127 -or $octets[0] -eq 0 -or $octets[0] -ge 224 -or
                ($octets[0] -eq 100 -and $octets[1] -ge 64 -and $octets[1] -le 127) -or
                ($octets[0] -eq 169 -and $octets[1] -eq 254) -or
                ($octets[0] -eq 172 -and $octets[1] -ge 16 -and $octets[1] -le 31) -or
                ($octets[0] -eq 192 -and $octets[1] -eq 168)) {
                throw "Refusing a private or special address for $($Uri.DnsSafeHost)"
            }
        }
    }
}

function New-RegistryClient {
    $handler = [Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $false
    $handler.UseProxy = $false
    $client = [Net.Http.HttpClient]::new($handler, $true)
    $client.Timeout = $RequestTimeout
    $client.DefaultRequestHeaders.UserAgent.ParseAdd('HomeOnTheRange-HOTR-15/1.0')
    return $client
}

function Assert-TransferDeadline([Diagnostics.Stopwatch]$Timer) {
    if ($Timer.Elapsed -gt $RequestTimeout) { throw "Model transfer exceeded the $($RequestTimeout.TotalSeconds)-second deadline" }
}

function Get-ExpectedHash([string]$Path, [string]$ExpectedDigest, [int64]$ExpectedBytes) {
    $safe = Assert-NoReparsePoint $Path
    if (-not (Test-Path -LiteralPath $safe -PathType Leaf)) { return $false }
    $length = [int64](Get-Item -LiteralPath $safe -Force).Length
    if ($length -ne $ExpectedBytes) { return $false }
    return ((Get-FileHash -LiteralPath $safe -Algorithm SHA256).Hash.ToLowerInvariant() -eq $ExpectedDigest)
}

function Get-VerifiedManifest([string]$Path) {
    $safe = Assert-NoReparsePoint $Path
    if (-not (Test-Path -LiteralPath $safe -PathType Leaf)) { return $false }
    $length = [int64](Get-Item -LiteralPath $safe -Force).Length
    if ($length -le 0 -or $length -gt $MaxManifestBytes) { return $false }
    return ((Get-FileHash -LiteralPath $safe -Algorithm SHA256).Hash.ToLowerInvariant() -eq $ManifestDigest)
}

function Get-ObjectToStaging([Net.Http.HttpClient]$Client, [string]$RelativePath, [int64]$MaximumBytes, [string]$ExpectedDigest, [bool]$IsBlob) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $cancellation = [Threading.CancellationTokenSource]::new($RequestTimeout)
    $completed = $false
    $response = $null
    $initial = [uri]("https://$RegistryHost/v2/library/nomic-embed-text/$RelativePath")
    try {
        Assert-PublicHttpsUri $initial
        Assert-TransferDeadline $timer
        $response = $Client.GetAsync($initial, [Net.Http.HttpCompletionOption]::ResponseHeadersRead, $cancellation.Token).GetAwaiter().GetResult()
        Assert-TransferDeadline $timer
        if ([int]$response.StatusCode -ge 300 -and [int]$response.StatusCode -lt 400) {
            if (-not $IsBlob -or -not $response.Headers.Location) { throw "Redirect refused for $RelativePath" }
            $redirect = [uri]::new($initial, $response.Headers.Location)
            Assert-PublicHttpsUri $redirect
            if ($redirect.DnsSafeHost -eq $RegistryHost) { throw "Unexpected registry redirect for $RelativePath" }
            if ($script:ProvenBlobHost -and $script:ProvenBlobHost -ne $redirect.DnsSafeHost) { throw "Blob host changed during installation" }
            $script:ProvenBlobHost = $redirect.DnsSafeHost
            $response.Dispose()
            $response = $Client.GetAsync($redirect, [Net.Http.HttpCompletionOption]::ResponseHeadersRead, $cancellation.Token).GetAwaiter().GetResult()
            Assert-TransferDeadline $timer
        }
        if (-not $response.IsSuccessStatusCode) { throw "Registry request failed for ${RelativePath}: $([int]$response.StatusCode)" }
        if ($response.Content.Headers.ContentLength -and $response.Content.Headers.ContentLength -gt $MaximumBytes) { throw "Registry object exceeds its size limit: $RelativePath" }

        $staging = Join-Path $StagingRoot ("$([guid]::NewGuid().ToString('N')).partial")
        Assert-NoReparsePoint $StagingRoot | Out-Null
        $input = $response.Content.ReadAsStreamAsync($cancellation.Token).GetAwaiter().GetResult()
        $output = [IO.FileStream]::new($staging, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
        $hasher = [Security.Cryptography.SHA256]::Create()
        [int64]$total = 0
        try {
            $buffer = New-Object byte[] 131072
            while (($read = $input.ReadAsync($buffer, 0, $buffer.Length, $cancellation.Token).GetAwaiter().GetResult()) -gt 0) {
                Assert-TransferDeadline $timer
                $total += $read
                if ($total -gt $MaximumBytes) { throw "Registry object exceeds its size limit while streaming: $RelativePath" }
                $hasher.TransformBlock($buffer, 0, $read, $buffer, 0) | Out-Null
                $output.Write($buffer, 0, $read)
            }
            $hasher.TransformFinalBlock($buffer, 0, 0) | Out-Null
        } finally {
            $output.Dispose()
            $input.Dispose()
        }
        $actual = ([BitConverter]::ToString($hasher.Hash)).Replace('-', '').ToLowerInvariant()
        $hasher.Dispose()
        if ($actual -ne $ExpectedDigest) { throw "Checksum mismatch for $RelativePath; retained staging file: $staging" }
        Assert-TransferDeadline $timer
        $completed = $true
        return [pscustomobject]@{ Path = $staging; Bytes = $total; Timer = $timer; Cancellation = $cancellation }
    } catch {
        if ($cancellation.IsCancellationRequested -or $timer.Elapsed -gt $RequestTimeout) {
            throw 'Model transfer exceeded its 90-second deadline; any staging file is retained for inspection'
        }
        throw "Registry transfer failed for $RelativePath; any staging file is retained for inspection"
    } finally {
        if ($response) { $response.Dispose() }
        if (-not $completed) { $cancellation.Dispose() }
    }
}

function Publish-VerifiedFile([string]$StagingPath, [string]$Destination, [string]$ExpectedDigest, [int64]$ExpectedBytes, [Diagnostics.Stopwatch]$Timer) {
    Assert-TransferDeadline $Timer
    if (-not (Get-ExpectedHash $StagingPath $ExpectedDigest $ExpectedBytes)) {
        throw "Refusing to publish an unverified file: $StagingPath"
    }
    Assert-TransferDeadline $Timer
    $target = Assert-NoReparsePoint $Destination
    if (Test-Path -LiteralPath $target) { throw "Refusing to overwrite existing destination: $target" }
    Move-Item -LiteralPath $StagingPath -Destination $target -ErrorAction Stop
    Assert-TransferDeadline $Timer
    if (-not (Get-ExpectedHash $target $ExpectedDigest $ExpectedBytes)) { throw "Published file did not verify: $target" }
    Assert-TransferDeadline $Timer
}

New-SafeDirectory $ModelRoot | Out-Null
New-SafeDirectory (Split-Path $ManifestPath -Parent) | Out-Null
New-SafeDirectory $BlobsRoot | Out-Null
New-SafeDirectory $StagingRoot | Out-Null
Assert-ResourceBudget $MaxModelBytes

$client = New-RegistryClient
try {
    if (-not (Get-VerifiedManifest $ManifestPath)) {
        if (Test-Path -LiteralPath $ManifestPath) { throw "Existing manifest is not the pinned exact file: $ManifestPath" }
        $download = Get-ObjectToStaging $client 'manifests/v1.5' $MaxManifestBytes $ManifestDigest $false
        try { Publish-VerifiedFile $download.Path $ManifestPath $ManifestDigest $download.Bytes $download.Timer } finally { $download.Cancellation.Dispose() }
    }

    $manifestText = [IO.File]::ReadAllText((Assert-NoReparsePoint $ManifestPath), [Text.Encoding]::UTF8)
    $manifest = $manifestText | ConvertFrom-Json
    if ($manifest.schemaVersion -ne 2 -or -not $manifest.config -or -not $manifest.layers) { throw 'Pinned manifest has an unsupported OCI layout' }
    $descriptors = @($manifest.config) + @($manifest.layers)
    [int64]$declaredBytes = 0
    $seen = @{}
    $modelDescriptorFound = $false
    foreach ($descriptor in $descriptors) {
        if (-not $descriptor.digest -or -not $descriptor.size -or $descriptor.digest -notmatch '^sha256:[0-9a-f]{64}$' -or [int64]$descriptor.size -le 0) {
            throw 'Pinned manifest contains an invalid blob descriptor'
        }
        $digest = $descriptor.digest.Substring(7).ToLowerInvariant()
        if ($seen.ContainsKey($digest)) { throw 'Pinned manifest repeats a blob digest' }
        $seen[$digest] = [int64]$descriptor.size
        $declaredBytes += [int64]$descriptor.size
        if ($declaredBytes -gt $MaxModelBytes) { throw "Pinned model exceeds the $MaxModelBytes byte ceiling" }
        if ($digest -eq $ModelBlobDigest) {
            if ([int64]$descriptor.size -ne $ModelBlobBytes) { throw 'Pinned model layer size changed unexpectedly' }
            $modelDescriptorFound = $true
        }
    }
    if (-not $modelDescriptorFound) { throw 'Pinned manifest does not contain the expected model layer' }
    Assert-ResourceBudget $declaredBytes

    foreach ($entry in $seen.GetEnumerator()) {
        $destination = Join-Path $BlobsRoot ("sha256-" + $entry.Key)
        if (Get-ExpectedHash $destination $entry.Key $entry.Value) { continue }
        if (Test-Path -LiteralPath $destination) { throw "Existing blob is not the pinned exact file: $destination" }
        $download = Get-ObjectToStaging $client ("blobs/sha256:" + $entry.Key) $entry.Value $entry.Key $true
        try {
            if ($download.Bytes -ne $entry.Value) { throw "Blob size mismatch: $($entry.Key)" }
            Publish-VerifiedFile $download.Path $destination $entry.Key $entry.Value $download.Timer
        } finally { $download.Cancellation.Dispose() }
    }
} finally {
    $client.Dispose()
}

Write-Host "Verified $ModelName in $ModelRoot. Set OLLAMA_MODELS only in the HOTR-owned Ollama server process environment."
