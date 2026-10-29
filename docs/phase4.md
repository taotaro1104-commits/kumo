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

## `light-dark()`

明るい配色だけを描くので、`light-dark(明, 暗)` は第 1 引数（明）を色として使う（MDN が使っている）。

## SVG 画像

`<img>` の SVG は resvg（テキスト機能なし）で表示サイズに描く（`kumo-image/src/svg.rs`）。`<text>`・`data:` URI・CSS 背景・インライン `<svg>` は未対応。
幅だけ指定した画像が固有の高さのままになる不具合は、`vendor/taffy` の leaf 計測へスタイルの幅を渡して直した（Kumo patch）。

## ブックマーク

- 保存は履歴と同じ redb（`bookmarks` テーブル、`kumo-browser/src/bookmarks.rs`）。URL ごとに題名と追加時刻。
- 操作：URL 欄右の ☆/★（または Ctrl+D）で付け外し、☰（または Ctrl+B）で `about:bookmarks`（追加の新しい順、各行に「削除」）。`about:` のページは対象外。
- 削除リンク（`kumo:unbookmark?u=…`）は一覧を表示しているタブからだけ受け付ける（他のページからは操作できない）。
- 新しいタブのページの早見表にも Ctrl+D / Ctrl+B を載せた。

## ホーム URL

- `about:settings`（新しいタブのページの「設定」リンク）の入力欄に URL を書いて保存する。空で保存すると解除。保存先は redb の `settings` テーブル。
- 使われる場所：起動時（URL 指定なしのとき）、ホームボタン ⌂、Alt+Home。未設定なら新しいタブのページ。Ctrl+T の新しいタブは従来どおり。
- 保存できるのは http(s)・file の URL（URL 欄と同じ解釈。検索語は Wikipedia 検索になる）。`about:` は不可。保存フォームは設定ページを表示しているタブからだけ受け付ける。

## 読み込みの負荷レベル（5 段階）

サイトの読み込みで取得・実行する範囲を、軽い方（1）から重い方（5）まで選ぶ（`kumo-content/src/level.rs`）。

| レベル | 名前 | 取得するもの | スクリプトの予算（1 つ / 合計 / 時間） |
|---|---|---|---|
| 1 | 最軽量 | HTML と埋め込みの CSS だけ | 実行しない |
| 2 | 軽量 | + 外部 CSS | 実行しない |
| 3 | 標準（既定） | + 画像・スクリプト | 512KB / 1.5MB / 2 秒 |
| 4 | 高負荷 | 同上 | 2 倍 |
| 5 | 最大 | 同上、先に取得するスクリプトの数も 64 → 128 | 4 倍 |

- 選ぶ場所：`about:settings` の「読み込みの負荷」（redb に保存）、または起動引数 `--load-level 1〜5`（その回だけ。保存しない）。
- 次に始める読み込みから効く（表示中のページは変わらない）。Content を作り直したときも送り直す（`--no-js` も同様）。
- 内部ページ（新しいタブ・設定など、Browser が作る HTML）は常に標準で読む。
- 高いレベルほどメモリと時間を使う（スクリプトの予算が増えるため）。設計書のメモリ目標は標準での値。

## 画像の読み込みの設定

- `about:settings` の「画像」で「読み込む／読み込まない」を選ぶ（redb に保存。既定は読み込む）。起動引数 `--no-images` はその回だけ。
- 切ると画像を取得しない：文書と一緒に先読みする分に加え、レイアウト中の遅延取得（スクリプトで足した画像など）も止める（`FetchLoader` が共有の旗を見る）。
- 負荷レベルが 2 以下のときは、この設定にかかわらず読み込まない。次に始める読み込みから効き、Content を作り直したときも送り直す。
- 既に取得済みの画像（キャッシュ内）は、同じ URL なら引き続き使われる。

## パスワードマネージャ

