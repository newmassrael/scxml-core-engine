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

use crate::model::{SCXMLModel, Transition};
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

    #[test]
    fn the_annotation_is_the_tokens_uninterpreted_in_order() {
        let records = records();
        assert_eq!(records[0].annotation, ["12"]);
        assert_eq!(records[1].annotation, ["3", "REQ-X"]);
        assert!(records[2].annotation.is_empty());
    }
}
