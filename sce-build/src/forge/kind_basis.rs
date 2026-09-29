// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `<sce:kind-basis>` — why a document is the kind it declares.
//!
//! # Why the reason is part of the document
//!
//! The kind is the first decision an author makes from a specification, and
//! the product cannot make it: the evidence is prose. `sce-codegen kinds`
//! carries what the choice rests on and the manifest reports the reading the
//! product made, but neither says WHY this document is a transform rather
//! than a lookup. That reason used to live only in the conversation that
//! produced the document — gone the moment the author closed it, so the
//! owner reviewing the pseudocode saw a kind and never the clauses it was
//! chosen from, and a later reader could not tell a considered choice from a
//! default.
//!
//! So the document states it, on its root, for every kind alike:
//!
//! ```xml
//! <sce:kind-basis>
//!   <sce:evidence provenance="SPEC-7@2#4.1">the output is computed from the
//!     current count by one formula</sce:evidence>
//!   <sce:rejected kind="interpolation">the text gives a formula, not values
//!     at breakpoints</sce:rejected>
//! </sce:kind-basis>
//! ```
//!
//! It is pure metadata — no code generator reads it — and it reaches the
//! pseudocode page and the acceptance report, which are what the owner
//! reads.
//!
//! ⚠ The parser enforces every rule here itself. `schemas/sce-forge-ext.xsd`
//! states the same shape, but XSD validation is compiled out of builds
//! without the `xsd` feature, and a rule only a schema states is a rule
//! those builds do not have.

use serde::Serialize;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{ForgeKind, SCE_NAMESPACE};
use crate::provenance::SpecProvenance;
use crate::read_ledger;

/// The element on the root, and its two children.
pub const ELEMENT: &str = "kind-basis";
pub const EVIDENCE: &str = "evidence";
pub const REJECTED: &str = "rejected";

/// Why the document is the kind it declares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct KindBasis {
    /// What the specification states that makes the document this kind.
    /// At least one.
    pub evidence: Vec<Evidence>,
    /// The kinds the author considered and did not choose, each with the
    /// behaviour that ruled it out.
    pub rejected: Vec<Rejected>,
    /// The kind itself left open: the `sce:unresolved` / `sce:assumed`
    /// family on the `<sce:kind-basis>` element (docs/SCE_ACCEPTED_SUBSET.md
    /// §2.10), for a document drafted as one kind while the specification
    /// does not decide between it and the kinds its candidates name. It is
    /// the same marker everywhere else, so `--strict-unresolved` refuses the
    /// build and `sce-codegen unresolved` lists it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<crate::provenance::UnresolvedMarker>,
}

/// One statement of the specification the choice rests on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Evidence {
    /// The author's account of what the specification states.
    pub text: String,
    /// Where the specification states it, in the `sce:provenance` compact
    /// form (docs/SCE_ACCEPTED_SUBSET.md §2.10). Absent when the author
    /// named no anchor.
    pub provenance: Option<SpecProvenance>,
}

/// A kind the author considered and did not choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Rejected {
    pub kind: ForgeKind,
    /// The behaviour the specification states that rules the kind out.
    pub because: String,
}

