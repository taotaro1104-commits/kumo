//! Content 全体の統計（about:memory・メモリ管理用）。

use crate::engine::ContentEngine;
use crate::protocol::ContentStats;

impl ContentEngine {
    pub fn stats(&self) -> ContentStats {
        let (mut dom_nodes, mut display_items, mut js_tabs, mut js_errors) = (0, 0, 0, 0);
        let mut frozen_tabs = 0;
        for tab in self.tabs.values() {
            let Some(page) = tab.page.as_ref() else {
                continue;
            };
            let stats = page.stats();
            dom_nodes += stats.dom_nodes;
            display_items += stats.display_items;
            js_tabs += usize::from(page.has_js());
            frozen_tabs += usize::from(page.is_frozen());
            js_errors += tab.js_errors;
        }
        ContentStats {
            tabs: self.tabs.len(),
            dom_nodes,
            display_items,
            js_tabs,
            js_errors,
            frozen_tabs,
            decoded_image_bytes: self.services.images.decoded_bytes() as u64,
            compressed_image_bytes: self.services.images.compressed_bytes() as u64,
            glyph_bytes: self.frame.raster.glyphs.bytes() as u64,
            frame_bytes: self.frame.bytes() as u64,
            private_bytes: (self.private_bytes)(),
        }
    }
}
