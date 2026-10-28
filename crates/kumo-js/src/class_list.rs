//! `element.classList`：`class` 属性を要素のトークン列として読み書きする。

use boa_engine::{Context, JsObject, JsResult, JsValue};
use kumo_dom::{Document, NodeId};

use crate::host::Host;
use crate::node_api::write;
use crate::wrap::{Native, NodeData, define_accessor, define_method, js_str, this_node};

fn read<R>(ctx: &Context, f: impl FnOnce(&Document) -> R) -> R {
    let doc = Host::of(ctx).doc();
    let doc = doc.borrow();
    f(&doc)
}

fn classes(ctx: &Context, id: NodeId) -> Vec<String> {
    read(ctx, |d| {
        d.attr(id, "class")
            .unwrap_or_default()
            .split_ascii_whitespace()
            .map(str::to_owned)
            .collect()
    })
}

fn store_classes(ctx: &Context, id: NodeId, list: &[String]) {
    write(ctx, |d| d.set_attr(id, "class", &list.join(" ")));
}

fn class_tokens(args: &[JsValue], ctx: &mut Context) -> JsResult<Vec<String>> {
    args.iter()
        .map(|a| Ok(a.to_string(ctx)?.to_std_string_escaped()))
        .collect()
}

fn class_add(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let mut list = classes(ctx, id);
    for token in class_tokens(args, ctx)? {
        if !token.is_empty() && !list.contains(&token) {
            list.push(token);
        }
    }
    store_classes(ctx, id, &list);
    Ok(JsValue::undefined())
}

fn class_remove(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let remove = class_tokens(args, ctx)?;
    let mut list = classes(ctx, id);
    list.retain(|c| !remove.contains(c));
    store_classes(ctx, id, &list);
    Ok(JsValue::undefined())
}

fn class_toggle(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let token = crate::wrap::arg_string(args, 0, ctx)?;
    let mut list = classes(ctx, id);
    let has = list.contains(&token);
    let want = args
        .get(1)
        .filter(|v| !v.is_undefined())
        .map_or(!has, JsValue::to_boolean);
    if want && !has {
        list.push(token);
    } else if !want && has {
        list.retain(|c| *c != token);
    }
    store_classes(ctx, id, &list);
    Ok(JsValue::from(want))
}

fn class_contains(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let token = crate::wrap::arg_string(args, 0, ctx)?;
    Ok(JsValue::from(classes(ctx, id).contains(&token)))
}

fn class_replace(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let (old, new) = (
        crate::wrap::arg_string(args, 0, ctx)?,
        crate::wrap::arg_string(args, 1, ctx)?,
    );
    let mut list = classes(ctx, id);
    let Some(at) = list.iter().position(|c| *c == old) else {
        return Ok(JsValue::from(false));
    };
    list[at] = new;
    store_classes(ctx, id, &list);
    Ok(JsValue::from(true))
}

fn class_item(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let n = args.first().map_or(Ok(0.0), |v| v.to_number(ctx))? as usize;
    Ok(classes(ctx, id)
        .get(n)
        .map_or(JsValue::null(), |c| js_str(c)))
}

fn class_length(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(JsValue::from(classes(ctx, id).len() as u32))
}

fn class_value(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(js_str(&classes(ctx, id).join(" ")))
}

pub(crate) fn get_class_list(
    this: &JsValue,
    _: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let proto = Host::of(ctx).proto("class_list");
    Ok(JsObject::from_proto_and_data(Some(proto), NodeData { id })
        .upcast()
        .into())
}

/// `classList` のプロトタイプを作って登録する。
pub(crate) fn install_prototype(ctx: &mut Context) {
    let host = Host::of(ctx);
    let list = JsObject::with_object_proto(ctx.intrinsics());
    let methods: [(&str, Native, usize); 6] = [
        ("add", class_add, 0),
        ("remove", class_remove, 0),
        ("toggle", class_toggle, 1),
        ("contains", class_contains, 1),
        ("replace", class_replace, 2),
        ("item", class_item, 1),
    ];
    for (name, f, n) in methods {
        define_method(ctx, &list, name, f, n);
    }
    define_method(ctx, &list, "toString", class_value, 0);
    define_accessor(ctx, &list, "length", Some(class_length), None);
    define_accessor(ctx, &list, "value", Some(class_value), None);
    host.state_mut().protos.insert("class_list", list);
}
