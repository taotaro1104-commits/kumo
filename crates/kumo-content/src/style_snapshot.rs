//! 直近のスタイル計算の結果のうち、`getComputedStyle` が答える分だけを持つ（JS を使うページだけ）。
//! 計算値そのものではなく文字列にして持つので、スタイル計算用の一時ヒープを引き止めない。

use std::collections::HashMap;
use std::sync::Arc;

use kumo_css::{COMPUTED_PROPERTIES, ComputedStyle, StyleMap, computed_value};
use kumo_dom::{Document, NodeId};

/// 要素ごとに、共有された計算値（同じ計算値は1つ）の番号を持つ。
pub struct StyleSnapshot {
    index: Vec<u32>,
    values: Vec<Box<[Box<str>]>>,
}

const NONE: u32 = u32::MAX;

impl StyleSnapshot {
    pub fn capture(doc: &Document, styles: &StyleMap) -> Self {
        let mut seen: HashMap<*const ComputedStyle, u32> = HashMap::new();
        let mut values: Vec<Box<[Box<str>]>> = Vec::new();
        let mut index = Vec::with_capacity(doc.len());
        for i in 0..doc.len() {
            let Some(style) = styles.get(NodeId::from_index(i)) else {
                index.push(NONE);
                continue;
            };
            let slot = *seen.entry(Arc::as_ptr(style)).or_insert_with(|| {
                let row = COMPUTED_PROPERTIES
                    .iter()
                    .map(|p| Box::from(computed_value(style, p).unwrap_or_default().as_str()))
                    .collect();
                values.push(row);
                u32::try_from(values.len() - 1).unwrap_or(NONE)
            });
            index.push(slot);
        }
        Self { index, values }
    }
}

impl kumo_js::StyleSource for StyleSnapshot {
    fn value(&self, node: usize, name: &str) -> Option<String> {
        let row = self.values.get(*self.index.get(node)? as usize)?;
        let column = COMPUTED_PROPERTIES.iter().position(|p| *p == name)?;
        Some(row[column].to_string())
    }
}
