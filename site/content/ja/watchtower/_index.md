---
title: "watchtower-rs"
site_name: "watchtower-rs"
description: "changedetection.io 互換の Web ページ監視ツールを Rust で。単一バイナリ、5,000 件を見張っても約 24 MB。"
tagline: "ページの変化を、静かに見張る。"
lead: "watchtower-rs は、Web ページの変化を定期的に確かめて、差分を通知する監視ツールです。changedetection.io と互換の REST API を持ち、Rust の単一バイナリで動きます。"
pills: ["単一バイナリ", "SQLite", "changedetection.io 互換", "REST API"]
nav_all: "アプリ一覧"
nav_features: "特長"
nav_start: "使い方"
nav_status: "開発の状況"
stats_title: "メモリ使用量（Windows のリリースビルドでの実測）"
stats:
  - { value: "約 17 MB", label: "1,000 件を登録して待機しているとき" }
  - { value: "約 24 MB", label: "5,000 件を 300 秒間隔で監視したときのピーク" }
  - { value: "約 90 秒", label: "5,000 件の最初の取得がすべて終わるまで" }
stats_note: "実行ファイルの大きさは約 28 MB。測定は自作の模擬サイトに対して行ったもので、環境によって変わります。"
features_title: "特長"
features:
  - { icon: "🍃", title: "省メモリ", body: "多くのウォッチを登録しても、メモリは増えにくい作りです。スナップショットは圧縮してファイルに置きます。" }
  - { icon: "🔁", title: "本家と互換", body: "REST API は x-api-key で認証する本家の形。本家の url-watches.json も取り込めます。" }
  - { icon: "✂️", title: "必要な所だけ比べる", body: "CSS セレクタ、XPath、JSONPath、jq で対象を絞り、正規表現や無視する文字列で揺れを除きます。トリガーテキストの一致だけを通知することもできます。" }
  - { icon: "🌐", title: "JavaScript のページも", body: "別に動かした Chrome に接続して、描画後のページを取得します。クリック・入力・待機の操作を登録でき、画面から要素を選ぶビジュアルセレクタも使えます。" }
  - { icon: "🏷️", title: "価格・在庫・PDF", body: "商品ページの構造化データから価格を読み、しきい値以下で通知します。PDF の本文も比べられます。" }
  - { icon: "🔔", title: "多くの通知先", body: "Discord・Slack・Telegram・ntfy・Gotify・Mattermost・Pushover・メール・Webhook に対応。そのほかは Apprise API へ転送します。件名と本文はテンプレートで変えられます。" }
  - { icon: "🛠️", title: "取得の細かな指定", body: "ウォッチごと、タグごと、全体で、ヘッダ・メソッド・ボディ・プロキシ・間隔を決められます。近い設定が優先されます。" }
  - { icon: "🛡️", title: "安全に配慮", body: "内部のアドレスへの接続を断つ SSRF 対策、argon2 のパスワード、CSRF 対策、Host 名の許可リスト。ヘッダの値や通知先の秘密はログに出しません。" }
  - { icon: "📈", title: "運用しやすい", body: "OpenAPI の文書、Prometheus 形式の /metrics、RSS フィード、バックアップと復元、ヘルスチェック。Docker 用のファイルも付いています。" }
how_title: "しくみ"
how:
  - { title: "取得する", body: "決めた間隔でページを取得します。内部アドレスへの接続は既定で断ります。" }
  - { title: "整える", body: "対象を絞り、不要な要素や揺れる文字を除いて、テキストにします。" }
  - { title: "比べて知らせる", body: "前回との差分があれば保存し、条件に合うときだけ通知を送ります。送れなかった通知は再送します。" }
start_title: "使い方"
start_lead: "コマンドラインから、パスワードを決めて起動します。管理画面は既定で 127.0.0.1:5000 に開きます。"
start_code: |-
  watchtower set-password     # 管理画面のパスワード（標準入力から 1 行）
  watchtower api-key          # REST API のキーを発行
  watchtower watch add https://example.com/ --interval 300
  watchtower run              # 定期チェックと管理画面を開始
start_note: "設定は環境変数（WATCHTOWER_DATA_DIR・WATCHTOWER_LISTEN・WATCHTOWER_BASE_URL など）か設定ファイルで変えます。"
status_title: "開発の状況"
status_body: "開発中です。配布用のファイルとソースコードの公開は、まだ行っていません。Windows 11 で動作を確かめています。"
todo_title: "まだ確かめていないこと"
todo:
  - "Docker イメージのビルドと実行"
  - "Linux 用の静的バイナリ（musl）と aarch64"
  - "TLS・STARTTLS を使う実際のメールサーバーへの送信"
footer: "© watchtower-rs — Apache-2.0"
---
