//! Content が落ちたときの立て直し：作り直せれば全タブを読み込み直し、できなければ読み込み中のまま残さない。

use kumo_content::protocol::ToContent;
use kumo_content::{FrameReader, Notify};
use url::Url;

use crate::browser::{Browser, ContentFactory, Redraw};

/// 作り直す回数の上限（壊れたページで作り直しを繰り返さないため）。
const MAX_RESTARTS: u32 = 3;

impl Browser {
    /// Content が落ちたときに作り直す関数を設定する。
    pub fn set_content_restart(&mut self, factory: ContentFactory) {
        self.restart = Some(factory);
    }

    /// Content から通知が届いたときに呼ぶ関数（ウィンドウのイベントループを起こす）。
    pub fn set_notify(&mut self, notify: Notify) {
        self.content.set_notify(notify.clone());
        self.notify = Some(notify);
    }

    /// Content が終わっていれば立て直す。届いている通知を全部取り出した後に呼ぶ。
    pub(crate) fn recover_content(&mut self) -> Redraw {
        if !self.content.is_closed() || self.content_lost {
            return Redraw::default();
        }
        self.frame = FrameReader::default();
        self.frame_damage = None;
        let client = if self.restarts < MAX_RESTARTS {
            self.restart.as_mut().and_then(|make| match make() {
                Ok(client) => Some(client),
                Err(e) => {
                    eprintln!("[content] 作り直せません: {e}");
                    None
                }
            })
        } else {
            None
        };
        let Some(client) = client else {
            eprintln!("[content] Content プロセスが終了しました");
            self.content_lost = true;
            for tab in &mut self.tabs {
                tab.loading = None;
                tab.field_focused = false;
            }
            return self.invalidate_chrome();
        };
        self.restarts += 1;
        eprintln!(
            "[content] Content プロセスが終了したので作り直します（{} 回目）",
            self.restarts
        );
        if let Some(notify) = &self.notify {
            client.set_notify(notify.clone());
        }
        self.content = client;
        self.send_viewport();
        self.content.send(ToContent::Settings {
            javascript: self.javascript,
        });
        self.send_load_level();
        self.send_images();
        for index in 0..self.tabs.len() {
            let tab = &mut self.tabs[index];
            let url = tab
                .shown_url
                .take()
                .or_else(|| tab.session.current().cloned())
                .unwrap_or_else(|| Url::parse("about:blank").expect("固定の URL"));
            tab.field_focused = false;
            self.start_load(index, url, false, None);
        }
        if let Some(tab) = self.tabs.get(self.active) {
            self.content.send(ToContent::Show { tab: tab.id });
        }
        self.invalidate_chrome()
    }
}
