//! 色の値とパーサ（16進・名前付き・rgb()/hsl()）。

use cssparser::{Parser, Token};

use crate::values::{ColorValue, ParseResult, Rgba, invalid};

pub fn parse_color<'i>(input: &mut Parser<'i>) -> ParseResult<ColorValue> {
    let token = input.next()?.clone();
    let rgba = match token {
        Token::Hash(ref v) | Token::IDHash(ref v) => {
            cssparser::color::parse_hash_color(v.as_bytes())
                .map(|(r, g, b, a)| Rgba::new(r, g, b, unit_to_u8(a)))
                .ok()
        }
        Token::Ident(ref name) => {
            if name.eq_ignore_ascii_case("currentcolor") {
                return Ok(ColorValue::CurrentColor);
            }
            if name.eq_ignore_ascii_case("transparent") {
                Some(Rgba::TRANSPARENT)
            } else {
                cssparser::color::parse_named_color(&name.to_ascii_lowercase())
                    .map(|(r, g, b)| Rgba::new(r, g, b, 255))
                    .ok()
            }
        }
        Token::Function(ref name) => {
            let name = name.to_ascii_lowercase();
            if name == "light-dark" {
                // 明るい配色だけを描くので、第 1 引数（light）を採る
                return input.parse_nested_block(|i| {
                    let light = parse_color(i)?;
                    while i.next().is_ok() {}
                    Ok(light)
                });
            }
            return input
                .parse_nested_block(|i| parse_color_function(&name, i))
                .map(ColorValue::Rgba);
        }
        _ => None,
    };
    rgba.map(ColorValue::Rgba).map_or_else(invalid, Ok)
}

fn unit_to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn parse_color_function<'i>(name: &str, input: &mut Parser<'i>) -> ParseResult<Rgba> {
    let mut parts = [0.0f32; 4];
    parts[3] = 1.0;
    let mut count = 0;
    while !input.is_exhausted() {
        let token = input.next()?.clone();
        let value = match token {
            Token::Comma | Token::Delim('/') => continue,
            Token::Number { value, .. } => value,
            Token::Percentage { unit_value, .. } => match (name, count) {
                (_, 3) => unit_value,
                ("rgb" | "rgba", _) => unit_value * 255.0,
                _ => unit_value * 100.0,
            },
            Token::Dimension {
                value, ref unit, ..
            } if unit.eq_ignore_ascii_case("deg") => value,
            Token::Ident(ref none) if none.eq_ignore_ascii_case("none") => 0.0,
            _ => return invalid(),
        };
        if count >= 4 {
            return invalid();
        }
        parts[count] = value;
        count += 1;
    }
    let [a, b, c, alpha] = parts;
    let alpha = unit_to_u8(alpha);
    match name {
        "rgb" | "rgba" => Ok(Rgba::new(to_u8(a), to_u8(b), to_u8(c), alpha)),
        "hsl" | "hsla" => {
            let (r, g, b) = hsl_to_rgb(a, b / 100.0, c / 100.0);
            Ok(Rgba::new(r, g, b, alpha))
        }
        _ => invalid(),
    }
}

fn to_u8(v: f32) -> u8 {
    v.clamp(0.0, 255.0).round() as u8
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0) / 360.0;
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    let t2 = if l <= 0.5 {
        l * (s + 1.0)
    } else {
        l + s - l * s
    };
    let t1 = l * 2.0 - t2;
    let hue = |mut h: f32| {
        if h < 0.0 {
            h += 1.0;
        }
        if h > 1.0 {
            h -= 1.0;
        }
        if h * 6.0 < 1.0 {
            t1 + (t2 - t1) * h * 6.0
        } else if h * 2.0 < 1.0 {
            t2
        } else if h * 3.0 < 2.0 {
            t1 + (t2 - t1) * (2.0 / 3.0 - h) * 6.0
        } else {
            t1
        }
    };
    let c = |v: f32| (v * 255.0).round() as u8;
    (c(hue(h + 1.0 / 3.0)), c(hue(h)), c(hue(h - 1.0 / 3.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse<T>(s: &str, f: impl for<'i> FnOnce(&mut Parser<'i>) -> ParseResult<T>) -> Option<T> {
        let mut parser = Parser::new(s);
        parser.parse_entirely(f).ok()
    }

    #[test]
    fn colors() {
        let rgba = |r, g, b, a| Some(ColorValue::Rgba(Rgba::new(r, g, b, a)));
        assert_eq!(parse("#f00", parse_color), rgba(255, 0, 0, 255));
        assert_eq!(parse("#00ff0080", parse_color), rgba(0, 255, 0, 128));
        assert_eq!(parse("navy", parse_color), rgba(0, 0, 128, 255));
        assert_eq!(parse("rgb(1, 2, 3)", parse_color), rgba(1, 2, 3, 255));
        assert_eq!(parse("rgba(1 2 3 / 50%)", parse_color), rgba(1, 2, 3, 128));
        assert_eq!(
            parse("hsl(120, 100%, 50%)", parse_color),
            rgba(0, 255, 0, 255)
        );
        assert_eq!(
            parse("currentColor", parse_color),
            Some(ColorValue::CurrentColor)
        );
        assert_eq!(parse("nope", parse_color), None);
        assert_eq!(
            parse("light-dark(#f00, rgb(0 0 255))", parse_color),
            rgba(255, 0, 0, 255)
        );
    }
}
