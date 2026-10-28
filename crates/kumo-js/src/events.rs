//! イベント：`Event` の各クラス、`addEventListener`、捕捉・目標・バブリングの 3 段階の配送、`on*` ハンドラ。

use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::JsArray;
use boa_engine::property::{Attribute, PropertyDescriptor};
use boa_engine::{Context, JsData, JsError, JsObject, JsResult, JsString, JsValue, NativeFunction};
use boa_gc::{Finalize, Trace};

use crate::host::Host;
use crate::wrap::{Native, arg_string, define_accessor, define_method, js_str, type_error};

/// イベントの内部状態（JS から見えるプロパティとは別に、配送の制御に使う）。
#[derive(Trace, Finalize, JsData, Default)]
pub(crate) struct EventData {
    pub prevented: bool,
    pub stopped: bool,
    pub stopped_immediate: bool,
    pub cancelable: bool,
    pub bubbles: bool,
}

/// `Event` を継承するクラスの名前。
pub(crate) const EVENT_CLASSES: [&str; 8] = [
    "Event",
    "CustomEvent",
    "UIEvent",
    "MouseEvent",
    "KeyboardEvent",
    "FocusEvent",
    "InputEvent",
    "PointerEvent",
];

/// 例外を「TypeError: message」のような文字列にする。
pub(crate) fn describe_error(error: &JsError, ctx: &mut Context) -> String {
    // 実行時の上限（ループの反復・再帰）による停止は JS の値にできないので、エンジンの説明をそのまま使う
    let Ok(value) = error.clone().into_opaque(ctx) else {
        return format!("RangeError: {error}");
    };
    let message = value
        .to_string(ctx)
        .map_or_else(|_| "不明な例外".to_owned(), |s| s.to_std_string_escaped());
    // 原因の位置を追えるよう、スタックの先頭の1行（`at 関数 (… :行:桁)`）を添える
    let frame = value
        .as_object()
        .and_then(|o| o.get(JsString::from("stack"), ctx).ok())
        .and_then(|stack| stack.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .and_then(|s| {
            s.lines()
                .map(str::trim)
                .find(|l| l.starts_with("at "))
                .map(str::to_owned)
        });
    match frame {
        Some(frame) => format!("{message} [{frame}]"),
        None => message,
    }
}

pub(crate) fn bool_prop(object: &JsObject, name: &str, ctx: &mut Context) -> bool {
    object
        .get(JsString::from(name), ctx)
        .map(|v| v.to_boolean())
        .unwrap_or(false)
}

/// イベントオブジェクトを作る。`init` の自前プロパティはそのまま複製する（detail・key・clientX など）。
pub(crate) fn create_event(
    ctx: &mut Context,
    class: &str,
    kind: &str,
    init: Option<&JsObject>,
) -> JsResult<JsObject> {
    create_event_with_proto(ctx, class, kind, init, None)
}

/// `new_target` の `prototype`（サブクラスの継承用）。なければ `default`。
pub(crate) fn proto_from_new_target(
    new_target: &JsValue,
    default: JsObject,
    ctx: &mut Context,
) -> JsObject {
    new_target
        .as_object()
        .and_then(|c| c.get(JsString::from("prototype"), ctx).ok())
        .and_then(|p| p.as_object())
        .unwrap_or(default)
}

fn create_event_with_proto(
    ctx: &mut Context,
    class: &str,
    kind: &str,
    init: Option<&JsObject>,
    proto: Option<JsObject>,
) -> JsResult<JsObject> {
    let host = Host::of(ctx);
    let proto = proto.unwrap_or_else(|| host.proto(class));
    let object = JsObject::from_proto_and_data(Some(proto), EventData::default());
    let event: JsObject = object.clone().upcast();
    let (bubbles, cancelable) = match init {
        Some(init) => (
            bool_prop(init, "bubbles", ctx),
            bool_prop(init, "cancelable", ctx),
        ),
        None => (false, false),
    };
    if let Some(mut data) = event.downcast_mut::<EventData>() {
        data.bubbles = bubbles;
        data.cancelable = cancelable;
    }
    let keys: Vec<(&str, JsValue)> = vec![
        ("type", js_str(kind)),
        ("bubbles", bubbles.into()),
        ("cancelable", cancelable.into()),
        ("composed", false.into()),
        ("isTrusted", false.into()),
        ("target", JsValue::null()),
        ("currentTarget", JsValue::null()),
        ("eventPhase", 0.into()),
        ("detail", JsValue::null()),
        ("timeStamp", host.state().now.as_secs_f64().into()),
    ];
    for (name, value) in keys {
        event.set(JsString::from(name), value, false, ctx)?;
    }
    if let Some(init) = init {
        for key in init.own_property_keys(ctx)? {
            let value = init.get(key.clone(), ctx)?;
            event.set(key, value, false, ctx)?;
        }
    }
    Ok(event)
}

/// `new Event(type, init)`。`this` は `new` の対象（コンストラクタ）なので、その `prototype` を使う。
fn event_constructor(
    this: &JsValue,
    args: &[JsValue],
    class: &JsString,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    if this.is_undefined() {
        return Err(type_error("コンストラクタは new で呼んでください"));
    }
    let class = class.to_std_string_escaped();
    let proto = proto_from_new_target(this, Host::of(ctx).proto(&class), ctx);
    let kind = arg_string(args, 0, ctx)?;
    let init = args.get(1).and_then(JsValue::as_object);
    let event = create_event_with_proto(ctx, &class, &kind, init.as_ref(), Some(proto))?;
    Ok(event.into())
}

/// `EventTarget` を直接 `new` したオブジェクト（ノードでも window でもない）が持つ識別子。
#[derive(Trace, Finalize, JsData)]
pub(crate) struct TargetData {
    #[unsafe_ignore_trace]
    pub key: usize,
}

/// ノードの添字と重ならない、独立した対象のキーの下限。
pub(crate) const OBJECT_KEY_BASE: usize = 1 << 40;

fn event_target_constructor(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if this.is_undefined() {
        return Err(type_error("コンストラクタは new で呼んでください"));
    }
    let host = Host::of(ctx);
    let proto = proto_from_new_target(this, host.proto("event_target"), ctx);
    let key = {
        let state = host.state();
        OBJECT_KEY_BASE + state.wrappers.len()
    };
    let object: JsObject = JsObject::from_proto_and_data(Some(proto), TargetData { key }).upcast();
    host.state_mut().wrappers.insert(key, object.clone());
    Ok(object.into())
}

fn prevent_default(this: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    if let Some(object) = this.as_object()
        && let Some(mut data) = object.downcast_mut::<EventData>()
        && data.cancelable
    {
        data.prevented = true;
    }
    Ok(JsValue::undefined())
}

fn stop_propagation(this: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    if let Some(object) = this.as_object()
        && let Some(mut data) = object.downcast_mut::<EventData>()
    {
        data.stopped = true;
    }
    Ok(JsValue::undefined())
}

fn stop_immediate(this: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    if let Some(object) = this.as_object()
        && let Some(mut data) = object.downcast_mut::<EventData>()
    {
        data.stopped = true;
        data.stopped_immediate = true;
    }
    Ok(JsValue::undefined())
}

fn get_default_prevented(this: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let prevented = this
        .as_object()
        .and_then(|o| o.downcast_ref::<EventData>().map(|d| d.prevented))
        .unwrap_or(false);
    Ok(JsValue::from(prevented))
}

fn composed_path(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(JsArray::new(ctx)?.into())
}

fn create_event_method(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let name = arg_string(args, 0, ctx)?;
    let class = match name.to_ascii_lowercase().as_str() {
        "customevent" => "CustomEvent",
        "mouseevent" | "mouseevents" => "MouseEvent",
        "keyboardevent" => "KeyboardEvent",
        "uievent" | "uievents" => "UIEvent",
        _ => "Event",
    };
    Ok(create_event(ctx, class, "", None)?.into())
}

fn init_event(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let Some(event) = this.as_object() else {
        return Err(type_error("Illegal invocation"));
    };
    let kind = arg_string(args, 0, ctx)?;
    let bubbles = args.get(1).is_some_and(JsValue::to_boolean);
    let cancelable = args.get(2).is_some_and(JsValue::to_boolean);
    if let Some(mut data) = event.downcast_mut::<EventData>() {
        data.bubbles = bubbles;
        data.cancelable = cancelable;
    }
    event.set(JsString::from("type"), js_str(&kind), false, ctx)?;
    event.set(JsString::from("bubbles"), bubbles, false, ctx)?;
    event.set(JsString::from("cancelable"), cancelable, false, ctx)?;
    Ok(JsValue::undefined())
}

/// `document.createEvent` 用に外へ出す。
pub(crate) fn create_event_native() -> Native {
    create_event_method
}

/// `Event` 系のクラスを登録する。
pub(crate) fn install(ctx: &mut Context, element: &JsObject) {
    let host = Host::of(ctx);
    let base = JsObject::with_object_proto(ctx.intrinsics());
    let methods: [(&str, Native, usize); 5] = [
        ("preventDefault", prevent_default, 0),
        ("stopPropagation", stop_propagation, 0),
        ("stopImmediatePropagation", stop_immediate, 0),
        ("composedPath", composed_path, 0),
        ("initEvent", init_event, 3),
    ];
    for (name, f, n) in methods {
        define_method(ctx, &base, name, f, n);
    }
    define_accessor(
        ctx,
        &base,
        "defaultPrevented",
        Some(get_default_prevented),
        None,
    );
    define_accessor(ctx, &base, "returnValue", Some(get_default_prevented), None);
    host.state_mut().protos.insert("Event", base.clone());
    for class in EVENT_CLASSES {
        let proto = if class == "Event" {
            base.clone()
        } else {
            let derived = JsObject::with_object_proto(ctx.intrinsics());
            let _ = derived.set_prototype(Some(base.clone()));
            derived
        };
        host.state_mut()
            .protos
            .insert(class_key(class), proto.clone());
        let constructor: JsObject = FunctionObjectBuilder::new(
            ctx.realm(),
            NativeFunction::from_copy_closure_with_captures(
                |this, args, captured: &JsString, ctx| event_constructor(this, args, captured, ctx),
                JsString::from(class),
            ),
        )
        .name(JsString::from(class))
        .length(1)
        .constructor(true)
        .build()
        .into();
        let prototype = PropertyDescriptor::builder()
            .value(proto.clone())
            .writable(false)
            .enumerable(false);
        let _ = constructor.define_property_or_throw(JsString::from("prototype"), prototype, ctx);
        let _ = proto.set(
            JsString::from("constructor"),
            constructor.clone(),
            false,
            ctx,
        );
        let _ = ctx.register_global_property(JsString::from(class), constructor, Attribute::all());
    }
    define_method(ctx, element, "click", crate::dispatch::click, 0);
    install_event_target_class(ctx);
}

/// `EventTarget` コンストラクタ（`new EventTarget()` とサブクラス化のため）を登録する。
fn install_event_target_class(ctx: &mut Context) {
    let host = Host::of(ctx);
    let proto = host.proto("event_target");
    let constructor: JsObject = FunctionObjectBuilder::new(
        ctx.realm(),
        NativeFunction::from_fn_ptr(event_target_constructor),
    )
    .name(JsString::from("EventTarget"))
    .length(0)
    .constructor(true)
    .build()
    .into();
    let prototype = PropertyDescriptor::builder()
        .value(proto.clone())
        .writable(false)
        .enumerable(false);
    let _ = constructor.define_property_or_throw(JsString::from("prototype"), prototype, ctx);
    let _ = proto.set(
        JsString::from("constructor"),
        constructor.clone(),
        false,
        ctx,
    );
    let _ =
        ctx.register_global_property(JsString::from("EventTarget"), constructor, Attribute::all());
}

/// `&'static str` のキーにするための対応表（クラス名 → プロトタイプ表のキー）。
fn class_key(class: &str) -> &'static str {
    EVENT_CLASSES
        .iter()
        .copied()
        .find(|c| *c == class)
        .unwrap_or("Event")
}
