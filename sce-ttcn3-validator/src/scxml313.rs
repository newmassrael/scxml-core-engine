// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The exit and entry sets of a transition of a test component, computed
//! from its statechart alone, for the third check of
//! docs/SCE_TEST_PROFILE.md §9.4.
//!
//! §scxml-3.13: a transition's domain is the source when it is internal,
//! its source compound and every target a descendant of the source, and
//! otherwise the least common compound ancestor of the source and the
//! targets; it exits the active states below the domain, deepest first, and
//! enters the states from the domain down to each target, and below a
//! compound target its default entry, outermost first. This is §scxml-D's
//! `getTransitionDomain`, `findLCCA`, `computeExitSet` and `computeEntrySet`
//! with `addDescendantStatesToEnter` and `addAncestorStatesToEnter`, for
//! the statechart a test component is: no `<parallel>` and no `<history>`
//! (docs/SCE_TEST_PROFILE.md §8.2, which the reader refuses by name), so its
//! configuration is one atomic state and the ancestors of it, and the exit
//! and entry sets of a transition taken from a given atomic state are fixed.
//!
//! It is written here again, apart from the core the exporter asks
//! (`sce_rust_runtime::helpers::microstep`), so the validator does not share
//! the reading it checks.

use std::collections::BTreeSet;

use crate::documents::{ChartTransition, Statechart};

/// The state a transition's exits and entries stay below: the `<scxml>`
/// element, or a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Document,
    State(usize),
}

/// What taking one transition does to the configuration of an atomic state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The states exited, in exit order: deepest first.
    pub exited: Vec<usize>,
    /// The states entered, in entry order: document order.
    pub entered: Vec<usize>,
    /// The entered states entered by default: each compound state whose
    /// initial transition runs after its onentry.
    pub default_entry: BTreeSet<usize>,
    /// The atomic state the machine is left in.
    pub leaf: usize,
}

/// Whether `state` is a proper descendant of `of` (§scxml-D `isDescendant`);
/// every state is one of the `<scxml>` element.
pub fn is_descendant(chart: &Statechart, state: usize, of: Domain) -> bool {
    match of {
        Domain::Document => true,
        Domain::State(ancestor) => chart
            .chain(state)
            .into_iter()
            .skip(1)
            .any(|s| s == ancestor),
    }
}

/// The proper ancestors of `state` below `up_to`, innermost first
/// (§scxml-D `getProperAncestors`).
fn proper_ancestors(chart: &Statechart, state: usize, up_to: Domain) -> Vec<usize> {
    chart
        .chain(state)
        .into_iter()
        .skip(1)
        .take_while(|&ancestor| Domain::State(ancestor) != up_to)
        .collect()
}

/// The domain of a transition of `source` to `targets`, `internal` where it
/// says so (§scxml-D `getTransitionDomain`): none for a targetless one.
pub fn domain(
    chart: &Statechart,
    source: usize,
    targets: &[usize],
    internal: bool,
) -> Option<Domain> {
    if targets.is_empty() {
        return None;
    }
    let compound = !chart.states[source].children.is_empty();
    let inside = targets
        .iter()
        .all(|&target| is_descendant(chart, target, Domain::State(source)));
    if internal && compound && inside {
        return Some(Domain::State(source));
    }
    Some(lcca(chart, source, targets))
}

/// The least common compound ancestor of `source` and `targets`
/// (§scxml-D `findLCCA`): the innermost proper ancestor of the source of
/// which every target is a descendant, or the `<scxml>` element. A proper
/// ancestor holds a child, so with no `<parallel>` every one is compound.
fn lcca(chart: &Statechart, source: usize, targets: &[usize]) -> Domain {
    proper_ancestors(chart, source, Domain::Document)
        .into_iter()
        .find(|&ancestor| {
            targets
                .iter()
                .all(|&target| is_descendant(chart, target, Domain::State(ancestor)))
        })
        .map_or(Domain::Document, Domain::State)
}

/// The states and default entries an entry adds, each once
/// (§scxml-D `computeEntrySet`).
struct Entry<'c> {
    chart: &'c Statechart,
    states: BTreeSet<usize>,
    default_entry: BTreeSet<usize>,
}

impl<'c> Entry<'c> {
    fn new(chart: &'c Statechart) -> Self {
        Self {
            chart,
            states: BTreeSet::new(),
            default_entry: BTreeSet::new(),
        }
    }

