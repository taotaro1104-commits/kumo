//! ページ上の操作：スクロール、クリック（リンク・フォーム部品）、入力欄へのキー入力。
//! スクリプトがあるページでは、操作を DOM のイベント（click・keydown・input・change・submit）としても送り、
//! `preventDefault` されたら既定の動作（リンクの移動やフォームの送信）をしない。

use kumo_dom::{Document, NodeId};
use kumo_js::InitValue;
use kumo_layout::FOCUS_ATTR;
use url::Url;

use crate::engine::ContentEngine;
use crate::forms::{self, ControlClick};
use crate::protocol::{FieldKey, FromContent, HoverKind, TabId};

impl ContentEngine {
    pub(crate) fn scroll_by(&mut self, id: TabId, dy: f32) {
        let height = self.viewport_f32().1;
        let Some(tab) = self.tabs.get_mut(&id) else {
            return;
        };
        let before = tab.scroll_y;
        tab.scroll_y += dy;
        tab.clamp_scroll(height);
        if tab.scroll_y != before {
            self.scrolled(id);
        }
    }

    /// 表示中の文書の `#id` へ。見つからなければ先頭へ（`#` なしの戻る・進むを含む）。
    pub(crate) fn scroll_to_fragment(&mut self, id: TabId, url: &str) {
        let height = self.viewport_f32().1;
        let Some(tab) = self.tabs.get_mut(&id) else {
            return;
        };
        let Some(page) = &tab.page else { return };
        let target = Url::parse(url).ok().and_then(|u| crate::anchor_y(page, &u));
        tab.scroll_y = target.unwrap_or(0.0);
        tab.clamp_scroll(height);
        self.scrolled(id);
    }

    fn scrolled(&mut self, id: TabId) {
        let Some(tab) = self.tabs.get(&id) else {
            return;
        };
        self.out.push(FromContent::Scrolled {
            tab: id,
            y: tab.scroll_y,
        });
        if self.active == Some(id) {
            self.frame.touch();
        }
    }

    /// マウスの下にあるものを調べ、前回から変わっていれば知らせる。
    pub(crate) fn hover(&mut self, id: TabId, x: f32, y: f32) {
        let Some(tab) = self.tabs.get(&id) else {
            return;
        };
        let Some(page) = &tab.page else { return };
        let doc_y = y + tab.scroll_y;
        let found = if let Some(node) = page.list.control_at(x, doc_y) {
            let node = NodeId::from_index(node);
            let field = node.index() < page.document().len()
                && forms::is_text_field(&page.document(), node);
            (
                None,
                if field {
                    HoverKind::Field
                } else {
                    HoverKind::Control
                },
            )
        } else if let Some(url) = page
            .list
            .link_at(x, doc_y)
            .and_then(|href| page.resolve(href))
        {
            (Some(url.to_string()), HoverKind::Link)
        } else {
            (None, HoverKind::None)
        };
        let Some(tab) = self.tabs.get_mut(&id) else {
            return;
        };
        let unchanged = tab.hover.as_ref() == Some(&found)
            || (tab.hover.is_none() && found.1 == HoverKind::None);
        if unchanged {
            return;
        }
        tab.hover = Some(found.clone());
        let (link, kind) = found;
        self.out.push(FromContent::Hovered {
            tab: id,
            link,
            kind,
        });
    }

    /// コンテンツ領域でのクリック：スクリプトのイベント、フォーム部品、リンクの順に調べる。
    /// 何もなければ入力欄のフォーカスを外す。
    pub(crate) fn click(&mut self, id: TabId, x: f32, y: f32, new_tab: bool) {
        let Some(tab) = self.tabs.get(&id) else {
            return;
        };
        let Some(page) = &tab.page else { return };
        let doc_y = y + tab.scroll_y;
        let target = page.hit_node(x, doc_y);
        let control = page.list.control_at(x, doc_y);
        let link = page
            .list
            .link_at(x, doc_y)
            .and_then(|href| page.resolve(href));
        if let Some(node) = target {
            let init = [
                ("clientX", InitValue::Num(f64::from(x))),
                ("clientY", InitValue::Num(f64::from(y))),
                ("button", InitValue::Num(0.0)),
                ("ctrlKey", InitValue::Bool(new_tab)),
            ];
            if !self.js_event(id, node, "MouseEvent", "click", &init) {
                return; // スクリプトが既定の動作を止めた
            }
        }
        if !new_tab && let Some(node) = control {
            self.click_control(id, NodeId::from_index(node));
            return;
        }
        match link {
            // `javascript:` はスクリプトの実行（ここでは何もしない）
            Some(url) if url.scheme() == "javascript" => {}
            // 遠隔のページからローカルのファイルへは移動させない
            Some(url) if !self.may_navigate(id, &url) => {}
            Some(url) => {
                self.blur(id);
                self.out.push(FromContent::Navigate {
                    tab: id,
                    url: url.to_string(),
                    post_body: None,
                    new_tab,
                });
            }
            None => self.blur(id),
        }
    }

