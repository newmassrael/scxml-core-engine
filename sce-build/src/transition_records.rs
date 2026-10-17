//! The transitions of a machine, as one list produced once.
//!
//! A machine's transitions are one fact with several surfaces: the
//! design-time export (`sce-codegen transition-table`) and the table a
//! backend may emit into the generated machine. Each surface renders THIS
//! list. A surface that walked the model for transitions itself would be a
//! second producer, and two producers of one fact drift.
//!
//! The list is a filter over [`walk_nodes`], the traversal the requirements
//! report, the manifest classification and the transition table already
//! share, so every reading names the same `node_path` for the same node.
//!
//! The annotation is carried as the `sce:req` tokens, uninterpreted. Nothing
//! here reads their shape: a token such as `12` is the consumer's to parse.

use std::collections::BTreeMap;

use crate::model::{EmittedTransition, EmittedTransitionTable, SCXMLModel, Transition};
use crate::requirements_report::{walk_nodes, NodeSubject};

/// One `<transition>` of a machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionRecord {
    /// Document order of the transition within its state: the `N` of
    /// `states.<id>.transitions[N]`.
    pub index: usize,
    /// The owning state's id.
    pub from: String,
    /// The `event` descriptors as written, split on whitespace. Empty for an
    /// eventless transition. A descriptor is not resolved here: `*`, a
    /// prefix such as `error.*` and a literal name are all kept as written.
    pub events: Vec<String>,
    /// The target set, one id per token, as written. Empty for a targetless
    /// transition. A token naming a `<history>` stays that id.
    pub targets: Vec<String>,
    /// The `sce:req` tokens of the transition in the order written. Empty
    /// when it carries none.
    pub annotation: Vec<String>,
    /// `states.<id>.transitions[<index>]`, the path every other reading of
    /// the model prints for this node.
    pub node_path: String,
}

impl TransitionRecord {
    /// The record of one transition. `requirement_ids` is what the shared
    /// walk read from the node, so the annotation is the walk's and not a
    /// second parse of the attribute.
    pub(crate) fn of(
        state_id: &str,
        index: usize,
        transition: &Transition,
        requirement_ids: &[&str],
        node_path: &str,
    ) -> Self {
        Self {
            index,
            from: state_id.to_string(),
            events: transition
                .event
                .split_whitespace()
                .map(str::to_string)
                .collect(),
            targets: transition.targets.clone(),
            annotation: requirement_ids.iter().map(|id| id.to_string()).collect(),
            node_path: node_path.to_string(),
        }
    }
}

/// Every transition of `model`, in the order [`walk_nodes`] meets them:
/// states by document order, and within a state by position.
pub fn transition_records(model: &SCXMLModel) -> Vec<TransitionRecord> {
    walk_nodes(model)
        .into_iter()
        .filter_map(|node| match node.subject {
            NodeSubject::Transition {
                state,
                index,
                transition,
            } => Some(TransitionRecord::of(
                &state.id,
                index,
                transition,
                &node.record.requirement_ids,
                &node.record.node_path,
            )),
            _ => None,
        })
        .collect()
}

impl EmittedTransitionTable {
    /// The table of `model`, built from [`transition_records`] and nothing
    /// else, so it lists the transitions the export lists, in its order.
    pub fn of(model: &SCXMLModel, source_hash: String) -> Self {
        let transitions = transition_records(model)
            .into_iter()
            .map(|record| {
                let literal_event = match record.events.as_slice() {
                    [only] if is_literal_event(only) && model.events.contains(only) => {
                        Some(only.clone())
                    }
                    _ => None,
                };
                EmittedTransition {
                    index: record.index,
                    to: record
                        .targets
                        .first()
                        .filter(|target| model.states.contains_key(*target))
                        .cloned(),
                    event: literal_event,
                    targets: record.targets.join(" "),
                    events: record.events.join(" "),
                    annotation: record.annotation.join(" "),
                    from: record.from,
                }
            })
            .collect();
        Self {
            source_hash,
            transitions,
        }
    }
}

/// Whether a descriptor names one event rather than asking for a family:
/// `*` and a prefix written with a trailing `.*` match more than themselves.
fn is_literal_event(descriptor: &str) -> bool {
    descriptor != "*" && !descriptor.ends_with(".*")
}

