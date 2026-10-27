//! コンテンツブロック（Brave の adblock crate）。広告・トラッカーを読み込まないことが
//! 最大の省メモリ策（設計書）。構築済みエンジンはシリアライズしてディスクに置き、次回は読み込むだけにする。

use std::path::{Path, PathBuf};

use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;

use crate::protocol::ResourceKind;

/// 既定のフィルタ（EasyList がなくても代表的な広告・計測を止める最小セット）。
pub const DEFAULT_RULES: &str = include_str!("default_filters.txt");

pub struct ContentBlocker {
    engine: Engine,
}

impl ContentBlocker {
    /// フィルタ文字列からエンジンを作る。
    pub fn from_rules(rules: &str) -> Self {
        let mut set = FilterSet::new(false);
        set.add_filter_list(rules.to_owned(), ParseOptions::default());
        Self {
            engine: Engine::new_with_filter_set(set),
        }
    }

    /// 既定フィルタ＋ `extra_lists`（EasyList などのファイル）から作る。
    /// `cache_file` にシリアライズ済みエンジンがあり、どのリストより新しければそれを読み込む。
    pub fn load(extra_lists: &[PathBuf], cache_file: Option<&Path>) -> Self {
        if let Some(cache) = cache_file
            && is_newer_than_lists(cache, extra_lists)
            && let Ok(bytes) = std::fs::read(cache)
        {
            let mut engine = Engine::default();
            if engine.deserialize(&bytes).is_ok() {
                return Self { engine };
            }
        }
        let mut rules = String::from(DEFAULT_RULES);
        for list in extra_lists {
            match std::fs::read_to_string(list) {
                Ok(text) => {
                    rules.push('\n');
                    rules.push_str(&text);
                }
                Err(e) => eprintln!("[adblock] {} を読めません: {e}", list.display()),
            }
        }
        let blocker = Self::from_rules(&rules);
        if let Some(cache) = cache_file
            && let Err(e) = std::fs::write(cache, blocker.engine.serialize())
        {
            eprintln!("[adblock] エンジンを保存できません: {e}");
        }
        blocker
    }

    /// 遮断すべきリクエストか。文書（トップレベル）は遮断しない。
    pub fn should_block(&self, url: &str, source_url: Option<&str>, kind: ResourceKind) -> bool {
        if kind == ResourceKind::Document {
            return false;
        }
        let source = source_url.unwrap_or(url);
        Request::new(url, source, kind.as_adblock_type(), "GET")
            .is_ok_and(|request| self.engine.check_network_request(&request).should_block())
    }
}

fn is_newer_than_lists(cache: &Path, lists: &[PathBuf]) -> bool {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let Some(cache_time) = modified(cache) else {
        return false;
    };
    lists
        .iter()
        .all(|list| modified(list).is_none_or(|t| t <= cache_time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_third_party_ads_but_not_documents() {
        let blocker = ContentBlocker::from_rules("||ads.example.com^\n/tracker.js$third-party");
        let page = Some("https://news.example.org/article");
        assert!(blocker.should_block(
            "https://ads.example.com/banner.png",
            page,
            ResourceKind::Image
        ));
        assert!(blocker.should_block(
            "https://cdn.other.net/tracker.js",
            page,
            ResourceKind::Other
        ));
        assert!(!blocker.should_block(
            "https://news.example.org/tracker.js",
            page,
            ResourceKind::Other
        ));
        assert!(!blocker.should_block("https://ads.example.com/", None, ResourceKind::Document));
        assert!(!blocker.should_block(
            "https://news.example.org/a.css",
            page,
            ResourceKind::Stylesheet
        ));
    }

    #[test]
    fn default_rules_block_common_trackers() {
        let blocker = ContentBlocker::from_rules(DEFAULT_RULES);
        let page = Some("https://example.org/");
        assert!(blocker.should_block(
            "https://www.google-analytics.com/analytics.js",
            page,
            ResourceKind::Other
        ));
        assert!(blocker.should_block(
            "https://securepubads.g.doubleclick.net/tag/js/gpt.js",
            page,
            ResourceKind::Other
        ));
    }

    #[test]
    fn serialized_engine_round_trips_through_cache_file() {
        let dir = std::env::temp_dir().join(format!("kumo-adblock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let list = dir.join("list.txt");
        std::fs::write(&list, "||blocked.test^").unwrap();
        let cache = dir.join("engine.bin");
        let _ = std::fs::remove_file(&cache);
        let first = ContentBlocker::load(std::slice::from_ref(&list), Some(&cache));
        assert!(cache.exists(), "構築したエンジンを保存する");
        let page = Some("https://site.test/");
        assert!(first.should_block("https://blocked.test/x.png", page, ResourceKind::Image));
        let second = ContentBlocker::load(std::slice::from_ref(&list), Some(&cache));
        assert!(second.should_block("https://blocked.test/x.png", page, ResourceKind::Image));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
