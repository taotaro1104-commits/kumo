//! ブックマークの操作（追加・削除・一覧）。

use url::Url;

use crate::bookmarks_page::{BOOKMARKS_URL, UNBOOKMARK_PREFIX};
use crate::browser::{Browser, Redraw};

impl Browser {
    /// 表示中のページのブックマークを付け外しする。`about:` のページは対象にしない。
    pub fn toggle_bookmark(&mut self) -> Redraw {
        let (Some(history), Some(tab)) = (&self.history, self.active_tab()) else {
            return Redraw::default();
        };
        let Some(url) = tab.url().filter(|u| u.scheme() != "about") else {
            return Redraw::default();
        };
        let result = if history.is_bookmarked(url.as_str()) {
            history.remove_bookmark(url.as_str())
        } else {
            history.add_bookmark(url.as_str(), &tab.title())
        };
        if let Err(e) = result {
            eprintln!("[bookmark] {e}");
        }
        self.invalidate_chrome()
    }

    /// 一覧ページを開く（表示中のタブが空の新しいタブならそこで、それ以外は新しいタブで）。
    pub fn show_bookmarks(&mut self) -> Redraw {
        let url = Url::parse(BOOKMARKS_URL).expect("固定の URL");
        let on_new_tab = self
            .active_tab()
            .is_some_and(|t| t.url().is_none_or(|u| u.as_str() == crate::NEW_TAB_URL));
        if on_new_tab {
            self.navigate(url)
        } else {
            self.open_tab(Some(url), true)
        }
    }

    /// 一覧ページの「削除」リンク。一覧を表示しているときだけ受け付け、扱ったら `true`。
    pub(crate) fn remove_bookmark_link(&mut self, link: &Url) -> bool {
        if !link.as_str().starts_with(UNBOOKMARK_PREFIX) {
            return false;
        }
        let on_list = self
            .active_tab()
            .and_then(|t| t.url())
            .is_some_and(|u| u.as_str() == BOOKMARKS_URL);
        if !on_list {
            return true; // 他のページからは操作させない
        }
        let target = link
            .query_pairs()
            .find(|(k, _)| k == "u")
            .map(|(_, v)| v.into_owned());
        if let (Some(target), Some(history)) = (target, &self.history)
            && let Err(e) = history.remove_bookmark(&target)
        {
            eprintln!("[bookmark] {e}");
        }
        self.reload();
        true
    }
}
