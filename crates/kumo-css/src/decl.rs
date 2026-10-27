//! パース済み宣言（指定値）。長さの解決はカスケード時に行う。

use std::sync::Arc;

use kumo_dom::LocalName;

use crate::keywords::*;
use crate::style::Side;
use crate::values::{ColorValue, Length, LengthOrAuto};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeProp {
    Width,
    Height,
    MinWidth,
    MinHeight,
    MaxWidth,
    MaxHeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridProp {
    TemplateColumns,
    TemplateRows,
    TemplateAreas,
    AutoColumns,
    AutoRows,
    AutoFlow,
    ColumnStart,
    ColumnEnd,
    RowStart,
    RowEnd,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontSizeValue {
    Length(Length),
    Smaller,
    Larger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeightValue {
    Absolute(u16),
    Bolder,
    Lighter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeightValue {
    Normal,
    Number(f32),
    Length(Length),
}

/// `inherit` / `initial` / `unset`（`revert` は unset 扱い）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WideKeyword {
    Inherit,
    Initial,
    Unset,
}

impl WideKeyword {
    pub fn from_ident(ident: &str) -> Option<Self> {
        let lower = ident.to_ascii_lowercase();
        match lower.as_str() {
            "inherit" => Some(Self::Inherit),
            "initial" => Some(Self::Initial),
            "unset" | "revert" | "revert-layer" => Some(Self::Unset),
            _ => None,
        }
    }
}

/// 1つのロングハンド宣言。ショートハンドはパース時に複数の Decl へ展開される。
#[derive(Clone, Debug, PartialEq)]
pub enum Decl {
    Display(Display),
    Position(Position),
    Float(Float),
    Clear(Clear),
    Inset(Side, LengthOrAuto),
    Margin(Side, LengthOrAuto),
    Padding(Side, Length),
    BorderWidth(Side, Length),
    BorderStyle(Side, BorderStyle),
    BorderColor(Side, ColorValue),
    /// 角（0 = 左上、1 = 右上、2 = 右下、3 = 左下）の丸み
    BorderRadius(usize, Length),
    Size(SizeProp, LengthOrAuto),
    BoxSizing(BoxSizing),
    OverflowX(Overflow),
    OverflowY(Overflow),
    Opacity(f32),
    BackgroundColor(ColorValue),
    FlexDirection(FlexDirection),
    FlexWrap(FlexWrap),
    FlexGrow(f32),
    FlexShrink(f32),
    FlexBasis(LengthOrAuto),
    JustifyContent(Align),
    AlignItems(Align),
    AlignSelf(Align),
    AlignContent(Align),
    RowGap(Length),
    ColumnGap(Length),
    Order(i32),
    /// `None` は `auto`
    ZIndex(Option<i32>),
    Grid(GridProp, Option<Arc<str>>),
    BorderCollapse(BorderCollapse),
    BorderSpacing(Length),
    Color(ColorValue),
    FontFamily(Arc<[Box<str>]>),
    FontSize(FontSizeValue),
    FontWeight(FontWeightValue),
    FontStyle(FontStyle),
    LineHeight(LineHeightValue),
    TextAlign(TextAlign),
    TextIndent(Length),
    TextTransform(TextTransform),
    WhiteSpace(WhiteSpace),
    ListStyleType(ListStyleType),
    ListStylePosition(ListStylePosition),
    Visibility(Visibility),
    TextDecorationLine(u8),
    VerticalAlign(VerticalAlign),
    /// `--name: value`
    Custom(LocalName, Arc<str>),
    /// `var()` を含むため、カスケード時に置換してから再パースする宣言
    Unparsed(LocalName, Arc<str>),
    /// CSS全体キーワード。プロパティ名（ショートハンド可）で保持する
    Wide(LocalName, WideKeyword),
}

impl Decl {
    /// フォントサイズ系か（em の基準になるため先に適用する）。
    pub fn is_font_size(&self) -> bool {
        match self {
            Self::FontSize(_) => true,
            Self::Wide(name, _) | Self::Unparsed(name, _) => {
                matches!(&**name, "font-size" | "font" | "all")
            }
            _ => false,
        }
    }
}

/// `!important` 付きかどうかを持つ宣言ブロック。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeclarationBlock {
    pub normal: Vec<Decl>,
    pub important: Vec<Decl>,
}

impl DeclarationBlock {
    pub fn is_empty(&self) -> bool {
        self.normal.is_empty() && self.important.is_empty()
    }
}
