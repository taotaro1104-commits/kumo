//! ページの拡大・縮小：レイアウトは「幅 ÷ 倍率」で行い、描画命令を倍率で拡大する。
mod common;

use common::{Site, engine, pump};
use kumo_content::protocol::ToContent;

fn open(e: &mut kumo_content::ContentEngine, site: &Site) {
    e.handle(ToContent::Viewport {
        width: 200,
        height: 100,
    });
    e.handle(ToContent::Show { tab: 1 });
    e.handle(ToContent::Load {
        tab: 1,
        nav: 1,
        url: site.url("long.html").to_string(),
        post_body: None,
    });
    pump(e, true);
}

fn height(e: &kumo_content::ContentEngine) -> f32 {
    e.page(1).unwrap().document_height()
}

#[test]
fn zoom_scales_the_document_and_keeps_the_reading_position() {
    let site = Site::new("zoom");
    let mut e = engine();
    open(&mut e, &site);
    let normal = height(&e);

    e.handle(ToContent::Zoom {
        tab: 1,
        percent: 200,
    });
    pump(&mut e, true);
    let big = height(&e);
    assert!(
        (big / normal - 2.0).abs() < 0.05,
        "2 倍で縦も約 2 倍: {normal} → {big}"
    );
    assert_eq!(e.page(1).unwrap().zoom(), 2.0);

    e.handle(ToContent::Zoom {
        tab: 1,
        percent: 50,
    });
    pump(&mut e, true);
    assert!((height(&e) / normal - 0.5).abs() < 0.05);

    // 範囲外は丸める（25〜500%）
    e.handle(ToContent::Zoom {
        tab: 1,
        percent: 9999,
    });
    pump(&mut e, true);
    assert_eq!(e.page(1).unwrap().zoom(), 5.0);
    e.handle(ToContent::Zoom { tab: 1, percent: 1 });
    pump(&mut e, true);
    assert_eq!(e.page(1).unwrap().zoom(), 0.25);
}

#[test]
fn zoom_stays_after_navigating_in_the_same_tab() {
    let site = Site::new("zoom-nav");
    let mut e = engine();
    open(&mut e, &site);
    e.handle(ToContent::Zoom {
        tab: 1,
        percent: 150,
    });
    pump(&mut e, true);
    e.handle(ToContent::Load {
        tab: 1,
        nav: 2,
        url: site.url("red.html").to_string(),
        post_body: None,
    });
    pump(&mut e, true);
    assert_eq!(e.page(1).unwrap().zoom(), 1.5, "次のページにも効く");
}
