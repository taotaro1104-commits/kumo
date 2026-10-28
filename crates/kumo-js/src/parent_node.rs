//! `ParentNode` / `ChildNode` の JS API：セレクタ検索、子要素の取得、`append`・`before`・`remove` など。

use boa_engine::{Context, JsNativeError, JsObject, JsResult, JsValue};
use kumo_css::{parse_selector_list, query};
use kumo_dom::{Document, NodeId};

use crate::node_api::{insert, node_array, write};
use crate::wrap::{
    Native, arg_string, define_accessor, define_method, node_of, node_value, this_node,
};

use crate::element_api::{element_of, read};

// ---- 検索 ----

fn selector(args: &[JsValue], ctx: &mut Context) -> JsResult<kumo_css::SelectorList> {
    let text = arg_string(args, 0, ctx)?;
    parse_selector_list(&text).ok_or_else(|| {
        JsNativeError::syntax()
            .with_message(format!("'{text}' は有効なセレクタではありません"))
            .into()
    })
}

fn query_selector(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let list = selector(args, ctx)?;
    let found = read(ctx, |d| {
        query::query_all(d, root, &list, true).first().copied()
    });
    Ok(node_value(ctx, found))
}

fn query_selector_all(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let list = selector(args, ctx)?;
    let found = read(ctx, |d| query::query_all(d, root, &list, false));
    Ok(node_array(ctx, found))
}

pub(crate) fn matches(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let list = selector(args, ctx)?;
    Ok(JsValue::from(read(ctx, |d| query::matches(d, id, &list))))
}

pub(crate) fn closest(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let list = selector(args, ctx)?;
    let found = read(ctx, |d| query::closest(d, id, &list));
    Ok(node_value(ctx, found))
}

fn elements_where(
    ctx: &Context,
    root: NodeId,
    keep: impl Fn(&Document, NodeId) -> bool,
) -> Vec<NodeId> {
    read(ctx, |d| {
        d.descendants(root)
            .filter(|&n| n != root && d.element(n).is_some() && keep(d, n))
            .collect()
    })
}

fn get_elements_by_tag_name(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let name = arg_string(args, 0, ctx)?.to_ascii_lowercase();
    let found = elements_where(ctx, root, |d, n| {
        name == "*" || d.element(n).is_some_and(|e| *e.local == *name)
    });
    Ok(node_array(ctx, found))
}

fn get_elements_by_class_name(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let names = arg_string(args, 0, ctx)?;
    let wanted: Vec<&str> = names.split_ascii_whitespace().collect();
    let found = elements_where(ctx, root, |d, n| {
        !wanted.is_empty()
            && d.element(n)
                .is_some_and(|e| wanted.iter().all(|w| e.has_class(w)))
    });
    Ok(node_array(ctx, found))
}

// ---- 子要素 ----

fn element_children(ctx: &Context, id: NodeId) -> Vec<NodeId> {
    read(ctx, |d| {
        d.children(id).filter(|&c| d.element(c).is_some()).collect()
    })
}

fn get_children(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let children = element_children(ctx, id);
    Ok(node_array(ctx, children))
}

fn get_child_element_count(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(JsValue::from(element_children(ctx, id).len() as u32))
}

fn get_first_element_child(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let found = element_children(ctx, id).first().copied();
    Ok(node_value(ctx, found))
}

fn get_last_element_child(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let found = element_children(ctx, id).last().copied();
    Ok(node_value(ctx, found))
}

fn sibling_element(this: &JsValue, ctx: &mut Context, forward: bool) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let found = read(ctx, |d| {
        let mut current = id;
        loop {
            let next = if forward {
                d.next_sibling(current)
            } else {
                d.prev_sibling(current)
            }?;
            if d.element(next).is_some() {
                return Some(next);
            }
            current = next;
        }
    });
    Ok(node_value(ctx, found))
}

pub(crate) fn get_next_element_sibling(
    this: &JsValue,
    _: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    sibling_element(this, ctx, true)
}

pub(crate) fn get_previous_element_sibling(
    this: &JsValue,
    _: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    sibling_element(this, ctx, false)
}

/// ノードまたは文字列の引数を、挿入するノードの列にする（文字列はテキストノードにする）。
fn to_nodes(args: &[JsValue], ctx: &mut Context) -> JsResult<Vec<NodeId>> {
    let mut nodes = Vec::new();
    for arg in args {
        if let Some(id) = node_of(arg) {
            nodes.push(id);
        } else {
            let text = arg.to_string(ctx)?.to_std_string_escaped();
            nodes.push(write(ctx, |d| d.create_text(&text)));
        }
    }
    Ok(nodes)
}

fn append(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent = this_node(this)?;
    for node in to_nodes(args, ctx)? {
        insert(ctx, parent, node, None)?;
    }
    Ok(JsValue::undefined())
}

fn prepend(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent = this_node(this)?;
    let first = read(ctx, |d| d.first_child(parent));
    for node in to_nodes(args, ctx)? {
        insert(ctx, parent, node, first)?;
    }
    Ok(JsValue::undefined())
}

pub(crate) fn before(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let Some(parent) = read(ctx, |d| d.parent(id)) else {
        return Ok(JsValue::undefined());
    };
    for node in to_nodes(args, ctx)? {
        insert(ctx, parent, node, Some(id))?;
    }
    Ok(JsValue::undefined())
}

pub(crate) fn after(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let Some(parent) = read(ctx, |d| d.parent(id)) else {
        return Ok(JsValue::undefined());
    };
    let next = read(ctx, |d| d.next_sibling(id));
    for node in to_nodes(args, ctx)? {
        insert(ctx, parent, node, next)?;
    }
    Ok(JsValue::undefined())
}

pub(crate) fn remove(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    write(ctx, |d| d.detach(id));
    Ok(JsValue::undefined())
}

pub(crate) fn replace_with(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let Some(parent) = read(ctx, |d| d.parent(id)) else {
        return Ok(JsValue::undefined());
    };
    let nodes = to_nodes(args, ctx)?;
    for &node in &nodes {
        insert(ctx, parent, node, Some(id))?;
    }
    // 自分自身で置き換えるときは、そのまま残す
    if !nodes.contains(&id) {
        write(ctx, |d| d.detach(id));
    }
    Ok(JsValue::undefined())
}

/// `querySelector` など、要素・文書・フラグメントが共有する `ParentNode` の API。
pub(crate) fn install_parent_node(ctx: &mut Context, proto: &JsObject) {
    let methods: [(&str, Native, usize); 7] = [
        ("querySelector", query_selector, 1),
        ("querySelectorAll", query_selector_all, 1),
        ("getElementsByTagName", get_elements_by_tag_name, 1),
        ("getElementsByClassName", get_elements_by_class_name, 1),
        ("append", append, 0),
        ("prepend", prepend, 0),
        ("replaceChildren", replace_children, 0),
    ];
    for (name, f, n) in methods {
        define_method(ctx, proto, name, f, n);
    }
    let getters: [(&str, Native); 4] = [
        ("children", get_children),
        ("childElementCount", get_child_element_count),
        ("firstElementChild", get_first_element_child),
        ("lastElementChild", get_last_element_child),
    ];
    for (name, g) in getters {
        define_accessor(ctx, proto, name, Some(g), None);
    }
}

fn replace_children(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let nodes = to_nodes(args, ctx)?;
    write(ctx, |d| d.clear_children(id));
    for node in nodes {
        insert(ctx, id, node, None)?;
    }
    Ok(JsValue::undefined())
}
