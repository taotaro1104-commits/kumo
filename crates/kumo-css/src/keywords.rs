//! キーワード値の列挙型。`keyword_enum!` でパーサも同時に生成する。

use cssparser::Parser;

use crate::values::{ParseResult, invalid};

macro_rules! keyword_enum {
    ($(#[$meta:meta])* $name:ident default $default:ident { $($variant:ident = $($css:literal)|+),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant),+
        }

        impl Default for $name {
            fn default() -> Self {
                Self::$default
            }
        }

        impl $name {
            pub fn from_ident(ident: &str) -> Option<Self> {
                $(
                    if false $(|| ident.eq_ignore_ascii_case($css))+ {
                        return Some(Self::$variant);
                    }
                )+
                None
            }

            pub fn parse<'i>(input: &mut Parser<'i>) -> ParseResult<Self> {
                let ident = input.expect_ident()?.clone();
                Self::from_ident(&ident).map_or_else(invalid, Ok)
            }
        }
    };
}

keyword_enum!(Display default Inline {
    None = "none",
    Block = "block",
    Inline = "inline",
    InlineBlock = "inline-block",
    Flex = "flex",
    InlineFlex = "inline-flex",
    Grid = "grid",
    InlineGrid = "inline-grid",
    ListItem = "list-item",
    FlowRoot = "flow-root",
    Contents = "contents",
    Table = "table",
    InlineTable = "inline-table",
    TableRow = "table-row",
    TableCell = "table-cell",
    TableRowGroup = "table-row-group" | "table-header-group" | "table-footer-group",
    TableCaption = "table-caption",
    TableColumn = "table-column" | "table-column-group",
});

impl Display {
    /// 外側の表示型がインラインレベルか。
    pub fn is_inline_level(self) -> bool {
        matches!(
            self,
            Self::Inline
                | Self::InlineBlock
                | Self::InlineFlex
                | Self::InlineGrid
                | Self::InlineTable
        )
    }

    /// float / absolute / ルート要素で行う「ブロック化」。
    pub fn blockify(self) -> Self {
        match self {
            Self::Inline | Self::InlineBlock => Self::Block,
            Self::InlineFlex => Self::Flex,
            Self::InlineGrid => Self::Grid,
            Self::InlineTable => Self::Table,
            other => other,
        }
    }
}

keyword_enum!(Position default Static {
    Static = "static",
    Relative = "relative",
    Absolute = "absolute",
    Fixed = "fixed",
    Sticky = "sticky",
});

keyword_enum!(Float default None {
    None = "none",
    Left = "left" | "inline-start",
    Right = "right" | "inline-end",
});

keyword_enum!(Clear default None {
    None = "none",
    Left = "left" | "inline-start",
    Right = "right" | "inline-end",
    Both = "both",
});

keyword_enum!(BorderStyle default None {
    None = "none",
    Hidden = "hidden",
    Solid = "solid",
    Dashed = "dashed",
    Dotted = "dotted",
    Double = "double",
    Groove = "groove",
    Ridge = "ridge",
    Inset = "inset",
    Outset = "outset",
});

keyword_enum!(BoxSizing default ContentBox {
    ContentBox = "content-box",
    BorderBox = "border-box",
});

keyword_enum!(FontStyle default Normal {
    Normal = "normal",
    Italic = "italic" | "oblique",
});

keyword_enum!(TextAlign default Start {
    Start = "start",
    End = "end",
    Left = "left" | "-webkit-left",
    Right = "right" | "-webkit-right",
    Center = "center" | "-webkit-center",
    Justify = "justify",
});

keyword_enum!(TextTransform default None {
    None = "none",
    Uppercase = "uppercase",
    Lowercase = "lowercase",
    Capitalize = "capitalize",
});

keyword_enum!(WhiteSpace default Normal {
    Normal = "normal",
    Pre = "pre",
    Nowrap = "nowrap",
    PreWrap = "pre-wrap" | "break-spaces",
    PreLine = "pre-line",
});

impl WhiteSpace {
    pub fn collapses_spaces(self) -> bool {
        matches!(self, Self::Normal | Self::Nowrap | Self::PreLine)
    }

    pub fn wraps(self) -> bool {
        !matches!(self, Self::Pre | Self::Nowrap)
    }
}

keyword_enum!(VerticalAlign default Baseline {
    Baseline = "baseline",
    Sub = "sub",
    Super = "super",
    Top = "top" | "text-top",
    Middle = "middle",
    Bottom = "bottom" | "text-bottom",
});

keyword_enum!(ListStyleType default Disc {
    None = "none",
    Disc = "disc",
    Circle = "circle",
    Square = "square",
    Decimal = "decimal" | "decimal-leading-zero",
    LowerAlpha = "lower-alpha" | "lower-latin",
    UpperAlpha = "upper-alpha" | "upper-latin",
    LowerRoman = "lower-roman",
    UpperRoman = "upper-roman",
});

keyword_enum!(ListStylePosition default Outside {
    Outside = "outside",
    Inside = "inside",
});

keyword_enum!(Overflow default Visible {
    Visible = "visible",
    Hidden = "hidden",
    Clip = "clip",
    Scroll = "scroll",
    Auto = "auto" | "overlay",
});

keyword_enum!(Visibility default Visible {
    Visible = "visible",
    Hidden = "hidden",
    Collapse = "collapse",
});

keyword_enum!(FlexDirection default Row {
    Row = "row",
    RowReverse = "row-reverse",
    Column = "column",
    ColumnReverse = "column-reverse",
});

keyword_enum!(FlexWrap default NoWrap {
    NoWrap = "nowrap",
    Wrap = "wrap",
    WrapReverse = "wrap-reverse",
});

keyword_enum!(
    /// justify-content / align-items などで共通に使う配置キーワード。
    Align default Normal {
    Normal = "normal",
    Auto = "auto",
    Start = "start" | "self-start",
    End = "end" | "self-end",
    FlexStart = "flex-start",
    FlexEnd = "flex-end",
    Center = "center",
    Stretch = "stretch",
    Baseline = "baseline" | "first",
    SpaceBetween = "space-between",
    SpaceAround = "space-around",
    SpaceEvenly = "space-evenly",
    Left = "left",
    Right = "right",
});

keyword_enum!(BorderCollapse default Separate {
    Separate = "separate",
    Collapse = "collapse",
});
