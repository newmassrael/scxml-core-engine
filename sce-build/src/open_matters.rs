// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a design that the product accepts still leaves to a person.
//!
//! `accepted` says the product found nothing to refuse. It does not say the
//! design is finished. Drafts a specification owner was shown as
//! `accepted` were not: one left a count `sce:unresolved`, one sent an
//! output to `#_parent` where nothing invokes it, one sent it to the machine
//! itself. Each is valid SCXML with a defined meaning, so none is a
//! refusal, and each is a fact the owner has to settle before they call the
//! design theirs.
//!
//! # One place, three readers
//!
//! The sentences are written HERE, from the three records the stdout
//! manifest already carries (`unresolved`, `parent_sends`,
//! `host_processor_causes`), and every surface that tells a person reads
//! them from here:
//!
//! - the acceptance report prints them at the top of its risk block;
//! - the manifest publishes them as `open`, which the authoring MCP relays
//!   without a sentence of its own;
//! - the acceptance record keeps them, so the record says what was known to
//!   be open when a person accepted.
//!
//! A second author of these sentences — the authoring layer used to have
//! one — is two answers to "what is still open", and the one consulted less
//! often is the one that rots.
//!
//! # What is not here
//!
//! The pseudocode page is not a reader. It is the canonical rendering the
//! reverse converter reads back and every gate compares byte for byte, so a
//! derived sentence on it would either break the round trip or change what
//! every approval already means. What the page shows is the model — the
//! `unresolved` marker and the `to #_parent` line are on it — and what the
//! model implies is said beside it, here.

use serde::{Deserialize, Serialize};

use crate::host_processor_analyzer::HostProcessorCauseRecord;
use crate::parent_send_analyzer::ParentSend;
use crate::provenance::MarkerKind;
use crate::unresolved_check::UnresolvedRecord;

/// What kind of thing is left open. Closed, so a consumer that routes on it
/// is told when a kind is added rather than finding one it never handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenKind {
    /// A question the specification leaves open (`sce:unresolved`).
    Question,
    /// A value chosen without the specification (`sce:assumed`).
    Assumed,
    /// The statechart imports event-schemas and does not declare its
    /// interface closed, so an event no schema declares still crosses it
    /// unchecked: the schemas describe the boundary and nothing holds the
    /// statechart to them.
    Interface,
    /// The machine sends to its parent session, so it can only run as a
    /// child.
    Parent,
    /// The machine names an Event I/O Processor or invoker type the host
    /// has to serve.
    HostProcessor,
}

/// One thing a person still has to settle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenMatter {
    pub kind: OpenKind,
    /// A sentence saying what it is and what to do about it.
    pub message: String,
}

/// The aliases of the event-schemas a statechart imports without declaring
/// its interface closed, in document order — empty when it declares it, or
/// imports none.
///
/// ⚠ A fact about a statechart that imports schemas, and about no other.
/// The default interface is open and stays it: a statechart with no schema
/// says nothing here, since nothing tells the product it was meant to have
/// one. What is worth saying is the mismatch — schemas written to describe
/// the boundary, and a declaration that would hold the statechart to them
/// left off. Measured 2026-09-30: a draft handed on as passing had its
/// `sce:interface="closed"` removed to get past a check, with every import
/// still in place, and its answer said nothing of it.
pub fn interface_left_open(model: &crate::model::SCXMLModel) -> Vec<String> {
    if model.interface_closed {
        return Vec::new();
    }
    // An alias is unique per document — the parser refuses a repeat — so
    // nothing here needs to be told apart from itself.
    model
        .forge_imports
        .iter()
        .filter(|import| matches!(import.kind, crate::forge::model::ForgeKind::EventSchema))
        .map(|import| import.alias.clone())
        .collect()
}

