//! `Node` / `CharacterData` の JS API：木の構造、テキスト、挿入・削除・複製。

use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsObject, JsResult, JsValue};
use kumo_dom::{Document, NodeId, NodeKind};

use crate::host::Host;
use crate::wrap::{
    Native, arg_node, arg_string, define_accessor, define_method, js_str, node_value, this_node,
    type_error, wrap_node,
};

pub(crate) const FRAGMENT: &str = "#document-fragment";
/// `document.implementation.createHTMLDocument()` が作る、メインの文書とは別の文書の根。
pub(crate) const DETACHED_DOCUMENT: &str = "#document";

pub(crate) fn is_fragment(doc: &Document, id: NodeId) -> bool {
    doc.element(id).is_some_and(|e| &*e.local == FRAGMENT)
}

/// DOM の `nodeType`。
fn node_type(doc: &Document, id: NodeId) -> i32 {
    match doc.node(id).kind() {
        NodeKind::Element(_) if is_fragment(doc, id) => 11,
        NodeKind::Element(_) if is_detached_document(doc, id) => 9,
        NodeKind::Element(_) => 1,
        NodeKind::Text(_) => 3,
        NodeKind::Comment => 8,
        NodeKind::Document => 9,
        NodeKind::Doctype => 10,
    }
}

pub(crate) fn is_detached_document(doc: &Document, id: NodeId) -> bool {
    doc.element(id)
        .is_some_and(|e| &*e.local == DETACHED_DOCUMENT)
}

pub(crate) fn node_name(doc: &Document, id: NodeId) -> String {
    match doc.node(id).kind() {
        NodeKind::Element(_) => doc.element(id).map_or_else(String::new, |e| {
            if &*e.local == FRAGMENT || &*e.local == DETACHED_DOCUMENT {
                format!("#{}", e.local.trim_start_matches('#'))
            } else if e.ns == kumo_dom::ns!(html) {
                e.local.to_ascii_uppercase().to_string()
            } else {
                e.local.to_string()
            }
        }),
        NodeKind::Text(_) => "#text".to_owned(),
        NodeKind::Comment => "#comment".to_owned(),
        NodeKind::Document => "#document".to_owned(),
        NodeKind::Doctype => "html".to_owned(),
    }
}

fn read<R>(ctx: &Context, f: impl FnOnce(&Document) -> R) -> R {
    let doc = Host::of(ctx).doc();
    let doc = doc.borrow();
    f(&doc)
}

pub(crate) fn write<R>(ctx: &Context, f: impl FnOnce(&mut Document) -> R) -> R {
    let host = Host::of(ctx);
    host.mark_dirty();
    let doc = host.doc();
    let mut doc = doc.borrow_mut();
    f(&mut doc)
}

/// ノードの配列（`childNodes` などは生きたリストではなく、その時点の写し）。
pub(crate) fn node_array(ctx: &mut Context, ids: Vec<NodeId>) -> JsValue {
    let values: Vec<JsValue> = ids
        .into_iter()
        .map(|id| wrap_node(ctx, id).into())
        .collect();
    JsArray::from_iter(values, ctx).into()
}

// ---- 読み取り ----

fn get_node_type(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(JsValue::from(read(ctx, |d| node_type(d, id))))
}

fn get_node_name(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(js_str(&read(ctx, |d| node_name(d, id))))
}

macro_rules! relation {
    ($name:ident, $f:expr) => {
        fn $name(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
            let id = this_node(this)?;
            let found = read(ctx, |d| $f(d, id));
            Ok(node_value(ctx, found))
        }
    };
}

relation!(get_parent_node, |d: &Document, id| d.parent(id));
relation!(get_parent_element, |d: &Document, id| d
    .parent(id)
    .filter(|&p| d.element(p).is_some()));
relation!(get_first_child, |d: &Document, id| d.first_child(id));
relation!(get_last_child, |d: &Document, id| d.last_child(id));
relation!(get_next_sibling, |d: &Document, id| d.next_sibling(id));
relation!(get_prev_sibling, |d: &Document, id| d.prev_sibling(id));

fn get_child_nodes(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let children: Vec<NodeId> = read(ctx, |d| d.children(id).collect());
    Ok(node_array(ctx, children))
}

fn get_owner_document(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    if read(ctx, |d| id == d.root()) {
        return Ok(JsValue::null());
    }
    let root = read(ctx, Document::root);
    Ok(wrap_node(ctx, root).into())
}

fn get_is_connected(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(JsValue::from(read(ctx, |d| is_connected(d, id))))
}

pub(crate) fn is_connected(doc: &Document, id: NodeId) -> bool {
    doc.is_inclusive_descendant(id, doc.root())
}

fn has_child_nodes(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(JsValue::from(read(ctx, |d| d.first_child(id).is_some())))
}

fn contains(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let Some(other) = args.first().and_then(crate::wrap::node_of) else {
        return Ok(JsValue::from(false));
    };
    Ok(JsValue::from(read(ctx, |d| {
        d.is_inclusive_descendant(other, id)
    })))
}

// ---- テキスト ----

fn get_text_content(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let (is_document, text) = read(ctx, |d| {
        let text = match d.node(id).kind() {
            NodeKind::Text(_) => d.text(id).unwrap_or_default().to_owned(),
            _ => d.text_content(id),
        };
        (id == d.root(), text)
    });
    Ok(if is_document {
        JsValue::null()
    } else {
        js_str(&text)
    })
}

