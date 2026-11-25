// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document of the static class printed as its own step function
//! (docs/adr/0129): the arms the generator writes, computed here from the
//! document's tables, and the refusal of a document outside the class.
//!
//! The class is the one whose every step is fixed when the document is built.
//! It has no `<parallel>`, so a configuration is one atomic state and its
//! ancestors and a selection takes one transition at most; no `<history>`, so
//! what a transition enters depends on the transition alone; no transition of
//! more than one target, so what it enters is one path down the tree (ADR 0125
//! computes that path, [`DocumentTables::entry_paths`]); no `<invoke>`, so a
//! macrostep starts no session. Its internal queue is the engine's in both
//! forms, of the depth the build gives it (proven from the document by
//! docs/adr/0076, or the external rule where none is proven), so the class
//! asks nothing more of it.
//!
//! In that class what a transition exits from a given atomic state is fixed
//! too: every active state below the transition's domain, which is the atomic
//! state and its ancestors up to the domain. So a step function is four
//! tables of constants, each written out as the arms of one `match`:
//!
//! - for each atomic state, the states whose transitions a selection asks in
//!   order, the state first and its ancestors outward ([`Selection`]);
//! - for each written transition and each atomic state it can be selected
//!   from, the states it exits, whether its content runs, and the states it
//!   enters ([`Microstep`]);
//! - for each atomic state, the states the end of the run exits ([`Exit`]);
//! - the states the document's initial transition enters ([`Arm`]).
//!
//! Every list is in the order Appendix D takes the steps: exits atomic state
//! first, which is exit order, and entries in document order, which is entry
//! order. The shared interpreter computes the same lists from the same rows
//! at run time; the differential test of docs/adr/0129 holds the two forms of
//! one document to each other.

use serde::Serialize;

use crate::document_tables::{DocumentTables, PathStep, StateKind, TransitionOrigin};

/// What puts a document outside the class a step function serves, each named
/// by the element it is about where one is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutsideClass {
    /// A `<parallel>`, whose regions make a configuration more than one path
    /// down the tree.
    Parallel {
        /// The `<parallel>`'s id.
        state: String,
    },
    /// A `<history>`, whose record decides at run time what a transition to
    /// it enters.
    History {
        /// The `<history>`'s id.
        history: String,
        /// The id of the state it is declared in.
        parent: String,
    },
    /// An `<invoke>`, whose child sessions a step function does not start.
    Invoke,
    /// A transition of more than one target, which enters more than one path.
    TargetSet {
        /// The id of the state the transition is written in.
        source: String,
    },
    /// A compound state whose initial transition does not enter one of its own
    /// descendants, so what entering it enters is not one path down the tree.
    EntryNotAPath,
}

impl OutsideClass {
    /// The reason, as the refusal states it.
    pub fn describe(&self) -> String {
        match self {
            Self::Parallel { state } => format!(
                "the <parallel> `{state}`, whose regions make a configuration more than one path \
                 down the tree"
            ),
            Self::History { history, .. } => format!(
                "the <history> `{history}`, whose record decides what a transition to it enters"
            ),
            Self::Invoke => {
                "an <invoke>, whose sessions a step function does not start".to_string()
            }
            Self::TargetSet { source } => {
                format!("a transition of `{source}` with more than one target")
            }
            Self::EntryNotAPath => "a compound state whose initial transition does not enter one \
                                    of its own descendants"
                .to_string(),
        }
    }

    /// The id of the state the reason is about, for the refusal to point at:
    /// the `<parallel>` itself, the state a `<history>` or a transition is
    /// written in. `None` for a reason no one state answers.
    pub fn state(&self) -> Option<&str> {
        match self {
            Self::Parallel { state } => Some(state),
            Self::History { parent, .. } => Some(parent),
            Self::TargetSet { source } => Some(source),
            Self::Invoke | Self::EntryNotAPath => None,
        }
    }
}

