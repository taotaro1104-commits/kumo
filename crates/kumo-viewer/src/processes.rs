//! プロセス構成：Browser（このプロセス）・Content・Network。
//! 同じ実行ファイルを役割の引数付きで子プロセスとして起動する。`--in-process` ならどちらもスレッドで動かす。

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use kumo_browser::ContentFactory;
use kumo_content::{CONTENT_PROCESS_ARG, ContentClient, NETWORK_ARG};
use kumo_net::process::{Fetcher, NetworkClient, config_args};
use kumo_net::service::{NetworkConfig, NetworkService};

use crate::args::Args;

/// Network プロセスとして起動するときの最初の引数。
pub const NETWORK_PROCESS_ARG: &str = "--network-process";

/// キャッシュ・履歴の置き場（Windows: %LOCALAPPDATA%\Kumo、その他: $XDG_CACHE_HOME/kumo か ~/.cache/kumo）。
pub fn data_dir() -> Option<PathBuf> {
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        return Some(PathBuf::from(local).join("Kumo"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(xdg).join("kumo"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache").join("kumo"))
}

fn network_config(args: &Args) -> NetworkConfig {
    NetworkConfig {
        cache_dir: data_dir()
            .filter(|_| !args.no_disk_cache)
            .map(|d| d.join("cache")),
        filter_lists: args.filter_lists.clone(),
    }
}

fn own_exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("実行ファイルの場所が分かりません: {e}"))
}

/// 動いているプロセスへの口。
pub struct Processes {
    /// Network（統計の問い合わせ用）。プロセス内で動かしているなら [`NetworkService`]
    pub network: Arc<dyn Fetcher>,
    pub content: ContentClient,
    /// Content が落ちたときに作り直す関数
    pub restart: ContentFactory,
}

/// Network と Content を起動する。Content は Network のソケットへ自分で接続する。
pub fn start(args: &Args) -> Result<Processes, String> {
    let config = network_config(args);
    if args.in_process {
        let network: Arc<dyn Fetcher> = Arc::new(NetworkService::new(&config));
        let content = ContentClient::spawn_thread(network.clone());
        let again = network.clone();
        let restart: ContentFactory =
            Box::new(move || Ok(ContentClient::spawn_thread(again.clone())));
        return Ok(Processes {
            network,
            content,
            restart,
        });
    }
    let mut command = Command::new(own_exe()?);
    command.arg(NETWORK_PROCESS_ARG).args(config_args(&config));
    let network = NetworkClient::spawn(command)
        .map_err(|e| format!("Network プロセスを起動できません: {e}"))?;
    let exe = own_exe()?;
    let socket = network.socket_name().to_owned();
    let spawn_content = move || {
        let mut command = Command::new(&exe);
        command
            .arg(CONTENT_PROCESS_ARG)
            .args([NETWORK_ARG, socket.as_str()]);
        ContentClient::spawn_process(command)
    };
    let content = spawn_content().map_err(|e| format!("Content プロセスを起動できません: {e}"))?;
    Ok(Processes {
        network: Arc::new(network),
        content,
        restart: Box::new(spawn_content),
    })
}
