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

## 生成コンテンツ（::before / ::after）

- 以前は `::before` / `::after` の規則を読み捨てていた。いまは別の規則インデックスに入れ、要素ごとに疑似要素の計算値を作る。
- `content`：`none` / `normal`、文字列、`attr(x)`、`open-quote` / `close-quote`（`"` として）。カウンターと `url()` は未対応（その部品は空）。
- 疑似要素は `display: inline`（既定）なら親の段落に連ね、それ以外はボックスとして置く。フロートを囲むクリアフィックス
  （`content: ""; display: table; clear: both`）も効く。フロートした疑似要素・flex / grid の子の疑似要素も扱う。
- 疑似要素は生成元の要素と結びつけない匿名ボックス（クリック・幾何情報の対象にならない）。`::selection` などは無視。
- 実装：`kumo-css`（`cascade.rs` の `generate_pseudo`、`StyleMap::pseudo`）と `kumo-layout/src/pseudo.rs`。

## ヘッドレスの `--wait MS`

読み込み後にタイマーを動かしながら待ってから描画する（フェードインなどのアニメーションが終わった画面を撮れる）。

## ネットワーク

- リダイレクトは ureq の内部ではなく 1 段ずつ自分でたどる（上限 10。`http(s)` 以外への転送は拒否）。
  サーバーが閉じていた再利用の接続に当たったとき（TLS の close_notify なしの切断など）は、その段だけ新しい接続でやり直す。
  以前は内部でたどっていたため、途中の段で失敗するとやり直しが最初からになり、同じ接続にまた当たって読み込めなかった（craigslist）。
- 本文の読み取りも、サーバーが close_notify なしで閉じた場合に、長さが未宣言か宣言どおり揃っていれば、そこまでを本文とする。

## 未解決の観察

- python.org：スライドショー（左にコード、右に説明文）が出ない。例外はなくなり、クリアフィックスと `>>>` の生成は効いたが、
  説明文が全幅の中央寄せで、スライドの領域が重なる（フロートと幅の指定に関わる別のレイアウトの差。未調査）。
- craigslist.org：読み込めるようになったが、アプリの本体（1.3MB・760KB のスクリプト）が実行予算（512KB）を超えて走らないため、
  最初の「カーテン」（`.show-curtain` の白い覆い）が外れず白いまま。
