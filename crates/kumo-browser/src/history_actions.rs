//! 閲覧履歴の操作（一覧ページを開く、1件の削除、全消去）。

use url::Url;

use crate::browser::{Browser, Redraw};
use crate::history_page::{CLEAR_HISTORY, HISTORY_URL, UNHISTORY_PREFIX};

impl Browser {
    /// 履歴ページを開く（表示中のタブが空の新しいタブならそこで、それ以外は新しいタブで）。
    pub fn show_history(&mut self) -> Redraw {
        self.show_internal(HISTORY_URL)
    }

    /// 履歴ページのリンク（削除・全消去）。そのページを表示しているときだけ受け付け、扱ったら `true`。
    pub(crate) fn history_link(&mut self, link: &Url) -> bool {
        let clear = link.as_str() == CLEAR_HISTORY;
        if !clear && !link.as_str().starts_with(UNHISTORY_PREFIX) {
            return false;
        }
        if !self.active_url_is(HISTORY_URL) {
            return true; // 他のページからは操作させない
        }
        let Some(history) = &self.history else {
            return true;
        };
        let result = if clear {
            history.clear_visits()
        } else {
            let target = link
                .query_pairs()
                .find(|(k, _)| k == "u")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default();
            history.remove_visit(&target)
        };
        if let Err(e) = result {
            eprintln!("[history] {e}");
        }
        self.reload();
        true
    }
}
