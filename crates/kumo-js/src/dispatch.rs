//! イベントの登録と配送：`addEventListener`、捕捉・目標・バブリングの 3 段階、`on*` ハンドラ（属性を含む）、
//! `click()` とその既定の動作。`Event` オブジェクトそのものは [`crate::events`]。

use boa_engine::{Context, JsObject, JsResult, JsString, JsValue};
use kumo_dom::NodeId;

use crate::events::{EventData, OBJECT_KEY_BASE, bool_prop, create_event, describe_error};
use crate::host::{Host, Listener, WINDOW_KEY};
use crate::wrap::{
    arg_string, define_method, node_of, target_key, type_error, window_object, wrap_node,
};

/// EventTarget のメソッドを `target` に定義する。
pub(crate) fn install_event_target(ctx: &mut Context, target: &JsObject) {
    define_method(ctx, target, "addEventListener", add_event_listener, 2);
    define_method(ctx, target, "removeEventListener", remove_event_listener, 2);
    define_method(ctx, target, "dispatchEvent", dispatch_event_method, 1);
}

// ---- リスナーの登録 ----

/// `this` を対象にする。`window.addEventListener` を素の関数として呼んだ場合（this が未定義）は window。
fn this_key(this: &JsValue, ctx: &Context) -> JsResult<usize> {
    if this.is_null_or_undefined() {
        return Ok(WINDOW_KEY);
    }
    target_key(ctx, this).ok_or_else(|| type_error("Illegal invocation"))
}

fn options(arg: Option<&JsValue>, ctx: &mut Context) -> (bool, bool) {
    match arg {
        Some(v) if v.is_object() => {
            let object = v.as_object().expect("オブジェクト");
            (
                bool_prop(&object, "capture", ctx),
                bool_prop(&object, "once", ctx),
            )
        }
        Some(v) => (v.to_boolean(), false),
        None => (false, false),
    }
}

fn add_event_listener(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let key = this_key(this, ctx)?;
    let event = arg_string(args, 0, ctx)?;
    let Some(callback) = args.get(1).and_then(JsValue::as_object) else {
        return Ok(JsValue::undefined());
    };
    let (capture, once) = options(args.get(2), ctx);
    let host = Host::of(ctx);
    let mut state = host.state_mut();
    let list = state.listeners.entry(key).or_default();
    let duplicate = list.iter().any(|l| {
        l.event == event && l.capture == capture && JsObject::equals(&l.callback, &callback)
    });
    if !duplicate {
        list.push(Listener {
            event,
            callback,
            capture,
            once,
        });
    }
    Ok(JsValue::undefined())
}

fn remove_event_listener(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let key = this_key(this, ctx)?;
    let event = arg_string(args, 0, ctx)?;
    let Some(callback) = args.get(1).and_then(JsValue::as_object) else {
        return Ok(JsValue::undefined());
    };
    let (capture, _) = options(args.get(2), ctx);
    let host = Host::of(ctx);
    let mut state = host.state_mut();
    if let Some(list) = state.listeners.get_mut(&key) {
        list.retain(|l| {
            !(l.event == event && l.capture == capture && JsObject::equals(&l.callback, &callback))
        });
    }
    Ok(JsValue::undefined())
}

fn dispatch_event_method(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let key = this_key(this, ctx)?;
    let Some(event) = args.first().and_then(JsValue::as_object) else {
        return Err(type_error("引数が Event ではありません"));
    };
    if event.downcast_ref::<EventData>().is_none() {
        return Err(type_error("引数が Event ではありません"));
    }
    let target = if this.is_null_or_undefined() {
        window_object(ctx).into()
    } else {
        this.clone()
    };
    Ok(JsValue::from(dispatch(ctx, key, &target, &event)?))
}

// ---- 配送 ----

/// イベントの配送経路（対象から根へ）。ノードは親をたどり、文書に接続していれば最後に window が加わる。
fn path_of(ctx: &Context, key: usize) -> Vec<usize> {
    if key == WINDOW_KEY || key >= OBJECT_KEY_BASE {
        return vec![key];
    }
    let host = Host::of(ctx);
    let doc = host.doc();
    let doc = doc.borrow();
    let mut path = vec![key];
    let mut current = NodeId::from_index(key);
    while let Some(parent) = doc.parent(current) {
        path.push(parent.index());
        current = parent;
    }
    if path.last() == Some(&doc.root().index()) {
        path.push(WINDOW_KEY);
    }
    path
}

