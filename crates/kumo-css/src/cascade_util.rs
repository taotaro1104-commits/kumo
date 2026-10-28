//! カスケードの補助：カスタムプロパティ・`var()` の適用・後処理・`content` の解決。

use std::sync::Arc;

use kumo_dom::{Document, LocalName, NodeId};

use crate::apply::{ResolveContext, apply_decl};
use crate::decl::{ContentPart, Decl};
use crate::keywords::{Float, Position};
use crate::properties::parse_property;
use crate::style::ComputedStyle;
use crate::var::substitute;

/// カスタムプロパティを適用する（変更がある場合だけマップを複製）。
/// カスタムプロパティを適用する（変更がある場合だけマップを複製）。
///
/// まず各名前の勝ち値（カスケード順で最後のもの）を未解決のまま表に載せ、
/// その表に対して置換する。`var()` は参照先を再帰的に展開するので、宣言の並び順ではなく
/// 依存関係の順で解決される。循環参照は置換の深さ上限で無効（未定義）になる。
pub(crate) fn apply_custom_properties(style: &mut ComputedStyle, decls: &[&Decl]) {
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

pub(crate) fn apply_with_vars(style: &mut ComputedStyle, decl: &Decl, ctx: &ResolveContext<'_>) {
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
pub(crate) fn fixup(style: &mut ComputedStyle, is_root: bool) {
    let out_of_flow = matches!(style.position, Position::Absolute | Position::Fixed);
    if out_of_flow {
        style.float = Float::None;
    }
    if is_root || out_of_flow || style.float != Float::None {
        style.display = style.display.blockify();
    }
}

/// `content` の部品をつないで文字列にする（`attr()` は生成元の要素の属性値）。
pub(crate) fn resolve_content(parts: &[ContentPart], doc: &Document, node: NodeId) -> String {
    let mut out = String::new();
    for part in parts {
        match part {
            ContentPart::Text(text) => out.push_str(text),
            ContentPart::Attr(name) => out.push_str(doc.attr(node, name).unwrap_or("")),
        }
    }
    out
}
