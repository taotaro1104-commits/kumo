//! Kumo の最小CSSエンジン（パース・セレクタマッチ・カスケード・計算値）。
//!
//! 設計書では Stylo を採用候補としているが、Phase 0 では組み込みコストを避けて
//! 自前の最小カスケードを使う。外部からは [`StyleEngine`] トレイト越しに使うため、
//! 後から Stylo 実装に差し替えられる。
#![forbid(unsafe_code)]

mod apply;
mod cascade;
mod cascade_util;
mod color;
mod computed;
mod decl;
mod element;
mod hints;
mod index;
pub mod keywords;
mod media;
mod properties;
pub mod query;
mod selector;
mod shorthands;
pub mod style;
mod stylesheet;
pub mod values;
mod var;

pub use cascade::{CascadeEngine, StyleEngine, StyleMap};
pub use computed::{COMPUTED_PROPERTIES, computed_value};
pub use decl::{ContentPart, Decl, DeclarationBlock};
pub use element::ElementRef;
pub use media::{Device, MediaQueryList};
pub use properties::parse_property;
pub use selector::{SelectorList, parse_selector_list};
pub use style::{ComputedStyle, LengthPct, LengthPctAuto, LineHeight, Side, Sides};
pub use stylesheet::{StyleRule, Stylesheet, parse_declaration_block};
