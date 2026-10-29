//! Windows では、実行ファイルにアイコンと版情報を埋め込む（エクスプローラー・ショートカット・タスクバーの表示用）。

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    {
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
