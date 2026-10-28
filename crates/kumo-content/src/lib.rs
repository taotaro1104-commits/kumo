//! Kumo の Content プロセス：全タブのページ（取得要求・パース・スタイル・レイアウト・ラスタライズ）を持ち、
//! 共有メモリのフレームに描く。Browser プロセスはタブ・履歴・UI だけを持ち、ここへ要求を送る。
#![forbid(unsafe_code)]

mod client;
mod engine;
mod error_page;
mod fetch;
mod forms;
mod frame;
mod host;
pub mod i18n;
mod interact;
mod jobs;
mod js_glue;
mod level;
mod login;
mod page;
pub mod protocol;
mod services;
mod stats;
mod style_snapshot;
mod work;

pub use client::{ContentClient, Notify};
pub use engine::{BLANK, ContentEngine};
pub use error_page::{describe_error, describe_error_in, error_html};
pub use fetch::{FORM_CONTENT_TYPE, FetchedPage, fetch_document, image_urls, prepare};
pub use forms::{ControlClick, Submission, build_submission, click_control};
pub use frame::{FrameReader, FrameWriter};
pub use host::{Event, run, serve_stdio};
pub use i18n::Lang;
pub use level::LoadLevel;
pub use page::{Page, PageStats, escape};
pub use services::{FetchLoader, PageImages, Services};
pub use work::{Done, FetchJob, Fetched, SubFetched, SubJob, SubKind, Work};

use url::Url;

/// Content プロセスとして起動するときの最初の引数。
pub const CONTENT_PROCESS_ARG: &str = "--content-process";

/// 接続する Network プロセスのソケット名を渡す引数。
pub const NETWORK_ARG: &str = "--network";

/// 起動引数から Network プロセスのソケット名を読む。
pub fn network_socket(args: &[String]) -> Option<&str> {
    let at = args.iter().position(|a| a == NETWORK_ARG)?;
    args.get(at + 1).map(String::as_str)
}

/// `#` 以降を除いて同じ文書か。
pub fn same_document(a: &Url, b: &Url) -> bool {
    let strip = |u: &Url| {
        let mut u = u.clone();
        u.set_fragment(None);
        u
    };
    strip(a) == strip(b)
}

/// URL の `#id` が指す要素の y 座標（パーセントエンコードを戻して照合する）。
pub fn anchor_y(page: &Page, url: &Url) -> Option<f32> {
    let fragment = url.fragment().filter(|f| !f.is_empty())?;
    let decoded = percent_encoding::percent_decode_str(fragment).decode_utf8_lossy();
    page.list
        .anchor_y(&decoded)
        .or_else(|| page.list.anchor_y(fragment))
}
