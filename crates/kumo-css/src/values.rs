//! CSS の値型（色・長さ）とそのパーサ。

use cssparser::{Parser, Token};

pub use crate::color::parse_color;

pub type ParseResult<T> = Result<T, cssparser::ParseError<()>>;

/// 値が不正であることを表すエラー。
pub fn invalid<T>() -> ParseResult<T> {
    Err(cssparser::ParseError::custom(()))
}

/// 8bit RGBA。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn is_transparent(self) -> bool {
        self.a == 0
    }
}

/// 指定値としての色。`currentcolor` は計算時に `color` で置き換える。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorValue {
    Rgba(Rgba),
    CurrentColor,
}

/// 長さの線形結合（`calc(100% - 2em)` 等をそのまま表現できる）。
/// 絶対単位はパース時に px へ換算する。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Length {
    pub px: f32,
    pub em: f32,
    pub rem: f32,
    pub vw: f32,
    pub vh: f32,
    /// パーセント値（50% → 50.0）
    pub pct: f32,
}

impl Length {
    pub const ZERO: Self = Self {
        px: 0.0,
        em: 0.0,
        rem: 0.0,
        vw: 0.0,
        vh: 0.0,
        pct: 0.0,
    };

    pub fn px(px: f32) -> Self {
        Self { px, ..Self::ZERO }
    }

    pub fn em(em: f32) -> Self {
        Self { em, ..Self::ZERO }
    }

    pub fn percent(pct: f32) -> Self {
        Self { pct, ..Self::ZERO }
    }

    fn add(self, o: Self) -> Self {
        Self {
            px: self.px + o.px,
            em: self.em + o.em,
            rem: self.rem + o.rem,
            vw: self.vw + o.vw,
            vh: self.vh + o.vh,
            pct: self.pct + o.pct,
        }
    }

    fn scale(self, k: f32) -> Self {
        Self {
            px: self.px * k,
            em: self.em * k,
            rem: self.rem * k,
            vw: self.vw * k,
            vh: self.vh * k,
            pct: self.pct * k,
        }
    }

    fn from_dimension(value: f32, unit: &str) -> Option<Self> {
        let unit = unit.to_ascii_lowercase();
        let l = match unit.as_str() {
            "px" => Self::px(value),
            "em" => Self::em(value),
            "rem" => Self {
                rem: value,
                ..Self::ZERO
            },
            "ex" | "ch" => Self::em(value * 0.5),
            "vw" => Self {
                vw: value,
                ..Self::ZERO
            },
            "vh" => Self {
                vh: value,
                ..Self::ZERO
            },
            "vmin" | "vmax" => Self {
                vw: value,
                ..Self::ZERO
            },
            "pt" => Self::px(value * 4.0 / 3.0),
            "pc" => Self::px(value * 16.0),
            "in" => Self::px(value * 96.0),
            "cm" => Self::px(value * 96.0 / 2.54),
            "mm" => Self::px(value * 96.0 / 25.4),
            "q" => Self::px(value * 96.0 / 101.6),
            _ => return None,
        };
        Some(l)
    }
}

/// `auto` を取りうる長さ。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LengthOrAuto {
    Length(Length),
    Auto,
}

/// 長さ（`auto`/`none` 不可）。負値を許すかは呼び出し側が決める。
pub fn parse_length<'i>(input: &mut Parser<'i>) -> ParseResult<Length> {
    let token = input.next()?.clone();
    match token {
        Token::Dimension {
            value, ref unit, ..
        } => Length::from_dimension(value, unit).map_or_else(invalid, Ok),
        Token::Percentage { unit_value, .. } => Ok(Length::percent(unit_value * 100.0)),
        // unitless 0 と quirks の数値は px とみなす
        Token::Number { value: 0.0, .. } => Ok(Length::ZERO),
        Token::Function(ref name) if is_math_function(name) => {
            let name = name.to_ascii_lowercase();
            input.parse_nested_block(|i| parse_math_function(&name, i))
        }
        _ => invalid(),
    }
}

pub fn parse_length_or_auto<'i>(input: &mut Parser<'i>) -> ParseResult<LengthOrAuto> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(LengthOrAuto::Auto);
    }
    parse_length(input).map(LengthOrAuto::Length)
}

fn is_math_function(name: &str) -> bool {
    ["calc", "min", "max", "clamp"]
        .iter()
        .any(|f| name.eq_ignore_ascii_case(f))
}

