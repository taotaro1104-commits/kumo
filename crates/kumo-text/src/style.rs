//! CSS 由来のテキストスタイル（所有型）と Parley スタイルへの変換。

use std::borrow::Cow;
use std::sync::Arc;

use parley::{
    AlignmentBaseline, BaselineShift, FontFamily, FontStyle, FontWeight, LineHeight, TextStyle,
    TextWrapMode, VerticalAlign, WhiteSpaceCollapse,
};

use crate::BrushId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeightSpec {
    Normal,
    /// フォントサイズ倍率
    Number(f32),
    Px(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhiteSpaceSpec {
    Normal,
    Nowrap,
    Pre,
    PreWrap,
    PreLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlignSpec {
    Baseline,
    Sub,
    Super,
    Top,
    Middle,
    Bottom,
}

/// 1つのインライン区間に適用するスタイル。
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyleSpec {
    pub families: Arc<[Box<str>]>,
    pub size: f32,
    pub weight: u16,
    pub italic: bool,
    pub line_height: LineHeightSpec,
    pub white_space: WhiteSpaceSpec,
    pub vertical_align: VerticalAlignSpec,
    pub underline: bool,
    pub strikethrough: bool,
    pub brush: BrushId,
}

impl Default for TextStyleSpec {
    fn default() -> Self {
        Self {
            families: Arc::from([Box::from("sans-serif")]),
            size: 16.0,
            weight: 400,
            italic: false,
            line_height: LineHeightSpec::Normal,
            white_space: WhiteSpaceSpec::Normal,
            vertical_align: VerticalAlignSpec::Baseline,
            underline: false,
            strikethrough: false,
            brush: 0,
        }
    }
}

const GENERIC: [&str; 9] = [
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
];

/// 記号用のフォールバックフォント（Windows・Linux の代表的なもの）。
const SYMBOL_FALLBACK: &str = ", \"Segoe UI Symbol\", \"Noto Sans Symbols 2\", \"DejaVu Sans\"";

impl TextStyleSpec {
    /// CSS の font-family リスト表記を作る（固有名は引用符で囲む）。
    fn family_source(&self) -> String {
        let mut out = String::new();
        for (i, family) in self.families.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            if GENERIC.iter().any(|g| family.eq_ignore_ascii_case(g)) {
                out.push_str(&family.to_ascii_lowercase());
            } else {
                out.push('"');
                out.push_str(&family.replace('"', ""));
                out.push('"');
            }
        }
        // 最後の砦として sans-serif を付ける
        if !self
            .families
            .iter()
            .any(|f| GENERIC.iter().any(|g| f.eq_ignore_ascii_case(g)))
        {
            out.push_str(if out.is_empty() {
                "sans-serif"
            } else {
                ", sans-serif"
            });
        }
        // 記号（↻ など）はシステムの既定フォールバックで見つからないことがあるので、
        // 記号用フォントを最後に足す（存在しない名前は無視される）
        out.push_str(SYMBOL_FALLBACK);
        out
    }

    pub fn to_parley(&self) -> TextStyle<'static, 'static, BrushId> {
        let (collapse, wrap) = match self.white_space {
            WhiteSpaceSpec::Normal => (WhiteSpaceCollapse::Collapse, TextWrapMode::Wrap),
            WhiteSpaceSpec::Nowrap => (WhiteSpaceCollapse::Collapse, TextWrapMode::NoWrap),
            WhiteSpaceSpec::Pre => (WhiteSpaceCollapse::Preserve, TextWrapMode::NoWrap),
            WhiteSpaceSpec::PreWrap => (WhiteSpaceCollapse::Preserve, TextWrapMode::Wrap),
            WhiteSpaceSpec::PreLine => (WhiteSpaceCollapse::PreserveBreaks, TextWrapMode::Wrap),
        };
        let line_height = match self.line_height {
            LineHeightSpec::Normal => LineHeight::FontSizeRelative(1.2),
            LineHeightSpec::Number(n) => LineHeight::FontSizeRelative(n),
            LineHeightSpec::Px(px) => LineHeight::Absolute(px),
        };
        TextStyle {
            font_family: FontFamily::Source(Cow::Owned(self.family_source())),
            font_size: self.size,
            font_style: if self.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            },
            font_weight: FontWeight::new(f32::from(self.weight)),
            brush: self.brush,
            has_underline: self.underline,
            has_strikethrough: self.strikethrough,
            line_height,
            vertical_align: vertical_align(self.vertical_align),
            text_wrap_mode: wrap,
            white_space_collapse: collapse,
            ..TextStyle::default()
        }
    }
}

fn vertical_align(v: VerticalAlignSpec) -> VerticalAlign {
    match v {
        VerticalAlignSpec::Baseline => VerticalAlign::BASELINE,
        VerticalAlignSpec::Sub => VerticalAlign::SUB,
        VerticalAlignSpec::Super => VerticalAlign::SUPER,
        VerticalAlignSpec::Top => VerticalAlign::TEXT_TOP,
        VerticalAlignSpec::Bottom => VerticalAlign::TEXT_BOTTOM,
        VerticalAlignSpec::Middle => {
            VerticalAlign::new(AlignmentBaseline::Middle, BaselineShift::Length(0.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_list_quotes_named_fonts() {
        let spec = TextStyleSpec {
            families: Arc::from([
                Box::from("Linux Libertine"),
                Box::from("Georgia"),
                Box::from("Serif"),
            ]),
            ..TextStyleSpec::default()
        };
        assert_eq!(
            spec.family_source(),
            format!("\"Linux Libertine\", \"Georgia\", serif{SYMBOL_FALLBACK}")
        );
        let named_only = TextStyleSpec {
            families: Arc::from([Box::from("Arial")]),
            ..TextStyleSpec::default()
        };
        assert_eq!(
            named_only.family_source(),
            format!("\"Arial\", sans-serif{SYMBOL_FALLBACK}")
        );
    }
}
