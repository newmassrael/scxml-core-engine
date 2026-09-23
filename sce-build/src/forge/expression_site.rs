// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An expression attribute, as a refusal of the expression is placed
//! against it.
//!
//! # What was wrong
//!
//! A forge expression is judged in full only when it is lowered — the name
//! checks, the grammar and each backend's emitter run inside code
//! generation, after the document has become a model. Their refusals left
//! the generator with a file and nothing else: `celsius + conut` was refused
//! as `expression/unknown-identifier` with `location: {file}` on every
//! backend (measured 2026-09-23), although the model had kept the attribute
//! as written, row and column included, for exactly this
//! ([`crate::attribute_spelling`]). The pipeline knew which token it refused
//! and had no way to say where, and the generator knew where the attribute
//! was and never asked.
//!
//! # The rule
//!
//! The pipeline raises a refusal at the range of the expression it was
//! judging ([`Spanned`]); this module turns that range into the row,
//! the column and the text as the document spells it. One rule, three
//! outcomes, in this order:
//!
//! 1. the range reads back onto the attribute as written — the refusal is
//!    placed at the token, and the token as written becomes its `actual`
//!    (SCE_ERROR_CONTRACT §3.1.1: `&&` is `&amp;&amp;` on that row);
//! 2. it does not, or there is no range — at the attribute's first
//!    character;
//! 3. there is no attribute, because no document produced the model —
//!    nowhere.
//!
//! ⚠ An attribute only counts if it SPELLS the expression. A pass may
//! rewrite a model's text after reading it — `cycle_expand` splices each
//! `cycle_*` call into ordinary conditionals — while the field keeps the
//! spelling of what the author wrote. A range of the rewritten text read
//! back against that spelling lands wherever the offsets happen to fall,
//! so a site whose attribute does not decode to its source is outcome 3:
//! the text refused is text nobody wrote.
//!
//! ⚠ There is no fourth outcome at the row of the ELEMENT. An element's
//! start tag may run over several rows and the attribute may sit on any of
//! them, so its first row is a guess — and a record placed on a row its
//! token is not on is worse than one with no row: §3.1.1 has the consumer
//! search that row, and the search comes back empty.
//!
//! ⚠⚠ The same rule was written out four times before it lived here —
//! `quantity_check`, `validate` twice, `previous_value` — each by hand,
//! and each a place for the next check to copy from. [`ExpressionSite::locate`]
//! is the rule; a check that builds its own refusal reads its row, column
//! and `observed` from there.

use std::ops::Range;

use crate::attribute_spelling::{AttributeSpelling, Written};
use crate::forge::error::{AsWritten, ExprError, ForgeError, Placement, Spanned};

/// An expression as the reader decoded it, and the attribute it was read
/// from as written — `None` for a model no document produced.
#[derive(Debug, Clone, Copy)]
pub struct ExpressionSite<'a> {
    pub source: &'a str,
    pub spelling: Option<&'a AttributeSpelling>,
}

/// Where a range of an expression was written: its row and column, and the
/// text that produced it when the range reads back exactly.
#[derive(Debug, Clone, Copy, Default)]
pub struct WrittenAt<'a> {
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub written: Option<Written<'a>>,
}

impl WrittenAt<'_> {
    /// The text as written, when it lies on the one row a record's `actual`
    /// may occupy.
    pub fn observed(&self) -> Option<String> {
        self.written
            .and_then(|written| written.on_one_row())
            .map(str::to_string)
    }

    /// `error`, placed here — for a refusal a check built itself, whose
    /// payload already reports the text as written ([`Self::observed`]).
    /// As it is, when there is no row to place it on.
    pub fn place(&self, error: ForgeError) -> ForgeError {
        self.place_as(error, None)
    }

    fn place_as(&self, error: ForgeError, as_written: Option<AsWritten>) -> ForgeError {
        match self.line {
            None => error,
            line => error.placed(Placement {
                line,
                col: self.col,
                as_written,
                related: Vec::new(),
            }),
        }
    }
}

impl<'a> ExpressionSite<'a> {
    pub fn new(source: &'a str, spelling: Option<&'a AttributeSpelling>) -> Self {
        Self { source, spelling }
    }

