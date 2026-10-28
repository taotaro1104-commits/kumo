use kumo_css::{CascadeEngine, Device, StyleEngine, Stylesheet};
use kumo_layout::{NoImages, build_box_tree, layout};
use kumo_paint::{
    CpuRasterizer, DisplayItem, NoImageProvider, RectF, TileCache, build_display_list,
    render_region,
};
use kumo_text::TextSystem;

const W: f32 = 400.0;
const H: f32 = 300.0;

fn display_list(html: &str, css: &str) -> kumo_paint::DisplayList {
    let doc = kumo_html::parse_html(html);
    let engine = CascadeEngine::new(vec![Stylesheet::parse(css)]);
    let styles = engine.compute(
        &doc,
        &Device {
            width: W,
            height: H,
        },
    );
    let mut tree = build_box_tree(&doc, &styles, TextSystem::new(), &NoImages);
    layout(&mut tree, W, H);
    build_display_list(&tree)
}

fn pixel(p: &tiny_skia::Pixmap, x: u32, y: u32) -> (u8, u8, u8) {
    let c = p.pixel(x, y).unwrap();
    (c.red(), c.green(), c.blue())
}

#[test]
fn backgrounds_borders_and_canvas() {
    let css = "body { margin: 0; background: #eee } #a { margin: 10px; width: 50px; height: 40px; \
               background: red; border: 5px solid blue }";
    let list = display_list("<div id=a></div>", css);
    let mut raster = CpuRasterizer::default();
    let pm = render_region(
        &list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(0.0, 0.0, W, H),
    )
    .unwrap();
    assert_eq!(
        pixel(&pm, 2, 2),
        (0xee, 0xee, 0xee),
        "body 背景がキャンバスへ伝播"
    );
    assert_eq!(pixel(&pm, 12, 12), (0, 0, 255), "枠線");
    assert_eq!(pixel(&pm, 30, 30), (255, 0, 0), "背景");
}

#[test]
fn text_produces_glyphs_and_ink() {
    let css = "body { margin: 0; font-size: 40px; color: black }";
    let list = display_list("<p style='margin:0'>Kumo <u>rocks</u></p>", css);
    let glyph_runs = list
        .entries
        .iter()
        .filter(|e| matches!(e.item, DisplayItem::Glyphs(_)))
        .count();
    assert!(glyph_runs >= 1);
    let mut raster = CpuRasterizer::default();
    let pm = render_region(
        &list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(0.0, 0.0, W, 60.0),
    )
    .unwrap();
    let dark = pm.pixels().iter().filter(|p| p.red() < 100).count();
    assert!(dark > 100, "文字のインクが描かれている: {dark}");
    assert!(raster.glyphs.bytes() > 0);
}

#[test]
fn overflow_hidden_clips_children() {
    let css = "body { margin: 0 } #c { width: 20px; height: 20px; overflow: hidden } \
               #k { width: 100px; height: 100px; background: lime }";
    let list = display_list("<div id=c><div id=k></div></div>", css);
    let mut raster = CpuRasterizer::default();
    let pm = render_region(
        &list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(0.0, 0.0, W, H),
    )
    .unwrap();
    assert_eq!(pixel(&pm, 10, 10), (0, 255, 0));
    assert_eq!(
        pixel(&pm, 50, 50),
        (255, 255, 255),
        "クリップ外は描かれない"
    );
}

#[test]
fn tile_cache_composes_scrolled_viewport() {
    let css = "body { margin: 0 } #a { height: 1000px } #b { height: 50px; background: red }";
    let list = display_list("<div id=a></div><div id=b></div>", css);
    let mut raster = CpuRasterizer::default();
    let mut tiles = TileCache::default();
    let (vw, vh) = (400usize, 300usize);
    let mut out = vec![0u32; vw * vh];
    tiles.compose(
        &list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(0.0, 900.0, vw as f32, vh as f32),
        &mut out,
    );
    assert_eq!(out[10 * vw + 10], 0x00ff_ffff, "y=910 は白");
    assert_eq!(out[120 * vw + 10], 0x00ff_0000, "y=1020 は赤");
    assert!(tiles.bytes() <= kumo_paint::TILE_BUDGET);
}

