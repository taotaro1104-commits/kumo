# Kumo — Rust製・省メモリブラウザ（Phase 0：静的ビューア）

設計書 `Rust製・省メモリブラウザ 設計書.pdf` のロードマップ Phase 0 の実装です。
HTML/CSS → アリーナDOM → カスケード → Taffy + Parley レイアウト → tiny-skia タイル描画（JSなし）。

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
| `kumo-viewer` | winit + softbuffer のビューア / ヘッドレスPNG出力 |

## 使い方

```sh
cargo build --release
# ウィンドウ表示（↑↓/PageUp/PageDown/Space/Home/End でスクロール、m でメモリ内訳、Esc で終了）
target/release/kumo-viewer "https://en.wikipedia.org/wiki/Rust_(programming_language)"
# ヘッドレス：PNG に保存してメモリ内訳を表示
target/release/kumo-viewer tests/fixtures/basic.html --headless out.png --width 1280 --height 900 --mem-report
```

`KUMO_ALLOC_STATS=1` を付けると mimalloc の内部統計（コミット量・断片化）も出力します。

## 検証

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo llvm-cov --workspace --summary-only
```

メモリは Windows では Private Working Set（`\Process(kumo-viewer)\Working Set - Private`）、
Linux では PSS（`/proc/<pid>/smaps_rollup`）で計測します。
