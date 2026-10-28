//! SVG 画像（resvg）。文字は描かない（フォントを持ち込まないため）。

use resvg::{tiny_skia, usvg};

/// 先頭付近に `<svg` があれば SVG とみなす。
pub(crate) fn is_svg(data: &[u8]) -> bool {
    let head = &data[..data.len().min(1024)];
    head.windows(4).any(|w| w.eq_ignore_ascii_case(b"<svg"))
}

fn parse(data: &[u8]) -> Option<usvg::Tree> {
    usvg::Tree::from_data(data, &usvg::Options::default()).ok()
}

/// 固有サイズ（`width`/`height`、無ければ `viewBox`）。
pub(crate) fn size(data: &[u8]) -> Option<(f32, f32)> {
    let s = parse(data)?.size();
    Some((s.width(), s.height()))
}

/// 指定サイズへ拡縮して描く。結果は乗算済みアルファの RGBA。
pub(crate) fn render(data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let tree = parse(data)?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let s = tree.size();
    let transform =
        tiny_skia::Transform::from_scale(width as f32 / s.width(), height as f32 / s.height());
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="40" height="20" fill="#f00"/></svg>"##;

    #[test]
    fn sniffs_and_measures() {
        assert!(is_svg(RED));
        assert!(!is_svg(b"\x89PNG"));
        assert_eq!(size(RED), Some((40.0, 20.0)));
    }

    #[test]
    fn renders_scaled() {
        let px = render(RED, 8, 4).unwrap();
        assert_eq!(px.len(), 8 * 4 * 4);
        assert_eq!(&px[..4], &[255, 0, 0, 255]);
    }
}
