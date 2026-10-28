//! ホーム URL：ホームボタン・起動時・設定ページからの保存。

use kumo_content::protocol::ToContent;
use kumo_content::{Lang, LoadLevel};
use url::Url;

use crate::browser::{Browser, Redraw};
use crate::chrome::parse_input;
use crate::history::HistoryDb;
use crate::settings_page::{
    HOME_FIELD, SET_HOME_ACTION, SET_IMAGES_ACTION, SET_LANGUAGE_ACTION, SET_LEVEL_ACTION,
    SET_PASSWORDS_ACTION, SETTINGS_URL,
};

/// 保存済みの負荷レベル（なければ既定）。
pub(crate) fn saved_load_level(history: Option<&HistoryDb>) -> LoadLevel {
    history
        .and_then(HistoryDb::load_level_number)
        .and_then(LoadLevel::from_number)
        .unwrap_or_default()
}

impl Browser {
    /// 設定済みのホーム URL（保存された文字列が壊れていれば `None`）。
    pub fn home_url(&self) -> Option<Url> {
        let text = self.history.as_ref()?.home_url()?;
        Url::parse(&text).ok()
    }

    /// 表示中のタブをホームへ移す（未設定なら新しいタブのページ）。
    pub fn go_home(&mut self) -> Redraw {
        let url = self
            .home_url()
            .unwrap_or_else(|| Url::parse(crate::NEW_TAB_URL).expect("固定の URL"));
        self.navigate(url)
    }

    /// 起動時のタブ：ホームがあればそこ、なければ新しいタブのページ。
    pub fn open_start_tab(&mut self) -> Redraw {
        self.open_tab(self.home_url(), true)
    }

    /// 設定ページを開く。
    pub fn show_settings(&mut self) -> Redraw {
        self.open_tab(Url::parse(SETTINGS_URL).ok(), true)
    }

    /// 読み込みの負荷レベル。
    pub fn load_level(&self) -> LoadLevel {
        self.load_level
    }

    pub(crate) fn send_load_level(&self) {
        self.content.send(ToContent::LoadLevel(self.load_level));
    }

    /// 表示言語。
    pub fn language(&self) -> Lang {
        self.lang
    }

    pub(crate) fn send_language(&self) {
        self.content.send(ToContent::Language(self.lang));
    }

    /// 表示言語を変える（開いている内部ページは作り直すまで前の言語のまま）。`save` なら次の起動でも使う。
    pub fn set_language(&mut self, lang: Lang, save: bool) {
        self.lang = lang;
        self.send_language();
        self.invalidate_chrome();
        if save
            && let Some(history) = &self.history
            && let Err(e) = history.set_language(lang)
        {
            eprintln!("[settings] {e}");
        }
    }

    /// 画像を読み込む設定。
    pub fn images_enabled(&self) -> bool {
        self.images
    }

    pub(crate) fn send_images(&self) {
        self.content.send(ToContent::Images(self.images));
    }

    /// 画像の読み込みを切り替える（以後に始める読み込みから効く）。`save` なら次の起動でも使う。
    pub fn set_images(&mut self, on: bool, save: bool) {
        self.images = on;
        self.send_images();
        if save
            && let Some(history) = &self.history
            && let Err(e) = history.set_images_enabled(on)
        {
            eprintln!("[settings] {e}");
        }
    }

    /// 負荷レベルを変える（以後に始める読み込みから効く）。`save` なら次の起動でも使う。
    pub fn set_load_level(&mut self, level: LoadLevel, save: bool) {
        self.load_level = level;
        self.send_load_level();
        if save
            && let Some(history) = &self.history
            && let Err(e) = history.set_load_level_number(level.number())
        {
            eprintln!("[settings] {e}");
        }
    }

    /// 設定ページのフォーム送信（`kumo:sethome?home=…`）。設定ページを表示しているときだけ受け付け、扱ったら `true`。
    /// 入力は http(s)・file・about の URL か、空（解除）だけを保存する。
    pub(crate) fn save_home_link(&mut self, link: &Url) -> bool {
        let is_level = link.as_str().starts_with(SET_LEVEL_ACTION);
        let is_images = link.as_str().starts_with(SET_IMAGES_ACTION);
        let is_passwords = link.as_str().starts_with(SET_PASSWORDS_ACTION);
        let is_language = link.as_str().starts_with(SET_LANGUAGE_ACTION);
        if !is_level
            && !is_images
            && !is_passwords
            && !is_language
            && !link.as_str().starts_with(SET_HOME_ACTION)
        {
            return false;
        }
        let on_settings = self
            .active_tab()
            .and_then(|t| t.url())
            .is_some_and(|u| u.as_str() == SETTINGS_URL);
        if !on_settings {
            return true; // 他のページからは操作させない
        }
        if is_language {
            let lang = link
                .query_pairs()
                .find(|(k, _)| k == "c")
                .and_then(|(_, v)| Lang::from_code(&v));
            if let Some(lang) = lang {
                self.set_language(lang, true);
                self.reload();
            }
            return true;
        }
        if is_images || is_passwords {
            let on = link
                .query_pairs()
                .find(|(k, _)| k == "on")
                .map(|(_, v)| v == "1");
            if let Some(on) = on {
                if is_passwords {
                    self.set_passwords_enabled(on, true);
                } else {
                    self.set_images(on, true);
                }
                self.reload();
            }
            return true;
        }
        if is_level {
            let level = link
                .query_pairs()
                .find(|(k, _)| k == "n")
                .and_then(|(_, v)| v.parse().ok())
                .and_then(LoadLevel::from_number);
            if let Some(level) = level {
                self.set_load_level(level, true);
                self.reload();
            }
            return true;
        }
        let input = link
            .query_pairs()
            .find(|(k, _)| k == HOME_FIELD)
            .map(|(_, v)| v.trim().to_owned())
            .unwrap_or_default();
        let value = if input.is_empty() {
            None
        } else if let Some(url) = parse_input(&input).filter(|u| u.scheme() != "about") {
            Some(url.to_string())
        } else {
            return true; // 解釈できない入力は保存しない
        };
        if let Some(history) = &self.history
            && let Err(e) = history.set_home_url(value.as_deref())
        {
            eprintln!("[settings] {e}");
        }
        self.reload();
        true
    }
}
