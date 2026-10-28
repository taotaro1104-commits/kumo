//! プロパティ名と値文字列から `Decl` 列を作る（ショートハンド展開を含む）。

use std::sync::Arc;

use cssparser::{Parser, Token};
use kumo_dom::LocalName;

use crate::decl::*;
use crate::keywords::*;
use crate::shorthands;
use crate::style::text_decoration;
use crate::values::*;

/// 宣言1つを解析する。未対応プロパティや不正値は `None`（宣言は無視される）。
pub fn parse_property(name: &str, value: &str) -> Option<Vec<Decl>> {
    if let Some(custom) = name.strip_prefix("--") {
        let _ = custom;
        return Some(vec![Decl::Custom(
            LocalName::from(name),
            Arc::from(value.trim()),
        )]);
    }
    let name = name.to_ascii_lowercase();
    if contains_var(value) {
        return Some(vec![Decl::Unparsed(
            LocalName::from(name),
            Arc::from(value),
        )]);
    }
    let mut parser = Parser::new(value);
    if let Ok(wide) = parser.try_parse(|i| {
        let ident = i.expect_ident()?.clone();
        i.expect_exhausted()?;
        WideKeyword::from_ident(&ident).map_or_else(invalid, Ok)
    }) {
        return Some(vec![Decl::Wide(LocalName::from(name), wide)]);
    }
    let mut out = Vec::with_capacity(1);
    parser
        .parse_entirely(|i| parse_declaration(&name, i, &mut out))
        .ok()?;
    Some(out)
}

fn contains_var(value: &str) -> bool {
    value
        .as_bytes()
        .windows(4)
        .any(|w| w.eq_ignore_ascii_case(b"var("))
}

/// 単一キーワードのプロパティを Decl に変換するヘルパ。
macro_rules! keyword {
    ($input:expr, $out:expr, $ty:ident, $variant:ident) => {{
        $out.push(Decl::$variant($ty::parse($input)?));
        Ok(())
    }};
}

fn parse_declaration<'i>(
    name: &str,
    input: &mut Parser<'i>,
    out: &mut Vec<Decl>,
) -> ParseResult<()> {
    match name {
        "display" => keyword!(input, out, Display, Display),
        "position" => keyword!(input, out, Position, Position),
        "float" => keyword!(input, out, Float, Float),
        "clear" => keyword!(input, out, Clear, Clear),
        "box-sizing" => keyword!(input, out, BoxSizing, BoxSizing),
        "overflow-x" => keyword!(input, out, Overflow, OverflowX),
        "overflow-y" => keyword!(input, out, Overflow, OverflowY),
        "flex-direction" => keyword!(input, out, FlexDirection, FlexDirection),
        "flex-wrap" => keyword!(input, out, FlexWrap, FlexWrap),
        "justify-content" => parse_align(input).map(|a| out.push(Decl::JustifyContent(a))),
        "align-items" => parse_align(input).map(|a| out.push(Decl::AlignItems(a))),
        "align-self" => parse_align(input).map(|a| out.push(Decl::AlignSelf(a))),
        "align-content" => parse_align(input).map(|a| out.push(Decl::AlignContent(a))),
        "border-collapse" => keyword!(input, out, BorderCollapse, BorderCollapse),
        "font-style" => keyword!(input, out, FontStyle, FontStyle),
        "text-align" => keyword!(input, out, TextAlign, TextAlign),
        "text-transform" => keyword!(input, out, TextTransform, TextTransform),
        "white-space" | "white-space-collapse" => keyword!(input, out, WhiteSpace, WhiteSpace),
        "content" => parse_content(input).map(|c| out.push(Decl::Content(c))),
        "list-style-type" => keyword!(input, out, ListStyleType, ListStyleType),
        "list-style-position" => keyword!(input, out, ListStylePosition, ListStylePosition),
        "visibility" => keyword!(input, out, Visibility, Visibility),
        "vertical-align" => parse_vertical_align(input).map(|v| out.push(Decl::VerticalAlign(v))),
        "color" => parse_color(input).map(|c| out.push(Decl::Color(c))),
        "background-color" => parse_color(input).map(|c| out.push(Decl::BackgroundColor(c))),
        "opacity" => parse_number_or_pct(input).map(|n| out.push(Decl::Opacity(n.clamp(0.0, 1.0)))),
        "flex-grow" => parse_non_negative_number(input).map(|n| out.push(Decl::FlexGrow(n))),
        "flex-shrink" => parse_non_negative_number(input).map(|n| out.push(Decl::FlexShrink(n))),
        "flex-basis" => parse_flex_basis(input).map(|l| out.push(Decl::FlexBasis(l))),
        "z-index" => {
            let value = if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
                None
            } else {
                Some(input.expect_integer()?)
            };
            out.push(Decl::ZIndex(value));
            Ok(())
        }
        "order" => {
            out.push(Decl::Order(input.expect_integer()?));
            Ok(())
        }
        "row-gap" => parse_gap(input).map(|l| out.push(Decl::RowGap(l))),
        "column-gap" => parse_gap(input).map(|l| out.push(Decl::ColumnGap(l))),
        "border-spacing" => parse_length(input).map(|l| out.push(Decl::BorderSpacing(l))),
        "text-indent" => parse_length(input).map(|l| out.push(Decl::TextIndent(l))),
        "font-size" => parse_font_size(input).map(|v| out.push(Decl::FontSize(v))),
        "font-weight" => parse_font_weight(input).map(|v| out.push(Decl::FontWeight(v))),
        "font-family" => parse_font_family(input).map(|v| out.push(Decl::FontFamily(v))),
        "line-height" => parse_line_height(input).map(|v| out.push(Decl::LineHeight(v))),
        "text-decoration-line" | "text-decoration" => {
            parse_text_decoration(input).map(|v| out.push(Decl::TextDecorationLine(v)))
        }
        _ => parse_size_or_grid(name, input, out)
            .unwrap_or_else(|| shorthands::parse_shorthand(name, input, out)),
    }
}

