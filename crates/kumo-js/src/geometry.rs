//! 幾何情報（`getBoundingClientRect`・`offsetWidth` など）。Content が最後にレイアウトした結果の表を
//! [`crate::JsRuntime::set_geometry`] で受け取って答える。DOM を変えた直後は、次のレイアウトまで古い値になる。
//! 表にない要素（`display: none` など）は 0。

use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsObject, JsResult, JsString, JsValue};

use crate::host::Host;
use crate::wrap::{define_accessor, define_method, this_node};

/// 文書座標の境界ボックス（x, y, 幅, 高さ）。
pub(crate) type Rect = [f32; 4];

/// `this` の要素の境界ボックス。`<html>` は表示領域の大きさ。
fn rect_of(this: &JsValue, ctx: &mut Context) -> JsResult<Rect> {
    let id = this_node(this)?;
    let host = Host::of(ctx);
    let state = host.state();
    let is_root = state
        .doc
        .borrow()
        .element(id)
        .is_some_and(|e| e.is_html("html"));
    if is_root {
        return Ok([0.0, 0.0, state.viewport.0, state.viewport.1]);
    }
    Ok(state.geometry.get(&id.index()).copied().unwrap_or_default())
}

fn rect_object(ctx: &mut Context, [x, y, w, h]: Rect, scroll_y: f32) -> JsResult<JsObject> {
    let (x, y, w, h) = (
        f64::from(x),
        f64::from(y - scroll_y),
        f64::from(w),
        f64::from(h),
    );
    let rect = JsObject::with_object_proto(ctx.intrinsics());
    let fields = [
        ("x", x),
        ("y", y),
        ("width", w),
        ("height", h),
        ("top", y),
        ("left", x),
        ("right", x + w),
        ("bottom", y + h),
    ];
    for (key, value) in fields {
        rect.set(JsString::from(key), value, false, ctx)?;
    }
    Ok(rect)
}

fn bounding_rect(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let rect = rect_of(this, ctx)?;
    let scroll = Host::of(ctx).state().scroll_y;
    Ok(rect_object(ctx, rect, scroll)?.into())
}

fn client_rects(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let rect = rect_of(this, ctx)?;
    if rect[2] <= 0.0 && rect[3] <= 0.0 {
        return Ok(JsArray::new(ctx)?.into());
    }
    let scroll = Host::of(ctx).state().scroll_y;
    let object = rect_object(ctx, rect, scroll)?;
    Ok(JsArray::from_iter([JsValue::from(object)], ctx).into())
}

macro_rules! dimension {
    ($name:ident, $index:expr) => {
        fn $name(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
            Ok(JsValue::from(rect_of(this, ctx)?[$index].round()))
        }
    };
}

dimension!(width, 2);
dimension!(height, 3);
dimension!(left, 0);
dimension!(top, 1);

fn zero(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(0))
}

fn noop(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::undefined())
}

pub(crate) fn install(ctx: &mut Context, element: &JsObject) {
    define_method(ctx, element, "getBoundingClientRect", bounding_rect, 0);
    define_method(ctx, element, "getClientRects", client_rects, 0);
    for name in ["offsetWidth", "clientWidth", "scrollWidth"] {
        define_accessor(ctx, element, name, Some(width), None);
    }
    for name in ["offsetHeight", "clientHeight", "scrollHeight"] {
        define_accessor(ctx, element, name, Some(height), None);
    }
    define_accessor(ctx, element, "offsetLeft", Some(left), None);
    define_accessor(ctx, element, "offsetTop", Some(top), None);
    // スクロール位置は要素ごとには持たない（ページ全体のスクロールは `window.scrollY`）
    for name in ["scrollTop", "scrollLeft"] {
        define_accessor(ctx, element, name, Some(zero), Some(noop));
    }
}
