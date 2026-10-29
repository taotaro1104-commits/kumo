mod common;

use common::TestServer;
use kumo_net::process::Fetcher;
use kumo_net::protocol::{FetchError, FetchRequest, RequestMethod, ResourceKind};
use kumo_net::service::{NetworkConfig, NetworkService};

fn request(url: &str, kind: ResourceKind) -> FetchRequest {
    FetchRequest {
        id: 1,
        url: url.to_owned(),
        method: RequestMethod::Get,
        kind,
        source_url: Some("http://site.test/".into()),
    }
}

#[test]
fn fresh_responses_are_served_from_cache() {
    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    let url = server.url("/fresh.css");
    let first = service
        .fetch(&request(&url, ResourceKind::Stylesheet))
        .unwrap();
    assert_eq!(first.body, b"a{color:red}");
    assert!(!first.from_cache);
    let second = service
        .fetch(&request(&url, ResourceKind::Stylesheet))
        .unwrap();
    assert!(second.from_cache);
    assert_eq!(server.hits("/fresh.css"), 1, "2回目はネットワークに出ない");
    assert_eq!(service.stats().cache_hits, 1);
}

#[test]
fn etag_revalidation_uses_304() {
    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    let url = server.url("/etag.css");
    service
        .fetch(&request(&url, ResourceKind::Stylesheet))
        .unwrap();
    let again = service
        .fetch(&request(&url, ResourceKind::Stylesheet))
        .unwrap();
    assert_eq!(again.body, b"b{color:blue}", "304 なら保存済みの本文");
    assert!(again.from_cache);
    assert_eq!(server.hits("/etag.css"), 2, "no-cache なので毎回再検証する");
    assert_eq!(service.stats().revalidated, 1);
}

#[test]
fn no_store_redirects_and_errors() {
    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    let url = server.url("/nostore.css");
    service
        .fetch(&request(&url, ResourceKind::Stylesheet))
        .unwrap();
    service
        .fetch(&request(&url, ResourceKind::Stylesheet))
        .unwrap();
    assert_eq!(server.hits("/nostore.css"), 2);

    let redirected = service
        .fetch(&request(&server.url("/redirect"), ResourceKind::Document))
        .unwrap();
    assert!(redirected.final_url.ends_with("/fresh.css"));
    assert_eq!(redirected.status, 200);

    let missing = service
        .fetch(&request(&server.url("/missing"), ResourceKind::Other))
        .unwrap();
    assert_eq!(missing.status, 404);
    assert!(matches!(
        service.fetch(&request("data:text/plain,hi", ResourceKind::Other)),
        Err(FetchError::UnsupportedScheme(_))
    ));
    assert!(matches!(
        service.fetch(&request("not a url", ResourceKind::Other)),
        Err(FetchError::BadUrl(_))
    ));
    assert!(matches!(
        service.fetch(&request("http://127.0.0.1:1/", ResourceKind::Document)),
        Err(FetchError::Network(_))
    ));
}

#[test]
fn blocked_requests_never_reach_the_network() {
    let service = NetworkService::new(&NetworkConfig::default());
    let ad = request(
        "https://securepubads.g.doubleclick.net/gpt.js",
        ResourceKind::Other,
    );
    assert_eq!(service.fetch(&ad), Err(FetchError::Blocked));
    assert_eq!(service.stats().blocked, 1);
}

