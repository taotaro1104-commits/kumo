//! 1ページ分のパイプラインの後半：スクリプトの実行 → スタイル → レイアウト → ディスプレイリスト
//! （取得とパースは [`crate::fetch`]）。DOM は JS と共有するので `Rc<RefCell<…>>` で持つ。

use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use kumo_css::{CascadeEngine, Device, StyleEngine, StyleMap};
use kumo_dom::{Document, NodeId};
use kumo_js::JsRuntime;
use kumo_layout::build_box_tree;
use kumo_mem::{Subsystem, scope};
use kumo_paint::{DisplayList, build_display_list_with};
use url::Url;

use crate::fetch::{FetchedPage, prepare};
use crate::level::LoadLevel;
use crate::services::{PageImages, Services};
use crate::style_snapshot::StyleSnapshot;

pub struct Page {
    url: Url,
    title: String,
    doc: Rc<RefCell<Document>>,
    engine: CascadeEngine,
    pub list: DisplayList,
    viewport: (f32, f32),
    /// 拡大率（1.0 が等倍）。レイアウトは `viewport / zoom` で行い、描画命令を `zoom` 倍して持つ
    zoom: f32,
    boxes: usize,
    unique_styles: usize,
    js: Option<JsRuntime>,
    /// 凍結中（スクリプトと描画命令を捨てている）
    frozen: bool,
}

impl Drop for Page {
    fn drop(&mut self) {
        self.stop_js();
    }
}

/// 計測用の各段階の数値。
#[derive(Debug, Default)]
pub struct PageStats {
    pub dom_nodes: usize,
    pub rules: usize,
    pub unique_styles: usize,
    pub boxes: usize,
    pub display_items: usize,
    /// スクリプトの未捕捉の例外の数（これまでの合計）
    pub js_errors: usize,
}

impl Page {
    /// 取得済みのページからスクリプトの実行・スタイル・レイアウトを行う（Content のメインスレッドで動かす）。
    /// `clock` は Content の時計（タイマーと `performance.now()` の基準）。`None` ならスクリプトを実行しない。
    pub fn build(
        fetched: FetchedPage,
        services: &mut Services,
        viewport: (f32, f32),
        clock: Option<Duration>,
        limits: kumo_js::ScriptLimits,
    ) -> Self {
        for (src, data) in fetched.images {
            services.images.preload(&src, data);
        }
        let engine = {
            let _tag = scope(Subsystem::Style);
            CascadeEngine::new(fetched.stylesheets)
        };
        let title = title_of(&fetched.doc).unwrap_or_else(|| fetched.url.to_string());
        let doc = Rc::new(RefCell::new(fetched.doc));
        let js =
            clock.and_then(|c| start_js(&doc, &fetched.url, &fetched.scripts, viewport, c, limits));
        let mut page = Self {
            url: fetched.url,
            title,
            doc,
            engine,
            list: DisplayList::default(),
            viewport,
            zoom: 1.0,
            boxes: 0,
            unique_styles: 0,
            js,
            frozen: false,
        };
        page.refresh_title();
        // 文書の読み込みの完了（DOMContentLoaded・load）は最初のレイアウトの後に送る：
        // スクリプトの初期化が `offsetWidth` などの幾何情報を読めるように
        if let Some(js) = &mut page.js {
            js.take_dirty();
        }
        page.relayout(viewport, services);
        page.finish_load(services);
        page
    }

    /// `DOMContentLoaded` と `load` を送る。スクリプトが DOM を変えたら再レイアウトする。
    fn finish_load(&mut self, services: &mut Services) {
        let Some(js) = &mut self.js else { return };
        js.set_ready_state("interactive");
        js.fire_dom_content_loaded();
        js.set_ready_state("complete");
        js.fire_load();
        // スクリプトの一時データ（構文木・中間表現）を回収して、ページが保持するメモリを小さくする
        js.collect_garbage();
        let dirty = js.take_dirty();
        if dirty {
            self.relayout(self.viewport, services);
        }
        kumo_mem::release_free_memory();
    }

    /// HTML 文字列から作る（ブラウザ UI やエラーページ用）。外部 CSS と画像はその場で取得する。
    pub fn from_html(html: &str, url: Url, services: &mut Services, viewport: (f32, f32)) -> Self {
        let fetcher = services.fetcher.clone();
        Self::build(
            prepare(html, url, &*fetcher, LoadLevel::Standard, true),
            services,
            viewport,
            Some(Duration::ZERO),
            kumo_js::ScriptLimits::default(),
        )
    }

