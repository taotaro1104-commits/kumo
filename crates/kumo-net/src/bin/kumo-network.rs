//! Network プロセス単体の実行ファイル（テストや別配置用）。
//! 使い方: kumo-network --listen NAME [--cache-dir DIR] [--filter-list FILE]...
#![forbid(unsafe_code)]

use kumo_net::process::{listen_name, parse_config, serve_listener};
use kumo_net::service::NetworkService;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(name) = listen_name(&args) else {
        eprintln!("[network] --listen NAME が必要です");
        std::process::exit(2);
    };
    let config = parse_config(args.iter().cloned());
    if let Err(e) = serve_listener(NetworkService::new(&config), name, || 0) {
        eprintln!("[network] {e}");
        std::process::exit(1);
    }
}
