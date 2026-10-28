//! 計算値（computed style）。同一の計算値は `Arc` で共有される。

use std::collections::HashMap;
use std::sync::Arc;

use kumo_dom::LocalName;

use crate::keywords::*;
use crate::values::Rgba;

/// 解決済みの長さ（px 部分 + パーセント部分）。`calc(50% + 10px)` も表現できる。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LengthPct {
    pub px: f32,
    pub pct: f32,
}

impl LengthPct {
    pub const ZERO: Self = Self { px: 0.0, pct: 0.0 };

    pub fn px(px: f32) -> Self {
        Self { px, pct: 0.0 }
    }

    /// 基準長 `basis` に対して解決する。
    pub fn resolve(self, basis: f32) -> f32 {
        self.px + basis * self.pct / 100.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum LengthPctAuto {
    LengthPct(LengthPct),
    #[default]
    Auto,
}

impl LengthPctAuto {
    pub const ZERO: Self = Self::LengthPct(LengthPct::ZERO);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    Normal,
    Number(f32),
    Px(f32),
}

impl LineHeight {
    /// 実際の行の高さ（px）。`normal` はフォントサイズの1.2倍で近似。
    pub fn resolve(self, font_size: f32) -> f32 {
        match self {
            Self::Normal => font_size * 1.2,
            Self::Number(n) => font_size * n,
            Self::Px(px) => px,
        }
    }
}

/// 上右下左の4辺。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sides<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Sides<T> {
    pub fn all(v: T) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Top, Side::Right, Side::Bottom, Side::Left];
}

impl<T> Sides<T> {
    pub fn get_mut(&mut self, side: Side) -> &mut T {
        match side {
            Side::Top => &mut self.top,
            Side::Right => &mut self.right,
            Side::Bottom => &mut self.bottom,
            Side::Left => &mut self.left,
        }
    }

    pub fn get(&self, side: Side) -> &T {
        match side {
            Side::Top => &self.top,
            Side::Right => &self.right,
            Side::Bottom => &self.bottom,
            Side::Left => &self.left,
        }
    }
}

/// 下線などの装飾線（ビットフラグ）。
pub mod text_decoration {
    pub const UNDERLINE: u8 = 1;
    pub const OVERLINE: u8 = 2;
    pub const LINE_THROUGH: u8 = 4;
}

/// グリッド関連は Taffy のパーサへそのまま渡す生文字列で保持する。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridStyle {
    pub template_columns: Option<Arc<str>>,
    pub template_rows: Option<Arc<str>>,
    pub template_areas: Option<Arc<str>>,
    pub auto_columns: Option<Arc<str>>,
    pub auto_rows: Option<Arc<str>>,
    pub auto_flow: Option<Arc<str>>,
    pub column_start: Option<Arc<str>>,
    pub column_end: Option<Arc<str>>,
    pub row_start: Option<Arc<str>>,
    pub row_end: Option<Arc<str>>,
}

pub type CustomProperties = Arc<HashMap<LocalName, Arc<str>>>;

/// 1要素の計算値。継承プロパティと非継承プロパティを同じ構造体に持つ。
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedStyle {
    // ---- ボックス ----
    pub display: Display,
    pub position: Position,
    pub float: Float,
    pub clear: Clear,
    pub inset: Sides<LengthPctAuto>,
    pub margin: Sides<LengthPctAuto>,
    pub padding: Sides<LengthPct>,
    pub border_width: Sides<f32>,
    pub border_style: Sides<BorderStyle>,
    pub border_color: Sides<Rgba>,
    /// 角の丸み（左上・右上・右下・左下）。% は箱の幅が基準（縦横別の楕円は扱わない）
    pub border_radius: [LengthPct; 4],
    pub width: LengthPctAuto,
    pub height: LengthPctAuto,
    pub min_width: LengthPctAuto,
    pub min_height: LengthPctAuto,
    /// `none` は Auto で表す
    pub max_width: LengthPctAuto,
    pub max_height: LengthPctAuto,
    pub box_sizing: BoxSizing,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub opacity: f32,
    pub background_color: Rgba,
    // ---- flex / grid ----
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: LengthPctAuto,
    pub justify_content: Align,
    pub align_items: Align,
    pub align_self: Align,
    pub align_content: Align,
    pub row_gap: LengthPct,
    pub column_gap: LengthPct,
    pub order: i32,
    /// `None` は `auto`
    pub z_index: Option<i32>,
    /// `content`（疑似要素 ::before / ::after のときだけ意味を持つ。継承しない）
    pub content: Option<Arc<[crate::decl::ContentPart]>>,
    pub grid: Box<GridStyle>,
    // ---- 表 ----
    pub border_collapse: BorderCollapse,
    pub border_spacing: f32,
    // ---- テキスト（継承） ----
    pub color: Rgba,
    pub font_family: Arc<[Box<str>]>,
    pub font_size: f32,
    pub font_weight: u16,
    pub font_style: FontStyle,
    pub line_height: LineHeight,
    pub text_align: TextAlign,
    pub text_indent: LengthPct,
    pub text_transform: TextTransform,
    pub white_space: WhiteSpace,
    pub list_style_type: ListStyleType,
    pub list_style_position: ListStylePosition,
    pub visibility: Visibility,
    // ---- テキスト（非継承） ----
    pub text_decoration_line: u8,
    pub vertical_align: VerticalAlign,
    // ---- カスタムプロパティ（継承） ----
    pub custom: CustomProperties,
}