#[test]
fn scroll_renderer_matches_full_redraw() {
    let css = "body { margin: 0 } div { height: 37px } div:nth-child(odd) { background: #369 } \
               div:nth-child(3n) { background: #c33 }";
    let html = "<div></div>".repeat(80) + "<p>text at the end</p>";
    let list = display_list(&html, css);
    let (vw, vh) = (400usize, 300usize);
    let full = |y: f32| {
        let mut out = vec![0u32; vw * vh];
        kumo_paint::ScrollRenderer::default().render(
            &list,
            &mut CpuRasterizer::default(),
            &mut NoImageProvider,
            RectF::new(0.0, y, vw as f32, vh as f32),
            &mut out,
            false,
        );
        out
    };
    let mut renderer = kumo_paint::ScrollRenderer::default();
    let mut raster = CpuRasterizer::default();
    let mut out = vec![0u32; vw * vh];
    let first = renderer.render(
        &list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(0.0, 500.0, vw as f32, vh as f32),
        &mut out,
        false,
    );
    assert_eq!(first, Some(0..vh));
    for y in [550.0, 530.0, 900.0, 899.0] {
        let damage = renderer.render(
            &list,
            &mut raster,
            &mut NoImageProvider,
            RectF::new(0.0, y, vw as f32, vh as f32),
            &mut out,
            true,
        );
        assert!(
            damage.is_some_and(|r| r.len() < vh || y == 900.0),
            "大きく飛んだときだけ全面描画"
        );
        assert!(out == full(y), "y={y} で全面描画と一致");
    }
    assert!(renderer.bytes() <= vw * 64 * 4);
}

#[test]
fn floats_paint_above_following_block_backgrounds() {
    let css = "body { margin: 0 } #f { float: right; width: 100px; height: 100px; background: lime } \
               p { margin: 0; height: 50px; background: red }";
    let list = display_list("<div id=f></div><p></p>", css);
    let mut raster = CpuRasterizer::default();
    let pm = render_region(
        &list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(0.0, 0.0, W, H),
    )
    .unwrap();
    assert_eq!(
        pixel(&pm, (W - 50.0) as u32, 25),
        (0, 255, 0),
        "フロートが段落の背景より手前"
    );
    assert_eq!(pixel(&pm, 10, 25), (255, 0, 0));
}

/// (x, y) の色を 1 点だけ描いて返す。
fn color_at(list: &kumo_paint::DisplayList, x: f32, y: f32) -> (u8, u8, u8) {
    let mut raster = CpuRasterizer::default();
    let pm = render_region(
        list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(x, y, 1.0, 1.0),
    )
    .unwrap();
    pixel(&pm, 0, 0)
}

#[test]
fn z_index_orders_positioned_boxes() {
    // 後に書かれた #b が通常なら上に来るが、z-index で #a を上にする
    let css = "body { margin: 0 } div { position: absolute; top: 0; left: 0; width: 50px; height: 50px } \
               #a { background: red; z-index: 2 } #b { background: blue; z-index: 1 } \
               #c { background: lime; left: 100px; z-index: -1 } #d { position: static; width: 300px; height: 300px; background: yellow }";
    let list = display_list(
        "<div id=a></div><div id=b></div><div id=c></div><div id=d></div>",
        css,
    );
    assert_eq!(
        color_at(&list, 10.0, 10.0),
        (255, 0, 0),
        "z-index の大きい方が上"
    );
    // 負の z-index は通常フローのブロック背景（黄）より奥
    assert_eq!(color_at(&list, 110.0, 10.0), (255, 255, 0));
}

#[test]
fn positioned_boxes_paint_above_in_flow_content_and_floats() {
    let css = "body { margin: 0 } #f { float: left; width: 100px; height: 100px; background: lime } \
               #r { position: relative; top: 0; width: 50px; height: 50px; background: blue } \
               p { margin: 0; height: 100px; background: red }";
    let list = display_list("<p></p><div id=f></div><div id=r></div>", css);
    // 相対配置のボックスは、後から描くフロートよりさらに上に描く
    assert_eq!(color_at(&list, 10.0, 110.0), (0, 0, 255));
    assert_eq!(color_at(&list, 80.0, 150.0), (0, 255, 0));
}

#[test]
fn hidden_visibility_skips_box_but_not_visible_children() {
    let css = "body { margin: 0 } #h { visibility: hidden; height: 100px; background: red } \
               #v { visibility: visible; height: 50px; background: blue }";
    let list = display_list("<div id=h><div id=v></div></div>", css);
    assert_eq!(color_at(&list, 10.0, 10.0), (0, 0, 255));
    assert_eq!(
        color_at(&list, 10.0, 80.0),
        (255, 255, 255),
        "親の背景は描かない"
    );
}

