//! グリッドの配置ショートハンド（`grid-column` / `grid-row` / `grid-area`）。

use std::sync::Arc;

use cssparser::Parser;

use crate::decl::{Decl, GridProp};
use crate::properties::rest_of_input;
use crate::values::ParseResult;

fn grid_value(s: &str) -> Option<Arc<str>> {
    let trimmed = s.trim();
    (!trimmed.is_empty()).then(|| Arc::from(trimmed))
}

/// `auto` / `span` / 数字ではない、名前だけの値か。
fn is_custom_ident(value: &str) -> bool {
    let value = value.trim();
    value.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '-')
        && !value.contains(char::is_whitespace)
        && !value.eq_ignore_ascii_case("auto")
        && !value.eq_ignore_ascii_case("span")
}

/// `grid-column: 1 / 3` を start / end に分ける。
pub(crate) fn parse_grid_line_pair<'i>(
    input: &mut Parser<'i>,
    out: &mut Vec<Decl>,
    start: GridProp,
    end: GridProp,
) -> ParseResult<()> {
    let raw = rest_of_input(input);
    let mut parts = raw.splitn(2, '/');
    let first = parts.next().and_then(grid_value);
    let mut second = parts.next().and_then(grid_value);
    // `grid-column: content` のように名前だけを書いたときは、終わりも同じ名前（`content-start` から `content-end`）
    if second.is_none()
        && let Some(name) = &first
        && is_custom_ident(name)
    {
        second = Some(name.clone());
    }
    out.push(Decl::Grid(start, first.clone()));
    out.push(Decl::Grid(end, second));
    Ok(())
}

/// `grid-area: name` または `row-start / column-start / row-end / column-end`。
pub(crate) fn parse_grid_area<'i>(input: &mut Parser<'i>, out: &mut Vec<Decl>) -> ParseResult<()> {
    let raw = rest_of_input(input);
    let parts: Vec<Option<Arc<str>>> = raw.split('/').map(grid_value).collect();
    let first = parts.first().cloned().flatten();
    // 省略された値は対になる start 側（さらに無ければ先頭）の名前を使う
    let get = |i: usize| {
        parts
            .get(i)
            .cloned()
            .flatten()
            .or_else(|| {
                i.checked_sub(2)
                    .and_then(|j| parts.get(j).cloned().flatten())
            })
            .or_else(|| first.clone())
    };
    out.extend([
        Decl::Grid(GridProp::RowStart, get(0)),
        Decl::Grid(GridProp::ColumnStart, get(1)),
        Decl::Grid(GridProp::RowEnd, get(2)),
        Decl::Grid(GridProp::ColumnEnd, get(3)),
    ]);
    Ok(())
}
