//! Network サービス本体：コンテンツブロック → キャッシュ → 取得（再検証）→ 保存。
//! Network プロセスの中で全タブ共有の1つとして動く（スレッド安全）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use url::Url;

use crate::blocker::ContentBlocker;
use crate::cache::{CachedEntry, CachedMeta, DISK_BUDGET, HttpCache, MEMORY_BUDGET, unix};
use crate::cookies::CookiePersistence;
use crate::http_io::{is_stale_connection, read_response};
use crate::local_file::read_file;
use crate::policy::{CacheHeaders, Storability, storability};
use crate::protocol::{
    FetchError, FetchRequest, FetchResponse, NetworkStats, RequestMethod, ResourceKind,
};

const USER_AGENT: &str =
    "Mozilla/5.0 (compatible; Kumo/0.1; +https://github.com/taotaro1104-commits/kumo)";
const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: u32 = 10;
/// 接続ごとの送受信バッファ（ureq の既定 128KB は省メモリ方針には大きい）。
const CONNECTION_BUFFER: usize = 32 * 1024;
/// 使い回すために保持する接続の上限。
const IDLE_CONNECTIONS: usize = 6;
const IDLE_CONNECTIONS_PER_HOST: usize = 2;
const IDLE_AGE: Duration = Duration::from_secs(30);

/// Network プロセスの設定。
#[derive(Clone, Debug, Default)]
pub struct NetworkConfig {
    /// ディスクキャッシュとシリアライズ済みフィルタの置き場。`None` ならメモリのみ
    pub cache_dir: Option<PathBuf>,
    /// 追加のフィルタリスト（EasyList など）
    pub filter_lists: Vec<PathBuf>,
}

#[derive(Default)]
struct Counters {
    requests: AtomicU64,
    cache_hits: AtomicU64,
    revalidated: AtomicU64,
    blocked: AtomicU64,
}

pub struct NetworkService {
    agent: ureq::Agent,
    /// 別オリジンへのスクリプトの要求用。Cookie の保管庫が別で、使うたびに空にする
    anonymous_agent: ureq::Agent,
    cache: HttpCache,
    blocker: ContentBlocker,
    counters: Counters,
    cookies: CookiePersistence,
}

