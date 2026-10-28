//! `about:settings`：設定ページ。ホーム URL を入力して保存する（フォームは `kumo:sethome` へ送る）。

use kumo_content::i18n::pages::Pg;
use kumo_content::{Lang, LoadLevel, escape};

pub const SETTINGS_URL: &str = "about:settings";
/// 保存フォームの送信先（ページ内の `kumo:` リンク）。
pub(crate) const SET_HOME_ACTION: &str = "kumo:sethome";
/// 負荷レベルを選ぶリンク（`kumo:setlevel?n=1〜5`）。
pub(crate) const SET_LEVEL_ACTION: &str = "kumo:setlevel";
/// 画像の読み込みを切り替えるリンク（`kumo:setimages?on=1` / `on=0`）。
pub(crate) const SET_IMAGES_ACTION: &str = "kumo:setimages";
/// パスワードの保存・自動入力を切り替えるリンク（`kumo:setpasswords?on=1` / `on=0`）。
/// 表示言語を選ぶリンク（`kumo:setlang?c=ja|en|zh|ko|es`）。
pub(crate) const SET_LANGUAGE_ACTION: &str = "kumo:setlang";
pub(crate) const SET_PASSWORDS_ACTION: &str = "kumo:setpasswords";
/// フォームの入力欄の名前。
pub(crate) const HOME_FIELD: &str = "home";

