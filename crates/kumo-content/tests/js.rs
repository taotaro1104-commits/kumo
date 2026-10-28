//! JS を使うページを Content で動かす結合テスト：スクリプトの実行、DOM の変更の反映、イベント、タイマー、fetch。

mod common;

use std::time::Duration;

use common::{Site, engine, pump};
use kumo_content::ContentEngine;
use kumo_content::protocol::{FromContent, ToContent};

fn load(url: &url::Url) -> ToContent {
    ToContent::Load {
        tab: 1,
        nav: 1,
        url: url.to_string(),
        post_body: None,
    }
}

/// ページを作って読み込み、表示中にする。
fn open(site: &Site, files: &[(&str, &str)], page: &str) -> ContentEngine {
    for (name, body) in files {
        std::fs::write(site.dir.join(name), body).unwrap();
    }
    let mut e = engine();
    e.handle(ToContent::Viewport {
        width: 400,
        height: 300,
    });
    e.handle(ToContent::Show { tab: 1 });
    e.handle(load(&site.url(page)));
    pump(&mut e, true);
    e
}

/// タブ `tab` の `id` 属性が `id` の要素のテキスト。
fn text_of(e: &ContentEngine, tab: u64, id: &str) -> String {
    let page = e.page(tab).unwrap();
    let doc = page.document();
    let node = doc
        .descendants(doc.root())
        .find(|&n| doc.attr(n, "id") == Some(id))
        .unwrap_or_else(|| panic!("#{id}"));
    doc.text_content(node)
}

fn text(e: &ContentEngine) -> String {
    text_of(e, 1, "out")
}

fn navigations(out: &[FromContent]) -> Vec<String> {
    out.iter()
        .filter_map(|m| match m {
            FromContent::Navigate { url, .. } => Some(url.clone()),
            _ => None,
        })
        .collect()
}

/// 時計を `ms` に進めてタイマーを動かし、取得の仕事も片づける。
fn advance(e: &mut ContentEngine, ms: u64) -> Vec<FromContent> {
    e.set_time(Duration::from_millis(ms));
    e.tick();
    pump(e, true)
}

#[test]
fn scripts_run_in_order_and_change_the_page_before_first_layout() {
    let site = Site::new("js-run");
    let e = open(
        &site,
        &[
            ("lib.js", "window.shared = 'from lib';"),
            (
                "p.html",
                "<html><head><title>Old</title><script src=lib.js></script></head><body>\
                 <p id=out>before</p><script>\
                   document.getElementById('out').textContent = shared + ' & inline';\
                   document.title = 'New';\
                 </script></body></html>",
            ),
        ],
        "p.html",
    );
    assert_eq!(text(&e), "from lib & inline");
    assert_eq!(
        e.page(1).unwrap().title(),
        "New",
        "スクリプトが変えた題名が使われる"
    );
    assert_eq!(e.stats().js_tabs, 1);
}

#[test]
fn dom_changes_are_painted_and_loaded_event_carries_the_new_title() {
    let site = Site::new("js-paint");
    let mut e = open(
        &site,
        &[(
            "p.html",
            "<html><head><title>T</title><style>body{margin:0}</style></head><body>\
             <div id=box style='width:50px;height:50px;background:#00ff00'></div>\
             <script>document.getElementById('box').style.background = '#ff0000';</script></body></html>",
        )],
        "p.html",
    );
    e.handle(ToContent::FrameDone);
    e.handle(ToContent::Show { tab: 1 });
    let out = pump(&mut e, true);
    let frame = out.iter().find_map(|m| match m {
        FromContent::Frame { path, width, .. } => Some((path.clone(), *width)),
        _ => None,
    });
    let (path, width) = frame.expect("フレームが描かれる");
    let mut reader = kumo_content::FrameReader::default();
    reader.attach(&path, width, 300).unwrap();
    assert_eq!(
        reader.pixels().unwrap()[25 * width as usize + 25],
        0x00ff_0000,
        "スクリプトが変えた色"
    );
}

