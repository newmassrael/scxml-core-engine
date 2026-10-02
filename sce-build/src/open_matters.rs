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
    /// A gap settled by a house rule of the authoring profile: the owner's
    /// standing answer, applied where the specification is silent and cited by
    /// its id. It is listed so a rule is never applied silently; it is not a
    /// value chosen without an answer, and it is not counted as one.
    HouseRule,
    /// The statechart imports event-schemas and does not declare its
    /// interface closed, so an event no schema declares still crosses it
    /// unchecked: the schemas describe the boundary and nothing holds the
    /// statechart to them.
    Interface,
    /// The machine sends an event to ITSELF with no target, and something
    /// takes it. §scxml-6.2.4 puts that on the EXTERNAL queue, the one a
    /// caller delivers to, so a caller can send the same name and take the
    /// same transition: for a timer, a way to skip the wait. Whether a caller
    /// should be able to is the owner's to say, and a closed interface does not
    /// say it for them — a draft that declares the event as one of its inputs
    /// is accepted, and its interface then lists a timer as something to send.
    SelfDelivered,
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

/// The events a statechart sends ITSELF with no target that something takes,
/// in the order they are first written: what a caller can send too.
///
/// ⚠ §scxml-6.2.4: a `<send>` with no target goes to the session's
/// EXTERNAL queue, the one a caller delivers to, so an outside party can send
/// the same name and reach the same transition. What the statechart puts on
/// its INTERNAL queue — a `<raise>`, a send to `#_internal` — is left out, by
/// NAME: the reading the externally drivable events already take
/// ([`crate::model::SCXMLModel::internal_queue_events`]), so a name it also sends
/// with no target is left out too and the two lists cannot disagree. (A test of
/// the send's own target would add nothing: the parser has already put every
/// literal send to `#_internal` in that set.) A self-send nothing takes is the
/// lint's (`scxml/self-send-discarded`) and not this list's.
///
/// ⚠ Read from the model as PARSED, before the analyzer runs, like
/// [`interface_left_open`]: the manifest's `open` is built on routes that read
/// the model at that point, and a fact that needed the analysis would be missing
/// from the ones that do not run it. The literal names only — a computed name
/// (`eventexpr`) cannot be listed, the same limit the closed interface has.
pub fn self_delivered_events(model: &crate::model::SCXMLModel) -> Vec<String> {
    let mut events: Vec<String> = Vec::new();
    crate::host_processor_analyzer::walk_model_actions(model, &mut |_state, action| {
        if action.action_type == "send"
            && action.eventexpr.is_empty()
            && !action.event.is_empty()
            && crate::scxml_self_send::sends_to_itself(action)
            && !crate::analyzer::is_reserved_ingress_event(&action.event)
            && !model.internal_queue_events.contains(&action.event)
            && !crate::scxml_self_send::is_discarded(model, action)
            && !events.contains(&action.event)
        {
            events.push(action.event.clone());
        }
    });
    events
}

/// The markers of one kind counted by id, in the order each id is first
/// written.
///
/// An id is a QUESTION or a VALUE, and a marker is a PLACE: a specification
/// that leaves the caller's route open leaves ONE question and the draft may
/// mark it where six sends need it. Saying "6 question(s)" told an owner there
/// were six things to decide when there was one, so a sentence names both —
/// how many questions, and how many places — whenever they differ, and keeps
/// its old bytes when they do not.
struct Tally<'a> {
    counted: Vec<(&'a str, usize)>,
}

impl<'a> Tally<'a> {
    fn of(ids: impl Iterator<Item = &'a str>) -> Self {
        let mut counted: Vec<(&str, usize)> = Vec::new();
        for id in ids {
            match counted.iter_mut().find(|(seen, _)| *seen == id) {
                Some((_, times)) => *times += 1,
                None => counted.push((id, 1)),
            }
        }
        Tally { counted }
    }

    fn is_empty(&self) -> bool {
        self.counted.is_empty()
    }

    /// How many different ids.
    fn distinct(&self) -> usize {
        self.counted.len()
    }

    /// How many markers.
    fn places(&self) -> usize {
        self.counted.iter().map(|(_, times)| times).sum()
    }

