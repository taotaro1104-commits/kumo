//! ショートハンドと論理プロパティの展開。

use std::sync::Arc;

use cssparser::Parser;

use crate::decl::*;
use crate::keywords::*;
use crate::properties::{
    non_negative, parse_font_family, parse_font_size, parse_font_weight, parse_line_height,
    rest_of_input,
};
use crate::style::Side;
use crate::values::*;

pub(crate) fn parse_shorthand<'i>(
    name: &str,
    input: &mut Parser<'i>,
    out: &mut Vec<Decl>,
) -> ParseResult<()> {
    if let Some(sides) = name.strip_prefix("margin").and_then(sides_for) {
        return parse_sides(input, sides, parse_length_or_auto, |s, v| {
            out.push(Decl::Margin(s, v))
        });
    }
    if let Some(sides) = name.strip_prefix("padding").and_then(sides_for) {
        let p = |i: &mut Parser<'i>| non_negative(parse_length(i)?);
        return parse_sides(input, sides, p, |s, v| out.push(Decl::Padding(s, v)));
    }
    if let Some(sides) = inset_sides(name) {
        return parse_sides(input, sides, parse_length_or_auto, |s, v| {
            out.push(Decl::Inset(s, v))
        });
    }
    if let Some(rest) = name.strip_prefix("border") {
        return parse_border(rest, input, out);
    }
    match name {
        "background" => parse_background(input, out),
        "font" => parse_font(input, out),
        "flex" => parse_flex(input, out),
        "flex-flow" => parse_flex_flow(input, out),
        "gap" | "grid-gap" => parse_gap_shorthand(input, out),
        "overflow" => parse_overflow(input, out),
        "list-style" => parse_list_style(input, out),
        "place-items" => {
            out.push(Decl::AlignItems(Align::parse(input)?));
            Ok(())
        }
        "place-content" => {
            out.push(Decl::AlignContent(Align::parse(input)?));
            Ok(())
        }
        "grid-column" => {
            parse_grid_line_pair(input, out, GridProp::ColumnStart, GridProp::ColumnEnd)
        }
        "grid-row" => parse_grid_line_pair(input, out, GridProp::RowStart, GridProp::RowEnd),
        "grid-area" => parse_grid_area(input, out),
        _ => invalid(),
    }
}

/// `margin` 等の接尾辞から対象の辺を返す（論理プロパティは横書きLTRで解決）。
fn sides_for(suffix: &str) -> Option<&'static [Side]> {
    Some(match suffix {
        "" => &Side::ALL,
        "-top" | "-block-start" => &[Side::Top],
        "-right" | "-inline-end" => &[Side::Right],
        "-bottom" | "-block-end" => &[Side::Bottom],
        "-left" | "-inline-start" => &[Side::Left],
        "-block" => &[Side::Top, Side::Bottom],
        "-inline" => &[Side::Left, Side::Right],
        _ => return None,
    })
}

fn inset_sides(name: &str) -> Option<&'static [Side]> {
    match name {
        "top" => Some(&[Side::Top]),
        "right" => Some(&[Side::Right]),
        "bottom" => Some(&[Side::Bottom]),
        "left" => Some(&[Side::Left]),
        _ => name.strip_prefix("inset").and_then(sides_for),
    }
}

/// 1〜4値（または論理ペアの1〜2値）構文で辺ごとの値を割り当てる。
fn parse_sides<'i, T: Copy>(
    input: &mut Parser<'i>,
    sides: &[Side],
    mut parse_one: impl FnMut(&mut Parser<'i>) -> ParseResult<T>,
    mut emit: impl FnMut(Side, T),
) -> ParseResult<()> {
    let mut values = Vec::with_capacity(4);
    while !input.is_exhausted() && values.len() < 4 {
        values.push(parse_one(input)?);
    }
    let assigned: Vec<T> = match (sides.len(), values.as_slice()) {
        (1, [v]) => vec![*v],
        (2, [v]) => vec![*v, *v],
        (2, [a, b]) => vec![*a, *b],
        (4, [v]) => vec![*v; 4],
        (4, [tb, lr]) => vec![*tb, *lr, *tb, *lr],
        (4, [t, lr, b]) => vec![*t, *lr, *b, *lr],
        (4, [t, r, b, l]) => vec![*t, *r, *b, *l],
        _ => return invalid(),
    };
    for (side, value) in sides.iter().zip(assigned) {
        emit(*side, value);
    }
    Ok(())
}

