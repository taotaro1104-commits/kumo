//! 利用者の操作：UI のリンク、ページのクリック、入力欄・URL 欄への入力、スクロール。

use kumo_content::protocol::{FieldKey, ToContent};
use url::Url;

use crate::browser::{Browser, Redraw};
use crate::chrome::{ChromeAction, parse_input};
use crate::tab::Tab;

impl Browser {
    /// UI 上のリンクのクリック。
    pub fn chrome_action(&mut self, action: ChromeAction) -> Redraw {
        match action {
            ChromeAction::ActivateTab(i) => self.activate(i),
            ChromeAction::CloseTab(i) => self.close_tab(i),
            ChromeAction::NewTab => self.open_tab(None, true),
            ChromeAction::Back => self.back(),
            ChromeAction::Forward => self.forward(),
            ChromeAction::Reload => self.reload(),
            ChromeAction::FocusUrlBar => self.focus_url_bar(),
        }
    }

    /// コンテンツ領域でのクリック（`y` は領域上端から）。部品やリンクの判定は Content が行う。
    pub fn click_content(&mut self, x: f32, y: f32, new_tab: bool) -> Redraw {
        let Some(tab) = self.active_tab().map(|t| t.id) else {
            return Redraw::default();
        };
        self.content.send(ToContent::Click { tab, x, y, new_tab });
        if self.url_focused {
            return self.cancel_url_edit();
        }
        Redraw::default()
    }

    /// 表示中の文書を基準に `href` へ移動する。`new_tab` なら背景の新しいタブで開く。
    pub fn follow_link(&mut self, href: &str, new_tab: bool) -> Redraw {
        let Some(url) = self
            .active_tab()
            .and_then(|t| t.shown_url.as_ref())
            .and_then(|base| base.join(href.trim()).ok())
        else {
            return Redraw::default();
        };
        if url.scheme() == "javascript" {
            return Redraw::default(); // JS は Phase 2
        }
        if new_tab {
            return self.open_tab(Some(url), false);
        }
        self.navigate(url)
    }

    /// ページ内の `kumo:` リンク（エラーページのボタン）。再読み込みと戻るだけを受け付け、
    /// それ以外の UI 操作（タブを閉じるなど）はページから行わせない。扱ったら `true`。
    pub(crate) fn page_action(&mut self, index: usize, url: &Url) -> bool {
        if url.scheme() != "kumo" {
            return false;
        }
        if index == self.active {
            match ChromeAction::from_href(url.as_str()) {
                Some(ChromeAction::Reload) => {
                    self.reload();
                }
                Some(ChromeAction::Back) => {
                    self.back();
                }
                _ => {}
            }
        }
        true
    }

    /// スクリプトの実行を切り替える（以後に読み込むページから効く）。
    pub fn set_javascript(&mut self, on: bool) {
        self.content.send(ToContent::Settings { javascript: on });
    }

    // ---- ページ内の入力欄 ----

    pub fn field_focused(&self) -> bool {
        self.active_tab().is_some_and(|t| t.field_focused)
    }

    fn field_key(&mut self, key: FieldKey) -> Redraw {
        if let Some(tab) = self.active_tab().filter(|t| t.field_focused) {
            self.content.send(ToContent::Key { tab: tab.id, key });
        }
        Redraw::default()
    }

    pub fn type_into_field(&mut self, text: &str) -> Redraw {
        self.field_key(FieldKey::Text(text.to_owned()))
    }

    pub fn backspace_field(&mut self) -> Redraw {
        self.field_key(FieldKey::Backspace)
    }

    /// Enter：textarea なら改行、それ以外はフォームを送信する。
    pub fn submit_field(&mut self) -> Redraw {
        self.field_key(FieldKey::Enter)
    }

    pub fn blur_field(&mut self) -> Redraw {
        self.field_key(FieldKey::Blur)
    }

    // ---- URL 欄 ----

    pub fn focus_url_bar(&mut self) -> Redraw {
        self.blur_field();
        self.url_focused = true;
        self.url_text.clear();
        self.invalidate_chrome()
    }

    pub fn url_focused(&self) -> bool {
        self.url_focused
    }

    /// URL 欄の文字列。
    pub fn url_text(&self) -> &str {
        &self.url_text
    }

    pub fn type_text(&mut self, text: &str) -> Redraw {
        if !self.url_focused {
            return Redraw::default();
        }
        self.url_text
            .extend(text.chars().filter(|c| !c.is_control()));
        self.invalidate_chrome()
    }

    pub fn backspace(&mut self) -> Redraw {
        if !self.url_focused {
            return Redraw::default();
        }
        self.url_text.pop();
        self.invalidate_chrome()
    }

    pub fn cancel_url_edit(&mut self) -> Redraw {
        self.url_focused = false;
        self.url_text = self
            .active_tab()
            .and_then(Tab::url)
            .filter(|u| u.scheme() != "about")
            .map(Url::to_string)
            .unwrap_or_default();
        self.invalidate_chrome()
    }

    /// URL 欄で Enter。履歴に前方一致する候補があればそれを使う。
    pub fn submit_url(&mut self) -> Redraw {
        let input = self.url_text.trim().to_owned();
        let completed = self.history.as_ref().and_then(|h| {
            h.search(&input, 5)
                .into_iter()
                .map(|e| e.url)
                .find(|u| u.trim_start_matches("https://").starts_with(&input))
        });
        let target = completed
            .and_then(|u| Url::parse(&u).ok())
            .or_else(|| parse_input(&input));
        match target {
            Some(url) => self.navigate(url),
            None => self.cancel_url_edit(),
        }
    }

    // ---- スクロール ----

    /// 表示中のタブをスクロールさせる。新しい位置のフレームは後で届く。
    pub fn scroll_by(&mut self, dy: f32) -> bool {
        let Some(tab) = self.active_tab().map(|t| t.id) else {
            return false;
        };
        self.content.send(ToContent::ScrollBy { tab, dy })
    }

    /// 表示中のタブのスクロール位置（Content が最後に知らせた値）。
    pub fn scroll_y(&self) -> f32 {
        self.active_tab().map_or(0.0, |t| t.scroll_y)
    }

    /// UI の変化だけを知らせる（タブ数が増えたなど）。
    pub fn redraw_chrome(&mut self) -> Redraw {
        self.invalidate_chrome()
    }
}
