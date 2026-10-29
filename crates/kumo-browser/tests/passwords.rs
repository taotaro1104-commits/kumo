//! パスワードマネージャ：ログイン送信 → 保存の確認 → 次回の自動入力 → 一覧ページ。
//! origin は http(s) だけが対象なので、手元の HTTP サーバーを立てて試す。
mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use common::{browser, settle, title};
use kumo_browser::{Browser, HistoryDb, Vault};
use url::Url;

const FORM_STYLE: &str = "<style>body{margin:0} input{display:block;position:absolute;left:0;\
                          width:200px;height:30px;margin:0} #u{top:0} #p{top:40px} #s{top:80px}</style>";

fn login_page(user: &str, pass: &str) -> String {
    format!(
        "<html><head><title>Login</title>{FORM_STYLE}</head><body>\
         <form action=/welcome method=post><input id=u name=user value='{user}'>\
         <input id=p type=password name=pass value='{pass}'><input id=s type=submit value=Go></form><script>0</script></body></html>"
    )
}

/// `/login`（入力済み）・`/empty`（空）・`POST /welcome` を返す小さなサーバー。
fn serve() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            std::thread::spawn(move || {
                // ヘッダーの終わりと、本文（POST）まで読み切ってから返す（残すと接続が切れて失敗する）
                let mut data = Vec::new();
                let mut buf = [0u8; 4096];
                let head_end = loop {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break data.len();
                    }
                    data.extend_from_slice(&buf[..n]);
                    if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let head = String::from_utf8_lossy(&data[..head_end.min(data.len())]).to_string();
                let length: usize = head
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap_or(0))
                    })
                    .unwrap_or(0);
                while data.len() < head_end + length {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    data.extend_from_slice(&buf[..n]);
                }
                let first = head.lines().next().unwrap_or("").to_owned();
                let body = if first.contains("/welcome") {
                    "<html><head><title>Welcome</title></head><body>hi</body></html>".to_owned()
                } else if first.contains("/empty") {
                    login_page("", "")
                } else {
                    login_page("alice", "s3cret")
                };
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
            });
        }
    });
    port
}

struct Rig {
    b: Browser,
    path: std::path::PathBuf,
    port: u16,
}

fn rig(name: &str) -> Rig {
    let path = std::env::temp_dir().join(format!("kumo-pw-{name}-{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut b = browser(Some(HistoryDb::open(&path).unwrap()));
    b.set_vault(Vault::from_key([3; 32]));
    Rig {
        b,
        path,
        port: serve(),
    }
}

impl Rig {
    fn url(&self, path: &str) -> Url {
        Url::parse(&format!("http://127.0.0.1:{}{path}", self.port)).unwrap()
    }

    fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// 入力済みのログインフォームを開いて送信する。
    fn log_in(&mut self) {
        self.b.open_tab(Some(self.url("/login")), true);
        settle(&mut self.b);
        self.b.click_content(10.0, 95.0, false); // 送信ボタン
        settle(&mut self.b);
        assert_eq!(title(&self.b), "Welcome", "フォームが送信された");
    }

    fn field(&mut self, id: &str) -> String {
        self.b
            .eval(
                &format!("document.getElementById('{id}').value"),
                Duration::from_secs(10),
            )
            .unwrap()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn login_is_offered_saved_and_filled_in_next_time() {
    let mut r = rig("flow");
    r.log_in();
    let prompt = r.b.login_prompt().expect("保存の確認が出る");
    assert!(
        prompt.contains("alice") && prompt.contains(&r.origin()),
        "{prompt}"
    );
    assert!(
        r.b.chrome_page()
            .list
            .links
            .iter()
            .any(|l| &*l.href == "kumo:pwsave")
    );
    r.b.chrome_action(kumo_browser::ChromeAction::SaveLogin);
    assert!(r.b.login_prompt().is_none());

    r.b.open_tab(Some(r.url("/empty")), true);
    settle(&mut r.b);
    assert_eq!(r.field("u"), "alice");
    assert_eq!(r.field("p"), "s3cret", "保存した値が入る");
}

#[test]
fn nothing_is_saved_until_the_user_agrees() {
    let mut r = rig("decline");
    r.log_in();
    assert!(r.b.login_prompt().is_some());
    r.b.chrome_action(kumo_browser::ChromeAction::DismissLogin);
    assert!(r.b.login_prompt().is_none());
    r.b.open_tab(Some(r.url("/empty")), true);
    settle(&mut r.b);
    assert_eq!(r.field("p"), "", "保存していないので入らない");
}

#[test]
fn never_save_silences_the_site() {
    let mut r = rig("never");
    r.log_in();
    r.b.chrome_action(kumo_browser::ChromeAction::NeverSaveLogin);
    r.log_in();
    assert!(r.b.login_prompt().is_none(), "このサイトでは確認しない");
}

#[test]
fn disabled_setting_stops_offering_and_filling() {
    let mut r = rig("off");
    r.b.set_passwords_enabled(false, false);
    r.log_in();
    assert!(r.b.login_prompt().is_none());

    r.b.set_passwords_enabled(true, false);
    r.log_in();
    r.b.chrome_action(kumo_browser::ChromeAction::SaveLogin);
    r.b.set_passwords_enabled(false, false);
    r.b.open_tab(Some(r.url("/empty")), true);
    settle(&mut r.b);
    assert_eq!(r.field("p"), "", "設定を切れば自動入力しない");
}

#[test]
fn passwords_page_hides_reveals_and_deletes_only_from_itself() {
    let mut r = rig("page");
    r.log_in();
    r.b.chrome_action(kumo_browser::ChromeAction::SaveLogin);
    let query = kumo_browser_query(&r.origin(), "alice");

    // ほかのページからのリンクでは消せない
    r.b.follow_link(&format!("kumo:delpw?{query}"), false);
    settle(&mut r.b);
    r.b.show_passwords();
    settle(&mut r.b);
    let text = |r: &mut Rig| r.b.passwords_html();
    let hidden = text(&mut r);
    assert!(
        hidden.contains("alice") && !hidden.contains("s3cret"),
        "{hidden}"
    );

    r.b.follow_link(&format!("kumo:showpw?{query}"), false);
    settle(&mut r.b);
    assert!(text(&mut r).contains("s3cret"), "表示を押した1件だけ見える");
    r.b.follow_link("kumo:hidepw", false);
    settle(&mut r.b);
    assert!(!text(&mut r).contains("s3cret"));

    r.b.follow_link(&format!("kumo:delpw?{query}"), false);
    settle(&mut r.b);
    assert!(text(&mut r).contains("保存したパスワードはありません"));
}

fn kumo_browser_query(origin: &str, user: &str) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("o", origin)
        .append_pair("u", user)
        .finish()
}

#[test]
fn password_setting_persists_and_is_changed_only_on_the_settings_page() {
    let mut r = rig("setting");
    assert!(r.b.passwords_enabled(), "既定は使う");
    r.b.open_tab(Some(r.url("/login")), true);
    settle(&mut r.b);
    r.b.follow_link("kumo:setpasswords?on=0", false);
    assert!(r.b.passwords_enabled(), "設定ページ以外からは変えられない");
    r.b.show_settings();
    settle(&mut r.b);
    r.b.follow_link("kumo:setpasswords?on=0", false);
    settle(&mut r.b);
    assert!(!r.b.passwords_enabled());
    // 開き直す（同じファイルを同時に開けないので、先に閉じる）
    drop(std::mem::replace(&mut r.b, browser(None)));
    r.b = browser(Some(HistoryDb::open(&r.path).unwrap()));
    assert!(!r.b.passwords_enabled(), "再起動後も残る");
}
