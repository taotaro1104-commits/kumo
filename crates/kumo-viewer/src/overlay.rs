//! コンテンツの上に重ねる表示：リンク先の吹き出し（左下）と、カーソルの形。

use std::ops::Range;

use kumo_browser::{Browser, FrameView, Pointer};
use kumo_paint::{CpuRasterizer, NoImageProvider, RectF, render_region};
use winit::window::CursorIcon;

pub fn cursor_icon(pointer: Pointer) -> CursorIcon {
    match pointer {
        Pointer::Default => CursorIcon::Default,
        Pointer::Link => CursorIcon::Pointer,
        Pointer::Text => CursorIcon::Text,
    }
}

/// 吹き出しを描き直す。前に描いた行は最後のフレームで元に戻す。
/// `out` はコンテンツ領域（幅 `w`、`rows` 行）。描き直した行（コンテンツ領域の座標）を返す。
pub fn draw_status(
    browser: &mut Browser,
    raster: &mut CpuRasterizer,
    out: &mut [u32],
    (w, rows): (usize, usize),
    shown: &mut Option<Range<usize>>,
) -> Vec<Range<usize>> {
    let mut damage = Vec::new();
    if let Some(previous) = shown.take() {
        if let Some(frame) = browser.last_frame() {
            restore_rows(&frame, out, w, previous.clone());
        }
        damage.push(previous);
    }
    let Some(bubble) = browser.status_bubble() else {
        return damage;
    };
    let (bw, bh) = (bubble.width.min(w as u32), bubble.height.min(rows as u32));
    let region = RectF::new(0.0, 0.0, bw as f32, bh as f32);
    let Some(pixmap) = render_region(&bubble.page.list, raster, &mut NoImageProvider, region)
    else {
        return damage;
    };
    let top = rows - bh as usize;
    for (y, row) in pixmap.data().chunks_exact(bw as usize * 4).enumerate() {
        let dst = &mut out[(top + y) * w..(top + y) * w + bw as usize];
        for (px, rgba) in dst.iter_mut().zip(row.as_chunks::<4>().0) {
            *px = (u32::from(rgba[0]) << 16) | (u32::from(rgba[1]) << 8) | u32::from(rgba[2]);
        }
    }
    damage.push(top..rows);
    *shown = Some(top..rows);
    damage
}

/// フレームの行をバッファへ写し戻す（大きさが違えば重なる部分だけ）。
fn restore_rows(frame: &FrameView<'_>, out: &mut [u32], w: usize, rows: Range<usize>) {
    let fw = frame.width as usize;
    let cols = fw.min(w);
    for y in rows.start..rows.end.min(frame.height as usize) {
        if (y + 1) * w > out.len() {
            break;
        }
        out[y * w..y * w + cols].copy_from_slice(&frame.pixels[y * fw..y * fw + cols]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointers_map_to_cursor_icons() {
        assert_eq!(cursor_icon(Pointer::Link), CursorIcon::Pointer);
        assert_eq!(cursor_icon(Pointer::Text), CursorIcon::Text);
        assert_eq!(cursor_icon(Pointer::Default), CursorIcon::Default);
    }

    #[test]
    fn restores_only_overlapping_rows() {
        let pixels: Vec<u32> = (0..6).collect();
        let frame = FrameView {
            pixels: &pixels,
            width: 3,
            height: 2,
            damage: (0, 2),
        };
        let mut out = vec![9u32; 4 * 3];
        restore_rows(&frame, &mut out, 4, 1..3);
        assert_eq!(&out[..4], &[9, 9, 9, 9], "範囲外の行はそのまま");
        assert_eq!(&out[4..8], &[3, 4, 5, 9], "幅が違えば重なる部分だけ");
        assert_eq!(&out[8..], &[9, 9, 9, 9], "フレームにない行は触らない");
    }

    #[test]
    fn hovered_link_bubble_is_drawn_bottom_left_and_removed() {
        use std::sync::Arc;
        use std::time::Duration;

        use kumo_net::service::{NetworkConfig, NetworkService};

        let dir = std::env::temp_dir().join(format!("kumo-overlay-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.html"),
            "<body style='margin:0;font-size:20px;background:#fff'><a href='https://example.com/docs/page'>link</a></body>",
        )
        .unwrap();
        let fetcher = Arc::new(NetworkService::new(&NetworkConfig::default()));
        let mut browser = Browser::in_process(fetcher, None, (600.0, 300.0));
        browser.open_tab(
            Some(url::Url::from_file_path(dir.join("a.html")).unwrap()),
            true,
        );
        assert!(browser.settle(Duration::from_secs(20)));
        browser.hover_content(5.0, 10.0);
        assert!(browser.settle(Duration::from_secs(20)));
        let (w, rows) = (600usize, (300.0 - kumo_browser::CHROME_HEIGHT) as usize);
        let mut out = browser
            .wait_frame(Duration::from_secs(20))
            .unwrap()
            .pixels
            .to_vec();
        let mut raster = CpuRasterizer::default();
        let mut shown = None;
        let damage = draw_status(&mut browser, &mut raster, &mut out, (w, rows), &mut shown);
        assert_eq!(damage, vec![rows - 24..rows], "左下の 24 行");
        assert_ne!(out[(rows - 12) * w + 4], 0x00ff_ffff, "吹き出しの背景");
        assert_eq!(
            out[(rows - 12) * w + 590],
            0x00ff_ffff,
            "右側はページのまま"
        );
        if let Some(path) = std::env::var_os("KUMO_OVERLAY_PNG") {
            let mut pm = tiny_skia::Pixmap::new(w as u32, rows as u32).unwrap();
            for (d, &px) in pm.data_mut().as_chunks_mut::<4>().0.iter_mut().zip(&out) {
                *d = [(px >> 16) as u8, (px >> 8) as u8, px as u8, 255];
            }
            pm.save_png(path).unwrap();
        }
        browser.hover_content(500.0, 200.0);
        assert!(browser.settle(Duration::from_secs(20)));
        let damage = draw_status(&mut browser, &mut raster, &mut out, (w, rows), &mut shown);
        assert_eq!(damage, vec![rows - 24..rows], "消すときも前の行を描き直す");
        assert_eq!(out[(rows - 12) * w + 4], 0x00ff_ffff, "ページに戻る");
        assert!(shown.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