#[test]
fn click_handlers_run_and_can_cancel_navigation() {
    let site = Site::new("js-click");
    let mut e = open(
        &site,
        &[
            (
                "p.html",
                "<html><body style='margin:0;font-size:20px'>\
                 <a id=go href='other.html'>go</a> <a id=stay href='other.html'>stay</a>\
                 <script>\
                   document.getElementById('stay').addEventListener('click', (e) => { e.preventDefault(); document.title = 'stopped'; });\
                   document.getElementById('go').onclick = () => { document.title = 'ran'; };\
                 </script></body></html>",
            ),
            ("other.html", "<p>x</p>"),
        ],
        "p.html",
    );
    e.take_output();
    let links = e.page(1).unwrap().list.links.clone();
    let center = |href_index: usize| {
        (
            links[href_index].rect.x + 2.0,
            links[href_index].rect.y + 2.0,
        )
    };
    // 1つ目のリンク：ハンドラが走り、既定の移動も行われる
    let (x, y) = center(0);
    e.handle(ToContent::Click {
        tab: 1,
        x,
        y,
        new_tab: false,
    });
    let out = e.take_output();
    assert_eq!(
        navigations(&out).len(),
        1,
        "preventDefault しなければ移動する"
    );
    assert!(out.contains(&FromContent::TitleChanged {
        tab: 1,
        title: "ran".into()
    }));
    // 2つ目のリンク：preventDefault で移動しない
    let (x, y) = center(1);
    e.handle(ToContent::Click {
        tab: 1,
        x,
        y,
        new_tab: false,
    });
    let out = e.take_output();
    assert!(
        navigations(&out).is_empty(),
        "preventDefault されたら移動しない"
    );
    assert!(out.contains(&FromContent::TitleChanged {
        tab: 1,
        title: "stopped".into()
    }));
}

#[test]
fn timers_fetch_and_dynamic_scripts_flow_through_the_engine() {
    let site = Site::new("js-async");
    let mut e = open(
        &site,
        &[
            ("data.json", "{\"n\": 7}"),
            (
                "dyn.js",
                "document.getElementById('log').textContent += ' dyn';",
            ),
            (
                "p.html",
                "<html><body><p id=log>start</p><script>\
                   setTimeout(() => { document.getElementById('log').textContent += ' t100'; }, 100);\
                   fetch('data.json').then((r) => r.json()).then((d) => { document.getElementById('log').textContent += ' fetch' + d.n; });\
                   const s = document.createElement('script'); s.src = 'dyn.js'; document.body.appendChild(s);\
                 </script></body></html>",
            ),
        ],
        "p.html",
    );
    // 読み込み直後の pump で、fetch と動的スクリプトの取得が済んでいる
    pump(&mut e, true);
    let log = text_of(&e, 1, "log");
    assert!(
        log.starts_with("start") && log.contains("fetch7") && log.contains("dyn"),
        "{log}"
    );
    assert_eq!(
        e.next_wake(),
        Some(Duration::from_millis(100)),
        "次のタイマーまで"
    );
    advance(&mut e, 99);
    assert!(!text_of(&e, 1, "log").contains("t100"));
    advance(&mut e, 100);
    assert!(text_of(&e, 1, "log").contains("t100"));
    assert_eq!(e.next_wake(), None, "タイマーは尽きた");
}

