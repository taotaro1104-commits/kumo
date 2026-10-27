//! カスケード：規則のマッチ → 優先順位で並べる → 計算値を作る。
//! 一致した規則集合と親スタイルが同じ要素は計算値を `Arc` で共有する。

use std::collections::HashMap;
use std::sync::Arc;

use kumo_dom::{Document, LocalName, NodeId};
use selectors::context::{
    MatchingContext, MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode,
    SelectorCaches,
};
use selectors::matching::matches_selector;

use crate::apply::{ResolveContext, apply_decl};
use crate::decl::{Decl, DeclarationBlock};
use crate::element::ElementRef;
use crate::hints::presentational_hints;
use crate::index::{IndexEntry, RuleIndex};
use crate::keywords::{Display, Float, Position};
use crate::media::Device;
use crate::properties::parse_property;
use crate::style::ComputedStyle;
use crate::stylesheet::{StyleRule, Stylesheet, parse_declaration_block};
use crate::var::substitute;

/// 要素ごとの計算値。テキストノード等は `None`（親のスタイルを使う）。
#[derive(Debug, Default)]
pub struct StyleMap {
    styles: Vec<Option<Arc<ComputedStyle>>>,
    /// 共有されずに新規作成された計算値の数（計測用）
    pub unique_styles: usize,
}

impl StyleMap {
    pub fn get(&self, node: NodeId) -> Option<&Arc<ComputedStyle>> {
        self.styles.get(node.index())?.as_ref()
    }
}

/// スタイル計算の抽象。将来 Stylo に差し替えられるようにトレイトで隔離する。
pub trait StyleEngine {
    fn compute(&self, doc: &Document, device: &Device) -> StyleMap;
}

/// 規則の出自（カスケードの優先順位に使う）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Origin {
    UserAgent,
    Author,
}

/// 自前の最小カスケード実装。
pub struct CascadeEngine {
    rules: Vec<(Origin, StyleRule)>,
    index: RuleIndex,
}

const UA_CSS: &str = include_str!("ua.css");

impl CascadeEngine {
    /// UA スタイルシート + 作者スタイルシート（文書順）から構築する。
    pub fn new(author_sheets: Vec<Stylesheet>) -> Self {
        let ua = Stylesheet::parse(UA_CSS);
        let sheets = std::iter::once((Origin::UserAgent, ua))
            .chain(author_sheets.into_iter().map(|s| (Origin::Author, s)));
        let mut rules = Vec::new();
        let mut index = RuleIndex::default();
        for (origin, sheet) in sheets {
            for rule in sheet.rules {
                let rule_id = u32::try_from(rules.len()).expect("規則数が多すぎる");
                for (i, selector) in rule.selectors.slice().iter().enumerate() {
                    if selector.has_pseudo_element() {
                        continue; // ::before 等は Phase 0 では描画しない
                    }
                    let entry = IndexEntry {
                        rule: rule_id,
                        selector: u16::try_from(i).unwrap_or(u16::MAX),
                        specificity: selector.specificity(),
                    };
                    index.insert(selector, entry);
                }
                rules.push((origin, rule));
            }
        }
        Self { rules, index }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// 適用順に並べた宣言1つ分。
struct Applicable<'a> {
    key: (u8, u32, u32),
    decl: &'a Decl,
}

impl StyleEngine for CascadeEngine {
    fn compute(&self, doc: &Document, device: &Device) -> StyleMap {
        let active: Vec<bool> = self
            .rules
            .iter()
            .map(|(_, r)| r.media.as_ref().is_none_or(|m| m.matches(device)))
            .collect();
        let mut run = CascadeRun {
            engine: self,
            doc,
            device,
            active,
            caches: SelectorCaches::default(),
            share: HashMap::new(),
            map: StyleMap {
                styles: vec![None; doc.len()],
                unique_styles: 0,
            },
            root_font_size: crate::style::DEFAULT_FONT_SIZE,
        };
        let initial = Arc::new(ComputedStyle::initial());
        run.visit(doc.root(), &initial);
        run.map
    }
}

type ShareKey = (usize, Vec<(u32, u32)>, Option<Box<str>>);

struct CascadeRun<'a> {
    engine: &'a CascadeEngine,
    doc: &'a Document,
    device: &'a Device,
    active: Vec<bool>,
    caches: SelectorCaches,
    share: HashMap<ShareKey, Arc<ComputedStyle>>,
    map: StyleMap,
    root_font_size: f32,
}

