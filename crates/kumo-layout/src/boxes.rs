//! ボックスツリー。DOM とは別に、レイアウト対象の箱（匿名ボックスを含む）を持つ。
//! レイアウトツリーは画面外・非アクティブ時に破棄できる派生データとして扱う（設計書）。

use std::sync::Arc;

use kumo_css::ComputedStyle;
use kumo_dom::NodeId as DomId;
use kumo_text::{BrushId, Layout, TextStyleSpec};
use taffy::{Cache, Layout as TaffyLayout};

/// ボックスの種類。
#[derive(Debug)]
pub enum BoxKind {
    /// block / flex / grid / table（grid で近似）のコンテナ
    Container,
    /// 匿名のインライン整形コンテキスト（テキストとインライン要素の並び）
    Inline(Box<InlineContent>),
    /// 画像などの置換要素
    Replaced(ReplacedContent),
}

/// 置換要素の内容。
#[derive(Clone, Debug, PartialEq)]
pub struct ReplacedContent {
    /// 画像のURL（未解決）。画像以外は `None`
    pub src: Option<Arc<str>>,
    /// 固有サイズ（不明なら `None`）
    pub intrinsic: Option<(f32, f32)>,
}

/// テキストの描画属性。Parley の Brush（`BrushId`）から引く。
#[derive(Clone, Debug, PartialEq)]
pub struct TextPaint {
    pub style: Arc<ComputedStyle>,
    /// このテキストを含む最も近い要素
    pub node: Option<DomId>,
    /// 最も近い `<a href>` 祖先の href（未解決のまま）
    pub link: Option<Arc<str>>,
}

/// インラインレベル要素の背景・枠（`<code>` の背景など）を描くための区間。
#[derive(Clone, Debug)]
pub struct InlineSpan {
    pub style: Arc<ComputedStyle>,
    /// 段落テキスト上のバイト範囲
    pub range: std::ops::Range<usize>,
}

/// Parley に渡す前のインライン内容の並び。
#[derive(Clone, Debug)]
pub enum InlineItem {
    PushStyle(TextStyleSpec),
    PopStyle,
    Text(String),
    /// アトミックインライン（inline-block・画像）。値は子ボックスの添字
    Atomic(usize),
    /// 段落の途中にあるフロート。行分割中にブロック整形コンテキストへ配置する
    Float(usize),
    /// インライン要素の開始・終了（背景描画用の区間記録）
    SpanStart(Arc<ComputedStyle>),
    SpanEnd,
}

/// インライン整形コンテキスト。
#[derive(Debug)]
pub struct InlineContent {
    pub root_style: TextStyleSpec,
    pub items: Vec<InlineItem>,
    pub paints: Vec<TextPaint>,
    pub text_align: kumo_css::keywords::TextAlign,
    pub text_indent: f32,
    /// シェーピング済みの段落（初回の計測時に作る）
    pub layout: Option<Layout<BrushId>>,
    /// Parley が空白処理した後の段落テキスト
    pub text: String,
    pub spans: Vec<InlineSpan>,
    /// アトミックインラインの子ボックス添字（Parley の InlineBox id と対応）
    pub atomics: Vec<usize>,
    /// 段落内のフロートの子ボックス添字
    pub floats: Vec<usize>,
    /// Parley の段落に現在適用されている改行
    pub breaks: Option<BreakPlan>,
    /// 最終配置（PerformLayout）で確定した改行。描画はこれに従う
    pub final_breaks: Option<BreakPlan>,
}

/// 1行分の配置枠（フロートを避けた位置と幅）。`x`/`y` は Parley の行位置（line_x/line_y）
/// として渡すので、グリフ座標にはすでに反映されている。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineSlot {
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

/// 改行のしかた。`lines` が空なら全行を `width` で折り返す。
#[derive(Clone, Debug, PartialEq)]
pub struct BreakPlan {
    pub width: Option<f32>,
    pub lines: Vec<LineSlot>,
}

impl BreakPlan {
    pub fn uniform(width: Option<f32>) -> Self {
        Self {
            width,
            lines: Vec::new(),
        }
    }
}

impl InlineContent {
    /// 空白以外の内容を持つか（空白だけの匿名ボックスは作らない）。
    pub fn has_content(&self) -> bool {
        self.items.iter().any(|item| match item {
            InlineItem::Text(t) => !t.trim().is_empty(),
            InlineItem::Atomic(_) | InlineItem::Float(_) => true,
            _ => false,
        })
    }
}

/// ボックスツリーの1ノード。
#[derive(Debug)]
pub struct LayoutBox {
    pub kind: BoxKind,
    /// 生成元の DOM 要素（匿名ボックスは `None`）
    pub dom: Option<DomId>,
    pub computed: Arc<ComputedStyle>,
    pub style: taffy::Style<Arc<str>>,
    pub children: Vec<usize>,
    /// リストマーカー（`list-style-position: outside` のみ）
    pub marker: Option<Box<Layout<BrushId>>>,
    /// このボックスを含む最も近い `<a href>` の href（クリック判定用）
    pub link: Option<Arc<str>>,
    /// ページ内リンクの飛び先名（要素の `id`、または `<a name>`）
    pub anchor: Option<Arc<str>>,
    /// クリックで操作するフォーム部品（input・select・textarea・button）か
    pub form_control: bool,
    pub(crate) cache: Cache,
    pub(crate) unrounded: TaffyLayout,
    pub(crate) final_layout: TaffyLayout,
    pub(crate) hoisted: Vec<taffy::NodeId>,
    /// 子孫にフロートを含むか（含むならフロート文脈を伝えるためキャッシュを使わない）
    pub(crate) has_float_inside: bool,
}

impl LayoutBox {
    pub fn new(kind: BoxKind, dom: Option<DomId>, computed: Arc<ComputedStyle>) -> Self {
        Self {
            kind,
            dom,
            computed,
            style: taffy::Style::default(),
            children: Vec::new(),
            marker: None,
            link: None,
            anchor: None,
            form_control: false,
            cache: Cache::new(),
            unrounded: TaffyLayout::with_order(0),
            final_layout: TaffyLayout::with_order(0),
            hoisted: Vec::new(),
            has_float_inside: false,
        }
    }

    /// 丸め後の最終レイアウト（親のボーダーボックス原点からの相対位置）。
    pub fn layout(&self) -> &TaffyLayout {
        &self.final_layout
    }

    pub fn inline(&self) -> Option<&InlineContent> {
        match &self.kind {
            BoxKind::Inline(content) => Some(content),
            _ => None,
        }
    }
}