fn set_text_content(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let text = match args.first() {
        Some(v) if v.is_null_or_undefined() => String::new(),
        _ => arg_string(args, 0, ctx)?,
    };
    write(ctx, |d| match d.node(id).kind() {
        NodeKind::Text(_) => {
            d.set_text_data(id, &text);
        }
        NodeKind::Element(_) => {
            d.clear_children(id);
            if !text.is_empty() {
                d.append_text(id, &text);
            }
        }
        _ => {}
    });
    Ok(JsValue::undefined())
}

fn get_length(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let len = read(ctx, |d| d.text(id).map_or(0, |t| t.encode_utf16().count()));
    Ok(JsValue::from(len as u32))
}

// ---- 変更 ----

/// `child` を `parent` の `reference` の前（なければ末尾）へ入れる。フラグメントは子を移す。
pub(crate) fn insert(
    ctx: &mut Context,
    parent: NodeId,
    child: NodeId,
    reference: Option<NodeId>,
) -> JsResult<()> {
    let host = Host::of(ctx);
    let inserted: Vec<NodeId> = {
        let doc = host.doc();
        let mut doc = doc.borrow_mut();
        if doc.is_inclusive_descendant(parent, child) {
            return Err(type_error(
                "HierarchyRequestError: ノードを自分の子孫には入れられません",
            ));
        }
        if reference.is_some_and(|r| doc.parent(r) != Some(parent)) {
            return Err(type_error(
                "NotFoundError: 基準のノードがこのノードの子ではありません",
            ));
        }
        let moving: Vec<NodeId> = if is_fragment(&doc, child) {
            doc.children(child).collect()
        } else {
            vec![child]
        };
        for &node in &moving {
            match reference {
                Some(r) if r == node => {} // すでにその位置にある
                Some(r) => doc.insert_before(r, node),
                None => doc.append(parent, node),
            }
        }
        let connected = is_connected(&doc, parent);
        moving
            .iter()
            .flat_map(|&n| doc.descendants(n).collect::<Vec<_>>())
            .filter(|&n| connected && doc.element(n).is_some_and(|e| e.is_html("script")))
            .collect()
    };
    let mut state = host.state_mut();
    state.dirty = true;
    state.new_scripts.extend(inserted);
    Ok(())
}

fn append_child(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent = this_node(this)?;
    let child = arg_node(args, 0)?;
    insert(ctx, parent, child, None)?;
    Ok(args[0].clone())
}

fn insert_before_method(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent = this_node(this)?;
    let child = arg_node(args, 0)?;
    let reference = args.get(1).and_then(crate::wrap::node_of);
    insert(ctx, parent, child, reference)?;
    Ok(args[0].clone())
}

fn remove_child(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent = this_node(this)?;
    let child = arg_node(args, 0)?;
    if read(ctx, |d| d.parent(child)) != Some(parent) {
        return Err(type_error(
            "NotFoundError: ノードはこのノードの子ではありません",
        ));
    }
    write(ctx, |d| d.detach(child));
    Ok(args[0].clone())
}

fn replace_child(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let parent = this_node(this)?;
    let new_child = arg_node(args, 0)?;
    let old = arg_node(args, 1)?;
    if read(ctx, |d| d.parent(old)) != Some(parent) {
        return Err(type_error(
            "NotFoundError: ノードはこのノードの子ではありません",
        ));
    }
    if new_child != old {
        insert(ctx, parent, new_child, Some(old))?;
        write(ctx, |d| d.detach(old));
    }
    Ok(args[1].clone())
}

fn clone_node(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let deep = args.first().is_some_and(JsValue::to_boolean);
    let copy = write(ctx, |d| d.clone_node(id, deep));
    copy.map(|c| wrap_node(ctx, c).into())
        .ok_or_else(|| type_error("このノードは複製できません"))
}

fn normalize(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::undefined())
}

pub(crate) fn install(ctx: &mut Context, proto: &JsObject) {
    let getters: [(&str, Native); 12] = [
        ("nodeType", get_node_type),
        ("nodeName", get_node_name),
        ("parentNode", get_parent_node),
        ("parentElement", get_parent_element),
        ("firstChild", get_first_child),
        ("lastChild", get_last_child),
        ("nextSibling", get_next_sibling),
        ("previousSibling", get_prev_sibling),
        ("childNodes", get_child_nodes),
        ("ownerDocument", get_owner_document),
        ("isConnected", get_is_connected),
        ("length", get_length),
    ];
    for (name, getter) in getters {
        define_accessor(ctx, proto, name, Some(getter), None);
    }
    define_accessor(
        ctx,
        proto,
        "textContent",
        Some(get_text_content),
        Some(set_text_content),
    );
    let methods: [(&str, Native, usize); 9] = [
        ("hasChildNodes", has_child_nodes, 0),
        ("contains", contains, 1),
        ("appendChild", append_child, 1),
        ("insertBefore", insert_before_method, 2),
        ("removeChild", remove_child, 1),
        ("replaceChild", replace_child, 2),
        ("cloneNode", clone_node, 0),
        ("normalize", normalize, 0),
        ("getRootNode", get_owner_document, 0),
    ];
    for (name, method, length) in methods {
        define_method(ctx, proto, name, method, length);
    }
}

/// テキストノード（`CharacterData`）のプロパティ。
pub(crate) fn install_text(ctx: &mut Context, proto: &JsObject) {
    define_accessor(
        ctx,
        proto,
        "data",
        Some(get_text_content),
        Some(set_text_content),
    );
    define_accessor(
        ctx,
        proto,
        "nodeValue",
        Some(get_text_content),
        Some(set_text_content),
    );
}