/// One state an arm enters, by id, and whether it is entered by default, so
/// its initial transition's content runs after its `<onentry>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// The state's id.
    pub state: String,
    /// Whether it is entered by default.
    pub by_default: bool,
}

/// What one step does, in order: the states it exits, whether the
/// transition's content runs, the states it enters, the atomic state it leaves
/// the machine in and whether that state is a `<final>` child of `<scxml>`,
/// whose entry ends the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Arm {
    /// The states exited, atomic state first.
    pub exits: Vec<String>,
    /// Whether the transition's executable content runs.
    pub content: bool,
    /// The states entered, in entry order.
    pub entries: Vec<Entry>,
    /// The atomic state the step leaves the machine in; `None` when it
    /// enters nothing.
    pub current: Option<String>,
    /// Whether `current` is a `<final>` child of `<scxml>`.
    pub ends_the_run: bool,
}

/// The selection from one atomic state: the states asked for an enabled
/// transition, in order. A state without a transition of its own is not
/// asked, and an atomic state none of whose states has one has no selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Selection {
    /// The atomic state.
    pub atomic: String,
    /// The atomic state and its ancestors that have transitions, innermost
    /// first.
    pub asked: Vec<String>,
}

/// The microstep of one written transition, from one atomic state below its
/// source, or from any when it exits nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Microstep {
    /// The transition's number in the document's tables.
    pub transition: u16,
    /// The atomic state the transition is taken from; `None` for a
    /// transition with no target, which does the same from every one.
    pub atomic: Option<String>,
    /// What the step does.
    pub arm: Arm,
}

/// What the end of a run exits from one atomic state: the configuration,
/// atomic state first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Exit {
    /// The atomic state.
    pub atomic: String,
    /// The states exited, atomic state first.
    pub exits: Vec<String>,
}

/// A document's step function, as the generator writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepFunction {
    /// The entry into the initial configuration.
    pub initial: Arm,
    /// The selection from each atomic state that has one, in document order.
    pub selections: Vec<Selection>,
    /// The microstep of each written transition, by transition and then by
    /// atomic state in document order.
    pub microsteps: Vec<Microstep>,
    /// The exit of the configuration from each atomic state, in document
    /// order.
    pub exits: Vec<Exit>,
}

impl StepFunction {
    /// The step function of the document whose tables are `tables`, or every
    /// reason it is outside the class. `has_invoke` is whether it has an
    /// `<invoke>`, which is not a fact of the tables.
    pub fn of(tables: &DocumentTables, has_invoke: bool) -> Result<Self, Vec<OutsideClass>> {
        let mut outside = Vec::new();
        if let Some(row) = tables
            .states
            .iter()
            .find(|row| row.kind == StateKind::Parallel)
        {
            outside.push(OutsideClass::Parallel {
                state: row.id.clone(),
            });
        }
        if let Some(history) = tables.histories.first() {
            outside.push(OutsideClass::History {
                history: history.id.clone(),
                parent: tables.states[usize::from(history.parent)].id.clone(),
            });
        }
        if has_invoke {
            outside.push(OutsideClass::Invoke);
        }
        if let Some(source) = tables
            .transitions
            .iter()
            .find(|row| row.targets.len > 1)
            .and_then(|row| row.source)
        {
            outside.push(OutsideClass::TargetSet {
                source: tables.states[usize::from(source)].id.clone(),
            });
        }
        let paths = tables.entry_paths();
        if paths.is_none() && tables.fixes_its_sets() {
            outside.push(OutsideClass::EntryNotAPath);
        }
        match paths {
            Some(paths) if outside.is_empty() => Ok(Self::from_paths(tables, &paths)),
            _ => Err(outside),
        }
    }

