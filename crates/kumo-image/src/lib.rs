//! 画像：圧縮データを保持し、表示サイズでデコードしたビットマップを予算付き LRU で持つ。
#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use image::imageops::FilterType;
use kumo_paint::{ImageBitmap, ImageProvider};

/// 展開済み画像の予算（設計書：8MB）。
pub const DECODED_BUDGET: usize = 8 * 1024 * 1024;
/// 1枚あたりの展開上限（巨大画像で予算を食い尽くさないため）。
const MAX_DIMENSION: u32 = 4096;

/// `src` から圧縮バイト列を取得する（ファイル読み込みや HTTP 取得）。
pub trait ImageLoader {
    fn load(&self, src: &str) -> Option<Vec<u8>>;
}

/// `src` ごとの圧縮データ（`None` は取得失敗）。
type SourceMap = HashMap<Arc<str>, Option<Arc<[u8]>>>;

struct Decoded {
    bitmap: ImageBitmap,
    last_used: u64,
}

/// 画像ストア。圧縮データは常駐、展開データは LRU で破棄する。
pub struct ImageStore<L: ImageLoader> {
    loader: L,
    /// 取得済みの圧縮データ（`None` は取得失敗）
    sources: RefCell<SourceMap>,
    decoded: HashMap<(Arc<str>, u32, u32), Decoded>,
    decoded_bytes: usize,
    budget: usize,
    tick: u64,
}

impl<L: ImageLoader> ImageStore<L> {
    pub fn new(loader: L) -> Self {
        Self::with_budget(loader, DECODED_BUDGET)
    }

    pub fn with_budget(loader: L, budget: usize) -> Self {
        Self {
            loader,
            sources: RefCell::new(HashMap::new()),
            decoded: HashMap::new(),
            decoded_bytes: 0,
            budget,
            tick: 0,
        }
    }

    pub fn decoded_bytes(&self) -> usize {
        self.decoded_bytes
    }

    pub fn compressed_bytes(&self) -> usize {
        self.sources
            .borrow()
            .values()
            .flatten()
            .map(|d| d.len())
            .sum()
    }

    fn source(&self, src: &str) -> Option<Arc<[u8]>> {
        if let Some(entry) = self.sources.borrow().get(src) {
            return entry.clone();
        }
        let data: Option<Arc<[u8]>> = self.loader.load(src).map(Arc::from);
        self.sources
            .borrow_mut()
            .insert(Arc::from(src), data.clone());
        data
    }

    /// 先に取得しておいた圧縮データを入れる（既にあれば何もしない）。
    /// 取得を別スレッドで済ませ、レイアウト中に取得で待たないようにするため。
    pub fn preload(&self, src: &str, data: Option<Vec<u8>>) {
        self.sources
            .borrow_mut()
            .entry(Arc::from(src))
            .or_insert_with(|| data.map(Arc::from));
    }

    /// 取得済み（失敗を含む）か。
    pub fn has_source(&self, src: &str) -> bool {
        self.sources.borrow().contains_key(src)
    }

    /// ヘッダだけを読んで固有サイズを返す（全体はデコードしない）。
    pub fn intrinsic_size(&self, src: &str) -> Option<(f32, f32)> {
        let data = self.source(src)?;
        let reader = image::ImageReader::new(Cursor::new(&*data))
            .with_guessed_format()
            .ok()?;
        let (w, h) = reader.into_dimensions().ok()?;
        Some((w as f32, h as f32))
    }

    fn decode(&self, src: &str, width: u32, height: u32) -> Option<ImageBitmap> {
        let data = self.source(src)?;
        let image = image::load_from_memory(&data).ok()?;
        let (w, h) = (width.min(MAX_DIMENSION), height.min(MAX_DIMENSION));
        let resized = if image.width() == w && image.height() == h {
            image
        } else {
            image.resize_exact(w, h, FilterType::Triangle)
        };
        let mut rgba = resized.into_rgba8().into_raw();
        premultiply(&mut rgba);
        Some(ImageBitmap {
            width: w,
            height: h,
            data: rgba,
        })
    }

