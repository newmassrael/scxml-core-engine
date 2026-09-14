// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! §scxml-3.3.1 — the grammar W3C gives an identifier-bearing attribute,
//! checked on the attribute itself.
//!
//! # What was wrong
//!
//! W3C types a state's `id` as an XML Schema `ID` (§3.3.1, §3.4.1, §3.7.1 —
//! *"A valid id as defined in [XML Schema]"*) and an event name as
//! dot-separated tokens (§3.12.1). SCE enforced neither.
//! `schemas/sce-forge.xsd` is `xs:any processContents="lax"` for W3C
//! structural elements, so the schema calls every spelling valid, and no
//! parse-time check stood in for it. Measured 2026-09-13, a document with
//! `<state id="s0*/X">` generated without a diagnostic and the id became a
//! code identifier — `…_STATE_S0*/X` in C, `S0*/X = 1` in Python — so the
//! emitted source did not compile. The author learned this from a C compiler
//! pointing at generated code they had never seen.
//!
//! # Why a document sweep rather than a check at each read
//!
//! `parser.rs` reads these attributes at roughly twenty sites, and a guard
//! per site is a guard the twenty-first forgets. The check is one pass over
//! the parsed tree instead, driven by [`IDENTIFIER_ATTRIBUTES`]: it is
//! exhaustive over the DOCUMENT rather than over the parser's call graph, so
//! an attribute a future parser reads somewhere new is already covered, and
//! extending it to a new attribute is a row rather than a call.
//!
//! It runs at parse — `parser::SCXMLParser::parse_impl`, immediately after
//! `reject_unexpanded_directives` — and not in a later analysis pass, for
//! two reasons. The tree is still in hand, so a rejection can name the
//! attribute's own line and column; and every later stage is downstream of
//! the model, where the offending text has already become a field.
//!
//! ⚠ Position matters for a second reason: the sweep must see the document
//! AFTER `<xi:include>` and `<sce:use>` expansion. Four values in
//! `tests/w3c_template_parity/fixtures/` are pre-expansion placeholders —
//! `s_{$id}`, `tick_{$n}` — and are the only values in this tree that any of
//! these grammars refuses. They are placeholders precisely because expansion
//! replaces them, so running after it is what makes them legal.
//!
//! # The grammars, and where they part company
//!
//! Two, not one, because W3C gives two.
//!
//! * [`Grammar::Id`] and [`Grammar::IdRefs`] — an XML Name, which
//!   [`is_ncname`] spells: a token starts with a letter or `_`, continues
//!   with alphanumerics, `_` or `-`, and `.` separates tokens.
//! * [`Grammar::EventDescriptors`] and [`Grammar::EventName`] — §3.12.1's
//!   token grammar, owned by [`crate::event_descriptor`] because that module
//!   is already the one definition of what a descriptor means.
//!
//! The two differ at the first character: an XML Name may not begin with a
//! digit and an event token may. That is not a tidy-up, it is what the two
//! specifications say, and both halves were measured rather than recalled:
//! reading §3.12.1's "alphanumeric" literally rejects W3C's own conformance
//! documents 364 and 576 (`In-s11p112`), and reading it as an XML Name
//! rejects §3.12.1's own calculator example (`<transition event="DIGIT.0">`).
//!
//! # What SCE narrows, stated rather than hidden
//!
//! Both grammars here are ASCII, and the event grammar refuses `:`. W3C is
//! wider on both counts — an XML Name admits most of Unicode's letters, and
//! §3.12.1 shows `<transition event="ccxml:connection.alerting"/>` under the
//! words *"This markup is legal"*.
//!
//! SCE is narrower because these values do not stay in the document: an id
//! becomes a code identifier in C, C++, Kotlin, Rust, Go and Python, and an
//! event name becomes one too. A value this sweep admits is one all six can
//! carry. A value it refuses is refused at the author's own line rather than
//! by a compiler reading generated source.
//!
//! Measured 2026-09-14 over the 794 SCXML documents in this tree, across
//! every row of [`IDENTIFIER_ATTRIBUTES`]: the narrowing costs nothing here —
//! zero documents use a Unicode or `:`-bearing identifier — but it is a
//! boundary SCE draws and not one W3C drew, which is why it is written down
//! here and registered in `docs/SCE_ACCEPTED_SUBSET.md` §1 rather than left
//! for a rejected author to discover.

use crate::forge::error::{ForgeError, Located, ValidationError};

/// The grammar W3C gives one attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    /// An XML Schema `ID`: exactly one XML Name, declaring it.
    Id,
    /// An XML Schema `IDREFS`: whitespace-separated XML Names, referencing.
    IdRefs,
    /// §3.12.1 event descriptors, whitespace-separated. Wildcards legal.
    EventDescriptors,
    /// §3.12.1 event name — one event, so no wildcard spelling is legal.
    EventName,
}

