//! 表のレイアウト。Taffy には表アルゴリズムがないため、セルを明示配置した
//! grid（列は `auto`）で近似する。auto 列の幅決定は自動表レイアウトに近い結果になる。

use std::sync::Arc;

use kumo_css::keywords::{BorderCollapse, Display as CssDisplay, VerticalAlign};
use kumo_css::{ComputedStyle, LengthPctAuto};
use kumo_dom::NodeId as DomId;
use taffy::prelude::*;
use taffy::style::AlignItems;

use crate::boxes::BoxKind;
use crate::build::Builder;
use crate::convert::auto_columns;

/// 列数・span の上限（主要ブラウザと同程度）。
const MAX_COLUMNS: usize = 1000;
const MAX_ROWSPAN: usize = 1000;
/// grid の行番号（i16）に収まる行数の上限。
const MAX_ROWS: usize = 30_000;

/// セルの占有状況（rowspan のため行をまたいで記録する）。
#[derive(Default)]
struct Occupancy {
    rows: Vec<Vec<bool>>,
    /// 列ごとの幅の指定（セルの `width="25%"` の最大値。なければ 0）
    percents: Vec<f32>,
}

impl Occupancy {
    fn is_taken(&self, row: usize, col: usize) -> bool {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .copied()
            .unwrap_or(false)
    }

    fn take(&mut self, row: usize, col: usize, rows: usize, cols: usize) {
        for r in row..row + rows {
            if self.rows.len() <= r {
                self.rows.resize_with(r + 1, Vec::new);
            }
            let line = &mut self.rows[r];
            if line.len() < col + cols {
                line.resize(col + cols, false);
            }
            line[col..col + cols].iter_mut().for_each(|c| *c = true);
        }
    }

    /// colspan=1 のセルのパーセント幅を列に記録する。
    fn note_percent(&mut self, col: usize, width: LengthPctAuto) {
        let LengthPctAuto::LengthPct(w) = width else {
            return;
        };
        if w.px != 0.0 || w.pct <= 0.0 {
            return;
        }
        if self.percents.len() <= col {
            self.percents.resize(col + 1, 0.0);
        }
        self.percents[col] = self.percents[col].max(w.pct);
    }

    fn column_count(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }
}

fn span_attr(builder: &Builder<'_>, cell: DomId, name: &str) -> usize {
    builder
        .doc
        .attr(cell, name)
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(1)
}

