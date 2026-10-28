//! `window` のグローバル：location・history・storage・navigator・URL・cookie・`document.implementation`。
mod common;

use common::{PAGE, eval, runtime};

#[test]
fn window_globals_location_and_storage() {
    let (mut js, _) = runtime(PAGE);
    assert_eq!(
        eval(
            &mut js,
            "window === globalThis && self === window && window.document === document"
        ),
        "true"
    );
    assert_eq!(
        eval(
            &mut js,
            "[location.href, location.origin, location.pathname, location.search, location.hash, location.hostname].join(' ')"
        ),
        "https://example.com/dir/page.html?q=1#top https://example.com /dir/page.html ?q=1 #top example.com"
    );
    eval(&mut js, "location.href = '../other.html'");
    assert_eq!(
        js.take_navigation().as_deref(),
        Some("https://example.com/other.html")
    );
    eval(&mut js, "location.hash = 'sec'");
    assert_eq!(
        js.take_navigation().as_deref(),
        Some("https://example.com/dir/page.html?q=1#sec")
    );
    eval(&mut js, "history.pushState({}, '', '/pushed?a=2')");
    assert_eq!(
        eval(&mut js, "location.pathname + location.search"),
        "/pushed?a=2"
    );
    assert_eq!(
        eval(
            &mut js,
            "try { history.pushState({}, '', 'https://evil.example/') } catch (e) { e.message.includes('SecurityError') }"
        ),
        "true"
    );
    eval(
        &mut js,
        "localStorage.setItem('k', 'v'); localStorage.other = 5; sessionStorage.a = 'b'",
    );
    assert_eq!(
        eval(
            &mut js,
            "[localStorage.getItem('k'), localStorage.other, localStorage.length, sessionStorage.length, localStorage.nope].join()"
        ),
        "v,5,2,1,"
    );
    assert_eq!(
        eval(
            &mut js,
            "[innerWidth, innerHeight, screen.width, devicePixelRatio].join()"
        ),
        "1000,700,1000,1"
    );
    assert_eq!(
        eval(
            &mut js,
            "matchMedia('(min-width: 800px)').matches + ',' + matchMedia('(min-width: 1200px)').matches"
        ),
        "true,false"
    );
    assert_eq!(
        eval(
            &mut js,
            "new URL('../x?y=1', 'https://a.example/b/c').href + ' ' + new URLSearchParams({a: '1 2'}).toString()"
        ),
        "https://a.example/x?y=1 a=1+2"
    );
    assert_eq!(
        eval(
            &mut js,
            "btoa('abc') + atob('YWJj') + new TextEncoder().encode('é').length"
        ),
        "YWJjabc2"
    );
    assert_eq!(
        eval(
            &mut js,
            "navigator.userAgent.includes('Kumo') && navigator.language"
        ),
        "ja"
    );
    document_cookie();
}

fn document_cookie() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "document.cookie = 'a=1; path=/'; document.cookie = 'b=2'; document.cookie = 'a=3'",
    );
    assert_eq!(eval(&mut js, "document.cookie"), "b=2; a=3");
    eval(&mut js, "document.cookie = 'b=; max-age=0'");
    assert_eq!(eval(&mut js, "document.cookie"), "a=3");
    assert_eq!(eval(&mut js, "document.readyState"), "loading");
}

#[test]
fn implementation_creates_independent_html_documents() {
    let (mut js, doc) = runtime(PAGE);
    assert_eq!(
        eval(
            &mut js,
            "const d = document.implementation.createHTMLDocument('T');
             d.body.innerHTML = '<form></form><form></form>';
             [d.nodeType, d.title, d.body.childNodes.length, d.documentElement.tagName, d.head.tagName,
              d.getElementById('p'), d.querySelectorAll('form').length, document.querySelectorAll('form').length,
              d === document].join()"
        ),
        "9,T,2,HTML,HEAD,,2,0,false"
    );
    // 別の文書の要素は、メインの文書の検索には出てこない（取り込むまで）
    eval(
        &mut js,
        "const e = document.implementation.createHTMLDocument('').createElement('i'); e.id = 'detached'",
    );
    assert_eq!(
        eval(&mut js, "String(document.getElementById('detached'))"),
        "null"
    );
    let before = doc.borrow().len();
    assert!(before > 0);
}
