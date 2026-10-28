//! ログイン情報：送信するフォームから取り出す／保存した値をページの入力欄へ入れる。
//! 保存や暗号化は Browser の仕事。Content はページの DOM を読み書きするだけ。

use kumo_dom::{Document, NodeId};
use kumo_layout::input_type;
use url::Url;

use crate::engine::ContentEngine;
use crate::forms::{self, field_value};
use crate::protocol::{FromContent, TabId};

/// 文字を入れる欄（ユーザー名の候補）。
fn is_text_like(doc: &Document, node: NodeId) -> bool {
    doc.element(node).is_some_and(|e| e.is_html("input"))
        && matches!(
            input_type(doc, node).as_str(),
            "text" | "email" | "tel" | "search" | ""
        )
        && doc.attr(node, "disabled").is_none()
}

fn is_password(doc: &Document, node: NodeId) -> bool {
    doc.element(node).is_some_and(|e| e.is_html("input"))
        && input_type(doc, node) == "password"
        && doc.attr(node, "disabled").is_none()
}

/// `password` 欄の直前にある、ユーザー名の欄。
fn username_field(doc: &Document, root: NodeId, password: NodeId) -> Option<NodeId> {
    let mut found = None;
    for node in doc.descendants(root) {
        if node == password {
            break;
        }
        if is_text_like(doc, node) {
            found = Some(node);
        }
    }
    found
}

/// 送信するフォームのログイン情報（ユーザー名, パスワード）。パスワード欄に値がなければ `None`。
pub fn credentials(doc: &Document, form: NodeId) -> Option<(String, String)> {
    let password = doc
        .descendants(form)
        .find(|&n| is_password(doc, n) && !field_value(doc, n).is_empty())?;
    let user = username_field(doc, form, password)
        .map(|n| field_value(doc, n))
        .unwrap_or_default();
    Some((user.trim().to_owned(), field_value(doc, password)))
}

/// 空のパスワード欄とその前のユーザー名欄へ値を入れる。入れたら入れた欄を返す。
/// すでに何か入っている欄は上書きしない。
pub fn fill(doc: &mut Document, username: &str, password: &str) -> Vec<NodeId> {
    let root = doc.root();
    let Some(target) = doc.descendants(root).find(|&n| {
        is_password(doc, n) && doc.attr(n, "readonly").is_none() && field_value(doc, n).is_empty()
    }) else {
        return Vec::new();
    };
    let scope = forms::form_of(doc, target).unwrap_or(root);
    let mut filled = vec![target];
    if let Some(user) = username_field(doc, scope, target)
        .filter(|&u| field_value(doc, u).is_empty() && doc.attr(u, "readonly").is_none())
    {
        doc.set_attr(user, "value", username);
        filled.push(user);
    }
    doc.set_attr(target, "value", password);
    filled
}

impl ContentEngine {
    /// 送信するフォームにログイン情報があれば Browser へ知らせる（保存の確認用）。
    pub(crate) fn report_login(&mut self, id: TabId, form: NodeId) {
        let Some(page) = self.tabs.get(&id).and_then(|t| t.page.as_ref()) else {
            return;
        };
        let Some((username, password)) = credentials(&page.document(), form) else {
            return;
        };
        let url = page.url().to_string();
        self.out.push(FromContent::LoginSubmitted {
            tab: id,
            url,
            username,
            password,
        });
    }

    /// 保存したログイン情報をページに入れる。ページが `url` のままのときだけ。
    pub(crate) fn autofill(&mut self, id: TabId, url: &str, username: &str, password: &str) {
        let same = self
            .tabs
            .get(&id)
            .and_then(|t| t.page.as_ref())
            .is_some_and(|p| Url::parse(url).is_ok_and(|u| &u == p.url()));
        if !same {
            return;
        }
        let mut filled = Vec::new();
        self.edit(id, |doc| filled = fill(doc, username, password));
        for node in filled {
            self.js_event(id, node, "Event", "input", &[]);
            self.js_event(id, node, "Event", "change", &[]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(html: &str) -> Document {
        kumo_html::parse_html(html)
    }

    fn form(doc: &Document) -> NodeId {
        doc.descendants(doc.root())
            .find(|&n| doc.element(n).is_some_and(|e| e.is_html("form")))
            .unwrap()
    }

    const LOGIN: &str = "<form><input type=hidden name=t value=x><input name=u value=' alice '>\
                         <input type=password name=p value=secret><input type=submit></form>";

    #[test]
    fn reads_the_username_before_the_password() {
        let d = doc(LOGIN);
        assert_eq!(
            credentials(&d, form(&d)),
            Some(("alice".into(), "secret".into()))
        );
    }

    #[test]
    fn no_password_value_means_no_login() {
        let d = doc("<form><input name=u value=a><input type=password name=p></form>");
        assert_eq!(credentials(&d, form(&d)), None);
        let d = doc("<form><input name=q value=a></form>");
        assert_eq!(credentials(&d, form(&d)), None);
    }

    #[test]
    fn fills_empty_fields_but_keeps_typed_values() {
        let mut d = doc("<form><input name=u><input type=password name=p></form>");
        assert_eq!(fill(&mut d, "bob", "pw").len(), 2);
        let f = form(&d);
        assert_eq!(credentials(&d, f), Some(("bob".into(), "pw".into())));
        // すでに入っているときは触らない
        assert!(fill(&mut d, "eve", "x").is_empty());
        assert_eq!(credentials(&d, f), Some(("bob".into(), "pw".into())));
    }

    #[test]
    fn typed_username_is_not_overwritten() {
        let mut d = doc("<form><input name=u value=me><input type=password></form>");
        assert_eq!(fill(&mut d, "bob", "pw").len(), 1);
        assert_eq!(credentials(&d, form(&d)), Some(("me".into(), "pw".into())));
    }
}
