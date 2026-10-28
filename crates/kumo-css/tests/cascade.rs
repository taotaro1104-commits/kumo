use std::sync::Arc;

use kumo_css::keywords::{Display, Float, TextAlign};
use kumo_css::values::Rgba;
use kumo_css::{
    CascadeEngine, ComputedStyle, Device, LengthPct, LengthPctAuto, StyleEngine, StyleMap,
    Stylesheet,
};
use kumo_dom::{Document, NodeId};

const DEVICE: Device = Device {
    width: 1000.0,
    height: 800.0,
};

fn style(html: &str, css: &str) -> (Document, StyleMap) {
    let doc = kumo_html::parse_html(html);
    let engine = CascadeEngine::new(vec![Stylesheet::parse(css)]);
    let map = engine.compute(&doc, &DEVICE);
    (doc, map)
}

fn by_id(doc: &Document, id: &str) -> NodeId {
    doc.descendants(doc.root())
        .find(|&n| doc.attr(n, "id") == Some(id))
        .unwrap_or_else(|| panic!("#{id} が見つからない"))
}

fn computed<'a>(doc: &Document, map: &'a StyleMap, id: &str) -> &'a Arc<ComputedStyle> {
    map.get(by_id(doc, id)).expect("スタイルがある")
}

const RED: Rgba = Rgba::new(255, 0, 0, 255);
const BLUE: Rgba = Rgba::new(0, 0, 255, 255);

#[test]
fn ua_defaults_apply() {
    let (doc, map) = style("<div id=d><span id=s>x</span></div><h1 id=h>t</h1>", "");
    assert_eq!(computed(&doc, &map, "d").display, Display::Block);
    assert_eq!(computed(&doc, &map, "s").display, Display::Inline);
    let h1 = computed(&doc, &map, "h");
    assert_eq!(h1.font_size, 32.0);
    assert_eq!(h1.font_weight, 700);
    assert_eq!(
        h1.margin.top,
        LengthPctAuto::LengthPct(LengthPct::px(32.0 * 0.67))
    );
}

#[test]
fn specificity_and_order() {
    let css = "p { color: red } #p { color: blue } p.c { color: red }";
    let (doc, map) = style("<p id=p class=c>x</p>", css);
    assert_eq!(computed(&doc, &map, "p").color, BLUE);

    let css = ".a { color: blue } .b { color: red }";
    let (doc, map) = style("<p id=p class='b a'>x</p>", css);
    assert_eq!(computed(&doc, &map, "p").color, RED);
}

#[test]
fn important_and_inline_style() {
    let css = "#p { color: red !important } p { color: blue }";
    let (doc, map) = style("<p id=p style='color: green'>x</p>", css);
    assert_eq!(computed(&doc, &map, "p").color, RED);

    let (doc, map) = style("<p id=p style='color: blue'>x</p>", "#p { color: red }");
    assert_eq!(computed(&doc, &map, "p").color, BLUE);
}

#[test]
fn inheritance_and_em() {
    let css = "div { font-size: 20px; color: blue; padding: 1em } span { font-size: 1.5em }";
    let (doc, map) = style("<div id=d><span id=s>x</span></div>", css);
    let div = computed(&doc, &map, "d");
    assert_eq!(div.padding.left, LengthPct::px(20.0));
    let span = computed(&doc, &map, "s");
    assert_eq!(span.font_size, 30.0);
    assert_eq!(span.color, BLUE);
    // 非継承プロパティは引き継がない
    assert_eq!(span.padding.left, LengthPct::ZERO);
}

#[test]
fn rem_uses_root_font_size() {
    let css = "html { font-size: 10px } p { margin-left: 2rem; font-size: 3em }";
    let (doc, map) = style("<p id=p>x</p>", css);
    let p = computed(&doc, &map, "p");
    assert_eq!(p.margin.left, LengthPctAuto::LengthPct(LengthPct::px(20.0)));
    assert_eq!(p.font_size, 30.0);
}

#[test]
fn custom_properties_and_var() {
    let css = ":root { --main: #00f; --pad: 4px } \
               p { color: var(--main); padding: var(--pad) var(--missing, 2px) } \
               .x { --main: red }";
    let (doc, map) = style("<p id=p>a</p><div class=x><p id=q>b</p></div>", css);
    let p = computed(&doc, &map, "p");
    assert_eq!(p.color, BLUE);
    assert_eq!(p.padding.top, LengthPct::px(4.0));
    assert_eq!(p.padding.left, LengthPct::px(2.0));
    assert_eq!(computed(&doc, &map, "q").color, RED);
}

