//! 1ページ分のパイプライン：取得 → パース → スタイル → レイアウト → ディスプレイリスト。

use kumo_css::{CascadeEngine, Device, StyleEngine, StyleMap, Stylesheet};
use kumo_dom::Document;
use kumo_html::{StyleSource, stylesheets};
use kumo_image::ImageStore;
use kumo_layout::build_box_tree;
use kumo_mem::{Subsystem, scope};
use kumo_paint::{DisplayList, build_display_list};
use kumo_text::TextSystem;

use crate::loader::Loader;

pub struct Page {
    doc: Document,
    engine: CascadeEngine,
    /// フォントDB・シェーピング用の文脈（ページをまたいで共有する前提）
    text: Option<TextSystem>,
    boxes: usize,
    pub list: DisplayList,
    pub images: ImageStore<Loader>,
    viewport: (f32, f32),
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
    /// 文書と外部スタイルシートを読み込み、指定ビューポートでレイアウトする。
    pub fn load(target: &str, width: f32, height: f32) -> Result<Self, String> {
        let loader = Loader::new(target)?;
        let doc = {
            let _tag = scope(Subsystem::Dom);
            // 取得バッファとパーサの一時データは専用ヒープに置き、完成した DOM だけを
            // 通常ヒープへ詰めて複製する（アリーナを連続領域にして断片化を防ぐ）
            kumo_mem::with_scratch_heap(|| {
                let bytes = loader.load_document()?;
                let parsed = kumo_html::parse_html_bytes(&bytes);
                Ok::<_, String>(kumo_mem::with_long_lived(|| parsed.clone()))
            })?
        };
        let sheets = {
            let _tag = scope(Subsystem::Style);
            load_stylesheets(&doc, &loader)
        };
        let engine = {
            let _tag = scope(Subsystem::Style);
            CascadeEngine::new(sheets)
        };
        let images = ImageStore::new(loader);
        let mut page = Self {
            doc,
            engine,
            text: None,
            boxes: 0,
            list: DisplayList::default(),
            images,
            viewport: (width, height),
            unique_styles: 0,
        };
        page.relayout(width, height);
        Ok(page)
    }

    /// ビューポート変更時：スタイル（@media）から作り直す。
    pub fn relayout(&mut self, width: f32, height: f32) {
        self.viewport = (width, height);
        let text = self.text.take().unwrap_or_else(|| {
            let _tag = scope(Subsystem::Text);
            TextSystem::new()
        });
        // スタイル計算とレイアウトの一時データは専用ヒープに置き、終わったらページごと返す。
        // ディスプレイリストとフォントDBだけを通常ヒープに残す。
        let (list, text, boxes, unique_styles) = kumo_mem::with_scratch_heap(|| {
            let styles: StyleMap = {
                let _tag = scope(Subsystem::Style);
                self.engine.compute(&self.doc, &Device { width, height })
            };
            let tree = {
                let _tag = scope(Subsystem::Layout);
                let mut tree = build_box_tree(&self.doc, &styles, text, &self.images);
                kumo_layout::layout(&mut tree, width, height);
                tree
            };
            let list = {
                let _tag = scope(Subsystem::Paint);
                kumo_mem::with_long_lived(|| build_display_list(&tree))
            };
            // レイアウトツリーは再生成できる派生データなので、描画命令を作ったら捨てる（設計書）。
            // 再レイアウト時は DOM から作り直し、フォントDBだけを使い回す。
            let boxes = tree.boxes.len();
            (list, tree.into_text_system(), boxes, styles.unique_styles)
        });
        self.list = list;
        self.boxes = boxes;
        self.unique_styles = unique_styles;
        self.text = Some(text);
        kumo_mem::release_free_memory();
    }

    pub fn viewport(&self) -> (f32, f32) {
        self.viewport
    }

    pub fn document_height(&self) -> f32 {
        self.list.height
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

fn load_stylesheets(doc: &Document, loader: &Loader) -> Vec<Stylesheet> {
    stylesheets(doc)
        .into_iter()
        .filter_map(|source| match source {
            StyleSource::Inline(css) => Some(Stylesheet::parse(&css)),
            StyleSource::External(href) => match loader.load(&href) {
                Ok(bytes) => Some(Stylesheet::parse(&String::from_utf8_lossy(&bytes))),
                Err(e) => {
                    eprintln!("[css] {e}");
                    None
                }
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        format!("{}/../../tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn loads_fixture_through_whole_pipeline() {
        let mut page = Page::load(&fixture("basic.html"), 900.0, 700.0).expect("読み込める");
        let stats = page.stats();
        assert!(stats.dom_nodes > 50);
        assert!(stats.rules > 0);
        assert!(stats.boxes > 10);
        assert!(stats.display_items > 20);
        assert!(stats.unique_styles > 0);
        assert!(page.document_height() >= 700.0);

        // 幅を変えると @media とレイアウトが再計算される
        let before = page.list.entries.len();
        page.relayout(400.0, 700.0);
        assert_eq!(page.viewport(), (400.0, 700.0));
        assert!(page.document_height() > 700.0, "狭い幅では縦に伸びる");
        assert!(page.list.entries.len() >= before / 2);
    }

    #[test]
    fn missing_document_is_an_error() {
        assert!(Page::load(&fixture("nope.html"), 800.0, 600.0).is_err());
    }
}
