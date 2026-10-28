//! DOM ノードの JS ラッパー：同じノードは同じ JS オブジェクトを返す。ヘルパーもここに置く。

use boa_engine::object::FunctionObjectBuilder;
use boa_engine::property::PropertyDescriptor;
use boa_engine::{
    Context, JsData, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction,
};
use boa_gc::{Finalize, Trace};
use kumo_dom::{NodeId, NodeKind};

use crate::host::{Host, WINDOW_KEY};

/// ネイティブ関数の型。
pub(crate) type Native = fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue>;

/// ラッパーが指すノード。
#[derive(Trace, Finalize, JsData)]
pub(crate) struct NodeData {
    #[unsafe_ignore_trace]
    pub id: NodeId,
}

pub(crate) fn type_error(message: &str) -> boa_engine::JsError {
    JsNativeError::typ().with_message(message.to_owned()).into()
}

/// ノードの JS ラッパー（なければ作る）。ノードの種類でプロトタイプが変わる。
pub(crate) fn wrap_node(ctx: &mut Context, id: NodeId) -> JsObject {
    let host = Host::of(ctx);
    if let Some(found) = host.state().wrappers.get(&id.index()) {
        return found.clone();
    }
    let proto_name = {
        let doc = host.doc();
        let doc = doc.borrow();
        match doc.node(id).kind() {
            NodeKind::Document => "document",
            NodeKind::Text(_) => "text",
            NodeKind::Element(_) => match doc.element(id) {
                Some(e) if &*e.local == "#document-fragment" => "fragment",
                Some(e) if &*e.local == crate::node_api::DETACHED_DOCUMENT => "document",
                Some(e) if e.ns == kumo_dom::ns!(html) => "html_element",
                _ => "element",
            },
            _ => "node",
        }
    };
    let object =
        JsObject::from_proto_and_data(Some(host.proto(proto_name)), NodeData { id }).upcast();
    host.state_mut().wrappers.insert(id.index(), object.clone());
    object
}

/// `window` のラッパー（グローバルオブジェクト）。
pub(crate) fn window_object(ctx: &Context) -> JsObject {
    ctx.global_object()
}

/// ラッパーからノードを取り出す。ノードでなければ `None`。
pub(crate) fn node_of(value: &JsValue) -> Option<NodeId> {
    value.as_object()?.downcast_ref::<NodeData>().map(|d| d.id)
}

/// `this` がノードであることを確かめる。
pub(crate) fn this_node(this: &JsValue) -> JsResult<NodeId> {
    node_of(this).ok_or_else(|| type_error("Illegal invocation"))
}

/// イベント対象のキー（ノードは添字、`window` は [`WINDOW_KEY`]）。
pub(crate) fn target_key(ctx: &Context, value: &JsValue) -> Option<usize> {
    if let Some(id) = node_of(value) {
        return Some(id.index());
    }
    let object = value.as_object()?;
    if let Some(data) = object.downcast_ref::<crate::events::TargetData>() {
        return Some(data.key);
    }
    (JsObject::equals(&object, &window_object(ctx))).then_some(WINDOW_KEY)
}

/// 引数 `i` を文字列にする（なければ空）。
pub(crate) fn arg_string(args: &[JsValue], i: usize, ctx: &mut Context) -> JsResult<String> {
    match args.get(i) {
        Some(v) => Ok(v.to_string(ctx)?.to_std_string_escaped()),
        None => Ok(String::new()),
    }
}

/// 引数 `i` をノードにする。
pub(crate) fn arg_node(args: &[JsValue], i: usize) -> JsResult<NodeId> {
    args.get(i)
        .and_then(node_of)
        .ok_or_else(|| type_error("引数が Node ではありません"))
}

pub(crate) fn js_str(s: &str) -> JsValue {
    JsValue::from(JsString::from(s))
}

/// ノードの JS 値（`None` は null）。
pub(crate) fn node_value(ctx: &mut Context, id: Option<NodeId>) -> JsValue {
    id.map_or(JsValue::null(), |id| wrap_node(ctx, id).into())
}

fn native_function(ctx: &mut Context, name: &str, f: Native, length: usize) -> JsObject {
    FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(f))
        .name(JsString::from(name))
        .length(length)
        .build()
        .into()
}

/// `target` にメソッドを定義する（列挙しない）。
pub(crate) fn define_method(
    ctx: &mut Context,
    target: &JsObject,
    name: &str,
    f: Native,
    length: usize,
) {
    let function = native_function(ctx, name, f, length);
    let descriptor = PropertyDescriptor::builder()
        .value(function)
        .writable(true)
        .enumerable(false)
        .configurable(true);
    let _ = target.define_property_or_throw(JsString::from(name), descriptor, ctx);
}

/// `target` にアクセサ（getter / setter）を定義する。
pub(crate) fn define_accessor(
    ctx: &mut Context,
    target: &JsObject,
    name: &str,
    getter: Option<Native>,
    setter: Option<Native>,
) {
    let mut descriptor = PropertyDescriptor::builder()
        .enumerable(false)
        .configurable(true);
    if let Some(g) = getter {
        descriptor = descriptor.get(native_function(ctx, name, g, 0));
    }
    if let Some(s) = setter {
        descriptor = descriptor.set(native_function(ctx, name, s, 1));
    }
    let _ = target.define_property_or_throw(JsString::from(name), descriptor, ctx);
}

type NamedNative = fn(&JsValue, &[JsValue], &JsString, &mut Context) -> JsResult<JsValue>;

/// 名前（属性名など）を捕捉する関数。同じ実装を複数のプロパティで使い回すため。
fn named_function(
    ctx: &mut Context,
    display: &str,
    captured: &str,
    f: NamedNative,
    length: usize,
) -> JsObject {
    FunctionObjectBuilder::new(
        ctx.realm(),
        NativeFunction::from_copy_closure_with_captures(
            move |this, args, captured: &JsString, ctx| f(this, args, captured, ctx),
            JsString::from(captured),
        ),
    )
    .name(JsString::from(display))
    .length(length)
    .build()
    .into()
}

/// プロパティ `prop` の getter / setter を、属性名 `attr` を捕捉する関数として定義する。
pub(crate) fn define_named_accessor(
    ctx: &mut Context,
    target: &JsObject,
    (prop, attr): (&str, &str),
    getter: NamedNative,
    setter: Option<NamedNative>,
) {
    let mut descriptor = PropertyDescriptor::builder()
        .enumerable(true)
        .configurable(true)
        .get(named_function(ctx, prop, attr, getter, 0));
    if let Some(s) = setter {
        descriptor = descriptor.set(named_function(ctx, prop, attr, s, 1));
    }
    let _ = target.define_property_or_throw(JsString::from(prop), descriptor, ctx);
}
