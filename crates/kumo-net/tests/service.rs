mod common;

use common::TestServer;
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
