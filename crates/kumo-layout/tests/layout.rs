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
fn percent_side_columns_keep_the_middle_cell_centered() {
    // google.com の検索欄：25% / 自動 / 25% の表を <center> に置く
    let html = "<center><table id=t><tr><td width=25%>&nbsp;</td>                <td id=m><div style='width:200px;height:10px'></div></td>                <td width=25% id=r>opt</td></tr></table></center>";
    let laid = lay(
        html,
        "body { margin: 0 } table { border-spacing: 0 } td { padding: 0 }",
    );
    let (mx, _, mw, _) = rect(&laid, "m");
    assert!(
        ((mx + mw / 2.0) - W / 2.0).abs() < 1.0,
        "中央の列は中央: x={mx} w={mw}"
    );
}

#[test]
fn table_directly_inside_center_is_centered() {
    let laid = lay(
        "<center><table id=t><tr><td>x</td></tr></table></center>",
        "body { margin: 0 } td { padding: 0 }",
    );
    let (tx, _, tw, _) = rect(&laid, "t");
    assert!(((tx + tw / 2.0) - W / 2.0).abs() < 1.0, "x={tx} w={tw}");
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
fn image_with_only_width_keeps_aspect_ratio() {
    // 固有サイズが無いときの縦横比は 1:1。幅だけ指定した画像の高さが 0 に潰れていた
    let laid = lay("<img id=i width=40>", RESET);
    let (_, _, w, h) = rect(&laid, "i");
    assert_eq!((w, h), (40.0, 40.0));
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

#[test]
fn clearfix_after_pseudo_element_contains_floats() {
    let css = "body { margin: 0 } #wrap:after { content: \"\"; display: table; clear: both }\
               #f { float: left; width: 100px; height: 60px } #next { height: 10px }";
    let laid = lay(
        "<div id=wrap><div id=f></div></div><div id=next></div>",
        css,
    );
    // ::after がフロートを囲み、wrap の高さが 60、次の要素は 60 から始まる
    assert_eq!(rect(&laid, "wrap").3, 60.0);
    assert_eq!(rect(&laid, "next").1, 60.0);
}

#[test]
fn before_content_adds_inline_text_and_attr() {
    let css = "body { margin: 0 } p { margin: 0 } #a { font-size: 20px; line-height: 20px }\
               #a:before { content: \"ab\" attr(data-x) }";
    let with = lay("<p id=a data-x=cd>x</p>", css);
    let without = lay(
        "<p id=a data-x=cd>x</p>",
        "body{margin:0} p{margin:0} #a{font-size:20px;line-height:20px}",
    );
    // 疑似要素のテキスト（"ab" + 属性 "cd"）が段落に入る：高さは同じで 1 行のまま
    assert_eq!(rect(&with, "a").3, rect(&without, "a").3);
    assert_eq!(
        inline_text_len(&with, "a"),
        inline_text_len(&without, "a") + 4
    );
}

#[test]
fn pseudo_without_content_or_with_none_is_not_generated() {
    let css = "body { margin: 0 } #a { height: 10px } #a:after { display: block; height: 50px }\
               #b { height: 10px } #b:after { content: none; display: block; height: 50px }";
    let laid = lay("<div id=a></div><div id=b></div>", css);
    assert_eq!(rect(&laid, "a").3, 10.0);
    assert_eq!(rect(&laid, "b").1, 10.0);
}

fn inline_text_len(laid: &Laid, id: &str) -> usize {
    fn find(laid: &Laid, index: usize, id: &str) -> Option<usize> {
        let b = &laid.tree.boxes[index];
        if b.dom.and_then(|d| laid.doc.attr(d, "id")) == Some(id) {
            return Some(index);
        }
        b.children.iter().find_map(|&c| find(laid, c, id))
    }
    let index = find(laid, 0, id).expect("要素");
    laid.tree.boxes[index]
        .children
        .iter()
        .filter_map(|&c| match &laid.tree.boxes[c].kind {
            BoxKind::Inline(content) => Some(content.text.chars().count()),
            _ => None,
        })
        .sum()
}

#[test]
fn clear_never_moves_a_block_above_its_normal_position() {
    let css = "body { margin: 0 } #fl { float: right; width: 100px; height: 20px }\
               #a { height: 50px } #b { clear: both; height: 10px }";
    let laid = lay("<div id=fl></div><div id=a></div><div id=b></div>", css);
    // フロートの下端（20）より、通常の位置（50）が下にあるので 50 のまま
    assert_eq!(rect(&laid, "b").1, 50.0);
    let css2 = "body { margin: 0 } #fl { float: right; width: 100px; height: 80px }\
                #a { height: 50px } #b { clear: both; height: 10px }";
    let laid2 = lay("<div id=fl></div><div id=a></div><div id=b></div>", css2);
    assert_eq!(rect(&laid2, "b").1, 80.0, "フロートの下端まで下がる");
}

#[test]
fn consecutive_cleared_blocks_stack_normally() {
    let css = "body { margin: 0 } #fl { float: right; width: 100px; height: 80px }               #a { clear: both; height: 50px } #b { clear: both; height: 10px }";
    let laid = lay("<div id=fl></div><div id=a></div><div id=b></div>", css);
    assert_eq!(rect(&laid, "a").1, 80.0);
    assert_eq!(rect(&laid, "b").1, 130.0);
}

#[test]
fn cleared_relative_block_after_float_containing_block() {
    // nav の中身はフロートだがクリアフィックスで囲まれている。次の clear:both の相対配置ブロックは nav の下
    let css = "body { margin: 0 } #fl { float: right; width: 100px; height: 80px }               #a { clear: both; margin-bottom: 1px } #a:after { content: \"\"; display: table; clear: both }               #g { float: left; width: 50px; height: 55px }               #b { clear: both; position: relative; height: 10px }";
    let laid = lay(
        "<div id=fl></div><div id=a><div id=g></div></div><div id=b></div>",
        css,
    );
    assert_eq!(rect(&laid, "a").1, 80.0);
    assert_eq!(rect(&laid, "a").3, 55.0);
    assert_eq!(rect(&laid, "b").1, 136.0, "a の下端 135 + margin 1");
}

#[test]
fn clear_after_nav_with_two_floats_and_wrapper() {
    let css = "body { margin: 0 } .c { padding: 0 10px } #l { float: left; width: 100px; height: 30px; margin: 4px 0 8px }               #r { float: right; width: 100px; height: 200px }               #n { clear: both; text-align: center; margin-bottom: 1px } #n li { display: inline-block; position: relative; height: 53px }               #n ul:after { content: \"\"; display: table; clear: both }\n               #b { clear: both; position: relative; height: 10px }";
    let laid = lay(
        "<div class=c><div id=l></div><div id=r></div><div id=n><ul style='margin:0;padding:0'><li>a</li><li>b</li></ul></div><div id=b></div></div>",
        css,
    );
    let (n, b) = (rect(&laid, "n"), rect(&laid, "b"));
    assert_eq!(n.1, 200.0);
    assert!(b.1 >= n.1 + n.3, "b={b:?} n={n:?}");
}

#[test]
fn cleared_block_containing_floats_stays_below_the_previous_sibling() {
    let css = "body { margin: 0 } #r { float: right; width: 100px; height: 200px }               #n { clear: both; height: 55px; margin-bottom: 1px }               #b { clear: both; position: relative } #b ul { margin: 0; padding: 0; list-style: none }               #b ul:after { content: \"\"; display: table; clear: both }               #b li { float: left; width: 100%; margin-right: -100%; position: relative; height: 80px }";
    let laid = lay(
        "<div id=r></div><div id=n></div><div id=b><ul><li>x</li></ul></div>",
        css,
    );
    assert_eq!(rect(&laid, "n").1, 200.0);
    assert_eq!(rect(&laid, "b").1, 256.0, "n の下端 255 + margin 1");
    assert_eq!(rect(&laid, "b").3, 80.0);
}

#[test]
fn cleared_block_with_inner_floats_variants() {
    let base = |b_css: &str, li_css: &str, after: &str| {
        let css = format!(
            "body {{ margin: 0 }} #r {{ float: right; width: 100px; height: 200px }}            #n {{ clear: both; height: 55px }} #b {{ {b_css} }} #b ul {{ margin: 0; padding: 0 }} {after}            #b li {{ {li_css} height: 80px }}"
        );
        let laid = lay(
            "<div id=r></div><div id=n></div><div id=b><ul><li>x</li></ul></div>",
            &css,
        );
        rect(&laid, "b").1
    };
    let r = [
        base("clear: both", "float: left; width: 100%;", ""),
        base(
            "clear: both",
            "float: left; width: 100%;",
            "#b ul:after { content: \"\"; display: table; clear: both }",
        ),
        base("clear: both", "", ""),
        base("", "float: left; width: 100%;", ""),
        base("clear: both", "float: left; width: 50%;", ""),
        base(
            "clear: both; overflow: hidden",
            "float: left; width: 100%;",
            "",
        ),
        base(
            "clear: both; position: relative",
            "float: left; width: 100%; margin-right: -100%;",
            "",
        ),
    ];
    // どの組み合わせでも、直前の兄弟（#n の下端 255）の下に置かれる
    assert!(r.iter().all(|&y| y == 255.0), "{r:?}");
}

#[test]
fn math_functions_and_viewport_units_are_evaluated() {
    // 表示領域は 800x600
    let cases = [
        ("width: max(100px, calc(50vw - 720px + 1rem))", 100.0, 0.0),
        ("width: calc(100vw - 32px)", 768.0, 0.0),
        ("width: min(300px, 100%)", 300.0, 0.0),
        ("width: clamp(100px, 50%, 300px)", 300.0, 0.0),
        ("width: clamp(500px, 50%, 900px)", 500.0, 0.0),
        (
            "--p: max(16px, calc(50vw - 720px + 16px)); padding-left: var(--p); width: 100px",
            116.0,
            0.0,
        ),
        ("height: 100svh", 800.0, 600.0),
        ("height: 50dvh", 800.0, 300.0),
        ("height: 10vmin; width: 10vmax", 80.0, 60.0),
    ];
    for (decl, width, height) in cases {
        let css = format!("body {{ margin: 0 }} #a {{ {decl} }}");
        let laid = lay("<div id=a>x</div>", &css);
        let (_, _, w, h) = rect(&laid, "a");
        assert_eq!(w, width, "{decl}");
        if height > 0.0 {
            assert_eq!(h, height, "{decl}");
        }
    }
}

#[test]
fn grid_column_with_only_a_name_spans_the_named_lines() {
    let css = "body { margin: 0 } #g { display: grid; column-gap: 20px;        grid-template-columns: minmax(0,1fr) [decoration-start title-start] repeat(3,60px) [title-end content-start] repeat(12,60px) [content-end hide-start] 60px [decoration-end hide-end] minmax(0,1fr);        grid-template-rows: [headline-start controls-start] auto [controls-end headline-end content-toggleable-start content-start] auto [content-end content-toggleable-end bottom-content-start] auto [bottom-content-end] }        #c { grid-column: content; grid-row: content-toggleable; height: 30px }        #t { grid-column: title; grid-row: headline; height: 20px }";
    let laid = lay("<div id=g><div id=t>t</div><div id=c>c</div></div>", css);
    // content は 12 トラック（60 * 12 + 隙間 20 * 11）、title は 3 トラック（60 * 3 + 20 * 2）
    assert_eq!(rect(&laid, "c").2, 940.0);
    assert_eq!(rect(&laid, "t").2, 220.0);
}
