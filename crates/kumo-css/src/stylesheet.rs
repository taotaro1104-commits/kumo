//! スタイルシートのパース（スタイル規則・@media・@supports・@layer）。

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, Delimiter, ParseError, Parser, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser,
};

use crate::decl::DeclarationBlock;
use crate::media::MediaQueryList;
use crate::properties::parse_property;
use crate::selector::{SelectorList, parse_selector_list_from};

/// スタイル規則（セレクタ + 宣言ブロック）。`media` は外側の @media 条件。
#[derive(Debug)]
pub struct StyleRule {
    pub selectors: SelectorList,
    pub block: DeclarationBlock,
    pub media: Option<std::sync::Arc<MediaQueryList>>,
}

/// 解析済みスタイルシート。規則はソース順に平坦化して保持する。
#[derive(Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<StyleRule>,
}

impl Stylesheet {
    pub fn parse(css: &str) -> Self {
        let mut rules = Vec::new();
        let mut parser = Parser::new(css);
        let mut top = TopLevelParser {
            rules: &mut rules,
            media: None,
        };
        for _ in StyleSheetParser::new(&mut parser, &mut top) {}
        Self { rules }
    }
}

/// `style` 属性などの宣言列を解析する。
pub fn parse_declaration_block(css: &str) -> DeclarationBlock {
    let mut parser = Parser::new(css);
    parse_declarations(&mut parser)
}

fn parse_declarations(input: &mut Parser<'_>) -> DeclarationBlock {
    let mut block = DeclarationBlock::default();
    let mut decl_parser = DeclParser;
    for (decls, important) in RuleBodyParser::new(input, &mut decl_parser).flatten() {
        let target = if important {
            &mut block.important
        } else {
            &mut block.normal
        };
        target.extend(decls);
    }
    block
}

type Error = ParseError<()>;

enum AtPrelude {
    Media(MediaQueryList),
    Transparent,
}

struct TopLevelParser<'a> {
    rules: &'a mut Vec<StyleRule>,
    media: Option<std::sync::Arc<MediaQueryList>>,
}

impl<'i> QualifiedRuleParser<'i> for TopLevelParser<'_> {
    type Prelude = SelectorList;
    type QualifiedRule = ();
    type Error = ();

    fn parse_prelude(&mut self, input: &mut Parser<'i>) -> Result<SelectorList, Error> {
        parse_selector_list_from(input).map_err(|_| ParseError::custom(()))
    }

    fn parse_block(
        &mut self,
        selectors: SelectorList,
        _start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), Error> {
        let block = parse_declarations(input);
        if !block.is_empty() {
            self.rules.push(StyleRule {
                selectors,
                block,
                media: self.media.clone(),
            });
        }
        Ok(())
    }
}

impl<'i> AtRuleParser<'i> for TopLevelParser<'_> {
    type Prelude = AtPrelude;
    type AtRule = ();
    type Error = ();

    fn parse_prelude(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i>,
    ) -> Result<AtPrelude, Error> {
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "media" => Ok(AtPrelude::Media(MediaQueryList::parse(input))),
            // 条件は評価せず中身を通常規則として扱う（Phase 0 の近似）
            "supports" | "layer" | "container" | "scope" | "document" | "-moz-document" => {
                while input.next().is_ok() {}
                Ok(AtPrelude::Transparent)
            }
            _ => Err(ParseError::custom(())),
        }
    }

    fn rule_without_block(&mut self, _prelude: AtPrelude, _start: &ParserState) -> Result<(), ()> {
        Ok(()) // `@layer a, b;` など
    }

    fn parse_block(
        &mut self,
        prelude: AtPrelude,
        _start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), Error> {
        let media = match prelude {
            AtPrelude::Media(list) => Some(std::sync::Arc::new(match &self.media {
                Some(outer) => outer.and(&list),
                None => list,
            })),
            AtPrelude::Transparent => self.media.clone(),
        };
        let mut nested = TopLevelParser {
            rules: self.rules,
            media,
        };
        for _ in RuleBodyParser::new(input, &mut nested) {}
        Ok(())
    }
}

impl<'i> DeclarationParser<'i> for TopLevelParser<'_> {
    type Declaration = ();
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, (), ()> for TopLevelParser<'_> {
    fn parse_declarations(&self) -> bool {
        false
    }

    fn parse_qualified(&self) -> bool {
        true
    }
}

/// 宣言パーサ。値は生文字列で切り出して `parse_property` に渡す。
struct DeclParser;

type ParsedDecl = (Vec<crate::decl::Decl>, bool);

impl<'i> DeclarationParser<'i> for DeclParser {
    type Declaration = ParsedDecl;
    type Error = ();

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i>,
        _start: &ParserState,
    ) -> Result<ParsedDecl, Error> {
        let start = input.position();
        input.parse_until_before(Delimiter::Bang, |i| {
            while i.next().is_ok() {}
            Ok::<_, Error>(())
        })?;
        let value = input.slice_from(start);
        let important = input.try_parse(cssparser::parse_important).is_ok();
        input.expect_exhausted()?;
        parse_property(&name, value)
            .map(|decls| (decls, important))
            .ok_or_else(|| ParseError::custom(()))
    }
}

impl<'i> AtRuleParser<'i> for DeclParser {
    type Prelude = ();
    type AtRule = ParsedDecl;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for DeclParser {
    type Prelude = ();
    type QualifiedRule = ParsedDecl;
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, ParsedDecl, ()> for DeclParser {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        false // CSS Nesting は Phase 0 では未対応
    }
}
