//! `getComputedStyle`：スタイルシートが決めた値を、Content が最後にスタイルを計算した結果から答える。
//! DOM を変えた直後は次のスタイル計算まで古い値（インラインのスタイルは常に最新）。

use std::rc::Rc;

use boa_engine::{Context, JsResult, JsString, JsValue};

use crate::host::Host;
use crate::wrap::{arg_string, define_method, node_of};

/// 計算値の提供元（Content が実装する）。
pub trait StyleSource {
    /// 要素（添字）の `name`（kebab-case）の計算値。分からなければ `None`。
    fn value(&self, node: usize, name: &str) -> Option<String>;
}

pub(crate) type SharedStyleSource = Rc<dyn StyleSource>;

/// `__kumo_computed(element, name)`：幅・高さは直近のレイアウトから、ほかは提供元から。なければ空文字列。
fn computed(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let Some(id) = args.first().and_then(node_of) else {
        return Ok(JsValue::from(JsString::from("")));
    };
    let name = arg_string(args, 1, ctx)?;
    let host = Host::of(ctx);
    let state = host.state();
    let from_layout = |index: usize| {
        state
            .geometry
            .get(&id.index())
            .map(|rect| format!("{}px", rect[index]))
    };
    let value = match name.as_str() {
        "width" => from_layout(2),
        "height" => from_layout(3),
        _ => None,
    }
    .or_else(|| state.style_source.as_ref()?.value(id.index(), &name))
    // まだスタイル計算されていない要素（スクリプトが作ったばかりのもの）は、既定の値で答える
    .or_else(|| {
        let tag = state.doc.borrow().element(id)?.local.to_string();
        default_value(&tag, &name)
    })
    .unwrap_or_default();
    Ok(JsValue::from(JsString::from(value.as_str())))
}

/// UA スタイルシートが決める既定の値（`display` と `visibility` だけ）。
fn default_value(tag: &str, name: &str) -> Option<String> {
    match name {
        "visibility" => Some("visible".to_owned()),
        "display" => Some(default_display(tag).to_owned()),
        _ => None,
    }
}

fn default_display(tag: &str) -> &'static str {
    match tag {
        "address" | "article" | "aside" | "blockquote" | "body" | "center" | "dd" | "details"
        | "dialog" | "div" | "dl" | "dt" | "fieldset" | "figcaption" | "figure" | "footer"
        | "form" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "header" | "hgroup" | "hr"
        | "html" | "main" | "menu" | "nav" | "ol" | "p" | "pre" | "section" | "summary" | "ul" => {
            "block"
        }
        "li" => "list-item",
        "table" => "table",
        "caption" => "table-caption",
        "tr" => "table-row",
        "td" | "th" => "table-cell",
        "thead" | "tbody" | "tfoot" => "table-row-group",
        "img" | "input" | "button" | "select" | "textarea" | "video" | "canvas" | "iframe" => {
            "inline-block"
        }
        "head" | "script" | "style" | "meta" | "link" | "title" | "template" | "noscript" => "none",
        _ => "inline",
    }
}

pub(crate) fn install(ctx: &mut Context, global: &boa_engine::JsObject) {
    define_method(ctx, global, "__kumo_computed", computed, 2);
}