/// Every identifier-bearing attribute of W3C SCXML, with the grammar its
/// clause gives it.
///
/// The table is what makes this rule extensible: a new attribute is a row.
///
/// ⚠ It is deliberately not "every attribute that looks like a name".
/// `<param name>` and `<scxml name>` are `NMTOKEN`, a different XML Schema
/// type with a different grammar — a leading digit is legal in an NMTOKEN —
/// and `<invoke idlocation>` names a data-model location rather than an id.
/// Putting any of the three here under [`Grammar::Id`] would refuse
/// documents W3C accepts, so the table's membership rule is the W3C
/// TYPE of the attribute, not the spelling of its name.
pub const IDENTIFIER_ATTRIBUTES: &[(&str, &str, Grammar)] = &[
    // ── Declarations: W3C types each of these `ID`. ──────────────
    ("state", "id", Grammar::Id),    // §3.3.1
    ("parallel", "id", Grammar::Id), // §3.4.1
    ("final", "id", Grammar::Id),    // §3.7.1
    ("history", "id", Grammar::Id),  // §3.10.1
    ("data", "id", Grammar::Id),     // §5.2.1
    ("invoke", "id", Grammar::Id),   // §6.4.1
    ("send", "id", Grammar::Id),     // §6.2.1
    // ── References: `IDREFS`, so a list. ─────────────────────────
    ("transition", "target", Grammar::IdRefs), // §3.13
    ("scxml", "initial", Grammar::IdRefs),     // §3.2.1
    ("state", "initial", Grammar::IdRefs),     // §3.3.1
    // ⚠ No `("parallel", "initial", …)` row. §3.4.1 gives `<parallel>` one
    // attribute, `id`, and a row for an attribute W3C does not define is a
    // row no document can reach — dead weight that reads as coverage.
    // ── Event names and descriptors, §3.12.1. ────────────────────
    ("transition", "event", Grammar::EventDescriptors), // §3.13
    ("raise", "event", Grammar::EventName),             // §4.5
    ("send", "event", Grammar::EventName),              // §6.2.1
];

/// Whether `name` is an XML Name in the ASCII form SCE accepts — the
/// grammar W3C's `ID` and `IDREF` defer to XML Schema for.
///
/// A token starts with a letter or `_`, continues with alphanumerics, `_` or
/// `-`, and `.` separates tokens.
///
/// ⚠ `.` is written as a SEPARATOR, and that is a third narrowing beside the
/// two the module header states. XML makes `.` an ordinary Name character, so
/// a true NCName admits `a.2b` — first character a letter, everything after
/// it a NameChar — and this refuses it, because the token after the dot
/// begins with a digit. The shape is deliberate: it is how §3.12.1 segments
/// an event name, and one shape for both is what lets the two grammars be
/// compared at the only place they differ. Measured 2026-09-14, no document
/// in this tree writes a dotted identifier whose later token starts with a
/// digit, so the narrowing costs nothing here — but it is a narrowing, and a
/// reader who finds `a.2b` refused should find the reason here rather than
/// conclude the check is broken.
pub fn is_ncname(name: &str) -> bool {
    !name.is_empty() && name.split('.').all(is_ncname_token)
}

/// One token of an XML Name: the unit `.` separates.
fn is_ncname_token(token: &str) -> bool {
    let mut chars = token.chars();
    match chars.next() {
        None => false,
        Some(first) if !(first.is_ascii_alphabetic() || first == '_') => false,
        Some(_) => chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
    }
}

/// The production name a rejection reports in `expected`.
///
/// A grammar production, not a repair candidate: `SCE_ERROR_CONTRACT.md` §3.2
/// keeps the two fields disjoint, and there is no closed set of legal
/// replacements for a name, so these records carry `expected` and no `fix` —
/// the shape `expression/parse-mismatch` already uses.
/// Named for the TOKEN rather than the whole attribute: `target="a b*c"`
/// fails on one of its two names, and telling the author their attribute is
/// not an `xs:IDREFS` when one word in it is the problem is the less useful
/// half of what the parser knows.
fn production(grammar: Grammar) -> &'static str {
    match grammar {
        Grammar::Id => "xs:ID",
        Grammar::IdRefs => "xs:IDREF",
        Grammar::EventDescriptors => "event descriptor",
        Grammar::EventName => "event name",
    }
}