/// `border`, `border-top`, `border-width`, `border-inline-start-color` 等。
fn parse_border<'i>(rest: &str, input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    for (suffix, kind) in [("-width", 0), ("-style", 1), ("-color", 2)] {
        if let Some(sides) = rest.strip_suffix(suffix).and_then(sides_for) {
            return match kind {
                0 => parse_sides(input, sides, parse_border_width, |s, v| {
                    out.push(Decl::BorderWidth(s, v))
                }),
                1 => parse_sides(input, sides, BorderStyle::parse, |s, v| {
                    out.push(Decl::BorderStyle(s, v))
                }),
                _ => parse_sides(input, sides, parse_color, |s, v| {
                    out.push(Decl::BorderColor(s, v))
                }),
            };
        }
    }
    if rest.contains("radius") || rest.contains("image") {
        return invalid(); // Phase 0 では未対応
    }
    let sides = sides_for(rest).map_or_else(invalid, Ok)?;
    let (mut width, mut style, mut color) = (None, None, None);
    while !input.is_exhausted() {
        if width.is_none()
            && let Ok(w) = input.try_parse(parse_border_width)
        {
            width = Some(w);
        } else if style.is_none()
            && let Ok(s) = input.try_parse(BorderStyle::parse)
        {
            style = Some(s);
        } else if color.is_none()
            && let Ok(c) = input.try_parse(parse_color)
        {
            color = Some(c);
        } else {
            return invalid();
        }
    }
    for &side in sides {
        out.push(Decl::BorderWidth(side, width.unwrap_or(Length::px(3.0))));
        out.push(Decl::BorderStyle(side, style.unwrap_or_default()));
        out.push(Decl::BorderColor(
            side,
            color.unwrap_or(ColorValue::CurrentColor),
        ));
    }
    Ok(())
}

fn parse_border_width<'i>(input: &mut Parser<'i>) -> ParseResult<Length> {
    if let Ok(ident) = input.try_parse(|i| i.expect_ident_cloned()) {
        let lower = ident.to_ascii_lowercase();
        return match lower.as_str() {
            "thin" => Ok(Length::px(1.0)),
            "medium" => Ok(Length::px(3.0)),
            "thick" => Ok(Length::px(5.0)),
            _ => invalid(),
        };
    }
    non_negative(parse_length(input)?)
}

/// background ショートハンドからは色だけを取り出す。色がなければ透明にリセット。
fn parse_background<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    let mut color = ColorValue::Rgba(Rgba::TRANSPARENT);
    while !input.is_exhausted() {
        if let Ok(c) = input.try_parse(parse_color) {
            color = c;
        } else {
            input.next()?; // url() や位置指定などは読み飛ばす
        }
    }
    out.push(Decl::BackgroundColor(color));
    Ok(())
}

/// `font: [style] [weight] size[/line-height] family`
fn parse_font<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    let (mut style, mut weight) = (FontStyle::Normal, FontWeightValue::Absolute(400));
    let size = loop {
        if let Ok(s) = input.try_parse(parse_font_size) {
            break s;
        }
        if let Ok(s) = input.try_parse(FontStyle::parse) {
            style = s;
        } else if let Ok(w) = input.try_parse(parse_font_weight) {
            weight = w;
        } else {
            // normal / small-caps / condensed などは無視
            let ident = input.expect_ident()?.clone();
            let ok = ["normal", "small-caps"]
                .iter()
                .any(|k| ident.eq_ignore_ascii_case(k))
                || ident.to_ascii_lowercase().ends_with("condensed")
                || ident.to_ascii_lowercase().ends_with("expanded");
            if !ok {
                return invalid();
            }
        }
    };
    let line_height = if input.try_parse(|i| i.expect_delim('/')).is_ok() {
        parse_line_height(input)?
    } else {
        LineHeightValue::Normal
    };
    let family = parse_font_family(input)?;
    out.extend([
        Decl::FontStyle(style),
        Decl::FontWeight(weight),
        Decl::FontSize(size),
        Decl::LineHeight(line_height),
        Decl::FontFamily(family),
    ]);
    Ok(())
}

