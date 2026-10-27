use kumo_css::{CascadeEngine, Device, StyleEngine, Stylesheet};
use kumo_layout::{BoxKind, BoxTree, NoImages, build_box_tree, document_height, layout};
use kumo_text::TextSystem;

const W: f32 = 800.0;
const H: f32 = 600.0;

struct Laid {
    tree: BoxTree,
    doc: kumo_dom::Document,
}

fn lay(html: &str, css: &str) -> Laid {
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
    Laid { tree, doc }
}

/// id 属性で要素ボックスを探し、絶対座標の (x, y, w, h) を返す。
fn rect(laid: &Laid, id: &str) -> (f32, f32, f32, f32) {
    fn walk(laid: &Laid, index: usize, ox: f32, oy: f32, id: &str) -> Option<(f32, f32, f32, f32)> {
        let b = &laid.tree.boxes[index];
        let l = b.layout();
        let (x, y) = (ox + l.location.x, oy + l.location.y);
        if b.dom.and_then(|d| laid.doc.attr(d, "id")) == Some(id) {
            return Some((x, y, l.size.width, l.size.height));
        }
        b.children.iter().find_map(|&c| walk(laid, c, x, y, id))
    }
    walk(laid, 0, 0.0, 0.0, id).unwrap_or_else(|| panic!("#{id} のボックスがない"))
}

const RESET: &str = "body { margin: 0 } p { margin: 0 }";

#[test]
fn blocks_stack_vertically_with_margins() {
    let css = "body { margin: 0 } #a { height: 50px; margin-bottom: 20px } #b { height: 30px; margin-top: 10px }";
    let laid = lay("<div id=a></div><div id=b></div>", css);
    assert_eq!(rect(&laid, "a"), (0.0, 0.0, W, 50.0));
    // 縦マージンの相殺：max(20, 10) = 20
    assert_eq!(rect(&laid, "b").1, 70.0);
}

#[test]
fn width_padding_border_and_auto_margins() {
    let css =
        "body { margin: 0 } #a { width: 200px; padding: 10px; border: 5px solid; margin: 0 auto }";
    let laid = lay("<div id=a></div>", css);
    let (x, _, w, _) = rect(&laid, "a");
    assert_eq!(w, 230.0);
    assert_eq!(x, (W - 230.0) / 2.0);
}

#[test]
fn text_wraps_inside_narrow_block() {
    let css = format!("{RESET} #p {{ width: 100px; font-size: 16px; line-height: 20px }}");
    let laid = lay("<p id=p>one two three four five six seven eight</p>", &css);
    let (_, _, w, h) = rect(&laid, "p");
    assert_eq!(w, 100.0);
    assert!(h >= 60.0, "複数行に折り返されるはず: h={h}");
    assert_eq!(h % 20.0, 0.0, "行の高さの倍数: h={h}");
}

#[test]
fn inline_elements_share_one_paragraph() {
    let laid = lay(
        "<p id=p>Hello <b>bold</b> and <a href=#>link</a></p>",
        RESET,
    );
    let p_box = laid
        .tree
        .boxes
        .iter()
        .find(|b| b.dom.and_then(|d| laid.doc.attr(d, "id")) == Some("p"))
        .unwrap();
    assert_eq!(p_box.children.len(), 1, "匿名インラインボックス1つ");
    let inline = laid.tree.boxes[p_box.children[0]].inline().unwrap();
    assert_eq!(inline.text, "Hello bold and link");
    assert!(inline.paints.len() >= 3);
}

#[test]
fn block_inside_inline_splits_paragraph() {
    let laid = lay(
        "<div id=d><span>a<div id=inner>b</div>c</span></div>",
        RESET,
    );
    let (_, ay, _, _) = rect(&laid, "inner");
    let (_, dy, _, dh) = rect(&laid, "d");
    assert!(ay > dy, "ブロックは前のテキストの下");
    assert!(dh > ay - dy, "後ろのテキスト分の高さがある");
}

#[test]
fn flex_row_distributes_space() {
    let css = "body { margin: 0 } #f { display: flex } #a { flex: 1 } #b { width: 100px }";
    let laid = lay("<div id=f><div id=a>x</div><div id=b>y</div></div>", css);
    assert_eq!(rect(&laid, "a").2, W - 100.0);
    assert_eq!(rect(&laid, "b").0, W - 100.0);
}

#[test]
fn grid_template_areas_place_items() {
    let css = "body { margin: 0 } #g { display: grid; grid-template-columns: 200px 1fr; \
               grid-template-areas: 'side main' } #m { grid-area: main } #s { grid-area: side }";
    let laid = lay("<div id=g><div id=m>m</div><div id=s>s</div></div>", css);
    assert_eq!(rect(&laid, "s").0, 0.0);
    assert_eq!(rect(&laid, "m").0, 200.0);
    assert_eq!(rect(&laid, "m").2, W - 200.0);
}

