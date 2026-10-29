//! Network プロセスとのやりとり。
//!
//! - サーバ側（子プロセス）：[`serve_listener`] がローカルソケットで複数のクライアント（Browser・Content）を
//!   受け付け、接続ごとにワーカーで並行に取得して応答する。大きな本文は接続ごとの共有メモリのリングで渡す。
//! - クライアント側：[`NetworkClient`] が子プロセスを起動するか、既存の Network プロセスへ接続する。
//! - どちらも [`Fetcher`] トレイトで同じように使える（テストや単体実行では [`NetworkService`] を直接使う）。

pub use crate::client::NetworkClient;
pub use crate::server::{RING_CAPACITY, SHARED_BODY_MIN, serve, serve_listener};

use std::path::Path;

use crate::protocol::{
    DownloadEvent, FetchError, FetchRequest, FetchResponse, NetworkStats, RequestMethod,
    ResourceKind,
};
use crate::service::{NetworkConfig, NetworkService};

pub type FetchResult = Result<FetchResponse, FetchError>;

/// 待ち受けるソケット名を渡す起動引数。
pub const LISTEN_ARG: &str = "--listen";

/// リソース取得の抽象。プロセス越しでもプロセス内でも同じ API で使う。
pub trait Fetcher: Send + Sync {
    fn fetch(&self, url: &str, kind: ResourceKind, source_url: Option<&str>) -> FetchResult;

    /// 複数のリソースをまとめて取得する（実装によっては並行に）。結果は入力と同じ順。
    fn fetch_all(
        &self,
        requests: &[(String, ResourceKind)],
        source_url: Option<&str>,
    ) -> Vec<FetchResult> {
        requests
            .iter()
            .map(|(url, kind)| self.fetch(url, *kind, source_url))
            .collect()
    }

    fn stats(&self) -> Option<NetworkStats>;

    /// `url` を `dir` のファイルへ保存し始める（経過は [`Fetcher::take_download_events`]）。
    /// 返す ID で中止できる。保存に対応しない取得元は `None`。
    fn start_download(&self, _url: &str, _dir: &Path, _source_url: Option<&str>) -> Option<u64> {
        None
    }

    fn cancel_download(&self, _id: u64) {}

    /// 溜まっているダウンロードの経過を取り出す。
    fn take_download_events(&self) -> Vec<DownloadEvent> {
        Vec::new()
    }

    /// スクリプトからの要求（`fetch`）。GET はキャッシュを使い、それ以外は送るだけ。
    /// 既定の実装は GET と POST だけを扱う（他は [`NetworkService`] と [`NetworkClient`] が処理する）。
    /// `kind` は [`ResourceKind::Fetch`]（同じオリジン）か [`ResourceKind::FetchAnonymous`]（別オリジン）。
    fn send(
        &self,
        method: &str,
        url: &str,
        kind: ResourceKind,
        content_type: Option<&str>,
        body: Vec<u8>,
        source_url: Option<&str>,
    ) -> FetchResult {
        match method {
            "GET" | "HEAD" => self.fetch(url, kind, source_url),
            "POST" => self.post(
                url,
                content_type.unwrap_or("text/plain;charset=UTF-8"),
                body,
                source_url,
            ),
            other => Err(FetchError::Network(format!(
                "{other} には未対応の取得元です"
            ))),
        }
    }

    /// フォームを POST で送信する（文書として取得する）。
    fn post(
        &self,
        url: &str,
        content_type: &str,
        body: Vec<u8>,
        source_url: Option<&str>,
    ) -> FetchResult;
}

/// スクリプトの要求のメソッド。GET と HEAD はキャッシュを使う GET として扱う。
pub(crate) fn script_method(
    method: &str,
    content_type: Option<&str>,
    body: Vec<u8>,
) -> RequestMethod {
    match method {
        "GET" | "HEAD" => RequestMethod::Get,
        other => RequestMethod::Send {
            method: other.to_owned(),
            content_type: content_type.map(str::to_owned),
            body,
        },
    }
}

pub(crate) fn post_method(content_type: &str, body: Vec<u8>) -> RequestMethod {
    RequestMethod::Post {
        content_type: content_type.to_owned(),
        body,
    }
}

impl Fetcher for NetworkService {
    fn send(
        &self,
        method: &str,
        url: &str,
        kind: ResourceKind,
        content_type: Option<&str>,
        body: Vec<u8>,
        source_url: Option<&str>,
    ) -> FetchResult {
        NetworkService::fetch(
            self,
            &FetchRequest {
                id: 0,
                url: url.to_owned(),
                method: script_method(method, content_type, body),
                kind,
                source_url: source_url.map(str::to_owned),
            },
        )
    }

    fn fetch(&self, url: &str, kind: ResourceKind, source_url: Option<&str>) -> FetchResult {
        NetworkService::fetch(
            self,
            &FetchRequest {
                id: 0,
                url: url.to_owned(),
                method: RequestMethod::Get,
                kind,
                source_url: source_url.map(str::to_owned),
            },
        )
    }

    fn post(
        &self,
        url: &str,
        content_type: &str,
        body: Vec<u8>,
        source_url: Option<&str>,
    ) -> FetchResult {
        NetworkService::fetch(
            self,
            &FetchRequest {
                id: 0,
                url: url.to_owned(),
                method: post_method(content_type, body),
                kind: ResourceKind::Document,
                source_url: source_url.map(str::to_owned),
            },
        )
    }

    fn stats(&self) -> Option<NetworkStats> {
        Some(NetworkService::stats(self))
    }
}

/// 設定を子プロセスの起動引数にする（[`parse_config`] の逆）。
pub fn config_args(config: &NetworkConfig) -> Vec<std::ffi::OsString> {
    let mut args = Vec::new();
    if let Some(dir) = &config.cache_dir {
        args.push("--cache-dir".into());
        args.push(dir.clone().into_os_string());
    }
    for list in &config.filter_lists {
        args.push("--filter-list".into());
        args.push(list.clone().into_os_string());
    }
    args
}

/// 子プロセスの起動引数から設定を読む。知らない引数は無視する。
pub fn parse_config(args: impl Iterator<Item = String>) -> NetworkConfig {
    let mut config = NetworkConfig::default();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cache-dir" => config.cache_dir = args.next().map(Into::into),
            "--filter-list" => config.filter_lists.extend(args.next().map(Into::into)),
            _ => {}
        }
    }
    config
}

/// 起動引数から待ち受けるソケット名を読む。
pub fn listen_name(args: &[String]) -> Option<&str> {
    let at = args.iter().position(|a| a == LISTEN_ARG)?;
    args.get(at + 1).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_through_arguments() {
        let config = NetworkConfig {
            cache_dir: Some("c".into()),
            filter_lists: vec!["a.txt".into(), "b.txt".into()],
        };
        let mut args: Vec<String> = config_args(&config)
            .into_iter()
            .map(|a| a.into_string().unwrap())
            .collect();
        args.extend([LISTEN_ARG.to_owned(), "sock".to_owned()]);
        let parsed = parse_config(args.clone().into_iter());
        assert_eq!(parsed.cache_dir, config.cache_dir);
        assert_eq!(parsed.filter_lists, config.filter_lists);
        assert_eq!(listen_name(&args), Some("sock"));
        assert_eq!(listen_name(&["--listen".to_owned()]), None);
    }
}
