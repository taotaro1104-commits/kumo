//! パスワードマネージャの操作：保存の確認・自動入力・一覧ページのリンク。
//!
//! - ログインのフォームを送信すると Content が [`FromContent::LoginSubmitted`] を送る。次のページの読み込みが
//!   終わったところで「保存しますか？」を UI に出す（保存・今回はしない・このサイトでは保存しない）。
//! - ページを読み込み終えたとき、同じ origin（スキーム・ホスト・ポートが同じ）の保存があれば入力欄へ入れる。
//! - 確認と一覧の操作は UI とこのページからだけ受け付け、ページの内容からは操作できない。

use kumo_content::i18n::{fill, ui::Ui};
use kumo_content::protocol::{TabId, ToContent};
use url::Url;

use crate::browser::{Browser, Redraw};
use crate::logins::origin_of;
use crate::passwords_page::{DELETE_PASSWORD, HIDE_PASSWORD, PASSWORDS_URL, SHOW_PASSWORD};
use crate::vault::Vault;

/// 保存を確認中（または次の読み込みを待っている）ログイン情報。
pub(crate) struct PendingLogin {
    tab: TabId,
    origin: String,
    username: String,
    password: String,
    /// UI に出した
    offered: bool,
}

impl Drop for PendingLogin {
    fn drop(&mut self) {
        self.password.clear(); // 使い終わったら消す
    }
}

impl Browser {
    /// パスワードを保存・自動入力する設定。
    pub fn passwords_enabled(&self) -> bool {
        self.passwords
    }

    pub fn set_passwords_enabled(&mut self, on: bool, save: bool) {
        self.passwords = on;
        if !on {
            self.pending_login = None;
        }
        if save
            && let Some(history) = &self.history
            && let Err(e) = history.set_passwords_enabled(on)
        {
            eprintln!("[settings] {e}");
        }
    }

    /// 鍵を差し込む（テスト用。通常は OS の資格情報ストアから得る）。
    pub fn set_vault(&mut self, vault: Vault) {
        self.vault = Some(vault);
    }

    /// 鍵を用意する。`create` なら、なければ作る。使える鍵があれば `true`（ストアが使えなければ `false`）。
    fn ensure_vault(&mut self, create: bool) -> bool {
        if self.vault.is_none() {
            self.vault = Vault::system(create);
        }
        self.vault.is_some()
    }

    /// 保存の確認に出す文（出すものがなければ `None`）。
    pub fn login_prompt(&self) -> Option<String> {
        let pending = self.pending_login.as_ref().filter(|p| p.offered)?;
        (self.active_tab()?.id == pending.tab).then(|| {
            let user = if pending.username.is_empty() {
                Ui::ThisAccount.t(self.lang)
            } else {
                pending.username.as_str()
            };
            fill(Ui::SaveLoginAsk.t(self.lang), &[user, &pending.origin])
        })
    }

    /// Content がログインのフォームの送信を知らせた。
    pub(crate) fn login_submitted(
        &mut self,
        tab: TabId,
        url: &str,
        username: String,
        password: String,
    ) {
        if !self.passwords {
            return;
        }
        let Some(origin) = Url::parse(url).ok().as_ref().and_then(origin_of) else {
            return;
        };
        if self
            .history
            .as_ref()
            .is_none_or(|h| h.is_never_save(&origin))
        {
            return;
        }
        // すでに同じ内容を保存していれば確認しない
        let has = self.history.as_ref().is_some_and(|h| h.has_logins(&origin));
        if has && self.ensure_vault(false) {
            let saved = self
                .history
                .as_ref()
                .zip(self.vault.as_ref())
                .and_then(|(h, v)| h.login_password(v, &origin, &username));
            if saved.as_deref() == Some(password.as_str()) {
                return;
            }
        }
        self.pending_login = Some(PendingLogin {
            tab,
            origin,
            username,
            password,
            offered: false,
        });
    }

    /// ページを読み込み終えた：確認を出す、または保存した値を入れる。確認を出したら `true`。
    pub(crate) fn login_page_loaded(&mut self, tab: TabId, url: &str) -> bool {
        if let Some(pending) = self.pending_login.as_mut().filter(|p| p.tab == tab) {
            if !pending.offered {
                pending.offered = true;
                return true;
            }
            self.pending_login = None; // 確認を出したまま次のページへ移った
        }
        self.autofill(tab, url);
        false
    }

