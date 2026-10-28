# Windows 向け配布パッケージ（zip）を dist/ に作る。先に `cargo build --release` を済ませておく。
# kumo-viewer.exe は同じ実行ファイルを子プロセス（Network・Content）としても起動するので、これ1つで動く。
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$name = "kumo-$version-windows-x64"
$stage = Join-Path $root "dist\$name"
$exe = Join-Path $root 'target\release\kumo-viewer.exe'
if (-not (Test-Path $exe)) { throw "$exe がありません。先に cargo build --release を実行してください" }

Remove-Item -Recurse -Force (Join-Path $root 'dist') -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force (Join-Path $stage 'docs') | Out-Null
Copy-Item $exe (Join-Path $stage 'kumo.exe')
Copy-Item (Join-Path $root 'README.md') $stage
Copy-Item (Join-Path $root 'docs\*.md') (Join-Path $stage 'docs')

$notes = @"
Kumo $version（Windows x64）

起動: kumo.exe [URL ...]        例: kumo.exe https://example.com/
ヘルプ: kumo.exe --help
設定: 起動して about:settings（言語・ホーム・読み込みの負荷・画像・パスワード）
インストールは不要です。フォルダごと好きな場所に置いてください。
履歴・設定は %LOCALAPPDATA%\kumo に保存されます。パスワードの暗号鍵は Windows の資格情報マネージャにあります。
ライセンス: MIT OR Apache-2.0
"@
Set-Content -Path (Join-Path $stage 'はじめに.txt') -Value $notes -Encoding UTF8

$zip = Join-Path $root "dist\$name.zip"
Compress-Archive -Path $stage -DestinationPath $zip
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
Set-Content -Path "$zip.sha256" -Value "$hash  $name.zip" -Encoding ASCII
Get-Item $zip | Select-Object FullName, @{n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
Write-Output "sha256: $hash"
