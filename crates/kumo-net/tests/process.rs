mod common;

use std::process::Command;

use common::TestServer;
use kumo_net::process::{Fetcher, NetworkClient, config_args};
use kumo_net::protocol::{FetchError, ResourceKind};
use kumo_net::service::NetworkConfig;

fn spawn(config: &NetworkConfig) -> NetworkClient {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kumo-network"));
    command.args(config_args(config));
    NetworkClient::spawn(command).expect("Network プロセスを起動できる")
}

#[test]
fn fetches_through_a_real_child_process() {
    let server = TestServer::start();
    let client = spawn(&NetworkConfig::default());
    assert!(client.process_id() != std::process::id(), "別プロセス");
    let page = client
        .fetch(&server.url("/page.html"), ResourceKind::Document, None)
        .unwrap();
    assert!(String::from_utf8_lossy(&page.body).contains("Hello"));

    // 複数リソースは並行に取得され、入力と同じ順で返る
    let requests = vec![
        (server.url("/fresh.css"), ResourceKind::Stylesheet),
        (server.url("/missing"), ResourceKind::Image),
        (
            "https://www.google-analytics.com/analytics.js".to_owned(),
            ResourceKind::Other,
        ),
        (server.url("/etag.css"), ResourceKind::Stylesheet),
    ];
    let results = client.fetch_all(&requests, Some(&page.final_url));
    assert_eq!(results[0].as_ref().unwrap().body, b"a{color:red}");
    assert_eq!(results[1].as_ref().unwrap().status, 404);
    assert_eq!(results[2], Err(FetchError::Blocked));
    assert_eq!(results[3].as_ref().unwrap().body, b"b{color:blue}");

    let stats = client.stats().expect("統計が返る");
    assert_eq!(stats.requests, 5);
    assert_eq!(stats.blocked, 1);
}

#[test]
fn large_bodies_travel_through_the_shared_ring_and_are_released() {
    let server = TestServer::start();
    let client = spawn(&NetworkConfig::default());
    // 1MB × 12 = リング（4MB）の数倍：返却されなければ途中から入らなくなる（その場合も
    // パイプにフォールバックするので結果は正しいが、共有経由の件数で区別する）
    for (i, c) in "abcdefghijkl".chars().enumerate() {
        let url = server.url(&format!("/big?n={}&c={c}", 1 << 20));
        let body = client.fetch(&url, ResourceKind::Other, None).unwrap().body;
        assert_eq!(body.len(), 1 << 20, "{i} 件目の長さ");
        assert!(body.iter().all(|&b| b == c as u8), "{i} 件目の中身");
    }
    // 小さい本文はメッセージで直接届く
    let small = client
        .fetch(&server.url("/big?n=100&c=z"), ResourceKind::Other, None)
        .unwrap();
    assert_eq!(small.body, vec![b'z'; 100]);
    assert_eq!(client.shared_responses(), 12, "大きな本文は全部リング経由");
}

#[test]
fn a_second_process_can_connect_to_the_same_network_process() {
    let server = TestServer::start();
    let owner = spawn(&NetworkConfig::default());
    let other = NetworkClient::connect(owner.socket_name()).expect("2つ目の接続");
    let url = server.url("/big?n=40000&c=q");
    let body = other.fetch(&url, ResourceKind::Other, None).unwrap().body;
    assert_eq!(body, vec![b'q'; 40000]);
    assert_eq!(other.shared_responses(), 1, "接続ごとにリングがある");
    drop(other);
    // 2つ目が切れても Network プロセスは owner のために動き続ける
    let page = owner
        .fetch(&server.url("/page.html"), ResourceKind::Document, None)
        .unwrap();
    assert_eq!(page.status, 200);
    let stats = owner.stats().unwrap();
    assert_eq!(stats.requests, 2, "同じ Network プロセスが両方を処理した");
}

#[test]
fn reports_disconnection_when_the_child_dies() {
    let client = spawn(&NetworkConfig::default());
    let pid = client.process_id();
    // 子プロセスを外から終了させる
    let killed = if cfg!(windows) {
        Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
    } else {
        Command::new("kill").args(["-9", &pid.to_string()]).output()
    };
    assert!(killed.is_ok());
    std::thread::sleep(std::time::Duration::from_millis(500));
    let result = client.fetch("http://127.0.0.1:1/", ResourceKind::Document, None);
    assert_eq!(result, Err(FetchError::Disconnected));
    assert!(!client.is_connected());
}

#[test]
fn shutdown_on_drop_is_clean() {
    let client = spawn(&NetworkConfig::default());
    let started = std::time::Instant::now();
    drop(client);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "すぐに終了する"
    );
}
