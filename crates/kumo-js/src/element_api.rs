//! `Element` / `HTMLElement` の JS API：属性、検索、HTML の読み書き、子要素の挿入と削除。

use boa_engine::{Context, JsObject, JsResult, JsString, JsValue};
use kumo_dom::{Document, NodeId};

use crate::element_html::{
    get_inner_html, get_outer_html, get_value, insert_adjacent_html, set_inner_html, set_value,
};
use crate::host::Host;
use crate::node_api::{node_name, write};
use crate::parent_node::{
    after, before, closest, get_next_element_sibling, get_previous_element_sibling,
    install_parent_node, matches, remove, replace_with,
};
use crate::wrap::{
    Native, arg_node, arg_string, define_accessor, define_method, define_named_accessor, js_str,
    this_node, type_error,
};

pub(crate) fn read<R>(ctx: &Context, f: impl FnOnce(&Document) -> R) -> R {
    let doc = Host::of(ctx).doc();
    let doc = doc.borrow();
    f(&doc)
}

pub(crate) fn element_of(this: &JsValue, ctx: &Context) -> JsResult<NodeId> {
    let id = this_node(this)?;
    if read(ctx, |d| d.element(id).is_none()) {
        return Err(type_error("Illegal invocation"));
    }
    Ok(id)
}

// ---- 属性 ----

fn attr_name(args: &[JsValue], ctx: &mut Context) -> JsResult<String> {
    Ok(arg_string(args, 0, ctx)?.to_ascii_lowercase())
}

fn get_attribute(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr_name(args, ctx)?;
    Ok(read(ctx, |d| d.attr(id, &name).map(str::to_owned)).map_or(JsValue::null(), |v| js_str(&v)))
}

fn set_attribute(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr_name(args, ctx)?;
    let value = arg_string(args, 1, ctx)?;
    write(ctx, |d| d.set_attr(id, &name, &value));
    Ok(JsValue::undefined())
}

fn remove_attribute(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr_name(args, ctx)?;
    write(ctx, |d| d.remove_attr(id, &name));
    Ok(JsValue::undefined())
}

fn has_attribute(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr_name(args, ctx)?;
    Ok(JsValue::from(read(ctx, |d| d.attr(id, &name).is_some())))
}

fn toggle_attribute(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr_name(args, ctx)?;
    let has = read(ctx, |d| d.attr(id, &name).is_some());
    let want = args.get(1).map_or(!has, JsValue::to_boolean);
    if want != has {
        write(ctx, |d| {
            if want {
                d.set_attr(id, &name, "");
            } else {
                d.remove_attr(id, &name);
            }
        });
    }
    Ok(JsValue::from(want))
}

fn get_attribute_names(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let names: Vec<JsValue> = read(ctx, |d| {
        d.element(id)
            .map(|e| e.attrs.iter().map(|a| js_str(&a.name)).collect())
            .unwrap_or_default()
    });
    Ok(boa_engine::object::builtins::JsArray::from_iter(names, ctx).into())
}

/// 文字列属性の getter（属性名を捕捉）。
fn reflect_get(
    this: &JsValue,
    _: &[JsValue],
    attr: &JsString,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr.to_std_string_escaped();
    Ok(js_str(&read(ctx, |d| {
        d.attr(id, &name).unwrap_or_default().to_owned()
    })))
}

fn reflect_set(
    this: &JsValue,
    args: &[JsValue],
    attr: &JsString,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let value = arg_string(args, 0, ctx)?;
    let name = attr.to_std_string_escaped();
    write(ctx, |d| d.set_attr(id, &name, &value));
    Ok(JsValue::undefined())
}

/// URL を持つ属性の getter：文書の URL を基準にした絶対 URL を返す。
fn url_get(this: &JsValue, _: &[JsValue], attr: &JsString, ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr.to_std_string_escaped();
    let value = read(ctx, |d| d.attr(id, &name).map(str::to_owned));
    let Some(value) = value else {
        return Ok(js_str(""));
    };
    let base = Host::of(ctx).state().url.clone();
    Ok(js_str(
        base.join(value.trim()).map_or(value, String::from).as_str(),
    ))
}

fn bool_get(
    this: &JsValue,
    _: &[JsValue],
    attr: &JsString,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr.to_std_string_escaped();
    Ok(JsValue::from(read(ctx, |d| d.attr(id, &name).is_some())))
}

