// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a document's state tree bounds in the Rust runtime's microstep.
//!
//! Appendix D's procedures hold sets of states and of transitions, and a
//! bare-metal build has no allocator to grow them with. The runtime names no
//! size of its own for them: each is the size this document needs, read here
//! from its tree and written on the generated policy
//! (`StatePolicy::MAX_CONFIGURATION` and the rest), so a machine of three flat
//! states holds three-slot buffers and one with a wide `<parallel>` holds as many
//! as its regions need.
//!
//! Each number is an upper bound that follows from the tree and not an estimate:
//!
//! - a configuration (§scxml-3.4) holds one root-to-leaf path, with every region
//!   of each `<parallel>` on it. Its size is
//!   `1 + max(children)` for a compound state, `1 + sum(children)` for a
//!   `<parallel>` and `1` for an atomic state, taken over the `<scxml>` element's
//!   children, of which one is active;
//! - a selection (§scxml-D-selectTransitions) takes at most one transition from
//!   each atomic state of a configuration, so its bound is the same walk counting
//!   atomic states only;
//! - the exit set, the entry set, a list of effective targets and a history's
//!   record are sets of states that are, or are about to be, in one
//!   configuration, so they share the configuration's bound;
//! - a microstep owes the default transition of a `<history>` once for each
//!   parent that has one, so that is how many it can hold;
//! - a walk up the hierarchy reaches the `<scxml>` element in as many steps as
//!   its start sits levels deep, a `<history>` being one level below its parent.

use std::collections::BTreeMap;

use crate::model::SCXMLModel;

/// The bounds of one document, in the units the runtime's buffers are counted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MicrostepBounds {
    /// The most states one configuration holds (`StatePolicy::MAX_CONFIGURATION`).
    pub configuration: usize,
    /// The most transitions one selection enables (`StatePolicy::MAX_ENABLED`).
    pub enabled: usize,
    /// The most `<history>` default transitions a microstep owes
    /// (the capacity of `StatePolicy::HistoryDefaults`).
    pub history_defaults: usize,
    /// How many levels the deepest state or `<history>` is at
    /// (`StatePolicy::MAX_DEPTH`), the `<scxml>` element's child being at 1.
    pub depth: usize,
    /// The most steps the entry walks have owed at once (the capacity of
    /// `StatePolicy::EntryWalk`): `3 * depth + histories + 3` for a document
    /// with a `<parallel>`, `2 * depth + histories + 3` for one without.
    ///
    /// Appendix D's recursions are run over a stack of steps, and a step that has
    /// more to do puts itself back with its next index before the step it spawns,
    /// so a stack holds one step for each recursion level, never a list of
    /// siblings. Every level lies strictly inside the subtree of the level above
    /// it (a target's descendants, its ancestors below the state that named it,
    /// the regions of a `<parallel>` among them), so there are at most `depth`
    /// levels. A level holds a list's resumption, the resumption of the walk up
    /// and, where a `<parallel>` is entered, the resumption of the region loop:
    /// three, or two in a document with no `<parallel>`, which no step of the
    /// region loop is ever put down for. A `<history>` adds the resumption of its
    /// own list at the level of its parent's children, which is one more for each
    /// of them, and the step being taken puts down at most two at once, with one
    /// in hand: three of headroom.
    pub entry_walk: usize,
}

