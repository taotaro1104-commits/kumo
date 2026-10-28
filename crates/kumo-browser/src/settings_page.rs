//! `about:settings`：設定ページ。ホーム URL を入力して保存する（フォームは `kumo:sethome` へ送る）。

use kumo_content::{LoadLevel, escape};

pub const SETTINGS_URL: &str = "about:settings";
/// 保存フォームの送信先（ページ内の `kumo:` リンク）。
pub(crate) const SET_HOME_ACTION: &str = "kumo:sethome";
/// 負荷レベルを選ぶリンク（`kumo:setlevel?n=1〜5`）。
pub(crate) const SET_LEVEL_ACTION: &str = "kumo:setlevel";
/// 画像の読み込みを切り替えるリンク（`kumo:setimages?on=1` / `on=0`）。
pub(crate) const SET_IMAGES_ACTION: &str = "kumo:setimages";
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

/// `home` は設定済みのホーム URL（未設定なら空）。
pub fn settings_html(home: &str, level: LoadLevel, images: bool) -> String {
    let image_buttons: String = [(true, "読み込む"), (false, "読み込まない")]
        .iter()
        .map(|(on, label)| {
            let class = if *on == images { "lvb on" } else { "lvb" };
            format!(
                "<a class='{class}' href='{SET_IMAGES_ACTION}?on={}'>{label}</a>",
                u8::from(*on)
            )
        })
        .collect();
    let buttons: String = LoadLevel::ALL
        .iter()
        .map(|l| {
            let class = if *l == level { "lvb on" } else { "lvb" };
            format!(
                "<a class='{class}' href='{SET_LEVEL_ACTION}?n={n}'><span class=lvn>{n}</span>{label}</a>",
                n = l.number(),
                label = l.label()
            )
        })
        .collect();
    format!(
        "<html><head><title>設定</title><style>{CSS}</style></head><body><div class=wrap><h1>設定</h1>\
         <div class=box><form action='{SET_HOME_ACTION}' method=get>\
         <label>ホームページ</label>\
         <input type=text name={HOME_FIELD} value='{}' placeholder='https://example.com/'>\
         <div class=row><button type=submit>保存</button>\
         <span class=hint>空にして保存すると解除します。起動時と、ホームボタン・Alt+Home で開きます。</span></div>\
         </form></div>\n         <div class=box><label>読み込みの負荷（低 → 高）</label><div class=lv>{buttons}</div>\n         <div class=row><span class=hint>{}。次に読み込むページから効きます。</span></div></div>\n         <div class=box><label>画像</label><div class=lv>{image_buttons}</div>\
         <div class=row><span class=hint>「読み込まない」にすると画像を取得せず、通信とメモリを節約します。負荷レベルが 2 以下のときは、この設定にかかわらず読み込みません。</span></div></div>\
         </div></body></html>",
        escape(home),
        level.description()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_prefills_and_escapes_the_current_value() {
        let html = settings_html("https://a.example/?q='<x>", LoadLevel::Standard, true);
        assert!(html.contains("action='kumo:sethome'"));
        assert!(html.contains("&lt;x&gt;") && !html.contains("<x>"));
        assert!(settings_html("", LoadLevel::Standard, true).contains("value=''"));
        let html = settings_html("", LoadLevel::Light, false);
        assert_eq!(html.matches("class='lvb'").count(), 4 + 1);
        assert!(html.contains("class='lvb on' href='kumo:setimages?on=0'"));
        assert!(html.contains("class='lvb on' href='kumo:setlevel?n=2'"));
    }
}
