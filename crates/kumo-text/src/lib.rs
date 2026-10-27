//! テキストのシェーピング・改行（Parley）とグリフのラスタライズ（swash）。
//!
//! フォントはシステムフォントを fontique 経由で mmap し、`TextSystem` は
//! 全タブで1つを共有する前提で作る。
#![forbid(unsafe_code)]

mod raster;
mod style;

pub use parley;
pub use parley::{
    Alignment, AlignmentOptions, FontContext, InlineBox, InlineBoxKind, Layout, LayoutContext,
    PositionedLayoutItem, VerticalAlign,
};
pub use raster::{GLYPH_CACHE_BUDGET, GlyphBitmap, GlyphCache, GlyphKey};
pub use style::{LineHeightSpec, TextStyleSpec, VerticalAlignSpec, WhiteSpaceSpec};

/// テキストの描画属性テーブルへの添字。Parley の Brush として使う。
pub type BrushId = u32;

/// フォントDBとレイアウト用スクラッチ領域。
pub struct TextSystem {
    pub fonts: FontContext,
    pub layouts: LayoutContext<BrushId>,
}

impl Default for TextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl TextSystem {
    pub fn new() -> Self {
        Self {
            fonts: FontContext::new(),
            layouts: LayoutContext::new(),
        }
    }

    /// 段落ビルダを開始する。`root` は段落全体（ブロックコンテナ）のスタイル。
    pub fn tree_builder<'a>(
        &'a mut self,
        root: &TextStyleSpec,
    ) -> parley::TreeBuilder<'a, BrushId> {
        let root_style = root.to_parley();
        self.layouts
            .tree_builder(&mut self.fonts, 1.0, true, &root_style)
    }
}

/// 単一スタイルのテキストを1行も折り返さずに計測するユーティリティ（テスト・マーカー用）。
pub fn measure_single_line(
    system: &mut TextSystem,
    text: &str,
    style: &TextStyleSpec,
) -> (f32, f32) {
    let mut builder = system.tree_builder(style);
    builder.push_text(text);
    let (mut layout, _) = builder.build();
    layout.break_all_lines(None);
    (layout.width(), layout.height())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_and_wraps_text() {
        let mut system = TextSystem::new();
        let style = TextStyleSpec::default();
        let (w, h) = measure_single_line(&mut system, "Hello world", &style);
        assert!(w > 20.0, "幅 {w}");
        assert!(h > 10.0, "高さ {h}");

        let mut builder = system.tree_builder(&style);
        builder.push_text("one two three four five six seven eight nine ten");
        let (mut layout, _) = builder.build();
        layout.break_all_lines(Some(w));
        assert!(layout.len() > 1, "折り返されるはず");
    }

    #[test]
    fn collapses_whitespace() {
        let mut system = TextSystem::new();
        let style = TextStyleSpec::default();
        let mut builder = system.tree_builder(&style);
        builder.push_text("  a \n\t b  ");
        let (_, text) = builder.build();
        assert_eq!(text.trim(), "a b");
    }
}
