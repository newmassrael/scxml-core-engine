//! Requirement-closure RFC ② — the transition table, which is the
//! trace table.
//!
//! ```text
//!   source     | from      | event      | guard | after | to         | action
//!   -----------+-----------+------------+-------+-------+------------+---------------------
//!   REQ-042    | idle      | btn_press  | -     | -     | emergency  | -
//!   REQ-043    | emergency | (entry)    | -     | 3s    | -          | send event=drive_off
//!   (none)     | idle      | diag_req   | -     | -     | diagnostic | -
//! ```
//!
//! Two readings, and they are the point rather than a convenience
//! (RFC §6.2):
//!
//! - **Sort by `source`.** Every `(none)` row collects into one block:
//!   behaviour the specification never asked for. That is the fifth
//!   case of §5.3, and no id-based check can see it — `dangling`
//!   catches an invented *id*, this catches unannotated *behaviour*.
//! - **Compare `source` against the manifest.** A requirement with no
//!   row at all is `missing`.
//!
//! So the hand-built trace table of certified practice and a table
//! mechanically extracted from the SCXML are the same object.
//!
//! # ⚠ Why every requirement-bearing node gets a row, not just transitions
//!
//! RFC §6.2's worked example shows transitions and one `(entry)` row,
//! and says in the same breath that "a requirement with no row at all
//! is `missing`". Those two statements are not compatible as written.
//! `sce:req` is admissible on a `<state>`, an `<invoke>` and any
//! executable action — the Atomic A fixture puts one on a state — so a
//! table of transitions alone would show no row for a requirement that
//! is implemented, and the second reading would call it `missing`.
//!
//! The derivation is the load-bearing half, so the table widens to
//! match it: one row per node the shared traversal yields, with the
//! pseudo-events `(state)`, `(entry)`, `(exit)`, `(transition)`,
//! `(initial)`, `(history)` and `(invoke)` where there is no real event.
//! An action nested inside `<if>` / `<foreach>` gets its own row with its
//! block's pseudo-event; its `node_path` says how deep it is.
//!
//! That also makes §6.2's closing claim true rather than aspirational —
//! the classification and the table become two readings of ONE walk, so
//! a requirement's verdict and its row are answers about the same node.
//!
//! # What `after` means here
//!
//! Statechart notation writes `after(3s)` on a transition. This IR does
//! not: a delay lives on a `<send>` action (`delay` / `delayexpr`),
//! which is how SCXML expresses it. So `after` is populated on the
//! action row that actually carries the delay and is `-` on
//! transitions, rather than being synthesised onto a transition the IR
//! never gave one to.

use std::io::{self, Write};

use serde::Serialize;

use crate::model::{SCXMLModel, Transition};
use crate::requirements_report::{walk_nodes, ActionSite, NodeSubject};
use crate::transition_records::transition_records_by_path;

/// The literal printed in `source` for a node claiming no requirement.
///
/// A word rather than an empty cell, because the whole value of the
/// column is that these rows COLLECT: an empty string sorts with
/// whatever else is empty and reads as a rendering slip, while
/// `(none)` sorts as one block and reads as a finding.
pub const NO_SOURCE: &str = "(none)";

/// One row. Columns are RFC §6.2's, in its order.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TransitionRow {
    /// Requirement ids claimed by the node, space-joined, or
    /// [`NO_SOURCE`].
    pub source: String,
    /// Owning state.
    pub from: String,
    /// The event, or a pseudo-event in parentheses for a node that is
    /// not event-driven.
    pub event: String,
    pub guard: String,
    pub after: String,
    pub to: String,
    pub action: String,
    /// The same `node_path` the requirements report and the manifest
    /// classification print, so a row, a record and a verdict about one
    /// node can be lined up by a reader.
    pub node_path: String,
    /// What kind of node the row is, so a consumer does not recover it from
    /// the pseudo-event in `event` or the shape of `node_path`.
    pub kind: RowKind,
    /// Position of the transition within its state: the `N` of
    /// `states.<id>.transitions[N]`. Present on a `transition` row only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    /// The transition's target set, one id per token, as written. Present on
    /// a `transition` row only, and empty for a targetless transition. The
    /// `to` column joins these and may add `type=`; this does not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
}

