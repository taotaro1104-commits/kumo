//! 取得を重ねたときの Network プロセスのメモリを測る診断用プログラム。
//! cargo run -p kumo-net --example churn --release -- <URL> [<URL> ...]

use kumo_mem::{Subsystem, TaggedAllocator, scope};
use kumo_net::process::Fetcher;
use kumo_net::protocol::{FromNetwork, ResourceKind, write_message};
use kumo_net::service::{NetworkConfig, NetworkService};

#[global_allocator]
static ALLOC: TaggedAllocator = TaggedAllocator;

fn mb(bytes: f64) -> f64 {
    bytes / 1048576.0
}

fn main() {
    kumo_mem::configure_for_low_memory();
    let commit = || mb(kumo_mem::process_memory().map_or(0.0, |p| p.primary as f64));
    let heap = || {
        let total: isize = kumo_mem::report().iter().map(|u| u.current).sum();
        mb(total as f64)
    };
    let service = NetworkService::new(&NetworkConfig::default());
    eprintln!("start: commit={:.2}MB heap={:.2}MB", commit(), heap());
    for url in std::env::args().skip(1) {
        let response = {
            let _t = scope(Subsystem::Network);
            Fetcher::fetch(&service, &url, ResourceKind::Document, None)
        };
        if let Ok(response) = response {
            let mut sink = std::io::sink();
            write_message(&mut sink, &FromNetwork::Response(response)).unwrap();
        }
        eprintln!(
            "{url}: commit={:.2}MB heap={:.2}MB memcache={:.2}MB",
            commit(),
            heap(),
            mb(service.stats().memory_cache_bytes as f64)
        );
    }
    kumo_mem::release_free_memory();
    eprintln!(
        "end (collected): commit={:.2}MB heap={:.2}MB",
        commit(),
        heap()
    );
}
