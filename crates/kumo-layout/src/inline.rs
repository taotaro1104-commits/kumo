//! インライン整形コンテキストのレイアウト（Parley で改行・配置する）。
//!
//! Parley の段落は初回に一度だけ作り、以後はアトミックの大きさを差し替えて改行し直す。
//! Taffy は同じノードを別の幅で何度も計測するため、最終配置（PerformLayout）で確定した
//! 改行を `final_breaks` に記録し、レイアウト完了後に [`finalize_breaks`] でそれへ戻す。

use kumo_css::keywords::TextAlign;
use kumo_text::{Alignment, AlignmentOptions, InlineBox, InlineBoxKind, PositionedLayoutItem};
use taffy::Point;
use taffy::prelude::*;
use taffy::{
    Baselines, BlockContext, LayoutInput, LayoutOutput, LayoutPartialTree, RequestedAxis, RunMode,
    SizingMode,
};

use crate::boxes::{BoxKind, BreakPlan, InlineContent, InlineItem, InlineSpan};
use crate::inline_break::{apply_plan, break_around_floats};
use crate::tree::BoxTree;

/// インラインノードの計測・配置。`floats` があれば各行をフロートの隙間に流し込み、
/// 段落内のフロートもそこへ配置する。
pub fn compute_inline_layout(
    tree: &mut BoxTree,
    id: NodeId,
    inputs: LayoutInput,
    floats: Option<&mut BlockContext<'_>>,
) -> LayoutOutput {
    let index = usize::from(id);
    let width = inputs
        .known_dimensions
        .width
        .or(match inputs.available_space.width {
            AvailableSpace::Definite(w) => Some(w),
            AvailableSpace::MinContent => Some(0.0),
            AvailableSpace::MaxContent => None,
        });
    ensure_built(tree, index);
    if refresh_atomic_sizes(tree, index, width) {
        inline_mut(tree, index).breaks = None;
    }
    let has_floats_inside = !inline_mut(tree, index).floats.is_empty();
    let plan = match (floats, width) {
        (Some(ctx), Some(width)) if ctx.has_floats() || has_floats_inside => {
            break_around_floats(tree, index, width, ctx, inputs.run_mode)
        }
        _ => {
            let plan = BreakPlan::uniform(width);
            let content = inline_mut(tree, index);
            if content.breaks.as_ref() != Some(&plan) {
                apply_plan(content.layout.as_mut().expect("構築済み"), &plan);
            }
            if inputs.run_mode == RunMode::PerformLayout && has_floats_inside {
                place_floats_without_context(tree, index, width);
            }
            plan
        }
    };
    let content = inline_mut(tree, index);
    content.breaks = Some(plan.clone());
    let layout = content.layout.as_mut().expect("構築済み");
    let text_width = match inputs.available_space.width {
        AvailableSpace::MinContent if inputs.known_dimensions.width.is_none() => {
            layout.calculate_content_widths().min
        }
        _ => layout.width(),
    };
    // フロートの下へ送った行は line_y でずれているので、最後の行の下端まで含める
    let text_height = layout
        .lines()
        .map(|l| l.metrics().block_max_coord)
        .fold(layout.height(), f32::max);
    let size = Size {
        width: inputs.known_dimensions.width.unwrap_or(text_width),
        height: inputs.known_dimensions.height.unwrap_or(text_height),
    };
    let first = layout.lines().next().map(|l| l.metrics().baseline);
    let last = layout.lines().last().map(|l| l.metrics().baseline);
    if inputs.run_mode == RunMode::PerformLayout {
        layout.align(alignment(content.text_align), AlignmentOptions::default());
        content.final_breaks = Some(plan);
        place_atomics(tree, index);
    }
    LayoutOutput::from_sizes_and_baselines(size, Rect::ZERO, Baselines { first, last })
}

/// レイアウト完了後、計測のために別の幅で折り返されたままの段落を確定した改行へ戻す。
pub fn finalize_breaks(tree: &mut BoxTree) {
    for b in &mut tree.boxes {
        let BoxKind::Inline(content) = &mut b.kind else {
            continue;
        };
        let Some(plan) = content.final_breaks.clone() else {
            continue;
        };
        if content.breaks.as_ref() == Some(&plan) {
            continue;
        }
        let align = alignment(content.text_align);
        if let Some(layout) = content.layout.as_mut() {
            apply_plan(layout, &plan);
            layout.align(align, AlignmentOptions::default());
        }
        content.breaks = Some(plan);
    }
}