/// The open matters of ONE document, from the records the manifest carries.
///
/// In a fixed order — questions, assumed values, an open interface, parent,
/// host processor — so two runs over one document say the same thing in the
/// same order, and nothing when the document leaves nothing, so a page or
/// manifest for a finished design stays byte for byte what it was.
///
/// `open_interface` is [`interface_left_open`]: the schemas a statechart
/// imports and does not hold itself to.
pub fn of(
    unresolved: &[UnresolvedRecord],
    parent_sends: &[ParentSend],
    host_causes: &[HostProcessorCauseRecord],
    open_interface: &[String],
) -> Vec<OpenMatter> {
    let mut out = Vec::new();

    let ids = |kind: MarkerKind| -> Vec<&str> {
        unresolved
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| m.id.as_str())
            .collect()
    };
    let questions = ids(MarkerKind::Unresolved);
    if !questions.is_empty() {
        out.push(OpenMatter {
            kind: OpenKind::Question,
            message: format!(
                "{} question(s) the specification leaves open ({}): ask the owner and \
                 record each answer; the strict check (--strict-unresolved) refuses this \
                 document until then",
                questions.len(),
                questions.join(", ")
            ),
        });
    }
    let assumed = ids(MarkerKind::Assumed);
    if !assumed.is_empty() {
        out.push(OpenMatter {
            kind: OpenKind::Assumed,
            message: format!(
                "{} value(s) chosen without the specification ({}): the owner confirms or \
                 corrects each",
                assumed.len(),
                assumed.join(", ")
            ),
        });
    }

    if !open_interface.is_empty() {
        out.push(OpenMatter {
            kind: OpenKind::Interface,
            message: format!(
                "imports event-schema(s) ({}) without declaring sce:interface=\"closed\", so \
                 an event no schema declares still crosses the statechart unchecked: declare \
                 the interface closed, or tell the owner the boundary is open",
                open_interface.join(", ")
            ),
        });
    }

    if !parent_sends.is_empty() {
        let mut events: Vec<&str> = parent_sends
            .iter()
            .filter_map(|s| s.event.as_deref())
            .collect();
        events.sort_unstable();
        events.dedup();
        let named = if events.is_empty() {
            String::new()
        } else {
            format!(" ({})", events.join(", "))
        };
        out.push(OpenMatter {
            kind: OpenKind::Parent,
            message: format!(
                "sends to its parent session{named}, so it can only run as a child: check it \
                 together with the statechart that invokes it (check --document, one per \
                 document), or send to a host-served processor if the specification names \
                 no parent"
            ),
        });
    }

    if !host_causes.is_empty() {
        let mut types: Vec<&str> = host_causes
            .iter()
            .map(|c| c.processor_type.as_str())
            .collect();
        types.sort_unstable();
        types.dedup();
        out.push(OpenMatter {
            kind: OpenKind::HostProcessor,
            message: format!(
                "names a processor type this build has no path for ({}): the host has to \
                 serve it, or the site raises error.execution at run time",
                types.join(", ")
            ),
        });
    }

    out
}

