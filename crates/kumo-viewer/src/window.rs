//! winit + softbuffer のウィンドウ。上部に UI を描き、その下に Content が共有メモリに描いたフレームを写す。
//! Content からの通知はイベントループのユーザーイベントとして届き、そのたびに取り込む。

use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use kumo_browser::{Browser, CHROME_HEIGHT, FrameView, Pointer, Redraw};
use kumo_net::process::Fetcher;
use kumo_paint::{CpuRasterizer, NoImageProvider, RectF, ScrollRenderer};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::input::{self, LINE_SCROLL, Outcome};

/// メモリ管理の事務を呼ぶ間隔（ウィンドウのイベントがなくても起きる）。
const MEMORY_CHECK_INTERVAL: Duration = Duration::from_secs(2);

/// Content から通知が届いた。
#[derive(Clone, Copy, Debug)]
pub struct ContentWake;

struct Surface {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
}

pub struct Viewer {
    browser: Browser,
    network: Arc<dyn Fetcher>,
    raster: CpuRasterizer,
    chrome_renderer: ScrollRenderer,
    chrome_dirty: bool,
    surface: Option<Surface>,
    cursor: PhysicalPosition<f64>,
    mods: ModifiersState,
    mem_report: bool,
    reported: bool,
    /// リンク先の吹き出しを描いた行（コンテンツ領域の座標）
    status_rows: Option<std::ops::Range<usize>>,
    pointer: Pointer,
}

impl Viewer {
    fn request_redraw(&self) {
        if let Some(s) = &self.surface {
            s.window.request_redraw();
        }
    }

    fn apply(&mut self, outcome: Outcome, event_loop: &ActiveEventLoop) {
        self.mark(outcome.redraw);
        if outcome.report {
            self.report();
        }
        if outcome.exit {
            event_loop.exit();
        }
    }

    /// カーソルの形を、マウスの下にあるものに合わせる。
    fn update_cursor(&mut self) {
        let pointer = self.browser.pointer();
        if pointer != self.pointer
            && let Some(s) = &self.surface
        {
            s.window.set_cursor(crate::overlay::cursor_icon(pointer));
            self.pointer = pointer;
        }
    }

    fn mark(&mut self, redraw: Redraw) {
        self.chrome_dirty |= redraw.chrome;
        if redraw.chrome || redraw.content {
            self.request_redraw();
        }
    }

    fn report(&mut self) {
        let frame = self.chrome_renderer.bytes();
        crate::report::print(&mut self.browser, &*self.network, &self.raster, frame);
    }

    fn resize(&mut self, width: u32, height: u32) {
        let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else {
            return;
        };
        if let Some(s) = &mut self.surface
            && let Err(e) = s.surface.resize(w, h)
        {
            eprintln!("[window] サーフェスの再確保に失敗: {e}");
            return;
        }
        let redraw = self.browser.resize(width as f32, height as f32);
        self.mark(redraw);
    }

    fn redraw(&mut self) {
        let Some(size) = self.surface.as_ref().map(|s| s.window.inner_size()) else {
            return;
        };
        let (w, h) = (size.width as usize, size.height as usize);
        let chrome_rows = (CHROME_HEIGHT as usize).min(h);
        if w == 0 || h <= chrome_rows {
            return;
        }
        if self.chrome_dirty
            && let Some(s) = &self.surface
        {
            let title = self
                .browser
                .active_tab()
                .map(|t| t.title())
                .unwrap_or_default();
            s.window.set_title(&format!("{title} — Kumo"));
        }
        let Some(s) = &mut self.surface else { return };
        let mut buffer = match s.surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[window] バッファ取得に失敗: {e}");
                return;
            }
        };
        let buffer_valid = buffer.age() > 0;
        let (chrome_out, content_out) = buffer.split_at_mut(chrome_rows * w);
        let mut damage = Vec::new();
        if self.chrome_dirty || !buffer_valid {
            self.chrome_renderer.invalidate();
            let viewport = RectF::new(0.0, 0.0, w as f32, chrome_rows as f32);
            let chrome = self.browser.chrome_page();
            if let Some(rows) = self.chrome_renderer.render(
                &chrome.list,
                &mut self.raster,
                &mut NoImageProvider,
                viewport,
                chrome_out,
                false,
            ) {
                damage.push(rows);
            }
            self.chrome_dirty = false;
        }
        let content_rows = h - chrome_rows;
        let mut copied = None;
        let presented = self.browser.present_frame(|frame| {
            copied = Some(copy_frame(
                &frame,
                content_out,
                w,
                content_rows,
                buffer_valid,
            ));
        });
        if !presented && !buffer_valid {
            // バッファが作り直された：新しいフレームがなくても最後のフレームを全面写す
            copied = self
                .browser
                .last_frame()
                .map(|frame| copy_frame(&frame, content_out, w, content_rows, false));
        }
        if let Some(rows) = copied {
            damage.push(rows.start + chrome_rows..rows.end + chrome_rows);
        }
        let status = crate::overlay::draw_status(
            &mut self.browser,
            &mut self.raster,
            content_out,
            (w, content_rows),
            &mut self.status_rows,
        );
        damage.extend(
            status
                .into_iter()
                .map(|r| r.start + chrome_rows..r.end + chrome_rows),
        );
        let width = NonZeroU32::new(w as u32).expect("w > 0");
        let rects: Vec<softbuffer::Rect> = damage
            .into_iter()
            .filter_map(|rows| {
                Some(softbuffer::Rect {
                    x: 0,
                    y: rows.start as u32,
                    width,
                    height: NonZeroU32::new(rows.len() as u32)?,
                })
            })
            .collect();
        // 何も変わっていなくても（再表示要求など）画面全体を転送する
        let result = if rects.is_empty() {
            buffer.present()
        } else {
            buffer.present_with_damage(&rects)
        };
        if let Err(e) = result {
            eprintln!("[window] 表示に失敗: {e}");
        }
        if self.mem_report && !self.reported && !self.browser.tabs().iter().any(|t| t.is_loading())
        {
            self.reported = true;
            self.report();
        }
    }
}

