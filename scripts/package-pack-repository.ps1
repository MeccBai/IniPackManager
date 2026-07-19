[CmdletBinding()]
param(
    [string]$PacksRoot = "D:\Program Files (x86)\Mental Omega\PackRepo\Packs",
    [string]$CatalogPath,
    [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"

function Get-ConfigValue {
    param(
        [string]$ConfigPath,
        [string]$Key
    )

    $match = [regex]::Match(
        (Get-Content -LiteralPath $ConfigPath -Raw -Encoding UTF8),
        ('(?mi)^\s*{0}\s*=\s*"([^"]+)"\s*$' -f [regex]::Escape($Key))
    )
    if (-not $match.Success) {
        throw "$ConfigPath 缺少 [Config].$Key"
    }
    $match.Groups[1].Value.Trim()
}

function New-PackArchive {
    param(
        [System.IO.DirectoryInfo]$Source,
        [string]$ArchivePath
    )

    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path -LiteralPath $ArchivePath) {
        Remove-Item -LiteralPath $ArchivePath -Force
    }

    $archive = [System.IO.Compression.ZipFile]::Open(
        $ArchivePath,
        [System.IO.Compression.ZipArchiveMode]::Create
    )
    try {
        $prefix = $Source.FullName.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
        Get-ChildItem -LiteralPath $Source.FullName -Recurse -File -Force | ForEach-Object {
            $relativePath = $_.FullName.Substring($prefix.Length).Replace('\', '/')
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                $archive,
                $_.FullName,
                $relativePath,
                [System.IO.Compression.CompressionLevel]::Optimal
            ) | Out-Null
        }
    }
    finally {
        $archive.Dispose()
    }
}

function Update-CatalogHashes {
    param(
        [string]$Path,
        [hashtable]$Hashes
    )

    $content = Get-Content -LiteralPath $Path -Raw -Encoding UTF8
    $packagePattern = '(?ms)^\[\[Packages\]\].*?(?=^\[\[Packages\]\]|\z)'
    $matches = [regex]::Matches($content, $packagePattern)
    $result = [System.Text.StringBuilder]::new()
    $position = 0
    $updated = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

    foreach ($match in $matches) {
        [void]$result.Append($content.Substring($position, $match.Index - $position))
        $block = $match.Value
        $urlMatch = [regex]::Match($block, '(?mi)^[ \t]*Url[ \t]*=[ \t]*"([^\"]+)"[ \t]*(?=\r?$)')
        if ($urlMatch.Success) {
            $urlPath = $urlMatch.Groups[1].Value.Split('?')[0]
            $archiveName = ($urlPath -split '/')[-1]
            if ($Hashes.ContainsKey($archiveName)) {
                $value = 'sha256:' + $Hashes[$archiveName]
                $shaMatch = [regex]::Match($block, '(?mi)^[ \t]*Sha256[ \t]*=[ \t]*"[^\"]*"[ \t]*(?=\r?$)')
                if ($shaMatch.Success) {
                    $block = $block.Remove($shaMatch.Index, $shaMatch.Length).Insert(
                        $shaMatch.Index,
                        "Sha256 = `"$value`""
                    )
                }
                else {
                    $block = $block.TrimEnd() + "`r`nSha256 = `"$value`"`r`n"
                }
                [void]$updated.Add($archiveName)
            }
        }
        [void]$result.Append($block)
        $position = $match.Index + $match.Length
    }
    [void]$result.Append($content.Substring($position))

    $missing = @($Hashes.Keys | Where-Object { -not $updated.Contains($_) })
    if ($missing.Count -gt 0) {
        throw "mo.toml 中未找到这些 ZIP 的 Packages.Url 条目: $($missing -join ', ')"
    }
    [System.IO.File]::WriteAllText($Path, $result.ToString(), [System.Text.UTF8Encoding]::new($false))
}

$PacksRoot = (Resolve-Path -LiteralPath $PacksRoot).Path
$repositoryRoot = Split-Path -Parent $PacksRoot
if (-not $CatalogPath) {
    $CatalogPath = Join-Path $repositoryRoot "mo.toml"
}
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repositoryRoot "release"
}
if (-not (Test-Path -LiteralPath $CatalogPath -PathType Leaf)) {
    throw "未找到包目录索引: $CatalogPath"
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null

$hashes = @{}
Get-ChildItem -LiteralPath $PacksRoot -Directory | ForEach-Object {
    $configPath = Join-Path $_.FullName "Config.toml"
    if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
        Write-Host "跳过 $($_.Name)：没有 Config.toml" -ForegroundColor Yellow
        return
    }

    $id = Get-ConfigValue -ConfigPath $configPath -Key "Id"
    $archiveName = "$id.zip"
    if ($hashes.ContainsKey($archiveName)) {
        throw "重复的 Config.Id: $id"
    }
    $archivePath = Join-Path $OutputDirectory $archiveName
    New-PackArchive -Source $_ -ArchivePath $archivePath
    $hash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToUpperInvariant()
    $hashes[$archiveName] = $hash
    Write-Host "已打包 $archiveName  sha256:$hash" -ForegroundColor Green
}

if ($hashes.Count -eq 0) {
    throw "未在 $PacksRoot 中找到包含 Config.toml 的包"
}
Update-CatalogHashes -Path $CatalogPath -Hashes $hashes
Write-Host "已更新 $CatalogPath" -ForegroundColor Green
