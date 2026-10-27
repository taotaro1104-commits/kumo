//! メモリの内訳（about:memory 相当）を標準エラーへ出す。Browser・Content・Network の3プロセス分。

use std::time::Duration;

use kumo_browser::Browser;
use kumo_net::process::Fetcher;
use kumo_paint::CpuRasterizer;

const STATS_TIMEOUT: Duration = Duration::from_secs(10);

fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// `frame_bytes` は Browser 側の描画用バッファ（UI の帯）。
pub fn print(
    browser: &mut Browser,
    network: &dyn Fetcher,
    raster: &CpuRasterizer,
    frame_bytes: usize,
) {
    let browser_bytes = kumo_mem::process_memory().map_or(0, |p| p.primary as u64);
    eprint!(
        "{}",
        kumo_mem::format_report(&kumo_mem::report(), kumo_mem::process_memory())
    );
    eprintln!(
        "[browser] {} タブ、UI 描画 {:.2}MB、UI グリフ {:.2}MB",
        browser.tabs().len(),
        mb(frame_bytes as u64),
        mb(raster.glyphs.bytes() as u64)
    );
    let content = browser.content_stats(STATS_TIMEOUT);
    match &content {
        Some(c) => eprintln!(
            "[content] {} タブ（DOM {} ノード、描画命令 {} 件） frame={:.2}MB glyphs={:.2}MB images(decoded)={:.2}MB images(compressed)={:.2}MB",
            c.tabs,
            c.dom_nodes,
            c.display_items,
            mb(c.frame_bytes),
            mb(c.glyph_bytes),
            mb(c.decoded_image_bytes),
            mb(c.compressed_image_bytes)
        ),
        None => eprintln!("[content] 統計を取得できません"),
    }
    let net = network.stats();
    if let Some(net) = &net {
        eprintln!(
            "[network] requests={} cache_hits={} revalidated={} blocked={} memcache={:.2}MB diskcache={:.2}MB",
            net.requests,
            net.cache_hits,
            net.revalidated,
            net.blocked,
            mb(net.memory_cache_bytes),
            mb(net.disk_cache_bytes)
        );
    } else {
        eprintln!("[network] 統計を取得できません");
    }
    // プロセス内で動かしているときは子プロセス分が 0 になる（Browser の値に含まれる）
    let content_bytes = content.map_or(0, |c| c.private_bytes);
    let network_bytes = net.map_or(0, |n| n.private_bytes);
    eprintln!(
        "[total] browser {:.2}MB + content {:.2}MB + network {:.2}MB = {:.2}MB",
        mb(browser_bytes),
        mb(content_bytes),
        mb(network_bytes),
        mb(browser_bytes + content_bytes + network_bytes)
    );
    if std::env::var_os("KUMO_ALLOC_STATS").is_some() {
        kumo_mem::print_allocator_stats();
    }
}