    /// `state`, and below a compound one its default entry
    /// (§scxml-D `addDescendantStatesToEnter`).
    fn add_descendants(&mut self, state: usize) {
        self.states.insert(state);
        let chart = self.chart;
        let initial = &chart.states[state].initial;
        if chart.states[state].children.is_empty() {
            return;
        }
        self.default_entry.insert(state);
        for &target in initial {
            self.add_descendants(target);
        }
        for &target in initial {
            self.add_ancestors(target, Domain::State(state));
        }
    }

    /// The proper ancestors of `state` below `ancestor`
    /// (§scxml-D `addAncestorStatesToEnter`).
    fn add_ancestors(&mut self, state: usize, ancestor: Domain) {
        self.states
            .extend(proper_ancestors(self.chart, state, ancestor));
    }

    /// Each of `targets` entered from `domain`.
    fn targets(&mut self, targets: &[usize], domain: Domain) {
        for &target in targets {
            self.add_descendants(target);
        }
        for &target in targets {
            self.add_ancestors(target, domain);
        }
    }
}

/// What `transition`, of the state `source`, does when taken from the
/// configuration whose atomic state is `leaf` (§scxml-3.13).
pub fn take(chart: &Statechart, leaf: usize, source: usize, transition: &ChartTransition) -> Taken {
    let domain = domain(chart, source, &transition.targets, transition.internal);
    // §scxml-D `computeExitSet`: the active states below the domain, in exit
    // order, which the configuration of one atomic state lists innermost
    // first.
    let exited: Vec<usize> = match domain {
        Some(domain) => chart
            .chain(leaf)
            .into_iter()
            .filter(|&state| is_descendant(chart, state, domain))
            .collect(),
        None => Vec::new(),
    };
    let mut entry = Entry::new(chart);
    if let Some(domain) = domain {
        entry.targets(&transition.targets, domain);
    }
    let left: Vec<usize> = chart
        .chain(leaf)
        .into_iter()
        .filter(|state| !exited.contains(state))
        .collect();
    finish(chart, entry, exited, &left, leaf)
}

/// The machine started: the document's initial transition, which has no
/// source and no content, taken from no configuration.
pub fn start(chart: &Statechart) -> Option<Taken> {
    let mut entry = Entry::new(chart);
    entry.targets(&chart.initial, Domain::Document);
    let entered_atomic = entry
        .states
        .iter()
        .rev()
        .copied()
        .find(|&state| chart.states[state].children.is_empty())?;
    Some(finish(chart, entry, Vec::new(), &[], entered_atomic))
}

/// The states the machine exits as it halts in the configuration whose
/// atomic state is `leaf` (§scxml-D `exitInterpreter`), in exit order.
pub fn halt(chart: &Statechart, leaf: usize) -> Vec<usize> {
    chart.chain(leaf)
}

