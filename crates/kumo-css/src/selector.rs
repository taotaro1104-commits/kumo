//! `selectors` crate を Kumo の型で具体化する（SelectorImpl と Parser）。

use std::borrow::Borrow;
use std::fmt;

use cssparser::{CowRcStr, ParseError, ToCss};
use kumo_dom::{LocalName, Namespace};
use precomputed_hash::PrecomputedHash;
use selectors::parser::{SelectorImpl, SelectorParseErrorKind};

/// selectors が要求する ToCss + From<&str> を満たすためのインターン済み文字列ラッパ。
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CssAtom(pub LocalName);

impl From<&str> for CssAtom {
    fn from(s: &str) -> Self {
        Self(LocalName::from(s))
    }
}

impl ToCss for CssAtom {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        cssparser::serialize_identifier(&self.0, dest)
    }
}

impl PrecomputedHash for CssAtom {
    fn precomputed_hash(&self) -> u32 {
        self.0.precomputed_hash()
    }
}

impl Borrow<LocalName> for CssAtom {
    fn borrow(&self) -> &LocalName {
        &self.0
    }
}

/// 名前空間URL。HTML文書のみを対象にするため比較用途に限る。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CssNamespace(pub Namespace);

impl PrecomputedHash for CssNamespace {
    fn precomputed_hash(&self) -> u32 {
        self.0.precomputed_hash()
    }
}

impl Borrow<Namespace> for CssNamespace {
    fn borrow(&self) -> &Namespace {
        &self.0
    }
}

/// 属性値（`[href$=".pdf"]` の右辺）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttrValue(pub String);

impl From<&str> for AttrValue {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl ToCss for AttrValue {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        cssparser::serialize_string(&self.0, dest)
    }
}

/// 対応する非構造疑似クラス。Phase 0 は静的表示のため状態系は常に不一致。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoClass {
    Link,
    AnyLink,
    Visited,
    Hover,
    Active,
    Focus,
    FocusWithin,
    FocusVisible,
    Checked,
    Disabled,
    Enabled,
    Target,
    Lang,
    Dir,
}

impl ToCss for PseudoClass {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        let name = match self {
            Self::Link => ":link",
            Self::AnyLink => ":any-link",
            Self::Visited => ":visited",
            Self::Hover => ":hover",
            Self::Active => ":active",
            Self::Focus => ":focus",
            Self::FocusWithin => ":focus-within",
            Self::FocusVisible => ":focus-visible",
            Self::Checked => ":checked",
            Self::Disabled => ":disabled",
            Self::Enabled => ":enabled",
            Self::Target => ":target",
            Self::Lang => ":lang()",
            Self::Dir => ":dir()",
        };
        dest.write_str(name)
    }
}

impl selectors::parser::NonTSPseudoClass for PseudoClass {
    fn is_active_or_hover(&self) -> bool {
        matches!(self, Self::Active | Self::Hover)
    }

    fn is_user_action_state(&self) -> bool {
        matches!(self, Self::Active | Self::Hover | Self::Focus)
    }
}

/// 疑似要素。Phase 0 では生成コンテンツを描画しないため、マッチは常に失敗させる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoElement {
    Before,
    After,
    Other,
}

impl ToCss for PseudoElement {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        dest.write_str(match self {
            Self::Before => "::before",
            Self::After => "::after",
            Self::Other => "::-kumo-other",
        })
    }
}

impl selectors::parser::PseudoElement for PseudoElement {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KumoSelectors;

impl SelectorImpl for KumoSelectors {
    type ExtraMatchingData<'a> = ();
    type AttrValue = AttrValue;
    type Identifier = CssAtom;
    type LocalName = CssAtom;
    type NamespaceUrl = CssNamespace;
    type NamespacePrefix = CssAtom;
    type BorrowedNamespaceUrl = Namespace;
    type BorrowedLocalName = LocalName;
    type NonTSPseudoClass = PseudoClass;
    type PseudoElement = PseudoElement;
}

pub type SelectorList = selectors::SelectorList<KumoSelectors>;
pub type Selector = selectors::parser::Selector<KumoSelectors>;

/// セレクタ文字列のパーサ設定。
pub(crate) struct SelectorParser;

impl<'i> selectors::parser::Parser<'i> for SelectorParser {
    type Impl = KumoSelectors;
    type Error = SelectorParseErrorKind;

    fn parse_is_and_where(&self) -> bool {
        true
    }

    fn parse_has(&self) -> bool {
        true
    }

    fn parse_nth_child_of(&self) -> bool {
        true
    }