impl MicrostepBounds {
    /// The bounds of `model`'s state tree.
    pub fn of(model: &SCXMLModel) -> Self {
        // The tree is read from each state's parent, the one fact the parser
        // writes first, so no field derived from it can disagree with it.
        let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut top_level: Vec<&str> = Vec::new();
        for (id, state) in &model.states {
            match state.parent.as_deref() {
                Some(parent) => children.entry(parent).or_default().push(id),
                None => top_level.push(id),
            }
        }

        let tree = Tree {
            model,
            children: &children,
        };
        // Which of the `<scxml>` element's children is active is the document's
        // choice, and any may be, so the bound is the largest of them.
        let configuration = top_level.iter().map(|id| tree.configuration(id)).max();
        let enabled = top_level.iter().map(|id| tree.atomic(id)).max();

        let mut depth = model
            .states
            .keys()
            .map(|id| tree.depth(id))
            .max()
            .unwrap_or(0);
        // A `<history>` sits one level below its parent, and the walk that
        // starts at one must still reach the `<scxml>` element in `depth` steps.
        let mut history_parents: Vec<&str> = Vec::new();
        for history in model.history_states.values() {
            depth = depth.max(tree.depth(&history.parent) + 1);
            if !history_parents.contains(&history.parent.as_str()) {
                history_parents.push(&history.parent);
            }
        }

        let configuration = configuration.unwrap_or(0).max(1);
        let depth = depth.max(1);
        // The resumptions a level of the entry walk holds: a list's and the walk
        // up's, and the region loop's only where a `<parallel>` can be entered.
        let steps_a_level = if model.states.values().any(|s| s.is_parallel) {
            3
        } else {
            2
        };
        Self {
            configuration,
            entry_walk: steps_a_level * depth + model.history_states.len() + 3,
            // A selection that finds nothing still holds an empty buffer, and a
            // buffer of no slots has no use to anything that holds one, so the
            // floor is one.
            enabled: enabled.unwrap_or(0).max(1),
            history_defaults: history_parents.len().min(configuration),
            depth,
        }
    }
}

/// The tree of a document's states, walked down from a state or up from one.
struct Tree<'m> {
    model: &'m SCXMLModel,
    children: &'m BTreeMap<&'m str, Vec<&'m str>>,
}

