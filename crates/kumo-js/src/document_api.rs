//! `document` の JS API：要素の生成と検索、`body`・`title`・`cookie`・`readyState` など。

use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsObject, JsResult, JsString, JsValue};
use kumo_dom::{Document, LocalName, Namespace, NodeId, QualName};

use crate::element_html::parse_fragment;
use crate::host::Host;
use crate::node_api::{FRAGMENT, insert, node_array, write};
use crate::wrap::{
    Native, arg_string, define_accessor, define_method, js_str, node_value, this_node,
    window_object, wrap_node,
};

fn read<R>(ctx: &Context, f: impl FnOnce(&Document) -> R) -> R {
    let doc = Host::of(ctx).doc();
    let doc = doc.borrow();
    f(&doc)
}

fn create_element(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let name = arg_string(args, 0, ctx)?;
    let id = write(ctx, |d| d.create_html_element(&name));
    Ok(wrap_node(ctx, id).into())
}

fn create_element_ns(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let namespace = arg_string(args, 0, ctx)?;
    let name = arg_string(args, 1, ctx)?;
    let id = write(ctx, |d| {
        if namespace == "http://www.w3.org/1999/xhtml" {
            d.create_html_element(&name)
        } else {
            let qualified = QualName::new(None, Namespace::from(namespace), LocalName::from(name));
            d.create_element(qualified, Vec::new())
        }
    });
    Ok(wrap_node(ctx, id).into())
}

fn create_text_node(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let text = arg_string(args, 0, ctx)?;
    let id = write(ctx, |d| d.create_text(&text));
    Ok(wrap_node(ctx, id).into())
}

fn create_comment(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = write(ctx, Document::create_comment);
    Ok(wrap_node(ctx, id).into())
}

fn create_fragment(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = write(ctx, |d| d.create_html_element(FRAGMENT));
    Ok(wrap_node(ctx, id).into())
}

fn get_element_by_id(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let wanted = arg_string(args, 0, ctx)?;
    let found = read(ctx, |d| {
        d.descendants(root)
            .find(|&n| d.element(n).and_then(|e| e.id()) == Some(wanted.as_str()))
    });
    Ok(node_value(ctx, found))
}

fn get_elements_by_name(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let wanted = arg_string(args, 0, ctx)?;
    let found: Vec<NodeId> = read(ctx, |d| {
        d.descendants(root)
            .filter(|&n| d.attr(n, "name") == Some(wanted.as_str()))
            .collect()
    });
    Ok(node_array(ctx, found))
}

/// `root` の子孫で最初の `<local>` 要素。
fn first_element(ctx: &Context, root: NodeId, local: &str) -> Option<NodeId> {
    read(ctx, |d| {
        d.descendants(root)
            .find(|&n| d.element(n).is_some_and(|e| e.is_html(local)))
    })
}

fn get_document_element(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let found = first_element(ctx, this_node(this)?, "html");
    Ok(node_value(ctx, found))
}

fn get_body(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let found = first_element(ctx, this_node(this)?, "body");
    Ok(node_value(ctx, found))
}

fn get_head(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let found = first_element(ctx, this_node(this)?, "head");
    Ok(node_value(ctx, found))
}

fn get_title(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let title =
        first_element(ctx, this_node(this)?, "title").map(|t| read(ctx, |d| d.text_content(t)));
    let text = title
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Ok(js_str(&text))
}

fn set_title(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let text = arg_string(args, 0, ctx)?;
    let head = first_element(ctx, root, "head");
    write(ctx, |d| {
        let title = first_element_in(d, root, "title").unwrap_or_else(|| {
            let t = d.create_html_element("title");
            if let Some(h) = head {
                d.append(h, t);
            }
            t
        });
        d.clear_children(title);
        d.append_text(title, &text);
    });
    Ok(JsValue::undefined())
}

fn first_element_in(d: &Document, root: NodeId, local: &str) -> Option<NodeId> {
    d.descendants(root)
        .find(|&n| d.element(n).is_some_and(|e| e.is_html(local)))
}

fn collection(ctx: &mut Context, root: NodeId, local: &str) -> JsResult<JsValue> {
    let found: Vec<NodeId> = read(ctx, |d| {
        d.descendants(root)
            .filter(|&n| d.element(n).is_some_and(|e| e.is_html(local)))
            .collect()
    });
    Ok(node_array(ctx, found))
}

fn get_forms(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    collection(ctx, this_node(this)?, "form")
}

