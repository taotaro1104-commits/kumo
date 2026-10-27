//! ページの見た目に関わる Content の処理：マウスの下にあるもの、エラーページ。
mod common;

use common::{Site, engine, pump};
use kumo_content::protocol::{FromContent, ToContent};

fn load(nav: u64, url: &url::Url) -> ToContent {
    ToContent::Load {
        tab: 1,
        nav,
        url: url.to_string(),
        post_body: None,
    }
}

#[test]
fn hover_reports_links_fields_and_nothing_only_on_change() {
    use kumo_content::protocol::HoverKind;
    let site = Site::new("hover");
    let mut e = engine();
    e.handle(ToContent::Show { tab: 1 });
    e.handle(load(1, &site.url("long.html")));
    pump(&mut e, true);
    e.handle(ToContent::Hover {
        tab: 1,
        x: 5.0,
        y: 10.0,
    });
    let red = site.url("red.html").to_string();
    assert_eq!(
        e.take_output(),
        vec![FromContent::Hovered {
            tab: 1,
            link: Some(red),
            kind: HoverKind::Link
        }]
    );
    e.handle(ToContent::Hover {
        tab: 1,
        x: 6.0,
        y: 11.0,
    });
    assert!(
        e.take_output().is_empty(),
        "同じリンクの上では知らせ直さない"
    );
    e.handle(ToContent::Hover {
        tab: 1,
        x: 390.0,
        y: 290.0,
    });
    assert_eq!(
        e.take_output(),
        vec![FromContent::Hovered {
            tab: 1,
            link: None,
            kind: HoverKind::None
        }]
    );

    e.handle(load(2, &site.url("form.html")));
    pump(&mut e, true);
    let controls = e.page(1).unwrap().list.controls.clone();
    let (rect, _) = controls[0];
    e.handle(ToContent::Hover {
        tab: 1,
        x: rect.x + 2.0,
        y: rect.y + 2.0,
    });
    let kinds: Vec<HoverKind> = e
        .take_output()
        .into_iter()
        .filter_map(|m| match m {
            FromContent::Hovered { kind, .. } => Some(kind),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, vec![HoverKind::Field], "文字の入力欄");
    let (rect, _) = controls[2];
    e.handle(ToContent::Hover {
        tab: 1,
        x: rect.x + 2.0,
        y: rect.y + 2.0,
    });
    assert!(matches!(
        e.take_output().as_slice(),
        [FromContent::Hovered {
            kind: HoverKind::Control,
            ..
        }]
    ));
}

#[test]
fn error_pages_explain_the_cause_in_plain_words() {
    use kumo_content::describe_error;
    use kumo_net::protocol::FetchError;
    let site = Site::new("errors");
    let mut e = engine();
    e.handle(load(1, &site.url("missing.html")));
    pump(&mut e, true);
    let page = e.page(1).unwrap();
    let text = page.document().text_content(page.document().root());
    assert_eq!(page.title(), "読み込めません");
    assert!(text.contains("ファイルが見つかりません"), "{text}");
    assert!(text.contains("再読み込み") && text.contains("戻る"));
    let headline = |e: FetchError| describe_error(&e).0;
    assert_eq!(
        headline(FetchError::Blocked),
        "このページはブロックされました"
    );
    assert_eq!(
        headline(FetchError::Network("failed to lookup address".into())),
        "サーバーが見つかりません"
    );
    assert_eq!(
        headline(FetchError::Network("connection timed out".into())),
        "応答がありません"
    );
    assert_eq!(
        headline(FetchError::Network(
            "Connection refused (os error 10061)".into()
        )),
        "接続が拒否されました"
    );
    assert_eq!(
        headline(FetchError::Network("invalid peer certificate".into())),
        "安全な接続を確立できません"
    );
    assert_eq!(
        headline(FetchError::UnsupportedScheme("ftp".into())),
        "この種類のアドレスは開けません"
    );
    assert_eq!(
        headline(FetchError::Network("odd".into())),
        "ページを読み込めませんでした"
    );
}