pub(crate) fn inline_mut(tree: &mut BoxTree, index: usize) -> &mut InlineContent {
    match &mut tree.boxes[index].kind {
        BoxKind::Inline(content) => content,
        _ => unreachable!("インラインノードではない"),
    }
}

fn alignment(align: TextAlign) -> Alignment {
    match align {
        TextAlign::Start => Alignment::Start,
        TextAlign::End => Alignment::End,
        TextAlign::Left => Alignment::Left,
        TextAlign::Right => Alignment::Right,
        TextAlign::Center => Alignment::Center,
        TextAlign::Justify => Alignment::Justify,
    }
}

/// アトミックインラインの外寸（幅・高さ・ベースライン、マージン込み）。
type AtomicSize = (f32, f32, Option<f32>);

/// 子ボックスを計測する（Taffy のキャッシュが効くので繰り返し呼んでも安い）。
pub(crate) fn measure_child(
    tree: &mut BoxTree,
    child: usize,
    width: Option<f32>,
    run_mode: RunMode,
) -> AtomicSize {
    let available = width.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite);
    let output = tree.compute_child_layout(
        NodeId::from(child),
        LayoutInput {
            run_mode,
            sizing_mode: SizingMode::InherentSize,
            axis: RequestedAxis::Both,
            known_dimensions: Size::NONE,
            known_dimensions_are_definite: Size {
                width: false,
                height: false,
            },
            parent_size: Size {
                width,
                height: None,
            },
            available_space: Size {
                width: available,
                height: AvailableSpace::MaxContent,
            },
            vertical_margins_are_collapsible: Line::FALSE,
        },
    );
    let margin = margins(tree, child, width);
    let w = output.size.width + margin.left + margin.right;
    let h = output.size.height + margin.top + margin.bottom;
    let baseline = output.baselines.first.map(|b| b + margin.top);
    (w, h, baseline)
}

pub(crate) fn margins(tree: &BoxTree, child: usize, width: Option<f32>) -> Rect<f32> {
    let basis = width.unwrap_or(0.0);
    let m = &tree.boxes[child].computed.margin;
    let resolve = |v: &kumo_css::LengthPctAuto| match v {
        kumo_css::LengthPctAuto::LengthPct(l) => l.resolve(basis),
        kumo_css::LengthPctAuto::Auto => 0.0,
    };
    Rect {
        left: resolve(&m.left),
        right: resolve(&m.right),
        top: resolve(&m.top),
        bottom: resolve(&m.bottom),
    }
}

/// Parley の段落を一度だけ作る（アトミックの大きさは後で差し替える）。
fn ensure_built(tree: &mut BoxTree, index: usize) {
    let content = inline_mut(tree, index);
    if content.layout.is_some() {
        return;
    }
    let items = std::mem::take(&mut content.items);
    let root = content.root_style.clone();
    let mut builder = tree.text.tree_builder(&root);
    let mut spans: Vec<InlineSpan> = Vec::new();
    let mut open: Vec<(std::sync::Arc<kumo_css::ComputedStyle>, usize)> = Vec::new();
    for item in &items {
        match item {
            InlineItem::PushStyle(spec) => builder.push_style_span(spec.to_parley()),
            InlineItem::PopStyle => builder.pop_style_span(),
            InlineItem::Text(text) => builder.push_text(text),
            InlineItem::Atomic(child) | InlineItem::Float(child) => {
                let kind = if matches!(item, InlineItem::Float(_)) {
                    InlineBoxKind::CustomOutOfFlow
                } else {
                    InlineBoxKind::InFlow
                };
                builder.push_inline_box(InlineBox {
                    id: *child as u64,
                    kind,
                    index: 0,
                    width: 0.0,
                    height: 0.0,
                    baseline: None,
                    vertical_align: kumo_text::VerticalAlign::BASELINE,
                });
            }
            InlineItem::SpanStart(style) => open.push((style.clone(), builder.text().len())),
            InlineItem::SpanEnd => {
                if let Some((style, start)) = open.pop() {
                    spans.push(InlineSpan {
                        style,
                        range: start..builder.text().len(),
                    });
                }
            }
        }
    }
    let (mut layout, text) = builder.build();
    let content = inline_mut(tree, index);
    if content.text_indent != 0.0 {
        layout.set_text_indent(content.text_indent, Default::default());
    }
    content.items = items;
    content.layout = Some(layout);
    content.text = text;
    content.spans = spans;
    content.breaks = None;
}