/// ブラウザ既定のフォントサイズ（px）。
pub const DEFAULT_FONT_SIZE: f32 = 16.0;

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
}

impl ComputedStyle {
    /// すべてのプロパティが初期値のスタイル。
    pub fn initial() -> Self {
        Self {
            display: Display::Inline,
            position: Position::Static,
            float: Float::None,
            clear: Clear::None,
            inset: Sides::all(LengthPctAuto::Auto),
            margin: Sides::all(LengthPctAuto::ZERO),
            padding: Sides::all(LengthPct::ZERO),
            border_width: Sides::all(3.0), // medium
            border_style: Sides::all(BorderStyle::None),
            border_color: Sides::all(Rgba::BLACK),
            border_radius: [LengthPct::ZERO; 4],
            width: LengthPctAuto::Auto,
            height: LengthPctAuto::Auto,
            min_width: LengthPctAuto::Auto,
            min_height: LengthPctAuto::Auto,
            max_width: LengthPctAuto::Auto,
            max_height: LengthPctAuto::Auto,
            box_sizing: BoxSizing::ContentBox,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            opacity: 1.0,
            background_color: Rgba::TRANSPARENT,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: LengthPctAuto::Auto,
            justify_content: Align::Normal,
            align_items: Align::Normal,
            align_self: Align::Auto,
            align_content: Align::Normal,
            row_gap: LengthPct::ZERO,
            column_gap: LengthPct::ZERO,
            order: 0,
            z_index: None,
            content: None,
            grid: Box::default(),
            border_collapse: BorderCollapse::Separate,
            border_spacing: 0.0,
            color: Rgba::BLACK,
            font_family: Arc::from([Box::from("serif")]),
            font_size: DEFAULT_FONT_SIZE,
            font_weight: 400,
            font_style: FontStyle::Normal,
            line_height: LineHeight::Normal,
            text_align: TextAlign::Start,
            text_indent: LengthPct::ZERO,
            text_transform: TextTransform::None,
            white_space: WhiteSpace::Normal,
            list_style_type: ListStyleType::Disc,
            list_style_position: ListStylePosition::Outside,
            visibility: Visibility::Visible,
            text_decoration_line: 0,
            vertical_align: VerticalAlign::Baseline,
            custom: Arc::default(),
        }
    }

    /// 親から継承プロパティだけを引き継いだ新しいスタイル。
    pub fn inherit_from(parent: &Self) -> Self {
        Self {
            border_collapse: parent.border_collapse,
            border_spacing: parent.border_spacing,
            color: parent.color,
            font_family: parent.font_family.clone(),
            font_size: parent.font_size,
            font_weight: parent.font_weight,
            font_style: parent.font_style,
            line_height: parent.line_height,
            text_align: parent.text_align,
            text_indent: parent.text_indent,
            text_transform: parent.text_transform,
            white_space: parent.white_space,
            list_style_type: parent.list_style_type,
            list_style_position: parent.list_style_position,
            visibility: parent.visibility,
            custom: parent.custom.clone(),
            ..Self::initial()
        }
    }

    /// 実際に描画される境界線の幅（style が none/hidden なら 0）。
    pub fn used_border_width(&self, side: Side) -> f32 {
        match self.border_style.get(side) {
            BorderStyle::None | BorderStyle::Hidden => 0.0,
            _ => *self.border_width.get(side),
        }
    }

    /// 位置指定されているか（`position` が static 以外）。
    pub fn is_positioned(&self) -> bool {
        self.position != Position::Static
    }

    /// 自身がスタッキングコンテキストを作るか（CSS 2.1 §9.9.1 と CSS Color の opacity）。
    /// ルート要素は呼び出し側で別扱いにする。
    pub fn creates_stacking_context(&self) -> bool {
        (self.is_positioned() && self.z_index.is_some())
            || matches!(self.position, Position::Fixed | Position::Sticky)
            || self.opacity < 1.0
    }

    pub fn line_height_px(&self) -> f32 {
        self.line_height.resolve(self.font_size)
    }
}