/// The kind of node a [`TransitionRow`] describes.
///
/// The names are the wire spellings. An action row takes the kind of the
/// site it sits in, nested actions included, for the reason its pseudo-event
/// does: it runs when that site runs.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RowKind {
    State,
    Transition,
    /// An action inside `<onentry>`.
    Entry,
    /// An action inside `<onexit>`.
    Exit,
    /// An action inside a `<transition>`.
    TransitionAction,
    /// An action inside the default transition of `<initial>`.
    Initial,
    /// An action inside the default transition of a `<history>`.
    History,
    Invoke,
    Data,
    /// A top-level `<script>`.
    Script,
}

impl TransitionRow {
    /// Whether this row claims no requirement — the `(none)` block.
    pub fn is_unclaimed(&self) -> bool {
        self.source == NO_SOURCE
    }
}

/// The literal printed in a column the row has no value for.
///
/// `pub` for the same reason [`NO_SOURCE`] is: a caller asking "does
/// this column ever carry anything" has to compare against the
/// producer's own sentinel. A test that spelled `"-"` itself would be
/// checking its own copy, and would keep passing if this one changed —
/// every cell would then differ from the test's literal and read as
/// filled, which is the wrong answer in the safe-looking direction.
pub const EMPTY_CELL: &str = "-";

fn or_dash(value: &str) -> String {
    if value.is_empty() {
        EMPTY_CELL.to_string()
    } else {
        value.to_string()
    }
}

/// The `to` cell of a transition: where it goes, and how it gets there when
/// that is not the default.
///
/// `type` decides whether a transition to a descendant of its source leaves
/// the source (`external`, the default) or does not (`internal`), and so
/// whether the source's `<onexit>` and `<onentry>` run again. A document
/// counting in that `<onentry>` counted 2 after one such transition when it
/// was `external` and 1 when it was `internal`, and this cell was the same
/// for both: the acceptance record's evidence is a digest of these cells, so
/// `acceptance-delta` called the requirement `unchanged`.
/// `an_attribute_that_changes_a_transition_changes_its_row` is what moves
/// every attribute of a transition and requires this row to move with it.
///
/// Written as `name=value`, as the `action` cell writes an attribute, and only
/// when it is not the default, so no row of a document that never wrote the
/// attribute changes and no record taken of one is disturbed.
fn where_the_transition_goes(transition: &Transition) -> String {
    let target = or_dash(&transition.target);
    match transition.transition_type.as_str() {
        "" | "external" => target,
        other => format!("{target} type={other}"),
    }
}