ログイン情報の保存・自動入力・一覧。`about:settings` の「パスワード」で使う／使わないを切り替える（既定は使う。redb の設定 `passwords` に保存）。

- **保存の確認:** ログインのフォーム（値の入ったパスワード欄を持つ `form`）を送信すると、Content が `FromContent::LoginSubmitted` を送る。Browser は次のページの読み込みが終わったところで、URL 欄の位置に「保存／今回は保存しない／このサイトでは保存しない」を出す。すでに同じ内容を保存していれば出さない。
- **自動入力:** ページの読み込みが終わったとき、同じ origin（スキーム・ホスト・ポート）の保存があれば `ToContent::Autofill` で空のパスワード欄とその前のユーザー名欄に入れる。入力済みの欄は上書きしない。ページが送った時点の URL と一致しなければ Content が捨てる。http(s) 以外（file・about）は対象外。
- **暗号化:** パスワードは ChaCha20-Poly1305 で包んで redb の `logins` に置く。キーは `origin NUL ユーザー名`で、認証データにも使うので、暗号文を別の行（別のサイト）へ移しても開けない。鍵（32 バイト）は Windows の資格情報マネージャ（`kumo-browser` / `password-vault-key`）にあり、データベースのファイルだけが持ち出されても読めない。ストアが使えないときは保存しない（平文では持たない）。
- **一覧:** `about:passwords`（新しいタブ・設定からリンク）。パスワードは伏せてあり、「表示」を押した 1 件だけ見える。「削除」もある。操作リンク（`kumo:showpw` / `hidepw` / `delpw`）と設定の切り替えは、そのページを表示しているタブからだけ受け付ける。確認の 3 つの操作は UI（`ChromeAction`）からだけで、ページからは起こせない。
- **ファイル:** `kumo-browser` の `vault.rs`（暗号）、`logins.rs`（保存）、`login_actions.rs`（確認・自動入力・一覧の操作）、`passwords_page.rs`（一覧のページ）。`kumo-content` の `login.rs`（フォームから取り出す・欄へ入れる）。

### 既知の制限

- マスターパスワードや OS の再認証はなく、「表示」は誰でも押せる（端末にログインしている人は読める）。
- ログインに失敗したページ（同じフォームがまた出るページ）でも保存を確認する。
- 自動入力は複数の保存があるとユーザー名の順で先頭の 1 件を入れる（選択 UI はない）。
- 動的に後から作られるフォーム（読み込み完了後に出るログイン欄）には入れない。
- 変更用のフォーム（新しいパスワード欄が 2 つ）を区別しない。

## 表示言語（5 か国語）

Kumo 自身の画面（ブラウザの枠・`about:` の内部ページ・エラーページ）を、日本語（既定）・English・简体中文・한국어・Español から選べる。開いた Web ページの中身は訳さない。

- **選び方:** `about:settings` の先頭の「言語」。保存して（redb の設定 `language`）次の起動でも使う。1 回だけ変えるには `kumo-viewer --lang ja|en|zh|ko|es`。変えると UI と開いている設定ページをすぐ作り直し、ほかの内部ページ・新しいタブ・エラーページは次に開くとき（再読み込み）から新しい言語になる。設定の切り替えは設定ページを表示しているタブからだけ受け付ける。
- **文言の表:** `kumo-content/src/i18n/`。`messages!` で「キー → [日・英・中・韓・西]」の表を作り、`キー.t(lang)` で引く（`ui.rs` 枠、`pages.rs` 内部ページ、`errors.rs` エラーページ）。`{0}` `{1}` は `fill` で埋める。テストが、全部の文言が 5 言語とも埋まっていること、`{n}` の場所が言語ごとに揃っていること、英語が日本語のままでないことを確かめる。新しい文言を足すときは表に 1 行足すだけ。
- **Content への伝え方:** エラーページは Content が作るので、`ToContent::Language` で言語を渡す（Content を作り直したときも送り直す）。
- **字形:** 既定のフォールバックは日本語の字形のフォントを選び、簡体字だけの字（请・页・简 など）が欠けるので、フォントの並びの最後に Microsoft YaHei・Noto Sans CJK SC・Malgun Gothic を足した（`kumo-text/src/style.rs`）。