/// Content のフレームをウィンドウのバッファ（幅 `w`、`rows` 行）へ写し、写した行範囲を返す。
/// バッファが前回の内容を保っていれば描き直された行だけを写す。大きさが違えば重なる部分だけ。
fn copy_frame(
    frame: &FrameView<'_>,
    out: &mut [u32],
    w: usize,
    rows: usize,
    buffer_valid: bool,
) -> std::ops::Range<usize> {
    let fw = frame.width as usize;
    let fh = (frame.height as usize).min(rows);
    let (start, end) = if buffer_valid && fw == w {
        (frame.damage.0 as usize, (frame.damage.1 as usize).min(fh))
    } else {
        (0, fh)
    };
    let cols = fw.min(w);
    for y in start..end {
        let src = &frame.pixels[y * fw..y * fw + cols];
        out[y * w..y * w + cols].copy_from_slice(src);
    }
    start..end.max(start)
}

impl ApplicationHandler<ContentWake> for Viewer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.surface.is_some() {
            return;
        }
        let (w, h) = self.browser.content_viewport();
        let attributes = Window::default_attributes()
            .with_title("Kumo")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                w as u32,
                (h + CHROME_HEIGHT) as u32,
            ));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("[window] ウィンドウを作れません: {e}");
                event_loop.exit();
                return;
            }
        };
        let surface = softbuffer::Context::new(window.clone())
            .and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()));
        match surface {
            Ok(surface) => {
                self.surface = Some(Surface {
                    window: window.clone(),
                    surface,
                });
                let size = window.inner_size();
                self.resize(size.width, size.height);
            }
            Err(e) => {
                eprintln!("[window] softbuffer の初期化に失敗: {e}");
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let redraw = self.browser.maintain_memory(now);
        self.mark(redraw);
        event_loop.set_control_flow(ControlFlow::WaitUntil(now + MEMORY_CHECK_INTERVAL));
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ContentWake) {
        let redraw = self.browser.pump();
        self.mark(redraw);
        self.update_cursor();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize(size.width, size.height),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::ModifiersChanged(mods) => self.mods = mods.state(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = position;
                let (x, y) = (position.x as f32, position.y as f32);
                let redraw = if y < CHROME_HEIGHT {
                    self.browser.hover_chrome(x, y)
                } else {
                    self.browser.hover_content(x, y - CHROME_HEIGHT)
                };
                self.mark(redraw);
                self.update_cursor();
            }
            WindowEvent::CursorLeft { .. } => {
                let redraw = self.browser.leave_window();
                self.mark(redraw);
                self.update_cursor();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                let (x, y) = (self.cursor.x as f32, self.cursor.y as f32);
                let outcome = input::click(&mut self.browser, x, y, button, self.mods);
                self.apply(outcome, event_loop);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * LINE_SCROLL,
                    MouseScrollDelta::PixelDelta(p) => -p.y as f32,
                };
                // 新しい位置のフレームは Content から届く
                self.browser.scroll_by(dy);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let outcome = input::key(&mut self.browser, &event, self.mods);
                self.apply(outcome, event_loop);
            }
            _ => {}
        }
    }
}

pub fn run(browser: Browser, network: Arc<dyn Fetcher>, mem_report: bool) -> Result<(), String> {
    let event_loop = EventLoop::<ContentWake>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    let proxy = event_loop.create_proxy();
    let mut browser = browser;
    browser.set_notify(Arc::new(move || {
        let _ = proxy.send_event(ContentWake);
    }));
    let mut viewer = Viewer {
        browser,
        network,
        raster: CpuRasterizer::default(),
        chrome_renderer: ScrollRenderer::default(),
        chrome_dirty: true,
        surface: None,
        cursor: PhysicalPosition::new(0.0, 0.0),
        mods: ModifiersState::empty(),
        mem_report,
        reported: false,
        status_rows: None,
        pointer: Pointer::Default,
    };
    // 通知を設定する前に届いていた分を取り込む
    let redraw = viewer.browser.pump();
    viewer.chrome_dirty |= redraw.chrome;
    event_loop.run_app(&mut viewer).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_damaged_rows_or_everything() {
        let pixels: Vec<u32> = (0..12).collect();
        let frame = FrameView {
            pixels: &pixels,
            width: 4,
            height: 3,
            damage: (2, 3),
        };
        let mut out = vec![0u32; 5 * 3];
        assert_eq!(copy_frame(&frame, &mut out, 4, 3, true), 2..3);
        assert_eq!(&out[8..12], &[8, 9, 10, 11]);
        assert_eq!(out[0], 0, "描き直していない行は写さない");
        // 幅が違う（リサイズ直後）なら重なる部分を全部
        let mut wide = vec![7u32; 5 * 2];
        assert_eq!(copy_frame(&frame, &mut wide, 5, 2, true), 0..2);
        assert_eq!(&wide[..5], &[0, 1, 2, 3, 7]);
        assert_eq!(&wide[5..9], &[4, 5, 6, 7]);
    }
}