    /// The step function of a document in the class, whose entry paths are
    /// `paths`.
    fn from_paths(tables: &DocumentTables, paths: &[Vec<PathStep>]) -> Self {
        let id = |state: u16| tables.states[usize::from(state)].id.clone();
        let parent = |state: u16| tables.states[usize::from(state)].parent;
        // The state and its ancestors, innermost first.
        let chain = |state: u16| {
            let mut chain = vec![state];
            let mut current = parent(state);
            while let Some(ancestor) = current {
                chain.push(ancestor);
                current = parent(ancestor);
            }
            chain
        };
        let atomic: Vec<u16> = (0..tables.states.len())
            .filter(|&index| {
                matches!(
                    tables.states[index].kind,
                    StateKind::Atomic | StateKind::Final
                )
            })
            .map(|index| u16::try_from(index).expect("the tables number every state as a u16"))
            .collect();
        let has_transitions = |state: u16| {
            tables.transitions.iter().any(|row| {
                matches!(row.origin, TransitionOrigin::Written { .. }) && row.source == Some(state)
            })
        };
        // What entering a path does, after `exits` are exited and the
        // content, when `content`, has run.
        let arm = |exits: Vec<String>, content: bool, path: &[PathStep]| {
            let last = path.last().map(|step| step.state);
            Arm {
                exits,
                content,
                entries: path
                    .iter()
                    .map(|step| Entry {
                        state: id(step.state),
                        by_default: step.by_default,
                    })
                    .collect(),
                current: last.map(id),
                ends_the_run: last.is_some_and(|state| {
                    let row = &tables.states[usize::from(state)];
                    row.kind == StateKind::Final && row.parent.is_none()
                }),
            }
        };

        let selections = atomic
            .iter()
            .filter_map(|&state| {
                let asked: Vec<String> = chain(state)
                    .into_iter()
                    .filter(|&asked| has_transitions(asked))
                    .map(id)
                    .collect();
                (!asked.is_empty()).then(|| Selection {
                    atomic: id(state),
                    asked,
                })
            })
            .collect();

        let mut microsteps = Vec::new();
        for (number, row) in tables.transitions.iter().enumerate() {
            let (TransitionOrigin::Written { .. }, Some(source)) = (row.origin, row.source) else {
                continue;
            };
            let transition =
                u16::try_from(number).expect("the tables number every transition as a u16");
            let path = &paths[number];
            let Some(first) = path.first() else {
                // No target: the content alone, whatever the atomic state.
                microsteps.push(Microstep {
                    transition,
                    atomic: None,
                    arm: arm(Vec::new(), row.has_content, path),
                });
                continue;
            };
            // Every active state below the domain, which is the parent of the
            // path's first state, exits: from an atomic state below the
            // source, that state and its ancestors up to the domain.
            let domain = parent(first.state);
            let last_below_source = tables.states[usize::from(source)].last_descendant;
            for &from in atomic
                .iter()
                .filter(|&&state| source <= state && state <= last_below_source)
            {
                let exits = chain(from)
                    .into_iter()
                    .take_while(|&state| Some(state) != domain)
                    .map(id)
                    .collect();
                microsteps.push(Microstep {
                    transition,
                    atomic: Some(id(from)),
                    arm: arm(exits, row.has_content, path),
                });
            }
        }

        let exits = atomic
            .iter()
            .map(|&state| Exit {
                atomic: id(state),
                exits: chain(state).into_iter().map(id).collect(),
            })
            .collect();

        Self {
            initial: arm(Vec::new(), false, &paths[usize::from(tables.initial)]),
            selections,
            microsteps,
            exits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SCXMLModel;
    use crate::parser::SCXMLParser;

    /// `body` as a document of the null data model, parsed and analysed as
    /// the generator receives it.
    fn tables_of(body: &str) -> (SCXMLModel, DocumentTables) {
        let text = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null"{body}</scxml>"#
        );
        let mut model = SCXMLParser::new()
            .parse_string(&text, "step_function")
            .expect("parse");
        crate::analyzer::analyze(&mut model, "");
        let tables = DocumentTables::of(&model).expect("tables");
        (model, tables)
    }

    fn entry(state: &str, by_default: bool) -> Entry {
        Entry {
            state: state.to_string(),
            by_default,
        }
    }

    fn names(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// The document every arm below is read from: a compound state with an
    /// atomic child it enters by default and a second it does not, a
    /// transition held by the compound state, an internal transition into a
    /// child of its source, a targetless one, and a top-level final.
    const NESTED: &str = r#" initial="session">
      <state id="session" initial="idle">
        <transition event="abort" target="closed"/>
        <transition event="pause" type="internal" target="held"/>
        <state id="idle">
          <onentry><log label="idle"/></onentry>
          <transition event="go" target="held"><log label="go"/></transition>
          <transition event="tick"/>
        </state>
        <state id="held"/>
      </state>
      <final id="closed"/>
    "#;

    #[test]
    fn the_initial_arm_enters_the_default_path_down_to_an_atomic_state() {
        let (_, tables) = tables_of(NESTED);
        let function = StepFunction::of(&tables, false).expect("in the class");
        assert_eq!(
            function.initial,
            Arm {
                exits: Vec::new(),
                content: false,
                entries: vec![entry("session", true), entry("idle", false)],
                current: Some("idle".to_string()),
                ends_the_run: false,
            }
        );
    }

    #[test]
    fn a_selection_asks_the_atomic_state_then_each_ancestor_with_transitions() {
        let (_, tables) = tables_of(NESTED);
        let function = StepFunction::of(&tables, false).expect("in the class");
        assert_eq!(
            function.selections,
            vec![
                Selection {
                    atomic: "idle".to_string(),
                    asked: names(&["idle", "session"]),
                },
                Selection {
                    atomic: "held".to_string(),
                    asked: names(&["session"]),
                },
            ],
            "`closed` and its ancestors have no transition, so it has no selection",
        );
    }

    #[test]
    fn a_transition_of_a_compound_source_has_an_arm_for_each_atomic_state_below_it() {
        let (_, tables) = tables_of(NESTED);
        let function = StepFunction::of(&tables, false).expect("in the class");
        let abort: Vec<&Microstep> = function
            .microsteps
            .iter()
            .filter(|step| step.arm.current.as_deref() == Some("closed"))
            .collect();
        assert_eq!(abort.len(), 2, "one arm from `idle`, one from `held`");
        assert_eq!(abort[0].atomic.as_deref(), Some("idle"));
        assert_eq!(abort[0].arm.exits, names(&["idle", "session"]));
        assert_eq!(abort[1].atomic.as_deref(), Some("held"));
        assert_eq!(abort[1].arm.exits, names(&["held", "session"]));
        assert_eq!(abort[0].arm.entries, vec![entry("closed", false)]);
        assert!(
            abort[0].arm.ends_the_run,
            "`closed` is a <final> child of <scxml>"
        );
    }

    #[test]
    fn an_internal_transition_exits_only_below_its_source() {
        let (_, tables) = tables_of(NESTED);
        let function = StepFunction::of(&tables, false).expect("in the class");
        let pause: Vec<&Microstep> = function
            .microsteps
            .iter()
            .filter(|step| {
                step.arm.current.as_deref() == Some("held")
                    && step.arm.exits.last().map(String::as_str) != Some("session")
                    && !step.arm.content
            })
            .collect();
        assert_eq!(pause.len(), 2, "one arm from `idle`, one from `held`");
        assert_eq!(pause[0].arm.exits, names(&["idle"]));
        assert_eq!(pause[1].arm.exits, names(&["held"]));
        assert_eq!(pause[0].arm.entries, vec![entry("held", false)]);
    }

    #[test]
    fn an_external_transition_to_a_sibling_runs_its_content_between_exit_and_entry() {
        let (_, tables) = tables_of(NESTED);
        let function = StepFunction::of(&tables, false).expect("in the class");
        let go: Vec<&Microstep> = function
            .microsteps
            .iter()
            .filter(|step| step.arm.content && step.arm.current.is_some())
            .collect();
        assert_eq!(go.len(), 1, "`go` is written in the atomic `idle` alone");
        assert_eq!(go[0].atomic.as_deref(), Some("idle"));
        assert_eq!(go[0].arm.exits, names(&["idle"]));
        assert_eq!(go[0].arm.entries, vec![entry("held", false)]);
    }

    #[test]
    fn a_targetless_transition_has_one_arm_that_exits_and_enters_nothing() {
        let (_, tables) = tables_of(NESTED);
        let function = StepFunction::of(&tables, false).expect("in the class");
        let tick: Vec<&Microstep> = function
            .microsteps
            .iter()
            .filter(|step| step.atomic.is_none())
            .collect();
        assert_eq!(tick.len(), 1);
        assert_eq!(
            tick[0].arm,
            Arm {
                exits: Vec::new(),
                content: false,
                entries: Vec::new(),
                current: None,
                ends_the_run: false,
            }
        );
    }

    #[test]
    fn the_end_of_a_run_exits_the_atomic_state_then_its_ancestors() {
        let (_, tables) = tables_of(NESTED);
        let function = StepFunction::of(&tables, false).expect("in the class");
        assert_eq!(
            function.exits,
            vec![
                Exit {
                    atomic: "idle".to_string(),
                    exits: names(&["idle", "session"]),
                },
                Exit {
                    atomic: "held".to_string(),
                    exits: names(&["held", "session"]),
                },
                Exit {
                    atomic: "closed".to_string(),
                    exits: names(&["closed"]),
                },
            ]
        );
    }

    #[test]
    fn a_parallel_puts_a_document_outside_the_class() {
        let (_, tables) = tables_of(
            r#" initial="p">
              <parallel id="p">
                <state id="a"/>
                <state id="b"/>
              </parallel>
            "#,
        );
        assert_eq!(
            StepFunction::of(&tables, false),
            Err(vec![OutsideClass::Parallel {
                state: "p".to_string()
            }])
        );
    }

    #[test]
    fn a_history_puts_a_document_outside_the_class() {
        let (_, tables) = tables_of(
            r#" initial="s">
              <state id="s" initial="a">
                <history id="h"><transition target="a"/></history>
                <state id="a"><transition event="e" target="b"/></state>
                <state id="b"><transition event="back" target="h"/></state>
              </state>
            "#,
        );
        let refusal = StepFunction::of(&tables, false).expect_err("outside the class");
        assert_eq!(
            refusal,
            vec![OutsideClass::History {
                history: "h".to_string(),
                parent: "s".to_string(),
            }]
        );
        assert_eq!(refusal[0].state(), Some("s"));
    }

    #[test]
    fn an_invoke_puts_a_document_outside_the_class() {
        let (_, tables) = tables_of(NESTED);
        assert_eq!(
            StepFunction::of(&tables, true),
            Err(vec![OutsideClass::Invoke])
        );
    }

    #[test]
    fn a_reason_names_the_element_it_is_about() {
        let parallel = OutsideClass::Parallel {
            state: "p".to_string(),
        };
        assert!(parallel.describe().contains("<parallel> `p`"));
        assert_eq!(parallel.state(), Some("p"));
        let history = OutsideClass::History {
            history: "h".to_string(),
            parent: "s".to_string(),
        };
        assert!(history.describe().contains("<history> `h`"));
        assert_eq!(history.state(), Some("s"));
        let targets = OutsideClass::TargetSet {
            source: "t".to_string(),
        };
        assert!(targets.describe().contains("`t`"));
        assert_eq!(targets.state(), Some("t"));
        assert_eq!(OutsideClass::Invoke.state(), None);
        assert_eq!(OutsideClass::EntryNotAPath.state(), None);
    }
}
