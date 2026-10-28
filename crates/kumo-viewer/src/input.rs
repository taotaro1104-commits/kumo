//! キーボード・マウス操作を Browser の操作に変換する。

use kumo_browser::{Browser, CHROME_HEIGHT, ChromeAction, Redraw};
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// 矢印キー・ホイール1段のスクロール量（px）。
pub const LINE_SCROLL: f32 = 48.0;

/// 入力の処理結果。
#[derive(Default)]
pub struct Outcome {
    pub redraw: Redraw,
    pub exit: bool,
    pub report: bool,
}

impl Outcome {
    fn redraw(redraw: Redraw) -> Self {
        Self {
            redraw,
            ..Self::default()
        }
    }
}

pub fn key(browser: &mut Browser, event: &KeyEvent, mods: ModifiersState) -> Outcome {
    if event.state != ElementState::Pressed {
        return Outcome::default();
    }
    if browser.url_focused() {
        return url_bar_key(browser, event);
    }
    if browser.field_focused() && !mods.control_key() && !mods.alt_key() {
        return field_key(browser, event);
    }
    let ctrl = mods.control_key();
    let page_height = browser.content_viewport().1;
    // 新しい位置のフレームは Content から届く
    let scroll = |browser: &mut Browser, dy: f32| {
        browser.scroll_by(dy);
        Outcome::default()
    };
    match &event.logical_key {
        Key::Character(c) if ctrl => match c.to_lowercase().as_str() {
            "t" => Outcome::redraw(browser.open_tab(None, true)),
            "w" => Outcome::redraw(browser.close_tab(browser.active_index())),
            "l" => Outcome::redraw(browser.focus_url_bar()),
            "r" => Outcome::redraw(browser.reload()),
            "d" => Outcome::redraw(browser.toggle_bookmark()),
            "b" => Outcome::redraw(browser.show_bookmarks()),
            "m" => Outcome {
                report: true,
                ..Outcome::default()
            },
            _ => Outcome::default(),
        },
        Key::Named(NamedKey::Tab) if ctrl => {
            let count = browser.tabs().len().max(1);
            let next = if mods.shift_key() {
                (browser.active_index() + count - 1) % count
            } else {
                (browser.active_index() + 1) % count
            };
            Outcome::redraw(browser.activate(next))
        }
        Key::Named(NamedKey::Home) if mods.alt_key() => Outcome::redraw(browser.go_home()),
        Key::Named(NamedKey::ArrowLeft) if mods.alt_key() => Outcome::redraw(browser.back()),
        Key::Named(NamedKey::ArrowRight) if mods.alt_key() => Outcome::redraw(browser.forward()),
        Key::Named(NamedKey::BrowserBack) => Outcome::redraw(browser.back()),
        Key::Named(NamedKey::BrowserForward) => Outcome::redraw(browser.forward()),
        Key::Named(NamedKey::F5) => Outcome::redraw(browser.reload()),
        Key::Named(NamedKey::ArrowDown) => scroll(browser, LINE_SCROLL),
        Key::Named(NamedKey::ArrowUp) => scroll(browser, -LINE_SCROLL),
        Key::Named(NamedKey::PageDown | NamedKey::Space) => scroll(browser, page_height * 0.9),
        Key::Named(NamedKey::PageUp) => scroll(browser, -page_height * 0.9),
        Key::Named(NamedKey::Home) => scroll(browser, f32::MIN / 2.0),
        Key::Named(NamedKey::End) => scroll(browser, f32::MAX / 2.0),
        Key::Named(NamedKey::Escape) => Outcome {
            exit: true,
            ..Outcome::default()
        },
        _ => Outcome::default(),
    }
}

/// ページ内の入力欄に文字を入れる。
fn field_key(browser: &mut Browser, event: &KeyEvent) -> Outcome {
    let redraw = match &event.logical_key {
        Key::Named(NamedKey::Enter) => browser.submit_field(),
        Key::Named(NamedKey::Escape) => browser.blur_field(),
        Key::Named(NamedKey::Backspace) => browser.backspace_field(),
        _ => match &event.text {
            Some(text) => browser.type_into_field(text),
            None => Redraw::default(),
        },
    };
    Outcome::redraw(redraw)
}