impl Tree<'_> {
    fn children_of(&self, id: &str) -> &[&str] {
        self.children.get(id).map_or(&[], Vec::as_slice)
    }

    fn is_parallel(&self, id: &str) -> bool {
        self.model.states.get(id).is_some_and(|s| s.is_parallel)
    }

    /// The most states one configuration of the subtree rooted at `id` holds,
    /// `id` included.
    fn configuration(&self, id: &str) -> usize {
        let below = self.children_of(id).iter().map(|c| self.configuration(c));
        1 + if self.is_parallel(id) {
            below.sum()
        } else {
            below.max().unwrap_or(0)
        }
    }

    /// The most atomic states one configuration of the subtree rooted at `id`
    /// holds. A state with no children is atomic unless it is a `<parallel>`,
    /// which the engine never asks for a transition itself.
    fn atomic(&self, id: &str) -> usize {
        let children = self.children_of(id);
        if self.is_parallel(id) {
            children.iter().map(|c| self.atomic(c)).sum()
        } else if children.is_empty() {
            1
        } else {
            children.iter().map(|c| self.atomic(c)).max().unwrap_or(0)
        }
    }

    /// How many levels `id` is at: 1 for a child of the `<scxml>` element. A
    /// parent chain that loops is cut where a state repeats, since the parser
    /// refuses it elsewhere and a bound is not the place to hang on it.
    fn depth(&self, id: &str) -> usize {
        let mut depth = 1;
        let mut current = id;
        while let Some(parent) = self
            .model
            .states
            .get(current)
            .and_then(|s| s.parent.as_deref())
        {
            depth += 1;
            current = parent;
            if depth > self.model.states.len() {
                break;
            }
        }
        depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HistoryInfo, State};

    /// A document of the given states, `(id, parent, is_parallel)` in order.
    fn document(states: &[(&str, Option<&str>, bool)]) -> SCXMLModel {
        let mut model = SCXMLModel::default();
        for (id, parent, is_parallel) in states {
            model.states.insert(
                (*id).to_string(),
                State {
                    id: (*id).to_string(),
                    parent: parent.map(str::to_string),
                    is_parallel: *is_parallel,
                    ..State::default()
                },
            );
        }
        model
    }

    /// Every legal configuration of the document, listed by walking the tree the
    /// way the specification defines one (§scxml-3.4, §scxml-3.11): a compound
    /// state's is itself with one child's, a `<parallel>`'s is itself with one
    /// of each region's, an atomic state's is itself.
    fn legal_configurations(model: &SCXMLModel) -> Vec<Vec<String>> {
        fn below(model: &SCXMLModel, id: &str) -> Vec<Vec<String>> {
            let children: Vec<&String> = model
                .states
                .iter()
                .filter(|(_, s)| s.parent.as_deref() == Some(id))
                .map(|(c, _)| c)
                .collect();
            let own = |rest: Vec<String>| {
                let mut config = vec![id.to_string()];
                config.extend(rest);
                config
            };
            if children.is_empty() {
                return vec![own(vec![])];
            }
            if model.states[id].is_parallel {
                // One choice from every region: the product.
                let mut product: Vec<Vec<String>> = vec![vec![]];
                for child in children {
                    let mut next = Vec::new();
                    for chosen in &product {
                        for option in below(model, child) {
                            let mut joined = chosen.clone();
                            joined.extend(option);
                            next.push(joined);
                        }
                    }
                    product = next;
                }
                product.into_iter().map(own).collect()
            } else {
                children
                    .into_iter()
                    .flat_map(|child| below(model, child))
                    .map(own)
                    .collect()
            }
        }
        let top: Vec<&String> = model
            .states
            .iter()
            .filter(|(_, s)| s.parent.is_none())
            .map(|(id, _)| id)
            .collect();
        top.into_iter().flat_map(|id| below(model, id)).collect()
    }

    fn atomic_count(model: &SCXMLModel, config: &[String]) -> usize {
        config
            .iter()
            .filter(|id| {
                let is_parallel = model.states[*id].is_parallel;
                let has_children = model
                    .states
                    .values()
                    .any(|s| s.parent.as_deref() == Some(id.as_str()));
                !is_parallel && !has_children
            })
            .count()
    }

    /// The formula answers what listing every configuration answers.
    fn holds_to_the_listing(model: &SCXMLModel) -> MicrostepBounds {
        let bounds = MicrostepBounds::of(model);
        let configurations = legal_configurations(model);
        let largest = configurations.iter().map(Vec::len).max().unwrap();
        let most_atomic = configurations
            .iter()
            .map(|c| atomic_count(model, c))
            .max()
            .unwrap();
        assert_eq!(bounds.configuration, largest, "configuration");
        assert_eq!(bounds.enabled, most_atomic.max(1), "enabled");
        bounds
    }

    #[test]
    fn a_flat_machine_holds_one_state_at_a_time() {
        let model = document(&[("a", None, false), ("b", None, false), ("c", None, false)]);
        let bounds = holds_to_the_listing(&model);
        assert_eq!(
            bounds,
            MicrostepBounds {
                configuration: 1,
                enabled: 1,
                history_defaults: 0,
                depth: 1,
                entry_walk: 5
            }
        );
    }

    #[test]
    fn a_nested_machine_holds_the_path_to_its_deepest_leaf() {
        let model = document(&[
            ("a", None, false),
            ("a1", Some("a"), false),
            ("a2", Some("a"), false),
            ("a2x", Some("a2"), false),
            ("b", None, false),
        ]);
        let bounds = holds_to_the_listing(&model);
        assert_eq!(bounds.configuration, 3);
        assert_eq!(bounds.enabled, 1);
        assert_eq!(bounds.depth, 3);
    }

    #[test]
    fn a_parallel_holds_every_region_at_once() {
        // p holds r1 (atomic leaf) and r2 (a compound of two leaves): the
        // largest configuration is p, r1, r2 and one leaf of r2.
        let model = document(&[
            ("p", None, true),
            ("r1", Some("p"), false),
            ("r2", Some("p"), false),
            ("r2a", Some("r2"), false),
            ("r2b", Some("r2"), false),
        ]);
        let bounds = holds_to_the_listing(&model);
        assert_eq!(bounds.configuration, 4);
        assert_eq!(bounds.enabled, 2);
    }

    #[test]
    fn a_wide_parallel_is_not_held_to_the_depth() {
        // Twenty regions: the configuration is twenty-one states in a tree two
        // levels deep, which a bound written for depth alone cannot hold.
        let mut states = vec![("p".to_string(), None::<String>, true)];
        for i in 0..20 {
            states.push((format!("r{i}"), Some("p".to_string()), false));
        }
        let refs: Vec<(&str, Option<&str>, bool)> = states
            .iter()
            .map(|(id, parent, par)| (id.as_str(), parent.as_deref(), *par))
            .collect();
        let model = document(&refs);
        let bounds = holds_to_the_listing(&model);
        assert_eq!(bounds.configuration, 21);
        assert_eq!(bounds.enabled, 20);
        assert_eq!(bounds.depth, 2);
    }

    #[test]
    fn nested_parallels_multiply_through_their_compounds() {
        // A compound that holds a parallel whose regions hold parallels.
        let model = document(&[
            ("top", None, false),
            ("outer", Some("top"), true),
            ("o1", Some("outer"), true),
            ("o1a", Some("o1"), false),
            ("o1b", Some("o1"), false),
            ("o2", Some("outer"), false),
            ("o2a", Some("o2"), false),
            ("alt", Some("top"), false),
        ]);
        holds_to_the_listing(&model);
    }

    #[test]
    fn a_history_sits_one_level_below_its_parent_and_is_owed_once_per_parent() {
        let mut model = document(&[
            ("a", None, false),
            ("a1", Some("a"), false),
            ("a2", Some("a"), false),
        ]);
        for (id, parent) in [("h1", "a"), ("h2", "a")] {
            model.history_states.insert(
                id.to_string(),
                HistoryInfo {
                    parent: parent.to_string(),
                    presence: None,
                    history_type: "shallow".to_string(),
                    default_target: "a1".to_string(),
                    default_targets: vec!["a1".to_string()],
                    leaf_target: "a1".to_string(),
                    default_actions: vec![],
                    source_location: None,
                },
            );
        }
        let bounds = MicrostepBounds::of(&model);
        // a1 is at depth 2, so a history of a, which sits below a, is at 2 too.
        assert_eq!(bounds.depth, 2);
        // Two histories of the one parent are owed one default at a time.
        assert_eq!(bounds.history_defaults, 1);
    }

    #[test]
    fn a_history_of_the_deepest_state_is_a_level_below_it() {
        let mut model = document(&[("a", None, false), ("a1", Some("a"), false)]);
        model.history_states.insert(
            "h".to_string(),
            HistoryInfo {
                parent: "a1".to_string(),
                presence: None,
                history_type: "shallow".to_string(),
                default_target: String::new(),
                default_targets: vec![],
                leaf_target: String::new(),
                default_actions: vec![],
                source_location: None,
            },
        );
        assert_eq!(MicrostepBounds::of(&model).depth, 3);
    }

    #[test]
    fn a_document_with_no_states_still_has_buffers_of_one_slot() {
        let bounds = MicrostepBounds::of(&SCXMLModel::default());
        assert_eq!(
            bounds,
            MicrostepBounds {
                configuration: 1,
                enabled: 1,
                history_defaults: 0,
                depth: 1,
                entry_walk: 5
            }
        );
    }

    #[test]
    fn the_entry_walk_is_three_steps_a_level_with_a_parallel_and_two_without() {
        // The nested document of the depth test: three levels, no history and
        // no `<parallel>`, so no level owes the region loop's resumption.
        let model = document(&[
            ("a", None, false),
            ("a1", Some("a"), false),
            ("a2", Some("a"), false),
            ("a2x", Some("a2"), false),
            ("b", None, false),
        ]);
        assert_eq!(MicrostepBounds::of(&model).entry_walk, 2 * 3 + 3);

        // The same depth with a `<parallel>` on it owes three a level.
        let model = document(&[
            ("a", None, true),
            ("a1", Some("a"), false),
            ("a2", Some("a"), false),
            ("a2x", Some("a2"), false),
            ("b", None, false),
        ]);
        assert_eq!(MicrostepBounds::of(&model).entry_walk, 3 * 3 + 3);

        // Two histories add one step each, and the deeper level one history of
        // the deepest state opens adds two in a document with no `<parallel>`.
        let mut model = document(&[("a", None, false), ("a1", Some("a"), false)]);
        for (id, parent) in [("h1", "a1"), ("h2", "a")] {
            model.history_states.insert(
                id.to_string(),
                HistoryInfo {
                    parent: parent.to_string(),
                    presence: None,
                    history_type: "shallow".to_string(),
                    default_target: String::new(),
                    default_targets: vec![],
                    leaf_target: String::new(),
                    default_actions: vec![],
                    source_location: None,
                },
            );
        }
        let bounds = MicrostepBounds::of(&model);
        assert_eq!(bounds.depth, 3);
        assert_eq!(bounds.entry_walk, 2 * 3 + 2 + 3);
    }
}
