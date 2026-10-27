//! フォーム：部品のクリック・値の編集・送信内容の組み立て（HTML の form submission の主要部分）。
//! 値は DOM の属性（`value`・`checked`・`selected`）として持ち、表示はレイアウトのやり直しで反映する。

use kumo_dom::{Document, NodeId};
use kumo_layout::input_type;
use url::Url;

/// 部品をクリックした結果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlClick {
    /// 文字入力欄にフォーカスする
    Focus(NodeId),
    /// チェック・選択が変わった（再描画が必要）
    Changed,
    /// フォームを送信する
    Submit { form: NodeId, submitter: NodeId },
    /// 何もしない
    Nothing,
}

/// フォームの送信内容。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    pub url: Url,
    /// POST のときの本文（application/x-www-form-urlencoded）
    pub post_body: Option<Vec<u8>>,
}

const TEXT_LIKE: [&str; 9] = [
    "text", "search", "email", "url", "tel", "password", "number", "date", "time",
];

fn is_html(doc: &Document, node: NodeId, tag: &str) -> bool {
    doc.element(node).is_some_and(|e| e.is_html(tag))
}

/// 文字を入力できる部品か。
pub fn is_text_field(doc: &Document, node: NodeId) -> bool {
    is_html(doc, node, "textarea")
        || (is_html(doc, node, "input") && TEXT_LIKE.contains(&input_type(doc, node).as_str()))
}

/// 部品が属するフォーム（最も近い祖先の `<form>`）。
pub fn form_of(doc: &Document, node: NodeId) -> Option<NodeId> {
    let mut current = doc.parent(node);
    while let Some(n) = current {
        if is_html(doc, n, "form") {
            return Some(n);
        }
        current = doc.parent(n);
    }
    None
}

pub fn click_control(doc: &mut Document, node: NodeId) -> ControlClick {
    if doc.attr(node, "disabled").is_some() {
        return ControlClick::Nothing;
    }
    if is_text_field(doc, node) {
        return ControlClick::Focus(node);
    }
    if is_html(doc, node, "select") {
        cycle_select(doc, node);
        return ControlClick::Changed;
    }
    let submit = |doc: &Document| match form_of(doc, node) {
        Some(form) => ControlClick::Submit {
            form,
            submitter: node,
        },
        None => ControlClick::Nothing,
    };
    if is_html(doc, node, "button") {
        let kind = doc
            .attr(node, "type")
            .unwrap_or("submit")
            .to_ascii_lowercase();
        return if kind == "submit" {
            submit(doc)
        } else {
            ControlClick::Nothing
        };
    }
    match input_type(doc, node).as_str() {
        "checkbox" => {
            if doc.attr(node, "checked").is_some() {
                doc.remove_attr(node, "checked");
            } else {
                doc.set_attr(node, "checked", "");
            }
            ControlClick::Changed
        }
        "radio" => {
            check_radio(doc, node);
            ControlClick::Changed
        }
        "submit" | "image" => submit(doc),
        _ => ControlClick::Nothing,
    }
}

/// ラジオボタンを選び、同じフォーム・同じ名前の他のものを外す。
fn check_radio(doc: &mut Document, node: NodeId) {
    let name = doc.attr(node, "name").map(str::to_owned);
    let scope = form_of(doc, node).unwrap_or(doc.root());
    if let Some(name) = name {
        let group: Vec<NodeId> = doc
            .descendants(scope)
            .filter(|&n| {
                is_html(doc, n, "input")
                    && input_type(doc, n) == "radio"
                    && doc.attr(n, "name") == Some(name.as_str())
            })
            .collect();
        for other in group {
            doc.remove_attr(other, "checked");
        }
    }
    doc.set_attr(node, "checked", "");
}

fn options(doc: &Document, select: NodeId) -> Vec<NodeId> {
    doc.descendants(select)
        .filter(|&n| is_html(doc, n, "option"))
        .collect()
}

/// `select` の選択を次の `option` へ送る（ドロップダウンの代わりの簡易操作）。
fn cycle_select(doc: &mut Document, select: NodeId) {
    let all = options(doc, select);
    if all.is_empty() {
        return;
    }
    let current = all
        .iter()
        .position(|&o| doc.attr(o, "selected").is_some())
        .unwrap_or(0);
    for &o in &all {
        doc.remove_attr(o, "selected");
    }
    doc.set_attr(all[(current + 1) % all.len()], "selected", "");
}

/// 文字入力欄の現在の値。
pub fn field_value(doc: &Document, node: NodeId) -> String {
    match doc.attr(node, "value") {
        Some(v) => v.to_owned(),
        None if is_html(doc, node, "textarea") => doc.text_content(node),
        None => String::new(),
    }
}

/// 入力欄に文字を足す（改行は textarea のみ）。
pub fn insert_text(doc: &mut Document, node: NodeId, text: &str) {
    let multiline = is_html(doc, node, "textarea");
    let mut value = field_value(doc, node);
    value.extend(
        text.chars()
            .filter(|&c| !c.is_control() || (multiline && c == '\n')),
    );
    doc.set_attr(node, "value", &value);
}

/// 入力欄の最後の1文字を消す。
pub fn backspace(doc: &mut Document, node: NodeId) {
    let mut value = field_value(doc, node);
    value.pop();
    doc.set_attr(node, "value", &value);
}