#[test]
fn float_right_box() {
    let css = "body { margin: 0 } #f { float: right; width: 300px; height: 100px }";
    let laid = lay("<div id=f></div><p id=p>text</p>", css);
    assert_eq!(rect(&laid, "f").0, W - 300.0);
}

#[test]
fn table_cells_form_grid_with_spans() {
    let html = "<table id=t><tr><td id=a colspan=2>wide cell</td></tr>\
                <tr><td id=b>x</td><td id=c>y</td></tr></table>";
    let laid = lay(
        html,
        "body { margin: 0 } table { border-spacing: 0 } td { padding: 0 }",
    );
    let (tx, _, tw, _) = rect(&laid, "t");
    assert!(tw < W, "表は内容幅に縮む: {tw}");
    let (ax, ay, aw, _) = rect(&laid, "a");
    let (bx, by, _, _) = rect(&laid, "b");
    let (cx, _, _, _) = rect(&laid, "c");
    assert_eq!(ax, tx);
    assert_eq!(bx, tx);
    assert!(by > ay);
    assert!(cx > bx);
    assert!((aw - tw).abs() < 1.0, "colspan=2 は全幅");
}

#[test]
fn inline_block_and_image_are_atomic() {
    let html = "<p id=p>before <img id=i width=40 height=30> after <span id=s style='display:inline-block;width:50px;height:10px'></span></p>";
    let laid = lay(html, RESET);
    let (ix, _, iw, ih) = rect(&laid, "i");
    assert_eq!((iw, ih), (40.0, 30.0));
    assert!(ix > 0.0);
    let (sx, _, sw, _) = rect(&laid, "s");
    assert_eq!(sw, 50.0);
    assert!(sx > ix);
    assert!(matches!(
        laid.tree
            .boxes
            .iter()
            .find(|b| b.dom.and_then(|d| laid.doc.attr(d, "id")) == Some("i"))
            .unwrap()
            .kind,
        BoxKind::Replaced(_)
    ));
}

#[test]
fn list_items_get_markers() {
    let laid = lay("<ol><li id=a>x</li><li>y</li></ol><ul><li>z</li></ul>", "");
    let markers = laid
        .tree
        .boxes
        .iter()
        .filter(|b| b.marker.is_some())
        .count();
    assert_eq!(markers, 3);
}

#[test]
fn display_none_generates_no_box_and_document_height() {
    let css = "body { margin: 0 } #h { display: none } #a { height: 1000px }";
    let laid = lay("<div id=h>hidden</div><div id=a></div>", css);
    assert!(
        laid.tree
            .boxes
            .iter()
            .all(|b| b.dom.and_then(|d| laid.doc.attr(d, "id")) != Some("h"))
    );
    assert_eq!(document_height(&laid.tree), 1000.0);
}

#[test]
fn calc_widths_resolve() {
    let css = "body { margin: 0 } #a { width: calc(50% - 20px) }";
    let laid = lay("<div id=a></div>", css);
    assert_eq!(rect(&laid, "a").2, W / 2.0 - 20.0);
}

#[test]
fn hostile_inputs_do_not_crash_or_explode() {
    // 深い入れ子（スタック溢れしない）
    let deep = "<div>".repeat(5000) + "x" + &"</div>".repeat(5000);
    let laid = lay(&deep, RESET);
    assert!(laid.tree.boxes.len() < 500, "深さ上限で打ち切られる");
    let spans = "<span>".repeat(5000) + "y";
    lay(&spans, RESET);
    // 巨大な colspan / rowspan・不正な画像サイズ
    let cells = "<td colspan=1000 rowspan=1000>c</td>".repeat(40);
    let html = format!(
        "<table><tr>{cells}</tr><tr><td>z</td></tr></table><img width=inf height=NaN><ol start=2147483647><li>a<li>b</ol>"
    );
    lay(&html, RESET);
}

fn inline_of<'a>(laid: &'a Laid, id: &str) -> &'a kumo_layout::InlineContent {
    let b = laid
        .tree
        .boxes
        .iter()
        .find(|b| b.dom.and_then(|d| laid.doc.attr(d, "id")) == Some(id))
        .unwrap_or_else(|| panic!("#{id}"));
    assert_eq!(b.children.len(), 1, "#{id} の段落は1つ");
    laid.tree.boxes[b.children[0]].inline().expect("インライン")
}

