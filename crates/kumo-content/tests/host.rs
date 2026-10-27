mod common;

use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::Site;
use kumo_content::protocol::{FromContent, ToContent};
use kumo_content::{CONTENT_PROCESS_ARG, ContentClient, FrameReader, NETWORK_ARG, network_socket};
use kumo_net::process::{Fetcher, NetworkClient, config_args};
use kumo_net::service::{NetworkConfig, NetworkService};

/// `until` を満たす通知が来るまで受け取る。
fn wait_for(client: &ContentClient, until: impl Fn(&FromContent) -> bool) -> Vec<FromContent> {
    let mut seen = Vec::new();
    loop {
        let message = client
            .recv_timeout(Duration::from_secs(20))
            .expect("Content から通知が届く");
        let done = until(&message);
        seen.push(message);
        if done {
            return seen;
        }
    }
}

fn load_and_render(client: &ContentClient, site: &Site) -> Vec<FromContent> {
    assert!(client.send(ToContent::Viewport {
        width: 120,
        height: 80
    }));
    client.send(ToContent::Show { tab: 7 });
    client.send(ToContent::Load {
        tab: 7,
        nav: 1,
        url: site.url("red.html").to_string(),
        post_body: None,
    });
    let mut seen = wait_for(client, |m| matches!(m, FromContent::Loaded { .. }));
    // 読み込み前の白いフレームを写し終えたことにして、読み込み後のフレームを待つ
    client.send(ToContent::FrameDone);
    client.send(ToContent::Sync { id: 1 });
    seen.extend(wait_for(client, |m| {
        matches!(m, FromContent::Synced { id: 1 })
    }));
    seen
}

fn last_frame_pixel(seen: &[FromContent]) -> u32 {
    let (path, w, h) = seen
        .iter()
        .rev()
        .find_map(|m| match m {
            FromContent::Frame {
                path,
                width,
                height,
                ..
            } => Some((path.clone(), *width, *height)),
            _ => None,
        })
        .expect("フレームが届く");
    let mut reader = FrameReader::default();
    reader.attach(&path, w, h).unwrap();
    reader.pixels().unwrap()[(h as usize / 2) * w as usize + 5]
}

#[test]
fn thread_host_loads_without_blocking_and_notifies() {
    let site = Site::new("thread");
    let fetcher = Arc::new(NetworkService::new(&NetworkConfig::default()));
    let client = ContentClient::spawn_thread(fetcher);
    let woken = Arc::new(AtomicUsize::new(0));
    let counter = woken.clone();
    client.set_notify(Arc::new(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    }));
    let seen = load_and_render(&client, &site);
    assert!(
        seen.iter()
            .any(|m| matches!(m, FromContent::Loaded { title, .. } if title == "Red"))
    );
    assert_eq!(last_frame_pixel(&seen), 0x00ff_0000);
    assert!(
        woken.load(Ordering::Relaxed) >= seen.len(),
        "通知のたびに起こす"
    );
    client.send(ToContent::Stats { id: 2 });
    let stats = wait_for(&client, |m| matches!(m, FromContent::Stats { .. }));
    assert!(matches!(stats.last(), Some(FromContent::Stats { stats, .. }) if stats.tabs == 1));
    assert_eq!(client.process_id(), None);
}

#[test]
fn content_process_fetches_through_a_shared_network_process() {
    let site = Site::new("process");
    let mut network = Command::new(env!("CARGO_BIN_EXE_kumo-content"));
    network.args(config_args(&NetworkConfig::default()));
    let network = NetworkClient::spawn(network).expect("Network プロセス");
    let mut command = Command::new(env!("CARGO_BIN_EXE_kumo-content"));
    command.args([NETWORK_ARG, network.socket_name()]);
    let client = ContentClient::spawn_process(command).expect("Content プロセス");
    assert!(
        client
            .process_id()
            .is_some_and(|pid| pid != std::process::id())
    );
    let seen = load_and_render(&client, &site);
    assert_eq!(last_frame_pixel(&seen), 0x00ff_0000);
    let stats = network.stats().unwrap();
    assert_eq!(
        stats.requests, 2,
        "Content の取得（文書と CSS）は共有の Network プロセスを通る"
    );
    drop(client);
    assert!(
        network.is_connected(),
        "Content が終わっても Network は動き続ける"
    );
}

#[test]
fn reads_network_socket_argument() {
    let args: Vec<String> = [CONTENT_PROCESS_ARG, NETWORK_ARG, "s"]
        .map(String::from)
        .to_vec();
    assert_eq!(network_socket(&args), Some("s"));
    assert_eq!(network_socket(&args[..2]), None);
}
