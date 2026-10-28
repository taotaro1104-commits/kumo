mod common;

use common::{PAGE, advance, eval, runtime};
use kumo_js::FetchOutcome;

fn node(
    js_doc: &std::rc::Rc<std::cell::RefCell<kumo_dom::Document>>,
    id: &str,
) -> kumo_dom::NodeId {
    let doc = js_doc.borrow();
    doc.descendants(doc.root())
        .find(|&n| doc.attr(n, "id") == Some(id))
        .unwrap_or_else(|| panic!("#{id}"))
}

#[test]
fn events_capture_target_bubble_in_order() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "globalThis.log = [];
         const a = document.getElementById('a'), p = document.getElementById('p');
         window.addEventListener('click', () => log.push('window-bubble'));
         window.addEventListener('click', () => log.push('window-capture'), true);
         document.addEventListener('click', () => log.push('doc'));
         a.addEventListener('click', () => log.push('a-bubble'));
         a.addEventListener('click', () => log.push('a-capture'), { capture: true });
         p.addEventListener('click', (e) => log.push('p:' + e.eventPhase + ':' + (e.target === p) + ':' + (e.currentTarget === p)));
         p.onclick = () => log.push('p-onclick');
         p.click();",
    );
    assert_eq!(
        eval(&mut js, "log.join(' ')"),
        "window-capture a-capture p:2:true:true p-onclick a-bubble doc window-bubble"
    );
}

#[test]
fn stop_propagation_prevent_default_once_and_remove() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "globalThis.log = [];
         const a = document.getElementById('a'), p = document.getElementById('p');
         a.addEventListener('click', () => log.push('a'));
         const h = () => log.push('removed');
         p.addEventListener('click', h); p.removeEventListener('click', h);
         p.addEventListener('click', (e) => { log.push('first'); e.stopImmediatePropagation(); });
         p.addEventListener('click', () => log.push('second'));
         p.click();
         const s = document.getElementById('s');
         s.addEventListener('click', () => log.push('once'), { once: true });
         s.addEventListener('click', (e) => { e.stopPropagation(); });
         s.click(); s.click();
         globalThis.res = [];
         const b = document.getElementById('link');
         b.addEventListener('click', (e) => { res.push(e.cancelable); e.preventDefault(); res.push(e.defaultPrevented); });
         b.click();",
    );
    assert_eq!(
        eval(&mut js, "log.join(' ')"),
        "first once",
        "stopImmediate で後続を止め、once は1回、stopPropagation で a に届かない"
    );
    assert_eq!(eval(&mut js, "res.join()"), "true,true");
    assert_eq!(
        js.take_navigation(),
        None,
        "preventDefault されたリンクは移動しない"
    );
    eval(
        &mut js,
        "document.getElementById('link').onclick = null; document.getElementById('link').removeAttribute('onclick')",
    );
}

#[test]
fn link_click_requests_navigation_and_checkbox_toggles() {
    let (mut js, doc) =
        runtime("<a id=l href='/go?x=1'><b id=inner>t</b></a><input id=c type=checkbox>");
    let inner = node(&doc, "inner");
    assert!(
        js.dispatch_event(inner, "MouseEvent", "click", &[]),
        "止められなければ true"
    );
    js.default_click_action(inner);
    assert_eq!(
        js.take_navigation().as_deref(),
        Some("https://example.com/go?x=1")
    );
    eval(&mut js, "document.getElementById('c').click()");
    assert!(doc.borrow().attr(node(&doc, "c"), "checked").is_some());
    eval(&mut js, "document.getElementById('c').click()");
    assert!(doc.borrow().attr(node(&doc, "c"), "checked").is_none());
}

#[test]
fn custom_events_and_subclassing() {
    let (mut js, _) = runtime(PAGE);
    assert_eq!(
        eval(
            &mut js,
            "const t = new EventTarget(); let got = [];
             t.addEventListener('ping', (e) => got.push(e.detail.n + ':' + (e instanceof CustomEvent) + ':' + (e.target === t)));
             t.dispatchEvent(new CustomEvent('ping', { detail: { n: 7 } }));
             class Bus extends EventTarget { emit() { this.dispatchEvent(new Event('x')); } }
             const b = new Bus(); b.addEventListener('x', () => got.push('bus')); b.emit();
             const ev = document.createEvent('Event'); ev.initEvent('custom', true, true);
             document.addEventListener('custom', () => got.push('doc'));
             document.getElementById('p').dispatchEvent(ev);
             got.join()"
        ),
        "7:true:true,bus,doc"
    );
    assert_eq!(
        eval(
            &mut js,
            "const k = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }); k.key + k.bubbles + (k instanceof Event)"
        ),
        "Entertruetrue"
    );
    assert_eq!(
        eval(&mut js, "try { new Node() } catch (e) { e.message }"),
        "Illegal constructor"
    );
    assert_eq!(
        eval(
            &mut js,
            "document.body instanceof HTMLElement && document.getElementById('in') instanceof HTMLInputElement && !(document.body instanceof HTMLInputElement)"
        ),
        "true"
    );
}

