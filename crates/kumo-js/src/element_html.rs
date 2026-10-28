//! 要素の HTML（`innerHTML` など）とフォーム部品の `value`。

use boa_engine::{Context, JsResult, JsValue};
use kumo_dom::{Document, NodeId};

use crate::node_api::write;
use crate::wrap::{arg_string, js_str};

use crate::element_api::{element_of, read};

// ---- HTML ----

/// HTML 断片を解析して、`doc` に取り込んだノード列を返す（スクリプトは実行されない）。
pub(crate) fn parse_fragment(doc: &mut Document, html: &str) -> Vec<NodeId> {
    let parsed = kumo_html::parse_html(&format!("<!DOCTYPE html><html><body>{html}</body></html>"));
    let Some(body) = parsed
        .descendants(parsed.root())
        .find(|&n| parsed.element(n).is_some_and(|e| e.is_html("body")))
    else {
        return Vec::new();
    };
    parsed
        .children(body)
        .filter_map(|child| doc.import_subtree(&parsed, child))
        .collect()
}

pub(crate) fn get_inner_html(
    this: &JsValue,
    _: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    Ok(js_str(&read(ctx, |d| d.inner_html(id))))
}

pub(crate) fn set_inner_html(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let html = arg_string(args, 0, ctx)?;
    write(ctx, |d| {
        d.clear_children(id);
        for node in parse_fragment(d, &html) {
            d.append(id, node);
        }
    });
    Ok(JsValue::undefined())
}

pub(crate) fn get_outer_html(
    this: &JsValue,
    _: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    Ok(js_str(&read(ctx, |d| d.outer_html(id))))
}

pub(crate) fn insert_adjacent_html(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let position = arg_string(args, 0, ctx)?.to_ascii_lowercase();
    let html = arg_string(args, 1, ctx)?;
    write(ctx, |d| {
        let nodes = parse_fragment(d, &html);
        let parent = d.parent(id);
        match position.as_str() {
            "beforebegin" if parent.is_some() => {
                nodes.into_iter().for_each(|n| d.insert_before(id, n))
            }
            "afterbegin" => {
                let first = d.first_child(id);
                for n in nodes {
                    match first {
                        Some(f) => d.insert_before(f, n),
                        None => d.append(id, n),
                    }
                }
            }
            "beforeend" => nodes.into_iter().for_each(|n| d.append(id, n)),
            "afterend" if parent.is_some() => {
                let next = d.next_sibling(id);
                for n in nodes {
                    match next {
                        Some(s) => d.insert_before(s, n),
                        None => d.append(parent.expect("親がある"), n),
                    }
                }
            }
            _ => {}
        }
    });
    Ok(JsValue::undefined())
}

// ---- 値 ----

pub(crate) fn get_value(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let value = read(ctx, |d| {
        if d.element(id).is_some_and(|e| e.is_html("select")) {
            return select_value(d, id);
        }
        match d.attr(id, "value") {
            Some(v) => v.to_owned(),
            None if d.element(id).is_some_and(|e| e.is_html("textarea")) => d.text_content(id),
            None => String::new(),
        }
    });
    Ok(js_str(&value))
}

fn option_value(d: &Document, option: NodeId) -> String {
    d.attr(option, "value")
        .map_or_else(|| d.text_content(option).trim().to_owned(), str::to_owned)
}

fn select_value(d: &Document, select: NodeId) -> String {
    let options: Vec<NodeId> = d
        .descendants(select)
        .filter(|&n| d.element(n).is_some_and(|e| e.is_html("option")))
        .collect();
    options
        .iter()
        .find(|&&o| d.attr(o, "selected").is_some())
        .or(options.first())
        .map_or_else(String::new, |&o| option_value(d, o))
}

pub(crate) fn set_value(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let value = arg_string(args, 0, ctx)?;
    write(ctx, |d| {
        if d.element(id).is_some_and(|e| e.is_html("select")) {
            let options: Vec<NodeId> = d
                .descendants(id)
                .filter(|&n| d.element(n).is_some_and(|e| e.is_html("option")))
                .collect();
            for &o in &options {
                d.remove_attr(o, "selected");
            }
            if let Some(&chosen) = options.iter().find(|&&o| option_value(d, o) == value) {
                d.set_attr(chosen, "selected", "");
            }
        } else {
            d.set_attr(id, "value", &value);
        }
    });
    Ok(JsValue::undefined())
}
