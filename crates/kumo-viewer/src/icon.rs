//! ウィンドウ（タイトルバー・タスクバー）のアイコン。`assets/icon.svg` から作った 64px の PNG を埋め込む。
//! 実行ファイル自身のアイコン（エクスプローラー・ショートカット）は build.rs が資源として埋め込む。

use winit::window::Icon;

const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon-64.png");

/// 埋め込みの PNG をアイコンにする。壊れていれば `None`（アイコンなしで動く）。
pub fn window_icon() -> Option<Icon> {
    let image = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_decodes() {
        assert!(window_icon().is_some());
    }
}