/// Every row of `model`, in document order.
///
/// Built over [`walk_nodes`] — the traversal the requirements report
/// and the manifest comparison also read. A table built from its own
/// walk could disagree with the classification about which nodes
/// exist, and then "a requirement with no row is missing" would be a
/// statement about two different documents.
pub fn transition_table(model: &SCXMLModel) -> Vec<TransitionRow> {
    // The transition rows read their index and target set from the one list
    // of a machine's transitions, not from the model a second time.
    let records = transition_records_by_path(model);
    walk_nodes(model)
        .into_iter()
        .map(|node| {
            let source = if node.record.requirement_ids.is_empty() {
                NO_SOURCE.to_string()
            } else {
                node.record.requirement_ids.join(" ")
            };
            let node_path = node.record.node_path.clone();
            match node.subject {
                NodeSubject::State(state) => TransitionRow {
                    kind: RowKind::State,
                    index: None,
                    targets: None,
                    source,
                    from: state.id.clone(),
                    event: "(state)".to_string(),
                    guard: EMPTY_CELL.to_string(),
                    after: EMPTY_CELL.to_string(),
                    // Where entering this state actually goes, which is
                    // the same question this column answers for a
                    // transition.
                    to: or_dash(&state.initial),
                    action: what_the_state_is(state),
                    node_path,
                },
                NodeSubject::Transition {
                    state, transition, ..
                } => TransitionRow {
                    kind: RowKind::Transition,
                    index: Some(records[&node_path].index),
                    targets: Some(records[&node_path].targets.clone()),
                    source,
                    from: state.id.clone(),
                    // An eventless transition is the SCXML spelling for
                    // "taken as soon as the guard allows", so it is not
                    // a missing value to be dashed out.
                    event: if transition.event.is_empty() {
                        "(eventless)".to_string()
                    } else {
                        transition.event.clone()
                    },
                    guard: or_dash(&transition.cond),
                    after: EMPTY_CELL.to_string(),
                    to: where_the_transition_goes(transition),
                    action: if transition.actions.is_empty() {
                        EMPTY_CELL.to_string()
                    } else {
                        transition
                            .actions
                            .iter()
                            .map(|a| a.action_type.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    },
                    node_path,
                },
                NodeSubject::Action {
                    state,
                    action,
                    site,
                } => TransitionRow {
                    kind: match site {
                        ActionSite::Entry => RowKind::Entry,
                        ActionSite::Exit => RowKind::Exit,
                        ActionSite::Transition { .. } => RowKind::TransitionAction,
                        ActionSite::Initial => RowKind::Initial,
                        ActionSite::HistoryDefault { .. } => RowKind::History,
                    },
                    index: None,
                    targets: None,
                    source,
                    from: state.id.clone(),
                    // Five sites, five pseudo-events. `(transition)`
                    // is not redundant with the transition's own row:
                    // that row summarises its actions in the `action`
                    // column to say what the transition DOES, while
                    // this row is the node an `sce:req` can hang on.
                    // The table's rule is one row per node that can
                    // carry a requirement — without this row, "a
                    // requirement with no row is missing" would call
                    // an annotated transition action missing.
                    event: match site {
                        ActionSite::Entry => "(entry)",
                        ActionSite::Exit => "(exit)",
                        ActionSite::Transition { .. } => "(transition)",
                        ActionSite::Initial => "(initial)",
                        ActionSite::HistoryDefault { .. } => "(history)",
                    }
                    .to_string(),
                    guard: or_dash(&action.cond),
                    after: delay_of(action),
                    to: or_dash(&action.target),
                    action: what_the_action_does(action),
                    node_path,
                },
                NodeSubject::Invoke { state, base, .. } => TransitionRow {
                    kind: RowKind::Invoke,
                    index: None,
                    targets: None,
                    source,
                    from: state.id.clone(),
                    event: "(invoke)".to_string(),
                    guard: EMPTY_CELL.to_string(),
                    after: EMPTY_CELL.to_string(),
                    to: EMPTY_CELL.to_string(),
                    action: or_dash(&base.invoke_id),
                    node_path,
                },
                // §scxml-5.8: the document runs this before entering any
                // state, so `from` names the document rather than a
                // state. `(global script)` is its pseudo-event for the
                // same reason `(invoke)` above is one — it is not
                // event-driven, and leaving the cell empty would read as
                // "an event nobody wrote" instead of "not that kind of
                // row". The action cell goes through the one renderer,
                // so this row moves when the script does.
                NodeSubject::GlobalScript { action, .. } => TransitionRow {
                    kind: RowKind::Script,
                    index: None,
                    targets: None,
                    source,
                    from: "<scxml>".to_string(),
                    event: "(global script)".to_string(),
                    guard: EMPTY_CELL.to_string(),
                    after: EMPTY_CELL.to_string(),
                    to: EMPTY_CELL.to_string(),
                    action: what_the_action_does(action),
                    node_path,
                },
                // §scxml-5.2: a `<data>` is not event-driven either, so it
                // takes a pseudo-event for the reason `(global script)`
                // does. `from` is where it is declared, and the action
                // cell is the value it starts from — the thing a
                // requirement on it is usually about.
                NodeSubject::Variable {
                    state, variable, ..
                } => TransitionRow {
                    kind: RowKind::Data,
                    index: None,
                    targets: None,
                    source,
                    from: state.map_or_else(|| "<scxml>".to_string(), |s| s.id.clone()),
                    event: "(data)".to_string(),
                    guard: EMPTY_CELL.to_string(),
                    after: EMPTY_CELL.to_string(),
                    to: EMPTY_CELL.to_string(),
                    action: if variable.expr.is_empty() {
                        or_dash(&variable.id)
                    } else {
                        format!("{} = {}", variable.id, variable.expr)
                    },
                    node_path,
                },
            }
        })
        .collect()
}

/// The delay an action carries, in the author's own spelling.
///
/// `delay` verbatim before `delayexpr` before the resolved
/// `delay_ms`: the table is read beside a specification that says
/// "3 s", and printing `3000` would make the reader do arithmetic to
/// check a sentence. `delay_ms` is the fallback precisely because it
/// is the one form the author did not write.
fn delay_of(action: &crate::model::Action) -> String {
    if !action.delay.is_empty() {
        action.delay.clone()
    } else if !action.delayexpr.is_empty() {
        action.delayexpr.clone()
    } else if action.delay_ms > 0 {
        format!("{}ms", action.delay_ms)
    } else {
        EMPTY_CELL.to_string()
    }
}

/// What a state IS, for the reason `what_the_action_does` exists.
///
/// ⚠ Named, not placed. This line said "its sibling ABOVE" while that
/// sibling sat below, because the two functions were reordered and a
/// positional reference cannot be reordered with them. A name survives
/// the move; a direction is wrong the moment anything shifts.
///
/// A state's row carried its NAME and nothing else, so every structural
/// attribute an author writes was off the page: which child a compound
/// state enters, whether it is `<parallel>`, whether it is `<final>`,
/// and which history it restores. Change any of them and the machine
/// enters somewhere else, while the row stayed byte-identical — the
/// same shape the action cell had, found the same way, by
/// `an_acceptance_report_moves_when_a_requirement_is_violated` reporting
/// that a requirement cited on a state had nothing on the page that a
/// violation could move.
///
/// `initial` is left to the `to` column, which now carries it: a state's
/// initial child is where entering it actually goes.
pub(crate) fn what_the_state_is(state: &crate::model::State) -> String {
    let mut parts = vec![if state.is_parallel {
        "parallel".to_string()
    } else if state.is_final {
        "final".to_string()
    } else {
        "state".to_string()
    }];
    if !state.initial_children.is_empty() {
        parts.push(format!("enters({})", state.initial_children.join(" ")));
    }
    if !state.initial_history_id.is_empty() {
        parts.push(format!("history={}", state.initial_history_id));
    }
    if !state.initial_history_default_target.is_empty() {
        parts.push(format!(
            "history_default={}",
            state.initial_history_default_target
        ));
    }
    parts.join(" ")
}

/// What an action does, in the attribute names its author wrote.
///
/// ⭐ One rendering, because two readers need the same answer: this table,
/// and the acceptance report's per-requirement fragment (RFC §7a), which
/// has to move under every mutation of what a requirement depends on. The
/// fragment is a SELECTION of these rows rather than a second rendering —
/// `acceptance_report::render` prints the rows its `node_path`s name — so
/// a renderer added there would be free to print less, and the two would
/// then disagree about what one node does.
///
/// The cell used to be the action's kind alone — `send`, `cancel` — and
/// that hid exactly what a timer is made of: the event a `<send>` raises
/// and the `sendid` a `<cancel>` names. Rename either and the transition
/// waiting for the timer never fires, while the table stayed
/// byte-identical. `an_attribute_that_changes_an_action_changes_its_row`
/// mutates every attribute inside executable content and requires this
/// cell to move whenever the parsed action does.
///
/// Every field is printed as `name=value` in the SCXML attribute's own
/// spelling, empty ones omitted, in a fixed order. Three are left to the
/// columns that already carry them rather than printed twice: `target`
/// (`to`), `delay` / `delayexpr` (`after`) and `cond` (`guard`).
///
/// ⚠ Nested actions are not rendered here — each has a row of its own,
/// and its `node_path` says which branch or body it sits in.
pub(crate) fn what_the_action_does(action: &crate::model::Action) -> String {
    let mut parts = vec![action.action_type.clone()];
    let targets = action.targets.join(" ");
    let types = action.types.join(" ");
    let fields: [(&str, &str); 21] = [
        ("name", action.native_action_name.as_str()),
        ("event", action.event.as_str()),
        ("eventexpr", action.eventexpr.as_str()),
        ("id", action.id.as_str()),
        ("idlocation", action.idlocation.as_str()),
        ("sendid", action.sendid.as_str()),
        ("sendidexpr", action.sendidexpr.as_str()),
        ("targetexpr", action.targetexpr.as_str()),
        ("targets", targets.as_str()),
        ("type", action.send_type.as_str()),
        ("typeexpr", action.typeexpr.as_str()),
        ("types", types.as_str()),
        ("namelist", action.namelist.as_str()),
        ("location", action.location.as_str()),
        ("expr", action.expr.as_str()),
        ("label", action.label.as_str()),
        ("array", action.array.as_str()),
        ("item", action.item.as_str()),
        ("index", action.index.as_str()),
        ("contentexpr", action.contentexpr.as_str()),
        ("content", action.content.as_str()),
    ];
    for (name, value) in fields {
        if !value.is_empty() {
            parts.push(format!("{name}={value}"));
        }
    }
    // `<param>` on a `<send>`, `<sce:arg>` on an `<sce:action>`: both
    // parse into `params`, and each pair is part of what the action sends.
    for param in &action.params {
        let mut pair = vec![format!("name={}", param.name)];
        if !param.expr.is_empty() {
            pair.push(format!("expr={}", param.expr));
        }
        if !param.location.is_empty() {
            pair.push(format!("location={}", param.location));
        }
        parts.push(format!("param({})", pair.join(" ")));
    }
    // An `<if>`'s own `cond` is in the guard column; its `<elseif>`
    // conditions decide which nested row runs and have no column. They
    // come from `Action::nested_blocks`, which carries the cond guarding
    // each block — a block with none is not an `<elseif>`.
    for block in action.nested_blocks() {
        if let Some(cond) = block.cond {
            parts.push(format!("elseif({cond})"));
        }
    }
    parts.join(" ")
}

/// Requirement ids the manifest declares that no row claims.
///
/// This is `missing` derived from the table rather than from the
/// classifier, and the two must agree — RFC §6.2's claim that ① and ②
/// are two readings of one export is exactly that agreement, and
/// `transition_table_closure.rs` asserts it rather than assuming it.
///
/// ⚠ `declared` is the ids of the entries for which
/// [`crate::requirement_manifest::RequirementEntry::is_settled_by_annotation`]
/// holds, not of every entry, and the caller does that filtering
/// because only the caller holds the manifest. The agreement above is a
/// statement about one question — *is there a node carrying this id* —
/// and two kinds of entry may not be asked it.
/// [`crate::requirement_manifest::Modality::ShallNot`] entries are met
/// by an absence: passing them in reports a prohibition as `missing`
/// however well it is implemented, which is the false alarm
/// [`crate::requirement_manifest::Outcome::NeedsScenario`] exists to
/// stop. Entries disposed elsewhere
/// ([`crate::requirement_manifest::Disposition`]) are met in another
/// document or the deployment: passing them in reports a delegation as
/// dropped. Either way the two readings would disagree, and the
/// disagreement would be read as drift in the export rather than as the
/// confusion it is.
pub fn requirements_without_a_row<'a>(
    rows: &[TransitionRow],
    declared: impl Iterator<Item = &'a str>,
) -> Vec<String> {
    let claimed: std::collections::BTreeSet<&str> = rows
        .iter()
        .filter(|row| !row.is_unclaimed())
        .flat_map(|row| row.source.split(' '))
        .collect();
    declared
        .filter(|id| !claimed.contains(id))
        .map(str::to_string)
        .collect()
}

