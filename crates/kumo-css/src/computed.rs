//! 計算値を `getComputedStyle` が返す文字列にする。よく使われるプロパティだけを扱い、ほかは `None`。

use crate::style::{ComputedStyle, LengthPct, LengthPctAuto, Side};
use crate::values::Rgba;

/// [`computed_value`] が答えるプロパティ。
pub const COMPUTED_PROPERTIES: &[&str] = &[
    "display",
    "position",
    "float",
    "clear",
    "visibility",
    "overflow-x",
    "overflow-y",
    "box-sizing",
    "text-align",
    "white-space",
    "font-style",
    "opacity",
    "z-index",
    "color",
    "background-color",
    "font-size",
    "font-weight",
    "line-height",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
];

/// `name`（kebab-case）の計算値の文字列。
pub fn computed_value(style: &ComputedStyle, name: &str) -> Option<String> {
    let side = |suffix: &str| match suffix {
        "top" => Some(Side::Top),
        "right" => Some(Side::Right),
        "bottom" => Some(Side::Bottom),
        "left" => Some(Side::Left),
        _ => None,
    };
    let text = match name {
        "display" => style.display.css_name().to_owned(),
        "position" => style.position.css_name().to_owned(),
        "float" | "css-float" => style.float.css_name().to_owned(),
        "clear" => style.clear.css_name().to_owned(),
        "visibility" => style.visibility.css_name().to_owned(),
        "overflow-x" => style.overflow_x.css_name().to_owned(),
        "overflow-y" => style.overflow_y.css_name().to_owned(),
        "box-sizing" => style.box_sizing.css_name().to_owned(),
        "text-align" => style.text_align.css_name().to_owned(),
        "white-space" => style.white_space.css_name().to_owned(),
        "font-style" => style.font_style.css_name().to_owned(),
        "opacity" => style.opacity.to_string(),
        "z-index" => style
            .z_index
            .map_or_else(|| "auto".to_owned(), |z| z.to_string()),
        "color" => rgb(style.color),
        "background-color" => rgb(style.background_color),
        "font-size" => format!("{}px", style.font_size),
        "font-weight" => style.font_weight.to_string(),
        "line-height" => match style.line_height {
            crate::style::LineHeight::Normal => "normal".to_owned(),
            other => format!("{}px", other.resolve(style.font_size)),
        },
        _ => {
            let (property, suffix) = name.split_once('-')?;
            if let Some(rest) = suffix
                .strip_suffix("-width")
                .filter(|_| property == "border")
            {
                return Some(format!("{}px", style.used_border_width(side(rest)?)));
            }
            let s = side(suffix)?;
            match property {
                "margin" => auto_length(*style.margin.get(s)),
                "padding" => length(*style.padding.get(s)),
                _ => return None,
            }
        }
    };
    Some(text)
}

fn rgb(c: Rgba) -> String {
    if c.a == 255 {
        format!("rgb({}, {}, {})", c.r, c.g, c.b)
    } else {
        let alpha = (f32::from(c.a) / 255.0 * 1000.0).round() / 1000.0;
        format!("rgba({}, {}, {}, {alpha})", c.r, c.g, c.b)
    }
}

fn length(v: LengthPct) -> String {
    match (v.px, v.pct) {
        (px, 0.0) => format!("{px}px"),
        (0.0, pct) => format!("{pct}%"),
        (px, pct) => format!("calc({pct}% + {px}px)"),
    }
}

fn auto_length(v: LengthPctAuto) -> String {
    match v {
        LengthPctAuto::Auto => "auto".to_owned(),
        LengthPctAuto::LengthPct(l) => length(l),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_common_properties() {
        let mut s = ComputedStyle::initial();
        s.display = crate::keywords::Display::InlineBlock;
        s.color = Rgba::new(10, 20, 30, 255);
        s.padding.left = LengthPct::px(4.0);
        s.font_size = 16.0;
        assert_eq!(computed_value(&s, "display").unwrap(), "inline-block");
        assert_eq!(computed_value(&s, "color").unwrap(), "rgb(10, 20, 30)");
        assert_eq!(computed_value(&s, "padding-left").unwrap(), "4px");
        assert_eq!(computed_value(&s, "font-size").unwrap(), "16px");
        assert_eq!(computed_value(&s, "z-index").unwrap(), "auto");
        assert_eq!(computed_value(&s, "margin-top").unwrap(), "0px");
        assert!(computed_value(&s, "no-such-property").is_none());
    }

    #[test]
    fn transparent_background_is_rgba_zero() {
        let s = ComputedStyle::initial();
        assert_eq!(
            computed_value(&s, "background-color").unwrap(),
            "rgba(0, 0, 0, 0)"
        );
    }
}
