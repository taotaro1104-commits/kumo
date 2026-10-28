# Phase 2：JavaScript

設計書ロードマップ Phase 2：Boa 統合、DOM バインディング、イベント、fetch、タイマー。
**完了条件：主要 50 サイト中 35 サイトが実用可能。**

## 構成

```
Content プロセス
 ├─ 取得スレッド（Boa に触れない）：文書・CSS・画像・スクリプトの取得と HTML の解析、スクリプトと fetch の取得
 └─ エンジンスレッド（大きなスタック）：JS の実行、スタイル、レイアウト、描画
      └─ Page ── Rc<RefCell<Document>> ──┬─ スタイル・レイアウト（読むだけ）
                                         └─ JsRuntime（kumo-js、Boa）
```

- **`kumo-js`**（新規）：Boa 0.22 のコンテキストに DOM・イベント・タイマー・`fetch` を結びつける。公開 API は `JsRuntime`。
  DOM は Content が持つ `Rc<RefCell<Document>>` を共有し、JS からの変更は「変わった」印を立てるだけ。
  再スタイル・再レイアウトは Content が描画の前にまとめて 1 回行う。
- **JS が必要なページだけランタイムを作る**：実行できる `<script>` か、`onclick="…"` などのイベント属性があるページ。
  スクリプトのないページは Phase 1b と同じメモリのまま（空白タブ 21.7MB）。
- 既定は JS 有効。`--no-js`（`ToContent::Settings`）で切れる。

## 実装した Web API

| 領域 | 内容 |
|---|---|
| DOM | `document`・`Node`・`Element`・`Text`・`DocumentFragment`、木の操作（`appendChild`・`insertBefore`・`replaceChild`・`append`・`before`・`remove`・`cloneNode` など）、`querySelector(All)`・`matches`・`closest`（kumo-css のセレクタ）、`innerHTML`・`outerHTML`・`insertAdjacentHTML`、`getElementsBy*`、`document.implementation.createHTMLDocument` |
| 要素の属性 | `getAttribute` などと、`id`・`className`・`href`（絶対 URL）・`value`・`checked`・`disabled`・`hidden` など、`classList`・`dataset`・`style`（任意のプロパティ名を Proxy で扱う） |
| イベント | 捕捉 → 目標 → バブリングの 3 段階、`once`・`stopPropagation`・`stopImmediatePropagation`・`preventDefault`、`onclick` プロパティと `onclick="…"` 属性、`Event`・`CustomEvent`・`MouseEvent`・`KeyboardEvent` など（サブクラス化可）、`new EventTarget()` |
| 利用者の操作 | クリック（対象の要素へ）、キー入力（`keydown`・`input`）、`change`、`submit`。`preventDefault` でリンクの移動・フォーム送信を止められる。`DOMContentLoaded`・`load`・`readystatechange` |
| タイマー | `setTimeout`・`setInterval`・`requestAnimationFrame`・`requestIdleCallback`・`queueMicrotask`。表示中のタブは毎回、背景のタブは 1 秒に 1 回 |
| ネットワーク | `fetch`（`Request`・`Response`・`Headers`・`AbortController`）、`XMLHttpRequest`（fetch の上）、動的に挿入した `<script src>` |
| ブラウザの環境 | `window`・`location`（`href` の代入で移動）・`history`・`navigator`・`screen`・`performance`・`localStorage`・`sessionStorage`（ページの間だけ）・`document.cookie`（ページの間だけ）・`matchMedia`・`getComputedStyle`（インラインのスタイル）・`URL`・`URLSearchParams`・`TextEncoder`・`atob`/`btoa`・`structuredClone`・`MutationObserver`・`IntersectionObserver`・`ResizeObserver`（見た目の通知はしない） |
| 古い互換 | Annex B（`substr`・`escape`・`__proto__` など）、`new Image()`・`new Option()` |

プレリュード（`crates/kumo-js/src/prelude.js`）に JS で書いた部分が多く、ネイティブは DOM・イベント・タイマー・取得の入口だけ。

## 安全性と資源の制限

