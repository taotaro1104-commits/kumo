//! 取得スレッドで行う仕事：文書の取得とパース、スクリプトや `fetch` の取得。
//! どれも Boa（`Send` でない）に触れないので、Content のメインスレッドとは別のスレッドで動かせる。

use kumo_dom::NodeId;
use kumo_js::FetchOutcome;
use kumo_net::process::{FetchResult, Fetcher};
use kumo_net::protocol::{FetchError, ResourceKind};
use url::Url;

use crate::fetch::{FetchedPage, fetch_document};
use crate::level::LoadLevel;
use crate::protocol::TabId;

/// 文書の取得の依頼。
pub struct FetchJob {
    pub tab: TabId,
    pub nav: u64,
    pub url: Url,
    pub post_body: Option<Vec<u8>>,
    pub level: LoadLevel,
    pub images: bool,
}

/// 文書の取得の結果。
pub struct Fetched {
    pub(crate) tab: TabId,
    pub(crate) nav: u64,
    pub(crate) url: Url,
    pub(crate) result: Result<FetchedPage, FetchError>,
}

impl FetchJob {
    pub fn run(self, fetcher: &dyn Fetcher) -> Fetched {
        let result = fetch_document(fetcher, &self.url, self.post_body, self.level, self.images);
        Fetched {
            tab: self.tab,
            nav: self.nav,
            url: self.url,
            result,
        }
    }
}

/// スクリプトの `fetch` を送る。別オリジンへは資格情報（Cookie）なしで送り、応答は
/// `Access-Control-Allow-Origin` で許された場合だけ返す（同一オリジンポリシー）。
fn script_request(fetcher: &dyn Fetcher, job: &SubJob) -> Result<FetchOutcome, String> {
    let (Ok(page), Ok(target)) = (Url::parse(&job.source), Url::parse(&job.url)) else {
        return Err("URL が正しくありません".to_owned());
    };
    if !kumo_js::may_reach(&page, &target) {
        return Err("ローカルのファイルへは取得できません".to_owned());
    }
    let cross = !same_origin(&page, &target);
    if cross && !is_simple_request(&job.method, job.content_type.as_deref()) {
        return Err(
            "別のオリジンへは、事前確認（プリフライト）が要らない単純な要求しか送れません"
                .to_owned(),
        );
    }
    let kind = if cross {
        ResourceKind::FetchAnonymous
    } else {
        ResourceKind::Fetch
    };
    let response = fetcher.send(
        &job.method,
        &job.url,
        kind,
        job.content_type.as_deref(),
        job.body.clone().unwrap_or_default(),
        Some(job.source.as_str()),
    );
    if cross
        && let Ok(r) = &response
        && target.scheme() != "file"
        && !cors_allows(&page, r.allow_origin.as_deref())
    {
        return Err("CORS ポリシーにより、別のオリジンの応答は読めません".to_owned());
    }
    outcome(response)
}

/// ページが読み込んだ後に出した取得（動的に足した `<script>` と、スクリプトの `fetch`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubKind {
    Script(NodeId),
    Api(u64),
}

pub struct SubJob {
    pub tab: TabId,
    /// 要求したときのページの読み込み番号（その後に別のページへ移っていたら結果を捨てる）
    pub nav: u64,
    pub kind: SubKind,
    pub url: String,
    pub method: String,
    pub content_type: Option<String>,
    pub body: Option<Vec<u8>>,
    /// 要求元のページの URL（コンテンツブロックの判定に使う）
    pub source: String,
    pub level: LoadLevel,
}

pub struct SubFetched {
    pub(crate) tab: TabId,
    pub(crate) nav: u64,
    pub(crate) kind: SubKind,
    pub(crate) result: Result<FetchOutcome, String>,
}

/// 2つの URL が同じオリジンか。`file:` のページは `file:` どうしを同じとみなす。
pub(crate) fn same_origin(a: &Url, b: &Url) -> bool {
    if a.scheme() == "file" && b.scheme() == "file" {
        return true;
    }
    a.origin() == b.origin()
}