/// アトミックインラインを `width` で計測し直し、段落内の箱の大きさを更新する。
/// 大きさが変わったら `true`（改行をやり直す必要がある）。
fn refresh_atomic_sizes(tree: &mut BoxTree, index: usize, width: Option<f32>) -> bool {
    let atomics = inline_mut(tree, index).atomics.clone();
    if atomics.is_empty() {
        return false;
    }
    let sizes: Vec<(u64, AtomicSize)> = atomics
        .into_iter()
        .map(|child| {
            (
                child as u64,
                measure_child(tree, child, width, RunMode::ComputeSize),
            )
        })
        .collect();
    let layout = inline_mut(tree, index).layout.as_mut().expect("構築済み");
    let mut changed = false;
    for b in layout.inline_boxes_mut() {
        let Some(&(_, (w, h, baseline))) = sizes.iter().find(|(id, _)| *id == b.id) else {
            continue;
        };
        if b.width != w || b.height != h || b.baseline != baseline {
            (b.width, b.height, b.baseline) = (w, h, baseline);
            changed = true;
        }
    }
    changed
}

/// フロート文脈がない（flex アイテム直下など）場合は、段落内フロートを左右の端に置く。
fn place_floats_without_context(tree: &mut BoxTree, index: usize, width: Option<f32>) {
    let floats = inline_mut(tree, index).floats.clone();
    for child in floats {
        let (w, h, _) = measure_child(tree, child, width, RunMode::PerformLayout);
        let margin = margins(tree, child, width);
        let right = tree.boxes[child].computed.float == kumo_css::keywords::Float::Right;
        let x = if right { width.unwrap_or(w) - w } else { 0.0 };
        set_child_position(
            tree,
            child,
            x + margin.left,
            margin.top,
            w - margin.left - margin.right,
            h - margin.top - margin.bottom,
        );
    }
}

pub(crate) fn set_child_position(tree: &mut BoxTree, child: usize, x: f32, y: f32, w: f32, h: f32) {
    let layout = Layout {
        location: Point { x, y },
        size: Size {
            width: w.max(0.0),
            height: h.max(0.0),
        },
        ..Layout::with_order(0)
    };
    tree.set_unrounded_layout(NodeId::from(child), &layout);
}

/// 改行・整列後の位置をアトミックインラインの子ボックスに反映する。
fn place_atomics(tree: &mut BoxTree, index: usize) {
    let content = inline_mut(tree, index);
    if content.atomics.is_empty() {
        return;
    }
    let layout = content.layout.as_ref().expect("構築済み");
    let mut placed = Vec::new();
    for line in layout.lines() {
        for item in line.items() {
            if let PositionedLayoutItem::InlineBox(b) = item
                && b.kind == InlineBoxKind::InFlow
            {
                placed.push((b.id as usize, b.x, b.y, b.width, b.height));
            }
        }
    }
    let parent_width = Some(layout.width());
    for (child, x, y, w, h) in placed {
        let margin = margins(tree, child, parent_width);
        let size = Size {
            width: (w - margin.left - margin.right).max(0.0),
            height: (h - margin.top - margin.bottom).max(0.0),
        };
        tree.compute_child_layout(
            NodeId::from(child),
            LayoutInput {
                run_mode: RunMode::PerformLayout,
                sizing_mode: SizingMode::InherentSize,
                axis: RequestedAxis::Both,
                known_dimensions: size.map(Some),
                known_dimensions_are_definite: Size {
                    width: true,
                    height: true,
                },
                parent_size: Size {
                    width: parent_width,
                    height: None,
                },
                available_space: size.map(AvailableSpace::Definite),
                vertical_margins_are_collapsible: Line::FALSE,
            },
        );
        set_child_position(
            tree,
            child,
            x + margin.left,
            y + margin.top,
            size.width,
            size.height,
        );
    }
}