/// `flex: none | auto | <grow> [<shrink>] [<basis>]`
fn parse_flex<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    let (grow, shrink, basis) = if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        (0.0, 0.0, LengthOrAuto::Auto)
    } else if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        (1.0, 1.0, LengthOrAuto::Auto)
    } else {
        let mut grow = None;
        let mut shrink = None;
        let mut basis = None;
        while !input.is_exhausted() {
            if let Ok(n) = input.try_parse(|i| i.expect_number()) {
                if grow.is_none() {
                    grow = Some(n);
                } else if shrink.is_none() {
                    shrink = Some(n);
                } else {
                    return invalid();
                }
            } else if basis.is_none() {
                basis = Some(parse_length_or_auto(input)?);
            } else {
                return invalid();
            }
        }
        let basis = basis.unwrap_or(LengthOrAuto::Length(Length::ZERO));
        (grow.unwrap_or(1.0), shrink.unwrap_or(1.0), basis)
    };
    out.extend([
        Decl::FlexGrow(grow),
        Decl::FlexShrink(shrink),
        Decl::FlexBasis(basis),
    ]);
    Ok(())
}

fn parse_flex_flow<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    while !input.is_exhausted() {
        if let Ok(d) = input.try_parse(FlexDirection::parse) {
            out.push(Decl::FlexDirection(d));
        } else {
            out.push(Decl::FlexWrap(FlexWrap::parse(input)?));
        }
    }
    Ok(())
}

fn parse_gap_shorthand<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    let row = parse_length(input)?;
    let column = input.try_parse(parse_length).unwrap_or(row);
    out.extend([Decl::RowGap(row), Decl::ColumnGap(column)]);
    Ok(())
}

fn parse_overflow<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    let x = Overflow::parse(input)?;
    let y = input.try_parse(Overflow::parse).unwrap_or(x);
    out.extend([Decl::OverflowX(x), Decl::OverflowY(y)]);
    Ok(())
}

fn parse_list_style<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    let (mut ty, mut pos) = (ListStyleType::Disc, ListStylePosition::Outside);
    while !input.is_exhausted() {
        if let Ok(p) = input.try_parse(ListStylePosition::parse) {
            pos = p;
        } else if let Ok(t) = input.try_parse(ListStyleType::parse) {
            ty = t;
        } else {
            input.next()?; // list-style-image (url) は無視
        }
    }
    out.extend([Decl::ListStyleType(ty), Decl::ListStylePosition(pos)]);
    Ok(())
}

fn grid_value(s: &str) -> Option<Arc<str>> {
    let trimmed = s.trim();
    (!trimmed.is_empty()).then(|| Arc::from(trimmed))
}

/// `grid-column: 1 / 3` を start / end に分ける。
fn parse_grid_line_pair<'i>(
    input: &mut Parser<'i>,
    out: &mut Vec<Decl>,
    start: GridProp,
    end: GridProp,
) -> ParseResult<()> {
    let raw = rest_of_input(input);
    let mut parts = raw.splitn(2, '/');
    let first = parts.next().and_then(grid_value);
    let second = parts.next().and_then(grid_value);
    out.push(Decl::Grid(start, first.clone()));
    out.push(Decl::Grid(end, second));
    Ok(())
}

/// `grid-area: name` または `row-start / column-start / row-end / column-end`。
fn parse_grid_area<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    let raw = rest_of_input(input);
    let parts: Vec<Option<Arc<str>>> = raw.split('/').map(grid_value).collect();
    let first = parts.first().cloned().flatten();
    // 省略された値は対になる start 側（さらに無ければ先頭）の名前を使う
    let get = |i: usize| {
        parts
            .get(i)
            .cloned()
            .flatten()
            .or_else(|| {
                i.checked_sub(2)
                    .and_then(|j| parts.get(j).cloned().flatten())
            })
            .or_else(|| first.clone())
    };
    out.extend([
        Decl::Grid(GridProp::RowStart, get(0)),
        Decl::Grid(GridProp::ColumnStart, get(1)),
        Decl::Grid(GridProp::RowEnd, get(2)),
        Decl::Grid(GridProp::ColumnEnd, get(3)),
    ]);
    Ok(())
}