    fn autofill(&mut self, tab: TabId, url: &str) {
        if !self.passwords {
            return;
        }
        let Some(origin) = Url::parse(url).ok().as_ref().and_then(origin_of) else {
            return;
        };
        let Some(history) = &self.history else {
            return;
        };
        let mut users = history.logins_of(&origin);
        if users.is_empty() || !self.ensure_vault(false) {
            return;
        }
        users.sort();
        let (Some(history), Some(vault)) = (&self.history, &self.vault) else {
            return;
        };
        let Some(username) = users.into_iter().next() else {
            return;
        };
        let Some(password) = history.login_password(vault, &origin, &username) else {
            return;
        };
        self.content.send(ToContent::Autofill {
            tab,
            url: url.to_owned(),
            username,
            password,
        });
    }

    /// 確認の「保存」。
    pub fn save_pending_login(&mut self) -> Redraw {
        let Some(pending) = self.pending_login.take() else {
            return Redraw::default();
        };
        let (origin, username, password) = (
            pending.origin.clone(),
            pending.username.clone(),
            pending.password.clone(),
        );
        if !self.ensure_vault(true) {
            eprintln!("[passwords] 鍵を得られないので保存しません");
        } else if let (Some(history), Some(vault)) = (&self.history, &self.vault)
            && let Err(e) = history.save_login(vault, &origin, &username, &password)
        {
            eprintln!("[passwords] {e}");
        }
        self.invalidate_chrome()
    }

    /// 確認の「今回は保存しない」。
    pub fn dismiss_login_prompt(&mut self) -> Redraw {
        self.pending_login = None;
        self.invalidate_chrome()
    }

    /// 確認の「このサイトでは保存しない」。
    pub fn never_save_pending_login(&mut self) -> Redraw {
        if let (Some(pending), Some(history)) = (&self.pending_login, &self.history)
            && let Err(e) = history.set_never_save(&pending.origin, true)
        {
            eprintln!("[passwords] {e}");
        }
        self.dismiss_login_prompt()
    }

    /// 一覧ページを開く（空の新しいタブならそこで、それ以外は新しいタブで）。
    pub fn show_passwords(&mut self) -> Redraw {
        let url = Url::parse(PASSWORDS_URL).expect("固定の URL");
        let on_new_tab = self
            .active_tab()
            .is_some_and(|t| t.url().is_none_or(|u| u.as_str() == crate::NEW_TAB_URL));
        if on_new_tab {
            self.navigate(url)
        } else {
            self.open_tab(Some(url), true)
        }
    }

    /// 一覧ページ（`about:passwords`）の HTML。表示を押した1件だけパスワードを入れる。
    pub fn passwords_html(&self) -> String {
        let list = self
            .history
            .as_ref()
            .map(|h| h.logins())
            .unwrap_or_default();
        let revealed = self.revealed.clone().and_then(|(origin, username)| {
            let pw =
                self.history
                    .as_ref()?
                    .login_password(self.vault.as_ref()?, &origin, &username)?;
            Some((origin, username, pw))
        });
        crate::passwords_page::passwords_html(
            &list,
            revealed
                .as_ref()
                .map(|(o, u, p)| (o.as_str(), u.as_str(), p.as_str())),
            self.lang,
        )
    }

    /// 一覧ページのリンク（表示・隠す・削除）。一覧を表示しているときだけ受け付け、扱ったら `true`。
    pub(crate) fn password_link(&mut self, link: &Url) -> bool {
        let s = link.as_str();
        let show = s.starts_with(SHOW_PASSWORD);
        let hide = s.starts_with(HIDE_PASSWORD);
        let delete = s.starts_with(DELETE_PASSWORD);
        if !(show || hide || delete) {
            return false;
        }
        let on_list = self
            .active_tab()
            .and_then(|t| t.url())
            .is_some_and(|u| u.as_str() == PASSWORDS_URL);
        if !on_list {
            return true; // 他のページからは操作させない
        }
        let pair = |name: &str| {
            link.query_pairs()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.into_owned())
        };
        let target = pair("o").zip(pair("u"));
        if hide {
            self.revealed = None;
        } else if let Some((origin, username)) = target {
            if show {
                self.ensure_vault(false);
                self.revealed = Some((origin, username));
            } else {
                if self.revealed.as_ref() == Some(&(origin.clone(), username.clone())) {
                    self.revealed = None;
                }
                if let Some(history) = &self.history
                    && let Err(e) = history.remove_login(&origin, &username)
                {
                    eprintln!("[passwords] {e}");
                }
            }
        }
        self.reload();
        true
    }
}
