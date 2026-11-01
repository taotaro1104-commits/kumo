---
title: "RustHighBookViewer"
site_name: "RustHighBookViewer"
description: "Rust で作ったコミック・画像ビューア。ZIP/CBZ・7z・RAR・EPUB・画像フォルダを、右綴じの見開きなどで読めます。Android と Windows に対応。"
tagline: "本を開いて、すぐ読める。"
lead: "RustHighBookViewer は、コミックや画像の入った書庫・フォルダを開いて読むビューアです。右綴じの見開き、拡大、色調整、読みかけの位置の保存に対応し、Android と Windows で動きます。"
pills: ["Android", "Windows", "ZIP / CBZ / 7z / RAR / EPUB", "右綴じ対応"]
nav_all: "アプリ一覧"
nav_features: "特長"
nav_how: "使い方"
nav_get: "入手方法"
nav_status: "開発の状況"
stats_title: "対応していること"
stats:
  - { value: "5 種類", label: "ZIP/CBZ・7z・RAR・EPUB・画像フォルダを開けます" }
  - { value: "4 つの表示", label: "単ページ・見開き・自動見開き・縦スクロール" }
  - { value: "2 つの環境", label: "Android 8.0 以上と、Windows" }
stats_note: "RAR は 4.x / 5.x（暗号化なし）に対応。7z と RAR は、開くときに端末の保存領域へ展開します。EPUB は 1 ページが 1 枚の画像の本（コミック・写真集）に対応します。"
features_title: "特長"
features:
  - { icon: "📖", title: "右綴じも左綴じも", body: "綴じ方向を切り替えられ、見開きの並びも合わせて変わります。表紙だけ単独で出し、横長のページは 1 枚で表示します。" }
  - { icon: "🔍", title: "拡大とルーペ", body: "ピンチやダブルタップで拡大してパンできます。Android 版には、指でなぞった所を 2 倍に拡大するルーペもあります。" }
  - { icon: "🗂️", title: "ページ一覧", body: "全ページのサムネイルから、読みたいページへ移れます（Android 版）。" }
  - { icon: "🎨", title: "色調整", body: "明るさ・コントラスト・ガンマ、白黒・セピア・反転。本ごとに設定を覚えます（Android 版）。" }
  - { icon: "⏱️", title: "オートめくり", body: "3・5・10・20 秒ごとに自動でページをめくります。最後のページで止まります（Android 版）。" }
  - { icon: "📚", title: "本棚と続きから読む", body: "履歴・お気に入り・タイトル検索のある本棚。閉じても読んでいた位置を保存して、続きから開けます。" }
  - { icon: "📁", title: "フォルダも開ける", body: "フォルダを選ぶと、直下の画像を 1 冊、EPUB や ZIP・7z・RAR は 1 ファイルごとに 1 冊として、サブフォルダも含めてまとめて本棚へ追加します。Android では「すべてのファイルへのアクセス」を許可すると、Download なども直接選べます。" }
  - { icon: "🛡️", title: "安全に配慮", body: "展開するサイズとファイル数に上限があり、空き容量が少ないときは途中で止めます。書庫の中の ../ のようなパスは無効にします。" }
  - { icon: "🔢", title: "ページ指定としおり", body: "ページ番号を入力して移動できます。しおりの一覧から飛ぶことも、ワンタップで付け外しすることもできます（Android 版）。" }
  - { icon: "⚙️", title: "設定画面", body: "表示モード・綴じ方向・フィットの既定、音量キーでのページ送り、画面の点灯、本棚の並び順、画像キャッシュに使うメモリを決められます（Android 版）。" }
  - { icon: "📘", title: "EPUB のコミック", body: "1 ページが 1 枚の画像でできた EPUB を、読む順に開きます。文字主体の EPUB には対応していません。" }
  - { icon: "⚡", title: "2 回目から速い", body: "ページの大きさを覚えておくので、フォルダや書庫を次に開くときの読み込みが速くなります。" }
how_title: "使い方"
how:
  - { title: "開く", body: "Android は本棚の「ファイル」か「フォルダ」から選びます。Windows は、exe にファイルやフォルダをドラッグ＆ドロップします。" }
  - { title: "読む", body: "画面の左右をタップしてページを送ります。中央をタップするとメニューが出て、表示モードや綴じ方向を変えられます。" }
  - { title: "続きから読む", body: "閉じると読んでいた位置を保存します。本棚から選べば、続きから開きます。" }
download_title: "入手方法"
download_button: "Google ドライブからダウンロード"
download_body: "Android 用の APK と、Windows 用の exe を、Google ドライブで配布しています。インストーラは無く、exe は実行するだけで使えます。"
download_note: "APK は自分で作った鍵で署名しています。端末で「提供元不明のアプリ」の許可が必要です。exe は署名しておらず、Windows の SmartScreen が警告を出すことがあります。"
cta_source: "ソースコード"
sha_label: "SHA-256（v0.1.3）"
release_label: "更新履歴・ソースコード（GitHub）"
download_url: "https://drive.google.com/drive/folders/15qKnVY5FglIUeVdFlwyLt3eVNU_XtltK?usp=sharing"
repo_url: "https://github.com/taotaro1104-commits/RustHighBookViewer"
release_url: "https://github.com/taotaro1104-commits/RustHighBookViewer/releases"
android_file: "RustHighBookViewer-0.1.3-android.apk"
windows_file: "RustHighBookViewer-desktop.exe"
android_sha: "2bef5262f6cbb348b7961bee0426102f33318eae24f88c17bb6ad4bf15d6d5fc"
windows_sha: "5211618dec0cff0fc5f1179d5a7de8f4f4556fc6baadc59ac90c4b095672854a"
status_title: "開発の状況"
status_body: "開発中です（v0.1.3・プレリリース）。Android は Android 15 のエミュレータで、7z・RAR・EPUB・フォルダの一括読み込みを確かめています。ソースコードは GitHub で公開しています。"
todo_title: "まだ確かめていないこと・できないこと"
todo:
  - "Android の実機での動作（確認はエミュレータのみ）"
  - "Windows 版の画面の見た目と操作（起動して動き続けることまで確認）"
  - "パスワード付きの書庫と、複数ファイルに分割された RAR（非対応）"
footer: "© RustHighBookViewer — MIT License"
---
