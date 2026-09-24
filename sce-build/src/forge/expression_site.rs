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
    /// Where each piece of `source` was written, when a pass assembled it
    /// from pieces authors wrote in several places.
    pub splices: Option<&'a SpliceMap>,
}

/// An expression a pass assembled from text written in several places, and
/// where each piece of it was written.
///
/// ⚠ `cycle_expand` splices each `cycle_*` call into conditionals built from
/// the host expression and the cycle's `<sce:step when>` conditions. The
/// field keeps the spelling of what its author wrote, which no longer spells
/// the expanded text, so a refusal of it was placed nowhere at all — `conut`
/// beside a `cycle_next(…)`, or `normalOnn` in a step's `when`, reported with
/// a file and no row (measured 2026-09-24). With this map a range read back
/// onto one written piece is placed where that piece was written.
#[derive(Debug, Clone, Default)]
pub struct SpliceMap {
    /// The assembled text this map describes.
    pub text: String,
    /// Every text a piece was copied from; the first is the host expression.
    pub sources: Vec<SplicedSource>,
    /// `text` in order, piece by piece.
    pub pieces: Vec<SplicedPiece>,
}

/// A text pieces of a [`SpliceMap`] were copied from, and the attribute it
/// was read from as written.
#[derive(Debug, Clone)]
pub struct SplicedSource {
    pub text: String,
    pub spelling: Option<AttributeSpelling>,
}

/// A range of a [`SpliceMap`]'s text, and where it was copied from: a
/// source's index and the offset in that source's text it starts at. `None`
/// for text the pass wrote itself, which nobody wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplicedPiece {
    pub out: Range<usize>,
    pub from: Option<(usize, usize)>,
}

impl SpliceMap {
    /// Where `range` of [`Self::text`] was written: the source it was copied
    /// from and the range of that source's text, when one written piece
    /// holds all of it. An empty range at the end of a piece is that piece's.
    pub(crate) fn written(&self, range: Range<usize>) -> Option<(&SplicedSource, Range<usize>)> {
        let range = self.without_grouping(range);
        let piece = self.pieces.iter().find(|piece| {
            piece.out.start <= range.start
                && range.end <= piece.out.end
                && (range.start < piece.out.end || range.is_empty())
        })?;
        let (source, at) = piece.from?;
        let shift = |offset: usize| offset - piece.out.start + at;
        Some((
            self.sources.get(source)?,
            shift(range.start)..shift(range.end),
        ))
    }

    /// `range` without the grouping the pass wrote around a copied piece:
    /// parentheses and space at either end that no source holds.
    ///
    /// ⚠ The parser gives a parenthesised operand the span of its
    /// parentheses, and the pass parenthesises every piece it copies for
    /// precedence — so a step's `when` of `normalOnn`, refused, arrived as
    /// the range of `(normalOnn)`, which spans text nobody wrote on both
    /// sides and so read back onto no source. Only text the pass wrote is
    /// shed: a parenthesis an author wrote belongs to a source and stays.
    fn without_grouping(&self, range: Range<usize>) -> Range<usize> {
        let glue = |at: usize| {
            self.pieces
                .iter()
                .any(|piece| piece.out.contains(&at) && piece.from.is_none())
        };
        let byte = |at: usize| self.text.as_bytes().get(at).copied();
        let (mut start, mut end) = (range.start, range.end);
        while start < end && glue(start) && matches!(byte(start), Some(b'(' | b' ')) {
            start += 1;
        }
        while start < end && glue(end - 1) && matches!(byte(end - 1), Some(b')' | b' ')) {
            end -= 1;
        }
        start..end
    }
}

/// Where a range of an expression was written: its row and column, and the
/// text that produced it when the range reads back exactly.
#[derive(Debug, Clone, Copy, Default)]
pub struct WrittenAt<'a> {
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub written: Option<Written<'a>>,
}