#[test]
fn media_queries_filter_rules() {
    let css = "p { color: red } @media (max-width: 500px) { p { color: blue } } \
               @media screen and (min-width: 900px) { p { text-align: center } }";
    let (doc, map) = style("<p id=p>x</p>", css);
    let p = computed(&doc, &map, "p");
    assert_eq!(p.color, RED);
    assert_eq!(p.text_align, TextAlign::Center);
}

#[test]
fn inherit_keyword_and_currentcolor() {
    let css = "div { color: blue; border: 1px solid } p { color: red; background-color: inherit } \
               span { border-top: 2px solid currentColor; color: inherit }";
    let (doc, map) = style("<div id=d><p id=p><span id=s>x</span></p></div>", css);
    let d = computed(&doc, &map, "d");
    assert_eq!(d.border_color.top, BLUE);
    assert_eq!(d.used_border_width(kumo_css::Side::Left), 1.0);
    let s = computed(&doc, &map, "s");
    assert_eq!(s.color, RED);
    assert_eq!(s.border_color.top, RED);
}

#[test]
fn floats_are_blockified_and_display_none_skips_subtree() {
    let css = "span { float: right } .hide { display: none }";
    let (doc, map) = style(
        "<span id=s>x</span><div class=hide><p id=p>y</p></div>",
        css,
    );
    let s = computed(&doc, &map, "s");
    assert_eq!(s.float, Float::Right);
    assert_eq!(s.display, Display::Block);
    assert!(map.get(by_id(&doc, "p")).is_none());
}

#[test]
fn identical_elements_share_computed_style() {
    let html = "<ul><li id=a>1</li><li id=b>2</li><li id=c class=x>3</li></ul>";
    let (doc, map) = style(html, ".x { color: red }");
    let a = computed(&doc, &map, "a");
    let b = computed(&doc, &map, "b");
    let c = computed(&doc, &map, "c");
    assert!(Arc::ptr_eq(a, b));
    assert!(!Arc::ptr_eq(a, c));
}

#[test]
fn presentational_attributes() {
    let html = "<table id=t width=300 bgcolor=#ff0000 border=1><tr><td id=c align=center>x</td></tr></table>";
    let (doc, map) = style(html, "");
    let t = computed(&doc, &map, "t");
    assert_eq!(t.width, LengthPctAuto::LengthPct(LengthPct::px(300.0)));
    assert_eq!(t.background_color, RED);
    assert_eq!(t.used_border_width(kumo_css::Side::Top), 1.0);
    assert_eq!(computed(&doc, &map, "c").text_align, TextAlign::Center);
}

#[test]
fn shorthands_expand() {
    let css = "div { margin: 1px 2px 3px; flex: 2 0 10px; font: italic bold 12px/2 Arial, serif; \
               padding-inline: 5px; grid-column: 1 / 3; overflow: hidden }";
    let (doc, map) = style("<div id=d></div>", css);
    let d = computed(&doc, &map, "d");
    assert_eq!(d.margin.left, LengthPctAuto::LengthPct(LengthPct::px(2.0)));
    assert_eq!(
        d.margin.bottom,
        LengthPctAuto::LengthPct(LengthPct::px(3.0))
    );
    assert_eq!((d.flex_grow, d.flex_shrink), (2.0, 0.0));
    assert_eq!(d.font_size, 12.0);
    assert_eq!(d.font_weight, 700);
    assert_eq!(d.line_height_px(), 24.0);
    assert_eq!(&*d.font_family[0], "Arial");
    assert_eq!(d.padding.right, LengthPct::px(5.0));
    assert_eq!(d.grid.column_start.as_deref(), Some("1"));
    assert_eq!(d.grid.column_end.as_deref(), Some("3"));
}

#[test]
fn complex_selectors() {
    let css = "ul > li:first-child { color: red } li + li { color: blue } \
               a[href$='.pdf'] { color: red } :is(h1, h2).t { color: blue } div:not(.x) p { color: red }";
    let html = "<ul><li id=a>1</li><li id=b>2</li></ul><a id=l href='x.pdf'>p</a>\
                <h2 id=h class=t>h</h2><div><p id=p>y</p></div>";
    let (doc, map) = style(html, css);
    assert_eq!(computed(&doc, &map, "a").color, RED);
    assert_eq!(computed(&doc, &map, "b").color, BLUE);
    assert_eq!(computed(&doc, &map, "l").color, RED);
    assert_eq!(computed(&doc, &map, "h").color, BLUE);
    assert_eq!(computed(&doc, &map, "p").color, RED);
}

