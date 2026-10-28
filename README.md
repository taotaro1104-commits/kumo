# Kumo — Rust製・省メモリブラウザ（Phase 3：メモリ管理）

設計書 `Rust製・省メモリブラウザ 設計書.pdf` のロードマップ Phase 3 までの実装です。
HTML/CSS → アリーナDOM → カスケード → Taffy + Parley レイアウト → tiny-skia 描画。
タブ・履歴・Network プロセス（rustls・HTTPキャッシュ・コンテンツブロック・Cookie 永続化）を備え、
ページの処理は Content プロセスで非同期に行います（Browser / Content / Network の3プロセス）。
フォーム送信とページ内アンカーにも対応しています。
計画と計測結果は [docs/phase1.md](docs/phase1.md)、プロセス分離は [docs/phase1b.md](docs/phase1b.md)、
JavaScript（Boa）は [docs/phase2.md](docs/phase2.md)、メモリ管理（凍結・破棄・about:memory）は [docs/phase3.md](docs/phase3.md)。

```
Browser（タブ・履歴・UI）──stdin/stdout──▶ Content（全タブのページ、共有フレームへ描画）
   │        ▲ 共有メモリのフレーム ─────────────┘          │
   └──local socket──▶ Network ◀──local socket────────────┘   （接続ごとに共有メモリの本文リング）
```

## 構成

| crate | 役割 |
|---|---|
| `kumo-dom` | アリーナDOM（`u32` ID、ノード本体 ≤32B、ドキュメント単位で一括解放） |
| `kumo-html` | html5ever の `TreeSink` をアリーナDOMへ直接実装、`<style>`/`<link>` 抽出 |
| `kumo-css` | 自前の最小カスケード（cssparser + selectors、`var()`、`@media`、スタイル共有）。`StyleEngine` トレイトで Stylo へ差し替え可能 |
| `kumo-text` | Parley によるシェーピング・改行、swash によるグリフ描画（4MB LRU） |
| `kumo-layout` | ボックスツリー生成、Taffy 低レベルAPI（block/flex/grid/float）、インラインは Parley、表は grid で近似 |
| `kumo-paint` | ディスプレイリスト、tiny-skia CPU ラスタライザ。ウィンドウは前フレームをずらして再利用し、新しく見えた帯だけを描く（`ScrollRenderer`）。タイルキャッシュも併設 |
| `kumo-image` | 表示サイズでのデコード、展開済みビットマップ 8MB LRU |
| `kumo-mem` | タグ付きアロケータ（mimalloc v2）、一時ヒープ、プロセスメモリ計測。`unsafe` はこの crate のみ |
| `kumo-ipc` | 共有メモリ（ファイルのマップ）、リングの領域管理、ローカルソケット。`unsafe` は kumo-mem とこの crate のみ |
| `kumo-net` | Network プロセス：取得（ureq + rustls）、HTTP キャッシュ（メモリ 8MB + ディスク、索引は redb）、コンテンツブロック（adblock）、Cookie の永続化、複数クライアントの受付と本文リング |
| `kumo-js` | JavaScript：Boa に DOM・イベント・タイマー・`fetch`・Web API（URL・storage・XHR など）を結びつける。スクリプトの予算・panic の保護・同一オリジンポリシー |
| `kumo-content` | Content プロセス：取得・パース（別スレッド）→ スタイル・レイアウト → 共有フレームへの描画、フォーム・リンク・スクロールの処理 |
| `kumo-browser` | Browser プロセス：タブ・セッション履歴・閲覧履歴（redb）・ブラウザ UI（自エンジンで描く HTML）。Content へは要求を送るだけで待たない |
| `kumo-viewer` | ブラウザ本体（winit + softbuffer）。同じ実行ファイルを `--network-process`・`--content-process` で子プロセスとして起動 |

## 使い方

```sh
cargo build --release
# ブラウザとして起動（複数の URL はそれぞれタブで開く）
target/release/kumo-viewer "https://en.wikipedia.org/wiki/Rust_(programming_language)" example.com
# ヘッドレス：表示中タブを PNG に保存し、3プロセスのメモリ内訳を表示
target/release/kumo-viewer tests/fixtures/basic.html --headless out.png --width 1280 --height 900 --mem-report
```

| 操作 | キー |
|---|---|
| 新しいタブ / 閉じる | Ctrl+T / Ctrl+W（タブの中クリックでも閉じる） |
| タブ切り替え | Ctrl+Tab / Ctrl+Shift+Tab、タブのクリック |
| URL 欄 | Ctrl+L、入力して Enter（Esc で取り消し） |
| 戻る / 進む / 再読み込み | Alt+← / Alt+→ / F5・Ctrl+R |
| リンク | クリックで移動、Ctrl+クリック・中クリックで背景タブ。`#id` は取得し直さずにスクロール |
| フォーム | 入力欄をクリックして入力、Enter・送信ボタンで送信（GET/POST）。チェックボックス・ラジオ・選択（クリックで次へ） |
| スクロール | ホイール、↑↓、PageUp/PageDown/Space、Home/End |
| メモリ内訳 | Ctrl+M |

### 画面

- **URL 欄**：通信の安全性を左端に示す（🔒 https / ⚠ 保護されていない通信 / 🗋 ファイル）。ドメインを濃く、パスを薄く表示する。
- **読み込み中**：タブに青い ●、URL 欄の下に青い線と「読み込み中…」。Content プロセスが落ちて作り直せないときは赤い表示。
- **マウス**：UI のボタン・タブは乗せると淡く強調。リンク・ボタンの上は指のカーソル、入力欄の上は I ビーム。
  リンクの上では左下にリンク先を表示する。
- **新しいタブ**：よく見るページのカードと、主な操作キーの早見。
- **エラーページ**：原因を分かる言葉で示し（サーバーが見つからない、応答がない、ファイルがない など）、「再読み込み」「戻る」ボタンを置く。
- ページの `border-radius`（角丸の背景・枠線）に対応。フォーム部品も角を丸めて描く。

オプション：`--no-js`（スクリプトを実行しない）、`--in-process`（Content・Network を別プロセスにせずスレッドで動かす）、`--no-disk-cache`（キャッシュ・履歴を保存しない）、
`--memory-budget MB` / `--freeze-after 秒` / `--discard-after 秒`（メモリ管理の方針）、`--filter-list FILE`（EasyList などを追加）。キャッシュと履歴は `%LOCALAPPDATA%\Kumo`（Linux は `~/.cache/kumo`）。
`KUMO_ALLOC_STATS=1` で mimalloc の内部統計も、`KUMO_JS_LOG=1` でスクリプトのコンソール出力・例外・実行時間も出力します。

## 検証

実サイトの互換性の目安：`scripts/compat.sh`（50 サイトを開いて、読み込み・描画・JS の例外・メモリ・時間を判定）。

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo llvm-cov --workspace --summary-only
```

メモリは Windows では Private Working Set（`\Process(kumo-viewer)\Working Set - Private`）、
Linux では PSS（`/proc/<pid>/smaps_rollup`）で計測します。
