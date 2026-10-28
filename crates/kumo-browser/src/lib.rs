//! Kumo のブラウザ骨格：タブ、セッション履歴、閲覧履歴、ブラウザ UI。
//! ページの処理は Content（`kumo-content`、別プロセスまたはスレッド）、取得は Network プロセス（`kumo-net`）が行う。
#![forbid(unsafe_code)]

mod about_memory;
mod actions;
mod bookmark_actions;
mod bookmarks;
mod bookmarks_page;
mod browser;
mod chrome;
mod chrome_css;
mod chrome_page;
#[cfg(test)]
mod chrome_tests;
mod events;
mod governor;
mod history;
mod home;
mod login_actions;
mod logins;
mod memory;
mod newtab;
mod offline;
mod passwords_page;
mod pointer;
mod recovery;
mod settings;
mod settings_page;
mod tab;
mod url_display;
mod vault;

pub use about_memory::ABOUT_MEMORY_URL;
pub use bookmarks::Bookmark;
pub use bookmarks_page::BOOKMARKS_URL;
pub use browser::{Browser, ContentFactory, Redraw};
pub use chrome::{CHROME_HEIGHT, ChromeAction, ChromeState, TabLabel, chrome_html, parse_input};
pub use events::FrameView;
pub use governor::{Action, Policy, TabInfo, TabState, plan};
pub use history::{HistoryDb, HistoryEntry, Visit};
pub use kumo_content::{Lang, LoadLevel};
pub use logins::{LoginSummary, origin_of};
pub use newtab::{NEW_TAB_URL, new_tab_html};
pub use offline::Offline;
pub use passwords_page::PASSWORDS_URL;
pub use pointer::{Pointer, StatusBubble};
pub use settings_page::SETTINGS_URL;
pub use tab::{Session, Tab};
pub use url_display::{Security, UrlParts, format_url};
pub use vault::Vault;
