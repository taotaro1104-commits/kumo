//! ヘッドレス出力：UI と表示中タブのコンテンツ（Content が描いたフレーム）を1枚の PNG にする。

use std::time::Duration;

use kumo_browser::{Browser, CHROME_HEIGHT, FrameView};
use kumo_net::process::Fetcher;
use kumo_paint::{CpuRasterizer, NoImageProvider, RectF, render_region};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::args::Args;

const FRAME_TIMEOUT: Duration = Duration::from_secs(60);

pub fn render(
    browser: &mut Browser,
    network: &dyn Fetcher,
    args: &Args,
    out: &str,
) -> Result<(), String> {
    let width = args.width as f32;
    let mut raster = CpuRasterizer::default();
    let started = std::time::Instant::now();
    if args.scroll != 0.0 {
        browser.scroll_by(args.scroll);
    }
    let content = {
        let frame = browser
            .wait_frame(FRAME_TIMEOUT)
            .ok_or("コンテンツのフレームが届きません")?;
        frame_to_pixmap(&frame).ok_or("Pixmap を確保できません")?
    };
    let chrome = render_region(
        &browser.chrome_page().list,
        &mut raster,
        &mut NoImageProvider,
        RectF::new(0.0, 0.0, width, CHROME_HEIGHT),
    )
    .ok_or("UI を描けません")?;
    let mut image = Pixmap::new(args.width, args.height).ok_or("Pixmap を確保できません")?;
    let paint = PixmapPaint::default();
    image.draw_pixmap(0, 0, chrome.as_ref(), &paint, Transform::identity(), None);
    image.draw_pixmap(
        0,
        CHROME_HEIGHT as i32,
        content.as_ref(),
        &paint,
        Transform::identity(),
        None,
    );
    eprintln!(
        "[render] {}x{} を {:.1}ms で描画",
        args.width,
        args.height,
        started.elapsed().as_secs_f64() * 1000.0
    );
    image.save_png(out).map_err(|e| format!("{out}: {e}"))?;
    drop((image, chrome, content));
    kumo_mem::release_free_memory();
    eprintln!("[headless] {out} に保存");
    if args.mem_report {
        crate::report::print(browser, network, &raster, 0);
    }
    Ok(())
}

/// 0RGB の画素を不透明な RGBA の Pixmap にする。
fn frame_to_pixmap(frame: &FrameView<'_>) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(frame.width, frame.height)?;
    for (dst, &px) in pixmap
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(frame.pixels)
    {
        *dst = [(px >> 16) as u8, (px >> 8) as u8, px as u8, 255];
    }
    Some(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use kumo_net::service::{NetworkConfig, NetworkService};

    #[test]
    fn renders_chrome_and_content_to_png() {
        let dir = std::env::temp_dir().join(format!("kumo-headless-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let page = dir.join("p.html");
        std::fs::write(&page, "<body style='background:#f00;margin:0'></body>").unwrap();
        let fetcher = Arc::new(NetworkService::new(&NetworkConfig::default()));
        let mut browser = Browser::in_process(fetcher.clone(), None, (400.0, 300.0));
        browser.open_tab(Some(url::Url::from_file_path(&page).unwrap()), true);
        assert!(browser.settle(Duration::from_secs(20)));
        let out = dir.join("out.png");
        let args = crate::args::parse(
            ["--width", "400", "--height", "300", "--mem-report"]
                .iter()
                .map(|s| s.to_string()),
        )
        .unwrap();
        render(&mut browser, &*fetcher, &args, out.to_str().unwrap()).unwrap();
        let png = Pixmap::load_png(&out).unwrap();
        assert_eq!((png.width(), png.height()), (400, 300));
        let content = png.pixel(200, 250).unwrap();
        assert_eq!(
            (content.red(), content.green(), content.blue()),
            (255, 0, 0)
        );
        let chrome = png.pixel(200, 10).unwrap();
        assert_ne!(
            (chrome.red(), chrome.green(), chrome.blue()),
            (255, 0, 0),
            "上部は UI"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