    pub fn document(&self) -> Ref<'_, Document> {
        self.doc.borrow()
    }

    /// DOM を変更する（フォームの値など）。表示に反映するには再レイアウトが必要。
    pub fn document_mut(&self) -> RefMut<'_, Document> {
        self.doc.borrow_mut()
    }

    pub fn js_mut(&mut self) -> Option<&mut JsRuntime> {
        self.js.as_mut()
    }

    /// スクリプトを止めて、そのメモリを返す。
    fn stop_js(&mut self) {
        if let Some(js) = self.js.take() {
            js.shutdown();
        }
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// 凍結する：スクリプトと描画命令を捨てて DOM とスタイルシートだけ残す。
    /// 戻すには [`Page::relayout`]（スクリプトは戻らない。静的な文書として表示する）。
    pub fn freeze(&mut self) {
        self.stop_js();
        self.list = DisplayList::default();
        self.boxes = 0;
        self.unique_styles = 0;
        self.frozen = true;
        kumo_mem::release_free_memory();
    }

    pub fn has_js(&self) -> bool {
        self.js.is_some()
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// DOM の `<title>` から題名を読み直す（スクリプトが変えうる）。変わったら `true`。
    pub fn refresh_title(&mut self) -> bool {
        let latest = title_of(&self.doc.borrow());
        match latest {
            Some(title) if title != self.title => {
                self.title = title;
                true
            }
            _ => false,
        }
    }

    /// 文書座標 (x, y) にある最も手前の要素（JS のイベント対象）。
    pub fn hit_node(&self, x: f32, y: f32) -> Option<NodeId> {
        self.list.hit_at(x, y).map(NodeId::from_index)
    }

    pub fn viewport(&self) -> (f32, f32) {
        self.viewport
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// 拡大率を変える。表示に反映するには再レイアウトが必要。
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
    }

    pub fn document_height(&self) -> f32 {
        self.list.height
    }

    /// ビューポート変更時：スタイル（@media）から作り直す。
    pub fn relayout(&mut self, viewport: (f32, f32), services: &mut Services) {
        self.viewport = viewport;
        // 拡大すると CSS 上のビューポートは狭くなる（@media やレイアウトはこの大きさで見る）
        let zoom = self.zoom;
        let (width, height) = (viewport.0 / zoom, viewport.1 / zoom);
        self.frozen = false;
        if let Some(js) = &mut self.js {
            js.set_viewport((width, height));
        }
        let record_hits = self.js.is_some();
        let text = services.take_text();
        let doc = self.doc.clone();
        // スタイル計算とレイアウトの一時データは専用ヒープに置き、終わったらページごと返す。
        let (list, text, boxes, unique_styles, snapshot) = kumo_mem::with_scratch_heap(|| {
            let doc = doc.borrow();
            let styles: StyleMap = {
                let _tag = scope(Subsystem::Style);
                self.engine.compute(&doc, &Device { width, height })
            };
            let images = PageImages {
                base: &self.url,
                store: &services.images,
            };
            let tree = {
                let _tag = scope(Subsystem::Layout);
                let mut tree = build_box_tree(&doc, &styles, text, &images);
                kumo_layout::layout(&mut tree, width, height);
                tree
            };
            let list = {
                let _tag = scope(Subsystem::Paint);
                kumo_mem::with_long_lived(|| build_display_list_with(&tree, record_hits))
            };
            // レイアウトツリーは再生成できる派生データなので、描画命令を作ったら捨てる（設計書）
            let boxes = tree.boxes.len();
            // `getComputedStyle` 用に、JS を使うページだけ計算値を文字列にして持ち出す
            let snapshot = record_hits
                .then(|| kumo_mem::with_long_lived(|| StyleSnapshot::capture(&doc, &styles)));
            (
                list,
                tree.into_text_system(),
                boxes,
                styles.unique_styles,
                snapshot,
            )
        });
        services.return_text(text);
        let mut list = list;
        // 幾何情報は JS ランタイムへ渡して、描画命令側には残さない
        let geometry = std::mem::take(&mut list.geometry);
        if let (Some(js), Some(snapshot)) = (&mut self.js, snapshot) {
            js.set_style_source(Rc::new(snapshot));
        }
        if let Some(js) = &mut self.js {
            js.set_geometry(
                geometry.iter().map(|&(n, r)| (n, [r.x, r.y, r.w, r.h])),
                0.0,
            );
        }
        if zoom != 1.0 {
            list.scale(zoom);
        }
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
            dom_nodes: self.doc.borrow().len(),
            rules: self.engine.rule_count(),
            unique_styles: self.unique_styles,
            boxes: self.boxes,
            display_items: self.list.entries.len(),
            js_errors: 0,
        }
    }
}

/// JS が必要なページならランタイムを作り、文書順にスクリプトを実行して、読み込みのイベントを送る。
fn start_js(
    doc: &Rc<RefCell<Document>>,
    url: &Url,
    bodies: &[(NodeId, Option<String>)],
    viewport: (f32, f32),
    clock: Duration,
    limits: kumo_js::ScriptLimits,
) -> Option<JsRuntime> {
    if !kumo_js::needs_js(&doc.borrow(), url) {
        return None;
    }
    let mut js = match JsRuntime::new(doc.clone(), url.clone(), viewport) {
        Ok(js) => js,
        Err(e) => {
            eprintln!("[js] {e}");
            return None;
        }
    };
    js.set_limits(limits);
    js.set_time(clock);
    let external: HashMap<NodeId, &Option<String>> = bodies.iter().map(|(n, b)| (*n, b)).collect();
    let scripts = kumo_js::scripts_in_order(&doc.borrow(), url);
    for (node, info) in scripts {
        let source = match &info.src {
            Some(src) => match external.get(&node) {
                Some(Some(body)) => (src.clone(), body.clone()),
                _ => continue,
            },
            None => ("inline".to_owned(), info.text),
        };
        // 例外は記録され、残りのスクリプトの実行は続ける
        let started = std::time::Instant::now();
        let _ = js.run_script(&source.1, &source.0, Some(node));
        if std::env::var_os("KUMO_JS_LOG").is_some() {
            eprintln!(
                "[js] {} {}B {:.0}ms",
                source.0.chars().take(90).collect::<String>(),
                source.1.len(),
                started.elapsed().as_secs_f64() * 1000.0
            );
        }
    }
    Some(js)
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