| 対象 | 対策 |
|---|---|
| 終わらないループ | 1 つのループの反復を 2000 万回までにし、超えたら例外 |
| 巨大なスクリプト | 1 つ 512KB まで、1 ページの合計 1.5MB・実行時間 2 秒の予算。超えたものは実行しない（`[budget]` として記録） |
| エンジンの panic | `catch_unwind` で捕まえて内部エラーとして記録。3 回で、そのページの JS を止める（タブ・他のページは動き続ける） |
| スタック溢れ | エンジンスレッドは 256MB（使った分だけコミット）、取得スレッドは 32MB |
| ローカルのファイル | 遠隔のページから `file:` へは、移動・`fetch`・リンクのどれでも行けない |
| 別オリジンへの `fetch` | 同一オリジンポリシー：資格情報（Cookie）を送らず、キャッシュも使わず、応答は `Access-Control-Allow-Origin` が許したときだけ読める。事前確認（プリフライト）が要る要求（PUT・JSON の POST など）は送らない |
| 文字列のタイマー | `setTimeout("code")` は関数にして実行（`eval` 相当） |

## 計測（リリース、Windows 11、3 プロセス合計）

| シナリオ | 結果 | 目標 |
|---|---|---|
| 空白タブ | 24.8MB（Browser 7.2 + Content 9.7 + Network 7.9） | 30MB |
| Wikipedia 1 タブ（JS あり） | 64.9MB（Browser 7.4 + Content 37.2 + Network 20.4） | 70MB |
| 一般的なサイト 10 タブ（JS あり、Phase 1 と同じサイト） | 200.3MB（Browser 8.4 + Content 154.2 + Network 37.8） | 250MB |

JS ランタイムの土台は 1 ページあたり約 3.3MB、起動は 25〜40ms。

## 互換性の目安

`scripts/compat.sh` が 50 サイトを別プロセスで開き、読み込み・描画命令の数・JS の例外・メモリ・時間から
OK / WARN / FAIL を自動判定する（`scripts/compat-sites.txt`）。判定は「実用可能」の厳密な基準ではなく目安。

実行すると画像と記録が `target/compat/` に残る（サイトごとに `*.png` と `*.log`）。

### 結果（2026-10-28、50 サイト）

**OK 37 / WARN 13 / FAIL 0**（完了条件の 35 サイトを、この目安では満たした）。

WARN の内訳：

| 理由 | サイト |
|---|---|
| ボット対策の確認ページ | stackoverflow・medium・reuters・nytimes・imdb・old.reddit |
| ES モジュールで作られたアプリ（未対応） | crates.io・zenn.dev |
| 巨大なバンドル（512KB 超） | nhk.or.jp |
| Web Components・地図などの未実装 API | archive.org・openstreetmap.org |
| 描画命令が少ないだけで、表示は正しい | httpbin.org/html（文字だけの文書）・html.duckduckgo.com（検索欄だけの画面） |

後の 2 つは実際には表示できているので、実用可能なサイトはこの目安より少し多い。
一方で、OK の判定は「描画できた・例外が少ない」であって、操作まで確かめたわけではない
（クリック・入力・タイマーの動作は結合テストで確かめている）。

### 実用にならない主な理由

- ボット対策の確認ページ（Cloudflare・DataDome など）：JS の課題（チャレンジ）を解けない。
- ES モジュール（`<script type=module>`）で作られたアプリ：モジュールの読み込みが未実装。
- 巨大なバンドル（512KB 超）：純 Rust の JS エンジンでは実用的な時間・メモリで動かない。
- 地図・動画・Canvas・iframe・Web Components・Service Worker：未実装。

## 既知の制限

- `<script type=module>`・`import`・動的 `import()`、`async`/`defer` の区別（すべて文書順に実行）。
- `window.id` のような id による名前付きアクセス、`iframe`、`canvas`、Shadow DOM、`contentEditable`。
- `getBoundingClientRect`・`offsetWidth` などの幾何情報は直近のレイアウトの値（[phase4.md](phase4.md)）。`IntersectionObserver` は先頭の 40 回まで「見えている」と答える。
- `localStorage`・`document.cookie` はページを閉じると消える。Cookie は Network プロセスが管理し、JS からは見えない。
- Content が落ちて作り直すと、ページは読み込み直しになる（スクリプトの状態は失われる）。
- JS ヒープの上限は設けていない（巨大な配列や文字列を作るスクリプトで Content のメモリが増える）。
