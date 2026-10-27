//! メディアクエリ（幅・高さ・メディア種別・prefers-* の最小評価）。

use cssparser::{Parser, Token};

/// 評価に使う環境。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Device {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
enum Feature {
    MinWidth(f32),
    MaxWidth(f32),
    MinHeight(f32),
    MaxHeight(f32),
    Always(bool),
}

/// 1つのクエリ（`not`? 種別 and (特徴) and …）。
#[derive(Clone, Debug, PartialEq)]
struct MediaQuery {
    negated: bool,
    features: Vec<Feature>,
}

/// カンマ区切りのクエリ列（いずれかが真なら真）。入れ子 @media は `all_of` で AND する。
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MediaQueryList {
    queries: Vec<MediaQuery>,
    all_of: Vec<MediaQueryList>,
}

impl MediaQueryList {
    pub fn parse(input: &mut Parser<'_>) -> Self {
        let queries = input.parse_comma_separated_ignoring_errors(|i| {
            Ok::<_, cssparser::ParseError<()>>(parse_query(i))
        });
        Self {
            queries,
            all_of: Vec::new(),
        }
    }

    pub fn parse_str(css: &str) -> Self {
        Self::parse(&mut Parser::new(css))
    }

    /// 入れ子の @media 用に AND 結合する。
    pub fn and(&self, inner: &Self) -> Self {
        Self {
            queries: Vec::new(),
            all_of: vec![self.clone(), inner.clone()],
        }
    }

    pub fn matches(&self, device: &Device) -> bool {
        if !self.all_of.is_empty() {
            return self.all_of.iter().all(|m| m.matches(device));
        }
        // 空のリストは常に真（@media {} や解析不能だった場合の扱い）
        self.queries.is_empty() || self.queries.iter().any(|q| q.matches(device))
    }
}

impl MediaQuery {
    fn matches(&self, device: &Device) -> bool {
        let result = self.features.iter().all(|f| match *f {
            Feature::MinWidth(v) => device.width >= v,
            Feature::MaxWidth(v) => device.width <= v,
            Feature::MinHeight(v) => device.height >= v,
            Feature::MaxHeight(v) => device.height <= v,
            Feature::Always(b) => b,
        });
        result != self.negated
    }
}

fn parse_query(input: &mut Parser<'_>) -> MediaQuery {
    let mut query = MediaQuery {
        negated: false,
        features: Vec::new(),
    };
    while let Ok(token) = input.next().cloned() {
        match token {
            Token::Ident(ident) => {
                let lower = ident.to_ascii_lowercase();
                match lower.as_str() {
                    "not" => query.negated = true,
                    "screen" | "all" | "only" | "and" => {}
                    // print / speech などは画面表示では偽
                    _ => query.features.push(Feature::Always(false)),
                }
            }
            Token::ParenthesisBlock => {
                let feature = input
                    .parse_nested_block(|i| Ok::<_, cssparser::ParseError<()>>(parse_feature(i)))
                    .unwrap_or(Feature::Always(false));
                query.features.push(feature);
            }
            _ => query.features.push(Feature::Always(false)),
        }
    }
    query
}

/// `(min-width: 720px)` / `(width >= 720px)` / `(prefers-color-scheme: dark)` 等。
fn parse_feature(input: &mut Parser<'_>) -> Feature {
    let Ok(name) = input.expect_ident_cloned() else {
        return Feature::Always(false);
    };
    let name = name.to_ascii_lowercase();
    let token = input.next().cloned();
    let (op, value) = match token {
        Ok(Token::Colon) => (None, input.next().cloned().ok()),
        Ok(Token::Delim(c @ ('<' | '>'))) => {
            let eq = input.try_parse(|i| i.expect_delim('=')).is_ok();
            (Some((c, eq)), input.next().cloned().ok())
        }
        Ok(Token::Delim('=')) => (Some(('=', true)), input.next().cloned().ok()),
        // `(color)` のような真偽評価
        _ => return Feature::Always(matches!(name.as_str(), "color" | "hover" | "pointer")),
    };
    let px = value.as_ref().and_then(token_px);
    match (name.as_str(), op, px) {
        ("min-width", None, Some(v)) | ("width", Some(('>', _)), Some(v)) => Feature::MinWidth(v),
        ("max-width", None, Some(v)) | ("width", Some(('<', _)), Some(v)) => Feature::MaxWidth(v),
        ("min-height", None, Some(v)) | ("height", Some(('>', _)), Some(v)) => {
            Feature::MinHeight(v)
        }
        ("max-height", None, Some(v)) | ("height", Some(('<', _)), Some(v)) => {
            Feature::MaxHeight(v)
        }
        _ => Feature::Always(keyword_feature(&name, value.as_ref())),
    }
}

/// 既定環境：ライトテーマ、マウス操作、アニメーション許可。
fn keyword_feature(name: &str, value: Option<&Token<'_>>) -> bool {
    let Some(Token::Ident(v)) = value else {
        return false;
    };
    let v = v.to_ascii_lowercase();
    match name {
        "prefers-color-scheme" => v == "light",
        "prefers-reduced-motion" | "prefers-contrast" | "forced-colors" => {
            v == "no-preference" || v == "none"
        }
        "hover" | "any-hover" => v == "hover",
        "pointer" | "any-pointer" => v == "fine",
        "orientation" => v == "landscape",
        _ => false,
    }
}

fn token_px(token: &Token<'_>) -> Option<f32> {
    match *token {
        Token::Dimension {
            value, ref unit, ..
        } => match unit.to_ascii_lowercase().as_str() {
            "px" => Some(value),
            "em" | "rem" => Some(value * 16.0),
            _ => None,
        },
        Token::Number { value: 0.0, .. } => Some(0.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESKTOP: Device = Device {
        width: 1280.0,
        height: 800.0,
    };

    fn matches(q: &str) -> bool {
        MediaQueryList::parse_str(q).matches(&DESKTOP)
    }

    #[test]
    fn width_queries() {
        assert!(matches("screen and (min-width: 720px)"));
        assert!(!matches("(max-width: 719px)"));
        assert!(matches("(min-width: 50em)"));
        assert!(matches("(width >= 1000px)"));
        assert!(!matches("(width < 1000px)"));
    }

    #[test]
    fn media_types_and_lists() {
        assert!(!matches("print"));
        assert!(matches("print, screen"));
        assert!(matches("not print"));
        assert!(matches("all"));
    }

    #[test]
    fn preference_queries() {
        assert!(!matches("(prefers-color-scheme: dark)"));
        assert!(matches("(prefers-color-scheme: light)"));
        assert!(matches("(prefers-reduced-motion: no-preference)"));
    }
}
