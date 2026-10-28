//! 規則インデックス。セレクタを最右複合セレクタの ID / クラス / タグ名で振り分け、
//! 要素ごとに照合する候補を絞り込む（Servo の SelectorMap の簡易版）。

use std::collections::HashMap;

use kumo_dom::{ElementData, LocalName};
use selectors::parser::Component;

use crate::selector::Selector;

/// インデックスの1項目。`rule` は全シート通しの規則番号。
#[derive(Clone, Copy, Debug)]
pub struct IndexEntry {
    pub rule: u32,
    pub selector: u16,
    pub specificity: u32,
}

#[derive(Default, Debug)]
pub struct RuleIndex {
    by_id: HashMap<LocalName, Vec<IndexEntry>>,
    by_class: HashMap<LocalName, Vec<IndexEntry>>,
    by_tag: HashMap<LocalName, Vec<IndexEntry>>,
    universal: Vec<IndexEntry>,
}

enum Bucket {
    Id(LocalName),
    Class(LocalName),
    Tag(LocalName),
    Universal,
}

fn bucket_for(selector: &Selector) -> Bucket {
    let mut tag = None;
    let mut class = None;
    // iter() は最右の複合セレクタの構成要素だけを返す
    for component in selector.iter() {
        match component {
            Component::ID(id) => return Bucket::Id(id.0.clone()),
            Component::Class(c) if class.is_none() => class = Some(c.0.clone()),
            Component::LocalName(ln) if tag.is_none() => tag = Some(ln.lower_name.0.clone()),
            _ => {}
        }
    }
    match (class, tag) {
        (Some(c), _) => Bucket::Class(c),
        (None, Some(t)) => Bucket::Tag(t),
        (None, None) => Bucket::Universal,
    }
}

impl RuleIndex {
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
            && self.by_class.is_empty()
            && self.by_tag.is_empty()
            && self.universal.is_empty()
    }

    pub fn insert(&mut self, selector: &Selector, entry: IndexEntry) {
        let list = match bucket_for(selector) {
            Bucket::Id(id) => self.by_id.entry(id).or_default(),
            Bucket::Class(c) => self.by_class.entry(c).or_default(),
            Bucket::Tag(t) => self.by_tag.entry(t).or_default(),
            Bucket::Universal => &mut self.universal,
        };
        list.push(entry);
    }

    /// 要素に一致しうる候補を列挙する（重複なし）。
    pub fn candidates<'a>(
        &'a self,
        element: &'a ElementData,
    ) -> impl Iterator<Item = &'a IndexEntry> + 'a {
        let by_id = element
            .id()
            .and_then(|id| self.by_id.get(&LocalName::from(id)))
            .into_iter()
            .flatten();
        let by_class = element
            .classes()
            .enumerate()
            // 同じクラス名が重複指定されていても1回だけ引く
            .filter(|(i, c)| !element.classes().take(*i).any(|prev| prev == *c))
            .filter_map(|(_, c)| self.by_class.get(&LocalName::from(c)))
            .flatten();
        let by_tag = self.by_tag.get(&element.local).into_iter().flatten();
        by_id
            .chain(by_class)
            .chain(by_tag)
            .chain(self.universal.iter())
    }
}