impl<'a> WrittenAt<'a> {
    /// The first character of `spelling`'s attribute — where a refusal about
    /// the attribute as a WHOLE is placed, rather than about a range of an
    /// expression read from it (how many arguments `args` holds, say).
    /// Nowhere without an attribute.
    pub fn attribute(spelling: Option<&'a AttributeSpelling>) -> Self {
        spelling.map_or_else(Self::default, |spelling| Self {
            line: Some(spelling.row()),
            col: Some(spelling.col()),
            written: None,
        })
    }

    /// The whole value of `spelling`'s attribute as written — where a
    /// refusal of that value is placed, with the text it reports. At the
    /// attribute's first character, reporting nothing, when the value has no
    /// written counterpart; nowhere without an attribute.
    pub fn value(spelling: Option<&'a AttributeSpelling>) -> Self {
        match spelling.map(|spelling| (spelling, spelling.value())) {
            Some((_, Some(written))) => Self {
                line: Some(written.row),
                col: Some(written.col),
                written: Some(written),
            },
            Some((spelling, None)) => Self::attribute(Some(spelling)),
            None => Self::default(),
        }
    }
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

    /// `error`, placed here and reporting the text written here as its
    /// `actual` — for a refusal whose `actual` is the decoded value of
    /// exactly what this names, so the record carries what the row spells
    /// (`&lt;` where the reader decoded `<`).
    pub fn place_reporting(&self, error: ForgeError) -> ForgeError {
        self.place_as(error, self.reported())
    }

    /// What this reports as a record's `actual`: the text as written when it
    /// lies on one row, and nothing at all for an empty range — the end of an
    /// expression, where nothing is written. Text over several rows is
    /// neither, and the payload keeps its own `actual`.
    ///
    /// Visible to the crate for a record that is not a [`ForgeError`] — the
    /// statechart's ECMAScript refusals — so it reports what this reads by
    /// the same rule rather than by a copy of it.
    pub(crate) fn reported(&self) -> Option<AsWritten> {
        self.written
            .and_then(|written| written.on_one_row())
            .map(|text| match text {
                "" => AsWritten::Nothing,
                text => AsWritten::Text(text.to_string()),
            })
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
        Self {
            source,
            spelling,
            splices: None,
        }
    }

    /// This site, with where each piece of an assembled source was written.
    /// `None` leaves it as it is.
    pub fn with_splices(self, splices: Option<&'a SpliceMap>) -> Self {
        Self { splices, ..self }
    }

