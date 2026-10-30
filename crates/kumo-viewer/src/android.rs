//! Android 版の入り口。`kumo-android`（共有ライブラリ）の `android_main` から呼ばれる。
//!
//! Android のアプリは自分の実行ファイルを子プロセスとして起動できないので、Network も Content も
//! スレッドで動かす（`--in-process` と同じ）。画面は winit の android-activity と softbuffer。

use android_activity::AndroidApp;
use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;

use crate::window::{ContentWake, Platform};

/// ブラウザを起動し、アプリが終わるまで戻らない。
/// `keyboard` はソフトキーボードを出す・隠す関数。そこから届く文字は [`crate::ime::push`] で渡す。
/// `top_inset` は画面の上端を覆うステータスバーの高さ（物理 px）。
/// 呼ぶ前に `HOME`（履歴・設定の置き場）と `TMPDIR`（共有メモリの置き場）を、アプリが書ける場所へ向けておくこと。
pub fn run(app: AndroidApp, keyboard: Box<dyn Fn(bool)>, top_inset: Box<dyn Fn() -> u32>) {
    kumo_mem::configure_for_low_memory();
    let mut args = match crate::args::parse(std::iter::empty()) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    args.in_process = true;
    let event_loop = match EventLoop::<ContentWake>::with_user_event()
        .with_android_app(app.clone())
        .build()
    {
        Ok(event_loop) => event_loop,
        Err(e) => {
            eprintln!("[android] イベントループを作れません: {e}");
            return;
        }
    };
    let platform = Platform {
        event_loop: Some(event_loop),
        // 端末のファイル管理アプリから見えるアプリ専用フォルダ（なければアプリ内部）
        download_dir: app
            .external_data_path()
            .or_else(|| app.internal_data_path())
            .map(|dir| dir.join("Download")),
        keyboard: Some(keyboard),
        top_inset: Some(top_inset),
    };
    if let Err(e) = crate::run(args, platform) {
        eprintln!("エラー: {e}");
    }
}
