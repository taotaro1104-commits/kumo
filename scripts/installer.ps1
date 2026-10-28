# Windows のインストーラー（dist/kumo-<版>-windows-x64-setup.exe）を作る。
# 先に `cargo build --release` を済ませておく。Inno Setup 6 が必要（`winget install JRSoftware.InnoSetup`）。
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$exe = Join-Path $root 'target\release\kumo-viewer.exe'
if (-not (Test-Path $exe)) { throw "$exe がありません。先に cargo build --release を実行してください" }

$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 が見つかりません（winget install JRSoftware.InnoSetup）' }

# 入れるファイルを dist/stage にそろえる
$stage = Join-Path $root 'dist\stage'
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force (Join-Path $stage 'docs') | Out-Null
Copy-Item $exe (Join-Path $stage 'kumo.exe')
Copy-Item (Join-Path $root 'README.md') $stage
Copy-Item (Join-Path $root 'docs\*.md') (Join-Path $stage 'docs')
$license = @"
Kumo is dual-licensed under either of

  * Apache License, Version 2.0
  * MIT license

at your option. (MIT OR Apache-2.0)
Kumo は MIT または Apache-2.0 のどちらかを選んで使えます。
"@
Set-Content -Path (Join-Path $stage 'LICENSE.txt') -Value $license -Encoding UTF8

& $iscc "/DAppVersion=$version" "/DSourceDir=$stage" (Join-Path $root 'installer\kumo.iss')
if ($LASTEXITCODE -ne 0) { throw "ISCC が失敗しました ($LASTEXITCODE)" }

$setup = Join-Path $root "dist\kumo-$version-windows-x64-setup.exe"
$hash = (Get-FileHash $setup -Algorithm SHA256).Hash.ToLower()
Set-Content -Path "$setup.sha256" -Value "$hash  $(Split-Path -Leaf $setup)" -Encoding ASCII
Remove-Item -Recurse -Force $stage
Get-Item $setup | Select-Object FullName, @{n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
Write-Output "sha256: $hash"
