//! `JsRuntime` の単体テスト（panic の保護と、スクリプトの予算）。

use super::*;
use crate::limits::{SCRIPT_BYTES_BUDGET, SCRIPT_MAX_BYTES, SCRIPT_TIME_BUDGET};

fn runtime() -> JsRuntime {
    let doc = Rc::new(RefCell::new(kumo_html::parse_html("<p id=p>x</p>")));
    JsRuntime::new(
        doc,
        Url::parse("https://example.com/").unwrap(),
        (800.0, 600.0),
    )
    .unwrap()
}

fn explode(_: &JsValue, _: &[JsValue], _: &mut Context) -> boa_engine::JsResult<JsValue> {
    panic!("バインディングの不具合");
}

#[test]
fn a_native_panic_is_caught_reported_and_eventually_stops_the_page_script() {
    let mut js = runtime();
    let global = js.ctx.global_object();
    crate::wrap::define_method(&mut js.ctx, &global, "explode", explode, 0);
    for round in 1..=MAX_PANICS {
        let result = js.run_script("explode()", "bad.js", None);
        assert!(result.is_err(), "{round} 回目");
    }
    assert!(js.is_broken());
    let errors = js.take_errors();
    assert_eq!(errors.len(), MAX_PANICS as usize);
    assert!(errors[0].contains("内部エラー") && errors[0].contains("バインディングの不具合"));
    // 止まった後は何も実行しない（タブは動き続ける）
    assert!(js.run_script("1 + 1", "ok.js", None).is_err());
    assert_eq!(js.tick(Duration::from_secs(1)), 0);
    assert!(
        js.dispatch_event(NodeId::from_index(1), "Event", "click", &[]),
        "止まったら既定の動作は進める"
    );
}

#[test]
fn one_panic_does_not_stop_later_scripts() {
    let mut js = runtime();
    let global = js.ctx.global_object();
    crate::wrap::define_method(&mut js.ctx, &global, "explode", explode, 0);
    assert!(js.run_script("explode()", "bad.js", None).is_err());
    assert!(!js.is_broken());
    assert_eq!(
        js.eval_to_string("document.getElementById('p').textContent"),
        Ok("x".to_owned())
    );
}

#[test]
fn scripts_over_the_size_or_total_budget_are_skipped_without_errors() {
    let mut js = runtime();
    let huge = format!("/*{}*/ globalThis.big = 1;", " ".repeat(SCRIPT_MAX_BYTES));
    let err = js.run_script(&huge, "huge.js", None).unwrap_err();
    assert!(err.contains("大きすぎる"), "{err}");
    // 合計の予算：小さなスクリプトを重ねて使い切る
    let chunk = format!("/*{}*/", " ".repeat(SCRIPT_MAX_BYTES - 100));
    let allowed = SCRIPT_BYTES_BUDGET / chunk.len();
    for _ in 0..allowed {
        js.run_script(&chunk, "chunk.js", None).unwrap();
    }
    let err = js.run_script(&chunk, "late.js", None).unwrap_err();
    assert!(err.contains("大きさの予算"), "{err}");
    assert!(
        js.take_errors().is_empty(),
        "予算で飛ばしたものは例外として数えない"
    );
    assert!(js.take_console().iter().any(|l| l.starts_with("[budget]")));
    assert!(
        js.run_script("1", "tiny.js", None).is_ok(),
        "予算の範囲なら小さいものは動く"
    );
    let (bytes, _) = js.script_usage();
    assert!(bytes > SCRIPT_BYTES_BUDGET - SCRIPT_MAX_BYTES);
}

#[test]
fn limits_can_be_changed_per_runtime() {
    let mut js = runtime();
    js.set_limits(ScriptLimits {
        max_bytes: 10,
        ..ScriptLimits::default()
    });
    let err = js
        .run_script("/* 0123456789 */ 1", "big.js", None)
        .unwrap_err();
    assert!(err.contains("大きすぎる"), "{err}");
    assert!(js.run_script("1", "ok.js", None).is_ok());
    js.set_limits(ScriptLimits {
        time_budget: Duration::ZERO,
        ..ScriptLimits::default()
    });
    assert!(
        js.run_script("1", "late.js", None)
            .unwrap_err()
            .contains("実行時間")
    );
}

#[test]
fn running_out_of_time_budget_stops_later_scripts() {
    let mut js = runtime();
    js.script_time = SCRIPT_TIME_BUDGET;
    let err = js.run_script("1", "slow.js", None).unwrap_err();
    assert!(err.contains("実行時間の予算"), "{err}");
}