/// What is wrong with a `<sce:kind-basis>`. One diagnostic code
/// (`validation/kind-basis-malformed`) carries them all; the fault is the
/// message, because each names the one thing to change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// A second `<sce:kind-basis>` on one root.
    Repeated,
    /// A `<sce:kind-basis>` somewhere other than directly under the root.
    NotOnRoot { parent: String },
    /// An `<sce:evidence>` or `<sce:rejected>` outside any basis.
    OutsideBasis { element: &'static str },
    /// No `<sce:evidence>`: a basis that states nothing.
    NoEvidence,
    /// An `<sce:evidence>` or `<sce:rejected>` with no text.
    EmptyText { element: &'static str },
    /// A child that is neither of the two.
    UnknownChild { child: String },
    /// A `<sce:rejected>` without a `kind`.
    RejectedWithoutKind,
    /// A `kind`, or a candidate of the open kind, no kind goes by.
    UnknownKind { value: String },
    /// A `<sce:rejected>` naming the very kind the document declares.
    RejectsItsOwnKind { kind: ForgeKind },
    /// The open kind's candidates naming the kind the document declares:
    /// the candidates are the kinds it might be INSTEAD.
    OpenToItsOwnKind { kind: ForgeKind },
    /// The same kind rejected twice, or both rejected and still open.
    NamedTwice { kind: ForgeKind },
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::Repeated => write!(f, "a document states one <sce:kind-basis>, not two"),
            Fault::NotOnRoot { parent } => write!(
                f,
                "<sce:kind-basis> belongs directly under <scxml>, not under <{parent}>"
            ),
            Fault::OutsideBasis { element } => {
                write!(f, "<sce:{element}> belongs inside <sce:kind-basis>")
            }
            Fault::NoEvidence => write!(
                f,
                "<sce:kind-basis> names no <sce:evidence>: say what the specification \
                 states that makes the document this kind"
            ),
            Fault::EmptyText { element } => write!(f, "<sce:{element}> has no text"),
            Fault::UnknownChild { child } => write!(
                f,
                "<sce:kind-basis> takes <sce:evidence> and <sce:rejected>, not <{child}>"
            ),
            Fault::RejectedWithoutKind => write!(f, "<sce:rejected> names no kind"),
            Fault::UnknownKind { value } => {
                write!(f, "<sce:rejected kind=\"{value}\"> names no kind SCE has")
            }
            Fault::RejectsItsOwnKind { kind } => write!(
                f,
                "<sce:rejected kind=\"{}\"> rules out the kind this document declares",
                kind.as_attr()
            ),
            Fault::OpenToItsOwnKind { kind } => write!(
                f,
                "sce:unresolved-candidates names \"{}\", the kind this document declares; \
                 the candidates are the kinds it might be instead",
                kind.as_attr()
            ),
            Fault::NamedTwice { kind } => write!(
                f,
                "kind \"{}\" is named twice among the rejected and still-open kinds",
                kind.as_attr()
            ),
        }
    }
}

fn is_sce(node: &roxmltree::Node, local: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(SCE_NAMESPACE)
        && node.tag_name().name() == local
}

fn fault(node: &roxmltree::Node, doc: &str, fault: Fault) -> Located<ForgeError> {
    crate::forge::parser::located(node, doc, ValidationError::MalformedKindBasis { fault })
}