### 既知の制限

- 起動時の既定は常に日本語（OS の言語は見ない）。
- ログ（標準エラー出力）・コマンドラインの案内・フォームの既定のボタン名（`<input type=submit>` の「送信」）は訳さない。
- 右から左に書く言語（アラビア語など）は対象外。

## 閲覧履歴とダウンローダー

### 閲覧履歴（`about:history`、Ctrl+H）

- 訪れたページ（`about:` 以外）は `loaded()` で redb の `visits` に記録している。一覧は新しい順に最大 500 件、「たった今・N 分前・N 時間前・N 日前」で示し、7 日より古い訪問は日付（UTC）で示す（OS のタイムゾーンを取らないため）。
- 「削除」（`kumo:unhistory?u=`）で 1 件、「履歴をすべて消去」（`kumo:clearhistory`）で全部消せる。ブックマーク・設定・パスワードは残る。どちらも履歴ページを表示しているタブからだけ受け付ける。

### ダウンローダー（`about:downloads`、Ctrl+J）

- **検出（Network プロセス）:** ページの移動として取得した応答（`ResourceKind::Document`、2xx）が `Content-Disposition: attachment`、または表示できない種類（text・image・xml・json・javascript 以外。zip・pdf・exe・音声・動画など）なら、本文を読まずに `FetchError::Download` を返す。Content は表示中のページを変えずに `FromContent::Download` を Browser へ送る。
- **保存（Network プロセス）:** Browser が `ToNetwork::Download` を送ると、Network が専用のスレッドで取得し、読みながらファイルへ書く（メモリに溜めないので大きなファイルでも増えない。32MB の本文の上限は受けない）。本文の読み取りに時間の上限は付けず、接続と応答の待ちだけ 30 秒。リダイレクトとコンテンツブロックは通常の取得と同じ。書きかけは `名前.part` に置き、終わったら名前を変える。失敗・中止では消す。
- **ファイル名:** `Content-Disposition` の `filename*`（UTF-8）→ `filename` → URL の末尾。`/ \ : * ? " < > |` と制御文字は `_` にし、前後の空白とドットを除く。Windows の予約名（CON・NUL・COM1 など）は `_` を付け、120 文字までにする（拡張子は残す）。同じ名前があれば `名前 (1).拡張子` と番号を付け、上書きしない。
- **保存先:** `%USERPROFILE%\Downloads`（なければ `$HOME/Downloads`）。ページに保存先フォルダを表示する。
- **経過:** 150ms ごとに Network → Browser へ `FromNetwork::Download` を送り、Browser は `Fetcher::take_download_events` で取り込む。ウィンドウはダウンロード中だけ 300ms ごとに起きて一覧を作り直す。一覧にはバー・受信量・合計・中止・完了後の保存先を出す。
- **移動の取り消し:** 移動先がファイルだったときは、タブの履歴に足した項目を戻して URL 欄を元に戻す。背景のタブや空のタブで開いたものは、空になったタブを閉じる（唯一のタブなら新しいタブに戻す）。そのあと一覧のタブを開く。
- **操作:** 「中止」（`kumo:dlcancel?id=`）と「終わった項目を一覧から消す」（`kumo:dlclear`）。履歴と同じく、一覧を表示しているタブからだけ受け付ける。

### 既知の制限

- ダウンロードの一覧はセッション内だけ（再起動すると消える。ファイルは残る）。
- 一時停止・再開、保存先の変更、保存先の選択ダイアログはない。ログインが要るサイトの POST で得たファイルは GET で取り直すため保存できない。
- `--in-process`（診断用の同一プロセス実行）は保存に対応せず、一覧に失敗として出る。
