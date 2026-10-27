//! 指定値（Decl）を計算値（ComputedStyle）へ適用する。

use std::sync::Arc;

use crate::decl::*;
use crate::style::{ComputedStyle, LengthPct, LengthPctAuto, LineHeight, Side};
use crate::values::{ColorValue, Length, LengthOrAuto, Rgba};

/// 長さ解決に必要な環境。
#[derive(Clone, Copy, Debug)]
pub struct ResolveContext<'a> {
    pub parent: &'a ComputedStyle,
    pub root_font_size: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl ResolveContext<'_> {
    fn length(&self, l: Length, font_size: f32) -> LengthPct {
        LengthPct {
            px: l.px
                + l.em * font_size
                + l.rem * self.root_font_size
                + l.vw * self.viewport_width / 100.0
                + l.vh * self.viewport_height / 100.0,
            pct: l.pct,
        }
    }
}

/// 宣言を1つ適用する。`style.font_size` は確定済みである前提（em の基準）。
pub fn apply_decl(style: &mut ComputedStyle, decl: &Decl, ctx: &ResolveContext<'_>) {
    let fs = style.font_size;
    let len = |l: Length| ctx.length(l, fs);
    let lpa = |l: LengthOrAuto| match l {
        LengthOrAuto::Length(l) => LengthPctAuto::LengthPct(ctx.length(l, fs)),
        LengthOrAuto::Auto => LengthPctAuto::Auto,
    };
    match decl {
        Decl::Display(v) => style.display = *v,
        Decl::Position(v) => style.position = *v,
        Decl::Float(v) => style.float = *v,
        Decl::Clear(v) => style.clear = *v,
        Decl::Inset(side, v) => *style.inset.get_mut(*side) = lpa(*v),
        Decl::Margin(side, v) => *style.margin.get_mut(*side) = lpa(*v),
        Decl::Padding(side, v) => *style.padding.get_mut(*side) = len(*v),
        Decl::BorderWidth(side, v) => *style.border_width.get_mut(*side) = len(*v).px.max(0.0),
        Decl::BorderStyle(side, v) => *style.border_style.get_mut(*side) = *v,
        // currentcolor は color 確定後に解決するため、ここでは目印として保持しない
        Decl::BorderColor(side, v) => *style.border_color.get_mut(*side) = color(*v, style.color),
        Decl::Size(prop, v) => {
            let v = lpa(*v);
            match prop {
                SizeProp::Width => style.width = v,
                SizeProp::Height => style.height = v,
                SizeProp::MinWidth => style.min_width = v,
                SizeProp::MinHeight => style.min_height = v,
                SizeProp::MaxWidth => style.max_width = v,
                SizeProp::MaxHeight => style.max_height = v,
            }
        }
        Decl::BoxSizing(v) => style.box_sizing = *v,
        Decl::OverflowX(v) => style.overflow_x = *v,
        Decl::OverflowY(v) => style.overflow_y = *v,
        Decl::Opacity(v) => style.opacity = *v,
        Decl::BackgroundColor(v) => style.background_color = color(*v, style.color),
        Decl::FlexDirection(v) => style.flex_direction = *v,
        Decl::FlexWrap(v) => style.flex_wrap = *v,
        Decl::FlexGrow(v) => style.flex_grow = *v,
        Decl::FlexShrink(v) => style.flex_shrink = *v,
        Decl::FlexBasis(v) => style.flex_basis = lpa(*v),
        Decl::JustifyContent(v) => style.justify_content = *v,
        Decl::AlignItems(v) => style.align_items = *v,
        Decl::AlignSelf(v) => style.align_self = *v,
        Decl::AlignContent(v) => style.align_content = *v,
        Decl::RowGap(v) => style.row_gap = len(*v),
        Decl::ColumnGap(v) => style.column_gap = len(*v),
        Decl::Order(v) => style.order = *v,
        Decl::ZIndex(v) => style.z_index = *v,
        Decl::Grid(prop, v) => apply_grid(style, *prop, v.clone()),
        Decl::BorderCollapse(v) => style.border_collapse = *v,
        Decl::BorderSpacing(v) => style.border_spacing = len(*v).px,
        Decl::Color(v) => style.color = color(*v, ctx.parent.color),
        Decl::FontFamily(v) => style.font_family = v.clone(),
        Decl::FontSize(v) => style.font_size = font_size(*v, ctx),
        Decl::FontWeight(v) => style.font_weight = font_weight(*v, ctx.parent.font_weight),
        Decl::FontStyle(v) => style.font_style = *v,
        Decl::LineHeight(v) => style.line_height = line_height(*v, ctx, fs),
        Decl::TextAlign(v) => style.text_align = *v,
        Decl::TextIndent(v) => style.text_indent = len(*v),
        Decl::TextTransform(v) => style.text_transform = *v,
        Decl::WhiteSpace(v) => style.white_space = *v,
        Decl::ListStyleType(v) => style.list_style_type = *v,
        Decl::ListStylePosition(v) => style.list_style_position = *v,
        Decl::Visibility(v) => style.visibility = *v,
        Decl::TextDecorationLine(v) => style.text_decoration_line = *v,
        Decl::VerticalAlign(v) => style.vertical_align = *v,
        Decl::Wide(name, kw) => apply_wide(style, name, *kw, ctx.parent),
        // Custom / Unparsed はカスケード側で処理済み
        Decl::Custom(..) | Decl::Unparsed(..) => {}
    }
}

