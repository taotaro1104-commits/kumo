//! 計算値からテキストスタイル・リストマーカーを作る。

use kumo_css::ComputedStyle;
use kumo_css::keywords::{FontStyle, ListStyleType, TextTransform, VerticalAlign, WhiteSpace};
use kumo_css::style::text_decoration;
use kumo_text::{BrushId, LineHeightSpec, TextStyleSpec, VerticalAlignSpec, WhiteSpaceSpec};

pub fn text_spec(style: &ComputedStyle, brush: BrushId) -> TextStyleSpec {
    TextStyleSpec {
        families: style.font_family.clone(),
        size: style.font_size,
        weight: style.font_weight,
        italic: style.font_style == FontStyle::Italic,
        line_height: match style.line_height {
            kumo_css::LineHeight::Normal => LineHeightSpec::Normal,
            kumo_css::LineHeight::Number(n) => LineHeightSpec::Number(n),
            kumo_css::LineHeight::Px(px) => LineHeightSpec::Px(px),
        },
        white_space: match style.white_space {
            WhiteSpace::Normal => WhiteSpaceSpec::Normal,
            WhiteSpace::Nowrap => WhiteSpaceSpec::Nowrap,
            WhiteSpace::Pre => WhiteSpaceSpec::Pre,
            WhiteSpace::PreWrap => WhiteSpaceSpec::PreWrap,
            WhiteSpace::PreLine => WhiteSpaceSpec::PreLine,
        },
        vertical_align: match style.vertical_align {
            VerticalAlign::Baseline => VerticalAlignSpec::Baseline,
            VerticalAlign::Sub => VerticalAlignSpec::Sub,
            VerticalAlign::Super => VerticalAlignSpec::Super,
            VerticalAlign::Top => VerticalAlignSpec::Top,
            VerticalAlign::Middle => VerticalAlignSpec::Middle,
            VerticalAlign::Bottom => VerticalAlignSpec::Bottom,
        },
        underline: style.text_decoration_line & text_decoration::UNDERLINE != 0,
        strikethrough: style.text_decoration_line & text_decoration::LINE_THROUGH != 0,
        brush,
    }
}

/// text-transform を適用する。
pub fn transform_text(text: &str, transform: TextTransform) -> String {
    match transform {
        TextTransform::None => text.to_owned(),
        TextTransform::Uppercase => text.to_uppercase(),
        TextTransform::Lowercase => text.to_lowercase(),
        TextTransform::Capitalize => {
            let mut out = String::with_capacity(text.len());
            let mut at_word_start = true;
            for c in text.chars() {
                if at_word_start && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                } else {
                    out.push(c);
                }
                at_word_start = c.is_whitespace() || c == '-';
            }
            out
        }
    }
}

/// リストマーカーの文字列（`n` は1始まりの序数）。
pub fn marker_text(kind: ListStyleType, n: i32) -> Option<String> {
    Some(match kind {
        ListStyleType::None => return None,
        ListStyleType::Disc => "•".to_owned(),
        ListStyleType::Circle => "◦".to_owned(),
        ListStyleType::Square => "▪".to_owned(),
        ListStyleType::Decimal => format!("{n}."),
        ListStyleType::LowerAlpha => format!("{}.", alpha(n).to_lowercase()),
        ListStyleType::UpperAlpha => format!("{}.", alpha(n)),
        ListStyleType::LowerRoman => format!("{}.", roman(n).to_lowercase()),
        ListStyleType::UpperRoman => format!("{}.", roman(n)),
    })
}

fn alpha(mut n: i32) -> String {
    if n <= 0 {
        return n.to_string();
    }
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    out.iter().rev().collect()
}

fn roman(n: i32) -> String {
    if !(1..4000).contains(&n) {
        return n.to_string();
    }
    let table = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut rest = n;
    let mut out = String::new();
    for (value, numeral) in table {
        while rest >= value {
            out.push_str(numeral);
            rest -= value;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers() {
        assert_eq!(
            marker_text(ListStyleType::Decimal, 3).as_deref(),
            Some("3.")
        );
        assert_eq!(
            marker_text(ListStyleType::LowerAlpha, 28).as_deref(),
            Some("ab.")
        );
        assert_eq!(
            marker_text(ListStyleType::UpperRoman, 1994).as_deref(),
            Some("MCMXCIV.")
        );
        assert_eq!(marker_text(ListStyleType::None, 1), None);
    }

    #[test]
    fn transforms() {
        assert_eq!(
            transform_text("hello world-wide", TextTransform::Capitalize),
            "Hello World-Wide"
        );
        assert_eq!(transform_text("Ab", TextTransform::Uppercase), "AB");
    }
}