#[test]
fn var_expansion_is_bounded_and_font_size_var_sets_em_basis() {
    let mut css = String::from(":root { --v0: xxxxxxxxxx; ");
    for i in 1..40 {
        css.push_str(&format!("--v{i}: var(--v{0}) var(--v{0}); ", i - 1));
    }
    css.push_str("} p { color: blue; font-size: var(--fs, 20px); padding-left: 1em }");
    let (doc, map) = style("<p id=p>x</p>", &css);
    let p = computed(&doc, &map, "p");
    assert!(p.custom.values().all(|v| v.len() <= 64 * 1024));
    assert_eq!(p.font_size, 20.0);
    assert_eq!(
        p.padding.left,
        LengthPct::px(20.0),
        "var() の font-size が em の基準になる"
    );
    assert_eq!(p.color, BLUE);
}

#[test]
fn custom_properties_resolve_in_dependency_order() {
    // 同じ要素で、低い詳細度の --a が高い詳細度で定義される --b を参照する
    let css = "p { --a: var(--b) } #p { --b: #f00; color: var(--a) } \
               #q { --x: var(--y); --y: var(--x); color: var(--x, blue) }";
    let (doc, map) = style("<p id=p>a</p><p id=q>b</p>", css);
    assert_eq!(computed(&doc, &map, "p").color, RED);
    // 循環参照は無効になりフォールバックが使われる
    let q = computed(&doc, &map, "q");
    assert_eq!(q.color, BLUE);
    assert!(!q.custom.contains_key(&kumo_dom::LocalName::from("--x")));
}

#[test]
fn z_index_and_stacking_contexts() {
    let css = "#a { position: relative; z-index: 3 } #b { position: absolute } \
               #c { z-index: 5 } #d { opacity: 0.5 } #e { position: relative; z-index: auto }";
    let html = "<div id=a></div><div id=b></div><div id=c></div><div id=d></div><div id=e></div>";
    let (doc, map) = style(html, css);
    let get = |id| computed(&doc, &map, id);
    assert_eq!(get("a").z_index, Some(3));
    assert!(get("a").creates_stacking_context());
    assert!(get("b").is_positioned() && !get("b").creates_stacking_context());
    assert!(
        !get("c").creates_stacking_context(),
        "非位置指定の z-index は効かない"
    );
    assert!(get("d").creates_stacking_context());
    assert_eq!(get("e").z_index, None);
}

#[test]
fn border_radius_shorthand_and_longhands() {
    let (doc, map) = style(
        "<div id=a></div><div id=b></div><div id=c></div><div id=d></div>",
        "#a { border-radius: 6px }
         #b { border-radius: 1px 2px 3px 4px / 9px }
         #c { border-radius: 50%; border-top-left-radius: 0 }
         #d { border-radius: 2px 5px; border-bottom-right-radius: -3px }",
    );
    let px = LengthPct::px;
    let half = LengthPct { px: 0.0, pct: 50.0 };
    assert_eq!(computed(&doc, &map, "a").border_radius, [px(6.0); 4]);
    assert_eq!(
        computed(&doc, &map, "b").border_radius,
        [px(1.0), px(2.0), px(3.0), px(4.0)],
        "左上・右上・右下・左下（/ 以降の縦の半径は無視）"
    );
    assert_eq!(
        computed(&doc, &map, "c").border_radius,
        [px(0.0), half, half, half]
    );
    assert_eq!(
        computed(&doc, &map, "d").border_radius,
        [px(2.0), px(5.0), px(2.0), px(5.0)],
        "負の値は無効"
    );
    assert_eq!(ComputedStyle::default().border_radius, [LengthPct::ZERO; 4]);
}

#[test]
fn noscript_content_is_not_displayed() {
    let (doc, map) = style(
        "<div id=d><noscript id=n><img src=x.png></noscript></div>",
        "",
    );
    assert_eq!(computed(&doc, &map, "n").display, Display::None);
    assert_eq!(computed(&doc, &map, "d").display, Display::Block);
}

#[test]
fn print_media_rules_do_not_apply_to_the_screen() {
    let doc = kumo_html::parse_html("<p id=a>x</p>");
    let engine = CascadeEngine::new(vec![Stylesheet::parse(
        "@media print { p { display: none } } @media screen { p { color: #010203 } }",
    )]);
    let styles = engine.compute(
        &doc,
        &Device {
            width: 800.0,
            height: 600.0,
        },
    );
    let a = doc
        .descendants(doc.root())
        .find(|&n| doc.attr(n, "id") == Some("a"))
        .unwrap();
    let style = styles.get(a).unwrap();
    assert_ne!(style.display, kumo_css::keywords::Display::None);
    assert_eq!(style.color.r, 1, "screen の規則は効く");
}