/// 別オリジンへの要求で、プリフライトなしで送れる「単純な要求」か（GET・HEAD・POST、単純な Content-Type）。
/// プリフライト（OPTIONS）は未対応なので、それが必要な要求は送らない。
pub(crate) fn is_simple_request(method: &str, content_type: Option<&str>) -> bool {
    let method_ok = matches!(method, "GET" | "HEAD" | "POST");
    let type_ok = content_type.is_none_or(|t| {
        let t = t
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        matches!(
            t.as_str(),
            "text/plain" | "application/x-www-form-urlencoded" | "multipart/form-data"
        )
    });
    method_ok && type_ok
}

/// 別オリジンの応答をスクリプトに読ませてよいか：`Access-Control-Allow-Origin` が `*` か、ページのオリジンそのもの。
pub(crate) fn cors_allows(page: &Url, allow_origin: Option<&str>) -> bool {
    let Some(allowed) = allow_origin.map(str::trim) else {
        return false;
    };
    allowed == "*" || allowed == page.origin().ascii_serialization()
}

fn outcome(result: FetchResult) -> Result<FetchOutcome, String> {
    match result {
        Ok(response) => Ok(FetchOutcome {
            status: response.status,
            url: response.final_url,
            content_type: response.content_type,
            body: response.body,
        }),
        Err(FetchError::Blocked) => Err("ブロックされました".to_owned()),
        Err(e) => Err(e.to_string()),
    }
}

impl SubJob {
    pub fn run(self, fetcher: &dyn Fetcher) -> SubFetched {
        let source = Some(self.source.as_str());
        let result = match self.kind {
            SubKind::Script(_) => outcome(fetcher.fetch(&self.url, ResourceKind::Script, source)),
            SubKind::Api(_) => script_request(fetcher, &self),
        }
        .and_then(|response| match self.kind {
            SubKind::Script(_) if response.body.len() > self.level.script_limits().max_bytes => {
                Err(format!(
                    "{} バイトは大きすぎて実行しません",
                    response.body.len()
                ))
            }
            _ => Ok(response),
        });
        SubFetched {
            tab: self.tab,
            nav: self.nav,
            kind: self.kind,
            result,
        }
    }
}

/// 取得スレッドが受け持つ仕事。
pub enum Work {
    Document(FetchJob),
    Sub(SubJob),
    /// 取得スレッドのヒープを回収する。DOM などは取得スレッドで確保されるので、別スレッドで捨てても
    /// 確保元が回収するまで OS に返らない（タブの破棄・凍結の後に送る）
    Trim,
}

/// 仕事の結果。
pub enum Done {
    Document(Box<Fetched>),
    Sub(SubFetched),
}

impl Work {
    pub fn tab(&self) -> TabId {
        match self {
            Self::Document(job) => job.tab,
            Self::Sub(job) => job.tab,
            Self::Trim => 0, // タブ番号は 1 から
        }
    }

