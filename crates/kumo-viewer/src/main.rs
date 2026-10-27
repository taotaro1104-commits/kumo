//! Kumo Phase 0 静的ビューア。
//!
//! 使い方:
//!   kumo-viewer <file|url> [--width N] [--height N] [--scroll Y]
//!                [--headless out.png] [--mem-report]
#![forbid(unsafe_code)]

mod loader;
mod page;
mod window;

use kumo_mem::{Subsystem, TaggedAllocator};
use kumo_paint::{CpuRasterizer, RectF, render_region};

use crate::page::Page;

#[global_allocator]
static ALLOC: TaggedAllocator = TaggedAllocator;

#[derive(Debug)]
struct Args {
    target: String,
    width: u32,
    height: u32,
    scroll: f32,
    headless: Option<String>,
    mem_report: bool,
}

const USAGE: &str = "usage: kumo-viewer <file|url> [--width N] [--height N] [--scroll Y] [--headless out.png] [--mem-report]";

fn parse_args(raw: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut args = Args {
        target: String::new(),
        width: 1280,
        height: 800,
        scroll: 0.0,
        headless: None,
        mem_report: false,
    };
    let mut raw = raw.peekable();
    while let Some(arg) = raw.next() {
        let mut value = |name: &str| raw.next().ok_or_else(|| format!("{name} には値が必要です"));
        match arg.as_str() {
            "--width" => args.width = parse_dim(&value("--width")?)?,
            "--height" => args.height = parse_dim(&value("--height")?)?,
            "--scroll" => {
                args.scroll = value("--scroll")?
                    .parse()
                    .map_err(|_| "--scroll は数値".to_owned())?
            }
            "--headless" => args.headless = Some(value("--headless")?),
            "--mem-report" => args.mem_report = true,
            "-h" | "--help" => return Err(USAGE.to_owned()),
            other if other.starts_with("--") => {
                return Err(format!("不明なオプション: {other}\n{USAGE}"));
            }
            other => args.target = other.to_owned(),
        }
    }
    if args.target.is_empty() {
        return Err(USAGE.to_owned());
    }
    Ok(args)
}

fn parse_dim(s: &str) -> Result<u32, String> {
    match s.parse::<u32>() {
        Ok(v) if (100..=8192).contains(&v) => Ok(v),
        _ => Err(format!("幅・高さは 100〜8192 の整数: {s}")),
    }
}

pub(crate) fn print_memory_report(page: &Page, raster: &CpuRasterizer, frame_bytes: usize) {
    let stats = page.stats();
    eprintln!(
        "[stats] dom={} rules={} unique_styles={} boxes={} display_items={}",
        stats.dom_nodes, stats.rules, stats.unique_styles, stats.boxes, stats.display_items
    );
    eprintln!(
        "[caches] raster={:.2}MB glyphs={:.2}MB images(decoded)={:.2}MB images(compressed)={:.2}MB",
        mb(frame_bytes),
        mb(raster.glyphs.bytes()),
        mb(page.images.decoded_bytes()),
        mb(page.images.compressed_bytes())
    );
    eprint!(
        "{}",
        kumo_mem::format_report(&kumo_mem::report(), kumo_mem::process_memory())
    );
    if std::env::var_os("KUMO_ALLOC_STATS").is_some() {
        kumo_mem::print_allocator_stats();
    }
}

fn mb(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn run(args: Args) -> Result<(), String> {
    let started = std::time::Instant::now();
    let mut page = Page::load(&args.target, args.width as f32, args.height as f32)?;
    eprintln!(
        "[load] {} を {:.0}ms で読み込み（文書の高さ {:.0}px）",
        args.target,
        started.elapsed().as_secs_f64() * 1000.0,
        page.document_height()
    );
    let Some(out) = &args.headless else {
        let viewer = window::Viewer::new(page, args.target.clone(), args.mem_report);
        return window::run(viewer);
    };
    let mut raster = CpuRasterizer::default();
    let region = RectF::new(0.0, args.scroll, args.width as f32, args.height as f32);
    let render_started = std::time::Instant::now();
    let pixmap = {
        let _tag = kumo_mem::scope(Subsystem::Paint);
        render_region(&page.list, &mut raster, &mut page.images, region)
            .ok_or("Pixmap を確保できません")?
    };
    eprintln!(
        "[render] {}x{} を {:.1}ms で描画",
        args.width,
        args.height,
        render_started.elapsed().as_secs_f64() * 1000.0
    );
    pixmap.save_png(out).map_err(|e| format!("{out}: {e}"))?;
    drop(pixmap);
    kumo_mem::release_free_memory();
    eprintln!("[headless] {out} に保存");
    if args.mem_report {
        print_memory_report(&page, &raster, 0);
    }
    Ok(())
}

fn main() {
    kumo_mem::configure_for_low_memory();
    let args = match parse_args(std::env::args().skip(1)) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Args, String> {
        parse_args(list.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_options() {
        let a = args(&[
            "page.html",
            "--width",
            "1920",
            "--headless",
            "o.png",
            "--mem-report",
        ])
        .unwrap();
        assert_eq!((a.width, a.height), (1920, 800));
        assert_eq!(a.headless.as_deref(), Some("o.png"));
        assert!(a.mem_report);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(args(&[]).is_err());
        assert!(args(&["x", "--width", "5"]).is_err());
        assert!(args(&["x", "--bogus"]).is_err());
        assert!(args(&["x", "--width"]).is_err());
    }
}
