//! Content からの通知の取り込み、フレームの受け渡し、テストやヘッドレス用の待ち合わせ。

use std::time::{Duration, Instant};

use kumo_content::protocol::{ContentStats, FromContent, ToContent};
use url::Url;

use crate::browser::{Browser, Redraw};

/// 届いたフレーム（共有メモリの画素と、前回から描き直した行）。
pub struct FrameView<'a> {
    pub pixels: &'a [u32],
    pub width: u32,
    pub height: u32,
    pub damage: (u32, u32),
}

impl Browser {
    /// 届いている通知を全部処理する（待たない）。ウィンドウのイベントループから呼ぶ。
    pub fn pump(&mut self) -> Redraw {
        let mut redraw = Redraw::default();
        while let Some(message) = self.content.try_recv() {
            redraw = redraw.merge(self.handle(message));
        }
        redraw.merge(self.recover_content())
    }

    fn handle(&mut self, message: FromContent) -> Redraw {
        match message {
            FromContent::Loaded {
                tab,
                nav,
                url,
                title,
            } => self.loaded(tab, nav, &url, title),
            FromContent::Navigate {
                tab,
                url,
                post_body,
                new_tab,
            } => {
                let (Some(index), Ok(url)) = (self.tab_index(tab), Url::parse(&url)) else {
                    return Redraw::default();
                };
                if new_tab {
                    return self.open_tab(Some(url), false);
                }
                self.load(index, url, true, false, post_body);
                self.invalidate_chrome()
            }
            FromContent::FieldFocus { tab, focused } => {
                if let Some(index) = self.tab_index(tab) {
                    self.tabs[index].field_focused = focused;
                }
                Redraw::default()
            }
            FromContent::Scrolled { tab, y } => {
                if let Some(index) = self.tab_index(tab) {
                    self.tabs[index].scroll_y = y;
                }
                Redraw::default()
            }
            FromContent::Frame {
                tab,
                path,
                width,
                height,
                damage,
            } => self.frame_arrived(tab, &path, (width, height), damage),
            FromContent::Hovered { tab, link, kind } => self.hovered(tab, link, kind),
            FromContent::TitleChanged { tab, title } => {
                let Some(index) = self.tab_index(tab) else {
                    return Redraw::default();
                };
                self.tabs[index].title = Some(title);
                self.invalidate_chrome()
            }
            FromContent::Synced { .. } | FromContent::Stats { .. } => Redraw::default(),
        }
    }

    fn loaded(&mut self, tab: u64, nav: u64, url: &str, title: String) -> Redraw {
        let Some(index) = self.tab_index(tab) else {
            return Redraw::default();
        };
        let tab = &mut self.tabs[index];
        if tab.loading != Some(nav) {
            return Redraw::default(); // その後に別の移動を依頼した
        }
        tab.loading = None;
        tab.loads += 1;
        if let Ok(final_url) = Url::parse(url) {
            if final_url.scheme() != "about"
                && let Some(history) = &self.history
                && let Err(e) = history.record(final_url.as_str(), &title)
            {
                eprintln!("[history] {e}");
            }
            if tab.session.current().is_some() {
                tab.session.replace_current(final_url.clone());
            }
            if index == self.active && !self.url_focused && final_url.scheme() != "about" {
                self.url_text = final_url.to_string();
            }
            tab.shown_url = Some(final_url);
        }
        tab.title = Some(title);
        self.invalidate_chrome()
    }

    fn frame_arrived(
        &mut self,
        tab: u64,
        path: &str,
        size: (u32, u32),
        damage: (u32, u32),
    ) -> Redraw {
        let current = self.active_tab().is_some_and(|t| t.id == tab);
        if !current || self.frame.attach(path, size.0, size.1).is_err() {
            // 切り替える前のタブのフレームは使わない
            self.content.send(ToContent::FrameDone);
            return Redraw::default();
        }
        self.frame_damage = Some(match self.frame_damage {
            // 写す前に次が来ることはない（FrameDone を待つ）が、念のため範囲を合わせる
            Some((s, e)) => (s.min(damage.0), e.max(damage.1)),
            None => damage,
        });
        Redraw::CONTENT
    }