    /// Where `span` of this expression was written. `span` indexes the
    /// source TRIMMED — how every entry point of the expression pipeline
    /// hands it to the parser — and `None` asks for the attribute itself.
    pub fn locate(&self, span: Option<Range<usize>>) -> WrittenAt<'a> {
        // A map describes exactly the text it was built with; any other
        // text is not the assembly it records.
        if let Some(map) = self.splices.filter(|map| map.text == self.source) {
            return Self::locate_spliced(map, self.source, span);
        }
        // An attribute that does not spell this expression is not where it
        // was written: a pass rewrote the text after it was read.
        let Some(spelling) = self.spelling.filter(|s| s.spells(self.source)) else {
            return WrittenAt::default();
        };
        Self::locate_on(spelling, span)
    }

    /// `span` of `spelling`'s value, trimmed, placed there — at the
    /// attribute's first character when it does not read back.
    fn locate_on(spelling: &'a AttributeSpelling, span: Option<Range<usize>>) -> WrittenAt<'a> {
        match span.and_then(|span| spelling.locate_trimmed(span)) {
            Some(written) => WrittenAt {
                line: Some(written.row),
                col: Some(written.col),
                written: Some(written),
            },
            None => WrittenAt::attribute(Some(spelling)),
        }
    }

    /// `span` of an assembled `source`, placed where its piece was written:
    /// on that source's attribute when one written piece holds all of it,
    /// else at the host expression's attribute — text the pass wrote itself,
    /// or a range across pieces, was written by nobody in one place.
    fn locate_spliced(
        map: &'a SpliceMap,
        source: &str,
        span: Option<Range<usize>>,
    ) -> WrittenAt<'a> {
        let spelled = |spliced: &'a SplicedSource| {
            spliced
                .spelling
                .as_ref()
                .filter(|spelling| spelling.spells(&spliced.text))
        };
        let host = map.sources.first().and_then(spelled);
        let lead = source.len() - source.trim_start().len();
        let written = span.and_then(|span| map.written(span.start + lead..span.end + lead));
        let placed = written.and_then(|(spliced, range)| {
            let spelling = spelled(spliced)?;
            // A range of the source's text, as that text's spelling places
            // a range of it trimmed.
            let lead = spliced.text.len() - spliced.text.trim_start().len();
            let range = range.start.checked_sub(lead)?..range.end.checked_sub(lead)?;
            let written = spelling.locate_trimmed(range)?;
            Some(WrittenAt {
                line: Some(written.row),
                col: Some(written.col),
                written: Some(written),
            })
        });
        placed.unwrap_or_else(|| WrittenAt::attribute(host))
    }

    /// `refusal` of this expression, placed where it was raised — see the
    /// module documentation for the rule.
    pub fn place(&self, refusal: Spanned<ExprError>) -> ForgeError {
        let Spanned {
            error: refusal,
            span,
        } = refusal;
        self.locate(span)
            .place_reporting(ForgeError::Expression(refusal))
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

    /// An assembled expression is placed piece by piece: a range of a piece
    /// copied from the host attribute stands there, a range of a piece
    /// copied from another attribute stands on that one, and a range of text
    /// the pass wrote stands at the host attribute's first character.
    #[test]
    fn an_assembled_expression_is_placed_where_each_piece_was_written() {
        let document = roxmltree::Document::parse(
            "<r>\n  <step when=\"  normalOnn\"/>\n  <data expr=\"f(x) + conut\"/>\n</r>",
        )
        .expect("parses");
        let root = document.root_element();
        let element = |name: &str| {
            root.children()
                .find(|n| n.has_tag_name(name))
                .expect("the element")
        };
        let when = AttributeSpelling::of(&element("step"), None, "when");
        let host = AttributeSpelling::of(&element("data"), None, "expr");
        // `(normalOnn) + conut`: the step's condition, glue, the host's tail.
        let text = "(normalOnn) + conut";
        let map = SpliceMap {
            text: text.to_string(),
            sources: vec![
                SplicedSource {
                    text: "f(x) + conut".to_string(),
                    spelling: host.clone(),
                },
                SplicedSource {
                    text: "normalOnn".to_string(),
                    spelling: when,
                },
            ],
            pieces: vec![
                SplicedPiece {
                    out: 0..1,
                    from: None,
                },
                SplicedPiece {
                    out: 1..10,
                    from: Some((1, 0)),
                },
                SplicedPiece {
                    out: 10..11,
                    from: None,
                },
                SplicedPiece {
                    out: 11..19,
                    from: Some((0, 4)),
                },
            ],
        };
        let site = ExpressionSite::new(text, host.as_ref()).with_splices(Some(&map));
        let at = |span: Range<usize>| {
            let written = site.locate(Some(span));
            (written.line, written.col, written.observed())
        };
        assert_eq!(
            at(1..10),
            (Some(2), Some(17), Some("normalOnn".to_string())),
            "on the step's attribute, past its leading space"
        );
        assert_eq!(
            at(0..11),
            (Some(2), Some(17), Some("normalOnn".to_string())),
            "the parser spans `(normalOnn)`; the pass wrote the parentheses"
        );
        assert_eq!(
            at(14..19),
            (Some(3), Some(22), Some("conut".to_string())),
            "on the host's attribute, where it was written"
        );
        assert_eq!(
            at(10..14),
            (Some(3), Some(15), None),
            "text the pass wrote: the host attribute's first character"
        );
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
