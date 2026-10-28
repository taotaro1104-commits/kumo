//! JS のテスト用ヘルパー：HTML を解析し、ランタイムを作って式を評価する。
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use kumo_dom::Document;
use kumo_js::JsRuntime;
use url::Url;

pub const PAGE: &str = "<!DOCTYPE html><html><head><title>Test  Page</title></head><body>\
    <div id=a class='box big' data-foo-bar=1><p id=p>hello</p><span id=s>x</span></div>\
    <ul id=list><li>1</li><li>2</li><li>3</li></ul>\
    <input id=in value=v><a id=link href='/next?x=1'>go</a></body></html>";

/// HTML から作ったランタイムと、共有している DOM。
pub fn runtime(html: &str) -> (JsRuntime, Rc<RefCell<Document>>) {
    let doc = Rc::new(RefCell::new(kumo_html::parse_html(html)));
    let url = Url::parse("https://example.com/dir/page.html?q=1#top").unwrap();
    let js = JsRuntime::new(doc.clone(), url, (1000.0, 700.0)).expect("ランタイムを作れる");
    (js, doc)
}

/// 式を評価して文字列にする。例外ならパニック。
pub fn eval(js: &mut JsRuntime, source: &str) -> String {
    js.eval_to_string(source)
        .unwrap_or_else(|e| panic!("{source}\n→ 例外: {e}"))
}

/// `PAGE` で式を評価する。
pub fn run(source: &str) -> String {
    let (mut js, _) = runtime(PAGE);
    eval(&mut js, source)
}

/// 時刻を `ms` ミリ秒進めてタイマーを実行する。
pub fn advance(js: &mut JsRuntime, ms: u64) -> usize {
    js.tick(Duration::from_millis(ms))
}
