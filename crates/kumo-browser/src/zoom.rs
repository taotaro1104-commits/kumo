//! ページの拡大・縮小（タブごと）。倍率は Content に伝えて、レイアウトと描画に反映させる。

use kumo_content::protocol::ToContent;

use crate::browser::{Browser, Redraw};

/// 拡大・縮小で順にたどる倍率（%）。
pub const ZOOM_STEPS: [u32; 17] = [
    25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400, 500,
];

impl Browser {
    /// 表示中のタブの拡大率（%）。
    pub fn zoom_percent(&self) -> u32 {
        self.active_tab().map_or(100, |t| t.zoom)
    }

    /// 一段階拡大する。
    pub fn zoom_in(&mut self) -> Redraw {
        let now = self.zoom_percent();
        let next = ZOOM_STEPS.iter().copied().find(|&s| s > now);
        self.set_zoom_percent(next.unwrap_or(now))
    }

    /// 一段階縮小する。
    pub fn zoom_out(&mut self) -> Redraw {
        let now = self.zoom_percent();
        let prev = ZOOM_STEPS.iter().rev().copied().find(|&s| s < now);
        self.set_zoom_percent(prev.unwrap_or(now))
    }

    /// 100% に戻す。
    pub fn zoom_reset(&mut self) -> Redraw {
        self.set_zoom_percent(100)
    }

    /// 表示中のタブの拡大率を `percent`（%）にする（25〜500 に丸める）。
    pub fn set_zoom_percent(&mut self, percent: u32) -> Redraw {
        let percent = percent.clamp(ZOOM_STEPS[0], ZOOM_STEPS[ZOOM_STEPS.len() - 1]);
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return Redraw::default();
        };
        if tab.zoom == percent {
            return Redraw::default();
        }
        tab.zoom = percent;
        let id = tab.id;
        self.content.send(ToContent::Zoom { tab: id, percent });
        self.invalidate_chrome()
    }

    /// Content を作り直したあとに、拡大中のタブの倍率を伝え直す。
    pub(crate) fn send_zooms(&mut self) {
        for tab in self.tabs.iter().filter(|t| t.zoom != 100) {
            self.content.send(ToContent::Zoom {
                tab: tab.id,
                percent: tab.zoom,
            });
        }
    }
}