#[test]
fn listener_errors_are_reported_and_do_not_stop_others() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "globalThis.n = 0; const p = document.getElementById('p');
         p.addEventListener('click', () => { throw new RangeError('boom'); });
         p.addEventListener('click', () => { n++; });
         p.click();",
    );
    assert_eq!(eval(&mut js, "n"), "1");
    let errors = js.take_errors();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].starts_with("Uncaught RangeError: boom [at "),
        "例外の位置が付く: {}",
        errors[0]
    );
}

#[test]
fn timers_run_in_order_with_arguments_and_clear() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "globalThis.log = [];
         setTimeout((a, b) => log.push('t30:' + a + b), 30, 'x', 'y');
         setTimeout(() => log.push('t10'), 10);
         setTimeout(() => log.push('t0'));
         const cancelled = setTimeout(() => log.push('never'), 5); clearTimeout(cancelled);
         let n = 0; const iv = setInterval(() => { log.push('i' + (++n)); if (n == 3) clearInterval(iv); }, 12);
         requestAnimationFrame((t) => log.push('raf:' + (typeof t)));
         Promise.resolve().then(() => log.push('micro'));",
    );
    assert_eq!(
        eval(&mut js, "log.join()"),
        "micro",
        "マイクロタスクは先に走る"
    );
    assert_eq!(
        js.next_timer_in(std::time::Duration::ZERO),
        Some(std::time::Duration::from_millis(1))
    );
    advance(&mut js, 1);
    advance(&mut js, 10);
    advance(&mut js, 12);
    advance(&mut js, 16);
    advance(&mut js, 24);
    advance(&mut js, 30);
    advance(&mut js, 36);
    assert_eq!(
        eval(&mut js, "log.join()"),
        "micro,t0,t10,i1,raf:number,i2,t30:xy,i3"
    );
    assert_eq!(
        js.next_timer_in(std::time::Duration::from_millis(36)),
        None,
        "全部終わった"
    );
    assert!(js.take_errors().is_empty());
}

#[test]
fn timers_cannot_hang_the_page() {
    let (mut js, _) = runtime(PAGE);
    eval(&mut js, "globalThis.n = 0; setInterval(() => { n++; }, 0)");
    let ran = advance(&mut js, 1000);
    assert!(ran <= 100, "1回の tick で実行する数に上限がある: {ran}");
    // 文字列のコールバックは関数にして実行される
    eval(&mut js, "globalThis.m = 0; setTimeout('m = 5', 0)");
    advance(&mut js, 2000);
    assert_eq!(eval(&mut js, "m"), "5");
}

#[test]
fn fetch_resolves_through_the_host() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "globalThis.out = [];
         fetch('/api/data?x=1', { method: 'post', body: 'a=1', headers: { 'Content-Type': 'text/plain' } })
           .then((r) => { out.push(r.status + ':' + r.ok + ':' + r.headers.get('content-type')); return r.json(); })
           .then((d) => out.push('json:' + d.v))
           .catch((e) => out.push('error:' + e.message));
         fetch('ftp://nope/').catch((e) => out.push('bad:' + e.name));
         fetch('/fail').catch((e) => out.push('fail:' + (e instanceof TypeError)));",
    );
    let requests = js.take_fetches();
    assert_eq!(
        requests.len(),
        2,
        "対応していないスキームは Network に出さない"
    );
    assert_eq!(requests[0].url, "https://example.com/api/data?x=1");
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].body.as_deref(), Some(&b"a=1"[..]));
    assert_eq!(requests[0].content_type.as_deref(), Some("text/plain"));
    js.fetch_done(
        requests[0].id,
        Ok(FetchOutcome {
            status: 201,
            url: "https://example.com/api/data?x=1".into(),
            content_type: Some("application/json".into()),
            body: br#"{"v": 42}"#.to_vec(),
        }),
    );
    js.fetch_done(requests[1].id, Err("connection refused".into()));
    assert_eq!(
        eval(&mut js, "out.join(' | ')"),
        "bad:TypeError | 201:true:application/json | json:42 | fail:true"
    );
}

#[test]
fn xhr_and_abort_work_on_top_of_fetch() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "globalThis.out = [];
         const x = new XMLHttpRequest();
         x.onreadystatechange = () => out.push('rs' + x.readyState);
         x.addEventListener('load', () => out.push('load:' + x.status + ':' + x.responseText));
         x.open('GET', '/x'); x.send();
         const c = new AbortController(); c.abort();
         fetch('/y', { signal: c.signal }).catch((e) => out.push(e.name));",
    );
    let requests = js.take_fetches();
    assert_eq!(requests.len(), 1, "中止済みの fetch は出さない");
    js.fetch_done(
        requests[0].id,
        Ok(FetchOutcome {
            status: 200,
            url: "https://example.com/x".into(),
            content_type: None,
            body: b"ok".to_vec(),
        }),
    );
    assert_eq!(
        eval(&mut js, "out.join(' ')"),
        "rs1 AbortError rs2 rs3 rs4 load:200:ok"
    );
}