const CSS: &str = "
html, body { margin: 0; background: #f6f7f9; color: #1f2328; font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; }
.wrap { max-width: 860px; margin: 0 auto; padding: 36px 24px; }
h1 { margin: 0 0 20px; font-size: 26px; color: #1a73e8; }
.box { background: #ffffff; border: 1px solid #e1e4e8; border-radius: 12px; padding: 16px; }
label { display: block; margin-bottom: 8px; font-size: 14px; font-weight: 600; }
input[type=text] { box-sizing: border-box; width: 100%; height: 34px; padding: 0 10px; font-size: 14px;
                   border: 1px solid #c3c7cc; border-radius: 8px; }
.row { display: flex; align-items: center; gap: 12px; margin-top: 12px; }
button { height: 32px; padding: 0 18px; border: 0; border-radius: 16px; background: #1a73e8; color: #ffffff; font-size: 13px; }
.lv { display: flex; gap: 8px; margin-top: 4px; }
a.lvb { display: block; flex: 1; padding: 10px 8px; text-align: center; border: 1px solid #c3c7cc; border-radius: 10px;
        background: #ffffff; color: #1f2328; font-size: 13px; text-decoration: none; }
a.lvb.on { border: 2px solid #1a73e8; background: #e8f0fe; color: #1a56c4; font-weight: 600; }
.lvn { display: block; font-size: 18px; }
.box + .box { margin-top: 16px; }
.hint { color: #6b7280; font-size: 12px; }
";

/// 選択肢のボタン列。`items` は（選択中か, リンク先, 表示）。
fn choices(items: impl IntoIterator<Item = (bool, String, String)>) -> String {
    items
        .into_iter()
        .map(|(on, href, label)| {
            let class = if on { "lvb on" } else { "lvb" };
            format!("<a class='{class}' href='{href}'>{label}</a>")
        })
        .collect()
}

fn on_off(on: bool, action: &str, labels: [&str; 2]) -> String {
    choices(
        [(true, labels[0]), (false, labels[1])]
            .into_iter()
            .map(|(value, label)| {
                (
                    value == on,
                    format!("{action}?on={}", u8::from(value)),
                    label.to_owned(),
                )
            }),
    )
}

/// `home` は設定済みのホーム URL（未設定なら空）。
pub fn settings_html(
    home: &str,
    level: LoadLevel,
    images: bool,
    passwords: bool,
    lang: Lang,
) -> String {
    let t = |m: Pg| m.t(lang);
    let languages = choices(Lang::ALL.into_iter().map(|l| {
        (
            l == lang,
            format!("{SET_LANGUAGE_ACTION}?c={}", l.code()),
            l.native_name().to_owned(),
        )
    }));
    let image_buttons = on_off(
        images,
        SET_IMAGES_ACTION,
        [t(Pg::ImagesOn), t(Pg::ImagesOff)],
    );
    let password_buttons = on_off(
        passwords,
        SET_PASSWORDS_ACTION,
        [t(Pg::PasswordsOn), t(Pg::PasswordsOff)],
    );
    let levels: String = LoadLevel::ALL
        .iter()
        .map(|l| {
            let class = if *l == level { "lvb on" } else { "lvb" };
            format!(
                "<a class='{class}' href='{SET_LEVEL_ACTION}?n={n}'><span class=lvn>{n}</span>{label}</a>",
                n = l.number(),
                label = l.label(lang)
            )
        })
        .collect();
    let level_hint = format!("{}　{}", level.description(lang), t(Pg::LoadLevelNote));
    format!(
        "<html><head><title>{settings}</title><style>{CSS}</style></head><body><div class=wrap><h1>{settings}</h1>\
         <div class=box><label>{language}</label><div class=lv>{languages}</div>\
         <div class=row><span class=hint>{language_hint}</span></div></div>\n\
         <div class=box><form action='{SET_HOME_ACTION}' method=get>\
         <label>{home_page}</label>\
         <input type=text name={HOME_FIELD} value='{home}' placeholder='https://example.com/'>\
         <div class=row><button type=submit>{save}</button>\
         <span class=hint>{home_hint}</span></div>\
         </form></div>\n\
         <div class=box><label>{level_title}</label><div class=lv>{levels}</div>\
         <div class=row><span class=hint>{level_hint}</span></div></div>\n\
         <div class=box><label>{images_title}</label><div class=lv>{image_buttons}</div>\
         <div class=row><span class=hint>{images_hint}</span></div></div>\
         <div class=box><label>{passwords_title}</label><div class=lv>{password_buttons}</div>\
         <div class=row><span class=hint>{passwords_hint}<a href='about:passwords'>{passwords_view}</a></span></div></div>\
         </div></body></html>",
        settings = t(Pg::Settings),
        language = t(Pg::LanguageTitle),
        language_hint = t(Pg::LanguageHint),
        home_page = t(Pg::HomePage),
        home = escape(home),
        save = t(Pg::Save),
        home_hint = t(Pg::HomeHint),
        level_title = t(Pg::LoadLevelTitle),
        level_hint = level_hint,
        images_title = t(Pg::ImagesTitle),
        images_hint = t(Pg::ImagesHint),
        passwords_title = t(Pg::PasswordsTitle),
        passwords_hint = t(Pg::PasswordsHint),
        passwords_view = t(Pg::PasswordsView),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_prefills_and_escapes_the_current_value() {
        let html = settings_html(
            "https://a.example/?q='<x>",
            LoadLevel::Standard,
            true,
            true,
            Lang::Ja,
        );
        assert!(html.contains("action='kumo:sethome'"));
        assert!(html.contains("&lt;x&gt;") && !html.contains("<x>"));
        assert!(settings_html("", LoadLevel::Standard, true, true, Lang::Ja).contains("value=''"));
        let html = settings_html("", LoadLevel::Light, false, false, Lang::Ja);
        assert_eq!(html.matches("class='lvb'").count(), 4 + 1 + 1 + 4);
        assert!(html.contains("class='lvb on' href='kumo:setimages?on=0'"));
        assert!(html.contains("class='lvb on' href='kumo:setlevel?n=2'"));
        assert!(html.contains("class='lvb on' href='kumo:setpasswords?on=0'"));
        assert!(html.contains("class='lvb on' href='kumo:setlang?c=ja'"));
    }

    #[test]
    fn the_page_follows_the_chosen_language() {
        let en = settings_html("", LoadLevel::Standard, true, true, Lang::En);
        assert!(en.contains("<h1>Settings</h1>") && en.contains("Load level"));
        assert!(en.contains("class='lvb on' href='kumo:setlang?c=en'"));
        let ko = settings_html("", LoadLevel::Standard, true, true, Lang::Ko);
        assert!(ko.contains("<h1>설정</h1>") && ko.contains("한국어") && ko.contains("日本語"));
        assert!(!en.contains("ホームページ"), "ほかの言語の文言が混ざらない");
    }
}
