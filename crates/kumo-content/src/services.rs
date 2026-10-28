//! 全タブで共有する資源：Network プロセスへの取得口、画像キャッシュ、フォントDB。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kumo_image::{ImageLoader, ImageStore};
use kumo_net::process::Fetcher;
use kumo_net::protocol::ResourceKind;
use kumo_paint::{ImageBitmap, ImageProvider};
use kumo_text::TextSystem;
use url::Url;

/// 画像の取得を Network プロセスへ流すローダ。キーは絶対 URL。
pub struct FetchLoader {
    fetcher: Arc<dyn Fetcher>,
    /// 偽なら画像を取得しない（先読み済みでないものを、レイアウト中に取りに行かない）
    enabled: Arc<AtomicBool>,
}

impl ImageLoader for FetchLoader {
    fn load(&self, src: &str) -> Option<Vec<u8>> {
        if !self.enabled.load(Ordering::Relaxed) {
            return None;
        }
        match self.fetcher.fetch(src, ResourceKind::Image, None) {
            Ok(response) if response.status == 200 => Some(response.body),
            Ok(_) => None,
            Err(kumo_net::protocol::FetchError::Blocked) => None,
            Err(e) => {
                eprintln!("[image] {src}: {e}");
                None
            }
        }
    }
}

pub struct Services {
    pub fetcher: Arc<dyn Fetcher>,
    /// 全タブ共通の画像キャッシュ（展開済みは 8MB 予算）
    pub images: ImageStore<FetchLoader>,
    /// 画像を取得するか（設定と負荷レベルの両方が許したとき）
    images_enabled: Arc<AtomicBool>,
    text: Option<TextSystem>,
}

impl Services {
    pub fn new(fetcher: Arc<dyn Fetcher>) -> Self {
        let images_enabled = Arc::new(AtomicBool::new(true));
        Self {
            images: ImageStore::new(FetchLoader {
                fetcher: fetcher.clone(),
                enabled: images_enabled.clone(),
            }),
            fetcher,
            images_enabled,
            text: None,
        }
    }

    pub(crate) fn set_images_enabled(&self, on: bool) {
        self.images_enabled.store(on, Ordering::Relaxed);
    }

    /// フォントDBを借りる（初回だけ作る）。使い終わったら [`Self::return_text`] で返す。
    pub(crate) fn take_text(&mut self) -> TextSystem {
        self.text.take().unwrap_or_else(|| {
            let _tag = kumo_mem::scope(kumo_mem::Subsystem::Text);
            TextSystem::new()
        })
    }

    pub(crate) fn return_text(&mut self, text: TextSystem) {
        self.text = Some(text);
    }
}

/// ページ内の相対 `src` を絶対 URL にしてから共有ストアを引くアダプタ。
pub struct PageImages<'a, S> {
    pub base: &'a Url,
    pub store: S,
}

fn resolve(base: &Url, src: &str) -> Option<String> {
    base.join(src.trim()).ok().map(String::from)
}

impl kumo_layout::ImageSizes for PageImages<'_, &ImageStore<FetchLoader>> {
    fn intrinsic_size(&self, src: &str) -> Option<(f32, f32)> {
        self.store.intrinsic_size(&resolve(self.base, src)?)
    }
}

impl ImageProvider for PageImages<'_, &mut ImageStore<FetchLoader>> {
    fn bitmap(&mut self, src: &str, width: u32, height: u32) -> Option<&ImageBitmap> {
        let url = resolve(self.base, src)?;
        self.store.bitmap(&url, width, height)
    }
}
