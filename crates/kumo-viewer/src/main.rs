//! Kumo ブラウザ。
//!
//! 使い方:
//!   kumo-viewer [URL|ファイル ...] [--width N] [--height N] [--scroll Y]
//!               [--headless out.png] [--mem-report] [--in-process] [--no-disk-cache]
//!               [--filter-list FILE ...]
//!
//! 同じ実行ファイルを `--network-process`・`--content-process` 付きで子プロセスとして起動し、
//! Network プロセス・Content プロセスにする（[`processes`]）。
#![forbid(unsafe_code)]

mod args;
mod headless;
mod input;
mod overlay;
mod processes;
mod report;
mod window;

use std::sync::Arc;
use std::time::Duration;

use kumo_browser::{Browser, HistoryDb, Policy, parse_input};
use kumo_content::CONTENT_PROCESS_ARG;
use kumo_mem::TaggedAllocator;
use kumo_net::process::{Fetcher, NetworkClient, listen_name, parse_config, serve_listener};
use kumo_net::service::NetworkService;

use crate::args::Args;
use crate::processes::{NETWORK_PROCESS_ARG, data_dir};

#[global_allocator]
static ALLOC: TaggedAllocator = TaggedAllocator;

/// このプロセスのメモリ使用量（Network・Content プロセスが統計として返す）。
fn private_bytes() -> u64 {
    if std::env::var_os("KUMO_ALLOC_STATS").is_some() {
        eprintln!("[child] アロケータ統計:");
        kumo_mem::print_allocator_stats();
    }
    kumo_mem::process_memory().map_or(0, |p| p.primary as u64)
}

fn open_history(args: &Args) -> Option<HistoryDb> {
    if args.no_disk_cache {
        return None;
    }
    let dir = data_dir()?;
    match HistoryDb::open(&dir.join("history.redb")) {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("[history] 履歴なしで起動します: {e}");
            None
        }
    }
}

/// ヘッドレスの計測用：`--freeze-after` / `--discard-after` が指定されていたら、その時間が経ったことにして
/// 背景のタブを降格し、反映を待つ（凍結・破棄したときのメモリを測れる）。
fn fast_forward(browser: &mut Browser, args: &Args) {
    let skip = args.discard_after_secs.or(args.freeze_after_secs);
    let Some(secs) = skip else { return };
    let later = std::time::Instant::now() + Duration::from_secs(secs + 1);
    browser.govern(0, kumo_mem::Pressure::None, later);
    browser.settle(LOAD_TIMEOUT);
}

/// ヘッドレスの `--wait`：タイマー（アニメーションなど）を動かしながら待ってから描画する。
fn wait_for_timers(browser: &mut Browser, millis: u64) {
    let deadline = std::time::Instant::now() + Duration::from_millis(millis);
    while std::time::Instant::now() < deadline {
        browser.pump();
        browser.settle(Duration::from_millis(500));
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// 既定の方針に、引数で指定された分を上書きする。
fn memory_policy(args: &Args) -> Policy {
    let mut policy = Policy::default();
    if let Some(mb) = args.memory_budget_mb {
        policy.budget = mb * 1024 * 1024;
    }
    if let Some(s) = args.freeze_after_secs {
        policy.freeze_after = Duration::from_secs(s);
    }
    if let Some(s) = args.discard_after_secs {
        policy.discard_after = Duration::from_secs(s);
    }
    policy
}

/// 起動時に開いたタブの読み込みを待つ上限（ヘッドレス）。
const LOAD_TIMEOUT: Duration = Duration::from_secs(120);

fn run(args: Args) -> Result<(), String> {
    let processes = processes::start(&args)?;
    let mut browser = Browser::new(
        processes.content,
        open_history(&args),
        (args.width as f32, args.height as f32),
    );
    browser.set_content_restart(processes.restart);
    browser.set_javascript(!args.no_js);
    if let Some(level) = args.load_level {
        browser.set_load_level(level, false);
    }
    browser.set_memory_policy(memory_policy(&args));
    let started = std::time::Instant::now();
    if args.targets.is_empty() {
        browser.open_start_tab();
    }
    for (i, target) in args.targets.iter().enumerate() {
        let url = parse_input(target).ok_or_else(|| format!("開けません: {target}"))?;
        browser.open_tab(Some(url), i == 0);
    }
    match &args.headless {
        Some(out) => {
            if !browser.settle(LOAD_TIMEOUT) {
                return Err("読み込みが時間内に終わりません".into());
            }
            eprintln!(
                "[load] {} タブを {:.0}ms で読み込み",
                args.targets.len(),
                started.elapsed().as_secs_f64() * 1000.0
            );
            fast_forward(&mut browser, &args);
            wait_for_timers(&mut browser, args.wait_ms);
            for code in &args.eval {
                match browser.eval(code, Duration::from_secs(10)) {
                    Ok(value) => println!("[eval] {value}"),
                    Err(e) => println!("[eval error] {e}"),
                }
                // 続きの式が、この式で始めたタイマーの結果を見られるように少し待つ
                wait_for_timers(&mut browser, 700);
            }
            headless::render(&mut browser, &*processes.network, &args, out)
        }
        None => window::run(browser, processes.network, args.mem_report),
    }
}

/// 子プロセスの役割で起動されたなら、その役割を果たして `true`。
fn run_child_role(raw: &[String]) -> bool {
    match raw.first().map(String::as_str) {
        Some(NETWORK_PROCESS_ARG) => {
            let config = parse_config(raw.iter().skip(1).cloned());
            let name = listen_name(raw).unwrap_or_default().to_owned();
            if let Err(e) = serve_listener(NetworkService::new(&config), &name, private_bytes) {
                eprintln!("[network] {e}");
                std::process::exit(1);
            }
            true
        }
        Some(CONTENT_PROCESS_ARG) => {
            let fetcher: Arc<dyn Fetcher> =
                match kumo_content::network_socket(raw).map(NetworkClient::connect) {
                    Some(Ok(client)) => Arc::new(client),
                    Some(Err(e)) => {
                        eprintln!("[content] Network プロセスに接続できません: {e}");
                        std::process::exit(1);
                    }
                    None => {
                        eprintln!("[content] --network SOCKET が必要です");
                        std::process::exit(2);
                    }
                };
            if let Err(e) = kumo_content::serve_stdio(fetcher, private_bytes) {
                eprintln!("[content] {e}");
                std::process::exit(1);
            }
            true
        }
        _ => false,
    }
}

fn main() {
    kumo_mem::configure_for_low_memory();
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if run_child_role(&raw) {
        return;
    }
    let args = match args::parse(raw.into_iter()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = run(args) {
        eprintln!("エラー: {e}");
        std::process::exit(1);
    }
}