#[test]
fn form_events_can_cancel_submission_and_report_changes() {
    let site = Site::new("js-form");
    let mut e = open(
        &site,
        &[
            ("result.html", "<p>done</p>"),
            (
                "p.html",
                "<html><body style='margin:0'><form id=f action=result.html>\
                 <input id=q name=q><input type=submit value=go></form><p id=log></p><script>\
                   const log = (s) => { document.getElementById('log').textContent += s + ';'; };\
                   const q = document.getElementById('q');\
                   q.addEventListener('input', () => log('input:' + q.value));\
                   q.addEventListener('change', () => log('change:' + q.value));\
                   q.addEventListener('keydown', (e) => { if (e.key === 'x') e.preventDefault(); });\
                   document.getElementById('f').addEventListener('submit', (e) => { log('submit'); if (q.value === 'stop') e.preventDefault(); });\
                 </script></body></html>",
            ),
        ],
        "p.html",
    );
    let control = e.page(1).unwrap().list.controls[0].0;
    let (x, y) = (control.x + 2.0, control.y + 2.0);
    e.handle(ToContent::Click {
        tab: 1,
        x,
        y,
        new_tab: false,
    });
    for t in ["a", "x", "b"] {
        e.handle(ToContent::Key {
            tab: 1,
            key: kumo_content::protocol::FieldKey::Text(t.into()),
        });
    }
    assert!(
        text_of(&e, 1, "log").contains("input:a;input:ab;"),
        "keydown で止めた x は入力されない"
    );
    e.take_output();
    e.handle(ToContent::Key {
        tab: 1,
        key: kumo_content::protocol::FieldKey::Enter,
    });
    assert_eq!(navigations(&e.take_output()).len(), 1);
    e.handle(ToContent::Key {
        tab: 1,
        key: kumo_content::protocol::FieldKey::Blur,
    });
    assert!(text_of(&e, 1, "log").contains("submit;"));
    // 値を stop にして送信 → 止められる
    let control = e.page(1).unwrap().list.controls[0].0;
    e.handle(ToContent::Click {
        tab: 1,
        x: control.x + 2.0,
        y: control.y + 2.0,
        new_tab: false,
    });
    for _ in 0..2 {
        e.handle(ToContent::Key {
            tab: 1,
            key: kumo_content::protocol::FieldKey::Backspace,
        });
    }
    for t in ["s", "t", "o", "p"] {
        e.handle(ToContent::Key {
            tab: 1,
            key: kumo_content::protocol::FieldKey::Text(t.into()),
        });
    }
    e.take_output();
    e.handle(ToContent::Key {
        tab: 1,
        key: kumo_content::protocol::FieldKey::Enter,
    });
    assert!(
        navigations(&e.take_output()).is_empty(),
        "submit を止めたら送らない"
    );
}

#[test]
fn stale_results_are_dropped_and_script_errors_are_counted() {
    let site = Site::new("js-stale");
    for (name, body) in [
        ("data.json", "{}"),
        (
            "p.html",
            "<html><body><p id=log>x</p><script>               fetch('data.json').then(() => { document.getElementById('log').textContent = 'late'; });               null.boom();             </script><script>document.getElementById('log').textContent = 'second ran';</script></body></html>",
        ),
        ("q.html", "<p id=other>q</p>"),
    ] {
        std::fs::write(site.dir.join(name), body).unwrap();
    }
    let mut e = engine();
    e.handle(ToContent::Show { tab: 1 });
    e.handle(load(&site.url("p.html")));
    let fetcher = e.fetcher();
    // 文書の取得だけ済ませ、`fetch` の取得は保留にしておく
    for job in e.take_jobs() {
        e.done(job.run(&*fetcher));
    }
    assert_eq!(
        text_of(&e, 1, "log"),
        "second ran",
        "例外の後のスクリプトも実行される"
    );
    assert_eq!(e.stats().js_errors, 1);
    let pending = e.take_jobs();
    assert_eq!(pending.len(), 1, "fetch の取得が保留中");
    // 取得の結果を受ける前に別のページへ移ると、結果は捨てられる
    e.handle(ToContent::Load {
        tab: 1,
        nav: 2,
        url: site.url("q.html").to_string(),
        post_body: None,
    });
    for job in pending {
        e.done(job.run(&*fetcher));
    }
    pump(&mut e, true);
    assert_eq!(text_of(&e, 1, "other"), "q");
}

#[test]
fn background_tabs_are_throttled_and_pages_without_scripts_have_no_runtime() {
    let site = Site::new("js-bg");
    let mut e = open(
        &site,
        &[
            ("plain.html", "<p>no js</p>"),
            (
                "p.html",
                "<html><body><p id=n>0</p><script>let c = 0; setInterval(() => { document.getElementById('n').textContent = ++c; }, 10);</script></body></html>",
            ),
        ],
        "plain.html",
    );
    assert_eq!(
        e.stats().js_tabs,
        0,
        "スクリプトのないページはランタイムを作らない"
    );
    e.handle(ToContent::Load {
        tab: 2,
        nav: 1,
        url: site.url("p.html").to_string(),
        post_body: None,
    });
    pump(&mut e, true);
    assert_eq!(e.stats().js_tabs, 1);
    // タブ 1 が表示中。タブ 2 は背景なので 1 秒に 1 回だけ動く
    let count = |e: &ContentEngine| text_of(e, 2, "n");
    for ms in (10..=900).step_by(10) {
        advance(&mut e, ms);
    }
    assert_eq!(count(&e), "0", "1 秒たつまで背景のタブのタイマーは動かない");
    advance(&mut e, 1_000);
    advance(&mut e, 1_020);
    assert_ne!(count(&e), "0");
}