    /// Where `span` of this expression was written. `span` indexes the
    /// source TRIMMED — how every entry point of the expression pipeline
    /// hands it to the parser — and `None` asks for the attribute itself.
    pub fn locate(&self, span: Option<Range<usize>>) -> WrittenAt<'a> {
        // An attribute that does not spell this expression is not where it
        // was written: a pass rewrote the text after it was read.
        let Some(spelling) = self.spelling.filter(|s| s.spells(self.source)) else {
            return WrittenAt::default();
        };
        match span.and_then(|span| spelling.locate_trimmed(span)) {
            Some(written) => WrittenAt {
                line: Some(written.row),
                col: Some(written.col),
                written: Some(written),
            },
            None => WrittenAt {
                line: Some(spelling.row()),
                col: Some(spelling.col()),
                written: None,
            },
        }
    }

    /// `refusal` of this expression, placed where it was raised — see the
    /// module documentation for the rule.
    pub fn place(&self, refusal: Spanned<ExprError>) -> ForgeError {
        let Spanned {
            error: refusal,
            span,
        } = refusal;
        let at = self.locate(span);
        // Text on one row is what the row spells; an empty range is the end
        // of the expression, where nothing is written. Text over several
        // rows is neither, and the payload keeps its own `actual`.
        let as_written =
            at.written
                .and_then(|written| written.on_one_row())
                .map(|text| match text {
                    "" => AsWritten::Nothing,
                    text => AsWritten::Text(text.to_string()),
                });
        at.place_as(ForgeError::Expression(refusal), as_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `refusal` of the `expr` of `element`, placed — and where it went.
    fn placed(element: &str, refusal: Spanned<ExprError>) -> (ForgeError, Placement) {
        let document = roxmltree::Document::parse(element).expect("the fixture parses");
        let node = document.root_element();
        let spelling = AttributeSpelling::of(&node, None, "expr");
        let source = node.attribute("expr").expect("an expr");
        ExpressionSite::new(source, spelling.as_ref())
            .place(refusal)
            .into_positioned()
    }

    fn unknown(name: &str) -> ExprError {
        ExprError::UnknownIdentifier {
            name: name.into(),
            candidates: Vec::new(),
        }
    }

    /// At the token, on the row it sits on — a continued value's second row
    /// here — and spelled as written.
    #[test]
    fn a_refusal_with_a_span_is_placed_at_its_token() {
        let (_, at) = placed(
            "<data\n  expr=\"a +\n    conut\"/>",
            unknown("conut").at(Some(8..13)),
        );
        assert_eq!((at.line, at.col), (Some(3), Some(5)));
        assert_eq!(at.as_written, Some(AsWritten::Text("conut".into())));
    }

    /// The token as the document spells it, which is not the token the
    /// reader decoded: `<` is `&lt;` on the row.
    #[test]
    fn an_escaped_token_is_reported_as_written() {
        let (_, at) = placed(
            "<data expr=\"a &lt; &lt; 1\"/>",
            ExprError::UnexpectedToken { token: "<".into() }.at(Some(4..5)),
        );
        assert_eq!((at.line, at.col), (Some(1), Some(20)));
        assert_eq!(at.as_written, Some(AsWritten::Text("&lt;".into())));
    }

    /// The end of the expression is placed where it is, and nothing is
    /// written there.
    #[test]
    fn a_refusal_at_the_end_is_placed_there_with_nothing_written() {
        let (_, at) = placed(
            "<data expr=\"(a\"/>",
            ExprError::ParseMismatch {
                expected: "')'".into(),
                got: "EOF".into(),
            }
            .at(Some(2..2)),
        );
        assert_eq!((at.line, at.col), (Some(1), Some(15)));
        assert_eq!(at.as_written, Some(AsWritten::Nothing));
    }

    /// A refusal the pipeline raised at no range goes to the attribute's
    /// first character, and claims nothing about what is written there.
    #[test]
    fn a_refusal_without_a_span_is_placed_at_the_attribute() {
        let (_, at) = placed(
            "<data\n  expr=\"\"/>",
            ExprError::Empty { what: "expression" }.at(None),
        );
        assert_eq!((at.line, at.col), (Some(2), Some(9)));
        assert_eq!(at.as_written, None);
    }

    /// Text a pass rewrote after reading it is not what the attribute
    /// spells, so a range of it is placed nowhere rather than wherever its
    /// offsets land in the author's text.
    #[test]
    fn a_rewritten_expression_is_not_placed_against_the_original() {
        let document = roxmltree::Document::parse("<data expr=\"cycle_next(c, v) + conut\"/>")
            .expect("parses");
        let node = document.root_element();
        let spelling = AttributeSpelling::of(&node, None, "expr");
        let rewritten = "(((v) === C.A) ? C.B : (v)) + conut";
        let (_, at) = ExpressionSite::new(rewritten, spelling.as_ref())
            .place(unknown("conut").at(Some(30..35)))
            .into_positioned();
        assert_eq!((at.line, at.col, at.as_written), (None, None, None));
    }

    /// With no document behind the model there is nothing to place against:
    /// the refusal stands, with no row — not the row of some element.
    #[test]
    fn a_model_no_document_produced_places_nothing() {
        let error = ExpressionSite::new("conut", None).place(unknown("conut").at(Some(0..5)));
        assert!(
            matches!(
                &error,
                ForgeError::Expression(ExprError::UnknownIdentifier { .. })
            ),
            "the bare refusal, unwrapped and unplaced: {error:?}"
        );
    }
}