fn url_bar_key(browser: &mut Browser, event: &KeyEvent) -> Outcome {
    let redraw = match &event.logical_key {
        Key::Named(NamedKey::Enter) => browser.submit_url(),
        Key::Named(NamedKey::Escape) => browser.cancel_url_edit(),
        Key::Named(NamedKey::Backspace) => browser.backspace(),
        _ => match &event.text {
            Some(text) => browser.type_text(text),
            None => Redraw::default(),
        },
    };
    Outcome::redraw(redraw)
}

/// クリック。UI 上なら UI の操作、コンテンツ上なら Content へ渡す。
pub fn click(
    browser: &mut Browser,
    x: f32,
    y: f32,
    button: MouseButton,
    mods: ModifiersState,
) -> Outcome {
    if y < CHROME_HEIGHT {
        let href = browser.chrome_page().list.link_at(x, y).cloned();
        let action = href.as_deref().and_then(ChromeAction::from_href);
        return match (action, button) {
            // タブの中クリックで閉じる
            (Some(ChromeAction::ActivateTab(i)), MouseButton::Middle) => {
                Outcome::redraw(browser.close_tab(i))
            }
            (Some(action), MouseButton::Left) => Outcome::redraw(browser.chrome_action(action)),
            _ => Outcome::default(),
        };
    }
    // 部品・リンクの判定は Content が行う（移動の依頼が後で届く）
    match button {
        MouseButton::Left | MouseButton::Middle => {
            let new_tab = button == MouseButton::Middle || mods.control_key();
            Outcome::redraw(browser.click_content(x, y - CHROME_HEIGHT, new_tab))
        }
        _ => Outcome::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    use kumo_net::service::{NetworkConfig, NetworkService};

    fn browser_with_page() -> (Browser, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("kumo-input-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.html"),
            "<title>A</title><body style='margin:0;font-size:20px'><a href=b.html>to B</a></body>",
        )
        .unwrap();
        std::fs::write(dir.join("b.html"), "<title>B</title><p>B</p>").unwrap();
        let fetcher = Arc::new(NetworkService::new(&NetworkConfig::default()));
        let mut browser = Browser::in_process(fetcher, None, (800.0, 600.0));
        let url = url::Url::from_file_path(dir.join("a.html")).unwrap();
        browser.open_tab(Some(url), true);
        settle(&mut browser);
        (browser, dir)
    }

    fn settle(browser: &mut Browser) {
        assert!(browser.settle(Duration::from_secs(20)));
    }

    fn link_center(list: &kumo_paint::DisplayList, href: &str) -> (f32, f32) {
        let link = list.links.iter().find(|l| &*l.href == href).expect(href);
        (
            link.rect.x + link.rect.w / 2.0,
            link.rect.y + link.rect.h / 2.0,
        )
    }

    #[test]
    fn clicks_on_chrome_and_content() {
        let (mut browser, dir) = browser_with_page();
        let none = ModifiersState::empty();
        // コンテンツ上のリンク（UI の高さ分ずらした位置）
        click(
            &mut browser,
            5.0,
            CHROME_HEIGHT + 10.0,
            MouseButton::Left,
            none,
        );
        settle(&mut browser);
        assert_eq!(browser.active_tab().unwrap().title(), "B");
        let back = link_center(&browser.chrome_page().list, "kumo:back");
        click(&mut browser, back.0, back.1, MouseButton::Left, none);
        settle(&mut browser);
        assert_eq!(browser.active_tab().unwrap().title(), "A");
        // Ctrl+クリックは背景タブ
        click(
            &mut browser,
            5.0,
            CHROME_HEIGHT + 10.0,
            MouseButton::Left,
            ModifiersState::CONTROL,
        );
        settle(&mut browser);
        assert_eq!(browser.tabs().len(), 2);
        assert_eq!(browser.active_index(), 0);
        // UI の「+」で新しいタブ、タブの中クリックで閉じる
        let plus = link_center(&browser.chrome_page().list, "kumo:new");
        click(&mut browser, plus.0, plus.1, MouseButton::Left, none);
        assert_eq!(browser.tabs().len(), 3);
        let tab1 = link_center(&browser.chrome_page().list, "kumo:tab/1");
        click(&mut browser, tab1.0, tab1.1, MouseButton::Middle, none);
        assert_eq!(browser.tabs().len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }
}