    /// The ids, an id written in more than one place followed by how many.
    fn named(&self) -> String {
        self.counted
            .iter()
            .map(|(id, times)| {
                if *times == 1 {
                    (*id).to_string()
                } else {
                    format!("{id} x{times}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// What follows the count in a sentence: the ids in parentheses, and when
    /// some id is written in more than one place, how many places there are.
    fn said(&self) -> String {
        if self.places() == self.distinct() {
            format!(" ({})", self.named())
        } else {
            format!(", written in {} place(s) ({})", self.places(), self.named())
        }
    }
}

/// The open matters of ONE document, from the records the manifest carries.
///
/// In a fixed order — questions, assumed values, house rules, an open
/// interface, events a caller can send that the machine sends itself, parent,
/// host processor — so two runs over one document say the same thing in the
/// same order, and nothing when the document leaves nothing, so a page or
/// manifest for a finished design stays byte for byte what it was.
///
/// `open_interface` is [`interface_left_open`]: the schemas a statechart
/// imports and does not hold itself to. `self_delivered` is
/// [`self_delivered_events`]: the events it sends itself that a caller can send
/// too.
pub fn of(
    unresolved: &[UnresolvedRecord],
    parent_sends: &[ParentSend],
    host_causes: &[HostProcessorCauseRecord],
    open_interface: &[String],
    self_delivered: &[String],
) -> Vec<OpenMatter> {
    let mut out = Vec::new();

    // A marker that cites a house rule is the owner's standing answer and not
    // a value chosen without one, so it is said separately below.
    let tally = |kind: MarkerKind, house_rule: bool| -> Tally {
        Tally::of(
            unresolved
                .iter()
                .filter(|m| m.kind == kind && m.house_rule == house_rule)
                .map(|m| m.id.as_str()),
        )
    };
    let questions = tally(MarkerKind::Unresolved, false);
    if !questions.is_empty() {
        out.push(OpenMatter {
            kind: OpenKind::Question,
            message: format!(
                "{} question(s) the specification leaves open{}: ask the owner and \
                 record each answer; the strict check (--strict-unresolved) refuses this \
                 document until then",
                questions.distinct(),
                questions.said(),
            ),
        });
    }
    let assumed = tally(MarkerKind::Assumed, false);
    if !assumed.is_empty() {
        out.push(OpenMatter {
            kind: OpenKind::Assumed,
            message: format!(
                "{} value(s) chosen without the specification{}: the owner confirms or \
                 corrects each",
                assumed.distinct(),
                assumed.said(),
            ),
        });
    }
    // A house rule is said by its places, since applying one twice is two
    // things for the owner to confirm and the rule is one.
    let applied = tally(MarkerKind::Assumed, true);
    if !applied.is_empty() {
        out.push(OpenMatter {
            kind: OpenKind::HouseRule,
            message: format!(
                "{} place(s) apply the profile's house rule(s) ({}): the owner's \
                 standing answer, not the specification's — the owner confirms each still fits",
                applied.places(),
                applied.named(),
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

    if !self_delivered.is_empty() {
        out.push(OpenMatter {
            kind: OpenKind::SelfDelivered,
            message: format!(
                "sends itself {} event(s) with no target ({}), which puts them on the \
                 queue a caller delivers to: a caller can send the same name and take the \
                 same transition, which for a timer is a way to skip the wait. If callers \
                 must not, send each with target=\"#_internal\"; if they may, the owner \
                 says so",
                self_delivered.len(),
                self_delivered.join(", ")
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
    of_statechart_under(model, &[])
}

/// [`of_statechart`] for a design held to an authoring profile: an
/// `sce:assumed` that cites one of `house_rule_ids` is the owner's standing
/// answer and is said as that ([`OpenKind::HouseRule`]), not as a value
/// chosen without one.
pub fn of_statechart_under(
    model: &crate::model::SCXMLModel,
    house_rule_ids: &[&str],
) -> Vec<OpenMatter> {
    let mut unresolved = crate::unresolved_check::unresolved_records(model);
    crate::unresolved_check::cite_house_rules(&mut unresolved, house_rule_ids);
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
        &unresolved,
        &crate::parent_send_analyzer::records(model),
        &host,
        &interface_left_open(model),
        &self_delivered_events(model),
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
            house_rule: false,
        }
    }

    fn parent(event: Option<&str>) -> ParentSend {
        ParentSend {
            event: event.map(str::to_string),
            state: "s".into(),
            location: None,
            decisions: Vec::new(),
        }
    }

    #[test]
    fn a_document_that_leaves_nothing_yields_nothing() {
        assert!(of(&[], &[], &[], &[], &[]).is_empty());
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
            &["tick".to_string()],
        );
        let kinds: Vec<OpenKind> = matters.iter().map(|m| m.kind).collect();
        assert_eq!(
            kinds,
            [
                OpenKind::Question,
                OpenKind::Assumed,
                OpenKind::Interface,
                OpenKind::SelfDelivered,
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
        assert!(
            matters[3]
                .message
                .contains("1 event(s) with no target (tick)"),
            "{matters:?}"
        );
        // A repeated event is named once, and a site with no literal event
        // is not named at all.
        assert!(matters[4].message.contains("(Out)"), "{matters:?}");
        assert!(matters[5].message.contains("(x-host)"));
    }

    #[test]
    fn an_assumption_alone_is_not_a_question() {
        let matters = of(&[marker(MarkerKind::Assumed, "a1")], &[], &[], &[], &[]);
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

    /// What a caller can send is what the machine sends ITSELF with no target
    /// and something takes (§scxml-6.2.4: the external queue). Every other
    /// shape is a control, each a different event so one that took a shape it
    /// should not, or missed the one it should, is named.
    #[test]
    fn an_event_the_machine_sends_itself_for_a_caller_to_send_too_is_listed() {
        let parse = |sends: &str| {
            crate::parser::SCXMLParser::new()
                .parse_string(
                    &format!(
                        r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
                             <state id="a">
                               <onentry>{sends}</onentry>
                               <transition event="listed" target="b"/>
                               <transition event="listed.again" target="b"/>
                               <transition event="on_internal" target="b"/>
                               <transition event="raised" target="b"/>
                               <transition event="both" target="b"/>
                               <transition event="typed" target="b"/>
                               <transition event="error.custom" target="b"/>
                             </state>
                             <state id="b"/>
                           </scxml>"##
                    ),
                    "open_matters",
                )
                .expect("parses")
        };
        let model = parse(
            r##"<send event="listed" delay="5s"/>
                <send event="listed"/>
                <send event="listed.again" target="#_scxml_x"/>
                <send event="on_internal" target="#_internal"/>
                <raise event="raised"/>
                <raise event="both"/>
                <send event="both"/>
                <send event="typed" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"/>
                <send event="nothing_takes_it"/>
                <send event="error.custom"/>
                <send eventexpr="'listed'"/>"##,
        );
        // Named once each, in the order first written: a delayed and an
        // immediate send of one event are one thing to tell the owner.
        assert_eq!(self_delivered_events(&model), ["listed"]);
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

    /// An id is a question and a marker is a place. One question written in six
    /// places is ONE thing to decide, and the sentence says so: measured
    /// 2026-09-30 on a draft whose caller's route was marked on six sends, the
    /// old sentence said "10 question(s)" over five, and the owner was told there
    /// were ten decisions. When every id is written once the sentence is what it
    /// always was.
    #[test]
    fn a_question_written_in_several_places_is_counted_once_and_placed_every_time() {
        let many = [
            marker(MarkerKind::Unresolved, "route"),
            marker(MarkerKind::Unresolved, "route"),
            marker(MarkerKind::Unresolved, "route"),
            marker(MarkerKind::Unresolved, "payload"),
        ];
        let said = of(&many, &[], &[], &[], &[]);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0].message.starts_with(
                "2 question(s) the specification leaves open, written in 4 place(s) \
                 (route x3, payload):"
            ),
            "{said:?}"
        );

        // The control: each id once is the sentence it always was, with no
        // mention of places.
        let once = of(
            &[
                marker(MarkerKind::Unresolved, "route"),
                marker(MarkerKind::Unresolved, "payload"),
            ],
            &[],
            &[],
            &[],
            &[],
        );
        assert!(
            once[0]
                .message
                .starts_with("2 question(s) the specification leaves open (route, payload):"),
            "{once:?}"
        );
        assert!(!once[0].message.contains("place(s)"), "{once:?}");

        // Values chosen without an answer are counted the same way, and a
        // question and a value of one id are still two different things.
        let mixed = of(
            &[
                marker(MarkerKind::Assumed, "delay"),
                marker(MarkerKind::Assumed, "delay"),
                marker(MarkerKind::Unresolved, "delay"),
            ],
            &[],
            &[],
            &[],
            &[],
        );
        assert!(
            mixed[0]
                .message
                .starts_with("1 question(s) the specification leaves open (delay):"),
            "{mixed:?}"
        );
        assert!(
            mixed[1].message.starts_with(
                "1 value(s) chosen without the specification, written in 2 place(s) (delay x2):"
            ),
            "{mixed:?}"
        );
    }

    /// A house rule is the owner's standing answer, so it is said apart from a
    /// value chosen without one, and counted once per place it is applied. An
    /// `sce:unresolved` that names a house rule's id is still a question.
    #[test]
    fn a_house_rule_is_said_apart_from_a_value_chosen_without_an_answer() {
        let mut records = vec![
            marker(MarkerKind::Assumed, "H1"),
            marker(MarkerKind::Assumed, "H1"),
            marker(MarkerKind::Assumed, "retry-count"),
            marker(MarkerKind::Assumed, "H2"),
            marker(MarkerKind::Unresolved, "H1"),
        ];
        // The control: with no profile every assumed value reads as one chosen
        // without an answer, whatever its id.
        let without = of(&records, &[], &[], &[], &[]);
        let kinds: Vec<OpenKind> = without.iter().map(|m| m.kind).collect();
        assert_eq!(kinds, [OpenKind::Question, OpenKind::Assumed]);
        assert!(
            without[1]
                .message
                .starts_with("3 value(s) chosen without the specification, written in 4 place(s)"),
            "{without:?}"
        );

        crate::unresolved_check::cite_house_rules(&mut records, &["H1", "H2"]);
        let matters = of(&records, &[], &[], &[], &[]);
        let kinds: Vec<OpenKind> = matters.iter().map(|m| m.kind).collect();
        assert_eq!(
            kinds,
            [OpenKind::Question, OpenKind::Assumed, OpenKind::HouseRule]
        );
        assert!(
            matters[0].message.starts_with("1 question(s)"),
            "{matters:?}"
        );
        assert!(
            matters[1].message.starts_with("1 value(s)")
                && matters[1].message.contains("retry-count"),
            "{matters:?}"
        );
        assert!(
            matters[2].message.starts_with("3 place(s)")
                && matters[2].message.contains("(H1 x2, H2)"),
            "{matters:?}"
        );
        assert!(
            !records[4].house_rule,
            "a question is not a standing answer"
        );
    }
}
