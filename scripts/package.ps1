param(
    [string]$ReleaseTag,
    [string]$ReleaseRepository = 'MeccBai/IniPackManager'
)

$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$signingKey = Join-Path $projectRoot 'tauri-updater'
$bundleRoot = Join-Path $projectRoot 'src-tauri\target\release\bundle'
$outputRoot = Join-Path $projectRoot 'out'
$cargoManifest = Join-Path $projectRoot 'src-tauri\Cargo.toml'

if (-not (Test-Path -LiteralPath $signingKey -PathType Leaf)) {
    throw "未找到 Tauri Updater 私钥: $signingKey"
}

$env:TAURI_SIGNING_PRIVATE_KEY = (Resolve-Path -LiteralPath $signingKey).Path
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''

$versionLine = Get-Content -LiteralPath $cargoManifest |
    Select-String -Pattern '^version\s*=\s*"(?<version>[^"]+)"' |
    Select-Object -First 1
if (-not $versionLine) {
    throw "无法从 $cargoManifest 读取应用版本"
}
$version = $versionLine.Matches[0].Groups['version'].Value
if ([string]::IsNullOrWhiteSpace($ReleaseTag)) {
    $ReleaseTag = "v$version"
}

Push-Location $projectRoot
try {
    & npm run tauri build
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri 打包失败，退出码: $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

$artifacts = @(
    (Join-Path $bundleRoot "msi\ini_pack_manager_${version}_x64_en-US.msi"),
    (Join-Path $bundleRoot "msi\ini_pack_manager_${version}_x64_en-US.msi.sig"),
    (Join-Path $bundleRoot "nsis\ini_pack_manager_${version}_x64-setup.exe"),
    (Join-Path $bundleRoot "nsis\ini_pack_manager_${version}_x64-setup.exe.sig")
)

$missing = $artifacts | Where-Object { -not (Test-Path -LiteralPath $_ -PathType Leaf) }
if ($missing) {
    throw "打包完成但缺少产物:`n$($missing -join "`n")"
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
Copy-Item -LiteralPath $artifacts -Destination $outputRoot -Force

$updaterFileName = "ini_pack_manager_${version}_x64-setup.exe"
$updaterManifest = [ordered]@{
    version = $version
    notes = "Ini Pack Manager $version"
    pub_date = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = (Get-Content -LiteralPath $artifacts[3] -Raw).Trim()
            url = "https://github.com/$ReleaseRepository/releases/download/$ReleaseTag/$updaterFileName"
        }
    }
}
$latestJson = Join-Path $outputRoot 'latest.json'
$updaterManifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $latestJson -Encoding utf8

Write-Host "打包和 OTA 签名已完成，产物已复制到: $outputRoot"
Write-Host "Updater 清单已生成: $latestJson"
Get-ChildItem -LiteralPath $outputRoot -File |
    Where-Object { $_.Name -match "^ini_pack_manager_$([regex]::Escape($version))_" } |
    Select-Object Name, Length, LastWriteTime