/// The sets, in their orders, and the atomic state of the configuration
/// `left` of the old one and the states entered make; `leaf` where nothing
/// is entered.
fn finish(
    chart: &Statechart,
    entry: Entry<'_>,
    exited: Vec<usize>,
    left: &[usize],
    leaf: usize,
) -> Taken {
    let entered: Vec<usize> = entry.states.into_iter().collect();
    let atomic = entered
        .iter()
        .chain(left)
        .copied()
        .find(|&state| chart.states[state].children.is_empty())
        .unwrap_or(leaf);
    Taken {
        exited,
        entered,
        default_entry: entry.default_entry,
        leaf: atomic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::statechart_tests::suite_of;

    /// `outer` holds `first`, `second` and, below `second`, `deep`; `side`
    /// is a sibling of `outer` with a default entry whose initial
    /// transition writes content; `done` is a top-level `<final>`.
    const COMPONENT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-component" name="c"
       datamodel="sce-static">
  <sce:test-port id="sut" type="Port"/>
  <state id="outer" initial="first">
    <transition sce:receive="sut" type="internal" target="first"/>
    <transition sce:receive="sut" target="first"/>
    <state id="first">
      <transition sce:receive="sut" target="deep"/>
      <transition sce:receive="sut"/>
    </state>
    <state id="second">
      <state id="deep">
        <transition sce:receive="sut" target="side"/>
        <transition sce:receive="sut" target="second"/>
      </state>
    </state>
  </state>
  <state id="side">
    <initial><transition target="inner"><log label="in"/></transition></initial>
    <state id="inner"/>
  </state>
  <final id="done"/>
</scxml>"#;

    fn chart() -> Statechart {
        suite_of(COMPONENT)
            .expect("documents")
            .document("c.scxml")
            .and_then(|document| document.statechart.clone())
            .expect("the statechart")
    }

    fn at(chart: &Statechart, id: &str) -> usize {
        chart.state(id).expect("a state")
    }

    fn ids(chart: &Statechart, states: &[usize]) -> Vec<String> {
        states.iter().map(|&s| chart.states[s].id.clone()).collect()
    }

    /// Taking the `index`-th transition of `source` from `leaf`: the
    /// exited, the entered and the atomic state left in, by their ids.
    fn taken(
        chart: &Statechart,
        leaf: &str,
        source: &str,
        index: usize,
    ) -> (Vec<String>, Vec<String>, String) {
        let source = at(chart, source);
        let taken = take(
            chart,
            at(chart, leaf),
            source,
            &chart.states[source].transitions[index],
        );
        (
            ids(chart, &taken.exited),
            ids(chart, &taken.entered),
            chart.states[taken.leaf].id.clone(),
        )
    }

    /// The machine starts in the root's first state, and enters it and its
    /// default entry outermost first.
    #[test]
    fn the_start_enters_the_first_state_and_its_default_entry() {
        let chart = chart();
        let started = start(&chart).expect("a start");
        assert!(started.exited.is_empty());
        assert_eq!(ids(&chart, &started.entered), ["outer", "first"]);
        assert_eq!(chart.states[started.leaf].id, "first");
        assert_eq!(started.default_entry, BTreeSet::from([at(&chart, "outer")]));
    }

    /// A transition between two states below `outer` has `outer` for its
    /// domain, the least common compound ancestor, which is not the root:
    /// it exits `first` alone and enters `second` and `deep`, and `second`
    /// is not entered by default, since the target is below it.
    #[test]
    fn a_transition_inside_a_compound_state_stays_below_it() {
        let chart = chart();
        let first = at(&chart, "first");
        let transition = &chart.states[first].transitions[0];
        assert_eq!(
            domain(&chart, first, &transition.targets, false),
            Some(Domain::State(at(&chart, "outer")))
        );
        assert_eq!(
            taken(&chart, "first", "first", 0),
            (
                vec!["first".to_string()],
                vec!["second".to_string(), "deep".to_string()],
                "deep".to_string()
            )
        );
        let taken = take(&chart, first, first, transition);
        assert!(taken.default_entry.is_empty());
    }

    /// An internal transition of a compound state to a descendant keeps
    /// the state; the same transition external exits and enters it again.
    #[test]
    fn an_internal_transition_keeps_its_source_and_an_external_one_leaves_it() {
        let chart = chart();
        assert_eq!(
            taken(&chart, "deep", "outer", 0),
            (
                vec!["deep".to_string(), "second".to_string()],
                vec!["first".to_string()],
                "first".to_string()
            )
        );
        assert_eq!(
            taken(&chart, "deep", "outer", 1),
            (
                vec![
                    "deep".to_string(),
                    "second".to_string(),
                    "outer".to_string()
                ],
                vec!["outer".to_string(), "first".to_string()],
                "first".to_string()
            )
        );
    }

    /// A compound target is entered by default, down its initial
    /// transition to an atomic state, outermost first; leaving a nested
    /// state for a sibling of its ancestor exits every ancestor below the
    /// root, innermost first.
    #[test]
    fn a_compound_target_is_entered_by_its_default_entry() {
        let chart = chart();
        let deep = at(&chart, "deep");
        let taken_out = take(&chart, deep, deep, &chart.states[deep].transitions[0]);
        assert_eq!(ids(&chart, &taken_out.exited), ["deep", "second", "outer"]);
        assert_eq!(ids(&chart, &taken_out.entered), ["side", "inner"]);
        assert_eq!(chart.states[taken_out.leaf].id, "inner");
        assert_eq!(
            taken_out.default_entry,
            BTreeSet::from([at(&chart, "side")])
        );
        // A transition of `deep` to its own parent exits both and enters
        // them again, `second` by default.
        assert_eq!(
            taken(&chart, "deep", "deep", 1),
            (
                vec!["deep".to_string(), "second".to_string()],
                vec!["second".to_string(), "deep".to_string()],
                "deep".to_string()
            )
        );
    }

    /// A targetless transition exits and enters nothing, and leaves the
    /// machine where it was.
    #[test]
    fn a_targetless_transition_exits_and_enters_nothing() {
        let chart = chart();
        let first = at(&chart, "first");
        assert_eq!(domain(&chart, first, &[], false), None);
        assert_eq!(
            taken(&chart, "first", "first", 1),
            (Vec::new(), Vec::new(), "first".to_string())
        );
        assert_eq!(
            ids(&chart, &halt(&chart, at(&chart, "deep"))),
            ["deep", "second", "outer"]
        );
    }
}
