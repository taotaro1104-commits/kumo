//! マウスの位置への反応：UI のリンクの強調、カーソルの形、リンク先の吹き出し（左下）。

use kumo_content::Page;
use kumo_content::protocol::{HoverKind, ToContent};
use url::Url;

use crate::browser::{Browser, Redraw};
use crate::url_display::format_url;

/// 吹き出しの高さ（px）と、コンテンツ幅に対する最大の割合。
const BUBBLE_HEIGHT: f32 = 24.0;
const BUBBLE_MAX_RATIO: f32 = 0.6;

/// カーソルの形。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    Default,
    /// リンク・ボタン（指）
    Link,
    /// 文字の入力欄（I ビーム）
    Text,
}

/// リンク先の吹き出し。`page` の左上から `width`×`height` を描く。
pub struct StatusBubble<'a> {
    pub page: &'a Page,
    pub width: u32,
    pub height: u32,
}

/// 作った吹き出しのページ（同じ文字列なら作り直さない）。
pub(crate) struct BubbleCache {
    text: String,
    page: Page,
    width: u32,
    height: u32,
}

const BUBBLE_CSS: &str = "html, body { margin: 0; background: #ffffff; }
.b { float: left; height: 24px; line-height: 22px; box-sizing: border-box; padding: 0 10px;
     background: #f8f9fa; border: 1px solid #d5d9de; border-radius: 0 8px 0 0; white-space: nowrap; overflow: hidden;
     font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; font-size: 12px; color: #5f6368; }
.h { color: #1f2328; }";

impl Browser {
    /// UI 上でマウスが動いた。乗っているリンクが変わったら UI を作り直す。
    pub fn hover_chrome(&mut self, x: f32, y: f32) -> Redraw {
        let link = self.chrome_page().list.link_at(x, y).map(|h| h.to_string());
        let mut redraw = self.leave_content();
        if link != self.chrome_hover {
            self.chrome_hover = link;
            redraw = redraw.merge(self.invalidate_chrome());
        }
        redraw
    }

    /// コンテンツ上でマウスが動いた（`y` は領域上端から）。下にあるものは Content が調べて知らせる。
    pub fn hover_content(&mut self, x: f32, y: f32) -> Redraw {
        let redraw = self.clear_chrome_hover();
        if let Some(tab) = self.active_tab() {
            self.content.send(ToContent::Hover { tab: tab.id, x, y });
        }
        redraw
    }

    /// マウスがウィンドウの外へ出た。
    pub fn leave_window(&mut self) -> Redraw {
        self.clear_chrome_hover().merge(self.leave_content())
    }

    fn clear_chrome_hover(&mut self) -> Redraw {
        if self.chrome_hover.take().is_some() {
            return self.invalidate_chrome();
        }
        Redraw::default()
    }

    /// コンテンツの hover を外す（Content にも領域外の位置を送り、次に戻ったとき知らせ直させる）。
    fn leave_content(&mut self) -> Redraw {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return Redraw::default();
        };
        if tab.hover_kind == HoverKind::None {
            return Redraw::default();
        }
        tab.hover_kind = HoverKind::None;
        tab.hover_link = None;
        let id = tab.id;
        self.content.send(ToContent::Hover {
            tab: id,
            x: -1.0,
            y: -1.0,
        });
        Redraw::CONTENT
    }

    /// Content からの通知を記録する。
    pub(crate) fn hovered(&mut self, tab: u64, link: Option<String>, kind: HoverKind) -> Redraw {
        let Some(index) = self.tab_index(tab) else {
            return Redraw::default();
        };
        let tab = &mut self.tabs[index];
        tab.hover_link = link;
        tab.hover_kind = kind;
        if index == self.active {
            Redraw::CONTENT
        } else {
            Redraw::default()
        }
    }

    /// 今のカーソルの形。
    pub fn pointer(&self) -> Pointer {
        if self.chrome_hover.is_some() {
            return Pointer::Link;
        }
        match self.active_tab().map(|t| t.hover_kind) {
            Some(HoverKind::Link | HoverKind::Control) => Pointer::Link,
            Some(HoverKind::Field) => Pointer::Text,
            _ => Pointer::Default,
        }
    }

    /// 表示中のタブでマウスが乗っているリンクの行き先（左下に出す吹き出し）。
    pub fn status_bubble(&mut self) -> Option<StatusBubble<'_>> {
        let link = self.active_tab()?.hover_link.clone()?;
        let parts = format_url(&link);
        let text = format!("{}{}", parts.host, parts.rest);
        if self.bubble.as_ref().is_none_or(|b| b.text != text) {
            self.bubble = Some(self.build_bubble(text, &parts.host, &parts.rest));
        }
        let bubble = self.bubble.as_ref()?;
        Some(StatusBubble {
            page: &bubble.page,
            width: bubble.width,
            height: bubble.height,
        })
    }

    fn build_bubble(&mut self, text: String, host: &str, rest: &str) -> BubbleCache {
        let max_width = (self.content_viewport().0 * BUBBLE_MAX_RATIO).max(80.0);
        let html = format!(
            "<html><head><style>{BUBBLE_CSS} .b {{ max-width: {max_width}px; }}</style></head>\
             <body><div class=b><span class=h>{}</span>{}</div></body></html>",
            kumo_content::escape(host),
            kumo_content::escape(rest)
        );
        let url = Url::parse("about:kumo-status").expect("固定の URL");
        let page = Page::from_html(
            &html,
            url,
            &mut self.chrome_services,
            (max_width, BUBBLE_HEIGHT),
        );
        // 文字の幅に合わせて切り出す（浮動した吹き出しの右端）
        let width = page
            .list
            .entries
            .iter()
            .map(|e| e.bounds.right())
            .fold(0.0_f32, f32::max)
            .min(max_width)
            .ceil() as u32;
        BubbleCache {
            text,
            page,
            width: width.max(1),
            height: BUBBLE_HEIGHT as u32,
        }
    }
}
