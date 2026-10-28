//! `element.dataset`（`data-*` 属性を Proxy で扱う）。

use boa_engine::{Context, JsResult, JsValue};

use crate::node_api::write;
use crate::style_api::{kebab, make_proxy, prop_name, read};
use crate::wrap::{js_str, this_node};

/// `fooBar` → `data-foo-bar`。
pub(crate) fn data_attr(name: &str) -> String {
    format!("data-{}", kebab(name))
}

pub(crate) fn dataset_get(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
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

pub(crate) fn dataset_set(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
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

pub(crate) fn dataset_delete(
    _: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    if let (Some(target), Some(name)) = (args.first(), args.get(1).and_then(prop_name)) {
        let id = this_node(target)?;
        write(ctx, |d| d.remove_attr(id, &data_attr(&name)));
    }
    Ok(JsValue::from(true))
}

pub(crate) fn get_dataset(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = this_node(this)?;
    make_proxy(ctx, "dataset", id)
}

/// `"fooBar" in el.dataset` は `data-foo-bar` 属性があるときだけ真。
pub(crate) fn dataset_has(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let (Some(target), Some(name)) = (args.first(), args.get(1).and_then(prop_name)) else {
        return Ok(JsValue::from(false));
    };
    let id = this_node(target)?;
    let present = read(ctx, |d| d.attr(id, &data_attr(&name)).is_some());
    Ok(JsValue::from(present))
}