/// The records of `model` keyed by `node_path`, for a reading that walks the
/// whole node list and needs the record of the transition it is standing on.
pub(crate) fn transition_records_by_path(model: &SCXMLModel) -> BTreeMap<String, TransitionRecord> {
    transition_records(model)
        .into_iter()
        .map(|record| (record.node_path.clone(), record))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="idle" datamodel="null">
        <state id="idle">
            <transition event="go" target="busy" sce:req="12"/>
            <transition event="a b.* *" target="busy done" sce:req="3 REQ-X"/>
            <transition target="done"/>
        </state>
        <state id="busy">
            <transition event="stop"/>
        </state>
        <final id="done"/>
    </scxml>"#;

    fn records() -> Vec<TransitionRecord> {
        let model = SCXMLParser::new()
            .parse_string(DOC, "transition_records")
            .expect("parse");
        transition_records(&model)
    }

    #[test]
    fn one_record_per_transition_in_document_order() {
        let paths: Vec<String> = records().into_iter().map(|r| r.node_path).collect();
        assert_eq!(
            paths,
            [
                "states.idle.transitions[0]",
                "states.idle.transitions[1]",
                "states.idle.transitions[2]",
                "states.busy.transitions[0]",
            ]
        );
    }

    #[test]
    fn index_is_the_position_within_the_owning_state() {
        let indices: Vec<(String, usize)> =
            records().into_iter().map(|r| (r.from, r.index)).collect();
        assert_eq!(
            indices,
            [
                ("idle".to_string(), 0),
                ("idle".to_string(), 1),
                ("idle".to_string(), 2),
                ("busy".to_string(), 0),
            ]
        );
    }

    #[test]
    fn events_and_targets_are_split_as_written() {
        let records = records();
        assert_eq!(records[1].events, ["a", "b.*", "*"]);
        assert_eq!(records[1].targets, ["busy", "done"]);
    }

    #[test]
    fn an_eventless_transition_has_no_events_and_a_targetless_one_no_targets() {
        let records = records();
        assert!(records[2].events.is_empty());
        assert_eq!(records[2].targets, ["done"]);
        assert_eq!(records[3].events, ["stop"]);
        assert!(records[3].targets.is_empty());
    }

    const EMITTED_DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="idle" datamodel="null">
        <state id="idle">
            <transition event="go" target="busy" sce:req="12 REQ-X"/>
            <transition event="a b" target="busy"/>
            <transition event="*" target="busy"/>
            <transition event="err.*" target="busy"/>
            <transition target="busy"/>
            <transition event="back" target="h"/>
            <transition event="stop"/>
        </state>
        <state id="busy">
            <history id="h"><transition target="idle"/></history>
            <transition event="done" target="end idle"/>
        </state>
        <final id="end"/>
    </scxml>"#;

    fn emitted() -> (EmittedTransitionTable, Vec<TransitionRecord>) {
        let model = SCXMLParser::new()
            .parse_string(EMITTED_DOC, "emitted_transition_table")
            .expect("parse");
        (
            EmittedTransitionTable::of(&model, "ab".repeat(32)),
            transition_records(&model),
        )
    }

    #[test]
    fn the_emitted_table_lists_the_transitions_of_the_export_in_its_order() {
        let (table, records) = emitted();
        assert_eq!(table.source_hash, "ab".repeat(32));
        let listed: Vec<(&str, usize)> = table
            .transitions
            .iter()
            .map(|t| (t.from.as_str(), t.index))
            .collect();
        let exported: Vec<(&str, usize)> =
            records.iter().map(|r| (r.from.as_str(), r.index)).collect();
        assert_eq!(listed, exported);
        assert_eq!(listed.len(), 8);
    }

    #[test]
    fn an_event_is_an_id_only_when_the_transition_names_one_literal_event() {
        let (table, _) = emitted();
        let events: Vec<Option<&str>> = table
            .transitions
            .iter()
            .map(|t| t.event.as_deref())
            .collect();
        assert_eq!(
            events,
            [
                Some("go"),
                None, // two descriptors
                None, // the wildcard
                None, // a prefix family
                None, // eventless
                Some("back"),
                Some("stop"),
                Some("done"),
            ]
        );
    }

    #[test]
    fn a_target_is_an_id_only_when_the_first_one_is_a_state() {
        let (table, _) = emitted();
        let targets: Vec<Option<&str>> =
            table.transitions.iter().map(|t| t.to.as_deref()).collect();
        assert_eq!(
            targets,
            [
                Some("busy"),
                Some("busy"),
                Some("busy"),
                Some("busy"),
                Some("busy"),
                None, // a history is named in a target list and is not a state
                None, // targetless
                Some("end"),
            ]
        );
    }

    #[test]
    fn the_whole_target_set_and_every_descriptor_are_carried_as_written() {
        let (table, records) = emitted();
        let carried: Vec<(&str, &str)> = table
            .transitions
            .iter()
            .map(|t| (t.targets.as_str(), t.events.as_str()))
            .collect();
        assert_eq!(
            carried,
            [
                ("busy", "go"),
                ("busy", "a b"),      // several descriptors
                ("busy", "*"),        // the wildcard, unresolved
                ("busy", "err.*"),    // a prefix family, unresolved
                ("busy", ""),         // eventless
                ("h", "back"),        // a history is a target as written
                ("", "stop"),         // targetless
                ("end idle", "done"), // the second target is not lost
            ]
        );
        for (entry, record) in table.transitions.iter().zip(&records) {
            assert_eq!(entry.targets, record.targets.join(" "));
            assert_eq!(entry.events, record.events.join(" "));
        }
    }

    #[test]
    fn the_annotation_is_carried_as_the_joined_tokens() {
        let (table, _) = emitted();
        assert_eq!(table.transitions[0].annotation, "12 REQ-X");
        assert_eq!(table.transitions[1].annotation, "");
    }

    #[test]
    fn the_annotation_is_the_tokens_uninterpreted_in_order() {
        let records = records();
        assert_eq!(records[0].annotation, ["12"]);
        assert_eq!(records[1].annotation, ["3", "REQ-X"]);
        assert!(records[2].annotation.is_empty());
    }
}