fn color(value: ColorValue, current: Rgba) -> Rgba {
    match value {
        ColorValue::Rgba(c) => c,
        ColorValue::CurrentColor => current,
    }
}

fn font_size(value: FontSizeValue, ctx: &ResolveContext<'_>) -> f32 {
    let parent = ctx.parent.font_size;
    match value {
        // font-size の em / % は親のフォントサイズ基準
        FontSizeValue::Length(l) => ctx.length(l, parent).resolve(parent),
        FontSizeValue::Smaller => parent / 1.2,
        FontSizeValue::Larger => parent * 1.2,
    }
}

fn font_weight(value: FontWeightValue, parent: u16) -> u16 {
    match value {
        FontWeightValue::Absolute(w) => w,
        FontWeightValue::Bolder => match parent {
            0..350 => 400,
            350..550 => 700,
            _ => 900,
        },
        FontWeightValue::Lighter => match parent {
            0..550 => 100,
            550..750 => 400,
            _ => 700,
        },
    }
}

fn line_height(value: LineHeightValue, ctx: &ResolveContext<'_>, fs: f32) -> LineHeight {
    match value {
        LineHeightValue::Normal => LineHeight::Normal,
        LineHeightValue::Number(n) => LineHeight::Number(n),
        LineHeightValue::Length(l) => LineHeight::Px(ctx.length(l, fs).px),
    }
}

fn apply_grid(style: &mut ComputedStyle, prop: GridProp, value: Option<Arc<str>>) {
    let grid = &mut style.grid;
    let slot = match prop {
        GridProp::TemplateColumns => &mut grid.template_columns,
        GridProp::TemplateRows => &mut grid.template_rows,
        GridProp::TemplateAreas => &mut grid.template_areas,
        GridProp::AutoColumns => &mut grid.auto_columns,
        GridProp::AutoRows => &mut grid.auto_rows,
        GridProp::AutoFlow => &mut grid.auto_flow,
        GridProp::ColumnStart => &mut grid.column_start,
        GridProp::ColumnEnd => &mut grid.column_end,
        GridProp::RowStart => &mut grid.row_start,
        GridProp::RowEnd => &mut grid.row_end,
    };
    *slot = value;
}