#[test]
fn link_regions_cover_link_text_and_block_links() {
    let css = "body { margin: 0; font-size: 20px; line-height: 30px } p { margin: 0 } \
               #blk { display: block; height: 40px; width: 100px }";
    let html = "<p>plain text <a href='/inline'>linked words</a></p><a id=blk href='/block'><span>x</span></a>";
    let list = display_list(html, css);
    let inline = list
        .links
        .iter()
        .find(|l| &*l.href == "/inline")
        .expect("テキストのリンク");
    assert_eq!(inline.rect.y, 0.0);
    assert_eq!(inline.rect.h, 30.0);
    assert!(inline.rect.x > 50.0, "リンク部分だけ: {:?}", inline.rect);
    let (cx, cy) = (inline.rect.x + 5.0, 15.0);
    assert_eq!(list.link_at(cx, cy).map(|h| &**h), Some("/inline"));
    assert_eq!(list.link_at(2.0, 15.0), None, "リンク外");
    assert_eq!(
        list.link_at(90.0, 50.0).map(|h| &**h),
        Some("/block"),
        "ブロック全体が当たり判定"
    );
}

#[test]
fn rounded_boxes_paint_round_corners() {
    let css = "body { margin: 0; background: #fff }
               #pill { width: 100px; height: 40px; background: red; border-radius: 50% }
               #ring { width: 60px; height: 60px; border: 6px solid blue; border-radius: 12px }
               #mixed { width: 40px; height: 40px; border: 4px solid; border-color: red blue; border-radius: 8px }";
    let list = display_list(
        "<div id=pill></div><div id=ring></div><div id=mixed></div>",
        css,
    );
    let rounded = list
        .entries
        .iter()
        .filter(|e| {
            matches!(
                e.item,
                DisplayItem::RoundedFill { .. } | DisplayItem::RoundedBorder { .. }
            )
        })
        .count();
    assert_eq!(
        rounded, 2,
        "ピルの背景と、単色の枠線だけが角丸（色違いの枠線は四角で描く）"
    );
    let mut raster = CpuRasterizer::default();
    let pm = render_region(
        &list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(0.0, 0.0, W, H),
    )
    .unwrap();
    assert_eq!(pixel(&pm, 50, 20), (255, 0, 0), "ピルの中央");
    assert_eq!(pixel(&pm, 1, 1), (255, 255, 255), "ピルの角は丸く抜ける");
    // 枠線の輪（上端 40px から）：辺は青、角の外は白、内側は白
    assert_eq!(pixel(&pm, 36, 42), (0, 0, 255), "輪の上辺");
    assert_eq!(pixel(&pm, 0, 40), (255, 255, 255), "輪の角の外");
    assert_eq!(pixel(&pm, 36, 76), (255, 255, 255), "輪の内側");
}

#[test]
fn hit_regions_find_the_innermost_element_including_inline_text() {
    use kumo_paint::build_display_list_with;
    let html = "<div id=a style='margin:0;width:300px;height:100px'><p id=p style='margin:0;font-size:20px'>text <span id=s>inline</span></p></div>";
    let doc = kumo_html::parse_html(html);
    let engine = CascadeEngine::new(vec![Stylesheet::parse("body { margin: 0 }")]);
    let styles = engine.compute(
        &doc,
        &Device {
            width: W,
            height: H,
        },
    );
    let mut tree = build_box_tree(&doc, &styles, TextSystem::new(), &NoImages);
    layout(&mut tree, W, H);
    let id_of = |name: &str| {
        doc.descendants(doc.root())
            .find(|&n| doc.attr(n, "id") == Some(name))
            .unwrap()
            .index()
    };
    let without = build_display_list(&tree);
    assert!(
        without.hits.is_empty() && without.hit_at(5.0, 5.0).is_none(),
        "既定では記録しない"
    );
    let list = build_display_list_with(&tree, true);
    assert_eq!(list.hit_at(5.0, 5.0), Some(id_of("p")), "段落の文字の上");
    assert_eq!(
        list.hit_at(250.0, 60.0),
        Some(id_of("a")),
        "段落の外の div の上"
    );
    let span_x = list
        .hits
        .iter()
        .find(|(_, n)| *n == id_of("s"))
        .map(|(r, _)| r.x + 2.0)
        .expect("span の領域がある");
    assert_eq!(
        list.hit_at(span_x, 5.0),
        Some(id_of("s")),
        "インラインの span"
    );
    assert_eq!(list.hit_at(250.0, 250.0), None, "何もない所");
}
