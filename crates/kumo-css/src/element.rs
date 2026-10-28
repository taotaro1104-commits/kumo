//! アリーナDOM上の要素を selectors の `Element` として見せるアダプタ。

use std::fmt;

use kumo_dom::{Document, ElementData, NodeId, NodeKind, ns};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::bloom::BloomFilter;
use selectors::matching::{ElementSelectorFlags, MatchingContext};
use selectors::{Element, OpaqueElement};

use crate::selector::{
    AttrValue, CssAtom, CssNamespace, KumoSelectors, PseudoClass, PseudoElement,
};

impl AsRef<str> for AttrValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// `Document` 内の要素ノードへの軽量参照。
#[derive(Clone, Copy)]
pub struct ElementRef<'a> {
    pub doc: &'a Document,
    pub id: NodeId,
}

impl fmt::Debug for ElementRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ElementRef({:?})", self.id)
    }
}

impl<'a> ElementRef<'a> {
    pub fn new(doc: &'a Document, id: NodeId) -> Self {
        debug_assert!(doc.element(id).is_some());
        Self { doc, id }
    }

    fn data(&self) -> &'a ElementData {
        self.doc.element(self.id).expect("ElementRef は要素を指す")
    }

    fn wrap(&self, id: Option<NodeId>) -> Option<Self> {
        id.map(|id| Self { doc: self.doc, id })
    }

    fn sibling_element(&self, forward: bool) -> Option<Self> {
        let step = |n| {
            if forward {
                self.doc.next_sibling(n)
            } else {
                self.doc.prev_sibling(n)
            }
        };
        let mut node = step(self.id);
        while let Some(n) = node {
            if self.doc.element(n).is_some() {
                return self.wrap(Some(n));
            }
            node = step(n);
        }
        None
    }
}

impl Element for ElementRef<'_> {
    type Impl = KumoSelectors;

    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.doc.node(self.id))
    }

    fn parent_element(&self) -> Option<Self> {
        let parent = self.doc.parent(self.id)?;
        self.doc.element(parent)?;
        self.wrap(Some(parent))
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.sibling_element(false)
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.sibling_element(true)
    }

    fn first_element_child(&self) -> Option<Self> {
        let first = self
            .doc
            .children(self.id)
            .find(|&n| self.doc.element(n).is_some());
        self.wrap(first)
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.data().ns == ns!(html)
    }

    fn has_local_name(&self, local_name: &kumo_dom::LocalName) -> bool {
        self.data().local == *local_name
    }

    fn has_namespace(&self, ns: &kumo_dom::Namespace) -> bool {
        self.data().ns == *ns
    }

    fn is_same_type(&self, other: &Self) -> bool {
        let (a, b) = (self.data(), other.data());
        a.local == b.local && a.ns == b.ns
    }

    fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&CssNamespace>,
        local_name: &CssAtom,
        operation: &AttrSelectorOperation<&AttrValue>,
    ) -> bool {
        self.data()
            .attrs
            .iter()
            .find(|a| a.name == local_name.0)
            .is_some_and(|a| operation.eval_str(&a.value))
    }

    fn match_non_ts_pseudo_class(
        &self,
        pc: &PseudoClass,
        _context: &mut MatchingContext<KumoSelectors>,
    ) -> bool {
        match pc {
            PseudoClass::Link | PseudoClass::AnyLink => self.is_link(),
            PseudoClass::Enabled => is_form_control(self.data()),
            PseudoClass::Checked => {
                is_form_control(self.data()) && self.data().attr("checked").is_some()
            }
            PseudoClass::Disabled => self.data().attr("disabled").is_some(),
            _ => false,
        }
    }

    fn match_pseudo_element(
        &self,
        pe: &PseudoElement,
        _context: &mut MatchingContext<KumoSelectors>,
    ) -> bool {
        // 疑似要素のセレクタは、生成元の要素に対して照合する（::before / ::after だけ。ほかは作らない）
        matches!(pe, PseudoElement::Before | PseudoElement::After)
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}

    fn is_link(&self) -> bool {
        let data = self.data();
        (data.is_html("a") || data.is_html("area")) && data.attr("href").is_some()
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(&self, id: &CssAtom, case_sensitivity: CaseSensitivity) -> bool {
        self.data()
            .id()
            .is_some_and(|v| case_sensitivity.eq(v.as_bytes(), id.0.as_bytes()))
    }

    fn has_class(&self, name: &CssAtom, case_sensitivity: CaseSensitivity) -> bool {
        self.data()
            .classes()
            .any(|c| case_sensitivity.eq(c.as_bytes(), name.0.as_bytes()))
    }

    fn has_custom_state(&self, _name: &CssAtom) -> bool {
        false
    }

    fn imported_part(&self, _name: &CssAtom) -> Option<CssAtom> {
        None
    }

    fn is_part(&self, _name: &CssAtom) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        self.doc
            .children(self.id)
            .all(|n| match self.doc.node(n).kind() {
                NodeKind::Element(_) => false,
                NodeKind::Text(_) => self.doc.text(n).is_some_and(str::is_empty),
                _ => true,
            })
    }

    fn is_root(&self) -> bool {
        self.doc
            .parent(self.id)
            .is_some_and(|p| self.doc.node(p).kind() == NodeKind::Document)
    }

    fn add_element_unique_hashes(&self, _filter: &mut BloomFilter) -> bool {
        false
    }
}

fn is_form_control(data: &ElementData) -> bool {
    ["input", "button", "select", "textarea", "option"]
        .iter()
        .any(|t| data.is_html(t))
}