fn parse_size_or_grid<'i>(
    name: &str,
    input: &mut Parser<'i>,
    out: &mut Vec<Decl>,
) -> Option<ParseResult<()>> {
    let size = match name {
        "width" | "inline-size" => Some(SizeProp::Width),
        "height" | "block-size" => Some(SizeProp::Height),
        "min-width" | "min-inline-size" => Some(SizeProp::MinWidth),
        "min-height" | "min-block-size" => Some(SizeProp::MinHeight),
        "max-width" | "max-inline-size" => Some(SizeProp::MaxWidth),
        "max-height" | "max-block-size" => Some(SizeProp::MaxHeight),
        _ => None,
    };
    if let Some(prop) = size {
        return Some(parse_size(input).map(|v| out.push(Decl::Size(prop, v))));
    }
    let grid = match name {
        "grid-template-columns" => GridProp::TemplateColumns,
        "grid-template-rows" => GridProp::TemplateRows,
        "grid-template-areas" => GridProp::TemplateAreas,
        "grid-auto-columns" => GridProp::AutoColumns,
        "grid-auto-rows" => GridProp::AutoRows,
        "grid-auto-flow" => GridProp::AutoFlow,
        "grid-column-start" => GridProp::ColumnStart,
        "grid-column-end" => GridProp::ColumnEnd,
        "grid-row-start" => GridProp::RowStart,
        "grid-row-end" => GridProp::RowEnd,
        _ => return None,
    };
    Some({
        out.push(Decl::Grid(grid, Some(rest_of_input(input))));
        Ok(())
    })
}

/// 残りのトークン列をそのまま文字列として取り出す（Taffy 側で解析する値用）。
pub(crate) fn rest_of_input(input: &mut Parser<'_>) -> Arc<str> {
    let start = input.position();
    while input.next().is_ok() {}
    Arc::from(input.slice_from(start).trim())
}