impl CascadeRun<'_> {
    /// 先行順で木をたどる。display:none の子孫はスタイル計算を省く。
    fn visit(&mut self, root: NodeId, root_parent: &Arc<ComputedStyle>) {
        let mut stack = vec![(root, root_parent.clone())];
        while let Some((node, parent_style)) = stack.pop() {
            let style = if self.doc.element(node).is_some() {
                let style = self.style_for(node, &parent_style);
                self.map.styles[node.index()] = Some(style.clone());
                if style.display == Display::None {
                    continue;
                }
                style
            } else {
                parent_style
            };
            let children: Vec<NodeId> = self.doc.children(node).collect();
            for child in children.into_iter().rev() {
                stack.push((child, style.clone()));
            }
        }
    }

    fn style_for(&mut self, node: NodeId, parent: &Arc<ComputedStyle>) -> Arc<ComputedStyle> {
        let matched = self.matched_rules(node);
        let element = self.doc.element(node).expect("要素");
        let inline = element.attr("style").map(Box::from);
        let hints = presentational_hints(element);
        let mut key_extra = inline.clone();
        if !hints.is_empty() {
            let mut s = key_extra.map(String::from).unwrap_or_default();
            s.push('\u{0}');
            s.push_str(&format!("{hints:?}"));
            key_extra = Some(s.into_boxed_str());
        }
        let is_root = self.doc.parent(node) == Some(self.doc.root());
        let key: ShareKey = (Arc::as_ptr(parent) as usize, matched.clone(), key_extra);
        if !is_root && let Some(shared) = self.share.get(&key) {
            return shared.clone();
        }
        let inline_block = inline.as_deref().map(parse_declaration_block);
        let style =
            Arc::new(self.cascade(&matched, &hints, inline_block.as_ref(), parent, is_root));
        if is_root {
            self.root_font_size = style.font_size;
        }
        self.map.unique_styles += 1;
        self.share.insert(key, style.clone());
        style
    }

    /// 一致した (規則番号, 一致セレクタの最大詳細度) を規則番号の昇順で返す。
    fn matched_rules(&mut self, node: NodeId) -> Vec<(u32, u32)> {
        let element = ElementRef::new(self.doc, node);
        let data = self.doc.element(node).expect("要素");
        let mut ctx = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut self.caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        let mut matched: Vec<(u32, u32)> = self
            .engine
            .index
            .candidates(data)
            .filter(|e| self.active[e.rule as usize])
            .filter(|e| {
                let (_, rule) = &self.engine.rules[e.rule as usize];
                let selector = &rule.selectors.slice()[e.selector as usize];
                matches_selector(selector, 0, None, &element, &mut ctx)
            })
            .map(|e| (e.rule, e.specificity))
            .collect();
        // 同じ規則の複数セレクタが一致したら最大の詳細度を残す
        matched.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        matched.dedup_by_key(|m| m.0);
        matched
    }

    fn cascade(
        &self,
        matched: &[(u32, u32)],
        hints: &[Decl],
        inline: Option<&DeclarationBlock>,
        parent: &ComputedStyle,
        is_root: bool,
    ) -> ComputedStyle {
        let decls = self.ordered_decls(matched, hints, inline);
        let mut style = ComputedStyle::inherit_from(parent);
        let ctx = ResolveContext {
            parent,
            root_font_size: if is_root {
                parent.font_size
            } else {
                self.root_font_size
            },
            viewport_width: self.device.width,
            viewport_height: self.device.height,
        };
        apply_custom_properties(&mut style, &decls);
        // em の基準と currentcolor を先に確定させる
        let is_color = |d: &Decl| match d {
            Decl::Color(_) => true,
            Decl::Unparsed(name, _) => &**name == "color",
            _ => false,
        };
        let phases: [&dyn Fn(&Decl) -> bool; 3] = [&Decl::is_font_size, &is_color, &|d| {
            !d.is_font_size() && !is_color(d)
        }];
        for phase in phases {
            for decl in decls.iter().filter(|d| phase(d)) {
                apply_with_vars(&mut style, decl, &ctx);
            }
        }
        fixup(&mut style, is_root);
        style
    }

    fn ordered_decls<'d>(
        &'d self,
        matched: &[(u32, u32)],
        hints: &'d [Decl],
        inline: Option<&'d DeclarationBlock>,
    ) -> Vec<&'d Decl> {
        let mut list: Vec<Applicable<'d>> = Vec::new();
        for &(rule_id, spec) in matched {
            let (origin, rule) = &self.engine.rules[rule_id as usize];
            let (normal_level, important_level) = match origin {
                Origin::UserAgent => (0, 6),
                Origin::Author => (2, 4),
            };
            push_block(
                &mut list,
                &rule.block,
                normal_level,
                important_level,
                spec,
                rule_id,
            );
        }
        list.extend(hints.iter().map(|decl| Applicable {
            key: (1, 0, 0),
            decl,
        }));
        if let Some(block) = inline {
            push_block(&mut list, block, 3, 5, u32::MAX, u32::MAX);
        }
        // 安定ソート：同じ優先度なら出現順（後勝ち）
        list.sort_by_key(|a| a.key);
        list.into_iter().map(|a| a.decl).collect()
    }
}

