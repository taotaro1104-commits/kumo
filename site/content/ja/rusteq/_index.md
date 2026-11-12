{
  "description": "Rust EQ は、端末の音楽を好みの音に調整する Android アプリ。16種類のプリセット、10／31バンドEQ、仮想低音や空間調整を、見やすい画面で楽しめます。",
  "tagline": "いつもの曲を、\n好きな音に。",
  "lead": "まずはプリセットをひとつ。もっと低音を、ボーカルをくっきりと。Rust EQ は、手元の音楽を聴きながら、自分に合うバランスを探せる Android アプリです。",
  "pills": [
    "Android 10 以上",
    "オフライン再生",
    "開発中 v0.4.7",
    "日本語 / English"
  ],
  "nav_all": "アプリ一覧",
  "nav_features": "できること",
  "nav_shots": "画面を見る",
  "nav_get": "ダウンロード",
  "download_button": "Google ドライブで入手",
  "source_button": "ソースコードを見る",
  "hero_caption": "音の変化を、曲線でも確かめる。",
  "features_title": "気軽にはじめて、じっくり音づくり。",
  "features": [
    {
      "title": "気分で選ぶ、16のプリセット",
      "body": "フラット、ボーカル、低音リッチなど、まずは好みの音から。31バンド用のロック・クラシックなどのプリセットも追加。小さなカーブを見比べながら選べます。"
    },
    {
      "title": "低音から高音まで、思いのままに",
      "body": "10／31バンドのパラメトリックEQで周波数・ゲイン・Qを調整。仮想低音は最大200%まで強められ、コンプレッサー、リミッター、エコー（量・間隔・繰り返し）も使えます。"
    },
    {
      "title": "広がりも、響きも",
      "body": "ステレオ幅やクロスフィードを調整。HRTF／SOFAとIRリバーブで、ヘッドホンの空間表現も試せます。"
    },
    {
      "title": "見つけた音を、とっておく",
      "body": "自作プリセットを保存して、JSONで書き出し・読み込み。AutoEQ形式のテキストやCSVの取り込みにも対応します。"
    }
  ],
  "shots_title": "3つの画面で、迷わず調整。",
  "shots_note": "「音づくり」「空間」「ツール」に機能を整理。画像は Android エミュレータで撮影した日本語UIです。画像を押すと原寸で開きます。",
  "shots": [
    {
      "img": "sound.png",
      "caption": "音づくり",
      "body": "プリセットとEQカーブをひとつの画面に。"
    },
    {
      "img": "space.png",
      "caption": "空間",
      "body": "スピーカーの図を見ながら、広がりを調整。"
    },
    {
      "img": "tools.png",
      "caption": "ツール",
      "body": "設定の保存・取り込みをまとめて。"
    },
    {
      "img": "presets.png",
      "caption": "プリセット一覧",
      "body": "カーブの違いから、気になる音を選ぶ。"
    }
  ],
  "how_title": "好きな曲で、はじめよう。",
  "how": [
    {
      "title": "音楽を開く",
      "body": "端末のファイル選択画面から、再生したい曲を選びます。フォルダを開いてキューに追加することもできます。"
    },
    {
      "title": "好みの音を探す",
      "body": "プリセットを選んで再生。EQのオン・オフを切り替え、聴き比べながら少しずつ調整します。"
    },
    {
      "title": "自分の設定を保存",
      "body": "気に入ったバランスは「ツール」で保存。通知からも再生・一時停止や曲送りができます。"
    }
  ],
  "download_title": "Rust EQ を使ってみる",
  "download_body": "Android向け署名APKをGoogleドライブの配布フォルダから入手できます。0.4.7のファイル名は rust-eq-0.4.7-release.apk です。",
  "download_note": "Android 10 以上／arm64-v8a・x86_64。APKを保存し、端末の案内に従ってインストールしてください。画面は日本語・Englishに対応し、端末の言語に合わせて切り替わります。Android 13以降はOSのアプリ言語設定でも選べます。 フォルダ内のファイル名とバージョンを確認してダウンロードしてください。",
  "details_title": "使う前に、知っておきたいこと",
  "details": [
    {
      "title": "再生できる音楽ファイル",
      "body": "MP3、WAV、FLAC、Ogg Vorbis、AAC-LC／M4A、ALAC、AIFFのモノラル・ステレオ音声に対応します。DRMで保護された音楽は再生できません。"
    },
    {
      "title": "他のアプリにもEQを使える？",
      "body": "対象を選んで保存し、未接続なら「接続」を押します。音声セッションを通知しないプレーヤーは、任意の「拡張検出」と追加のADB設定で接続できる場合があります。対象の接続表示を確認してください。端末EQは対応する音声セッション向けで、全アプリの出力をRust DSPへ強制転送する機能ではありません。仮想低音・ダイナミクス・HRTF・IRはアプリ内再生向けです。"
    },
    {
      "title": "曲情報が出ないときは？",
      "body": "YouTube Musicを対象に選び、Androidの「通知へのアクセス」を許可してください。通常の通知表示許可や拡張検出のDUMP権限とは別の設定です。Android 13以降で制限される場合は「アプリ情報 → メニュー → 制限付き設定を許可」を確認します。プレーヤーが曲情報を公開しない場合は表示できません。"
    },
    {
      "title": "音楽や設定の扱い",
      "body": "アプリ本体はインターネット権限やマイク権限を要求しません。選んだ音楽を端末内で処理します。ファイルはAndroidの選択画面から開きます。"
    },
    {
      "title": "開発と検証の状況",
      "body": "v0.4.7ではエコー調整を追加。Rustテストと、Android 15エミュレータでのandroidTest全44件の成功、APK署名を確認しましたが、実機でのテストと聴感確認は未実施です。v0.4.5。Android 13で別アプリの連携テスト42件、単体15件、Rust35件に成功。同一署名の更新と、アプリ終了後の設定復元・曲情報の再取得を確認しました。現行YouTube Music実機での動作は未確認です。"
    }
  ],
  "footer": "Rust で音づくり。Rust EQ",
  "listing_tagline": "音を、いい感じに。",
  "listing_body": "端末の音楽を、16のプリセットと10／31バンドEQで好みの音に。仮想低音や空間調整も、見やすい画面で楽しめます。",
  "listing_tags": [
    "Android 10 以上",
    "オフライン再生",
    "Rust DSP"
  ],
  "listing_cta": "Rust EQ の紹介を見る",
  "title": "Rust EQ",
  "site_name": "Rust EQ",
  "type": "rusteq",
  "download_url": "https://drive.google.com/drive/folders/1hImCSXhyR_w83_6F2caRq50dvMjMIgfF?usp=drive_link",
  "repo_url": "https://github.com/taotaro1104-commits/RustAndroidequalizer",
  "og_image": "rusteq/og-v2.png",
  "og_image_width": 1731,
  "og_image_height": 909,
  "og_image_alt": "Rust EQ。音を、いい感じに。16プリセット、10／31バンドEQ、オフライン再生に対応するAndroid音楽アプリ。",
  "external_title": "他アプリの音も、いつもの設定で。",
  "external_lead": "対応アプリを選んで保存。未接続なら「接続」を押して、いつものプレーヤーを開けます。",
  "external_steps": [
    {
      "title": "対象アプリを選ぶ",
      "body": "アプリ名で検索し、EQを使いたいアプリにチェック。複数のアプリを選べます。"
    },
    {
      "title": "選択内容を保存",
      "body": "使うアプリと待ち受け設定を保存。次回も同じ対象を使えます。"
    },
    {
      "title": "タップして接続",
      "body": "「接続」で対象アプリを開き、曲を再生し直します。接続中は「再接続」で試し直せます。"
    }
  ],
  "external_note": "音声セッションを公開する対応アプリに、端末EQを適用します。「再起動後に再開」で待ち受けを復元できます。すべてのアプリの音声を強制的にRust DSPへ通す機能ではありません。",
  "external_shots": [
    {
      "img": "target-picker-ja.png",
      "caption": "アプリを選んで保存"
    },
    {
      "img": "target-connected-ja.png",
      "caption": "接続状態を見て、再接続"
    }
  ],
  "playback_title": "YouTube Musicの曲情報も、ひと目で。",
  "playback_lead": "選んだプレーヤーの曲名・アーティスト・再生状態を「音づくり」と「再生情報」に表示します。",
  "playback_steps": [
    {
      "title": "YouTube Musicを選ぶ",
      "body": "「ツール → 他アプリにもEQ」でYouTube Musicを選んで保存。YouTubeとは別のアプリです。"
    },
    {
      "title": "通知へのアクセスを許可",
      "body": "「再生情報 → 再生情報の表示を設定」でAndroidの設定を開き、通知へのアクセスを許可します。"
    },
    {
      "title": "曲を再生して、戻る",
      "body": "YouTube Musicで再生し、Rust EQに戻ると曲情報を表示。必要なら「再生情報を再読み込み」を押してください。"
    }
  ],
  "playback_note": "曲情報の表示とEQ接続は別々に確認できます。通知本文の解析や曲履歴の保存は行いません。キャスト先の音声には端末EQは適用されません。"
}
