//! プレリュード（JS で書いた Web API）が使うネイティブ関数：ログ、URL、移動、`fetch`、メディアクエリ。

use boa_engine::object::builtins::JsPromise;
use boa_engine::{Context, JsObject, JsResult, JsString, JsValue};
use kumo_css::{Device, MediaQueryList};

use crate::host::{FetchOutcome, FetchRequest, Host, MAX_LOG, may_reach};
use crate::wrap::{arg_string, define_method, js_str, type_error};

fn log(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let level = arg_string(args, 0, ctx)?;
    let text = arg_string(args, 1, ctx)?;
    let host = Host::of(ctx);
    let mut state = host.state_mut();
    if state.console.len() < MAX_LOG {
        state.console.push(format!("[{level}] {text}"));
    }
    Ok(JsValue::undefined())
}

fn current_url(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(js_str(Host::of(ctx).state().url.as_str()))
}

/// `history.pushState` / `replaceState`：表示は変えず、`location` の値だけを差し替える。
fn set_url(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let target = arg_string(args, 0, ctx)?;
    let host = Host::of(ctx);
    let base = host.state().url.clone();
    let Ok(next) = base.join(&target) else {
        return Err(type_error("SecurityError: URL が正しくありません"));
    };
    // 同じオリジンの URL にしか変えられない
    if next.origin() != base.origin() {
        return Err(type_error(
            "SecurityError: 別のオリジンの URL にはできません",
        ));
    }
    host.state_mut().url = next;
    Ok(JsValue::undefined())
}

/// `location.href = …` など：移動を求める（実際の移動は Browser が行う）。
fn navigate(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let target = arg_string(args, 0, ctx)?;
    let host = Host::of(ctx);
    let base = host.state().url.clone();
    if let Ok(url) = base.join(target.trim())
        && url.scheme() != "javascript"
        && may_reach(&base, &url)
    {
        host.state_mut().navigation = Some(url.to_string());
    }
    Ok(JsValue::undefined())
}

fn viewport(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let (w, h) = Host::of(ctx).state().viewport;
    Ok(js_str(&format!("{w},{h}")))
}

fn media(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let query = arg_string(args, 0, ctx)?;
    let (width, height) = Host::of(ctx).state().viewport;
    Ok(JsValue::from(
        MediaQueryList::parse_str(&query).matches(&Device { width, height }),
    ))
}

/// `__kumo_fetch(url, method, body, contentType)`：Promise を返し、結果は Content が `fetch_done` で渡す。
fn fetch(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let target = arg_string(args, 0, ctx)?;
    let method = arg_string(args, 1, ctx)?.to_ascii_uppercase();
    let body = match args.get(2) {
        Some(v) if !v.is_null_or_undefined() => {
            Some(v.to_string(ctx)?.to_std_string_escaped().into_bytes())
        }
        _ => None,
    };
    let content_type = match args.get(3) {
        Some(v) if !v.is_null_or_undefined() => Some(v.to_string(ctx)?.to_std_string_escaped()),
        _ => None,
    };
    let host = Host::of(ctx);
    let base = host.state().url.clone();
    let (promise, resolvers) = JsPromise::new_pending(ctx);
    match base.join(target.trim()) {
        Ok(url) if matches!(url.scheme(), "http" | "https" | "file") && may_reach(&base, &url) => {
            let mut state = host.state_mut();
            let id = state.next_fetch;
            state.next_fetch += 1;
            state
                .pending_fetches
                .insert(id, (resolvers.resolve.into(), resolvers.reject.into()));
            state.fetches.push(FetchRequest {
                id,
                url: url.to_string(),
                method,
                body,
                content_type,
            });
        }
        _ => {
            let error = boa_engine::JsNativeError::typ()
                .with_message("Failed to fetch: URL がサポートされていません");
            resolvers
                .reject
                .call(&JsValue::undefined(), &[error.into_opaque(ctx).into()], ctx)?;
        }
    }
    Ok(promise.into())
}

/// 取得結果を Promise に渡す。
pub(crate) fn complete_fetch(ctx: &mut Context, id: u64, outcome: Result<FetchOutcome, String>) {
    let host = Host::of(ctx);
    let Some((resolve, reject)) = host.state_mut().pending_fetches.remove(&id) else {
        return;
    };
    let result = match outcome {
        Ok(response) => {
            let raw = JsObject::with_object_proto(ctx.intrinsics());
            let text = String::from_utf8_lossy(&response.body).into_owned();
            let content_type = response.content_type.unwrap_or_default();
            let fields = [
                ("status", JsValue::from(i32::from(response.status))),
                ("url", js_str(&response.url)),
                ("contentType", js_str(&content_type)),
                ("body", JsValue::from(JsString::from(text.as_str()))),
            ];
            for (name, value) in fields {
                let _ = raw.set(JsString::from(name), value, false, ctx);
            }
            resolve.call(&JsValue::undefined(), &[raw.into()], ctx)
        }
        Err(message) => {
            let error = boa_engine::JsNativeError::typ()
                .with_message(format!("Failed to fetch: {message}"));
            let value = error.into_opaque(ctx);
            reject.call(&JsValue::undefined(), &[value.into()], ctx)
        }
    };
    if let Err(error) = result {
        let message = crate::events::describe_error(&error, ctx);
        host.report_error(format!("fetch: {message}"));
    }
    let _ = ctx.run_jobs();
}

pub(crate) fn install(ctx: &mut Context, global: &JsObject) {
    define_method(ctx, global, "__kumo_log", log, 2);
    define_method(ctx, global, "__kumo_url", current_url, 0);
    define_method(ctx, global, "__kumo_set_url", set_url, 1);
    define_method(ctx, global, "__kumo_navigate", navigate, 1);
    define_method(ctx, global, "__kumo_viewport", viewport, 0);
    define_method(ctx, global, "__kumo_media", media, 1);
    define_method(ctx, global, "__kumo_fetch", fetch, 4);
}
