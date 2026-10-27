//! winit + softbuffer のウィンドウ。前フレームを再利用し、新しく見えた帯だけを描く。

use std::num::NonZeroU32;
use std::rc::Rc;

use kumo_paint::{CpuRasterizer, RectF, ScrollRenderer};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::page::Page;

const LINE_SCROLL: f32 = 48.0;

struct Surface {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
}

pub struct Viewer {
    page: Page,
    raster: CpuRasterizer,
    renderer: ScrollRenderer,
    scroll_y: f32,
    surface: Option<Surface>,
    title: String,
    mem_report: bool,
    reported: bool,
}

impl Viewer {
    pub fn new(page: Page, title: String, mem_report: bool) -> Self {
        Self {
            page,
            raster: CpuRasterizer::default(),
            renderer: ScrollRenderer::default(),
            scroll_y: 0.0,
            surface: None,
            title,
            mem_report,
            reported: false,
        }
    }

    fn scroll_by(&mut self, dy: f32) {
        let (_, vh) = self.page.viewport();
        let max = (self.page.document_height() - vh).max(0.0);
        let next = (self.scroll_y + dy).clamp(0.0, max).round();
        if next != self.scroll_y {
            self.scroll_y = next;
            self.request_redraw();
        }
    }

    fn request_redraw(&self) {
        if let Some(s) = &self.surface {
            s.window.request_redraw();
        }
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
        if self.page.viewport() != (width as f32, height as f32) {
            self.page.relayout(width as f32, height as f32);
            self.renderer.invalidate();
            self.scroll_by(0.0);
        }
    }

    fn redraw(&mut self) {
        let Some(s) = &mut self.surface else { return };
        let size = s.window.inner_size();
        let (w, h) = (size.width as usize, size.height as usize);
        if w == 0 || h == 0 {
            return;
        }
        let mut buffer = match s.surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[window] バッファ取得に失敗: {e}");
                return;
            }
        };
        let viewport = RectF::new(0.0, self.scroll_y, w as f32, h as f32);
        let buffer_valid = buffer.age() > 0;
        let damage = self.renderer.render(
            &self.page.list,
            &mut self.raster,
            &mut self.page.images,
            viewport,
            &mut buffer,
            buffer_valid,
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
        if self.mem_report && !self.reported {
            self.reported = true;
            crate::print_memory_report(&self.page, &self.raster, self.renderer.bytes());
        }
    }

    fn on_key(&mut self, event: &KeyEvent, event_loop: &ActiveEventLoop) {
        if event.state != ElementState::Pressed {
            return;
        }
        let (_, vh) = self.page.viewport();
        match &event.logical_key {
            Key::Named(NamedKey::ArrowDown) => self.scroll_by(LINE_SCROLL),
            Key::Named(NamedKey::ArrowUp) => self.scroll_by(-LINE_SCROLL),
            Key::Named(NamedKey::PageDown | NamedKey::Space) => self.scroll_by(vh * 0.9),
            Key::Named(NamedKey::PageUp) => self.scroll_by(-vh * 0.9),
            Key::Named(NamedKey::Home) => self.scroll_by(-self.scroll_y),
            Key::Named(NamedKey::End) => self.scroll_by(f32::MAX / 2.0),
            Key::Named(NamedKey::Escape) => event_loop.exit(),
            Key::Character(c) if c.as_str() == "m" => {
                crate::print_memory_report(&self.page, &self.raster, self.renderer.bytes());
            }
            _ => {}
        }
    }
}

impl ApplicationHandler for Viewer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.surface.is_some() {
            return;
        }
        let (w, h) = self.page.viewport();
        let attributes = Window::default_attributes()
            .with_title(format!("Kumo — {}", self.title))
            .with_inner_size(winit::dpi::PhysicalSize::new(w as u32, h as u32));
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
                window.request_redraw();
            }
            Err(e) => {
                eprintln!("[window] softbuffer の初期化に失敗: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize(size.width, size.height),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, y) => self.scroll_by(-y * LINE_SCROLL),
                MouseScrollDelta::PixelDelta(p) => self.scroll_by(-p.y as f32),
            },
            WindowEvent::KeyboardInput { event, .. } => self.on_key(&event, event_loop),
            _ => {}
        }
    }
}

pub fn run(viewer: Viewer) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    let mut viewer = viewer;
    event_loop.run_app(&mut viewer).map_err(|e| e.to_string())
}
