# Lite Simple Rust Browser

**日本語** | [English](#english)

紹介サイト: <https://originapl.matchinfo-rank.com/>

## 軽くて、シンプル。Rust で作った Web ブラウザ。

メモリの少なさを一番に考えて Rust で一から書いた、軽くてシンプルな Web ブラウザです（内部名 Kumo）。HTML・CSS の解釈から、レイアウト、描画、通信、JavaScript の実行まで自前のエンジンで行い、タブを何枚開いても軽く動きます。

### メモリ使用量（リリースビルドでの実測）

| 場面 | 使用量 |
|---|---|
| 空のタブ | 約 22 MB |
| Wikipedia の記事 1 枚 | 約 65 MB |
| 10 タブ | 約 185 MB |

### 特長

- **省メモリ** — 背景のタブは自動で凍結・破棄し、メモリの予算に収めます。
- **3 つのプロセス** — Browser・Content・Network を分離。ページが重くても画面は固まらず、Content が落ちても自動で立て直します。
- **JavaScript** — Boa エンジンで DOM・イベント・タイマー・fetch などを動かします。
- **負荷レベル 1〜5** — 「最軽量」から「最大」まで 5 段階。画像の読み込みは別に切り替えられます。
- **パスワードマネージャ** — ログイン時に保存を確認し、同じサイトで自動入力。パスワードは暗号化し、鍵は Windows の資格情報マネージャに置きます。
- **ブックマーク** — Ctrl+D で追加、Ctrl+B で一覧。ホームページも設定で決められます。
- **閲覧履歴・ダウンロード** — Ctrl+H で履歴、Ctrl+J でダウンロード一覧。ファイルへのリンクはページを離れずに保存し、進み具合を表示します。
- **拡大・縮小** — Ctrl+`+` / Ctrl+`-` / Ctrl+ホイールで 25〜500%、Ctrl+0 で元の大きさ。倍率はタブごとに持ち、URL 欄の右に表示します。
- **日本語入力** — IME で変換した文字を URL 欄や入力欄に入れられます。変換中の文字は URL 欄に表示します。
- **5 か国語の表示** — 日本語・English・简体中文・한국어・Español。
- **コンテンツブロック** — フィルタリストで広告などの読み込みを止められます。

### 使い方

- 起動すると新しいタブが開きます。上の欄に URL か調べたい言葉を入力します（Ctrl+L）。
- Ctrl+T 新しいタブ / Ctrl+W 閉じる / Alt+← 戻る / Alt+Home ホーム / Ctrl+クリック 背景のタブで開く
- 設定は `about:settings`、パスワードは `about:passwords`、メモリの状況は `about:memory`。

### 入手方法

Windows 11（64 ビット）用です。インストーラーを実行するか、zip を展開するだけで使えます。履歴と設定は `%LOCALAPPDATA%\Kumo` に保存されます。ライセンスは MIT または Apache-2.0 から選べます。

- [Google ドライブからダウンロード](https://drive.google.com/drive/folders/1QNuxUUjx4ZFg1fEMYfnRhtGv99VOiIKc?usp=drive_link)
- [ソースコード（GitHub）](https://github.com/taotaro1104-commits/kumo)

### 現在の制限

開発中のため、すべてのサイトが完全に表示されるわけではありません。ES モジュール、iframe、canvas、GPU 描画、インラインの SVG などは未対応です。

---

<a id="english"></a>

[日本語](#lite-simple-rust-browser) | **English**

Website: <https://originapl.matchinfo-rank.com/>

## Light and simple. A web browser written in Rust.

A light, simple web browser written from scratch in Rust with low memory use as its first goal (internal name: Kumo). It parses HTML and CSS, lays out, paints, fetches and runs JavaScript with its own engine, so it stays light however many tabs you open.

### Memory use (measured on a release build)

| Case | Memory |
|---|---|
| Blank tab | ~22 MB |
| One Wikipedia article | ~65 MB |
| Ten tabs | ~185 MB |

### Features

- **Low memory** — Background tabs are frozen and discarded automatically to stay within a memory budget.
- **Three processes** — Browser, Content and Network run separately. The window stays responsive on heavy pages, and Content restarts itself if it stops.
- **JavaScript** — Runs the DOM, events, timers, fetch and more on the Boa engine.
- **Load levels 1-5** — From "Minimal" to "Maximum". Image loading can be switched on or off separately.
- **Password manager** — Asks to save when you sign in and fills it in next time. Passwords are encrypted, with the key kept in the Windows Credential Manager.
- **Bookmarks** — Ctrl+D to add, Ctrl+B for the list. Choose your home page in Settings.
- **History and downloads** — Ctrl+H for history, Ctrl+J for downloads. Links to files are saved without leaving the page, with live progress.
- **Page zoom** — Ctrl+`+` / Ctrl+`-` / Ctrl+wheel from 25% to 500%, Ctrl+0 to reset. The zoom is kept per tab and shown beside the address bar.
- **Japanese input (IME)** — Text converted with an IME goes into the address bar and page fields, with the composing text shown in the address bar.
- **Five display languages** — Japanese, English, Simplified Chinese, Korean and Spanish.
- **Content blocking** — Block ads and other unwanted requests with filter lists.

### Getting started

- A new tab opens at startup. Type a URL or a search in the box at the top (Ctrl+L).
- Ctrl+T new tab / Ctrl+W close / Alt+Left back / Alt+Home home / Ctrl+click open in background
- Settings: `about:settings`, passwords: `about:passwords`, memory: `about:memory`.

### Get it

For Windows 11 (64-bit). Run the installer, or just unzip the package. History and settings are stored in `%LOCALAPPDATA%\Kumo`. Licensed under your choice of MIT or Apache-2.0.

- [Download from Google Drive](https://drive.google.com/drive/folders/1QNuxUUjx4ZFg1fEMYfnRhtGv99VOiIKc?usp=drive_link)
- [Source code (GitHub)](https://github.com/taotaro1104-commits/kumo)

### Current limitations

This is a work in progress, so not every site renders perfectly. ES modules, iframes, canvas, GPU rendering and inline SVG are not supported yet.
