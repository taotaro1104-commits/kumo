//! フォーム部品（input / select / textarea）の表示。値・プレースホルダ・チェック印などを
//! 中身のテキストとして持つコンテナにする（置換要素の空箱にはしない）。

use std::sync::Arc;

use kumo_css::ComputedStyle;
use kumo_css::values::Rgba;
use kumo_dom::{Document, NodeId as DomId};

use crate::boxes::BoxKind;
use crate::build::Builder;
use crate::inline_builder::InlineBuilder;

/// 入力中（フォーカス中）の部品に Browser が付ける属性。
pub const FOCUS_ATTR: &str = "data-kumo-focused";

/// プレースホルダの文字色。
const PLACEHOLDER: Rgba = Rgba::new(117, 117, 117, 255);
/// 空の欄でも1行分の高さを持たせるためのゼロ幅スペース。
const EMPTY_LINE: &str = "\u{200B}";

/// 部品に表示する文字列。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlText {
    pub text: String,
    pub placeholder: bool,
}

impl ControlText {
    fn value(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            placeholder: false,
        }
    }
}

/// `input` の type（小文字、既定は text）。
pub fn input_type(doc: &Document, node: DomId) -> String {
    doc.attr(node, "type")
        .map(|t| t.trim().to_ascii_lowercase())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "text".to_owned())
}

/// フォーム部品なら表示する文字列を返す。部品でない・表示しない（hidden・image）なら `None`。
pub fn control_text(doc: &Document, node: DomId) -> Option<ControlText> {
    let element = doc.element(node)?;
    if element.is_html("textarea") {
        let value = element
            .attr("value")
            .map(str::to_owned)
            .unwrap_or_else(|| doc.text_content(node));
        return Some(with_placeholder(value, element.attr("placeholder")));
    }
    if element.is_html("select") {
        return Some(ControlText::value(format!(
            "{} ▾",
            selected_option_text(doc, node)
        )));
    }
    if !element.is_html("input") {
        return None;
    }
    let checked = element.attr("checked").is_some();
    let value = element.attr("value").unwrap_or("");
    Some(match input_type(doc, node).as_str() {
        "hidden" | "image" => return None,
        "checkbox" => ControlText::value(if checked { "✓" } else { "" }),
        "radio" => ControlText::value(if checked { "●" } else { "" }),
        "submit" => ControlText::value(if value.is_empty() { "送信" } else { value }),
        "reset" => ControlText::value(if value.is_empty() {
            "リセット"
        } else {
            value
        }),
        "button" => ControlText::value(value),
        "file" => ControlText::value("ファイルを選択"),
        "password" => ControlText::value("•".repeat(value.chars().count())),
        _ => with_placeholder(value.to_owned(), element.attr("placeholder")),
    })
}

fn with_placeholder(value: String, placeholder: Option<&str>) -> ControlText {
    match placeholder {
        Some(p) if value.is_empty() && !p.is_empty() => ControlText {
            text: p.to_owned(),
            placeholder: true,
        },
        _ => ControlText::value(value),
    }
}

/// `select` で選ばれている（なければ最初の）`option` の文字列。
pub fn selected_option_text(doc: &Document, select: DomId) -> String {
    let options: Vec<DomId> = doc
        .descendants(select)
        .filter(|&n| doc.element(n).is_some_and(|e| e.is_html("option")))
        .collect();
    options
        .iter()
        .find(|&&o| doc.attr(o, "selected").is_some())
        .or_else(|| options.first())
        .map(|&o| {
            doc.text_content(o)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

impl Builder<'_> {
    /// フォーム部品のボックスを作る。部品でなければ `None`。
    pub(crate) fn build_form_control(
        &mut self,
        node: DomId,
        style: &Arc<ComputedStyle>,
    ) -> Option<usize> {
        let content = control_text(self.doc, node)?;
        let index = self.add_box(BoxKind::Container, Some(node), style.clone());
        self.boxes[index].form_control = true;
        let mut ifc = InlineBuilder::new(style.clone(), Some(node), self.current_link());
        let focused = self.doc.attr(node, FOCUS_ATTR).is_some();
        let mut text = if content.text.is_empty() {
            EMPTY_LINE.to_owned()
        } else {
            content.text
        };
        if focused {
            // 入力中の欄はプレースホルダを消してキャレットを出す
            if content.placeholder {
                text = String::new();
            }
            text.push('|');
        }
        let content = ControlText {
            text,
            placeholder: content.placeholder && !focused,
        };
        let text = content.text.clone();
        if content.placeholder {
            let mut grey = (**style).clone();
            grey.color = PLACEHOLDER;
            ifc.open(Arc::new(grey), node, None);
            ifc.push_text(&text);
            ifc.close();
        } else {
            ifc.push_text(&text);
        }
        self.flush(index, &mut ifc, style);
        Some(index)
    }
}
