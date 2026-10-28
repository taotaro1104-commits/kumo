//! `element.style`・`classList`・`dataset`。値は DOM の属性（`style`・`class`・`data-*`）として持つ。

use boa_engine::object::builtins::JsProxy;
use boa_engine::property::PropertyKey;
use boa_engine::{Context, JsObject, JsResult, JsValue, js_string};
use kumo_dom::{Document, NodeId};

use crate::host::Host;
use crate::node_api::write;
use crate::wrap::{Native, NodeData, define_accessor, define_method, js_str, this_node};

fn read<R>(ctx: &Context, f: impl FnOnce(&Document) -> R) -> R {
    let doc = Host::of(ctx).doc();
    let doc = doc.borrow();
    f(&doc)
}

// ---- インラインスタイルの読み書き ----

/// `style` 属性を（プロパティ, 値）の列にする。`url(…;…)` や引用符の中の `;` は区切りにしない。
pub(crate) fn parse_style(text: &str) -> Vec<(String, String)> {
    let mut decls = Vec::new();
    let mut current = String::new();
    let (mut depth, mut quote) = (0u32, None::<char>);
    let mut flush = |current: &mut String| {
        if let Some((name, value)) = current.split_once(':') {
            let (name, value) = (name.trim().to_ascii_lowercase(), value.trim().to_owned());
            if !name.is_empty() {
                decls.push((name, value));
            }
        }
        current.clear();
    };
    for c in text.chars() {
        match (c, quote) {
            ('"' | '\'', None) => quote = Some(c),
            (q, Some(open)) if q == open => quote = None,
            ('(', None) => depth += 1,
            (')', None) => depth = depth.saturating_sub(1),
            (';', None) if depth == 0 => {
                flush(&mut current);
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    flush(&mut current);
    decls
}

pub(crate) fn serialize_style(decls: &[(String, String)]) -> String {
    decls
        .iter()
        .map(|(n, v)| format!("{n}: {v};"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `backgroundColor` → `background-color`、`cssFloat` → `float`、`webkitX` → `-webkit-x`。
pub(crate) fn kebab(name: &str) -> String {
    if name == "cssFloat" {
        return "float".to_owned();
    }
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    // `webkitTransform` → `webkit-transform` → `-webkit-transform`
    if ["webkit-", "moz-", "ms-"]
        .iter()
        .any(|p| out.starts_with(p))
    {
        out.insert(0, '-');
    }
    out
}

fn style_decls(ctx: &Context, id: NodeId) -> Vec<(String, String)> {
    read(ctx, |d| {
        parse_style(d.attr(id, "style").unwrap_or_default())
    })
}

fn store_style(ctx: &Context, id: NodeId, decls: &[(String, String)]) {
    write(ctx, |d| {
        if decls.is_empty() {
            d.remove_attr(id, "style");
        } else {
            d.set_attr(id, "style", &serialize_style(decls));
        }
    });
}

pub(crate) fn set_style_property(ctx: &Context, id: NodeId, name: &str, value: &str) {
    let name = name.to_ascii_lowercase();
    let mut decls = style_decls(ctx, id);
    decls.retain(|(n, _)| *n != name);
    if !value.trim().is_empty() {
        decls.push((name, value.trim().to_owned()));
    }
    store_style(ctx, id, &decls);
}

fn get_style_property(ctx: &Context, id: NodeId, name: &str) -> String {
    let name = name.to_ascii_lowercase();
    style_decls(ctx, id)
        .into_iter()
        .rev()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| v)
        .unwrap_or_default()
}

// ---- style オブジェクト（Proxy で任意のプロパティ名を扱う） ----

fn prop_name(key: &JsValue) -> Option<String> {
    key.as_string().map(|s| s.to_std_string_escaped())
}

/// メソッド・アクセサとして本体オブジェクトに任せる名前。
const STYLE_MEMBERS: [&str; 7] = [
    "setProperty",
    "getPropertyValue",
    "removeProperty",
    "getPropertyPriority",
    "cssText",
    "length",
    "item",
];

fn style_get(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let target = args.first().and_then(JsValue::as_object);
    let (Some(target), Some(key)) = (target, args.get(1)) else {
        return Ok(JsValue::undefined());
    };
    let id = this_node(&target.clone().into())?;
    match prop_name(key) {
        Some(name) if !STYLE_MEMBERS.contains(&name.as_str()) => {
            Ok(js_str(&get_style_property(ctx, id, &kebab(&name))))
        }
        _ => {
            let key: PropertyKey = key.to_property_key(ctx)?;
            let value = target.get(key, ctx)?;
            // メソッドは Proxy ではなく本体を `this` にして呼ばれるよう、本体に束縛して返す
            match value.as_object().filter(JsObject::is_callable) {
                Some(function) => {
                    let bind = function.get(js_string!("bind"), ctx)?;
                    match bind.as_callable() {
                        Some(bind) => bind.call(&function.into(), &[target.into()], ctx),
                        None => Ok(value),
                    }
                }
                None => Ok(value),
            }
        }
    }
}

fn style_set(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let target = args.first().and_then(JsValue::as_object);
    let (Some(target), Some(key)) = (target, args.get(1)) else {
        return Ok(JsValue::from(false));
    };
    let id = this_node(&target.into())?;
    let value = match args.get(2) {
        Some(v) if !v.is_null_or_undefined() => v.to_string(ctx)?.to_std_string_escaped(),
        _ => String::new(),
    };
    match prop_name(key) {
        Some(name) if name == "cssText" => {
            write(ctx, |d| d.set_attr(id, "style", &value));
        }
        Some(name) if !STYLE_MEMBERS.contains(&name.as_str()) => {
            set_style_property(ctx, id, &kebab(&name), &value);
        }
        _ => {}
    }
    Ok(JsValue::from(true))
}

/// `"MozAppearance" in el.style` のような機能検出に正しく答える：
/// ほかのエンジンの接頭辞（Moz・ms・O）の付いたプロパティは持たない。
fn style_has(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let supported = args.get(1).and_then(prop_name).is_some_and(|name| {
        let foreign = ["Moz", "ms", "O"].iter().any(|prefix| {
            name.strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
        });
        !foreign
    });
    Ok(JsValue::from(supported))
}

/// `"fooBar" in el.dataset` は `data-foo-bar` 属性があるときだけ真。
fn dataset_has(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let (Some(target), Some(name)) = (args.first(), args.get(1).and_then(prop_name)) else {
        return Ok(JsValue::from(false));
    };
    let id = this_node(target)?;
    let present = read(ctx, |d| d.attr(id, &data_attr(&name)).is_some());
    Ok(JsValue::from(present))
}

fn style_delete(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if let (Some(target), Some(name)) = (args.first(), args.get(1).and_then(prop_name)) {
        let id = this_node(target)?;
        set_style_property(ctx, id, &kebab(&name), "");
    }
    Ok(JsValue::from(true))
}

fn make_proxy(ctx: &mut Context, proto: &str, id: NodeId) -> JsResult<JsValue> {
    let target =
        JsObject::from_proto_and_data(Some(Host::of(ctx).proto(proto)), NodeData { id }).upcast();
    let proxy = JsProxy::builder(target)
        .get(style_get_for(proto))
        .set(style_set_for(proto))
        .has(if proto == "css_style" {
            style_has
        } else {
            dataset_has
        })
        .delete_property(style_delete_for(proto))
        .build(ctx)?;
    Ok(proxy.into())
}

fn get_style(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    make_proxy(ctx, "css_style", id)
}

fn style_get_for(proto: &str) -> fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> {
    if proto == "css_style" {
        style_get
    } else {
        dataset_get
    }
}

fn style_set_for(proto: &str) -> fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> {
    if proto == "css_style" {
        style_set
    } else {
        dataset_set
    }
}

fn style_delete_for(proto: &str) -> fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> {
    if proto == "css_style" {
        style_delete
    } else {
        dataset_delete
    }
}

fn set_style(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let text = crate::wrap::arg_string(args, 0, ctx)?;
    write(ctx, |d| d.set_attr(id, "style", &text));
    Ok(JsValue::undefined())
}

// style の本体オブジェクトのメソッド
fn css_set_property(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let name = crate::wrap::arg_string(args, 0, ctx)?;
    let value = crate::wrap::arg_string(args, 1, ctx)?;
    set_style_property(ctx, id, &name, &value);
    Ok(JsValue::undefined())
}

fn css_get_property_value(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let name = crate::wrap::arg_string(args, 0, ctx)?;
    Ok(js_str(&get_style_property(ctx, id, &name)))
}

fn css_remove_property(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    let name = crate::wrap::arg_string(args, 0, ctx)?;
    let old = get_style_property(ctx, id, &name);
    set_style_property(ctx, id, &name, "");
    Ok(js_str(&old))
}

fn css_text(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(js_str(&serialize_style(&style_decls(ctx, id))))
}

fn css_length(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    Ok(JsValue::from(style_decls(ctx, id).len() as u32))
}

fn css_priority(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(js_str(""))
}

// ---- dataset ----

/// `fooBar` → `data-foo-bar`。
fn data_attr(name: &str) -> String {
    format!("data-{}", kebab(name))
}

fn dataset_get(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let (Some(target), Some(key)) = (args.first(), args.get(1)) else {
        return Ok(JsValue::undefined());
    };
    let id = this_node(target)?;
    match prop_name(key) {
        Some(name) => {
            let value = read(ctx, |d| d.attr(id, &data_attr(&name)).map(str::to_owned));
            Ok(value.map_or(JsValue::undefined(), |v| js_str(&v)))
        }
        None => Ok(JsValue::undefined()),
    }
}

fn dataset_set(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let (Some(target), Some(key)) = (args.first(), args.get(1)) else {
        return Ok(JsValue::from(false));
    };
    let id = this_node(target)?;
    if let Some(name) = prop_name(key) {
        let value = crate::wrap::arg_string(args, 2, ctx)?;
        write(ctx, |d| d.set_attr(id, &data_attr(&name), &value));
    }
    Ok(JsValue::from(true))
}

fn dataset_delete(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if let (Some(target), Some(name)) = (args.first(), args.get(1).and_then(prop_name)) {
        let id = this_node(target)?;
        write(ctx, |d| d.remove_attr(id, &data_attr(&name)));
    }
    Ok(JsValue::from(true))
}

fn get_dataset(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    make_proxy(ctx, "dataset", id)
}

/// プロトタイプを作って登録する（`element` に `style` などを足す）。
pub(crate) fn install(ctx: &mut Context, element: &JsObject) {
    let host = Host::of(ctx);
    let css = JsObject::with_object_proto(ctx.intrinsics());
    let methods: [(&str, Native, usize); 5] = [
        ("setProperty", css_set_property, 2),
        ("getPropertyValue", css_get_property_value, 1),
        ("removeProperty", css_remove_property, 1),
        ("getPropertyPriority", css_priority, 1),
        ("item", css_priority, 1),
    ];
    for (name, f, n) in methods {
        define_method(ctx, &css, name, f, n);
    }
    define_accessor(ctx, &css, "cssText", Some(css_text), Some(set_style));
    define_accessor(ctx, &css, "length", Some(css_length), None);
    host.state_mut().protos.insert("css_style", css);

    let dataset = JsObject::with_object_proto(ctx.intrinsics());
    host.state_mut().protos.insert("dataset", dataset);

    crate::class_list::install_prototype(ctx);

    define_accessor(ctx, element, "style", Some(get_style), Some(set_style));
    define_accessor(ctx, element, "dataset", Some(get_dataset), None);
    define_accessor(
        ctx,
        element,
        "classList",
        Some(crate::class_list::get_class_list),
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_text_round_trips_with_urls_and_quotes() {
        let decls = parse_style(
            "color: red; background: url(data:image/png;base64,AA); font-family: \"a;b\" ;;",
        );
        assert_eq!(
            decls,
            vec![
                ("color".to_owned(), "red".to_owned()),
                (
                    "background".to_owned(),
                    "url(data:image/png;base64,AA)".to_owned()
                ),
                ("font-family".to_owned(), "\"a;b\"".to_owned()),
            ]
        );
        assert_eq!(serialize_style(&decls[..1]), "color: red;");
    }

    #[test]
    fn camel_and_kebab_convert_both_ways() {
        assert_eq!(kebab("backgroundColor"), "background-color");
        assert_eq!(kebab("cssFloat"), "float");
        assert_eq!(kebab("webkitTransform"), "-webkit-transform");
        assert_eq!(kebab("color"), "color");
        assert_eq!(data_attr("fooBar"), "data-foo-bar");
    }
}