    fn parse_non_ts_pseudo_class(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<PseudoClass, ParseError<Self::Error>> {
        match_ignore_ascii_case_pc(&name).ok_or_else(unsupported)
    }

    fn parse_non_ts_functional_pseudo_class(
        &self,
        name: CowRcStr<'i>,
        parser: &mut cssparser::Parser<'i>,
        _after_part: bool,
    ) -> Result<PseudoClass, ParseError<Self::Error>> {
        // 引数は読み捨てる（:lang() / :dir() は常に不一致）
        let pc = if name.eq_ignore_ascii_case("lang") {
            PseudoClass::Lang
        } else if name.eq_ignore_ascii_case("dir") {
            PseudoClass::Dir
        } else {
            return Err(unsupported());
        };
        while parser.next().is_ok() {}
        Ok(pc)
    }

    fn parse_pseudo_element(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<PseudoElement, ParseError<Self::Error>> {
        Ok(if name.eq_ignore_ascii_case("before") {
            PseudoElement::Before
        } else if name.eq_ignore_ascii_case("after") {
            PseudoElement::After
        } else {
            PseudoElement::Other
        })
    }

    fn parse_functional_pseudo_element(
        &self,
        _name: CowRcStr<'i>,
        arguments: &mut cssparser::Parser<'i>,
    ) -> Result<PseudoElement, ParseError<Self::Error>> {
        while arguments.next().is_ok() {}
        Ok(PseudoElement::Other)
    }
}

fn unsupported() -> ParseError<SelectorParseErrorKind> {
    ParseError::custom(SelectorParseErrorKind::UnsupportedPseudoClassOrElement)
}

fn match_ignore_ascii_case_pc(name: &str) -> Option<PseudoClass> {
    let table = [
        ("link", PseudoClass::Link),
        ("any-link", PseudoClass::AnyLink),
        ("visited", PseudoClass::Visited),
        ("hover", PseudoClass::Hover),
        ("active", PseudoClass::Active),
        ("focus", PseudoClass::Focus),
        ("focus-within", PseudoClass::FocusWithin),
        ("focus-visible", PseudoClass::FocusVisible),
        ("checked", PseudoClass::Checked),
        ("disabled", PseudoClass::Disabled),
        ("enabled", PseudoClass::Enabled),
        ("target", PseudoClass::Target),
    ];
    table
        .iter()
        .find(|(n, _)| name.eq_ignore_ascii_case(n))
        .map(|&(_, pc)| pc)
}

/// セレクタリストを文字列から解析する。
pub fn parse_selector_list(input: &str) -> Option<SelectorList> {
    let mut parser = cssparser::Parser::new(input);
    parse_selector_list_from(&mut parser).ok()
}

pub(crate) fn parse_selector_list_from<'i>(
    parser: &mut cssparser::Parser<'i>,
) -> Result<SelectorList, ParseError<SelectorParseErrorKind>> {
    SelectorList::parse(
        &SelectorParser,
        parser,
        selectors::parser::ParseRelative::No,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ToCss;

    #[test]
    fn parses_supported_selectors() {
        for s in [
            "a:link, a:visited, a:hover, a:active, a:focus",
            "input:checked, input:disabled, input:enabled, :focus-within, :focus-visible",
            ":any-link, :target, p:lang(ja), p:dir(ltr)",
            "p::before, p::after, p::marker, ::highlight(x)",
            ":is(h1, h2) > a:not(.x), li:nth-child(2n+1 of .y), div:has(> img)",
            "[href^='http'][data-x~=y]",
        ] {
            assert!(parse_selector_list(s).is_some(), "{s}");
        }
        assert!(parse_selector_list(":unknown-thing").is_none());
        assert!(parse_selector_list(":nope(1)").is_none());
    }

    #[test]
    fn serializes_for_debugging() {
        let mut out = String::new();
        for pc in [
            PseudoClass::Link,
            PseudoClass::Hover,
            PseudoClass::Lang,
            PseudoClass::Dir,
            PseudoClass::Target,
        ] {
            pc.to_css(&mut out).unwrap();
        }
        PseudoElement::Before.to_css(&mut out).unwrap();
        PseudoElement::Other.to_css(&mut out).unwrap();
        CssAtom::from("a b").to_css(&mut out).unwrap();
        AttrValue::from("v").to_css(&mut out).unwrap();
        assert!(out.contains(":link") && out.contains("::before") && out.contains("\"v\""));
        assert!(selectors::parser::NonTSPseudoClass::is_active_or_hover(
            &PseudoClass::Hover
        ));
        assert!(!selectors::parser::NonTSPseudoClass::is_user_action_state(
            &PseudoClass::Link
        ));
    }
}
