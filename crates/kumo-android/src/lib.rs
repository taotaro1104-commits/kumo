//! Android 版 Kumo の共有ライブラリ（`libkumo.so`）。NativeActivity が `android_main` を呼ぶ。
//! 画面・入力・ブラウザ本体は `kumo-viewer` にあり、ここは起動前の環境づくりだけを受け持つ。
#![cfg(target_os = "android")]

mod jni_bridge;

use android_activity::AndroidApp;
use kumo_mem::TaggedAllocator;

#[global_allocator]
static ALLOC: TaggedAllocator = TaggedAllocator;

/// アプリが書ける場所へ `HOME`（履歴・設定）と `TMPDIR`（共有メモリ）を向ける。
/// Android の `temp_dir()` は既定で書き込めない `/data/local/tmp` を指すため。
fn prepare_environment(app: &AndroidApp) {
    let Some(files) = app.internal_data_path() else {
        return;
    };
    let tmp = files.join("tmp");
    let _ = std::fs::create_dir_all(&tmp);
    // SAFETY: アプリの最初のスレッド（まだ他のスレッドを作る前）で、環境変数を読む側もまだ動いていない。
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("HOME", &files);
        std::env::set_var("TMPDIR", &tmp);
    }
}

// SAFETY: android-activity が決めた名前と型のシンボルを、この共有ライブラリから公開するだけ。
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    prepare_environment(&app);
    jni_bridge::set_activity(app.activity_as_ptr());
    // 保存するパスワードの鍵は Android Keystore に置く
    kumo_browser::set_key_provider(jni_bridge::vault_key);
    kumo_viewer::android::run(
        app,
        Box::new(jni_bridge::show_keyboard),
        Box::new(jni_bridge::top_inset),
    );
}
