# Phase 1：ブラウザの骨格 — 実装計画

設計書ロードマップ Phase 1：タブ、履歴、Network プロセス、rustls、ディスクキャッシュ、コンテンツブロック。
**完了条件：10 タブで 250MB 以下（JS なし）。** 計測は Browser＋Network の全プロセス合計。

## 方針と Phase 1 での簡略化

| 設計書 | Phase 1 の実装 | 後続フェーズ |
|---|---|---|
| Network プロセス（全タブで共有） | 同じ実行ファイルを `--network-process` で子プロセス起動。標準入出力パイプ上の長さ付き postcard メッセージで通信 | 大きな本文を共有メモリのリングバッファで渡す |
| HTTP/1.1・2・3、rustls | ureq（rustls、HTTP/1.1）。Network プロセス内のワーカースレッドで並行取得 | hyper/h2/quinn |
| HTTP キャッシュ（メモリ 8MB、残りはディスク） | メモリ LRU 8MB ＋ ディスク（本文はファイル、索引は redb）。`Cache-Control: max-age/no-store`、`ETag`/`Last-Modified` の再検証 | Vary など詳細対応 |
| Cookie | Network プロセス内のメモリ上の cookie jar | redb への永続化 |
| コンテンツブロック（adblock、シリアライズ済み） | adblock crate。既定の小さなフィルタ＋任意の EasyList ファイル。構築済みエンジンをシリアライズしてディスクに保存し、次回起動時に読み込む | — |
| Content プロセス（1〜N） | Phase 1 では Browser プロセス内（タブごとに DOM・ディスプレイリストを持つ） | Phase 3 以降で分離 |
| タブ・履歴・ブラウザUI | タブ・前後移動・履歴DB（redb）。UI（タブ列・URL欄）は自エンジンで HTML/CSS として描画 | — |

フォントDB（`TextSystem`）とグリフキャッシュは全タブで1つを共有する（設計書の「全タブでグリフキャッシュを共有」）。

## crate 構成（追加分）

- `kumo-net`：プロトコル型・フレーミング、Network サービス本体（取得・キャッシュ・ブロック）、子プロセス側ループ、Browser 側クライアント
- `kumo-browser`：タブ・セッション履歴・履歴DB・UI の HTML 生成と操作の解釈
- `kumo-viewer`：上記を束ねる実行ファイル（`--network-process` で子プロセスとして動く）

## 段階

1. `kumo-net` プロトコルとフレーミング（テスト：往復・壊れた入力）
2. Network サービス：取得・メモリ/ディスクキャッシュ・adblock（テスト：ローカル HTTP サーバでキャッシュ・再検証・ブロック）
3. 子プロセス化とクライアント（テスト：実プロセス間で取得）
4. `kumo-browser`：タブ・前後移動・履歴DB（テスト：単体）
5. UI：タブ列と URL 欄、リンクのクリック、キーボード操作。ディスプレイリストにリンク領域を記録して当たり判定
6. 計測：URL を10個並べて起動し（`--headless --mem-report`）、全プロセス合計を計測

## 操作

Ctrl+T 新規タブ / Ctrl+W 閉じる / Ctrl+Tab・Ctrl+Shift+Tab 切替 / Ctrl+L URL欄 / Enter 移動 /
Alt+←→ 戻る・進む / リンクのクリックで移動、Ctrl+クリックで新規タブ

## 結果（2026-10-27）

計測：リリースビルド、1280×900、Windows 11。値は Browser・Network 両プロセスの private commit の合計（`--mem-report` の `[total]`）。

| シナリオ | 結果 | 目標 |
|---|---|---|
| 空白タブ（起動直後） | 13.8MB（Browser 7.3 + Network 6.5） | 30MB |
| 一般的なサイト 10 タブ（9 タブは背景） | 約 91MB（Browser 61 + Network 30） | 250MB |

10 タブ：英語版・日本語版 Wikipedia ×3、rust-lang.org、Hacker News、MDN、docs.rs、GitHub、BBC News、example.com。

### 計測で見つけて直した点

- **mimalloc v2 の先行コミット**：既定では最初の 32MiB セグメントを起動直後に丸ごとコミットしていた。
  `configure_for_low_memory()` で設定直後に回収し、起動時のコミットを約 33MB → 約 1MB にした。
- **接続プール**：ureq の接続バッファ（既定 128KB×2）を 32KB に縮め、保持する接続を 6 本までにした。
- **スレッドのヒープ**：Network のワーカーと Browser 側の受信スレッドで、1 件ごとに空きページを返す。
  （1 件ごとに一時ヒープを作る方式は mimalloc v2 で逆にコミットが 300MB 超に膨らんだため不採用）

### 残っていた簡略化（Phase 1b で解消）

- 読み込みが同期 → 非同期（Content の取得スレッド）。[phase1b.md](phase1b.md)
- Content プロセス未分離 → 分離（全タブを1つの Content プロセスに同居）。
- 本文の受け渡しがパイプ → 16KB 以上は接続ごとの共有メモリのリング。
- Cookie がメモリのみ → `cache_dir/cookies.json` に永続化（セッション Cookie は保存しない）。
- アンカー・フォーム → `#id` へのスクロール（戻る・進むでも取得し直さない）、フォームの入力と GET/POST 送信。