/// Emit the table as NDJSON, one record per row.
///
/// NDJSON rather than a rendered grid, for the reason §6.1 gives for
/// having a table at all: it must stay usable at 200 states, and it
/// must be **diffable**. A fixed-width grid re-flows every row when one
/// state is renamed, which makes the diff useless exactly when the
/// document is changing. Rendering for human reading belongs to the
/// acceptance report (§7a), which composes this.
pub fn emit_transition_table_ndjson<W: Write + ?Sized>(
    model: &SCXMLModel,
    writer: &mut W,
) -> io::Result<()> {
    for row in transition_table(model) {
        let line = serde_json::to_string(&row)
            .expect("TransitionRow serialises; every field is an owned String");
        writeln!(writer, "{line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;
    use crate::transition_records::transition_records;

    const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="idle" datamodel="null">
        <state id="idle" sce:req="REQ-S">
            <onentry sce:req="REQ-E"><raise event="e"/></onentry>
            <transition event="go" target="busy" sce:req="12"/>
            <transition event="a b" target="busy done"/>
        </state>
        <state id="busy">
            <transition event="stop"/>
        </state>
        <final id="done"/>
    </scxml>"#;

    fn model() -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(DOC, "transition_table")
            .expect("parse")
    }

    #[test]
    fn a_transition_row_carries_the_index_and_targets_of_its_record() {
        let model = model();
        let records = transition_records(&model);
        let rows: Vec<TransitionRow> = transition_table(&model)
            .into_iter()
            .filter(|row| row.kind == RowKind::Transition)
            .collect();
        assert_eq!(rows.len(), records.len());
        for (row, record) in rows.iter().zip(&records) {
            assert_eq!(row.node_path, record.node_path);
            assert_eq!(row.index, Some(record.index));
            assert_eq!(row.targets.as_ref(), Some(&record.targets));
            assert_eq!(row.from, record.from);
        }
    }

    #[test]
    fn only_a_transition_row_carries_an_index_or_targets() {
        for row in transition_table(&model()) {
            let is_transition = row.kind == RowKind::Transition;
            assert_eq!(row.index.is_some(), is_transition, "{}", row.node_path);
            assert_eq!(row.targets.is_some(), is_transition, "{}", row.node_path);
        }
    }

    #[test]
    fn every_node_of_the_walk_has_its_kind() {
        let kinds: Vec<(String, RowKind)> = transition_table(&model())
            .into_iter()
            .map(|row| (row.node_path, row.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("states.idle".to_string(), RowKind::State),
                (
                    "states.idle.transitions[0]".to_string(),
                    RowKind::Transition
                ),
                (
                    "states.idle.transitions[1]".to_string(),
                    RowKind::Transition
                ),
                (
                    "states.idle.on_entry_blocks[0][0]".to_string(),
                    RowKind::Entry
                ),
                ("states.busy".to_string(), RowKind::State),
                (
                    "states.busy.transitions[0]".to_string(),
                    RowKind::Transition
                ),
                ("states.done".to_string(), RowKind::State),
            ]
        );
    }

    #[test]
    fn the_new_fields_follow_the_existing_columns_on_the_wire() {
        let mut buf = Vec::new();
        emit_transition_table_ndjson(&model(), &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let line = out
            .lines()
            .find(|l| l.contains("\"node_path\":\"states.idle.transitions[1]\""))
            .expect("the second transition of idle has a row");
        assert!(
            line.starts_with(r#"{"source":"(none)","from":"idle","event":"a b","guard":"-","after":"-","to":"busy done","action":"-","node_path":"states.idle.transitions[1]","#),
            "the pre-existing columns keep their text and order: {line}"
        );
        assert!(
            line.ends_with(r#""kind":"transition","index":1,"targets":["busy","done"]}"#),
            "the additive fields follow them: {line}"
        );
    }
}