/// width 等：`auto` / `none` / 長さ。`fit-content` 等の内在サイズは auto 扱い。
fn parse_size<'i>(input: &mut Parser<'i>) -> ParseResult<LengthOrAuto> {
    if let Ok(ident) = input.try_parse(|i| i.expect_ident_cloned()) {
        let lower = ident.to_ascii_lowercase();
        return match lower.as_str() {
            "auto"
            | "none"
            | "fit-content"
            | "max-content"
            | "min-content"
            | "-moz-fit-content"
            | "-webkit-fill-available"
            | "stretch" => Ok(LengthOrAuto::Auto),
            _ => invalid(),
        };
    }
    non_negative(parse_length(input)?).map(LengthOrAuto::Length)
}

fn parse_flex_basis<'i>(input: &mut Parser<'i>) -> ParseResult<LengthOrAuto> {
    if input
        .try_parse(|i| i.expect_ident_matching("content"))
        .is_ok()
    {
        return Ok(LengthOrAuto::Auto);
    }
    parse_size(input)
}

fn parse_gap<'i>(input: &mut Parser<'i>) -> ParseResult<Length> {
    if input
        .try_parse(|i| i.expect_ident_matching("normal"))
        .is_ok()
    {
        return Ok(Length::ZERO);
    }
    parse_length(input)
}

pub(crate) fn non_negative(l: Length) -> ParseResult<Length> {
    let all = [l.px, l.em, l.rem, l.vw, l.vh, l.pct];
    if all.iter().all(|v| *v >= 0.0) || all.iter().any(|v| *v > 0.0) {
        Ok(l)
    } else {
        invalid()
    }
}

fn parse_non_negative_number<'i>(input: &mut Parser<'i>) -> ParseResult<f32> {
    let n = input.expect_number()?;
    if n >= 0.0 { Ok(n) } else { invalid() }
}

fn parse_number_or_pct<'i>(input: &mut Parser<'i>) -> ParseResult<f32> {
    match *input.next()? {
        Token::Number { value, .. } => Ok(value),
        Token::Percentage { unit_value, .. } => Ok(unit_value),
        _ => invalid(),
    }
}

fn parse_align<'i>(input: &mut Parser<'i>) -> ParseResult<Align> {
    // `safe center` / `first baseline` などの修飾子は読み飛ばす
    let _ = input.try_parse(|i| {
        let ident = i.expect_ident()?.clone();
        if ["safe", "unsafe", "first", "last", "legacy"]
            .iter()
            .any(|m| ident.eq_ignore_ascii_case(m))
            && !i.is_exhausted()
        {
            Ok(())
        } else {
            invalid()
        }
    });
    Align::parse(input)
}

fn parse_vertical_align<'i>(input: &mut Parser<'i>) -> ParseResult<VerticalAlign> {
    if let Ok(v) = input.try_parse(VerticalAlign::parse) {
        return Ok(v);
    }
    // 長さ指定はベースライン扱い（Phase 0）
    parse_length(input).map(|_| VerticalAlign::Baseline)
}

pub(crate) fn parse_font_size<'i>(input: &mut Parser<'i>) -> ParseResult<FontSizeValue> {
    if let Ok(ident) = input.try_parse(|i| i.expect_ident_cloned()) {
        let lower = ident.to_ascii_lowercase();
        let px = match lower.as_str() {
            "xx-small" => 9.0,
            "x-small" => 10.0,
            "small" => 13.0,
            "medium" => 16.0,
            "large" => 18.0,
            "x-large" => 24.0,
            "xx-large" => 32.0,
            "xxx-large" => 48.0,
            "smaller" => return Ok(FontSizeValue::Smaller),
            "larger" => return Ok(FontSizeValue::Larger),
            _ => return invalid(),
        };
        return Ok(FontSizeValue::Length(Length::px(px)));
    }
    non_negative(parse_length(input)?).map(FontSizeValue::Length)
}

pub(crate) fn parse_font_weight<'i>(input: &mut Parser<'i>) -> ParseResult<FontWeightValue> {
    if let Ok(n) = input.try_parse(|i| i.expect_number()) {
        return if (1.0..=1000.0).contains(&n) {
            Ok(FontWeightValue::Absolute(n as u16))
        } else {
            invalid()
        };
    }
    let ident = input.expect_ident()?.clone();
    let lower = ident.to_ascii_lowercase();
    match lower.as_str() {
        "normal" => Ok(FontWeightValue::Absolute(400)),
        "bold" => Ok(FontWeightValue::Absolute(700)),
        "bolder" => Ok(FontWeightValue::Bolder),
        "lighter" => Ok(FontWeightValue::Lighter),
        _ => invalid(),
    }
}

