//! Windows では、実行ファイルにアイコンと版情報を埋め込む（エクスプローラー・ショートカット・タスクバーの表示用）。

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    // Windows 上で Android 向けにビルドするときは、実行ファイルがないので埋め込まない
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icon.ico");
        resource.set("ProductName", "Lite Simple Rust Browser");
        resource.set("FileDescription", "Lite Simple Rust Browser");
        if let Err(e) = resource.compile() {
            // アイコンが付かないだけなので、ビルドは止めない
            println!("cargo:warning=アイコンを埋め込めません: {e}");
        }
    }
}
