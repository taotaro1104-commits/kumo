//! ボックスツリーからディスプレイリストを作る。

use kumo_css::keywords::BorderStyle;
use kumo_css::values::Rgba;
use kumo_css::{ComputedStyle, Side};
use kumo_layout::{BoxKind, BoxTree, InlineContent, LayoutBox};
use kumo_text::PositionedLayoutItem;

use crate::display_list::{DisplayItem, DisplayList, GlyphRunItem, PlacedGlyph, RectF};
use crate::stacking::{Placed, paint_layer};

/// ボックスツリー全体をディスプレイリストに変換する。
pub fn build_display_list(tree: &BoxTree) -> DisplayList {
    let mut list = DisplayList {
        width: tree.boxes[0].layout().size.width,
        height: kumo_layout::document_height(tree),
        background: canvas_background(tree),
        ..DisplayList::default()
    };
    let root = tree.boxes[0].layout();
    let placed = Placed {
        index: 0,
        rect: RectF::new(
            root.location.x,
            root.location.y,
            root.size.width,
            root.size.height,
        ),
        clip: None,
    };
    paint_layer(tree, placed, &mut list);
    list
}

/// ルート要素（なければ body）の背景をキャンバス全体に伝播する（CSS 2.1 §14.2）。
fn canvas_background(tree: &BoxTree) -> Option<Rgba> {
    let html = tree.boxes[0].children.first().copied()?;
    let html_bg = tree.boxes[html].computed.background_color;
    if !html_bg.is_transparent() {
        return Some(html_bg);
    }
    tree.boxes[html]
        .children
        .iter()
        .map(|&c| tree.boxes[c].computed.background_color)
        .find(|c| !c.is_transparent())
}

/// ボックス自身の内容（リストマーカー・インラインのテキスト・置換要素の画像）を描く。
pub(crate) fn paint_content(
    tree: &BoxTree,
    index: usize,
    rect: RectF,
    clip: Option<RectF>,
    list: &mut DisplayList,
) {
    let b = &tree.boxes[index];
    if let Some(marker) = &b.marker {
        paint_marker(b, marker, rect, clip, list);
    }
    match &b.kind {
        BoxKind::Inline(content) => paint_inline(content, rect.x, rect.y, clip, list),
        BoxKind::Replaced(replaced) => {
            if let Some(src) = &replaced.src {
                let content = content_box(&b.computed, rect);
                list.push(
                    DisplayItem::Image {
                        rect: content,
                        src: src.clone(),
                    },
                    content,
                    clip,
                );
            }
        }
        BoxKind::Container => {}
    }
}

pub(crate) fn inset(rect: RectF, mut f: impl FnMut(Side) -> f32) -> RectF {
    let (t, r, bo, l) = (f(Side::Top), f(Side::Right), f(Side::Bottom), f(Side::Left));
    RectF::new(
        rect.x + l,
        rect.y + t,
        (rect.w - l - r).max(0.0),
        (rect.h - t - bo).max(0.0),
    )
}

fn content_box(style: &ComputedStyle, rect: RectF) -> RectF {
    let border = inset(rect, |s| style.used_border_width(s));
    inset(border, |s| style.padding.get(s).resolve(rect.w))
}

pub(crate) fn paint_background_and_borders(
    style: &ComputedStyle,
    rect: RectF,
    clip: Option<RectF>,
    list: &mut DisplayList,
) {
    if !style.background_color.is_transparent() {
        list.push(
            DisplayItem::Fill {
                rect,
                color: style.background_color,
            },
            rect,
            clip,
        );
    }
    for side in Side::ALL {
        let width = style.used_border_width(side);
        let color = *style.border_color.get(side);
        if width <= 0.0 || color.is_transparent() {
            continue;
        }
        let edge = match side {
            Side::Top => RectF::new(rect.x, rect.y, rect.w, width),
            Side::Bottom => RectF::new(rect.x, rect.bottom() - width, rect.w, width),
            Side::Left => RectF::new(rect.x, rect.y, width, rect.h),
            Side::Right => RectF::new(rect.right() - width, rect.y, width, rect.h),
        };
        paint_border_edge(
            edge,
            *style.border_style.get(side),
            width,
            color,
            clip,
            list,
        );
    }
}