pub(crate) fn parse_line_height<'i>(input: &mut Parser<'i>) -> ParseResult<LineHeightValue> {
    if input
        .try_parse(|i| i.expect_ident_matching("normal"))
        .is_ok()
    {
        return Ok(LineHeightValue::Normal);
    }
    if let Ok(n) = input.try_parse(|i| i.expect_number()) {
        return Ok(LineHeightValue::Number(n));
    }
    let l = non_negative(parse_length(input)?)?;
    // パーセントは em に換算（line-height の % はフォントサイズ基準）
    Ok(LineHeightValue::Length(Length {
        em: l.em + l.pct / 100.0,
        pct: 0.0,
        ..l
    }))
}

pub(crate) fn parse_font_family<'i>(input: &mut Parser<'i>) -> ParseResult<Arc<[Box<str>]>> {
    let families = input.parse_comma_separated(|i| {
        if let Ok(s) = i.try_parse(|i| i.expect_string_cloned()) {
            return Ok(Box::from(&*s));
        }
        let mut name = String::new();
        while let Ok(ident) = i.try_parse(|i| i.expect_ident_cloned()) {
            if !name.is_empty() {
                name.push(' ');
            }
            name.push_str(&ident);
        }
        if name.is_empty() {
            invalid()
        } else {
            Ok(name.into_boxed_str())
        }
    })?;
    Ok(Arc::from(families))
}

fn parse_text_decoration<'i>(input: &mut Parser<'i>) -> ParseResult<u8> {
    let mut bits = 0;
    let mut any = false;
    while !input.is_exhausted() {
        if let Ok(ident) = input.try_parse(|i| i.expect_ident_cloned()) {
            let lower = ident.to_ascii_lowercase();
            bits |= match lower.as_str() {
                "underline" => text_decoration::UNDERLINE,
                "overline" => text_decoration::OVERLINE,
                "line-through" => text_decoration::LINE_THROUGH,
                _ => 0,
            };
            any = true;
        } else if input.try_parse(parse_color).is_ok() || input.try_parse(parse_length).is_ok() {
            // text-decoration ショートハンドの色・太さは無視
            any = true;
        } else {
            return invalid();
        }
    }
    if any { Ok(bits) } else { invalid() }
}

/// `content`：`none` / `normal`、文字列、`attr(x)`、`open-quote` / `close-quote` の並び。
/// カウンターや `url()` などは無視する（その部品は空）。
fn parse_content<'i>(input: &mut Parser<'i>) -> ParseResult<Option<Arc<[ContentPart]>>> {
    let mut parts: Vec<ContentPart> = Vec::new();
    let mut first = true;
    while !input.is_exhausted() {
        let token = input.next()?.clone();
        match token {
            Token::Ident(ident) if first && ident.eq_ignore_ascii_case("none") => {
                return Ok(None);
            }
            Token::Ident(ident) if first && ident.eq_ignore_ascii_case("normal") => {
                return Ok(None);
            }
            Token::Ident(ident)
                if ident.eq_ignore_ascii_case("open-quote")
                    || ident.eq_ignore_ascii_case("close-quote") =>
            {
                parts.push(ContentPart::Text(Arc::from("\"")));
            }
            Token::QuotedString(text) => parts.push(ContentPart::Text(Arc::from(&*text))),
            Token::Function(name) => {
                let is_attr = name.eq_ignore_ascii_case("attr");
                let argument: ParseResult<Option<Arc<str>>> = input.parse_nested_block(|i| {
                    let found = i.try_parse(|i| i.expect_ident().map(|s| Arc::from(&**s)));
                    while i.next().is_ok() {}
                    Ok(found.ok())
                });
                if let (true, Ok(Some(attr))) = (is_attr, argument) {
                    parts.push(ContentPart::Attr(attr));
                }
            }
            _ => {}
        }
        first = false;
    }
    Ok(Some(Arc::from(parts)))
}
