# Phase 4（途中）：互換性と速度

実サイトを開いて見つけた不具合から直している。Web Platform Tests・Vello（GPU）・JIT は未着手。

## 幾何情報（getBoundingClientRect・offsetWidth など）

- 描画命令を作るとき、要素ごとの境界ボックスを記録する（`DisplayList::geometry`、JS を使うページだけ）。
- Content は再レイアウトのたびに、その表を JS ランタイムへ渡す（`JsRuntime::set_geometry`）。描画命令側には残さない。
- `getBoundingClientRect`・`getClientRects`・`offsetWidth/Height/Left/Top`・`clientWidth/Height`・`scrollWidth/Height` が値を返す。
  `<html>` の `clientWidth/Height` は表示領域の大きさ。表にない要素（`display: none` など）は 0。
- **`DOMContentLoaded` と `load` は最初のレイアウトの後に送る**ので、読み込み時の初期化（jQuery の ready など）が幾何情報を読める。
  スクリプトの実行中（同期のスクリプト）は、まだレイアウト前なので 0。
- 制限：DOM を変えた直後は次のレイアウトまで古い値。`offsetParent` は null、スクロール位置は 0 として扱う。

## 機能検出

- `"MozAppearance" in el.style` のように、ほかのエンジンの接頭辞（Moz・ms・O）のプロパティは「持たない」と答える
  （以前は何でも「ある」と答え、modernizr が存在しない `createRange` を呼んで例外になっていた）。
- `"fooBar" in el.dataset` は `data-foo-bar` 属性があるときだけ真。
- `document.createRange()`（`createContextualFragment` など）と `window.getSelection()` の最小実装。

## getComputedStyle

- 以前は `el.style`（インラインだけ）を返していた。いまは、インラインのスタイルを優先し、なければ Content が最後に
  スタイルを計算した結果から答える（`display`・`position`・`float`・`color`・`background-color`・`opacity`・`z-index`・
  `font-size`・`font-weight`・`margin-*`・`padding-*`・`border-*-width` など。`width` / `height` は直近のレイアウトから）。
- 計算値は文字列にした小さな表（`StyleSnapshot`）で JS を使うページだけが持つ。スタイル計算の一時ヒープを引き止めない。
- 制限：クラスを変えた直後などは、次のスタイル計算まで古い値。表にないプロパティは空文字列。
- スクリプトが作ったばかりの要素（まだスタイル計算されていない）は、`display` と `visibility` を UA の既定値で答える。
  jQuery の `show()` は一時要素の既定の display を読み、空だと内部で例外になって `fadeIn` が止まっていた（python.org のスライドショーが出なかった原因）。
- jQuery の `.show()` / `.hide()` / `.is(':visible')` などがスタイルシート由来の `display: none` を正しく判定できる。
- `style.opacity` などは `.5` を `0.5` に正規化して返す（jQuery の機能検出が見る）。

## taffy の clear の不具合（vendor/taffy）

`clear: both` のブロックが中にフロートを持つと、直前の兄弟の下ではなく、外側のフロートの下端に置かれた
（python.org のスライドショーがナビに重なっていた原因）。taffy 0.15.1 のブロックレイアウトが、
要素自身の内側のフロート（自分の上マージンに隣接するもの）を取り込んだ後で「強制クリアランス」を判定していたため。
判定を取り込む前に行うよう直した写しを `vendor/taffy` に置き、ルートの `Cargo.toml` の `[patch.crates-io]` で差し替えている
（変更箇所は `compute/block.rs` の `Kumo patch`）。再現は `kumo-layout/tests/layout.rs` の
`cleared_block_with_inner_floats_variants`。上流で直ったら `[patch]` と `vendor/` を外す。

## 数学関数と表示領域の単位

- `min()` / `max()` / `clamp()` は、以前は代表値（`max` は最後の引数など）で近似していた。いまは式として保持し
  （`kumo-css/src/math.rs`、`Length::expr`）、フォントサイズ・表示領域が分かる適用時に評価する。
  パーセントを含む比較は表示領域の幅を基準にし、勝った引数をそのまま使う（包含ブロックの幅による厳密な評価は未対応）。
- 単位：`svh` / `lvh` / `dvh` / `svw` / `lvw` / `dvw` / `vi` / `vb` を `vh` / `vw` として扱う（表示領域は一定）。
  `vmin` / `vmax`（と `s`・`l`・`d` 付き）は `min` / `max` の式で正しく求める（以前は両方 `vw` だった）。
- MDN のサイドバー（`max(var(..), calc(50vw - 720px + 1rem))` の余白）が左に切れていたのが直った。

## グリッドの `grid-column: 名前`

`grid-column: content` のように名前だけを書いたとき、終わりも同じ名前（`content-start` から `content-end` まで）にする
（以前は終わりが空で 1 トラックだけになっていた）。The Guardian のようにグリッドの名前付きラインで組んだページが、
細い 1 列に押し込まれていたのが直った。

## 生成コンテンツ（::before / ::after）

- 以前は `::before` / `::after` の規則を読み捨てていた。いまは別の規則インデックスに入れ、要素ごとに疑似要素の計算値を作る。
- `content`：`none` / `normal`、文字列、`attr(x)`、`open-quote` / `close-quote`（`"` として）。カウンターと `url()` は未対応（その部品は空）。
- 疑似要素は `display: inline`（既定）なら親の段落に連ね、それ以外はボックスとして置く。フロートを囲むクリアフィックス
  （`content: ""; display: table; clear: both`）も効く。フロートした疑似要素・flex / grid の子の疑似要素も扱う。
- 疑似要素は生成元の要素と結びつけない匿名ボックス（クリック・幾何情報の対象にならない）。`::selection` などは無視。
- 実装：`kumo-css`（`cascade.rs` の `generate_pseudo`、`StyleMap::pseudo`）と `kumo-layout/src/pseudo.rs`。

## ヘッドレスの `--eval JS`

読み込みと `--wait` の後、表示中のタブのページで JS の式を評価して `[eval] 結果` を標準出力に出す（複数指定可。
式の間に 0.7 秒タイマーを動かす）。実サイトの DOM・スタイル・幾何情報を調べるための診断用
（`ToContent::Eval` / `FromContent::EvalResult`）。

## ヘッドレスの `--wait MS`

読み込み後にタイマーを動かしながら待ってから描画する（フェードインなどのアニメーションが終わった画面を撮れる）。

## ネットワーク

- リダイレクトは ureq の内部ではなく 1 段ずつ自分でたどる（上限 10。`http(s)` 以外への転送は拒否）。
  サーバーが閉じていた再利用の接続に当たったとき（TLS の close_notify なしの切断など）は、その段だけ新しい接続でやり直す。
  以前は内部でたどっていたため、途中の段で失敗するとやり直しが最初からになり、同じ接続にまた当たって読み込めなかった（craigslist）。
- 本文の読み取りも、サーバーが close_notify なしで閉じた場合に、長さが未宣言か宣言どおり揃っていれば、そこまでを本文とする。

## 未解決の観察

- python.org：スライドショーとナビ・説明文の並びは実物と同じになった。右上の検索欄（GO ボタンの位置）に細かい差が残る。
- craigslist.org：読み込めるようになったが、アプリの本体（1.3MB・760KB のスクリプト）が実行予算（512KB）を超えて走らないため、
  最初の「カーテン」（`.show-curtain` の白い覆い）が外れず白いまま。
