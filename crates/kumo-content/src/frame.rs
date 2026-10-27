//! 共有メモリのフレーム。Content が描き（[`FrameWriter`]）、Browser がウィンドウへ写す（[`FrameReader`]）。
//!
//! 書き手は Browser が写し終えた通知（FrameDone）を受けるまで次を描かないので、同時に触ることはない。
//! 大きさが変わると新しい共有メモリを作り、古いものは Browser が新しい方へ切り替えた後（次の FrameDone）に捨てる。

use std::io;
use std::path::Path;

use kumo_ipc::{SharedMemory, SharedView};
use kumo_paint::{CpuRasterizer, ImageProvider, RectF, ScrollRenderer};

use crate::page::Page;
use crate::protocol::{FromContent, TabId};

/// 何も読み込んでいないタブの色。
const BLANK_PIXEL: u32 = 0x00ff_ffff;

#[derive(Default)]
pub struct FrameWriter {
    memory: Option<SharedMemory>,
    size: (u32, u32),
    retired: Vec<SharedMemory>,
    renderer: ScrollRenderer,
    pub raster: CpuRasterizer,
    /// 送ったフレームを Browser がまだ写していない
    busy: bool,
    /// 描くべき変化がある（スクロールだけなら帯の描き足しで済む）
    pending: bool,
    /// 全面を描き直す（レイアウト・タブ切替・新しい共有メモリ）
    full: bool,
    shown: Option<TabId>,
}

impl FrameWriter {
    /// 表示を更新する必要がある（スクロールなど）。
    pub fn touch(&mut self) {
        self.pending = true;
    }

    /// 全面を描き直す必要がある（レイアウトの変更など）。
    pub fn invalidate(&mut self) {
        self.pending = true;
        self.full = true;
    }

    /// Browser が前のフレームを写し終えた。
    pub fn done(&mut self) {
        self.busy = false;
        self.retired.clear();
    }

    pub fn bytes(&self) -> usize {
        self.memory.as_ref().map_or(0, SharedMemory::len) + self.renderer.bytes()
    }

    /// 必要なら `tab` を描き、Browser へ送るメッセージを返す。
    pub fn render(
        &mut self,
        tab: TabId,
        page: Option<&Page>,
        images: &mut dyn ImageProvider,
        scroll_y: f32,
        size: (u32, u32),
    ) -> Option<FromContent> {
        if self.busy || !self.pending || size.0 == 0 || size.1 == 0 {
            return None;
        }
        if self.shown != Some(tab) {
            self.full = true;
            self.shown = Some(tab);
        }
        if let Err(e) = self.ensure_memory(size) {
            eprintln!("[content] フレームを確保できません: {e}");
            return None;
        }
        let memory = self.memory.as_mut()?;
        let out = memory.as_pixels_mut();
        let (w, h) = (size.0 as usize, size.1 as usize);
        let damage = match page {
            Some(page) => {
                if self.full {
                    self.renderer.invalidate();
                }
                let viewport = RectF::new(0.0, scroll_y, w as f32, h as f32);
                self.renderer
                    .render(&page.list, &mut self.raster, images, viewport, out, true)
                    .map(|rows| (rows.start as u32, rows.end as u32))
            }
            None => {
                out[..w * h].fill(BLANK_PIXEL);
                self.renderer.invalidate();
                Some((0, size.1))
            }
        };
        self.pending = false;
        self.full = false;
        let (start, end) = damage?;
        self.busy = true;
        Some(FromContent::Frame {
            tab,
            path: memory.path().to_string_lossy().into_owned(),
            width: size.0,
            height: size.1,
            damage: (start, end),
        })
    }

    fn ensure_memory(&mut self, size: (u32, u32)) -> io::Result<()> {
        if self.memory.is_some() && self.size == size {
            return Ok(());
        }
        let len = size.0 as usize * size.1 as usize * 4;
        let memory = SharedMemory::create(&kumo_ipc::unique_path("frame"), len)?;
        if let Some(old) = self.memory.replace(memory) {
            self.retired.push(old);
        }
        self.size = size;
        self.full = true;
        Ok(())
    }
}

/// Browser 側：Content が描いたフレームを開いて読む。
#[derive(Default)]
pub struct FrameReader {
    memory: Option<SharedView>,
    size: (u32, u32),
}

impl FrameReader {
    /// [`FromContent::Frame`] の共有メモリを開く（前と同じなら開き直さない）。
    pub fn attach(&mut self, path: &str, width: u32, height: u32) -> io::Result<()> {
        let same = self
            .memory
            .as_ref()
            .is_some_and(|m| m.path() == Path::new(path));
        if !same {
            self.memory = Some(SharedView::open(Path::new(path))?);
        }
        self.size = (width, height);
        Ok(())
    }

    /// 画素（0RGB、幅 `size().0`）。まだフレームがなければ `None`。
    pub fn pixels(&self) -> Option<&[u32]> {
        let (w, h) = (self.size.0 as usize, self.size.1 as usize);
        self.memory
            .as_ref()
            .map(SharedView::as_pixels)
            .filter(|p| p.len() >= w * h)
            .map(|p| &p[..w * h])
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }
}