impl NetworkService {
    pub fn new(config: &NetworkConfig) -> Self {
        let agent_config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(TIMEOUT))
            .max_redirects(MAX_REDIRECTS)
            .user_agent(USER_AGENT)
            .input_buffer_size(CONNECTION_BUFFER)
            .output_buffer_size(CONNECTION_BUFFER)
            .max_idle_connections(IDLE_CONNECTIONS)
            .max_idle_connections_per_host(IDLE_CONNECTIONS_PER_HOST)
            .max_idle_age(IDLE_AGE)
            .build();
        let cache = config
            .cache_dir
            .as_ref()
            .and_then(
                |dir| match HttpCache::open(&dir.join("http"), MEMORY_BUDGET, DISK_BUDGET) {
                    Ok(cache) => Some(cache),
                    Err(e) => {
                        eprintln!(
                            "[network] ディスクキャッシュを開けないためメモリのみで動作: {e}"
                        );
                        None
                    }
                },
            )
            .unwrap_or_else(|| HttpCache::in_memory(MEMORY_BUDGET));
        let filter_cache = config
            .cache_dir
            .as_ref()
            .map(|dir| dir.join("adblock-engine.bin"));
        let blocker = ContentBlocker::load(&config.filter_lists, filter_cache.as_deref());
        let anonymous_agent = ureq::Agent::new_with_config(agent_config.clone());
        let agent = ureq::Agent::new_with_config(agent_config);
        let cookies =
            CookiePersistence::new(config.cache_dir.as_ref().map(|d| d.join("cookies.json")));
        cookies.load(&agent);
        Self {
            agent,
            anonymous_agent,
            cache,
            blocker,
            counters: Counters::default(),
            cookies,
        }
    }

    /// 要求に使うエージェント。別オリジンへのスクリプトの要求は資格情報（Cookie）を送らない別のものを使う。
    fn agent_for(&self, request: &FetchRequest) -> &ureq::Agent {
        if request.kind == ResourceKind::FetchAnonymous {
            &self.anonymous_agent
        } else {
            &self.agent
        }
    }

    /// 資格情報なしの要求で受け取った Cookie を捨てる（次の要求に持ち越さない）。
    fn forget_anonymous_cookies(&self, request: &FetchRequest) {
        if request.kind == ResourceKind::FetchAnonymous {
            self.anonymous_agent.cookie_jar_lock().clear();
        }
    }

    /// Cookie をディスクへ書き出す（終了時に呼ぶ）。
    pub fn persist(&self) {
        self.cookies.save(&self.agent);
    }

    pub fn stats(&self) -> NetworkStats {
        let c = &self.counters;
        NetworkStats {
            requests: c.requests.load(Ordering::Relaxed),
            cache_hits: c.cache_hits.load(Ordering::Relaxed),
            revalidated: c.revalidated.load(Ordering::Relaxed),
            blocked: c.blocked.load(Ordering::Relaxed),
            memory_cache_bytes: self.cache.memory_bytes() as u64,
            disk_cache_bytes: self.cache.disk_bytes(),
            private_bytes: 0,
        }
    }

    pub fn fetch(&self, request: &FetchRequest) -> Result<FetchResponse, FetchError> {
        self.counters.requests.fetch_add(1, Ordering::Relaxed);
        let url = Url::parse(&request.url)
            .map_err(|e| FetchError::BadUrl(format!("{}: {e}", request.url)))?;
        match url.scheme() {
            "http" | "https" => {}
            "file" => return read_file(request, &url),
            other => return Err(FetchError::UnsupportedScheme(other.to_owned())),
        }
        if self
            .blocker
            .should_block(url.as_str(), request.source_url.as_deref(), request.kind)
        {
            self.counters.blocked.fetch_add(1, Ordering::Relaxed);
            return Err(FetchError::Blocked);
        }
        match &request.method {
            RequestMethod::Post { content_type, body } => {
                return self.post(request, &url, content_type, body);
            }
            RequestMethod::Send {
                method,
                content_type,
                body,
            } => return self.send_other(request, &url, method, content_type.as_deref(), body),
            RequestMethod::Get => {}
        }
        // 資格情報なしの要求はキャッシュを使わない（Cookie つきで取った別の内容を返さないため）
        let cached = (request.kind != ResourceKind::FetchAnonymous)
            .then(|| self.cache.get(url.as_str()))
            .flatten();
        if let Some(entry) = &cached
            && entry.meta.is_fresh(SystemTime::now())
        {
            self.counters.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(from_entry(request.id, entry));
        }
        self.fetch_network(request, &url, cached.filter(|e| e.meta.has_validator()))
    }

    /// フォーム送信。キャッシュは参照も保存もしない（Cookie は通常どおり送受信する）。
    fn post(
        &self,
        request: &FetchRequest,
        url: &Url,
        content_type: &str,
        body: &[u8],
    ) -> Result<FetchResponse, FetchError> {
        let response = self
            .agent_for(request)
            .post(url.as_str())
            .header("Content-Type", content_type)
            .send(body)
            .map_err(network_error)?;
        self.finish_uncached(request, response)
    }

    /// GET 以外の任意のメソッド（スクリプトの `fetch`）。キャッシュしない。
    fn send_other(
        &self,
        request: &FetchRequest,
        url: &Url,
        method: &str,
        content_type: Option<&str>,
        body: &[u8],
    ) -> Result<FetchResponse, FetchError> {
        let mut builder = ureq::http::Request::builder()
            .method(method)
            .uri(url.as_str());
        if let Some(content_type) = content_type {
            builder = builder.header("Content-Type", content_type);
        }
        let http_request = builder
            .body(body.to_vec())
            .map_err(|e| FetchError::Network(e.to_string()))?;
        let response = self
            .agent_for(request)
            .run(http_request)
            .map_err(network_error)?;
        self.finish_uncached(request, response)
    }

    /// キャッシュしない応答の後始末（本文の読み取りと Cookie の保存）。
    fn finish_uncached(
        &self,
        request: &FetchRequest,
        mut response: ureq::http::Response<ureq::Body>,
    ) -> Result<FetchResponse, FetchError> {
        let status = response.status().as_u16();
        let (final_url, content_type, allow_origin, body) = read_response(&mut response)?;
        self.forget_anonymous_cookies(request);
        self.cookies.save_if_due(&self.agent);
        Ok(FetchResponse {
            id: request.id,
            status,
            final_url,
            content_type,
            body,
            from_cache: false,
            allow_origin,
        })
    }

    /// GET を送り、リダイレクトは1段ずつ自分でたどる。サーバーが閉じていた再利用の接続に当たったとき
    /// （close_notify なしの切断など）は、その段だけ新しい接続でやり直す
    /// （ureq の内部でたどると、途中の段の失敗でやり直しが最初からになり、同じ接続にまた当たってしまう）。
    fn get_following_redirects(
        &self,
        request: &FetchRequest,
        url: &Url,
        stale: Option<&CachedEntry>,
    ) -> Result<ureq::http::Response<ureq::Body>, FetchError> {
        let mut target = url.clone();
        for hop in 0..=MAX_REDIRECTS {
            let send = || {
                let mut builder = self.agent_for(request).get(target.as_str());
                if hop == 0
                    && let Some(entry) = stale
                {
                    if let Some(etag) = &entry.meta.etag {
                        builder = builder.header("If-None-Match", etag.as_str());
                    }
                    if let Some(modified) = &entry.meta.last_modified {
                        builder = builder.header("If-Modified-Since", modified.as_str());
                    }
                }
                builder.config().max_redirects(0).build().call()
            };
            let response = match send() {
                Err(e) if is_stale_connection(&e) => send(),
                result => result,
            }
            .map_err(network_error)?;
            let location = matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308)
                .then(|| header(response.headers(), "location"))
                .flatten();
            let Some(location) = location else {
                return Ok(response);
            };
            target = target
                .join(&location)
                .ok()
                .filter(|next| matches!(next.scheme(), "http" | "https"))
                .ok_or_else(|| {
                    FetchError::Network(format!("リダイレクト先が不正です: {location}"))
                })?;
        }
        Err(FetchError::Network("リダイレクトが多すぎます".to_owned()))
    }

    fn fetch_network(
        &self,
        request: &FetchRequest,
        url: &Url,
        stale: Option<CachedEntry>,
    ) -> Result<FetchResponse, FetchError> {
        let mut response = self.get_following_redirects(request, url, stale.as_ref())?;
        let status = response.status().as_u16();
        let headers = cache_headers(response.headers());
        let now = SystemTime::now();
        if status == 304
            && let Some(entry) = stale
        {
            self.counters.revalidated.fetch_add(1, Ordering::Relaxed);
            let meta = refreshed_meta(&entry.meta, &headers, now);
            self.cache.refresh(url.as_str(), meta.clone());
            return Ok(from_entry(
                request.id,
                &CachedEntry {
                    meta,
                    body: entry.body,
                },
            ));
        }
        let (final_url, content_type, allow_origin, body) = read_response(&mut response)?;
        self.forget_anonymous_cookies(request);
        let cacheable = request.kind != ResourceKind::FetchAnonymous;
        if cacheable && let Storability::Store { fresh_until } = storability(status, &headers, now)
        {
            let meta = CachedMeta {
                final_url: final_url.clone(),
                content_type: content_type.clone(),
                fresh_until: fresh_until.map(unix),
                etag: headers.etag.clone(),
                last_modified: headers.last_modified.clone(),
                size: body.len() as u64,
                stored_at: unix(now),
            };
            self.cache.put(url.as_str(), meta, &body);
        }
        self.cookies.save_if_due(&self.agent);
        Ok(FetchResponse {
            id: request.id,
            status,
            final_url,
            content_type,
            body,
            from_cache: false,
            allow_origin,
        })
    }
}