/// 境界線1辺。点線・破線は短い矩形の列で近似する。
fn paint_border_edge(
    edge: RectF,
    style: BorderStyle,
    width: f32,
    color: Rgba,
    clip: Option<RectF>,
    list: &mut DisplayList,
) {
    let dash = match style {
        BorderStyle::Dotted => width,
        BorderStyle::Dashed => width * 3.0,
        _ => {
            list.push(DisplayItem::Fill { rect: edge, color }, edge, clip);
            return;
        }
    };
    let horizontal = edge.w >= edge.h;
    let length = if horizontal { edge.w } else { edge.h };
    let mut pos = 0.0;
    while pos < length {
        let seg = dash.min(length - pos);
        let rect = if horizontal {
            RectF::new(edge.x + pos, edge.y, seg, edge.h)
        } else {
            RectF::new(edge.x, edge.y + pos, edge.w, seg)
        };
        list.push(DisplayItem::Fill { rect, color }, rect, clip);
        pos += dash * 2.0;
    }
}

/// outside マーカーを内容ボックスの左、1行目のベースラインに揃えて置く。
fn paint_marker(
    b: &LayoutBox,
    marker: &kumo_text::Layout<u32>,
    rect: RectF,
    clip: Option<RectF>,
    list: &mut DisplayList,
) {
    let content = content_box(&b.computed, rect);
    let gap = b.computed.font_size * 0.5;
    let x = content.x - marker.width() - gap;
    paint_layout(marker, x, content.y, &|_| b.computed.color, clip, list);
}

fn paint_inline(
    content: &InlineContent,
    x: f32,
    y: f32,
    clip: Option<RectF>,
    list: &mut DisplayList,
) {
    let Some(layout) = &content.layout else {
        return;
    };
    paint_span_backgrounds(content, layout, x, y, clip, list);
    let color_of = |brush: u32| {
        content
            .paints
            .get(brush as usize)
            .map_or(Rgba::BLACK, |p| p.style.color)
    };
    paint_layout(layout, x, y, &color_of, clip, list);
}

/// Parley のレイアウトをグリフ列と装飾線に変換する。
fn paint_layout(
    layout: &kumo_text::Layout<u32>,
    x: f32,
    y: f32,
    color_of: &dyn Fn(u32) -> Rgba,
    clip: Option<RectF>,
    list: &mut DisplayList,
) {
    for line in layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                continue;
            };
            let style = glyph_run.style();
            let color = color_of(style.brush);
            if color.is_transparent() {
                continue;
            }
            let run = glyph_run.run();
            let metrics = run.font_metrics();
            let glyphs: Vec<PlacedGlyph> = glyph_run
                .positioned_glyphs()
                .map(|g| PlacedGlyph {
                    id: g.id,
                    x: x + g.x,
                    y: y + g.y,
                })
                .collect();
            let baseline = y + glyph_run.baseline();
            let left = x + glyph_run.offset();
            let advance = glyph_run.advance();
            let bounds = RectF::new(
                left,
                baseline - metrics.ascent,
                advance,
                metrics.ascent + metrics.descent.abs(),
            );
            if !glyphs.is_empty() {
                let item = GlyphRunItem {
                    font: run.font().clone(),
                    size: run.font_size(),
                    coords: run
                        .normalized_coords()
                        .iter()
                        .map(|c| c.to_bits())
                        .collect(),
                    glyphs,
                    color,
                };
                list.push(DisplayItem::Glyphs(Box::new(item)), bounds, clip);
            }
            let mut decorate = |offset: f32, size: f32| {
                let thickness = size.max(1.0);
                let rect = RectF::new(left, baseline - offset, advance, thickness);
                list.push(DisplayItem::Fill { rect, color }, rect, clip);
            };
            if let Some(d) = &style.underline {
                decorate(
                    d.offset.unwrap_or(metrics.underline_offset),
                    d.size.unwrap_or(metrics.underline_size),
                );
            }
            if let Some(d) = &style.strikethrough {
                decorate(
                    d.offset.unwrap_or(metrics.strikethrough_offset),
                    d.size.unwrap_or(metrics.strikethrough_size),
                );
            }
        }
    }
}

/// インライン要素（`<code>` など）の背景。行ごと・グリフ列単位で近似する。
fn paint_span_backgrounds(
    content: &InlineContent,
    layout: &kumo_text::Layout<u32>,
    x: f32,
    y: f32,
    clip: Option<RectF>,
    list: &mut DisplayList,
) {
    for span in &content.spans {
        let color = span.style.background_color;
        if color.is_transparent() || span.range.is_empty() {
            continue;
        }
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let range = glyph_run.run().text_range();
                if range.start >= span.range.end || range.end <= span.range.start {
                    continue;
                }
                let metrics = glyph_run.run().font_metrics();
                let baseline = y + glyph_run.baseline();
                let rect = RectF::new(
                    x + glyph_run.offset(),
                    baseline - metrics.ascent,
                    glyph_run.advance(),
                    metrics.ascent + metrics.descent.abs(),
                );
                list.push(DisplayItem::Fill { rect, color }, rect, clip);
            }
        }
    }
}