/// min()/max()/clamp() は解決前の比較ができないため、代表値で近似する（Phase 0）。
fn parse_math_function<'i>(name: &str, input: &mut Parser<'i>) -> ParseResult<Length> {
    match name {
        "calc" => parse_sum(input),
        _ => {
            let args = input.parse_comma_separated(parse_sum)?;
            let pick = match name {
                "clamp" => args.get(1),
                "max" => args.last(),
                _ => args.first(),
            };
            pick.copied().map_or_else(invalid, Ok)
        }
    }
}

fn parse_sum<'i>(input: &mut Parser<'i>) -> ParseResult<Length> {
    let mut total = parse_product(input)?;
    loop {
        let sign = match input.try_parse(|i| i.next().cloned()) {
            Ok(Token::Delim('+')) => 1.0,
            Ok(Token::Delim('-')) => -1.0,
            Ok(_) => return invalid(),
            Err(_) => return Ok(total),
        };
        total = total.add(parse_product(input)?.scale(sign));
    }
}

fn parse_product<'i>(input: &mut Parser<'i>) -> ParseResult<Length> {
    let mut value = parse_calc_term(input)?;
    loop {
        let op = input.try_parse(|i| match i.next()? {
            Token::Delim(c @ ('*' | '/')) => Ok(*c),
            _ => invalid(),
        });
        let Ok(op) = op else { break };
        let rhs = parse_calc_term(input)?;
        value = match (op, value, rhs) {
            ('*', CalcTerm::Number(n), rhs) => rhs.scale_by(n),
            ('*', lhs, CalcTerm::Number(n)) => lhs.scale_by(n),
            ('/', lhs, CalcTerm::Number(n)) if n != 0.0 => lhs.scale_by(1.0 / n),
            _ => return invalid(),
        };
    }
    value.into_length()
}

#[derive(Clone, Copy)]
enum CalcTerm {
    Number(f32),
    Length(Length),
}

impl CalcTerm {
    fn scale_by(self, k: f32) -> Self {
        match self {
            Self::Number(n) => Self::Number(n * k),
            Self::Length(l) => Self::Length(l.scale(k)),
        }
    }

    fn into_length(self) -> ParseResult<Length> {
        match self {
            Self::Length(l) => Ok(l),
            Self::Number(0.0) => Ok(Length::ZERO),
            Self::Number(_) => invalid(),
        }
    }
}

fn parse_calc_term<'i>(input: &mut Parser<'i>) -> ParseResult<CalcTerm> {
    if let Ok(n) = input.try_parse(|i| i.expect_number()) {
        return Ok(CalcTerm::Number(n));
    }
    if input.try_parse(|i| i.expect_parenthesis_block()).is_ok() {
        return input.parse_nested_block(parse_sum).map(CalcTerm::Length);
    }
    parse_length(input).map(CalcTerm::Length)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse<T>(s: &str, f: impl for<'i> FnOnce(&mut Parser<'i>) -> ParseResult<T>) -> Option<T> {
        let mut parser = Parser::new(s);
        parser.parse_entirely(f).ok()
    }

    #[test]
    fn lengths() {
        assert_eq!(parse("10px", parse_length), Some(Length::px(10.0)));
        assert_eq!(parse("1.5em", parse_length), Some(Length::em(1.5)));
        assert_eq!(parse("50%", parse_length), Some(Length::percent(50.0)));
        assert_eq!(parse("0", parse_length), Some(Length::ZERO));
        assert_eq!(parse("12pt", parse_length), Some(Length::px(16.0)));
        assert_eq!(parse("10", parse_length), None);
        assert_eq!(
            parse("auto", parse_length_or_auto),
            Some(LengthOrAuto::Auto)
        );
    }

    #[test]
    fn calc_linear_combination() {
        let l = parse("calc(100% - 2 * 1em + 4px / 2)", parse_length).unwrap();
        assert_eq!(
            l,
            Length {
                pct: 100.0,
                em: -2.0,
                px: 2.0,
                ..Length::ZERO
            }
        );
        let nested = parse("calc((10px + 2px) * 2)", parse_length).unwrap();
        assert_eq!(nested, Length::px(24.0));
        assert_eq!(
            parse("clamp(1px, 2px, 3px)", parse_length),
            Some(Length::px(2.0))
        );
    }
}