fn wrapper_for(ctx: &mut Context, key: usize) -> JsObject {
    if key == WINDOW_KEY {
        window_object(ctx)
    } else if key >= OBJECT_KEY_BASE {
        Host::of(ctx)
            .state()
            .wrappers
            .get(&key)
            .cloned()
            .unwrap_or_else(|| window_object(ctx))
    } else {
        wrap_node(ctx, NodeId::from_index(key))
    }
}

fn flags(event: &JsObject) -> (bool, bool, bool) {
    event
        .downcast_ref::<EventData>()
        .map_or((false, false, false), |d| {
            (d.stopped, d.stopped_immediate, d.bubbles)
        })
}

/// 1つの対象でリスナーを呼ぶ。`phase` は 1 = 捕捉、2 = 目標、3 = バブリング。
fn run_listeners(ctx: &mut Context, key: usize, kind: &str, phase: u8, event: &JsObject) {
    let host = Host::of(ctx);
    let current = wrapper_for(ctx, key);
    let _ = event.set(JsString::from("currentTarget"), current.clone(), false, ctx);
    let _ = event.set(JsString::from("eventPhase"), i32::from(phase), false, ctx);
    let snapshot: Vec<Listener> = host
        .state()
        .listeners
        .get(&key)
        .map(|list| {
            list.iter()
                .filter(|l| {
                    l.event == kind
                        && match phase {
                            1 => l.capture,
                            3 => !l.capture,
                            _ => true,
                        }
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    for listener in snapshot {
        if flags(event).1 {
            return;
        }
        if listener.once
            && let Some(list) = host.state_mut().listeners.get_mut(&key)
        {
            list.retain(|l| {
                !(l.event == listener.event && JsObject::equals(&l.callback, &listener.callback))
            });
        }
        call_listener(ctx, &listener.callback, &current, event);
    }
    if phase != 1 && !flags(event).1 {
        call_on_handler(ctx, &current, kind, event);
    }
}

fn call_listener(ctx: &mut Context, callback: &JsObject, current: &JsObject, event: &JsObject) {
    let result = if callback.is_callable() {
        callback.call(&current.clone().into(), &[event.clone().into()], ctx)
    } else {
        match callback.get(JsString::from("handleEvent"), ctx) {
            Ok(handler) if handler.is_callable() => handler
                .as_callable()
                .expect("呼び出せる")
                .call(&callback.clone().into(), &[event.clone().into()], ctx),
            _ => Ok(JsValue::undefined()),
        }
    };
    if let Err(error) = result {
        let message = describe_error(&error, ctx);
        Host::of(ctx).report_error(format!("Uncaught {message}"));
    }
}

/// `onclick="…"` のような属性のハンドラ。本文が変わっていなければコンパイル済みの関数を使い回す。
fn inline_handler(ctx: &mut Context, current: &JsObject, kind: &str) -> Option<JsObject> {
    let id = crate::wrap::node_of(&current.clone().into())?;
    let host = Host::of(ctx);
    let code = host
        .doc()
        .borrow()
        .attr(id, &format!("on{kind}"))?
        .to_owned();
    let key = (id.index(), kind.to_owned());
    if let Some((cached_code, function)) = host.state().inline_handlers.get(&key)
        && *cached_code == code
    {
        return Some(function.clone());
    }
    let source = format!(
        "(function(event) {{
{code}
}})"
    );
    let function = ctx
        .eval(boa_engine::Source::from_bytes(&source))
        .ok()?
        .as_object()?;
    host.state_mut()
        .inline_handlers
        .insert(key, (code, function.clone()));
    Some(function)
}

/// `onclick` などのプロパティに代入されたハンドラを呼ぶ。`false` を返したら既定動作を止める。
fn call_on_handler(ctx: &mut Context, current: &JsObject, kind: &str, event: &JsObject) {
    let property = current.get(JsString::from(format!("on{kind}")), ctx).ok();
    let Some(function) = property
        .and_then(|h| h.as_callable())
        .or_else(|| inline_handler(ctx, current, kind))
    else {
        return;
    };
    match function.call(&current.clone().into(), &[event.clone().into()], ctx) {
        Ok(result) if result.as_boolean() == Some(false) => {
            if let Some(mut data) = event.downcast_mut::<EventData>()
                && data.cancelable
            {
                data.prevented = true;
            }
        }
        Ok(_) => {}
        Err(error) => {
            let message = describe_error(&error, ctx);
            Host::of(ctx).report_error(format!("Uncaught {message}"));
        }
    }
}

/// イベントを配送する。`preventDefault` されていなければ `true`。
pub(crate) fn dispatch(
    ctx: &mut Context,
    key: usize,
    target: &JsValue,
    event: &JsObject,
) -> JsResult<bool> {
    let kind = event
        .get(JsString::from("type"), ctx)?
        .to_string(ctx)?
        .to_std_string_escaped();
    event.set(JsString::from("target"), target.clone(), false, ctx)?;
    event.set(JsString::from("srcElement"), target.clone(), false, ctx)?;
    let path = path_of(ctx, key);
    let bubbles = flags(event).2;
    // 捕捉：根から対象の親まで
    for &k in path.iter().skip(1).rev() {
        run_listeners(ctx, k, &kind, 1, event);
        if flags(event).0 {
            break;
        }
    }
    if !flags(event).0 {
        run_listeners(ctx, path[0], &kind, 2, event);
    }
    // バブリング：対象の親から根まで
    if bubbles && !flags(event).0 {
        for &k in path.iter().skip(1) {
            run_listeners(ctx, k, &kind, 3, event);
            if flags(event).0 {
                break;
            }
        }
    }
    event.set(JsString::from("currentTarget"), JsValue::null(), false, ctx)?;
    event.set(JsString::from("eventPhase"), 0, false, ctx)?;
    Ok(!event
        .downcast_ref::<EventData>()
        .is_some_and(|d| d.prevented))
}

/// ノードにイベントを送る（Content がユーザー操作を JS に渡すときに使う）。既定動作を止められたら `false`。
pub(crate) fn dispatch_to_node(
    ctx: &mut Context,
    node: NodeId,
    class: &str,
    kind: &str,
    init: &[(&str, JsValue)],
) -> JsResult<bool> {
    let init_object = JsObject::with_object_proto(ctx.intrinsics());
    init_object.set(JsString::from("bubbles"), true, false, ctx)?;
    init_object.set(JsString::from("cancelable"), true, false, ctx)?;
    for (name, value) in init {
        init_object.set(JsString::from(*name), value.clone(), false, ctx)?;
    }
    let event = create_event(ctx, class, kind, Some(&init_object))?;
    event.set(JsString::from("isTrusted"), true, false, ctx)?;
    let target: JsValue = wrap_node(ctx, node).into();
    dispatch(ctx, node.index(), &target, &event)
}

/// `window` にイベントを送る（`load` など。バブリングしない）。
pub(crate) fn dispatch_to_window(ctx: &mut Context, kind: &str) -> JsResult<bool> {
    let event = create_event(ctx, "Event", kind, None)?;
    let target: JsValue = window_object(ctx).into();
    dispatch(ctx, WINDOW_KEY, &target, &event)
}

// ---- click() ----

pub(crate) fn click(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let Some(node) = node_of(this) else {
        return Err(type_error("Illegal invocation"));
    };
    if dispatch_to_node(ctx, node, "MouseEvent", "click", &[])? {
        default_click_action(ctx, node);
    }
    Ok(JsValue::undefined())
}

/// `click` の既定動作：リンクなら移動、チェックボックス・ラジオなら切り替え。
pub(crate) fn default_click_action(ctx: &mut Context, node: NodeId) {
    let host = Host::of(ctx);
    let doc = host.doc();
    let anchor = {
        let doc = doc.borrow();
        let mut current = Some(node);
        let mut found = None;
        while let Some(n) = current {
            if doc.element(n).is_some_and(|e| e.is_html("a")) && doc.attr(n, "href").is_some() {
                found = doc.attr(n, "href").map(str::to_owned);
                break;
            }
            current = doc.parent(n);
        }
        found
    };
    if let Some(href) = anchor {
        let base = host.state().url.clone();
        if let Ok(url) = base.join(href.trim())
            && url.scheme() != "javascript"
            && crate::host::may_reach(&base, &url)
        {
            host.state_mut().navigation = Some(url.to_string());
        }
        return;
    }
    let mut doc = doc.borrow_mut();
    let kind = doc.attr(node, "type").map(str::to_ascii_lowercase);
    if doc.element(node).is_some_and(|e| e.is_html("input")) {
        match kind.as_deref() {
            Some("checkbox") => {
                if doc.attr(node, "checked").is_some() {
                    doc.remove_attr(node, "checked");
                } else {
                    doc.set_attr(node, "checked", "");
                }
                drop(doc);
                host.mark_dirty();
            }
            Some("radio") => {
                doc.set_attr(node, "checked", "");
                drop(doc);
                host.mark_dirty();
            }
            _ => {}
        }
    }
}