    /// 仕事を実行する。結果を返さない仕事（[`Work::Trim`]）は `None`。
    pub fn run(self, fetcher: &dyn Fetcher) -> Option<Done> {
        match self {
            Self::Document(job) => Some(Done::Document(Box::new(job.run(fetcher)))),
            Self::Sub(job) => Some(Done::Sub(job.run(fetcher))),
            Self::Trim => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn origins_requests_and_cors_decisions() {
        assert!(same_origin(
            &url("https://a.example/x"),
            &url("https://a.example/y?q")
        ));
        assert!(!same_origin(
            &url("https://a.example/"),
            &url("http://a.example/")
        ));
        assert!(!same_origin(
            &url("https://a.example/"),
            &url("https://b.example/")
        ));
        assert!(!same_origin(
            &url("https://a.example/"),
            &url("https://a.example:8443/")
        ));
        assert!(same_origin(
            &url("file:///a.html"),
            &url("file:///b/c.json")
        ));

        assert!(is_simple_request("GET", None));
        assert!(is_simple_request(
            "POST",
            Some("application/x-www-form-urlencoded; charset=UTF-8")
        ));
        assert!(
            !is_simple_request("POST", Some("application/json")),
            "JSON はプリフライトが必要"
        );
        assert!(!is_simple_request("PUT", None));
        assert!(!is_simple_request("DELETE", Some("text/plain")));

        let page = url("https://a.example/page");
        assert!(cors_allows(&page, Some("*")));
        assert!(cors_allows(&page, Some("https://a.example")));
        assert!(!cors_allows(&page, Some("https://b.example")));
        assert!(!cors_allows(&page, Some("null")));
        assert!(!cors_allows(&page, None));
    }

    /// 取得先を記録するだけの取得元。
    struct Recorder(
        std::sync::Mutex<Vec<(String, ResourceKind)>>,
        Option<String>,
    );

    impl Fetcher for Recorder {
        fn fetch(&self, url: &str, kind: ResourceKind, _: Option<&str>) -> FetchResult {
            self.0.lock().unwrap().push((url.to_owned(), kind));
            Ok(kumo_net::protocol::FetchResponse {
                id: 0,
                status: 200,
                final_url: url.to_owned(),
                content_type: None,
                body: b"ok".to_vec(),
                from_cache: false,
                allow_origin: self.1.clone(),
            })
        }
        fn post(&self, url: &str, _: &str, _: Vec<u8>, source: Option<&str>) -> FetchResult {
            self.fetch(url, ResourceKind::Document, source)
        }
        fn stats(&self) -> Option<kumo_net::protocol::NetworkStats> {
            None
        }
    }

    fn job(source: &str, target: &str, method: &str, content_type: Option<&str>) -> SubJob {
        SubJob {
            tab: 1,
            nav: 1,
            kind: SubKind::Api(1),
            url: target.to_owned(),
            method: method.to_owned(),
            content_type: content_type.map(str::to_owned),
            body: None,
            source: source.to_owned(),
            level: Default::default(),
        }
    }

    #[test]
    fn script_requests_follow_the_same_origin_policy() {
        let open = Recorder(Default::default(), Some("*".into()));
        let closed = Recorder(Default::default(), None);
        let page = "https://a.example/app";
        // 同じオリジン：資格情報つき（Fetch）、CORS ヘッダは要らない
        assert!(script_request(&closed, &job(page, "https://a.example/api", "GET", None)).is_ok());
        assert_eq!(closed.0.lock().unwrap()[0].1, ResourceKind::Fetch);
        // 別オリジン：資格情報なし（FetchAnonymous）で送り、許可がなければ読ませない
        let denied =
            script_request(&closed, &job(page, "https://b.example/api", "GET", None)).unwrap_err();
        assert!(denied.contains("CORS"), "{denied}");
        assert_eq!(closed.0.lock().unwrap()[1].1, ResourceKind::FetchAnonymous);
        assert!(script_request(&open, &job(page, "https://b.example/api", "GET", None)).is_ok());
        // プリフライトが要る要求は送らない
        let before = open.0.lock().unwrap().len();
        let err = script_request(
            &open,
            &job(
                page,
                "https://b.example/api",
                "POST",
                Some("application/json"),
            ),
        )
        .unwrap_err();
        assert!(err.contains("プリフライト"), "{err}");
        assert_eq!(
            open.0.lock().unwrap().len(),
            before,
            "ネットワークへは出ない"
        );
        // 遠隔のページからローカルのファイルは取得できない
        let err =
            script_request(&open, &job(page, "file:///C:/secret.txt", "GET", None)).unwrap_err();
        assert!(err.contains("ローカル"), "{err}");
        // ローカルのページはローカルのファイルを読める
        assert!(
            script_request(
                &closed,
                &job(
                    "file:///C:/dir/a.html",
                    "file:///C:/dir/data.json",
                    "GET",
                    None
                )
            )
            .is_ok()
        );
    }
}
