//! 1ページ分のパイプラインの後半：スタイル → レイアウト → ディスプレイリスト（取得とパースは [`crate::fetch`]）。

use kumo_css::{CascadeEngine, Device, StyleEngine, StyleMap};
use kumo_dom::Document;
use kumo_layout::build_box_tree;
use kumo_mem::{Subsystem, scope};
use kumo_paint::{DisplayList, build_display_list};
use url::Url;

use crate::fetch::{FetchedPage, prepare};
use crate::services::{PageImages, Services};

pub struct Page {
    url: Url,
    title: String,
    doc: Document,
    engine: CascadeEngine,
    pub list: DisplayList,
    viewport: (f32, f32),
    boxes: usize,
    unique_styles: usize,
}

/// 計測用の各段階の数値。
#[derive(Debug, Default)]
pub struct PageStats {
    pub dom_nodes: usize,
    pub rules: usize,
    pub unique_styles: usize,
    pub boxes: usize,
    pub display_items: usize,
}

impl Page {
    /// 取得済みのページからスタイルとレイアウトを作る（Content のメインスレッドで動かす）。
    pub fn build(fetched: FetchedPage, services: &mut Services, viewport: (f32, f32)) -> Self {
        for (src, data) in fetched.images {
            services.images.preload(&src, data);
        }
        let engine = {
            let _tag = scope(Subsystem::Style);
            CascadeEngine::new(fetched.stylesheets)
        };
        let title = title_of(&fetched.doc).unwrap_or_else(|| fetched.url.to_string());
        let mut page = Self {
            url: fetched.url,
            title,
            doc: fetched.doc,
            engine,
            list: DisplayList::default(),
            viewport,
            boxes: 0,
            unique_styles: 0,
        };
        page.relayout(viewport, services);
        page
    }

    /// HTML 文字列から作る（ブラウザ UI やエラーページ用）。外部 CSS と画像はその場で取得する。
    pub fn from_html(html: &str, url: Url, services: &mut Services, viewport: (f32, f32)) -> Self {
        let fetcher = services.fetcher.clone();
        Self::build(prepare(html, url, &*fetcher), services, viewport)
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// DOM を変更する（フォームの値など）。表示に反映するには再レイアウトが必要。
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn viewport(&self) -> (f32, f32) {
        self.viewport
    }

    pub fn document_height(&self) -> f32 {
        self.list.height
    }

    /// ビューポート変更時：スタイル（@media）から作り直す。
    pub fn relayout(&mut self, viewport: (f32, f32), services: &mut Services) {
        let (width, height) = viewport;
        self.viewport = viewport;
        let text = services.take_text();
        // スタイル計算とレイアウトの一時データは専用ヒープに置き、終わったらページごと返す。
        let (list, text, boxes, unique_styles) = kumo_mem::with_scratch_heap(|| {
            let styles: StyleMap = {
                let _tag = scope(Subsystem::Style);
                self.engine.compute(&self.doc, &Device { width, height })
            };
            let images = PageImages {
                base: &self.url,
                store: &services.images,
            };
            let tree = {
                let _tag = scope(Subsystem::Layout);
                let mut tree = build_box_tree(&self.doc, &styles, text, &images);
                kumo_layout::layout(&mut tree, width, height);
                tree
            };
            let list = {
                let _tag = scope(Subsystem::Paint);
                kumo_mem::with_long_lived(|| build_display_list(&tree))
            };
            // レイアウトツリーは再生成できる派生データなので、描画命令を作ったら捨てる（設計書）
            let boxes = tree.boxes.len();
            (list, tree.into_text_system(), boxes, styles.unique_styles)
        });
        services.return_text(text);
        self.list = list;
        self.boxes = boxes;
        self.unique_styles = unique_styles;
        kumo_mem::release_free_memory();
    }

    /// リンクの href をこのページ基準で解決する。
    pub fn resolve(&self, href: &str) -> Option<Url> {
        self.url.join(href.trim()).ok()
    }

    pub fn stats(&self) -> PageStats {
        PageStats {
            dom_nodes: self.doc.len(),
            rules: self.engine.rule_count(),
            unique_styles: self.unique_styles,
            boxes: self.boxes,
            display_items: self.list.entries.len(),
        }
    }
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&#39;")
        .replace('"', "&quot;")
}

fn title_of(doc: &Document) -> Option<String> {
    let title = doc
        .descendants(doc.root())
        .find(|&n| doc.element(n).is_some_and(|e| e.is_html("title")))?;
    let text: String = doc
        .text_content(title)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}