/// The open matters of a statechart read straight from its model, for a
/// caller that has no manifest run to read the records from — the
/// acceptance report parses the document and stops there.
///
/// The same records the manifest publishes, by the same functions, so a
/// report and a manifest over one document cannot disagree.
pub fn of_statechart(model: &crate::model::SCXMLModel) -> Vec<OpenMatter> {
    let host: Vec<HostProcessorCauseRecord> = crate::host_processor_analyzer::analyze(model)
        .iter()
        .map(|cause| {
            let mut record = cause.to_wire();
            record.location = record
                .location
                .as_ref()
                .map(|at| model.authored_location(at));
            record
        })
        .collect();
    of(
        &crate::unresolved_check::unresolved_records(model),
        &crate::parent_send_analyzer::records(model),
        &host,
        &interface_left_open(model),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::error::SourceLocation;

    fn marker(kind: MarkerKind, id: &str) -> UnresolvedRecord {
        UnresolvedRecord {
            node_path: "states.s".into(),
            node_type: "state",
            action_type: None,
            kind,
            id: id.into(),
            reason: None,
            candidates: vec![],
            location: Some(SourceLocation {
                file: "d.scxml".into(),
                line: Some(1),
                col: Some(1),
            }),
        }
    }

    fn parent(event: Option<&str>) -> ParentSend {
        ParentSend {
            event: event.map(str::to_string),
            state: "s".into(),
            location: None,
        }
    }

    #[test]
    fn a_document_that_leaves_nothing_yields_nothing() {
        assert!(of(&[], &[], &[], &[]).is_empty());
    }

    #[test]
    fn each_fact_yields_one_matter_in_a_fixed_order() {
        let matters = of(
            &[
                marker(MarkerKind::Assumed, "a1"),
                marker(MarkerKind::Unresolved, "q1"),
                marker(MarkerKind::Unresolved, "q2"),
            ],
            &[parent(Some("Out")), parent(Some("Out")), parent(None)],
            &[HostProcessorCauseRecord {
                kind: "send-type",
                processor_type: "x-host".into(),
                state: Some("s".into()),
                invoke: None,
                location: None,
            }],
            &["Requested".to_string(), "Cancelled".to_string()],
        );
        let kinds: Vec<OpenKind> = matters.iter().map(|m| m.kind).collect();
        assert_eq!(
            kinds,
            [
                OpenKind::Question,
                OpenKind::Assumed,
                OpenKind::Interface,
                OpenKind::Parent,
                OpenKind::HostProcessor
            ]
        );
        assert!(matters[0].message.contains("2 question(s)"), "{matters:?}");
        assert!(matters[0].message.contains("q1, q2"));
        assert!(matters[1].message.contains("1 value(s)"));
        assert!(
            matters[2].message.contains("(Requested, Cancelled)"),
            "{matters:?}"
        );
        // A repeated event is named once, and a site with no literal event
        // is not named at all.
        assert!(matters[3].message.contains("(Out)"), "{matters:?}");
        assert!(matters[4].message.contains("(x-host)"));
    }

    #[test]
    fn an_assumption_alone_is_not_a_question() {
        let matters = of(&[marker(MarkerKind::Assumed, "a1")], &[], &[], &[]);
        assert_eq!(matters.len(), 1);
        assert_eq!(matters[0].kind, OpenKind::Assumed);
        assert!(!matters[0].message.contains("question"));
    }

    /// The report has no manifest run, so it reads the model. A statechart
    /// that is finished has nothing; one that sends to a parent says so.
    #[test]
    fn a_statechart_is_read_from_its_model() {
        let parse = |body: &str| {
            crate::parser::SCXMLParser::new()
                .parse_string(body, "open_matters")
                .expect("parses")
        };
        let finished = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
                 <state id="a"><transition event="go" target="b"/></state>
                 <final id="b"/>
               </scxml>"#,
        );
        assert!(of_statechart(&finished).is_empty());
        let orphan = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
                  <state id="a"><onentry><send event="Out" target="#_parent"/></onentry></state>
                </scxml>"##,
        );
        let matters = of_statechart(&orphan);
        assert_eq!(matters.len(), 1, "{matters:?}");
        assert_eq!(matters[0].kind, OpenKind::Parent);
    }

    /// Only the mismatch is said: schemas imported, and the declaration that
    /// would hold the statechart to them left off. A statechart with no
    /// schema, and one that declares its interface closed, say nothing.
    #[test]
    fn an_open_interface_is_said_only_when_schemas_are_imported() {
        let parse = |root_attrs: &str, imports: &str| {
            crate::parser::SCXMLParser::new()
                .parse_string(
                    &format!(
                        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                                 xmlns:sce="http://sce.dev/ext" version="1.0" initial="a"
                                 {root_attrs}>
                             {imports}
                             <state id="a"/>
                           </scxml>"#
                    ),
                    "open_matters",
                )
                .expect("parses")
        };
        let import = r#"<sce:import src="s.scxml" kind="event-schema" as="Sig"/>
                        <sce:import src="t.scxml" kind="event-schema" as="Other"/>
                        <sce:import src="e.scxml" kind="enum" as="Mode"/>"#;
        assert_eq!(
            interface_left_open(&parse("", import)),
            ["Sig".to_string(), "Other".to_string()],
            "the event-schemas, in document order, and an enum is not one"
        );
        assert!(interface_left_open(&parse("sce:interface=\"closed\"", import)).is_empty());
        assert!(interface_left_open(&parse("", "")).is_empty());
    }
}
