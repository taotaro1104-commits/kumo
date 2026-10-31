---
title: "Lite Simple Rust Browser"
description: "メモリの少なさを一番に考えて Rust で一から書いた、軽くてシンプルな Web ブラウザ。"
tagline: "軽くて、シンプル。Rust で作った Web ブラウザ。"
lead: "HTML・CSS の解釈から、レイアウト、描画、通信、JavaScript の実行まで自前のエンジンで行い、タブを何枚開いても軽く動きます。"
nav_all: "アプリ一覧"
nav_features: "特長"
nav_get: "入手方法"
cta_source: "ソースコード"
ad_label: "広告"
stats_title: "メモリ使用量（リリースビルドでの実測）"
stats:
  - { mb: 22, value: "約 22 MB", label: "空のタブ" }
  - { mb: 65, value: "約 65 MB", label: "Wikipedia の記事 1 枚" }
  - { mb: 185, value: "約 185 MB", label: "10 タブ" }
features_title: "特長"
features:
  - { icon: "🍃", title: "省メモリ", body: "背景のタブは自動で凍結・破棄し、メモリの予算に収めます。" }
  - { icon: "🔀", title: "3 つのプロセス", body: "Browser・Content・Network を分離。ページが重くても画面は固まらず、Content が落ちても自動で立て直します。" }
  - { icon: "⚡", title: "JavaScript", body: "Boa エンジンで DOM・イベント・タイマー・fetch などを動かします。" }
  - { icon: "🎚️", title: "負荷レベル 1〜5", body: "「最軽量」から「最大」まで 5 段階。画像の読み込みは別に切り替えられます。" }
  - { icon: "🔑", title: "パスワードマネージャ", body: "ログイン時に保存を確認し、同じサイトで自動入力。パスワードは暗号化し、鍵は Windows の資格情報マネージャに置きます。" }
  - { icon: "⭐", title: "ブックマーク", body: "Ctrl+D で追加、Ctrl+B で一覧。ホームページも設定で決められます。" }
  - { icon: "🕘", title: "閲覧履歴・ダウンロード", body: "Ctrl+H で履歴、Ctrl+J でダウンロード一覧。ファイルへのリンクはページを離れずに保存し、進み具合を表示します。" }
  - { icon: "🔍", title: "拡大・縮小", body: "Ctrl+「+」「-」やCtrl+ホイールで 25〜500%。Ctrl+0 で元の大きさに戻ります。" }
  - { icon: "あ", title: "日本語入力", body: "IME で変換した文字を URL 欄や入力欄に入れられます。" }
  - { icon: "🌏", title: "5 か国語の表示", body: "日本語・English・简体中文・한국어・Español。" }
  - { icon: "🛡️", title: "コンテンツブロック", body: "フィルタリストで広告などの読み込みを止められます。" }
start_title: "使い方"
start_lead: "起動すると新しいタブが開きます。上の欄に URL か調べたい言葉を入力してください。"
keys:
  - { k: "Ctrl L", d: "アドレスバーへ" }
  - { k: "Ctrl T", d: "新しいタブ" }
  - { k: "Ctrl W", d: "タブを閉じる" }
  - { k: "Ctrl D", d: "ブックマークに追加" }
  - { k: "Ctrl H", d: "閲覧履歴" }
  - { k: "Ctrl J", d: "ダウンロード一覧" }
  - { k: "Ctrl +/-", d: "拡大・縮小" }
start_note: "設定は about:settings、パスワードは about:passwords、メモリの状況は about:memory で見られます。"
download_title: "入手方法"
download_button: "Google ドライブからダウンロード"
recommended: "おすすめ"
sha_summary: "チェックサム（SHA-256）を見る"
download_body: "Windows 11（64 ビット）用です。インストーラーを実行するか、zip を展開するだけで使えます。履歴と設定は %LOCALAPPDATA%\\Kumo に保存されます。ライセンスは MIT または Apache-2.0 から選べます。 Android 8.0 以上の端末に入れられます（提供元不明のアプリの許可が必要）。"
release_title: "最新版"
release_version: "バージョン 0.1.0（2026-10-29）"
release_files:
  - { name: "kumo-0.1.0-windows-x64-setup.exe", kind: "インストーラー", size: "8.8 MB", sha: "2f2a017fdaf6d8f85c7e70e497facdd4a00bfc2f6081c84974105158da1d1e84" }
  - { name: "kumo-0.1.0-windows-x64.zip", kind: "zip（展開するだけで使えます）", size: "10.4 MB", sha: "d78c7542a9a61225636428c30e1b2fc8d30df193cc19554963bbc030317bdba8" }
  - { name: "kumo-0.1.0-android-arm64.apk", kind: "Android アプリ（arm64）", size: "12.1 MB", sha: "68bb339dc891b375dd2fcc8f3ae131e8fa8a32a42c1517e2316f92eda4b29478" }
sha_note: "ダウンロードしたファイルの SHA-256 が下の値と同じか確かめられます。"
changes_title: "最近の更新"
changes:
  - "Android 版を追加（タッチでスクロール・タップ・つまみ操作、ソフトキーボード、変換入力、パスワード保存）"
  - "ページの拡大・縮小（Ctrl+「+」「-」、Ctrl+ホイール、Ctrl+0）を追加"
  - "日本語入力（IME）に対応。変換中の文字は URL 欄に表示"
  - "閲覧履歴（Ctrl+H）とダウンロード一覧（Ctrl+J）を追加。ファイルへのリンクはページを離れずに保存"
  - "アプリ名を Lite Simple Rust Browser にし、独自アイコンを追加"
  - "パスワードマネージャと 5 か国語の表示を追加"
limits_title: "現在の制限"
limits_body: "開発中のため、すべてのサイトが完全に表示されるわけではありません。ES モジュール、iframe、canvas、GPU 描画、インラインの SVG などは未対応です。 Android 版は試験的です。変換入力はキーボードアプリ側の対応に依存し、ページ内の入力欄では変換中の文字が表示されません。"
shop_title: "おすすめ"
shop_lead: "パソコンや周辺機器、ふだんの買い物はこちらから探せます。"
rakuten_label: "楽天市場で探す"
amazon_label: "Amazon.co.jp で探す"
shop_disclosure: "当サイトは広告（Google AdSense）とアフィリエイトプログラム（楽天アフィリエイト・Amazon アソシエイト）を利用しています。Amazon のアソシエイトとして、当サイトは適格販売により収入を得ています。"
---