/// Read the root's `<sce:kind-basis>`, if it has one, for a document that
/// declares `declared`.
///
/// Every element involved goes on the parse's read ledger, so a forge
/// parse that refuses unread elements counts them as read, and a statechart
/// parse — which has no element ledger — is held to the same rules here.
pub fn read(
    root: &roxmltree::Node,
    declared: ForgeKind,
    doc: &str,
) -> Result<Option<KindBasis>, Located<ForgeError>> {
    // Anywhere but on the root is refused by name: a forge parse would
    // refuse it as an unread child, a statechart parse would drop it, and
    // neither says what an author who put it there meant.
    if let Some(misplaced) = root
        .descendants()
        .find(|n| is_sce(n, ELEMENT) && n.parent_element() != Some(*root))
    {
        let parent = misplaced
            .parent_element()
            .map(|p| p.tag_name().name().to_string())
            .unwrap_or_default();
        return Err(fault(&misplaced, doc, Fault::NotOnRoot { parent }));
    }
    if let Some(stray) = root.descendants().find(|n| {
        (is_sce(n, EVIDENCE) || is_sce(n, REJECTED))
            && !n.parent_element().is_some_and(|p| is_sce(&p, ELEMENT))
    }) {
        let element = if is_sce(&stray, EVIDENCE) {
            EVIDENCE
        } else {
            REJECTED
        };
        return Err(fault(&stray, doc, Fault::OutsideBasis { element }));
    }

    read_ledger::asked(root, ELEMENT);
    let mut found = root.children().filter(|n| is_sce(n, ELEMENT));
    let Some(basis) = found.next() else {
        return Ok(None);
    };
    read_ledger::taken(&basis);
    if let Some(second) = found.next() {
        return Err(fault(&second, doc, Fault::Repeated));
    }
    // Every child is judged below, so the ledger has nothing left to find.
    read_ledger::closed(&basis);

    let mut evidence = Vec::new();
    let mut rejected: Vec<Rejected> = Vec::new();
    for child in basis.children().filter(roxmltree::Node::is_element) {
        if is_sce(&child, EVIDENCE) {
            let text = text_of(&child, doc, EVIDENCE)?;
            let provenance = match child.attribute("provenance") {
                None => None,
                Some(value) => Some(SpecProvenance::parse_compact(value).ok_or_else(|| {
                    crate::forge::parser::located(
                        &child,
                        doc,
                        ValidationError::MalformedProvenance {
                            element: "<sce:evidence>".into(),
                            value: value.to_string(),
                        },
                    )
                })?),
            };
            evidence.push(Evidence { text, provenance });
        } else if is_sce(&child, REJECTED) {
            let because = text_of(&child, doc, REJECTED)?;
            let value = child
                .attribute("kind")
                .ok_or_else(|| fault(&child, doc, Fault::RejectedWithoutKind))?;
            let kind = ForgeKind::from_attr(value).ok_or_else(|| {
                fault(
                    &child,
                    doc,
                    Fault::UnknownKind {
                        value: value.to_string(),
                    },
                )
            })?;
            if kind == declared {
                return Err(fault(&child, doc, Fault::RejectsItsOwnKind { kind }));
            }
            if rejected.iter().any(|r| r.kind == kind) {
                return Err(fault(&child, doc, Fault::NamedTwice { kind }));
            }
            rejected.push(Rejected { kind, because });
        } else {
            let name = child.tag_name();
            let child_name = match name.namespace() {
                Some(SCE_NAMESPACE) => format!("sce:{}", name.name()),
                _ => name.name().to_string(),
            };
            return Err(fault(
                &child,
                doc,
                Fault::UnknownChild { child: child_name },
            ));
        }
    }
    if evidence.is_empty() {
        return Err(fault(&basis, doc, Fault::NoEvidence));
    }
    // The marker reader every other element goes through, so an open kind
    // is recorded, reported and refused exactly as an open value is.
    let unresolved = crate::parser::collect_sce_unresolved(&basis, doc);
    for marker in &unresolved {
        for candidate in &marker.candidates {
            let kind = ForgeKind::from_attr(candidate).ok_or_else(|| {
                fault(
                    &basis,
                    doc,
                    Fault::UnknownKind {
                        value: candidate.clone(),
                    },
                )
            })?;
            if kind == declared {
                return Err(fault(&basis, doc, Fault::OpenToItsOwnKind { kind }));
            }
            if rejected.iter().any(|r| r.kind == kind) {
                return Err(fault(&basis, doc, Fault::NamedTwice { kind }));
            }
        }
    }
    Ok(Some(KindBasis {
        evidence,
        rejected,
        unresolved,
    }))
}

