//! 行分割：一様な幅での改行、フロートを避けた改行、記録した改行の再現。

use kumo_css::keywords::{Clear as CssClear, Float as CssFloat};
use kumo_text::parley::YieldData;
use taffy::prelude::*;
use taffy::style::{Clear, FloatDirection};
use taffy::{BlockContext, RunMode};

use crate::boxes::{BreakPlan, LineSlot};
use crate::inline::{inline_mut, margins, measure_child, set_child_position};
use crate::tree::BoxTree;

/// 狭い隙間を諦めて下へ送る試行の上限（フロートが大量にあっても止まるように）。
const MAX_PUSH_DOWN: usize = 64;
/// 行があふれたとみなす許容誤差（px）。
const OVERFLOW_EPSILON: f32 = 0.5;

/// 記録した改行をそのまま再現する（フロートはすでに配置済みなので触らない）。
pub fn apply_plan(layout: &mut kumo_text::Layout<u32>, plan: &BreakPlan) {
    let Some(width) = plan.width.filter(|_| !plan.lines.is_empty()) else {
        layout.break_all_lines(plan.width);
        return;
    };
    let mut breaker = layout.break_lines();
    breaker.state_mut().set_layout_max_advance(width);
    let mut line = 0;
    loop {
        let slot = plan.lines.get(line).copied().unwrap_or(LineSlot {
            x: 0.0,
            y: breaker.committed_y() as f32,
            width,
        });
        let state = breaker.state_mut();
        state.set_line_y(f64::from(slot.y));
        state.set_line_x(slot.x);
        state.set_line_max_advance(slot.width);
        match breaker.break_next() {
            None => break,
            Some(YieldData::InlineBoxBreak(data)) => {
                breaker
                    .state_mut()
                    .append_inline_box_to_line(data.advance, 0.0, 0.0);
            }
            Some(_) => line += 1,
        }
    }
    breaker.finish();
}

/// 各行をフロートの隙間に流し込み、行の配置枠を記録する。
///
/// - 隙間が狭くて1語も収まらない行は取り消し、フロートの下の次の隙間でやり直す（CSS 2.1 §9.5）。
/// - 段落内のフロートに出会ったら、その行の位置でブロック整形コンテキストへ配置する。
pub fn break_around_floats(
    tree: &mut BoxTree,
    index: usize,
    width: f32,
    ctx: &mut BlockContext<'_>,
    run_mode: RunMode,
) -> BreakPlan {
    let width = width.max(0.0);
    // フロートの計測で tree を使うため、段落を一時的に取り出す
    let mut layout = inline_mut(tree, index).layout.take().expect("構築済み");
    let mut lines = Vec::new();
    let mut placed_floats: Vec<usize> = Vec::new();
    {
        let mut breaker = layout.break_lines();
        breaker.state_mut().set_layout_max_advance(width);
        // 狭すぎて諦めた区間の下端。次の行はここから探す
        let mut min_y: Option<f32> = None;
        let mut push_downs = 0;
        loop {
            let y = min_y.unwrap_or(0.0).max(breaker.committed_y() as f32);
            let slot = ctx.find_content_slot(y, Clear::None, None);
            let x = slot.x.clamp(0.0, width);
            let line_width = slot.width.min(width - x).max(0.0);
            let top = slot.y.max(y);
            let state = breaker.state_mut();
            state.set_line_y(f64::from(top));
            state.set_line_x(x);
            state.set_line_max_advance(line_width);
            match breaker.break_next() {
                None => break,
                Some(YieldData::InlineBoxBreak(data)) => {
                    let child = data.inline_box_id as usize;
                    if !placed_floats.contains(&child) {
                        place_float(tree, child, width, top, ctx, run_mode);
                        placed_floats.push(child);
                    }
                    breaker
                        .state_mut()
                        .append_inline_box_to_line(data.advance, 0.0, 0.0);
                }
                Some(YieldData::LineBreak(data)) => {
                    let narrowed =
                        line_width + OVERFLOW_EPSILON < width && slot.segment_id.is_some();
                    let content_width = data.metrics.advance - data.metrics.hanging_advance;
                    let overflows = content_width > line_width + OVERFLOW_EPSILON;
                    if narrowed && overflows && push_downs < MAX_PUSH_DOWN && breaker.revert() {
                        min_y = Some(segment_end(ctx, top, slot.segment_id));
                        push_downs += 1;
                        continue;
                    }
                    lines.push(LineSlot {
                        x,
                        y: top,
                        width: line_width,
                    });
                    min_y = None;
                    push_downs = 0;
                }
                Some(YieldData::MaxHeightExceeded(_)) => {}
            }
        }
        breaker.finish();
    }
    inline_mut(tree, index).layout = Some(layout);
    BreakPlan {
        width: Some(width),
        lines,
    }
}

/// `y` を含むフロート区間の下端を求める（BlockContext は区間を公開しないため二分探索する）。
fn segment_end(ctx: &BlockContext<'_>, y: f32, segment: Option<usize>) -> f32 {
    let same = |probe: f32| ctx.find_content_slot(probe, Clear::None, None).segment_id == segment;
    let mut step = 1.0f32;
    while same(y + step) && step < 1.0e7 {
        step *= 2.0;
    }
    let (mut lo, mut hi) = (y + step / 2.0, y + step);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if same(mid) { lo = mid } else { hi = mid }
    }
    hi.ceil()
}

/// 段落内のフロートを計測し、行の上端 `min_y` 以降でブロック整形コンテキストに置く。
fn place_float(
    tree: &mut BoxTree,
    child: usize,
    width: f32,
    min_y: f32,
    ctx: &mut BlockContext<'_>,
    run_mode: RunMode,
) {
    let (w, h, _) = measure_child(tree, child, Some(width), run_mode);
    let computed = &tree.boxes[child].computed;
    let direction = match computed.float {
        CssFloat::Right => FloatDirection::Right,
        _ => FloatDirection::Left,
    };
    let clear = match computed.clear {
        CssClear::None => Clear::None,
        CssClear::Left => Clear::Left,
        CssClear::Right => Clear::Right,
        CssClear::Both => Clear::Both,
    };
    let position = ctx.place_floated_box(
        Size {
            width: w,
            height: h,
        },
        min_y,
        direction,
        clear,
        false,
    );
    if run_mode == RunMode::PerformLayout {
        let margin = margins(tree, child, Some(width));
        set_child_position(
            tree,
            child,
            position.x + margin.left,
            position.y + margin.top,
            w - margin.left - margin.right,
            h - margin.top - margin.bottom,
        );
    }
}
