//! カスケード：規則のマッチ → 優先順位で並べる → 計算値を作る。
//! 一致した規則集合と親スタイルが同じ要素は計算値を `Arc` で共有する。

use std::collections::HashMap;
use std::sync::Arc;

use kumo_dom::{Document, NodeId};
use selectors::context::{
    MatchingContext, MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode,
    SelectorCaches,
};
use selectors::matching::matches_selector;

use crate::apply::ResolveContext;
use crate::cascade_util::{apply_custom_properties, apply_with_vars, fixup, resolve_content};
use crate::decl::{Decl, DeclarationBlock};
use crate::element::ElementRef;
use crate::hints::presentational_hints;
use crate::index::{IndexEntry, RuleIndex};
use crate::keywords::Display;
use crate::media::Device;
use crate::selector::PseudoElement;
use crate::style::ComputedStyle;
use crate::stylesheet::{StyleRule, Stylesheet, parse_declaration_block};

/// 要素ごとの計算値。テキストノード等は `None`（親のスタイルを使う）。
#[derive(Debug, Default)]
pub struct StyleMap {
    styles: Vec<Option<Arc<ComputedStyle>>>,
    /// 疑似要素 ::before / ::after の計算値と、`content` を解決した文字列（キーは 要素の添字 * 2 + (after なら 1)）
    pseudo: HashMap<usize, PseudoStyle>,
    /// 共有されずに新規作成された計算値の数（計測用）
    pub unique_styles: usize,
}

/// 疑似要素の計算値と生成された文字列。
pub type PseudoStyle = (Arc<ComputedStyle>, Arc<str>);

impl StyleMap {
    /// 要素の ::before（`after` が偽）または ::after が生成するもの。
    pub fn pseudo(&self, node: NodeId, after: bool) -> Option<&PseudoStyle> {
        if self.pseudo.is_empty() {
            return None;
        }
        self.pseudo.get(&(node.index() * 2 + usize::from(after)))
    }

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
    /// ::before（0）と ::after（1）の規則
    pseudo: [RuleIndex; 2],
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
        let mut pseudo = [RuleIndex::default(), RuleIndex::default()];
        for (origin, sheet) in sheets {
            for rule in sheet.rules {
                let rule_id = u32::try_from(rules.len()).expect("規則数が多すぎる");
                for (i, selector) in rule.selectors.slice().iter().enumerate() {
                    let entry = IndexEntry {
                        rule: rule_id,
                        selector: u16::try_from(i).unwrap_or(u16::MAX),
                        specificity: selector.specificity(),
                    };
                    match selector.pseudo_element() {
                        None => index.insert(selector, entry),
                        Some(PseudoElement::Before) => pseudo[0].insert(selector, entry),
                        Some(PseudoElement::After) => pseudo[1].insert(selector, entry),
                        Some(PseudoElement::Other) => {} // ::selection など：描画に関わらない
                    }
                }
                rules.push((origin, rule));
            }
        }
        Self {
            rules,
            index,
            pseudo,
        }
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
                pseudo: HashMap::new(),
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
                self.generate_pseudo(node, &style);
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

    /// ::before と ::after の計算値を作る（`content` が文字列を生成するものだけ）。
    fn generate_pseudo(&mut self, node: NodeId, style: &Arc<ComputedStyle>) {
        for after in [false, true] {
            let which = usize::from(after);
            let engine = self.engine;
            if engine.pseudo[which].is_empty() {
                continue;
            }
            let matched = self.matched_rules(node, Some(which));
            if matched.is_empty() {
                continue;
            }
            let computed = Arc::new(self.cascade(&matched, &[], None, style, false));
            let Some(parts) = computed.content.clone() else {
                continue;
            };
            let text = resolve_content(&parts, self.doc, node);
            self.map
                .pseudo
                .insert(node.index() * 2 + which, (computed, Arc::from(text)));
        }
    }

    fn style_for(&mut self, node: NodeId, parent: &Arc<ComputedStyle>) -> Arc<ComputedStyle> {
        let matched = self.matched_rules(node, None);
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
    fn matched_rules(&mut self, node: NodeId, pseudo: Option<usize>) -> Vec<(u32, u32)> {
        let element = ElementRef::new(self.doc, node);
        let data = self.doc.element(node).expect("要素");
        // 疑似要素の規則は、生成元の要素に対して「疑似要素を除いた部分」を照合する
        let mode = if pseudo.is_some() {
            MatchingMode::ForStatelessPseudoElement
        } else {
            MatchingMode::Normal
        };
        let mut ctx = MatchingContext::new(
            mode,
            None,
            &mut self.caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        let index = match pseudo {
            Some(which) => &self.engine.pseudo[which],
            None => &self.engine.index,
        };
        let mut matched: Vec<(u32, u32)> = index
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
