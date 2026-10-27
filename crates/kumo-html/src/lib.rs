//! html5ever の仕様準拠ツリー構築を、Kumo のアリーナDOMへ直接出力する。
#![forbid(unsafe_code)]

mod sink;
mod styles;

use html5ever::tendril::TendrilSink;
use kumo_dom::Document;

pub use styles::{StyleSource, stylesheets};

/// UTF-8 の HTML 文字列を解析して DOM を構築する。
pub fn parse_html(input: &str) -> Document {
    html5ever::parse_document(sink::ArenaSink::default(), Default::default()).one(input)
}

/// バイト列を UTF-8 として（不正バイトは置換して）解析する。
pub fn parse_html_bytes(input: &[u8]) -> Document {
    parse_html(&String::from_utf8_lossy(input))
}