    /// 届いたフレームがあれば `present` に渡して写させ、Content へ写し終えたことを返す。
    pub fn present_frame(&mut self, present: impl FnOnce(FrameView<'_>)) -> bool {
        let Some(damage) = self.frame_damage.take() else {
            return false;
        };
        let (width, height) = self.frame.size();
        if let Some(pixels) = self.frame.pixels() {
            present(FrameView {
                pixels,
                width,
                height,
                damage,
            });
        }
        self.content.send(ToContent::FrameDone);
        true
    }

    /// 届いたがまだ写していないフレームがあるか。
    pub fn frame_pending(&self) -> bool {
        self.frame_damage.is_some()
    }

    /// 最後に届いたフレームの画素（ウィンドウのバッファを作り直したとき全面を写すため）。
    pub fn last_frame(&self) -> Option<FrameView<'_>> {
        let (width, height) = self.frame.size();
        self.frame.pixels().map(|pixels| FrameView {
            pixels,
            width,
            height,
            damage: (0, height),
        })
    }

    /// 通知を1つ待って処理する。届かなければ `false`。
    fn pump_wait(&mut self, deadline: Instant) -> bool {
        match self.recv_until(deadline) {
            Some(message) => {
                self.handle(message);
                true
            }
            None => false,
        }
    }

    fn request_id(&mut self) -> u64 {
        let id = self.next_request;
        self.next_request += 1;
        id
    }

    /// 全タブの読み込みと、それまでに送った要求の処理が終わるまで待つ（テスト・ヘッドレス用）。
    /// 届いたフレームは写したことにする。時間内に終われば `true`。
    pub fn settle(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        'resync: loop {
            let id = self.request_id();
            let generation = self.restarts;
            self.content.send(ToContent::Sync { id });
            loop {
                let Some(message) = self.recv_until(deadline) else {
                    return false;
                };
                let synced = message == FromContent::Synced { id };
                self.handle(message);
                self.present_frame(|_| {});
                if self.restarts != generation {
                    // 送った Sync は落ちた Content に届いていない。作り直した方へ送り直す
                    continue 'resync;
                }
                if synced {
                    break;
                }
            }
            if !self.tabs.iter().any(|t| t.is_loading()) {
                return true;
            }
            if !self.pump_wait(deadline) {
                return false;
            }
        }
    }

    /// 通知を1つ待つ。Content が落ちていたら立て直して待ち続ける（立て直せなければ `None`）。
    fn recv_until(&mut self, deadline: Instant) -> Option<FromContent> {
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.content.recv_timeout(left) {
                Ok(message) => return Some(message),
                Err(true) => {
                    self.recover_content();
                    if self.content_lost || self.content.is_closed() {
                        return None;
                    }
                }
                Err(false) => return None,
            }
        }
    }

    /// 表示中のタブを全面描かせ、そのフレームが届くまで待つ（ヘッドレス用）。
    pub fn wait_frame(&mut self, timeout: Duration) -> Option<FrameView<'_>> {
        let deadline = Instant::now() + timeout;
        let tab = self.active_tab()?.id;
        self.discard_frame();
        self.content.send(ToContent::Show { tab });
        while self.frame_damage.is_none() {
            if !self.pump_wait(deadline) {
                return None;
            }
        }
        self.frame_damage = None; // 下で写し終えたことを通知する
        self.content.send(ToContent::FrameDone);
        self.last_frame()
    }

    /// Content の計測値を問い合わせる。
    pub fn content_stats(&mut self, timeout: Duration) -> Option<ContentStats> {
        let deadline = Instant::now() + timeout;
        let id = self.request_id();
        self.content.send(ToContent::Stats { id });
        loop {
            match self.recv_until(deadline)? {
                FromContent::Stats { id: got, stats } if got == id => return Some(stats),
                other => {
                    self.handle(other);
                }
            }
        }
    }
}