#[test]
fn scripts_lifecycle_and_errors() {
    let (mut js, doc) = runtime(
        "<script id=s1>var a = 1;</script><script id=s2 src='/lib.js'></script><script id=s3 type='application/json'>{}</script>\
         <script id=s4 type=module>x</script><script id=s5>  </script>",
    );
    let info = |id: &str| js.script_info(node(&doc, id));
    assert_eq!(info("s1").unwrap().text, "var a = 1;");
    assert_eq!(
        info("s2").unwrap().src.as_deref(),
        Some("https://example.com/lib.js")
    );
    assert!(info("s3").is_none() && info("s4").is_none() && info("s5").is_none());

    let s1 = node(&doc, "s1");
    js.run_script(
        "globalThis.cs = document.currentScript.id",
        "inline",
        Some(s1),
    )
    .unwrap();
    assert_eq!(
        eval(&mut js, "cs + typeof document.currentScript"),
        "s1object"
    );
    let error = js.run_script("nope()", "bad.js", None).unwrap_err();
    assert!(error.starts_with("bad.js: ReferenceError"), "{error}");
    assert_eq!(js.take_errors().len(), 1);

    js.set_ready_state("interactive");
    eval(
        &mut js,
        "globalThis.log = [];
         document.addEventListener('DOMContentLoaded', () => log.push('dcl:' + document.readyState));
         window.addEventListener('load', () => log.push('load'));
         document.addEventListener('readystatechange', () => log.push('rsc:' + document.readyState));",
    );
    js.fire_dom_content_loaded();
    js.set_ready_state("complete");
    js.fire_load();
    assert_eq!(
        eval(&mut js, "log.join()"),
        "dcl:interactive,rsc:complete,load"
    );

    // 動的に挿入された script は実行待ちに入る
    eval(
        &mut js,
        "const e = document.createElement('script'); e.src = '/dyn.js'; document.head.appendChild(e);",
    );
    assert_eq!(js.take_new_scripts().len(), 1);
}

#[test]
fn inline_event_attributes_run_with_this_and_event() {
    let (mut js, doc) = runtime(
        "<button id=b onclick=\"window.hit = (window.hit || '') + this.id + ':' + event.type + ';'\">x</button>\
         <div id=d onclick='return false'></div>",
    );
    let b = node(&doc, "b");
    js.dispatch_event(b, "MouseEvent", "click", &[]);
    js.dispatch_event(b, "MouseEvent", "click", &[]);
    assert_eq!(eval(&mut js, "window.hit"), "b:click;b:click;");
    assert!(
        !js.dispatch_event(node(&doc, "d"), "MouseEvent", "click", &[]),
        "return false は既定動作を止める"
    );
    // 属性を書き換えたら新しい本文が使われる
    eval(
        &mut js,
        "document.getElementById('b').setAttribute('onclick', 'window.hit = 9')",
    );
    js.dispatch_event(b, "MouseEvent", "click", &[]);
    assert_eq!(eval(&mut js, "window.hit"), "9");
}

#[test]
fn infinite_loops_are_stopped_with_an_exception() {
    let (mut js, _) = runtime(PAGE);
    let error = js
        .run_script("while (true) {}", "loop.js", None)
        .unwrap_err();
    assert!(
        error.contains("RangeError") || error.contains("limit") || error.contains("iteration"),
        "{error}"
    );
    assert_eq!(
        eval(&mut js, "typeof document.body"),
        "object",
        "ループの後も動く"
    );
}

#[test]
fn remote_pages_cannot_reach_local_files() {
    let (mut js, _) = runtime(PAGE); // https://example.com/…
    eval(&mut js, "location.href = 'file:///C:/secret.txt'");
    assert_eq!(js.take_navigation(), None, "file: へは移動させない");
    eval(
        &mut js,
        "fetch('file:///C:/secret.txt').catch((e) => { globalThis.blocked = e.name; })",
    );
    assert!(js.take_fetches().is_empty(), "file: は Network に出さない");
    assert_eq!(eval(&mut js, "globalThis.blocked"), "TypeError");
    eval(
        &mut js,
        "const a = document.createElement('a'); a.href = 'file:///C:/x'; document.body.appendChild(a); a.click();",
    );
    assert_eq!(js.take_navigation(), None);
    assert!(kumo_js::may_reach(
        &url::Url::parse("file:///a.html").unwrap(),
        &url::Url::parse("file:///b.html").unwrap()
    ));
    assert!(kumo_js::may_reach(
        &url::Url::parse("https://a.example/").unwrap(),
        &url::Url::parse("http://b.example/").unwrap()
    ));
}
