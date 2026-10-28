//! `about:passwords`：保存したログイン情報の一覧。パスワードは伏せておき、「表示」を押した1件だけ見せる。
//! 操作は `kumo:` のリンクで表す（このページを表示しているときだけ受け付ける）。

use kumo_content::i18n::pages::Pg;
use kumo_content::{Lang, escape};
use url::form_urlencoded;

use crate::logins::LoginSummary;

pub const PASSWORDS_URL: &str = "about:passwords";
pub(crate) const SHOW_PASSWORD: &str = "kumo:showpw";
pub(crate) const HIDE_PASSWORD: &str = "kumo:hidepw";
pub(crate) const DELETE_PASSWORD: &str = "kumo:delpw";

const CSS: &str = "
html, body { margin: 0; background: #f6f7f9; color: #1f2328; font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; }
.wrap { max-width: 860px; margin: 0 auto; padding: 36px 24px; }
h1 { margin: 0 0 8px; font-size: 26px; color: #1a73e8; }
.note { margin: 0 0 20px; color: #6b7280; font-size: 12px; }
.box { background: #ffffff; border: 1px solid #e1e4e8; border-radius: 12px; padding: 4px 16px; }
.row { display: flex; align-items: center; padding: 10px 0; border-bottom: 1px solid #eef0f2; }
.row:last-child { border-bottom: 0; }
.main { flex: 1; overflow: hidden; }
.site { color: #1f2328; font-size: 14px; font-weight: 600; white-space: nowrap; overflow: hidden; }
.u { color: #6b7280; font-size: 12px; white-space: nowrap; overflow: hidden; }
.pw { color: #1f2328; font-size: 13px; font-family: Consolas, monospace; }
a.act { flex: none; padding: 4px 10px; border-radius: 12px; color: #1a56c4; font-size: 12px; text-decoration: none; }
a.rm { flex: none; padding: 4px 10px; border-radius: 12px; color: #b3261e; font-size: 12px; text-decoration: none; }
.empty { padding: 28px; text-align: center; color: #6b7280; font-size: 14px; background: #ffffff;
         border: 1px dashed #d5d9de; border-radius: 12px; }
";

/// 組を指すリンクの問い合わせ文字列（`o=origin&u=ユーザー名`）。
pub(crate) fn query(origin: &str, username: &str) -> String {
    form_urlencoded::Serializer::new(String::new())
        .append_pair("o", origin)
        .append_pair("u", username)
        .finish()
}

/// 一覧ページ。`revealed` は表示を押した組（origin, ユーザー名, パスワード）。
pub fn passwords_html(
    list: &[LoginSummary],
    revealed: Option<(&str, &str, &str)>,
    lang: Lang,
) -> String {
    let title = Pg::Passwords.t(lang);
    let mut html = format!(
        "<html><head><title>{title}</title><style>{CSS}</style></head><body><div class=wrap><h1>{title}</h1>\
         <div class=note>{}</div>",
        Pg::PasswordsNote.t(lang)
    );
    if list.is_empty() {
        html.push_str(&format!(
            "<div class=empty>{}</div>",
            Pg::PasswordsEmpty.t(lang)
        ));
    } else {
        html.push_str("<div class=box>");
        for l in list {
            let q = query(&l.origin, &l.username);
            let shown = revealed.filter(|(o, u, _)| *o == l.origin && *u == l.username);
            let (pw, toggle) = match shown {
                Some((_, _, pw)) => (
                    format!("<div class=pw>{}</div>", escape(pw)),
                    format!(
                        "<a class=act href='{HIDE_PASSWORD}'>{}</a>",
                        Pg::Hide.t(lang)
                    ),
                ),
                None => (
                    "<div class=pw>••••••••</div>".to_owned(),
                    format!(
                        "<a class=act href='{SHOW_PASSWORD}?{q}'>{}</a>",
                        Pg::Show.t(lang)
                    ),
                ),
            };
            let user = if l.username.is_empty() {
                Pg::NoUsername.t(lang)
            } else {
                &l.username
            };
            html.push_str(&format!(
                "<div class=row><div class=main><div class=site>{}</div><div class=u>{}</div>{pw}</div>{toggle}\
                 <a class=rm href='{DELETE_PASSWORD}?{q}'>{}</a></div>",
                escape(&l.origin),
                escape(user),
                Pg::Delete.t(lang),
            ));
        }
        html.push_str("</div>");
    }
    html.push_str("</div></body></html>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(origin: &str, username: &str) -> LoginSummary {
        LoginSummary {
            origin: origin.into(),
            username: username.into(),
        }
    }

    #[test]
    fn passwords_stay_hidden_until_one_is_shown() {
        let list = [
            entry("https://a.example", "al<ice>"),
            entry("https://b.example", "bob"),
        ];
        let hidden = passwords_html(&list, None, Lang::Ja);
        assert!(!hidden.contains("hunter2") && hidden.contains("••••"));
        assert!(hidden.contains("al&lt;ice&gt;") && !hidden.contains("<ice>"));
        assert!(hidden.contains("kumo:showpw?o=https%3A%2F%2Fb.example&u=bob"));
        let shown = passwords_html(
            &list,
            Some(("https://b.example", "bob", "hunter2<")),
            Lang::Ja,
        );
        assert_eq!(shown.matches("hunter2&lt;").count(), 1, "選んだ1件だけ");
        assert!(shown.contains("kumo:hidepw"));
    }

    #[test]
    fn empty_list_explains_itself() {
        assert!(passwords_html(&[], None, Lang::Ja).contains("保存したパスワードはありません"));
        assert!(passwords_html(&[], None, Lang::En).contains("No saved passwords"));
    }
}