fn get_images(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    collection(ctx, this_node(this)?, "img")
}

fn get_scripts(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    collection(ctx, this_node(this)?, "script")
}

fn get_links(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let found: Vec<NodeId> = read(ctx, |d| {
        d.descendants(root)
            .filter(|&n| {
                d.element(n).is_some_and(|e| e.is_html("a")) && d.attr(n, "href").is_some()
            })
            .collect()
    });
    Ok(node_array(ctx, found))
}

// ---- 状態 ----

fn get_ready_state(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(js_str(&Host::of(ctx).state().ready_state))
}

fn get_url(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(js_str(Host::of(ctx).state().url.as_str()))
}

fn get_domain(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(js_str(
        Host::of(ctx).state().url.host_str().unwrap_or_default(),
    ))
}

fn get_location(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    ctx.global_object().get(JsString::from("location"), ctx)
}

fn set_location(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let location = ctx.global_object().get(JsString::from("location"), ctx)?;
    if let (Some(object), Some(value)) = (location.as_object(), args.first()) {
        object.set(JsString::from("href"), value.clone(), false, ctx)?;
    }
    Ok(JsValue::undefined())
}

fn get_default_view(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(window_object(ctx).into())
}

fn empty_string(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(js_str(""))
}

fn utf8(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(js_str("UTF-8"))
}

fn html_type(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(js_str("text/html"))
}

fn css1_compat(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(js_str("CSS1Compat"))
}

fn visible(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(js_str("visible"))
}

fn not_hidden(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(false))
}

fn null(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::null())
}

fn yes(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(true))
}

fn noop(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::undefined())
}

fn get_active_element(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    get_body(this, &[], ctx)
}

// ---- 書き込み（読み込み中の `document.write` は本文の末尾へ追加する簡易版） ----

fn write_html(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let root = this_node(this)?;
    let mut html = String::new();
    for arg in args {
        html.push_str(&arg.to_string(ctx)?.to_std_string_escaped());
    }
    let Some(body) = first_element(ctx, root, "body") else {
        return Ok(JsValue::undefined());
    };
    let nodes = write(ctx, |d| parse_fragment(d, &html));
    for node in nodes {
        insert(ctx, body, node, None)?;
    }
    Ok(JsValue::undefined())
}

fn current_script(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let found = Host::of(ctx).state().current_script;
    Ok(node_value(ctx, found))
}

pub(crate) fn install(ctx: &mut Context, document: &JsObject) {
    crate::document_extras::install(ctx, document);
    crate::parent_node::install_parent_node(ctx, document);
    let methods: [(&str, Native, usize); 13] = [
        ("createElement", create_element, 1),
        ("createElementNS", create_element_ns, 2),
        ("createTextNode", create_text_node, 1),
        ("createComment", create_comment, 1),
        ("createDocumentFragment", create_fragment, 0),
        ("getElementById", get_element_by_id, 1),
        ("getElementsByName", get_elements_by_name, 1),
        ("createEvent", crate::events::create_event_native(), 1),
        ("hasFocus", yes, 0),
        ("write", write_html, 1),
        ("writeln", write_html, 1),
        ("open", noop, 0),
        ("close", noop, 0),
    ];
    for (name, f, n) in methods {
        define_method(ctx, document, name, f, n);
    }
    let getters: [(&str, Native); 19] = [
        ("documentElement", get_document_element),
        ("body", get_body),
        ("head", get_head),
        ("forms", get_forms),
        ("images", get_images),
        ("scripts", get_scripts),
        ("links", get_links),
        ("readyState", get_ready_state),
        ("URL", get_url),
        ("documentURI", get_url),
        ("domain", get_domain),
        ("referrer", empty_string),
        ("characterSet", utf8),
        ("charset", utf8),
        ("contentType", html_type),
        ("compatMode", css1_compat),
        ("visibilityState", visible),
        ("defaultView", get_default_view),
        ("activeElement", get_active_element),
    ];
    for (name, g) in getters {
        define_accessor(ctx, document, name, Some(g), None);
    }
    define_accessor(ctx, document, "hidden", Some(not_hidden), None);
    define_accessor(ctx, document, "doctype", Some(null), None);
    define_accessor(ctx, document, "currentScript", Some(current_script), None);
    define_accessor(ctx, document, "title", Some(get_title), Some(set_title));
    define_accessor(
        ctx,
        document,
        "location",
        Some(get_location),
        Some(set_location),
    );
    let _ = JsArray::new;
}
