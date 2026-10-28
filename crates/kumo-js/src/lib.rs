//! Kumo の JavaScript：Boa エンジンに DOM・イベント・タイマー・`fetch` を結びつける。
//!
//! `JsRuntime` が公開 API。DOM は Content が持つ `Rc<RefCell<Document>>` を共有し、JS からの変更は
//! 「変わった」印を立てて Content に伝える（再スタイル・再レイアウトは Content が行う）。

mod bootstrap;
mod class_list;
mod dispatch;
mod document_api;
mod document_extras;
mod element_api;
mod element_html;
mod events;
mod host;
mod natives;
mod node_api;
mod parent_node;
mod runtime;
mod scripts;
mod style_api;
mod timers;
mod wrap;

pub use host::{FetchOutcome, FetchRequest, may_reach};
pub use runtime::{
    InitValue, JsRuntime, SCRIPT_BYTES_BUDGET, SCRIPT_MAX_BYTES, SCRIPT_TIME_BUDGET, SharedDocument,
};
pub use scripts::{ScriptInfo, needs_js, script_info, scripts_in_order};
