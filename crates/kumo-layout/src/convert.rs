//! 計算値（kumo-css）を Taffy のスタイルへ変換する。

use std::sync::Arc;

use kumo_css::keywords::{self as kw, Align, Display as CssDisplay};
use kumo_css::{ComputedStyle, LengthPct, LengthPctAuto, Side};
use taffy::Point;
use taffy::prelude::*;
use taffy::style::{
    AlignContent, AlignItems, BoxSizing, Clear, Dimension, Float, GridTemplateComponent,
    LengthPercentage, LengthPercentageAuto, Overflow, Position,
};

use crate::grid::{parse_areas, parse_placement, parse_tracks};

/// `calc(50% + 10px)` のような px と % の混合値を保持する表。
/// Taffy には添字をタグ付きポインタ値として渡し、`resolve` で解決する（参照外しはしない）。
#[derive(Debug, Default)]
pub struct CalcTable {
    values: Vec<LengthPct>,
}

impl CalcTable {
    fn push(&mut self, value: LengthPct) -> *const () {
        self.values.push(value);
        // 下位3bitはタグ用に空けておく必要がある
        std::ptr::without_provenance(self.values.len() << 3)
    }

    pub fn resolve(&self, ptr: *const (), basis: f32) -> f32 {
        let index = (ptr as usize >> 3).wrapping_sub(1);
        self.values.get(index).map_or(0.0, |v| v.resolve(basis))
    }
}

fn length_pct(v: LengthPct, calc: &mut CalcTable) -> LengthPercentage {
    if v.pct == 0.0 {
        LengthPercentage::length(v.px)
    } else if v.px == 0.0 {
        LengthPercentage::percent(v.pct / 100.0)
    } else {
        LengthPercentage::calc(calc.push(v))
    }
}

fn length_pct_auto(v: LengthPctAuto, calc: &mut CalcTable) -> LengthPercentageAuto {
    match v {
        LengthPctAuto::Auto => LengthPercentageAuto::auto(),
        LengthPctAuto::LengthPct(v) if v.pct == 0.0 => LengthPercentageAuto::length(v.px),
        LengthPctAuto::LengthPct(v) if v.px == 0.0 => LengthPercentageAuto::percent(v.pct / 100.0),
        LengthPctAuto::LengthPct(v) => LengthPercentageAuto::calc(calc.push(v)),
    }
}

fn dimension(v: LengthPctAuto, calc: &mut CalcTable) -> Dimension {
    match v {
        LengthPctAuto::Auto => Dimension::auto(),
        LengthPctAuto::LengthPct(v) if v.pct == 0.0 => Dimension::length(v.px),
        LengthPctAuto::LengthPct(v) if v.px == 0.0 => Dimension::percent(v.pct / 100.0),
        LengthPctAuto::LengthPct(v) => Dimension::calc(calc.push(v)),
    }
}

fn rect<T, U>(sides: &kumo_css::Sides<T>, mut f: impl FnMut(&T) -> U) -> Rect<U> {
    Rect {
        left: f(&sides.left),
        right: f(&sides.right),
        top: f(&sides.top),
        bottom: f(&sides.bottom),
    }
}

/// レイアウトアルゴリズム上の表示種別。
pub fn taffy_display(display: CssDisplay) -> Display {
    match display {
        CssDisplay::None => Display::None,
        CssDisplay::Flex | CssDisplay::InlineFlex => Display::Flex,
        CssDisplay::Grid | CssDisplay::InlineGrid | CssDisplay::Table | CssDisplay::InlineTable => {
            Display::Grid
        }
        CssDisplay::FlowRoot | CssDisplay::InlineBlock | CssDisplay::TableCell => Display::FlowRoot,
        _ => Display::Block,
    }
}

