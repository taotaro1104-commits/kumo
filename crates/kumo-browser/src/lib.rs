//! Kumo のブラウザ骨格：タブ、セッション履歴、閲覧履歴、ブラウザ UI。
//! ページの処理は Content（`kumo-content`、別プロセスまたはスレッド）、取得は Network プロセス（`kumo-net`）が行う。
#![forbid(unsafe_code)]

mod about_memory;
mod actions;
mod browser;
mod chrome;
mod events;
mod governor;
mod history;
mod memory;
mod newtab;
mod offline;
mod pointer;
mod recovery;
mod tab;
mod url_display;

pub use about_memory::ABOUT_MEMORY_URL;
pub use browser::{Browser, ContentFactory, Redraw};
pub use chrome::{CHROME_HEIGHT, ChromeAction, ChromeState, TabLabel, chrome_html, parse_input};
pub use events::FrameView;
pub use governor::{Action, Policy, TabInfo, TabState, plan};
pub use history::{HistoryDb, HistoryEntry, Visit};
pub use newtab::{NEW_TAB_URL, new_tab_html};
pub use offline::Offline;
pub use pointer::{Pointer, StatusBubble};
pub use tab::{Session, Tab};
pub use url_display::{Security, UrlParts, format_url};