    fn evict(&mut self) {
        if self.decoded_bytes <= self.budget {
            return;
        }
        let mut by_age: Vec<_> = self
            .decoded
            .iter()
            .map(|(k, d)| (d.last_used, k.clone()))
            .collect();
        by_age.sort_unstable_by_key(|(age, _)| *age);
        for (_, key) in by_age {
            if self.decoded_bytes <= self.budget {
                break;
            }
            if let Some(d) = self.decoded.remove(&key) {
                self.decoded_bytes -= d.bitmap.data.len();
            }
        }
    }
}

fn premultiply(rgba: &mut [u8]) {
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = u16::from(px[3]);
        for c in &mut px[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
}

impl<L: ImageLoader> ImageProvider for ImageStore<L> {
    fn bitmap(&mut self, src: &str, width: u32, height: u32) -> Option<&ImageBitmap> {
        self.tick += 1;
        let key = (Arc::<str>::from(src), width, height);
        if !self.decoded.contains_key(&key) {
            let bitmap = self.decode(src, width, height)?;
            self.decoded_bytes += bitmap.data.len();
            self.decoded.insert(
                key.clone(),
                Decoded {
                    bitmap,
                    last_used: self.tick,
                },
            );
            self.evict();
        }
        let entry = self.decoded.get_mut(&key)?;
        entry.last_used = self.tick;
        Some(&entry.bitmap)
    }
}

impl<L: ImageLoader> kumo_layout::ImageSizes for ImageStore<L> {
    fn intrinsic_size(&self, src: &str) -> Option<(f32, f32)> {
        ImageStore::intrinsic_size(self, src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemoryLoader(HashMap<&'static str, Vec<u8>>);

    impl ImageLoader for MemoryLoader {
        fn load(&self, src: &str) -> Option<Vec<u8>> {
            self.0.get(src).cloned()
        }
    }

    fn png(w: u32, h: u32, color: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba(color));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    fn store(budget: usize) -> ImageStore<MemoryLoader> {
        let mut files = HashMap::new();
        files.insert("a.png", png(100, 50, [255, 0, 0, 255]));
        files.insert("b.png", png(10, 10, [0, 0, 255, 128]));
        ImageStore::with_budget(MemoryLoader(files), budget)
    }

    #[test]
    fn intrinsic_size_from_header() {
        let s = store(DECODED_BUDGET);
        assert_eq!(s.intrinsic_size("a.png"), Some((100.0, 50.0)));
        assert_eq!(s.intrinsic_size("missing.png"), None);
    }

    #[test]
    fn decodes_at_display_size_premultiplied() {
        let mut s = store(DECODED_BUDGET);
        let bitmap = s.bitmap("a.png", 20, 10).unwrap();
        assert_eq!((bitmap.width, bitmap.height), (20, 10));
        assert_eq!(&bitmap.data[..4], &[255, 0, 0, 255]);
        let half = s.bitmap("b.png", 10, 10).unwrap();
        assert_eq!(&half.data[..4], &[0, 0, 128, 128]);
    }

    #[test]
    fn evicts_over_budget() {
        let mut s = store(20 * 10 * 4);
        s.bitmap("a.png", 20, 10).unwrap();
        s.bitmap("b.png", 10, 10).unwrap();
        assert!(s.decoded_bytes() <= 20 * 10 * 4);
        assert!(s.compressed_bytes() > 0);
    }

    #[test]
    fn preloaded_sources_skip_the_loader() {
        let s = store(DECODED_BUDGET);
        assert!(!s.has_source("pre.png"));
        s.preload("pre.png", Some(png(7, 3, [0, 0, 0, 255])));
        s.preload("pre.png", None); // 2回目は無視
        assert!(s.has_source("pre.png"));
        assert_eq!(s.intrinsic_size("pre.png"), Some((7.0, 3.0)));
        s.preload("gone.png", None);
        assert_eq!(
            s.intrinsic_size("gone.png"),
            None,
            "失敗も記録して取りに行かない"
        );
    }
}
