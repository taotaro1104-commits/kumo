//! タブとセッション履歴（戻る・進む）。ページ本体は Content プロセスが持ち、ここには表示用の状態だけを置く。

use std::time::Instant;

use url::Url;

use kumo_content::Lang;
use kumo_content::i18n::ui::Ui;
use kumo_content::protocol::{HoverKind, TabId};

use crate::governor::TabState;

/// タブ内の移動履歴。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Session {
    entries: Vec<Url>,
    index: usize,
}

impl Session {
    /// 新しいページへ移動する（現在より先の履歴は捨てる）。
    pub fn push(&mut self, url: Url) {
        if self.entries.is_empty() {
            self.entries.push(url);
            self.index = 0;
            return;
        }
        self.entries.truncate(self.index + 1);
        if self.entries.last() != Some(&url) {
            self.entries.push(url);
        }
        self.index = self.entries.len() - 1;
    }

    /// リダイレクトなどで現在の項目の URL を差し替える。
    pub fn replace_current(&mut self, url: Url) {
        match self.entries.get_mut(self.index) {
            Some(current) => *current = url,
            None => self.push(url),
        }
    }

    /// 現在の項目を捨てて、ひとつ前に戻す（移動先がダウンロードで、ページが変わらなかったとき）。
    /// 現在の項目が `url` でなければ何もしない。
    pub fn discard_current(&mut self, url: &Url) {
        if self.entries.get(self.index) != Some(url) {
            return;
        }
        self.entries.remove(self.index);
        self.index = self.index.min(self.entries.len().saturating_sub(1));
    }

    pub fn current(&self) -> Option<&Url> {
        self.entries.get(self.index)
    }

    pub fn can_go_back(&self) -> bool {
        self.index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    pub fn back(&mut self) -> Option<&Url> {
        if !self.can_go_back() {
            return None;
        }
        self.index -= 1;
        self.current()
    }

    pub fn forward(&mut self) -> Option<&Url> {
        if !self.can_go_forward() {
            return None;
        }
        self.index += 1;
        self.current()
    }
}

/// 1つのタブ（Browser 側の状態）。
pub struct Tab {
    pub id: TabId,
    pub session: Session,
    /// 表示中の文書の題名（まだ読み込んでいなければ `None`）
    pub(crate) title: Option<String>,
    /// 表示中の文書の URL（`#` を含む。同じ文書内の移動の判定に使う）
    pub(crate) shown_url: Option<Url>,
    /// 読み込み中の移動の番号
    pub loading: Option<u64>,
    /// ページ内の入力欄にフォーカスがある
    pub field_focused: bool,
    pub scroll_y: f32,
    /// 読み込みが終わった回数（取得し直したかの確認用）
    pub loads: u32,
    /// マウスの下のリンク先と種類（Content が知らせる）
    pub hover_link: Option<String>,
    pub hover_kind: HoverKind,
    /// メモリ管理による状態
    pub state: TabState,
    /// 最後に表示を離れた（または作られた）時刻。背景に回ってからの時間の基準
    pub(crate) last_active: Instant,
    /// POST で開いたページ（読み込み直すと同じにならないので破棄しない）
    pub(crate) posted: bool,
    /// 破棄から戻すときに、読み込み後に戻すスクロール位置
    pub(crate) restore_scroll: Option<f32>,
}

/// 題名が届く前のタブ見出し：ドメイン、ファイルならファイル名。
fn short_label(url: &Url) -> String {
    if let Some(host) = url.host_str().filter(|h| !h.is_empty()) {
        return host.trim_start_matches("www.").to_owned();
    }
    url.path_segments()
        .and_then(|mut s| s.next_back())
        .filter(|name| !name.is_empty())
        .map(|name| {
            percent_encoding::percent_decode_str(name)
                .decode_utf8_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| url.to_string())
}

impl Tab {
    pub fn new(id: TabId) -> Self {
        Self {
            id,
            session: Session::default(),
            title: None,
            shown_url: None,
            loading: None,
            field_focused: false,
            scroll_y: 0.0,
            loads: 0,
            hover_link: None,
            hover_kind: HoverKind::None,
            state: TabState::Live,
            last_active: Instant::now(),
            posted: false,
            restore_scroll: None,
        }
    }

    pub fn title(&self) -> String {
        self.title_in(Lang::default())
    }

    /// 見出し。題名がなければ URL から作り、URL もなければ「新しいタブ」（`lang` の言語）。
    pub fn title_in(&self, lang: Lang) -> String {
        match (&self.title, self.session.current()) {
            (Some(title), _) => title.clone(),
            (None, Some(url)) => short_label(url),
            (None, None) => Ui::NewTab.t(lang).to_owned(),
        }
    }

    pub fn url(&self) -> Option<&Url> {
        self.session.current()
    }

    pub fn is_loading(&self) -> bool {
        self.loading.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(&format!("https://example.com/{s}")).unwrap()
    }

    #[test]
    fn session_back_forward_and_truncation() {
        let mut s = Session::default();
        assert!(s.current().is_none());
        s.push(u("a"));
        s.push(u("b"));
        s.push(u("c"));
        assert_eq!(s.back(), Some(&u("b")));
        assert_eq!(s.back(), Some(&u("a")));
        assert_eq!(s.back(), None);
        assert_eq!(s.forward(), Some(&u("b")));
        // 途中から別ページへ進むと先の履歴は消える
        s.push(u("d"));
        assert!(!s.can_go_forward());
        assert_eq!(s.back(), Some(&u("b")));
        s.replace_current(u("b2"));
        assert_eq!(s.current(), Some(&u("b2")));
        // 同じ URL の連続は1つにまとめる
        s.forward();
        s.push(u("d"));
        assert!(!s.can_go_forward());
        assert_eq!(s.back(), Some(&u("b2")));
    }

    #[test]
    fn tab_title_falls_back_to_url() {
        let mut tab = Tab::new(1);
        assert_eq!(tab.title(), "新しいタブ");
        tab.session.push(u("x"));
        assert_eq!(tab.title(), "example.com", "題名が届く前はドメイン");
        let mut file = Tab::new(2);
        file.session
            .push(Url::parse("file:///C:/dir/my%20page.html").unwrap());
        assert_eq!(file.title(), "my page.html", "ファイルはファイル名");
        tab.title = Some("X".into());
        assert_eq!(tab.title(), "X");
        assert!(!tab.is_loading());
    }
}