#[test]
fn line_moves_below_float_when_gap_is_too_narrow() {
    // フロート横の隙間は 30px しかなく、長い単語が入らない
    let css = "body { margin: 0; font-size: 16px; line-height: 20px } \
               #f { float: right; width: 770px; height: 100px } p { margin: 0 }";
    let laid = lay(
        "<div id=f></div><p id=p>Incomprehensibilities everywhere</p>",
        css,
    );
    let plan = inline_of(&laid, "p")
        .final_breaks
        .clone()
        .expect("確定済み");
    let first = plan.lines.first().expect("1行以上");
    assert!(first.y >= 100.0, "最初の行はフロートの下: y={}", first.y);
    assert_eq!(first.width, W, "フロートの下では全幅");
}

#[test]
fn float_inside_paragraph_does_not_split_it() {
    let css = "body { margin: 0; font-size: 16px; line-height: 20px } p { margin: 0 } \
               #f { float: right; width: 300px; height: 60px }";
    let words = "word ".repeat(80);
    let html = format!("<p id=p>start {words}<span id=f></span> {words} end</p>");
    let laid = lay(&html, css);
    let content = inline_of(&laid, "p");
    assert!(content.text.starts_with("start") && content.text.ends_with("end"));
    let (fx, fy, fw, fh) = rect(&laid, "f");
    assert_eq!((fx, fw, fh), (W - 300.0, 300.0, 60.0));
    let plan = content.final_breaks.as_ref().unwrap();
    // フロートより下の行は全幅、横の行は狭い
    let beside: Vec<_> = plan
        .lines
        .iter()
        .filter(|l| l.y >= fy && l.y < fy + fh)
        .collect();
    assert!(!beside.is_empty());
    assert!(beside.iter().all(|l| l.width <= W - 300.0 + 0.5));
    assert!(plan.lines.iter().any(|l| l.y >= fy + fh && l.width == W));
}

#[test]
fn final_breaks_match_painted_state_everywhere() {
    let css = "body { margin: 0 } .row { display: flex } .row > div { flex: 1 } \
               .ib { display: inline-block; width: 30% } #f { float: left; width: 120px; height: 80px }";
    let words = "lorem ipsum dolor sit amet ".repeat(20);
    let html = format!(
        "<div id=f></div><p>{words}</p><div class=row><div>{words}<span class=ib>{words}</span></div>\
         <div>{words}</div></div><p>{words}</p>"
    );
    let laid = lay(&html, css);
    for b in &laid.tree.boxes {
        if let Some(content) = b.inline() {
            assert_eq!(
                content.breaks, content.final_breaks,
                "描画される改行は確定済みのもの"
            );
        }
    }
}

#[test]
fn pushed_lines_count_toward_paragraph_height_and_left_floats_offset_once() {
    let css = "body { margin: 0; font-size: 16px; line-height: 20px } p { margin: 0 } \
               #f { float: right; width: 770px; height: 100px } \
               #g { float: left; width: 150px; height: 70px }";
    let html = "<div id=f></div><p id=p>Incomprehensibilities everywhere</p><p id=q>xx <span id=g></span> yy</p>";
    let laid = lay(html, css);
    let (_, py, _, ph) = rect(&laid, "p");
    assert!(
        py + ph >= 120.0,
        "下へ送った行まで段落に含む: y={py} h={ph}"
    );
    let (gx, _, gw, _) = rect(&laid, "g");
    let q = inline_of(&laid, "q");
    let glyph_x = q
        .layout
        .as_ref()
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .items()
        .find_map(|item| match item {
            kumo_text::PositionedLayoutItem::GlyphRun(run) => Some(run.offset()),
            _ => None,
        });
    assert_eq!(gx, 0.0);
    assert_eq!(
        glyph_x,
        Some(gx + gw),
        "左フロートの右から1回だけずれて始まる"
    );
}

#[test]
fn tall_line_avoids_float_that_starts_mid_line() {
    // 行の高さ 60px。2本目のフロートは y=30 から始まり、1行目の途中に食い込む
    let css = "body { margin: 0; font-size: 16px; line-height: 60px } p { margin: 0 } \
               #a { float: right; width: 10px; height: 30px } \
               #b { float: right; clear: right; width: 400px; height: 100px }";
    let laid = lay(
        "<div id=a></div><div id=b></div><p id=p>short words only here</p>",
        css,
    );
    let (_, by, _, _) = rect(&laid, "b");
    assert_eq!(by, 30.0);
    let plan = inline_of(&laid, "p").final_breaks.clone().unwrap();
    let first = plan.lines[0];
    assert_eq!(first.y, 0.0);
    assert!(
        first.width <= W - 400.0 + 0.5,
        "行全体の高さで重なるフロートを避ける: {first:?}"
    );
}