/// 送信する名前と値の組（HTML の「フォームデータの構築」の主要部分）。
fn form_data(doc: &Document, form: NodeId, submitter: Option<NodeId>) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for node in doc.descendants(form) {
        let Some(element) = doc.element(node) else {
            continue;
        };
        let Some(name) = element.attr("name").filter(|n| !n.is_empty()) else {
            continue;
        };
        if element.attr("disabled").is_some() {
            continue;
        }
        let value = if element.is_html("input") {
            match input_type(doc, node).as_str() {
                "checkbox" | "radio" if element.attr("checked").is_none() => continue,
                "checkbox" | "radio" => element.attr("value").unwrap_or("on").to_owned(),
                "submit" | "image" | "button" | "reset" if Some(node) != submitter => continue,
                "file" => continue,
                _ => element.attr("value").unwrap_or("").to_owned(),
            }
        } else if element.is_html("button") {
            if Some(node) != submitter {
                continue;
            }
            element.attr("value").unwrap_or("").to_owned()
        } else if element.is_html("select") {
            let all = options(doc, node);
            let Some(&chosen) = all
                .iter()
                .find(|&&o| doc.attr(o, "selected").is_some())
                .or_else(|| all.first())
            else {
                continue;
            };
            doc.attr(chosen, "value")
                .map(str::to_owned)
                .unwrap_or_else(|| doc.text_content(chosen).trim().to_owned())
        } else if element.is_html("textarea") {
            field_value(doc, node)
        } else {
            continue;
        };
        pairs.push((name.to_owned(), value));
    }
    pairs
}

/// フォームの送信先と本文を組み立てる。`base` はページの URL。
pub fn build_submission(
    doc: &Document,
    base: &Url,
    form: NodeId,
    submitter: Option<NodeId>,
) -> Option<Submission> {
    let action = submitter
        .and_then(|s| doc.attr(s, "formaction"))
        .or_else(|| doc.attr(form, "action"))
        .map(str::trim)
        .filter(|a| !a.is_empty());
    let mut url = match action {
        Some(action) => base.join(action).ok()?,
        None => base.clone(),
    };
    url.set_fragment(None);
    let method = submitter
        .and_then(|s| doc.attr(s, "formmethod"))
        .or_else(|| doc.attr(form, "method"))
        .unwrap_or("get")
        .to_ascii_lowercase();
    let encoded: String = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(form_data(doc, form, submitter))
        .finish();
    if method == "post" {
        return Some(Submission {
            url,
            post_body: Some(encoded.into_bytes()),
        });
    }
    url.set_query(Some(&encoded));
    Some(Submission {
        url,
        post_body: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(doc: &Document, name: &str) -> NodeId {
        doc.descendants(doc.root())
            .find(|&n| doc.attr(n, "name") == Some(name) || doc.attr(n, "id") == Some(name))
            .unwrap_or_else(|| panic!("{name}"))
    }

    const FORM: &str = "<form id=f action='/search' method=get>\
        <input name=q value='rust'>\
        <input type=hidden name=lang value=ja>\
        <input type=checkbox name=exact value=1>\
        <input type=radio name=sort value=new checked><input type=radio name=sort value=old>\
        <select name=size><option value=s>S<option value=m selected>M<option>L</select>\
        <textarea name=note>hello</textarea>\
        <input type=submit name=go value=Go><button name=alt value=b>Alt</button>\
        <input name=off disabled value=x><input value=noname>\
        </form><input id=outside>";

    #[test]
    fn get_submission_collects_successful_controls() {
        let doc = kumo_html::parse_html(FORM);
        let base = Url::parse("https://example.com/page?old=1#top").unwrap();
        let form = find(&doc, "f");
        let go = find(&doc, "go");
        let s = build_submission(&doc, &base, form, Some(go)).unwrap();
        assert_eq!(
            s.url.as_str(),
            "https://example.com/search?q=rust&lang=ja&sort=new&size=m&note=hello&go=Go"
        );
        assert!(s.post_body.is_none());
    }

    #[test]
    fn clicking_controls_changes_state() {
        let mut doc = kumo_html::parse_html(FORM);
        let exact = find(&doc, "exact");
        assert_eq!(click_control(&mut doc, exact), ControlClick::Changed);
        assert!(doc.attr(exact, "checked").is_some());
        let radios: Vec<NodeId> = doc
            .descendants(doc.root())
            .filter(|&n| doc.attr(n, "name") == Some("sort"))
            .collect();
        click_control(&mut doc, radios[1]);
        assert!(
            doc.attr(radios[0], "checked").is_none(),
            "同じグループは1つだけ"
        );
        assert!(doc.attr(radios[1], "checked").is_some());
        let size = find(&doc, "size");
        click_control(&mut doc, size);
        assert_eq!(kumo_layout::selected_option_text(&doc, size), "L");
        let q = find(&doc, "q");
        assert_eq!(click_control(&mut doc, q), ControlClick::Focus(q));
        insert_text(&mut doc, q, " lang\u{7}");
        backspace(&mut doc, q);
        assert_eq!(field_value(&doc, q), "rust lan");
        let alt = find(&doc, "alt");
        let form = find(&doc, "f");
        assert_eq!(
            click_control(&mut doc, alt),
            ControlClick::Submit {
                form,
                submitter: alt
            }
        );
        let outside = find(&doc, "outside");
        assert_eq!(form_of(&doc, outside), None);
    }

    #[test]
    fn post_submission_uses_body() {
        let doc = kumo_html::parse_html(
            "<form id=f method=POST action='https://other.example/login'><input name=user value='a b&c'></form>",
        );
        let base = Url::parse("https://example.com/").unwrap();
        let s = build_submission(&doc, &base, find(&doc, "f"), None).unwrap();
        assert_eq!(s.url.as_str(), "https://other.example/login");
        assert_eq!(s.post_body.as_deref(), Some(&b"user=a+b%26c"[..]));
    }
}
