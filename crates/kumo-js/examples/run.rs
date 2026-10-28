//! 診断用：HTML ファイルを読み込み、指定したスクリプトを順に実行して、例外とコンソール出力を表示する。
//! 使い方: cargo run -p kumo-js --example run -- page.html script1.js script2.js ...
//! 例外は `new Error().stack` 付きで表示する（Boa は例外の位置をスタックに持つ）。

use std::cell::RefCell;
use std::rc::Rc;

use kumo_js::JsRuntime;
use url::Url;

fn main() {
    // Boa の解析と実行は深く再帰するので、大きなスタックのスレッドで動かす
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

fn run() {
    let mut args = std::env::args().skip(1);
    let html = std::fs::read_to_string(args.next().expect("HTML ファイル")).expect("読める");
    let doc = Rc::new(RefCell::new(kumo_html::parse_html(&html)));
    let url = Url::parse("https://example.com/page.html").unwrap();
    let mut js = JsRuntime::new(doc, url, (1280.0, 800.0)).expect("ランタイム");
    for path in args {
        if path == "--" {
            // ここまでを読み込みとみなし、読み込みのイベントを送って 3 秒ぶんタイマーを動かす
            js.set_ready_state("interactive");
            js.fire_dom_content_loaded();
            js.set_ready_state("complete");
            js.fire_load();
            for ms in (0..3000).step_by(16) {
                let now = std::time::Duration::from_millis(ms);
                js.set_time(now);
                js.tick(now);
            }
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("スクリプトを読める");
        // 本文は JSON 風にエスケープして eval に渡し、捕まえた例外のスタックを表示する
        let escaped = js_string_literal(&source);
        let wrapped = format!(
            "try {{ (0, eval)({escaped}); }} catch (e) {{ __kumo_log('error', 'CAUGHT ' + e.name + ': ' + e.message + ' STACK=' + e.stack); }}"
        );
        let result = js.run_script(&wrapped, &path, None);
        println!("{path}: {result:?}");
    }
    for line in js.take_console() {
        println!("console {line}");
    }
    for line in js.take_errors() {
        println!("error {line}");
    }
}

/// JS の文字列リテラルにする（改行・引用符・バックスラッシュをエスケープ）。
fn js_string_literal(source: &str) -> String {
    let mut out = String::from("\"");
    for c in source.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