    fn click_control(&mut self, id: TabId, node: NodeId) {
        // 別の部品を押したら入力中の欄からフォーカスが移る
        if self.tabs.get(&id).is_some_and(|t| t.focused != Some(node)) {
            self.blur(id);
        }
        let outcome = {
            let Some(page) = self.tabs.get(&id).and_then(|t| t.page.as_ref()) else {
                return;
            };
            let mut doc = page.document_mut();
            if node.index() >= doc.len() {
                return;
            }
            forms::click_control(&mut doc, node)
        };
        match outcome {
            ControlClick::Focus(field)
                if self.tabs.get(&id).is_some_and(|t| t.focused == Some(field)) => {}
            ControlClick::Focus(field) => {
                let value = self.field_value_of(id, field);
                self.edit(id, |doc| doc.set_attr(field, FOCUS_ATTR, ""));
                if let Some(tab) = self.tabs.get_mut(&id) {
                    tab.focused = Some(field);
                    tab.focus_value = Some(value);
                }
                self.out.push(FromContent::FieldFocus {
                    tab: id,
                    focused: true,
                });
            }
            ControlClick::Changed => {
                self.edit(id, |_| {});
                self.js_event(id, node, "Event", "input", &[]);
                self.js_event(id, node, "Event", "change", &[]);
            }
            ControlClick::Submit { form, submitter } => self.submit(id, form, Some(submitter)),
            ControlClick::Nothing => {}
        }
    }

    fn field_value_of(&self, id: TabId, field: NodeId) -> String {
        self.tabs
            .get(&id)
            .and_then(|t| t.page.as_ref())
            .map(|p| forms::field_value(&p.document(), field))
            .unwrap_or_default()
    }

    pub(crate) fn key(&mut self, id: TabId, key: FieldKey) {
        let Some(field) = self.tabs.get(&id).and_then(|t| t.focused) else {
            return;
        };
        let name = match &key {
            FieldKey::Text(text) => text.clone(),
            FieldKey::Backspace => "Backspace".to_owned(),
            FieldKey::Enter => "Enter".to_owned(),
            FieldKey::Blur => String::new(),
        };
        if !matches!(key, FieldKey::Blur)
            && !self.js_event(
                id,
                field,
                "KeyboardEvent",
                "keydown",
                &[("key", InitValue::Str(name))],
            )
        {
            return; // スクリプトが入力を止めた
        }
        match key {
            FieldKey::Text(text) => {
                self.edit(id, |doc| forms::insert_text(doc, field, &text));
                self.js_event(
                    id,
                    field,
                    "InputEvent",
                    "input",
                    &[("data", InitValue::Str(text))],
                );
            }
            FieldKey::Backspace => {
                self.edit(id, |doc| forms::backspace(doc, field));
                self.js_event(id, field, "InputEvent", "input", &[]);
            }
            FieldKey::Blur => self.blur(id),
            FieldKey::Enter => {
                let (is_textarea, form) = {
                    let Some(page) = self.tabs.get(&id).and_then(|t| t.page.as_ref()) else {
                        return;
                    };
                    let doc = page.document();
                    (
                        doc.element(field).is_some_and(|e| e.is_html("textarea")),
                        forms::form_of(&doc, field),
                    )
                };
                if is_textarea {
                    self.edit(id, |doc| forms::insert_text(doc, field, "\n"));
                    self.js_event(id, field, "InputEvent", "input", &[]);
                } else if let Some(form) = form {
                    self.submit(id, form, None);
                }
            }
        }
    }

    /// フォームを送信する。スクリプトの `submit` イベントが止めたら送らない。
    fn submit(&mut self, id: TabId, form: NodeId, submitter: Option<NodeId>) {
        if !self.js_event(id, form, "Event", "submit", &[]) {
            return;
        }
        let submission = {
            let Some(page) = self.tabs.get(&id).and_then(|t| t.page.as_ref()) else {
                return;
            };
            forms::build_submission(&page.document(), page.url(), form, submitter)
        };
        let Some(submission) = submission else { return };
        self.report_login(id, form);
        self.blur(id);
        self.out.push(FromContent::Navigate {
            tab: id,
            url: submission.url.to_string(),
            post_body: submission.post_body,
            new_tab: false,
        });
    }

    /// 入力欄のフォーカスを外す。値が変わっていれば `change` を送る。
    pub(crate) fn blur(&mut self, id: TabId) {
        let Some(tab) = self.tabs.get_mut(&id) else {
            return;
        };
        let (Some(field), before) = (tab.focused.take(), tab.focus_value.take()) else {
            return;
        };
        self.edit(id, |doc| doc.remove_attr(field, FOCUS_ATTR));
        self.out.push(FromContent::FieldFocus {
            tab: id,
            focused: false,
        });
        if before.is_some_and(|b| b != self.field_value_of(id, field)) {
            self.js_event(id, field, "Event", "change", &[]);
        }
    }

    /// DOM を変更し、次の描画前に（まとめて1回）再レイアウトさせる。
    pub(crate) fn edit(&mut self, id: TabId, edit: impl FnOnce(&mut Document)) {
        let Some(tab) = self.tabs.get_mut(&id) else {
            return;
        };
        let Some(page) = tab.page.as_mut() else {
            return;
        };
        edit(&mut page.document_mut());
        tab.needs_layout = true;
        if self.active == Some(id) {
            self.frame.touch();
        }
    }
}
