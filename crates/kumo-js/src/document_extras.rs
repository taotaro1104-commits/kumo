//! `document` の補助的な API：`cookie`（このページの間だけ保持するメモリ上の保管庫）と `document.implementation`。

use boa_engine::{Context, JsObject, JsResult, JsValue};

use crate::host::Host;
use crate::node_api::write;
use crate::wrap::{arg_string, define_accessor, define_method, js_str, wrap_node};

// ---- cookie（このページの間だけ保持するメモリ上の保管庫） ----

fn get_cookie(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(js_str(&Host::of(ctx).state().cookie))
}

fn set_cookie(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let text = arg_string(args, 0, ctx)?;
    let mut parts = text.split(';');
    let Some((name, value)) = parts.next().and_then(|p| p.split_once('=')) else {
        return Ok(JsValue::undefined());
    };
    let (name, value) = (name.trim().to_owned(), value.trim().to_owned());
    let expired = parts.any(|attr| {
        let attr = attr.trim().to_ascii_lowercase();
        attr == "max-age=0"
            || attr.starts_with("max-age=-")
            || attr.starts_with("expires=thu, 01 jan 1970")
    });
    let host = Host::of(ctx);
    let mut state = host.state_mut();
    let mut jar: Vec<(String, String)> = state
        .cookie
        .split("; ")
        .filter_map(|p| p.split_once('='))
        .map(|(n, v)| (n.to_owned(), v.to_owned()))
        .collect();
    jar.retain(|(n, _)| *n != name);
    if !expired {
        jar.push((name, value));
    }
    state.cookie = jar
        .iter()
        .map(|(n, v)| format!("{n}={v}"))
        .collect::<Vec<_>>()
        .join("; ");
    Ok(JsValue::undefined())
}

// ---- document.implementation ----

/// `createHTMLDocument(title)`：メインの文書とは別の、空の HTML 文書（html > head > title、body）を作る。
/// 同じ DOM 領域の中に独立した根を作る（ownerDocument は共通）。
fn create_html_document(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let title = arg_string(args, 0, ctx)?;
    let root = write(ctx, |d| {
        let root = d.create_html_element(crate::node_api::DETACHED_DOCUMENT);
        let html = d.create_html_element("html");
        let head = d.create_html_element("head");
        let body = d.create_html_element("body");
        let title_element = d.create_html_element("title");
        d.append_text(title_element, &title);
        d.append(head, title_element);
        d.append(html, head);
        d.append(html, body);
        d.append(root, html);
        root
    });
    Ok(wrap_node(ctx, root).into())
}

fn has_feature(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(true))
}

fn get_implementation(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let implementation = JsObject::with_object_proto(ctx.intrinsics());
    define_method(
        ctx,
        &implementation,
        "createHTMLDocument",
        create_html_document,
        1,
    );
    define_method(ctx, &implementation, "hasFeature", has_feature, 0);
    Ok(implementation.into())
}

pub(crate) fn install(ctx: &mut Context, document: &JsObject) {
    define_accessor(
        ctx,
        document,
        "implementation",
        Some(get_implementation),
        None,
    );
    define_accessor(ctx, document, "cookie", Some(get_cookie), Some(set_cookie));
}