/// 継承プロパティかどうか（unset の解決に使う）。
fn is_inherited(name: &str) -> bool {
    name.starts_with("font")
        || name.starts_with("text-") && !name.starts_with("text-decoration")
        || name.starts_with("list-style")
        || matches!(
            name,
            "color"
                | "line-height"
                | "white-space"
                | "visibility"
                | "border-collapse"
                | "border-spacing"
        )
}

/// inherit / initial / unset を名前に応じたフィールドのコピーで実現する。
fn apply_wide(style: &mut ComputedStyle, name: &str, kw: WideKeyword, parent: &ComputedStyle) {
    let inherit = match kw {
        WideKeyword::Inherit => true,
        WideKeyword::Initial => false,
        WideKeyword::Unset => is_inherited(name),
    };
    let initial = ComputedStyle::initial();
    let src = if inherit { parent } else { &initial };
    copy_property(style, src, name);
}

macro_rules! copy_fields {
    ($dst:ident, $src:ident, $($field:ident),+) => {{
        $( $dst.$field = $src.$field.clone(); )+
    }};
}

fn copy_property(dst: &mut ComputedStyle, src: &ComputedStyle, name: &str) {
    if let Some(rest) = name.strip_prefix("margin") {
        return copy_sides(rest, |s| *dst.margin.get_mut(s) = *src.margin.get(s));
    }
    if let Some(rest) = name.strip_prefix("padding") {
        return copy_sides(rest, |s| *dst.padding.get_mut(s) = *src.padding.get(s));
    }
    if name.starts_with("border") && !name.contains("collapse") && !name.contains("spacing") {
        copy_fields!(dst, src, border_width, border_style, border_color);
        return;
    }
    match name {
        "all" => {
            let custom = dst.custom.clone();
            *dst = src.clone();
            dst.custom = custom;
        }
        "display" => copy_fields!(dst, src, display),
        "position" => copy_fields!(dst, src, position),
        "float" => copy_fields!(dst, src, float),
        "width" => copy_fields!(dst, src, width),
        "height" => copy_fields!(dst, src, height),
        "max-width" => copy_fields!(dst, src, max_width),
        "min-width" => copy_fields!(dst, src, min_width),
        "color" => copy_fields!(dst, src, color),
        "background" | "background-color" => copy_fields!(dst, src, background_color),
        "font" => copy_fields!(
            dst,
            src,
            font_family,
            font_size,
            font_weight,
            font_style,
            line_height
        ),
        "font-family" => copy_fields!(dst, src, font_family),
        "font-size" => copy_fields!(dst, src, font_size),
        "font-weight" => copy_fields!(dst, src, font_weight),
        "font-style" => copy_fields!(dst, src, font_style),
        "line-height" => copy_fields!(dst, src, line_height),
        "text-align" => copy_fields!(dst, src, text_align),
        "text-decoration" | "text-decoration-line" => copy_fields!(dst, src, text_decoration_line),
        "text-transform" => copy_fields!(dst, src, text_transform),
        "white-space" => copy_fields!(dst, src, white_space),
        "list-style" => copy_fields!(dst, src, list_style_type, list_style_position),
        "list-style-type" => copy_fields!(dst, src, list_style_type),
        "visibility" => copy_fields!(dst, src, visibility),
        "vertical-align" => copy_fields!(dst, src, vertical_align),
        "z-index" => copy_fields!(dst, src, z_index),
        "overflow" => copy_fields!(dst, src, overflow_x, overflow_y),
        _ => {}
    }
}

fn copy_sides(suffix: &str, mut copy: impl FnMut(Side)) {
    let sides: &[Side] = match suffix {
        "-top" | "-block-start" => &[Side::Top],
        "-right" | "-inline-end" => &[Side::Right],
        "-bottom" | "-block-end" => &[Side::Bottom],
        "-left" | "-inline-start" => &[Side::Left],
        _ => &Side::ALL,
    };
    sides.iter().for_each(|&s| copy(s));
}