fn bool_set(
    this: &JsValue,
    args: &[JsValue],
    attr: &JsString,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    let name = attr.to_std_string_escaped();
    let on = args.first().is_some_and(JsValue::to_boolean);
    write(ctx, |d| {
        if on {
            d.set_attr(id, &name, "");
        } else {
            d.remove_attr(id, &name);
        }
    });
    Ok(JsValue::undefined())
}

fn get_tag_name(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    Ok(js_str(&read(ctx, |d| node_name(d, id))))
}

fn get_local_name(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = element_of(this, ctx)?;
    Ok(js_str(&read(ctx, |d| {
        d.element(id)
            .map(|e| e.local.to_string())
            .unwrap_or_default()
    })))
}

// ---- 位置と大きさ（レイアウトの結果は JS に渡さないので 0） ----

fn null(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::null())
}

fn noop(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::undefined())
}

/// 文字列として反映する属性（プロパティ名, 属性名）。
const STRING_ATTRS: [(&str, &str); 13] = [
    ("alt", "alt"),
    ("title", "title"),
    ("type", "type"),
    ("name", "name"),
    ("placeholder", "placeholder"),
    ("rel", "rel"),
    ("target", "target"),
    ("method", "method"),
    ("htmlFor", "for"),
    ("lang", "lang"),
    ("dir", "dir"),
    ("role", "role"),
    ("media", "media"),
];

/// 真偽として反映する属性。
const BOOL_ATTRS: [(&str, &str); 9] = [
    ("checked", "checked"),
    ("disabled", "disabled"),
    ("hidden", "hidden"),
    ("selected", "selected"),
    ("readOnly", "readonly"),
    ("required", "required"),
    ("multiple", "multiple"),
    ("async", "async"),
    ("defer", "defer"),
];

pub(crate) fn install(ctx: &mut Context, element: &JsObject, html_element: &JsObject) {
    install_parent_node(ctx, element);
    let methods: [(&str, Native, usize); 17] = [
        ("getAttribute", get_attribute, 1),
        ("setAttribute", set_attribute, 2),
        ("removeAttribute", remove_attribute, 1),
        ("hasAttribute", has_attribute, 1),
        ("toggleAttribute", toggle_attribute, 1),
        ("getAttributeNames", get_attribute_names, 0),
        ("matches", matches, 1),
        ("webkitMatchesSelector", matches, 1),
        ("closest", closest, 1),
        ("insertAdjacentHTML", insert_adjacent_html, 2),
        ("before", before, 0),
        ("after", after, 0),
        ("remove", remove, 0),
        ("replaceWith", replace_with, 0),
        ("scrollIntoView", noop, 0),
        ("focus", noop, 0),
        ("blur", noop, 0),
    ];
    for (name, f, n) in methods {
        define_method(ctx, element, name, f, n);
    }
    let getters: [(&str, Native); 5] = [
        ("tagName", get_tag_name),
        ("localName", get_local_name),
        ("nextElementSibling", get_next_element_sibling),
        ("previousElementSibling", get_previous_element_sibling),
        ("offsetParent", null),
    ];
    for (name, g) in getters {
        define_accessor(ctx, element, name, Some(g), None);
    }
    crate::geometry::install(ctx, element);
    define_accessor(
        ctx,
        element,
        "innerHTML",
        Some(get_inner_html),
        Some(set_inner_html),
    );
    define_accessor(ctx, element, "outerHTML", Some(get_outer_html), None);
    define_named_accessor(ctx, element, ("id", "id"), reflect_get, Some(reflect_set));
    define_named_accessor(
        ctx,
        element,
        ("className", "class"),
        reflect_get,
        Some(reflect_set),
    );

    for (prop, attr) in STRING_ATTRS {
        define_named_accessor(
            ctx,
            html_element,
            (prop, attr),
            reflect_get,
            Some(reflect_set),
        );
    }
    for prop in ["href", "src", "action"] {
        define_named_accessor(ctx, html_element, (prop, prop), url_get, Some(reflect_set));
    }
    for (prop, attr) in BOOL_ATTRS {
        define_named_accessor(ctx, html_element, (prop, attr), bool_get, Some(bool_set));
    }
    define_accessor(ctx, html_element, "value", Some(get_value), Some(set_value));
    // `el.click()` はイベントの章（events）で定義する。
    let _ = arg_node;
}
