//! Content プロセス単体の実行ファイル（テストや別配置用）。
//! 使い方: kumo-content [--network SOCKET]
//! `--network` があればその Network プロセスへ接続し、なければプロセス内で取得する。
//! `--listen NAME` を付けると Network プロセスとして動く（ビューアと同じく1つの実行ファイルで両方の役を持つ）。
#![forbid(unsafe_code)]

use std::sync::Arc;

use kumo_net::process::{Fetcher, NetworkClient, listen_name, parse_config, serve_listener};
use kumo_net::service::{NetworkConfig, NetworkService};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(name) = listen_name(&args) {
        let config = parse_config(args.iter().cloned());
        if let Err(e) = serve_listener(NetworkService::new(&config), name, || 0) {
            eprintln!("[network] {e}");
            std::process::exit(1);
        }
        return;
    }
    let fetcher: Arc<dyn Fetcher> = match kumo_content::network_socket(&args) {
        Some(name) => match NetworkClient::connect(name) {
            Ok(client) => Arc::new(client),
            Err(e) => {
                eprintln!("[content] Network プロセスに接続できません: {e}");
                std::process::exit(1);
            }
        },
        None => Arc::new(NetworkService::new(&NetworkConfig::default())),
    };
    if let Err(e) = kumo_content::serve_stdio(fetcher, || 0) {
        eprintln!("[content] {e}");
        std::process::exit(1);
    }
}
