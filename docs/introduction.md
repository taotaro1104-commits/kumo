# Lite Simple Rust Browser

**日本語** | [English](#english)

## Rust で作った、軽くてシンプルな Web ブラウザ

### 概要

Lite Simple Rust Browser（内部名 Kumo）は、メモリの少なさを一番に考えて Rust で一から書いた Web ブラウザです。HTML・CSS の解釈から、レイアウト、描画、通信、JavaScript の実行までを自前のエンジンで行い、タブを何枚開いても軽く動くことを目指しています。

### 特長

- **省メモリ** — 空のタブで約 22MB、Wikipedia の記事 1 枚で約 65MB、10 タブで約 185MB（リリースビルドでの実測）。背景のタブは自動で凍結・破棄し、メモリの予算に収めます。
- **3 つのプロセス** — Browser・Content・Network を別のプロセスに分け、ページの処理が重くても画面は固まりません。Content が落ちても自動で立て直します。
- **JavaScript** — Boa エンジンで DOM・イベント・タイマー・fetch などを動かします。負荷レベルに応じて実行の予算を調整できます。
- **負荷レベル（1〜5）** — 「最軽量」（HTML と内蔵 CSS だけ）から「最大」まで 5 段階。画像の読み込みは別に切り替えられます。
- **パスワードマネージャ** — ログイン時に保存を確認し、同じサイトで自動入力。パスワードは暗号化し、鍵は Windows の資格情報マネージャに置きます。
- **ブックマーク・ホームページ** — Ctrl+D で追加、Ctrl+B で一覧。ホームページは設定で決められます。
- **閲覧履歴・ダウンロード** — Ctrl+H で閲覧履歴（1 件ずつ、またはまとめて削除）、Ctrl+J でダウンロードの一覧。ファイルへのリンクを開くと、ページを離れずに「ダウンロード」フォルダへ保存し、進み具合を見せます。
- **5 か国語の表示** — 日本語・English・简体中文・한국어・Español。設定画面、新しいタブ、エラーページなど Kumo 自身の画面が切り替わります。
- **コンテンツブロック** — フィルタリストで広告などの読み込みを止められます。

### 使い方

- 起動すると新しいタブが開きます。上の欄に URL か調べたい言葉を入力してください（Ctrl+L）。
- 主なキー: Ctrl+T（新しいタブ）、Ctrl+W（閉じる）、Alt+←（戻る）、Alt+Home（ホーム）、Ctrl+クリック（背景のタブで開く）。
- 設定は `about:settings`、保存したパスワードは `about:passwords`、メモリの状況は `about:memory` で見られます。
- コマンドラインから開くとき: `kumo.exe https://example.com/`（`--help` で一覧）。

### 現在の制限

開発中のため、すべてのサイトが完全に表示されるわけではありません。ES モジュール、iframe、canvas、GPU 描画、インラインの SVG などは未対応です。起動時の表示言語は日本語で、ログイン情報の「表示」にマスターパスワードはありません。

### 動作環境とライセンス

Windows 11（64 ビット）。インストーラーまたは zip を展開するだけで使えます。履歴と設定は `%LOCALAPPDATA%\Kumo` に保存されます。ライセンスは MIT または Apache-2.0 のどちらかを選べます。

---

<a id="english"></a>

[日本語](#lite-simple-rust-browser) | **English**

## A light, simple web browser written in Rust

### Overview

Lite Simple Rust Browser (internal name: Kumo) is a web browser written from scratch in Rust with low memory use as its first goal. It parses HTML and CSS, lays out, paints, fetches over the network and runs JavaScript with its own engine, so it stays light however many tabs you open.

### Features

- **Low memory** — About 22 MB for a blank tab, about 65 MB for one Wikipedia article and about 185 MB for ten tabs (measured on a release build). Background tabs are frozen and discarded automatically to stay within a memory budget.
- **Three processes** — Browser, Content and Network run as separate processes, so the window stays responsive even when a page is heavy. If the Content process stops, it is restarted automatically.
- **JavaScript** — Runs the DOM, events, timers, fetch and more on the Boa engine, with a script budget that follows the load level.
- **Load levels 1-5** — From "Minimal" (HTML and inline CSS only) to "Maximum". Image loading can be switched on or off separately.
- **Password manager** — Asks to save when you sign in and fills it in next time on the same site. Passwords are encrypted, and the key is kept in the Windows Credential Manager.
- **Bookmarks and home page** — Ctrl+D to add, Ctrl+B for the list. Choose your home page in Settings.
- **History and downloads** — Ctrl+H for your browsing history (delete one entry or clear all), Ctrl+J for the download list. Opening a link to a file saves it to your Downloads folder without leaving the page, with live progress.
- **Five display languages** — Japanese, English, Simplified Chinese, Korean and Spanish. The browser's own screens (settings, new tab, error pages) switch language.
- **Content blocking** — Block ads and other unwanted requests with filter lists.

### Getting started

- A new tab opens at startup. Type a URL or something to search for in the box at the top (Ctrl+L).
- Main keys: Ctrl+T (new tab), Ctrl+W (close), Alt+Left (back), Alt+Home (home), Ctrl+click (open in a background tab).
- See `about:settings` for settings, `about:passwords` for saved passwords and `about:memory` for memory use.
- From the command line: `kumo.exe https://example.com/` (use `--help` for the options).

### Current limitations

This is a work in progress, so not every site renders perfectly. ES modules, iframes, canvas, GPU rendering and inline SVG are not supported yet. The display language at startup is always Japanese, and showing a saved password does not require a master password.

### Requirements and license

Windows 11 (64-bit). Use the installer, or just unzip the package. History and settings are stored in `%LOCALAPPDATA%\Kumo`. Licensed under your choice of MIT or Apache-2.0.