fn push_block<'d>(
    list: &mut Vec<Applicable<'d>>,
    block: &'d DeclarationBlock,
    normal: u8,
    important: u8,
    spec: u32,
    order: u32,
) {
    list.extend(block.normal.iter().map(|decl| Applicable {
        key: (normal, spec, order),
        decl,
    }));
    list.extend(block.important.iter().map(|decl| Applicable {
        key: (important, spec, order),
        decl,
    }));
}

/// カスタムプロパティを適用する（変更がある場合だけマップを複製）。
/// カスタムプロパティを適用する（変更がある場合だけマップを複製）。
///
/// まず各名前の勝ち値（カスケード順で最後のもの）を未解決のまま表に載せ、
/// その表に対して置換する。`var()` は参照先を再帰的に展開するので、宣言の並び順ではなく
/// 依存関係の順で解決される。循環参照は置換の深さ上限で無効（未定義）になる。
fn apply_custom_properties(style: &mut ComputedStyle, decls: &[&Decl]) {
    let mut winners: Vec<(&LocalName, &Arc<str>)> = Vec::new();
    for decl in decls {
        if let Decl::Custom(name, value) = decl {
            match winners.iter_mut().find(|(n, _)| *n == name) {
                Some(slot) => slot.1 = value,
                None => winners.push((name, value)),
            }
        }
    }
    if winners.is_empty() {
        return;
    }
    let map = Arc::make_mut(&mut style.custom);
    for &(name, value) in &winners {
        map.insert(name.clone(), value.clone());
    }
    let unresolved: Vec<(LocalName, Arc<str>)> = winners
        .iter()
        .filter(|(_, v)| v.contains("var("))
        .map(|&(n, v)| (n.clone(), v.clone()))
        .collect();
    let resolved: Vec<(LocalName, Option<Arc<str>>)> = unresolved
        .iter()
        .map(|(name, value)| (name.clone(), substitute(value, map).map(Arc::from)))
        .collect();
    for (name, value) in resolved {
        match value {
            Some(v) => map.insert(name, v),
            None => map.remove(&name),
        };
    }
}

fn apply_with_vars(style: &mut ComputedStyle, decl: &Decl, ctx: &ResolveContext<'_>) {
    let Decl::Unparsed(name, css) = decl else {
        apply_decl(style, decl, ctx);
        return;
    };
    let Some(css) = substitute(css, &style.custom) else {
        return; // 計算値時点で無効 → 継承値/初期値のまま
    };
    for decl in parse_property(name, &css).unwrap_or_default() {
        if !matches!(decl, Decl::Unparsed(..)) {
            apply_decl(style, &decl, ctx);
        }
    }
}

/// CSS 2.1 §9.7 のブロック化などの後処理。
fn fixup(style: &mut ComputedStyle, is_root: bool) {
    let out_of_flow = matches!(style.position, Position::Absolute | Position::Fixed);
    if out_of_flow {
        style.float = Float::None;
    }
    if is_root || out_of_flow || style.float != Float::None {
        style.display = style.display.blockify();
    }
}