/// 要素ボックスの Taffy スタイルを作る。
pub fn to_taffy(cs: &ComputedStyle, calc: &mut CalcTable) -> Style<Arc<str>> {
    let border = Rect {
        left: LengthPercentage::length(cs.used_border_width(Side::Left)),
        right: LengthPercentage::length(cs.used_border_width(Side::Right)),
        top: LengthPercentage::length(cs.used_border_width(Side::Top)),
        bottom: LengthPercentage::length(cs.used_border_width(Side::Bottom)),
    };
    let mut style = Style {
        display: taffy_display(cs.display),
        box_sizing: match cs.box_sizing {
            kw::BoxSizing::BorderBox => BoxSizing::BorderBox,
            kw::BoxSizing::ContentBox => BoxSizing::ContentBox,
        },
        position: match cs.position {
            kw::Position::Static => Position::Static,
            kw::Position::Absolute | kw::Position::Fixed => Position::Absolute,
            kw::Position::Relative | kw::Position::Sticky => Position::Relative,
        },
        float: match cs.float {
            kw::Float::None => Float::None,
            kw::Float::Left => Float::Left,
            kw::Float::Right => Float::Right,
        },
        clear: match cs.clear {
            kw::Clear::None => Clear::None,
            kw::Clear::Left => Clear::Left,
            kw::Clear::Right => Clear::Right,
            kw::Clear::Both => Clear::Both,
        },
        overflow: Point {
            x: overflow(cs.overflow_x),
            y: overflow(cs.overflow_y),
        },
        inset: rect(&cs.inset, |v| length_pct_auto(*v, calc)),
        margin: rect(&cs.margin, |v| length_pct_auto(*v, calc)),
        padding: rect(&cs.padding, |v| length_pct(*v, calc)),
        border,
        size: Size {
            width: dimension(cs.width, calc),
            height: dimension(cs.height, calc),
        },
        min_size: Size {
            width: length_pct_auto(cs.min_width, calc),
            height: length_pct_auto(cs.min_height, calc),
        },
        max_size: Size {
            width: length_pct_auto(cs.max_width, calc),
            height: length_pct_auto(cs.max_height, calc),
        },
        flex_direction: match cs.flex_direction {
            kw::FlexDirection::Row => FlexDirection::Row,
            kw::FlexDirection::RowReverse => FlexDirection::RowReverse,
            kw::FlexDirection::Column => FlexDirection::Column,
            kw::FlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
        },
        flex_wrap: match cs.flex_wrap {
            kw::FlexWrap::NoWrap => FlexWrap::NoWrap,
            kw::FlexWrap::Wrap => FlexWrap::Wrap,
            kw::FlexWrap::WrapReverse => FlexWrap::WrapReverse,
        },
        flex_grow: cs.flex_grow,
        flex_shrink: cs.flex_shrink,
        flex_basis: dimension(cs.flex_basis, calc),
        justify_content: align_content(cs.justify_content).unwrap_or(AlignContent::NORMAL),
        align_content: align_content(cs.align_content).unwrap_or(AlignContent::NORMAL),
        align_items: align_items(cs.align_items).unwrap_or(AlignItems::NORMAL),
        align_self: match cs.align_self {
            Align::Auto => None,
            other => align_items(other),
        },
        gap: Size {
            width: length_pct(cs.column_gap, calc),
            height: length_pct(cs.row_gap, calc),
        },
        ..Style::default()
    };
    apply_grid(&mut style, cs);
    style
}

fn overflow(o: kw::Overflow) -> Overflow {
    match o {
        kw::Overflow::Visible => Overflow::Visible,
        kw::Overflow::Clip => Overflow::Clip,
        kw::Overflow::Hidden => Overflow::Hidden,
        kw::Overflow::Scroll | kw::Overflow::Auto => Overflow::Scroll,
    }
}

fn align_items(a: Align) -> Option<AlignItems> {
    Some(match a {
        Align::Normal | Align::Auto => return None,
        Align::Start | Align::Left => AlignItems::START,
        Align::End | Align::Right => AlignItems::END,
        Align::FlexStart => AlignItems::FLEX_START,
        Align::FlexEnd => AlignItems::FLEX_END,
        Align::Center => AlignItems::CENTER,
        Align::Baseline => AlignItems::BASELINE,
        Align::Stretch | Align::SpaceBetween | Align::SpaceAround | Align::SpaceEvenly => {
            AlignItems::STRETCH
        }
    })
}

fn align_content(a: Align) -> Option<AlignContent> {
    Some(match a {
        Align::Normal | Align::Auto | Align::Baseline => return None,
        Align::Start | Align::Left => AlignContent::START,
        Align::End | Align::Right => AlignContent::END,
        Align::FlexStart => AlignContent::FLEX_START,
        Align::FlexEnd => AlignContent::FLEX_END,
        Align::Center => AlignContent::CENTER,
        Align::Stretch => AlignContent::STRETCH,
        Align::SpaceBetween => AlignContent::SPACE_BETWEEN,
        Align::SpaceAround => AlignContent::SPACE_AROUND,
        Align::SpaceEvenly => AlignContent::SPACE_EVENLY,
    })
}

fn apply_grid(style: &mut Style<Arc<str>>, cs: &ComputedStyle) {
    let grid = &cs.grid;
    if let Some((tracks, names)) = grid.template_columns.as_deref().and_then(parse_tracks) {
        style.grid_template_columns = tracks;
        style.grid_template_column_names = names;
    }
    if let Some((tracks, names)) = grid.template_rows.as_deref().and_then(parse_tracks) {
        style.grid_template_rows = tracks;
        style.grid_template_row_names = names;
    }
    if let Some(areas) = grid.template_areas.as_deref().and_then(parse_areas) {
        style.grid_template_areas = Some(areas);
    }
    if let Some(flow) = grid.auto_flow.as_deref().and_then(|s| s.parse().ok()) {
        style.grid_auto_flow = flow;
    }
    let placement = |start: &Option<Arc<str>>, end: &Option<Arc<str>>| Line {
        start: start
            .as_deref()
            .map_or(GridPlacement::Auto, |s| parse_placement(s, true)),
        end: end
            .as_deref()
            .map_or(GridPlacement::Auto, |s| parse_placement(s, false)),
    };
    style.grid_column = placement(&grid.column_start, &grid.column_end);
    style.grid_row = placement(&grid.row_start, &grid.row_end);
}

/// 表を grid で近似するための列テンプレート（`repeat(n, auto)`）。
pub fn auto_columns(count: usize) -> Vec<GridTemplateComponent<Arc<str>>> {
    (0..count.max(1))
        .map(|_| GridTemplateComponent::AUTO)
        .collect()
}
