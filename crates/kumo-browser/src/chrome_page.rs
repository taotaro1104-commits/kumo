//! UI のページ（タブ列・ナビゲーション・URL 欄）を、いまの状態から作る。

use kumo_content::Page;
use url::Url;

use crate::browser::Browser;
use crate::chrome::{CHROME_HEIGHT, ChromeState, TabLabel, chrome_html};
use crate::governor::TabState;
use crate::tab::Tab;

impl Browser {
    /// UI のページ（必要なら作り直す）。
    pub fn chrome_page(&mut self) -> &Page {
        if self.chrome.is_none() {
            let login_prompt = self.login_prompt();
            let tab = self.tabs.get(self.active);
            let state = ChromeState {
                tabs: self
                    .tabs
                    .iter()
                    .map(|t| TabLabel {
                        title: t.title_in(self.lang),
                        loading: t.is_loading(),
                        dormant: t.state != TabState::Live,
                    })
                    .collect(),
                active: self.active,
                url_text: &self.url_text,
                url_focused: self.url_focused,
                can_go_back: tab.is_some_and(|t| t.session.can_go_back()),
                can_go_forward: tab.is_some_and(|t| t.session.can_go_forward()),
                loading: tab.is_some_and(Tab::is_loading),
                bookmarked: tab
                    .and_then(Tab::url)
                    .zip(self.history.as_ref())
                    .is_some_and(|(u, h)| h.is_bookmarked(u.as_str())),
                content_lost: self.content_lost,
                hover: self.chrome_hover.as_deref(),
                login_prompt: login_prompt.as_deref(),
                lang: self.lang,
            };
            let html = chrome_html(&state);
            let url = Url::parse("about:kumo-chrome").expect("固定の URL");
            let page = Page::from_html(
                &html,
                url,
                &mut self.chrome_services,
                (self.window.0, CHROME_HEIGHT),
            );
            self.chrome = Some(page);
        }
        self.chrome.as_ref().expect("作成済み")
    }
}
