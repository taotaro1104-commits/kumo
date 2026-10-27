//! 取得しない [`Fetcher`]。ブラウザ UI のページは外部資源を読まないので、これで足りる。

use kumo_net::process::{FetchResult, Fetcher};
use kumo_net::protocol::{FetchError, NetworkStats, ResourceKind};

pub struct Offline;

impl Fetcher for Offline {
    fn fetch(&self, _url: &str, _kind: ResourceKind, _source_url: Option<&str>) -> FetchResult {
        Err(FetchError::Blocked)
    }

    fn post(
        &self,
        _url: &str,
        _content_type: &str,
        _body: Vec<u8>,
        _source_url: Option<&str>,
    ) -> FetchResult {
        Err(FetchError::Blocked)
    }

    fn stats(&self) -> Option<NetworkStats> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_fetches() {
        assert_eq!(
            Offline.fetch("https://a/", ResourceKind::Document, None),
            Err(FetchError::Blocked)
        );
        assert_eq!(
            Offline.post("https://a/", "x", Vec::new(), None),
            Err(FetchError::Blocked)
        );
        assert!(Offline.stats().is_none());
    }
}
