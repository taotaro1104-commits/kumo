//! タイマー：`setTimeout` / `setInterval` / `requestAnimationFrame`。時刻は Content が進める仮想時刻で、
//! 期限の来たものだけを [`run_due`] が実行する（実時間の待ちは Content のイベントループが受け持つ）。

use std::time::Duration;

use boa_engine::{Context, JsObject, JsResult, JsValue};

use crate::events::describe_error;
use crate::host::{Host, Timer};
use crate::wrap::{arg_string, type_error};

/// `setTimeout(fn, 0)` でも最低これだけ待つ（ブラウザの入れ子制限に合わせた下限）。
const MIN_DELAY: Duration = Duration::from_millis(1);
/// `setInterval` の最短間隔。
const MIN_INTERVAL: Duration = Duration::from_millis(4);
/// `requestAnimationFrame` の間隔（60fps）。
const FRAME: Duration = Duration::from_millis(16);
/// 1回の [`run_due`] で実行するタイマーの上限（無限ループで固まらないように）。
const MAX_PER_TICK: usize = 100;
/// 同時に持てるタイマーの上限。
const MAX_TIMERS: usize = 10_000;

fn delay_arg(args: &[JsValue], ctx: &mut Context) -> JsResult<Duration> {
    let ms = match args.get(1) {
        Some(v) => v.to_number(ctx)?,
        None => 0.0,
    };
    let ms = if ms.is_finite() && ms > 0.0 { ms } else { 0.0 };
    Ok(Duration::from_secs_f64(ms.min(2_147_483_647.0) / 1000.0))
}

fn schedule(args: &[JsValue], ctx: &mut Context, repeat: bool) -> JsResult<JsValue> {
    let Some(callback) = args
        .first()
        .and_then(JsValue::as_object)
        .filter(JsObject::is_callable)
    else {
        return Err(type_error("コールバックが関数ではありません"));
    };
    let delay = delay_arg(args, ctx)?;
    let host = Host::of(ctx);
    let mut state = host.state_mut();
    if state.timers.len() >= MAX_TIMERS {
        return Err(type_error("タイマーが多すぎます"));
    }
    let id = state.next_timer;
    state.next_timer += 1;
    let (wait, interval) = if repeat {
        let d = delay.max(MIN_INTERVAL);
        (d, Some(d))
    } else {
        (delay.max(MIN_DELAY), None)
    };
    let due = state.now + wait;
    state.timers.push(Timer {
        id,
        due,
        interval,
        callback,
        args: args.iter().skip(2).cloned().collect(),
    });
    Ok(JsValue::from(id))
}

fn set_timeout(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    schedule(args, ctx, false)
}

fn set_interval(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    schedule(args, ctx, true)
}

fn clear_timer(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = args.first().map_or(Ok(0.0), |v| v.to_number(ctx))? as u32;
    Host::of(ctx).state_mut().timers.retain(|t| t.id != id);
    Ok(JsValue::undefined())
}

fn request_animation_frame(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let Some(callback) = args
        .first()
        .and_then(JsValue::as_object)
        .filter(JsObject::is_callable)
    else {
        return Err(type_error("コールバックが関数ではありません"));
    };
    let host = Host::of(ctx);
    let mut state = host.state_mut();
    let id = state.next_timer;
    state.next_timer += 1;
    let due = state.now + FRAME;
    state.timers.push(Timer {
        id,
        due,
        interval: None,
        callback,
        // 引数は実行時の時刻（ミリ秒）にする。`run_due` が差し込む
        args: vec![JsValue::undefined()],
    });
    Ok(JsValue::from(id))
}

/// 期限が来たタイマーを期限順に実行する。実行した数を返す。
pub(crate) fn run_due(ctx: &mut Context, now: Duration) -> usize {
    let host = Host::of(ctx);
    host.state_mut().now = now;
    let mut ran = 0;
    while ran < MAX_PER_TICK {
        let next = {
            let mut state = host.state_mut();
            let Some(at) = state
                .timers
                .iter()
                .enumerate()
                .filter(|(_, t)| t.due <= now)
                .min_by_key(|(_, t)| (t.due, t.id))
                .map(|(i, _)| i)
            else {
                break;
            };
            let timer = &mut state.timers[at];
            let job = (timer.callback.clone(), timer.args.clone(), timer.id);
            match timer.interval {
                Some(interval) => timer.due = (timer.due + interval).max(now + MIN_INTERVAL),
                None => {
                    state.timers.remove(at);
                }
            }
            job
        };
        let (callback, mut args, _) = next;
        if matches!(args.as_slice(), [v] if v.is_undefined()) {
            args = vec![JsValue::from(now.as_secs_f64() * 1000.0)];
        }
        if let Err(error) = callback.call(&JsValue::undefined(), &args, ctx) {
            let message = describe_error(&error, ctx);
            host.report_error(format!("Uncaught {message}"));
        }
        let _ = ctx.run_jobs();
        ran += 1;
    }
    ran
}

/// 次のタイマーまでの時間（なければ `None`）。
pub(crate) fn next_due(host: &Host, now: Duration) -> Option<Duration> {
    host.state()
        .timers
        .iter()
        .map(|t| t.due.saturating_sub(now))
        .min()
}

fn now_ms(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(
        Host::of(ctx).state().now.as_secs_f64() * 1000.0,
    ))
}

fn eval_string(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    // 文字列のタイマー（`setTimeout("code", 0)`）は eval 相当で危険なため実行しない
    let _ = arg_string(args, 0, ctx)?;
    Ok(JsValue::undefined())
}

pub(crate) fn install(ctx: &mut Context, global: &JsObject) {
    use crate::wrap::define_method;
    define_method(ctx, global, "setTimeout", set_timeout, 2);
    define_method(ctx, global, "setInterval", set_interval, 2);
    define_method(ctx, global, "clearTimeout", clear_timer, 1);
    define_method(ctx, global, "clearInterval", clear_timer, 1);
    define_method(
        ctx,
        global,
        "requestAnimationFrame",
        request_animation_frame,
        1,
    );
    define_method(ctx, global, "cancelAnimationFrame", clear_timer, 1);
    define_method(ctx, global, "requestIdleCallback", set_timeout, 1);
    define_method(ctx, global, "cancelIdleCallback", clear_timer, 1);
    define_method(ctx, global, "__kumo_now", now_ms, 0);
    let _ = eval_string;
}
