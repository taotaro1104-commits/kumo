//! HTML の表示系属性（width / bgcolor / align など）を宣言に変換する。

use kumo_dom::ElementData;

use crate::decl::{Decl, SizeProp};
use crate::keywords::{TextAlign, VerticalAlign};
use crate::style::Side;
use crate::values::{ColorValue, Length, LengthOrAuto, Rgba};

const SIZED: [&str; 8] = [
    "img", "table", "td", "th", "col", "iframe", "video", "canvas",
];

/// 表示系属性から宣言を作る。作者スタイルより優先度は低い。
pub fn presentational_hints(element: &ElementData) -> Vec<Decl> {
    let mut out = Vec::new();
    if SIZED.iter().any(|t| element.is_html(t)) {
        if let Some(w) = element.attr("width").and_then(dimension) {
            out.push(Decl::Size(SizeProp::Width, w));
        }
        if let Some(h) = element.attr("height").and_then(dimension) {
            out.push(Decl::Size(SizeProp::Height, h));
        }
    }
    if let Some(color) = element.attr("bgcolor").and_then(legacy_color) {
        out.push(Decl::BackgroundColor(color));
    }
    if element.is_html("font")
        && let Some(color) = element.attr("color").and_then(legacy_color)
    {
        out.push(Decl::Color(color));
    }
    if element.is_html("body")
        && let Some(color) = element.attr("text").and_then(legacy_color)
    {
        out.push(Decl::Color(color));
    }
    if let Some(align) = element.attr("align") {
        push_align(element, align, &mut out);
    }
    if let Some(valign) = element.attr("valign").and_then(VerticalAlign::from_ident) {
        out.push(Decl::VerticalAlign(valign));
    }
    if element.is_html("table") {
        table_hints(element, &mut out);
    }
    out
}

fn push_align(element: &ElementData, align: &str, out: &mut Vec<Decl>) {
    let is_cell_like = [
        "td", "th", "tr", "div", "p", "h1", "h2", "h3", "h4", "h5", "h6", "caption",
    ]
    .iter()
    .any(|t| element.is_html(t));
    if is_cell_like {
        if let Some(a) = TextAlign::from_ident(align) {
            out.push(Decl::TextAlign(a));
        }
    } else if element.is_html("table") && align.eq_ignore_ascii_case("center") {
        out.push(Decl::Margin(Side::Left, LengthOrAuto::Auto));
        out.push(Decl::Margin(Side::Right, LengthOrAuto::Auto));
    }
}

fn table_hints(element: &ElementData, out: &mut Vec<Decl>) {
    if let Some(border) = element
        .attr("border")
        .and_then(|b| b.trim().parse::<f32>().ok())
    {
        for side in Side::ALL {
            out.push(Decl::BorderWidth(side, Length::px(border)));
            out.push(Decl::BorderStyle(
                side,
                crate::keywords::BorderStyle::Outset,
            ));
            out.push(Decl::BorderColor(
                side,
                ColorValue::Rgba(Rgba::new(128, 128, 128, 255)),
            ));
        }
    }
    if let Some(spacing) = element
        .attr("cellspacing")
        .and_then(|s| s.trim().parse::<f32>().ok())
    {
        out.push(Decl::BorderSpacing(Length::px(spacing)));
    }
}

/// `100` → 100px、`50%` → 50%。
fn dimension(value: &str) -> Option<LengthOrAuto> {
    let value = value.trim();
    if let Some(pct) = value.strip_suffix('%') {
        return pct
            .trim()
            .parse()
            .ok()
            .map(|p| LengthOrAuto::Length(Length::percent(p)));
    }
    let digits: String = value
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits
        .parse::<f32>()
        .ok()
        .map(|px| LengthOrAuto::Length(Length::px(px)))
}

fn legacy_color(value: &str) -> Option<ColorValue> {
    let value = value.trim();
    let hex = value.strip_prefix('#').unwrap_or(value);
    if let Ok((r, g, b, a)) = cssparser::color::parse_hash_color(hex.as_bytes()) {
        return Some(ColorValue::Rgba(Rgba::new(r, g, b, (a * 255.0) as u8)));
    }
    cssparser::color::parse_named_color(&value.to_ascii_lowercase())
        .ok()
        .map(|(r, g, b)| ColorValue::Rgba(Rgba::new(r, g, b, 255)))
}