pub(crate) fn network_error(e: ureq::Error) -> FetchError {
    FetchError::Network(e.to_string())
}

pub(crate) fn header(headers: &ureq::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn cache_headers(headers: &ureq::http::HeaderMap) -> CacheHeaders {
    CacheHeaders {
        cache_control: header(headers, "cache-control"),
        expires: header(headers, "expires"),
        date: header(headers, "date"),
        last_modified: header(headers, "last-modified"),
        etag: header(headers, "etag"),
    }
}

/// 304 の新しいヘッダで期限と検証子を更新する（本文は保存済みのものを使う）。
fn refreshed_meta(old: &CachedMeta, headers: &CacheHeaders, now: SystemTime) -> CachedMeta {
    let merged = CacheHeaders {
        etag: headers.etag.clone().or_else(|| old.etag.clone()),
        last_modified: headers
            .last_modified
            .clone()
            .or_else(|| old.last_modified.clone()),
        ..headers.clone()
    };
    let fresh_until = match storability(200, &merged, now) {
        Storability::Store { fresh_until } => fresh_until.map(unix),
        Storability::NotStorable => None,
    };
    CachedMeta {
        fresh_until,
        etag: merged.etag,
        last_modified: merged.last_modified,
        stored_at: unix(now),
        ..old.clone()
    }
}

fn from_entry(id: u64, entry: &CachedEntry) -> FetchResponse {
    FetchResponse {
        id,
        status: 200,
        final_url: entry.meta.final_url.clone(),
        content_type: entry.meta.content_type.clone(),
        body: entry.body.to_vec(),
        from_cache: true,
        allow_origin: None,
    }
}
