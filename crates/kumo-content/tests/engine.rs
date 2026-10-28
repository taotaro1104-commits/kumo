mod common;

use common::{Site, engine, pump};
use kumo_content::FrameReader;
use kumo_content::protocol::{FieldKey, FromContent, ToContent};

fn load(nav: u64, url: &url::Url) -> ToContent {
    ToContent::Load {
        tab: 1,
        nav,
        url: url.to_string(),
        post_body: None,
    }
}

fn loaded_titles(out: &[FromContent]) -> Vec<(u64, String)> {
    out.iter()
        .filter_map(|m| match m {
            FromContent::Loaded { nav, title, .. } => Some((*nav, title.clone())),
            _ => None,
        })
        .collect()
}

fn frame(out: &[FromContent]) -> Option<(String, u32, u32, (u32, u32))> {
    out.iter().find_map(|m| match m {
        FromContent::Frame {
            path,
            width,
            height,
            damage,
            ..
        } => Some((path.clone(), *width, *height, *damage)),
        _ => None,
    })
}

#[test]
fn loads_renders_into_shared_frame_and_waits_for_frame_done() {
    let site = Site::new("render");
    let mut e = engine();
    e.handle(ToContent::Viewport {
        width: 200,
        height: 100,
    });
    e.handle(ToContent::Show { tab: 1 });
    e.handle(load(1, &site.url("red.html")));
    // 取得前でも空白のフレームは描く
    let out = pump(&mut e, false);
    assert!(frame(&out).is_some(), "読み込み中も白いフレーム");
    e.handle(ToContent::FrameDone);
    let out = pump(&mut e, true);
    assert_eq!(loaded_titles(&out), vec![(1, "Red".to_owned())]);
    let (path, w, h, damage) = frame(&out).expect("読み込み後のフレーム");
    assert_eq!((w, h, damage), (200, 100, (0, 100)));
    let mut reader = FrameReader::default();
    reader.attach(&path, w, h).unwrap();
    assert_eq!(
        reader.pixels().unwrap()[50 * 200 + 100],
        0x00ff_0000,
        "外部 CSS の背景色"
    );
    assert_eq!(reader.size(), (200, 100));

    // 写し終える前は次を描かない
    e.handle(ToContent::ScrollBy { tab: 1, dy: 30.0 });
    let out = pump(&mut e, true);
    assert!(frame(&out).is_none());
    assert!(
        out.contains(&FromContent::Scrolled { tab: 1, y: 30.0 }),
        "位置はすぐ知らせる"
    );
    e.handle(ToContent::FrameDone);
    let out = pump(&mut e, true);
    let (_, _, _, damage) = frame(&out).expect("スクロール分");
    assert_eq!(damage, (70, 100), "露出した帯だけ描く");
    // 大きさが変わると新しい共有メモリ
    e.handle(ToContent::FrameDone);
    e.handle(ToContent::Viewport {
        width: 300,
        height: 100,
    });
    let (new_path, w, _, _) = frame(&pump(&mut e, true)).unwrap();
    assert_ne!(new_path, path);
    assert_eq!(w, 300);
    let stats = e.stats();
    assert_eq!(stats.tabs, 1);
    assert!(stats.dom_nodes > 0 && stats.frame_bytes >= 300 * 100 * 4);
}

#[test]
fn stale_loads_are_discarded_and_errors_show_a_page() {
    let site = Site::new("stale");
    let mut e = engine();
    e.handle(load(1, &site.url("red.html")));
    let first = e.take_jobs();
    e.handle(load(2, &site.url("missing.html")));
    let fetcher = e.fetcher();
    for job in first {
        e.done(job.run(&*fetcher).expect("結果のある仕事"));
    }
    assert!(
        loaded_titles(&e.take_output()).is_empty(),
        "古い読み込みは捨てる"
    );
    let out = pump(&mut e, true);
    assert_eq!(loaded_titles(&out), vec![(2, "読み込めません".to_owned())]);
    e.handle(load(3, &url::Url::parse("about:blank").unwrap()));
    assert_eq!(
        loaded_titles(&e.take_output()),
        vec![(3, "新しいタブ".to_owned())]
    );
    e.handle(ToContent::Close { tab: 1 });
    assert_eq!(e.stats().tabs, 0);
}

