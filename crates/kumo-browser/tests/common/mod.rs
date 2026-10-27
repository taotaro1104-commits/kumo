//! 統合テストの共通部品：テスト用サイトと、プロセス内で動かすブラウザ。
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use kumo_browser::{Browser, HistoryDb};
use kumo_net::service::{NetworkConfig, NetworkService};
use url::Url;

pub struct Site {
    pub dir: PathBuf,
}

impl Site {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kumo-browser-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let write = |file: &str, body: &str| std::fs::write(dir.join(file), body).unwrap();
        write(
            "a.html",
            "<html><head><title>Page A</title><link rel=stylesheet href=style.css></head>\
             <body><p id=p><a href=b.html>page B</a> link</p></body></html>",
        );
        write(
            "b.html",
            "<html><head><title>Page B</title></head><body><p>B body</p></body></html>",
        );
        write(
            "style.css",
            "body { margin: 0; font-size: 20px } p { margin: 0 }",
        );
        Self { dir }
    }

    pub fn url(&self, file: &str) -> Url {
        Url::from_file_path(self.dir.join(file)).unwrap()
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn browser(history: Option<HistoryDb>) -> Browser {
    let fetcher = Arc::new(NetworkService::new(&NetworkConfig::default()));
    Browser::in_process(fetcher, history, (800.0, 600.0))
}

pub fn settle(b: &mut Browser) {
    assert!(b.settle(Duration::from_secs(20)), "読み込みが終わる");
}

pub fn title(b: &Browser) -> String {
    b.active_tab().unwrap().title()
}
