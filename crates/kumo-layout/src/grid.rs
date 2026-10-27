//! グリッド関連の生文字列を Taffy の型へ変換する。

use std::sync::Arc;

use taffy::prelude::GridPlacement;
use taffy::style::{
    GridTemplateArea, GridTemplateAreas, GridTemplateComponent, GridTemplateTracks,
};

type Tracks = Vec<GridTemplateComponent<Arc<str>>>;
type LineNames = Vec<Vec<Arc<str>>>;

/// `grid-template-columns` 等を解析する（`none` や解析不能なら `None`）。
pub fn parse_tracks(css: &str) -> Option<(Tracks, LineNames)> {
    if css.trim().eq_ignore_ascii_case("none") {
        return None;
    }
    let parsed: GridTemplateTracks<Arc<str>, GridTemplateComponent<Arc<str>>> =
        normalize_zero(css).parse().ok()?;
    Some((parsed.tracks, parsed.line_names))
}

/// Taffy のパーサは単位なしの `0` を受け付けないため `0px` に置き換える。
fn normalize_zero(css: &str) -> String {
    let bytes = css.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'%');
    let mut out = String::with_capacity(css.len() + 8);
    for (i, c) in css.char_indices() {
        out.push(c);
        let standalone = c == '0'
            && (i == 0 || !is_word(bytes[i - 1]))
            && bytes.get(i + 1).is_none_or(|&b| !is_word(b));
        if standalone {
            out.push_str("px");
        }
    }
    out
}

/// `grid-column-start` 等の1値。`span 2` / `3` / `name` / `auto`。
/// 名前だけの指定は、領域名なら start は `name-start`、end は `name-end` として扱う。
pub fn parse_placement(css: &str, is_start: bool) -> GridPlacement<Arc<str>> {
    let css = css.trim();
    let is_ident = css
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '-')
        && !css.contains(char::is_whitespace)
        && !css.eq_ignore_ascii_case("auto");
    if is_ident {
        let suffix = if is_start { "-start" } else { "-end" };
        return GridPlacement::NamedLine(Arc::from(format!("{css}{suffix}")), 1);
    }
    css.parse().unwrap_or(GridPlacement::Auto)
}

/// `grid-template-areas: "a a" "b c"` を長方形の領域リストに変換する。
pub fn parse_areas(css: &str) -> Option<GridTemplateAreas<Arc<str>>> {
    // 文字列は "…" と '…' のどちらでもよい
    let rows: Vec<Vec<&str>> = css
        .split(['"', '\''])
        .skip(1)
        .step_by(2)
        .map(|row| row.split_whitespace().collect())
        .collect();
    let column_count = rows.first()?.len();
    if column_count == 0 || rows.iter().any(|r| r.len() != column_count) {
        return None;
    }
    let mut areas: Vec<GridTemplateArea<Arc<str>>> = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, name) in row.iter().enumerate() {
            if name.starts_with('.') {
                continue;
            }
            let (r, c) = (r as u16 + 1, c as u16 + 1);
            match areas.iter_mut().find(|a| &*a.name == *name) {
                Some(area) => {
                    area.row_end = area.row_end.max(r + 1);
                    area.column_end = area.column_end.max(c + 1);
                }
                None => areas.push(GridTemplateArea {
                    name: Arc::from(*name),
                    row_start: r,
                    row_end: r + 1,
                    column_start: c,
                    column_end: c + 1,
                }),
            }
        }
    }
    Some(GridTemplateAreas {
        areas: areas.into_iter().collect(),
        row_count: rows.len() as u16,
        column_count: column_count as u16,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_and_names() {
        let (tracks, names) = parse_tracks("[a] 100px 1fr [b] minmax(0, 2fr)").unwrap();
        assert_eq!(tracks.len(), 3);
        assert_eq!(names[0].len(), 1);
        assert!(parse_tracks("none").is_none());
        assert!(parse_tracks("repeat(3, auto)").is_some());
    }

    #[test]
    fn unitless_zero_is_normalized() {
        assert_eq!(
            normalize_zero("minmax(0, 1fr) 10px 0.5fr"),
            "minmax(0px, 1fr) 10px 0.5fr"
        );
        assert_eq!(normalize_zero("0 20"), "0px 20");
    }

    #[test]
    fn areas_become_rectangles() {
        let areas = parse_areas(r#""head head" 'nav main' ". main""#).unwrap();
        assert_eq!((areas.row_count, areas.column_count), (3, 2));
        let main = areas.areas.iter().find(|a| &*a.name == "main").unwrap();
        assert_eq!(
            (
                main.row_start,
                main.row_end,
                main.column_start,
                main.column_end
            ),
            (2, 4, 2, 3)
        );
        assert!(parse_areas(r#""a b" "c""#).is_none());
    }

    #[test]
    fn placements() {
        assert_eq!(parse_placement("2", true), GridPlacement::Line(2.into()));
        assert_eq!(parse_placement("span 3", false), GridPlacement::Span(3));
        assert_eq!(parse_placement("auto", true), GridPlacement::Auto);
        assert_eq!(
            parse_placement("main", false),
            GridPlacement::NamedLine(Arc::from("main-end"), 1)
        );
    }
}