#[test]
fn clicks_follow_links_and_fragments_scroll() {
    let site = Site::new("links");
    let mut e = engine();
    e.handle(ToContent::Viewport {
        width: 400,
        height: 300,
    });
    e.handle(ToContent::Show { tab: 1 });
    e.handle(load(1, &site.url("long.html#far")));
    let out = pump(&mut e, true);
    let y = out.iter().find_map(|m| match m {
        FromContent::Scrolled { y, .. } => Some(*y),
        _ => None,
    });
    assert!(
        y.is_some_and(|y| (2000.0..2010.0).contains(&y)),
        "最初から # 付きなら飛び先へ: {y:?}"
    );
    e.handle(ToContent::ScrollToFragment {
        tab: 1,
        url: site.url("long.html").to_string(),
    });
    assert!(
        e.take_output()
            .contains(&FromContent::Scrolled { tab: 1, y: 0.0 })
    );
    e.handle(ToContent::Click {
        tab: 1,
        x: 5.0,
        y: 5.0,
        new_tab: true,
    });
    let navigate = e.take_output();
    assert_eq!(
        navigate,
        vec![FromContent::Navigate {
            tab: 1,
            url: site.url("red.html").to_string(),
            post_body: None,
            new_tab: true,
        }]
    );
    e.handle(ToContent::Click {
        tab: 1,
        x: 390.0,
        y: 290.0,
        new_tab: false,
    });
    assert!(e.take_output().is_empty(), "何もない所");
    e.handle(ToContent::ScrollBy { tab: 1, dy: 1e9 });
    let page_height = e.page(1).unwrap().document_height();
    assert!(e.take_output().contains(&FromContent::Scrolled {
        tab: 1,
        y: (page_height - 300.0).round()
    }));
}

#[test]
fn forms_focus_type_toggle_and_submit() {
    let site = Site::new("forms");
    let mut e = engine();
    e.handle(ToContent::Show { tab: 1 });
    e.handle(load(1, &site.url("form.html")));
    pump(&mut e, true);
    let controls = e.page(1).unwrap().list.controls.clone();
    assert_eq!(controls.len(), 3, "入力欄・チェックボックス・送信ボタン");
    let center = |i: usize| (controls[i].0.x + 2.0, controls[i].0.y + 2.0);

    let (x, y) = center(0);
    e.handle(ToContent::Click {
        tab: 1,
        x,
        y,
        new_tab: false,
    });
    assert!(e.take_output().contains(&FromContent::FieldFocus {
        tab: 1,
        focused: true
    }));
    for key in [
        FieldKey::Text("rust".into()),
        FieldKey::Backspace,
        FieldKey::Text("t 1".into()),
    ] {
        e.handle(ToContent::Key { tab: 1, key });
    }
    let (x, y) = center(1);
    e.handle(ToContent::Click {
        tab: 1,
        x,
        y,
        new_tab: false,
    });
    let out = e.take_output();
    assert!(
        out.contains(&FromContent::FieldFocus {
            tab: 1,
            focused: false
        }),
        "別の部品で外れる"
    );
    e.render();
    let (x, y) = center(0);
    e.handle(ToContent::Click {
        tab: 1,
        x,
        y,
        new_tab: false,
    });
    e.handle(ToContent::Key {
        tab: 1,
        key: FieldKey::Enter,
    });
    let out = e.take_output();
    let url = out.iter().find_map(|m| match m {
        FromContent::Navigate {
            url,
            post_body: None,
            ..
        } => Some(url.clone()),
        _ => None,
    });
    assert_eq!(
        url,
        Some(format!("{}?q=rust+1&all=yes", site.url("result.html")))
    );
    e.handle(ToContent::Key {
        tab: 1,
        key: FieldKey::Text("x".into()),
    });
    assert!(e.take_output().is_empty(), "送信でフォーカスは外れている");
}

#[test]
fn background_tabs_relayout_when_shown() {
    let site = Site::new("lazy");
    let mut e = engine();
    e.handle(ToContent::Viewport {
        width: 400,
        height: 300,
    });
    e.handle(ToContent::Show { tab: 1 });
    e.handle(load(1, &site.url("red.html")));
    e.handle(ToContent::Load {
        tab: 2,
        nav: 1,
        url: site.url("long.html").to_string(),
        post_body: None,
    });
    pump(&mut e, true);
    e.handle(ToContent::Viewport {
        width: 500,
        height: 300,
    });
    pump(&mut e, true);
    assert_eq!(e.page(1).unwrap().viewport(), (500.0, 300.0));
    assert_eq!(
        e.page(2).unwrap().viewport(),
        (400.0, 300.0),
        "背景タブは表示まで遅らせる"
    );
    e.handle(ToContent::FrameDone);
    e.handle(ToContent::Show { tab: 2 });
    pump(&mut e, true);
    assert_eq!(e.page(2).unwrap().viewport(), (500.0, 300.0));
}
