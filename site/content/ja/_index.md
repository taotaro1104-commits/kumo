---
title: "Lite Simple Rust Browser"
description: "メモリの少なさを一番に考えて Rust で一から書いた、軽くてシンプルな Web ブラウザ。"
tagline: "軽くて、シンプル。Rust で作った Web ブラウザ。"
lead: "HTML・CSS の解釈から、レイアウト、描画、通信、JavaScript の実行まで自前のエンジンで行い、タブを何枚開いても軽く動きます。"
cta_download: "入手方法"
cta_source: "ソースコード"
stats_title: "メモリ使用量（リリースビルドでの実測）"
stats:
  - { value: "約 22 MB", label: "空のタブ" }
  - { value: "約 65 MB", label: "Wikipedia の記事 1 枚" }
  - { value: "約 185 MB", label: "10 タブ" }
features_title: "特長"
features:
  - { title: "省メモリ", body: "背景のタブは自動で凍結・破棄し、メモリの予算に収めます。" }
  - { title: "3 つのプロセス", body: "Browser・Content・Network を分離。ページが重くても画面は固まらず、Content が落ちても自動で立て直します。" }
  - { title: "JavaScript", body: "Boa エンジンで DOM・イベント・タイマー・fetch などを動かします。" }
  - { title: "負荷レベル 1〜5", body: "「最軽量」から「最大」まで 5 段階。画像の読み込みは別に切り替えられます。" }
  - { title: "パスワードマネージャ", body: "ログイン時に保存を確認し、同じサイトで自動入力。パスワードは暗号化し、鍵は Windows の資格情報マネージャに置きます。" }
  - { title: "ブックマーク", body: "Ctrl+D で追加、Ctrl+B で一覧。ホームページも設定で決められます。" }
  - { title: "閲覧履歴・ダウンロード", body: "Ctrl+H で履歴、Ctrl+J でダウンロード一覧。ファイルへのリンクはページを離れずに保存し、進み具合を表示します。" }
  - { title: "5 か国語の表示", body: "日本語・English・简体中文・한국어・Español。" }
  - { title: "コンテンツブロック", body: "フィルタリストで広告などの読み込みを止められます。" }
start_title: "使い方"
start:
  - "起動すると新しいタブが開きます。上の欄に URL か調べたい言葉を入力します（Ctrl+L）。"
  - "Ctrl+T 新しいタブ / Ctrl+W 閉じる / Alt+← 戻る / Alt+Home ホーム / Ctrl+クリック 背景のタブで開く"
  - "設定は about:settings、パスワードは about:passwords、メモリの状況は about:memory。"
download_button: "Google ドライブからダウンロード"
download_title: "入手方法"
download_body: "Windows 11（64 ビット）用です。インストーラーを実行するか、zip を展開するだけで使えます。履歴と設定は %LOCALAPPDATA%\\Kumo に保存されます。ライセンスは MIT または Apache-2.0 から選べます。"
limits_title: "現在の制限"
limits_body: "開発中のため、すべてのサイトが完全に表示されるわけではありません。ES モジュール、iframe、canvas、GPU 描画、インラインの SVG などは未対応です。"
---