impl Builder<'_> {
    pub(crate) fn build_table(&mut self, node: DomId, style: Arc<ComputedStyle>) -> usize {
        let index = self.add_box(BoxKind::Container, Some(node), style.clone());
        let mut captions = Vec::new();
        let mut rows: Vec<DomId> = Vec::new();
        for child in self.doc.children(node).collect::<Vec<_>>() {
            let Some(child_style) = self.element_style(child) else {
                continue;
            };
            match child_style.display {
                CssDisplay::TableCaption => captions.push((child, child_style)),
                CssDisplay::TableRowGroup => rows.extend(self.row_children(child)),
                CssDisplay::TableRow => rows.push(child),
                _ => {}
            }
        }
        let caption_rows = usize::from(!captions.is_empty());
        for (caption, caption_style) in captions {
            let item = self.build_element(caption, caption_style);
            self.boxes[item].style.grid_row = Line {
                start: GridPlacement::Line(1.into()),
                end: GridPlacement::Span(1),
            };
            self.boxes[item].style.grid_column = Line {
                start: GridPlacement::Line(1.into()),
                end: GridPlacement::Line((-1).into()),
            };
            self.boxes[index].children.push(item);
        }
        let mut occupancy = Occupancy::default();
        for (r, row) in rows.into_iter().take(MAX_ROWS).enumerate() {
            self.place_row(index, row, r, caption_rows, &mut occupancy);
        }
        self.finish_table_style(index, &style, &occupancy);
        self.center_in_center_element(index, node);
        index
    }

    /// `<center>` の直下の表は、中央に寄せる（`text-align: -webkit-center` 相当）。
    fn center_in_center_element(&mut self, table: usize, node: DomId) {
        let in_center = self
            .doc
            .parent(node)
            .and_then(|p| self.doc.element(p))
            .is_some_and(|e| e.is_html("center"));
        if in_center {
            let margin = &mut self.boxes[table].style.margin;
            margin.left = LengthPercentageAuto::auto();
            margin.right = LengthPercentageAuto::auto();
        }
    }

    fn row_children(&self, group: DomId) -> Vec<DomId> {
        self.doc
            .children(group)
            .filter(|&c| {
                self.element_style(c)
                    .is_some_and(|s| s.display == CssDisplay::TableRow)
            })
            .collect()
    }

    fn place_row(
        &mut self,
        table: usize,
        row: DomId,
        r: usize,
        offset: usize,
        occ: &mut Occupancy,
    ) {
        let row_style = self.element_style(row);
        let mut col = 0;
        for cell in self.doc.children(row).collect::<Vec<_>>() {
            let Some(cell_style) = self.element_style(cell) else {
                continue;
            };
            if cell_style.display == CssDisplay::None {
                continue;
            }
            while occ.is_taken(r, col) {
                col += 1;
            }
            if col >= MAX_COLUMNS {
                break; // 上限を超える列は捨てる
            }
            let rowspan = span_attr(self, cell, "rowspan")
                .min(MAX_ROWSPAN)
                .min(MAX_ROWS - r);
            let colspan = span_attr(self, cell, "colspan").min(MAX_COLUMNS - col);
            occ.take(r, col, rowspan, colspan);
            if colspan == 1 {
                occ.note_percent(col, cell_style.width);
            }
            let cell_style = inherit_row_background(cell_style, row_style.as_deref());
            let item = self.build_element(cell, cell_style.clone());
            let taffy_style = &mut self.boxes[item].style;
            taffy_style.display = Display::FlowRoot;
            taffy_style.grid_row = Line {
                start: GridPlacement::Line(((r + offset + 1) as i16).into()),
                end: GridPlacement::Span(rowspan as u16),
            };
            taffy_style.grid_column = Line {
                start: GridPlacement::Line(((col + 1) as i16).into()),
                end: GridPlacement::Span(colspan as u16),
            };
            taffy_style.align_self = Some(match cell_style.vertical_align {
                VerticalAlign::Middle => AlignItems::CENTER,
                VerticalAlign::Bottom => AlignItems::END,
                _ => AlignItems::START,
            });
            self.boxes[table].children.push(item);
            col += colspan;
        }
    }

    fn finish_table_style(&mut self, table: usize, style: &ComputedStyle, occ: &Occupancy) {
        let columns = occ.column_count();
        let spacing = match style.border_collapse {
            BorderCollapse::Separate => style.border_spacing,
            BorderCollapse::Collapse => 0.0,
        };
        let taffy_style = &mut self.boxes[table].style;
        taffy_style.display = Display::Grid;
        let percent = (style.width == LengthPctAuto::Auto)
            .then(|| percent_columns(columns, &occ.percents))
            .flatten();
        taffy_style.grid_template_columns =
            percent.clone().unwrap_or_else(|| auto_columns(columns));
        taffy_style.gap = Size {
            width: LengthPercentage::length(spacing),
            height: LengthPercentage::length(spacing),
        };
        // 表は内容に合わせて縮む（shrink-to-fit）
        if percent.is_some() {
            // 割合の列と自動の列が混ざる表は、割合が効くよう幅いっぱいに広げる
            taffy_style.size.width = Dimension::percent(1.0);
        } else if style.width == LengthPctAuto::Auto {
            taffy_style.size.width = Dimension::fit_content();
        }
        taffy_style.justify_content = taffy::style::AlignContent::START;
        taffy_style.align_items = AlignItems::STRETCH;
    }
}

/// 行の背景色をセルに引き継ぐ（Phase 0 では行ボックスを描かないため）。
fn inherit_row_background(
    cell: Arc<ComputedStyle>,
    row: Option<&ComputedStyle>,
) -> Arc<ComputedStyle> {
    match row {
        Some(row)
            if cell.background_color.is_transparent() && !row.background_color.is_transparent() =>
        {
            let mut style = (*cell).clone();
            style.background_color = row.background_color;
            Arc::new(style)
        }
        _ => cell,
    }
}

/// パーセント指定の列と自動の列が混ざるとき、割合を `fr` にして比率を保つ。自動の列は残りの割合を
/// 等分する。割合の指定がない、または全部の列に指定があるときは `None`（全部 `auto` のまま）。
fn percent_columns(
    columns: usize,
    percents: &[f32],
) -> Option<Vec<GridTemplateComponent<Arc<str>>>> {
    let percent_of = |c: usize| percents.get(c).copied().unwrap_or(0.0);
    let specified: f32 = percents.iter().sum();
    let unspecified = (0..columns).filter(|&c| percent_of(c) <= 0.0).count();
    if specified <= 0.0 || unspecified == 0 {
        return None;
    }
    let share = ((100.0 - specified) / unspecified as f32).max(1.0);
    Some(
        (0..columns)
            .map(|c| match percent_of(c) {
                p if p > 0.0 => GridTemplateComponent::Single(fr(p)),
                _ => GridTemplateComponent::Single(fr(share)),
            })
            .collect(),
    )
}
