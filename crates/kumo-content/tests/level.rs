//! 読み込みの負荷レベル：レベルごとに取得するもの・実行するものが変わる。

mod common;

use common::{Site, engine, pump};
use kumo_content::protocol::ToContent;
use kumo_content::{LoadLevel, prepare};

const PAGE: &str = "<html><head><style>p{margin:0}</style><link rel=stylesheet href=red.css></head>\
    <body><p id=out>none</p><img src=pic.png><script src=run.js></script></body></html>";

fn site(name: &str) -> Site {
    let site = Site::new(name);
    std::fs::write(site.dir.join("page.html"), PAGE).unwrap();
    std::fs::write(
        site.dir.join("run.js"),
        "document.getElementById('out').textContent = 'ran';",
    )
    .unwrap();
    std::fs::write(site.dir.join("pic.png"), b"not really a png").unwrap();
    site
}

#[test]
fn each_level_fetches_only_what_it_allows() {
    let site = site("level-fetch");
    let engine = engine();
    let fetcher = engine.fetcher();
    let counts = |level| {
        let page = prepare(PAGE, site.url("page.html"), &*fetcher, level, true);
        (
            page.stylesheets.len(),
            page.images.len(),
            page.scripts.iter().filter(|(_, b)| b.is_some()).count(),
        )
    };
    assert_eq!(counts(LoadLevel::Minimal), (1, 0, 0), "埋め込み CSS だけ");
    assert_eq!(counts(LoadLevel::Light), (2, 0, 0), "外部 CSS が加わる");
    assert_eq!(counts(LoadLevel::Standard), (2, 1, 1));
    assert_eq!(counts(LoadLevel::Maximum), (2, 1, 1));
    // 画像の設定を切ると、標準でも画像だけ取得しない
    let page = prepare(
        PAGE,
        site.url("page.html"),
        &*fetcher,
        LoadLevel::Standard,
        false,
    );
    assert_eq!((page.stylesheets.len(), page.images.len()), (2, 0));
    assert_eq!(page.scripts.len(), 1);
}

#[test]
fn scripts_run_only_from_the_standard_level() {
    let site = site("level-js");
    let run = |level: LoadLevel| {
        let mut e = engine();
        e.handle(ToContent::LoadLevel(level));
        e.handle(ToContent::Viewport {
            width: 400,
            height: 300,
        });
        e.handle(ToContent::Show { tab: 1 });
        e.handle(ToContent::Load {
            tab: 1,
            nav: 1,
            url: site.url("page.html").to_string(),
            post_body: None,
        });
        pump(&mut e, true);
        let page = e.page(1).unwrap();
        let doc = page.document();
        let node = doc
            .descendants(doc.root())
            .find(|&n| doc.attr(n, "id") == Some("out"))
            .unwrap();
        doc.text_content(node)
    };
    assert_eq!(run(LoadLevel::Minimal), "none");
    assert_eq!(run(LoadLevel::Light), "none");
    assert_eq!(run(LoadLevel::Standard), "ran");
    assert_eq!(run(LoadLevel::Heavy), "ran");
}

#[test]
fn images_setting_stops_images_from_loading_even_lazily() {
    let site = Site::new("images-off");
    let img = image::RgbaImage::from_pixel(20, 10, image::Rgba([255, 0, 0, 255]));
    img.save(site.dir.join("r.png")).unwrap();
    std::fs::write(
        site.dir.join("p.html"),
        "<body style='margin:0'><img src=r.png width=20 height=10></body>",
    )
    .unwrap();
    let loaded = |messages: Vec<ToContent>| {
        let mut e = engine();
        for m in messages {
            e.handle(m);
        }
        e.handle(ToContent::Viewport {
            width: 100,
            height: 100,
        });
        e.handle(ToContent::Show { tab: 1 });
        e.handle(ToContent::Load {
            tab: 1,
            nav: 1,
            url: site.url("p.html").to_string(),
            post_body: None,
        });
        pump(&mut e, true);
        e.stats().compressed_image_bytes
    };
    let with = loaded(vec![]);
    let without = loaded(vec![ToContent::Images(false)]);
    assert_eq!(without, 0);
    assert!(with > 0, "画像ありなら取得する（{with} バイト）");
    assert_eq!(loaded(vec![ToContent::Images(true)]), with);
}