/// The element's text, whitespace collapsed to single spaces so a line
/// wrapped in the source reads as one sentence on the page.
fn text_of(
    node: &roxmltree::Node,
    doc: &str,
    element: &'static str,
) -> Result<String, Located<ForgeError>> {
    let text: String = node
        .text()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return Err(fault(node, doc, Fault::EmptyText { element }));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_doc(body: &str, declared: ForgeKind) -> Result<Option<KindBasis>, String> {
        let text = format!(
            "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
             xmlns:sce=\"http://sce.dev/ext\">{body}</scxml>"
        );
        let doc = roxmltree::Document::parse(&text).expect("the fixture is XML");
        read(&doc.root_element(), declared, "t.scxml").map_err(|e| e.to_string())
    }

    #[test]
    fn a_basis_reads_its_evidence_and_what_it_ruled_out() {
        let basis = read_doc(
            "<sce:kind-basis>\
               <sce:evidence provenance=\"SPEC@2#4.1\">one   formula\n over the count</sce:evidence>\
               <sce:rejected kind=\"interpolation\">no breakpoints</sce:rejected>\
             </sce:kind-basis>",
            ForgeKind::Transform,
        )
        .expect("valid")
        .expect("present");
        assert_eq!(basis.evidence[0].text, "one formula over the count");
        assert_eq!(
            basis.evidence[0]
                .provenance
                .as_ref()
                .and_then(|p| p.to_compact()),
            Some("SPEC@2#4.1".to_string())
        );
        assert_eq!(basis.rejected[0].kind, ForgeKind::Interpolation);
    }

    /// A kind the specification leaves open is the unresolved marker on
    /// the basis, with the other kinds as its candidates.
    #[test]
    fn an_open_kind_is_the_unresolved_marker_on_the_basis() {
        let basis = read_doc(
            "<sce:kind-basis sce:unresolved=\"kind\" \
               sce:unresolved-reason=\"the text never says what an unlisted input gives\" \
               sce:unresolved-candidates=\"transform interpolation\">\
               <sce:evidence>three listed inputs, each with its setting</sce:evidence>\
             </sce:kind-basis>",
            ForgeKind::Lookup,
        )
        .expect("valid")
        .expect("present");
        let marker = &basis.unresolved[0];
        assert_eq!(marker.id, "kind");
        assert_eq!(marker.candidates, ["transform", "interpolation"]);
    }

    #[test]
    fn no_basis_is_an_answer_not_an_error() {
        assert_eq!(read_doc("", ForgeKind::Lookup), Ok(None));
    }

    /// Each fault, from the nearest valid basis changed in one place.
    #[test]
    fn every_fault_is_refused_by_name() {
        let ok = "<sce:evidence>a table of codes</sce:evidence>";
        assert!(read_doc(
            &format!("<sce:kind-basis>{ok}</sce:kind-basis>"),
            ForgeKind::Lookup
        )
        .is_ok());
        let cases = [
            (
                format!("<sce:kind-basis>{ok}</sce:kind-basis><sce:kind-basis>{ok}</sce:kind-basis>"),
                "not two",
            ),
            (
                format!("<state id=\"s\"><sce:kind-basis>{ok}</sce:kind-basis></state>"),
                "not under <state>",
            ),
            ("<sce:kind-basis/>".to_string(), "names no <sce:evidence>"),
            (
                format!("<state id=\"s\">{ok}</state>"),
                "belongs inside <sce:kind-basis>",
            ),
            (
                "<sce:kind-basis><sce:evidence> </sce:evidence></sce:kind-basis>".to_string(),
                "<sce:evidence> has no text",
            ),
            (
                format!("<sce:kind-basis>{ok}<sce:note>x</sce:note></sce:kind-basis>"),
                "not <sce:note>",
            ),
            (
                format!("<sce:kind-basis>{ok}<sce:rejected>x</sce:rejected></sce:kind-basis>"),
                "names no kind",
            ),
            (
                format!(
                    "<sce:kind-basis>{ok}<sce:rejected kind=\"table\">x</sce:rejected></sce:kind-basis>"
                ),
                "names no kind SCE has",
            ),
            (
                format!(
                    "<sce:kind-basis>{ok}<sce:rejected kind=\"lookup\">x</sce:rejected></sce:kind-basis>"
                ),
                "rules out the kind this document declares",
            ),
            (
                format!(
                    "<sce:kind-basis>{ok}<sce:rejected kind=\"enum\">x</sce:rejected>\
                     <sce:rejected kind=\"enum\">y</sce:rejected></sce:kind-basis>"
                ),
                "named twice",
            ),
            (
                format!(
                    "<sce:kind-basis sce:unresolved=\"kind\" \
                     sce:unresolved-candidates=\"lookup\">{ok}</sce:kind-basis>"
                ),
                "the kinds it might be instead",
            ),
            (
                format!(
                    "<sce:kind-basis sce:unresolved=\"kind\" \
                     sce:unresolved-candidates=\"table\">{ok}</sce:kind-basis>"
                ),
                "names no kind SCE has",
            ),
            (
                format!(
                    "<sce:kind-basis sce:unresolved=\"kind\" sce:unresolved-candidates=\"enum\">\
                     {ok}<sce:rejected kind=\"enum\">x</sce:rejected></sce:kind-basis>"
                ),
                "named twice",
            ),
            (
                "<sce:kind-basis><sce:evidence provenance=\"@3\">x</sce:evidence></sce:kind-basis>"
                    .to_string(),
                "names no source document",
            ),
        ];
        for (body, says) in cases {
            let refused = read_doc(&body, ForgeKind::Lookup).expect_err(&body);
            assert!(refused.contains(says), "{body}: {refused}");
        }
    }
}