#[test]
fn disk_cache_persists_across_service_instances() {
    let server = TestServer::start();
    let dir = std::env::temp_dir().join(format!("kumo-net-svc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let config = NetworkConfig {
        cache_dir: Some(dir.clone()),
        filter_lists: Vec::new(),
    };
    let url = server.url("/fresh.css");
    NetworkService::new(&config)
        .fetch(&request(&url, ResourceKind::Stylesheet))
        .unwrap();
    let restarted = NetworkService::new(&config);
    let cached = restarted
        .fetch(&request(&url, ResourceKind::Stylesheet))
        .unwrap();
    assert!(cached.from_cache, "再起動後もディスクから返す");
    assert_eq!(server.hits("/fresh.css"), 1);
    assert!(dir.join("adblock-engine.bin").exists());
    drop(restarted);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reads_local_files() {
    let path = std::env::temp_dir().join(format!("kumo-net-file-{}.html", std::process::id()));
    std::fs::write(&path, "<p>hi</p>").unwrap();
    let url = url::Url::from_file_path(&path).unwrap();
    let response = NetworkService::new(&NetworkConfig::default())
        .fetch(&request(url.as_str(), ResourceKind::Document))
        .unwrap();
    assert_eq!(response.body, b"<p>hi</p>");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn persistent_cookies_survive_restart_but_session_cookies_do_not() {
    let server = TestServer::start();
    let dir = std::env::temp_dir().join(format!("kumo-net-cookie-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let config = NetworkConfig {
        cache_dir: Some(dir.clone()),
        filter_lists: Vec::new(),
    };
    let echo = server.url("/echo-cookie");
    {
        let service = NetworkService::new(&config);
        service
            .fetch(&request(&server.url("/set-cookie"), ResourceKind::Document))
            .unwrap();
        let sent = service
            .fetch(&request(&echo, ResourceKind::Document))
            .unwrap();
        let sent = String::from_utf8(sent.body).unwrap();
        assert!(
            sent.contains("persistent=yes") && sent.contains("session=1"),
            "{sent}"
        );
        service.persist();
    }
    assert!(dir.join("cookies.json").exists());
    let restarted = NetworkService::new(&config);
    let sent = restarted
        .fetch(&request(&echo, ResourceKind::Document))
        .unwrap();
    let sent = String::from_utf8(sent.body).unwrap();
    assert!(
        sent.contains("persistent=yes"),
        "永続 Cookie は再起動後も送る: {sent}"
    );
    assert!(
        !sent.contains("session=1"),
        "セッション Cookie は消える: {sent}"
    );
    drop(restarted);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn posts_form_bodies_without_caching() {
    use kumo_net::process::Fetcher;
    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    let url = server.url("/echo");
    for _ in 0..2 {
        let response = Fetcher::post(
            &service,
            &url,
            "application/x-www-form-urlencoded",
            b"q=rust+lang&x=1".to_vec(),
            None,
        )
        .unwrap();
        let body = String::from_utf8(response.body).unwrap();
        assert!(
            body.contains("POST application/x-www-form-urlencoded q=rust+lang&amp;x=1")
                || body.contains("POST application/x-www-form-urlencoded q=rust+lang&x=1"),
            "{body}"
        );
        assert!(!response.from_cache);
    }
    assert_eq!(server.hits("/echo"), 2, "POST は毎回送る");
}

#[test]
fn script_requests_use_any_method_and_are_not_cached() {
    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    for method in ["PUT", "DELETE", "POST", "PATCH"] {
        let response = service
            .send(
                method,
                &server.url("/echo"),
                ResourceKind::Fetch,
                Some("application/json"),
                br#"{"a":1}"#.to_vec(),
                None,
            )
            .unwrap();
        let body = String::from_utf8_lossy(&response.body).into_owned();
        assert!(
            body.contains(&format!("{method} application/json {{\"a\":1}}")),
            "{method}: {body}"
        );
    }
    // Content-Type なしでも送れる
    let bare = service
        .send(
            "PUT",
            &server.url("/echo"),
            ResourceKind::Fetch,
            None,
            b"x".to_vec(),
            None,
        )
        .unwrap();
    assert!(String::from_utf8_lossy(&bare.body).contains("PUT  x"));
    // GET は通常の取得（キャッシュあり）
    for _ in 0..2 {
        service
            .send(
                "GET",
                &server.url("/fresh.css"),
                ResourceKind::Fetch,
                None,
                Vec::new(),
                None,
            )
            .unwrap();
    }
    assert_eq!(server.hits("/fresh.css"), 1);
    assert_eq!(server.hits("/echo"), 5, "GET 以外はキャッシュしない");
}

#[test]
fn anonymous_requests_send_no_cookies_skip_the_cache_and_expose_cors_headers() {
    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    service
        .fetch(&request(&server.url("/set-cookie"), ResourceKind::Other))
        .unwrap();
    let get = |path: &str, kind: ResourceKind| {
        service
            .send("GET", &server.url(path), kind, None, Vec::new(), None)
            .unwrap()
    };
    let with_cookie = get("/echo-cookie", ResourceKind::Fetch);
    assert!(String::from_utf8_lossy(&with_cookie.body).contains("persistent=yes"));
    let without = get("/echo-cookie", ResourceKind::FetchAnonymous);
    assert_eq!(
        String::from_utf8_lossy(&without.body),
        "",
        "資格情報なしの要求は Cookie を送らない"
    );
    // 資格情報なしの要求で受け取った Cookie は、次の要求には持ち越さない
    get("/set-cookie", ResourceKind::FetchAnonymous);
    let later = get("/echo-cookie", ResourceKind::FetchAnonymous);
    assert_eq!(String::from_utf8_lossy(&later.body), "");
    // ヘッダを読める／キャッシュしない
    assert_eq!(
        get("/cors", ResourceKind::FetchAnonymous)
            .allow_origin
            .as_deref(),
        Some("*")
    );
    assert_eq!(get("/fresh.css", ResourceKind::Fetch).allow_origin, None);
    get("/fresh.css", ResourceKind::FetchAnonymous);
    get("/fresh.css", ResourceKind::FetchAnonymous);
    assert_eq!(
        server.hits("/fresh.css"),
        3,
        "通常の1回 + 資格情報なしの2回（キャッシュを使わない）"
    );
}

#[test]
fn redirects_are_followed_hop_by_hop_with_limits() {
    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    let chained = service
        .fetch(&request(
            &server.url("/redirect-chain"),
            ResourceKind::Document,
        ))
        .unwrap();
    assert!(
        chained.final_url.ends_with("/fresh.css"),
        "{}",
        chained.final_url
    );
    assert_eq!(chained.status, 200);
    // 終わらない転送は上限で止める
    assert!(matches!(
        service.fetch(&request(&server.url("/loop"), ResourceKind::Document)),
        Err(FetchError::Network(_))
    ));
    // http(s) 以外へは移動しない
    assert!(matches!(
        service.fetch(&request(
            &server.url("/redirect-to-file"),
            ResourceKind::Document
        )),
        Err(FetchError::Network(_))
    ));
}

#[test]
fn files_to_save_are_reported_instead_of_returned_as_pages() {
    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    for path in ["/file.zip", "/data.bin"] {
        let result = service.fetch(&request(&server.url(path), ResourceKind::Document));
        assert_eq!(result, Err(FetchError::Download), "{path}");
    }
    // 画像などの部品としての取得や、HTML はそのまま返す
    let part = service
        .fetch(&request(&server.url("/data.bin"), ResourceKind::Other))
        .unwrap();
    assert_eq!(part.body, b"0123456789");
    let page = service
        .fetch(&request(&server.url("/next.html"), ResourceKind::Document))
        .unwrap();
    assert!(String::from_utf8_lossy(&page.body).contains("Next page"));
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kumo-net-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn download_saves_the_file_with_the_header_name_and_avoids_overwriting() {
    use kumo_net::protocol::DownloadState;
    use std::sync::atomic::AtomicBool;

    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    let dir = temp_dir("save");
    let run = |path: &str| {
        let mut events = Vec::new();
        service.download(
            1,
            &server.url(path),
            &dir,
            None,
            &AtomicBool::new(false),
            &mut |e| events.push(e),
        );
        events
    };
    let first = run("/file.zip");
    let last = first.last().unwrap();
    let DownloadState::Done(path) = &last.state else {
        panic!("{first:?}");
    };
    assert_eq!(std::fs::read(path).unwrap(), b"PKzipbody");
    assert_eq!(last.name, "my file.zip");
    assert_eq!((last.received, last.total), (9, Some(9)));
    let second = run("/file.zip");
    assert_eq!(
        second.last().unwrap().name,
        "my file (1).zip",
        "上書きしない"
    );
    let by_url = run("/data.bin");
    assert_eq!(
        by_url.last().unwrap().name,
        "data.bin",
        "ヘッダーがなければ URL の名前"
    );
    let missing = run("/nothing.zip");
    assert!(
        matches!(&missing.last().unwrap().state, DownloadState::Failed(m) if m.contains("404"))
    );
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        3,
        "失敗や完了のあとに .part が残らない"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn download_can_be_cancelled_and_leaves_nothing() {
    use kumo_net::protocol::DownloadState;
    use std::sync::atomic::AtomicBool;

    let server = TestServer::start();
    let service = NetworkService::new(&NetworkConfig::default());
    let dir = temp_dir("cancel");
    let cancel = AtomicBool::new(true);
    let mut events = Vec::new();
    service.download(2, &server.url("/file.zip"), &dir, None, &cancel, &mut |e| {
        events.push(e);
    });
    assert_eq!(events.last().unwrap().state, DownloadState::Cancelled);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}