/// The first token of `value` that `grammar` refuses, or `None`.
///
/// Public so a caller holding a value without a document — a generator
/// synthesising an event name, say — can ask the same question the sweep
/// asks, rather than writing a second answer to it.
pub fn offending_token(grammar: Grammar, value: &str) -> Option<&str> {
    match grammar {
        // A single value. Present-and-empty is refused: `id=""` declares a
        // name that is not one, and no document in this tree writes it.
        Grammar::Id => (!is_ncname(value)).then_some(value),
        Grammar::EventName => (!crate::event_descriptor::is_event_name(value)).then_some(value),
        // A list. An attribute holding no tokens at all is a different rule
        // (`validation/empty-value`) and not this one's to raise, so an
        // empty list has nothing to refuse.
        Grammar::IdRefs => value.split_whitespace().find(|token| !is_ncname(token)),
        Grammar::EventDescriptors => value
            .split_whitespace()
            .find_map(crate::event_descriptor::malformed_token),
    }
}

/// Refuse a document whose identifier-bearing attributes do not satisfy the
/// grammar W3C gives them.
///
/// The first violation in document order, with the attribute value's own
/// line and column — `SCE_ERROR_CONTRACT.md` §3.1.1 requires the value in
/// `actual` to occur on the reported line, and an attribute is not always on
/// its element's line.
///
/// `root` must be the post-expansion tree; see the module header.
pub fn reject_malformed(root: &roxmltree::Node, doc_name: &str) -> Result<(), Located<ForgeError>> {
    for node in root.descendants() {
        if !node.is_element() || node.tag_name().namespace() != Some(crate::model::SCXML_NAMESPACE)
        {
            continue;
        }
        let element = node.tag_name().name();
        for attribute in node.attributes() {
            // Unprefixed only. `conf:id` and `sce:*` share a local name with
            // an attribute in this table and are governed by whoever owns
            // that namespace, not by W3C's clause.
            if attribute.namespace().is_some() {
                continue;
            }
            let Some(&(_, _, grammar)) = IDENTIFIER_ATTRIBUTES
                .iter()
                .find(|(tag, attr, _)| *tag == element && *attr == attribute.name())
            else {
                continue;
            };
            let Some(token) = offending_token(grammar, attribute.value()) else {
                continue;
            };

            let pos = node.document().text_pos_at(attribute.range_value().start);
            let error: ValidationError = match grammar {
                Grammar::EventDescriptors | Grammar::EventName => {
                    ValidationError::EventNameGrammar {
                        element: element.to_string(),
                        attr: attribute.name().to_string(),
                        value: attribute.value().to_string(),
                        token: token.to_string(),
                        expected: production(grammar),
                    }
                }
                Grammar::Id | Grammar::IdRefs => ValidationError::MalformedIdentifier {
                    element: element.to_string(),
                    attr: attribute.name().to_string(),
                    value: attribute.value().to_string(),
                    token: token.to_string(),
                    expected: production(grammar),
                },
            };
            return Err(Located::new(
                error.into(),
                doc_name,
                Some(pos.row),
                Some(pos.col),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_grammars_differ_only_where_the_two_clauses_do() {
        // A leading digit: legal in §3.12.1's own `DIGIT.0`, illegal in an
        // XML Name. This one case is the whole reason there are two.
        assert!(crate::event_descriptor::is_event_token("0"));
        assert!(!is_ncname_token("0"));
        // Everywhere else they agree, including on the hyphen W3C's own
        // conformance documents 364 and 576 need.
        for token in ["In-s11p112", "s0", "_private", "a-b-c", "x9"] {
            assert!(is_ncname_token(token), "{token}");
            assert!(crate::event_descriptor::is_event_token(token), "{token}");
        }
    }

    #[test]
    fn a_hostile_value_is_refused_by_every_grammar_that_can_hold_one() {
        // The shapes that reach generated code as syntax: the comment
        // terminator, a path separator, a line break, a trailing escape,
        // a quote. Each is refused before it can become an identifier.
        for hostile in ["s0*/X", "a/b", "a\nb", "a\\", "a\"b", "a b", "a'b", ""] {
            assert!(
                offending_token(Grammar::Id, hostile).is_some(),
                "id accepted {hostile:?}"
            );
            assert!(
                offending_token(Grammar::EventName, hostile).is_some(),
                "event name accepted {hostile:?}"
            );
        }
    }

    #[test]
    fn a_list_is_refused_for_the_token_that_offends_and_not_the_whole() {
        assert_eq!(offending_token(Grammar::IdRefs, "good also_good"), None);
        assert_eq!(
            offending_token(Grammar::IdRefs, "good bad*one"),
            Some("bad*one")
        );
        assert_eq!(
            offending_token(Grammar::EventDescriptors, "error.* fine. *"),
            None
        );
        assert_eq!(
            offending_token(Grammar::EventDescriptors, "ok go*/Y"),
            Some("go*/Y")
        );
    }

    #[test]
    fn the_table_carries_no_attribute_twice() {
        let mut seen: Vec<(&str, &str)> = IDENTIFIER_ATTRIBUTES
            .iter()
            .map(|(tag, attr, _)| (*tag, *attr))
            .collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(before, seen.len(), "a duplicated row would be dead");
    }
}
