// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The picture a kind is drawn as, beside its field table.
//!
//! A statechart is boxes and arrows; a lookup is a table, a buffer pool a
//! row of slots, an interpolation a curve, a codec a byte layout. Each kind
//! that has a picture is drawn here from the values of its typed model, so
//! the picture and the field table ([`super::fields`]) read one model and
//! cannot disagree about it.
//!
//! # What a picture promises
//!
//! A picture is an AID to reading, never the reading: the field table is
//! the one that is total. So a picture draws only what the model states —
//! a shape the document does not give (the curve between two points of a
//! method this module does not know) is not drawn, and a picture that has
//! nothing true to say about a document is not made. Nothing is shrunk
//! below the page's minimum type size, and a picture that does not fit is
//! refused with its measured size ([`super::canvas::Canvas::finish`]).
//!
//! # A new kind
//!
//! [`pictures`] matches the document kind without a wildcard, so a kind
//! added to the model does not compile until someone has decided whether it
//! has a picture. A kind with none says so in its arm.

use super::canvas::Canvas;
use super::fields::inline_text;
use super::fit::{Page, Refusal};
use super::metrics::Face;
use super::sheet::{Ink, Sheet};
use super::tree::{self, Node};
use super::words::{self, Phrase};
use crate::forge::model::ForgeDocument;
use crate::forge::page::Lexicon;

pub mod interpolation;
pub mod slots;

/// One picture of a document: the file it is written to (without its
/// extension) and the sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub stem: &'static str,
    pub sheet: Sheet,
}

/// The pictures of `doc`, in the order they are written; none for a kind
/// that is read from its field table alone.
pub fn pictures(
    doc: &ForgeDocument,
    lexicon: &Lexicon,
    page: Page,
) -> Result<Vec<Picture>, Refusal> {
    match doc {
        // Drawn as boxes and arrows by `fit::print`.
        ForgeDocument::Statechart(_) => Ok(Vec::new()),
        ForgeDocument::BufferPool(m) => slots::buffer_pool(m, lexicon, page),
        ForgeDocument::BoundedCollection(m) => slots::bounded_collection(m, lexicon, page),
        ForgeDocument::Interpolation(m) => interpolation::interpolation(m, lexicon, page),
        // Read from the field table alone, for now.
        ForgeDocument::Transform(_)
        | ForgeDocument::Lookup(_)
        | ForgeDocument::Condition(_)
        | ForgeDocument::Codec(_)
        | ForgeDocument::Validator(_)
        | ForgeDocument::Procedure(_)
        | ForgeDocument::Filter(_)
        | ForgeDocument::Timer(_)
        | ForgeDocument::Observer(_)
        | ForgeDocument::Algorithm(_)
        | ForgeDocument::Link(_)
        | ForgeDocument::Worker(_)
        | ForgeDocument::Enum(_)
        | ForgeDocument::EventSchema(_) => Ok(Vec::new()),
    }
}

/// The word `p` in the page's language.
pub(crate) fn say(lexicon: &Lexicon, p: Phrase) -> Result<&'static str, Refusal> {
    words::phrase(lexicon, p).ok_or(Refusal::Box(super::boxes::BoxError::NoPhrases(
        lexicon.name,
    )))
}

/// The scalar fields of `model` as `(name, value)`, in declaration order,
/// without those named in `drawn` — what a picture says in words beside the
/// part of the model it draws. A field that holds a list or a record is not
/// here: it is in the field table.
pub(crate) fn facts<T: serde::Serialize>(
    model: &T,
    drawn: &[&str],
) -> Result<Vec<(String, String)>, Refusal> {
    let tree = tree::of(model)
        .map_err(|e| Refusal::NotATree(e.to_string()))?
        .without(&super::fields::OMITTED_FIELDS);
    let Node::Record(fields) = tree else {
        return Ok(Vec::new());
    };
    Ok(fields
        .iter()
        .filter(|(k, v)| v.is_inline() && !drawn.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), inline_text(v)))
        .collect())
}

/// `facts` as lines under a picture, keys in a column, from `top` down;
/// the height they take.
pub(crate) fn caption(
    c: &mut Canvas,
    x: f64,
    top: f64,
    facts: &[(String, String)],
) -> Result<f64, Refusal> {
    let line = c.line_height();
    let mut key_width = 0.0f64;
    for (k, _) in facts {
        key_width = key_width.max(c.width_of(Face::Mono, k, c.style().body_pt)?);
    }
    let gap = c.style().body_pt * 1.5;
    for (i, (k, v)) in facts.iter().enumerate() {
        let y = top + i as f64 * line;
        c.text(x, y, k, Face::Mono)?;
        c.text(x + key_width + gap, y, v, Face::Mono)?;
    }
    Ok(facts.len() as f64 * line)
}

/// A measured span from `x0` to `x1` at height `y`, ticked at both ends and
/// labelled above (`above`) or below it.
pub(crate) fn dimension(
    c: &mut Canvas,
    (x0, x1): (f64, f64),
    y: f64,
    label: &str,
    above: bool,
) -> Result<(), Refusal> {
    let tick = c.style().body_pt * 0.4;
    c.stroke((x0, y), (x1, y), Ink::Black, false);
    c.stroke((x0, y - tick), (x0, y + tick), Ink::Black, false);
    c.stroke((x1, y - tick), (x1, y + tick), Ink::Black, false);
    let line = c.line_height();
    let top = if above { y - tick - line } else { y + tick };
    c.text_centered((x0 + x1) / 2.0, top, label, Face::Proportional)
}
