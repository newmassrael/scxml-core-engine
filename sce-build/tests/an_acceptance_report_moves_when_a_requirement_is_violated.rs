// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The completion test for the acceptance report (RFC §7a.1).
//!
//! "It emits three blocks" is not a completion test: a page can emit
//! three blocks and say nothing. The report is a DETECTOR, and this
//! repository already owns the way detectors are tested —
//! `scripts/mutate`'s own header: *"A test that passes proves nothing
//! about whether it could fail. The way to find out is to break the
//! code it guards and watch it turn red."*
//!
//! ```text
//!   take the accepted (spec, SCXML) pair
//!   break one element a requirement's evidence depends on
//!   regenerate the report
//!     byte-identical -> the differing value is NOT ON THE PAGE. No
//!                       reviewer, however careful, could catch it
//!     changed        -> at least it is visible
//! ```
//!
//! ⚠ What this cannot judge, stated rather than implied: *the page
//! changed* is not *a person noticed*. That half is irreducible and
//! stays a human judgement (RFC §15). This mechanises the half that can
//! be, and stops counting the other half as covered.

use std::path::{Path, PathBuf};

use sce_build::acceptance_report::{fragment, render};
use sce_build::model::{Action, SCXMLModel, Transition};
use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::RequirementManifest;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("requirement_closure")
}

fn load_manifest() -> RequirementManifest {
    RequirementManifest::load(&fixture_dir().join("iso13400_2_nl_socket_handling.manifest.json"))
        .expect("the committed manifest loads")
}

fn base_model() -> SCXMLModel {
    let path = fixture_dir().join("doip_nl_connection_states.scxml");
    let raw = std::fs::read_to_string(&path).expect("the committed document is readable");
    SCXMLParser::new()
        .parse_string(&raw, &path.display().to_string())
        .unwrap_or_else(|e| panic!("the committed document must parse: {:?}", e.error))
}

/// One requirement's block A, sliced out of the rendered page.
///
/// Sliced rather than rendered separately: the thing under test is the
/// page a person is handed, so the comparison has to be made on that
/// page and not on a convenience rendering built beside it.
fn block_of(report: &str, id: &str) -> String {
    let header = format!("\n  {id}");
    let start = report
        .find(&header)
        .unwrap_or_else(|| panic!("the report has no block for {id}"));
    let rest = &report[start + 1..];
    // The next block starts at a line indented by exactly two spaces;
    // evidence lines are indented deeper.
    let end = rest
        .match_indices("\n  ")
        .find(|(at, _)| {
            rest[at + 3..]
                .chars()
                .next()
                .is_some_and(|c| !c.is_whitespace())
                && *at > 0
        })
        .map(|(at, _)| at)
        .unwrap_or(rest.len());
    rest[..end].to_string()
}

/// What was changed, for the failure message. A reader meeting a red
/// here needs to know which relation went unseen, not a node path.
#[derive(Debug, Clone, Copy)]
enum Break {
    TransitionEvent,
    TransitionGuard,
    TransitionTarget,
    ActionDelay,
    ActionEvent,
    ActionSendId,
    /// A compound state entering a different child, which is a
    /// violation of anything requiring it to start where it says.
    StateInitial,
    /// A state becoming final, which ends the session where the
    /// requirement expects the machine to carry on.
    StateFinal,
}

/// Break a transition so a requirement citing it is violated. Returns
/// false when the mutation would not change the model at all — an
/// unobservable mutation must never be counted as an examined case.
fn break_transition(transition: &mut Transition, how: Break) -> bool {
    match how {
        Break::TransitionEvent if !transition.event.is_empty() => {
            transition.event.push_str(".moved");
            true
        }
        Break::TransitionGuard => {
            // Both directions are a violation: a requirement met only
            // under a guard is broken by removing it, and one met
            // unconditionally is broken by adding one.
            if transition.cond.is_empty() {
                transition.cond = "false".to_string();
            } else {
                transition.cond.clear();
            }
            true
        }
        Break::TransitionTarget if !transition.target.is_empty() => {
            transition.target.push_str("_elsewhere");
            true
        }
        _ => false,
    }
}

fn break_action(action: &mut Action, how: Break) -> bool {
    match how {
        Break::ActionDelay if !action.delay.is_empty() => {
            action.delay = format!("{}0", action.delay);
            action.delay_ms = action.delay_ms.saturating_mul(10).max(1);
            true
        }
        Break::ActionEvent if !action.event.is_empty() => {
            action.event.push_str(".moved");
            true
        }
        Break::ActionSendId if !action.sendid.is_empty() => {
            action.sendid.push_str("_other");
            true
        }
        _ => false,
    }
}

/// Apply `how` to the node at `path`, returning whether anything moved.
///
/// The path is the model's own field path, which is what
/// `requirements_report` assigns and what the table prints, so walking
/// it here is reading the same convention rather than inventing one.
fn break_node_at(model: &mut SCXMLModel, path: &str, how: Break) -> bool {
    let Some(rest) = path.strip_prefix("states.") else {
        return false;
    };
    // `states.<id>` with no tail is the state node itself.
    let (state_id, tail) = match rest.split_once('.') {
        Some(split) => split,
        None => (rest, ""),
    };
    let Some(state) = model.states.get_mut(state_id) else {
        return false;
    };

    if tail.is_empty() {
        return match how {
            Break::StateInitial => {
                // Both directions violate: a state told to enter a
                // particular child is broken by entering another, and
                // one with no `initial` is broken by gaining one.
                if state.initial.is_empty() {
                    state.initial = "elsewhere".to_string();
                    state.initial_children = vec!["elsewhere".to_string()];
                } else {
                    state.initial.push_str("_elsewhere");
                    state.initial_children = vec![state.initial.clone()];
                }
                true
            }
            Break::StateFinal => {
                state.is_final = !state.is_final;
                true
            }
            _ => false,
        };
    }

    if let Some(index) = tail
        .strip_prefix("transitions[")
        .and_then(|t| t.strip_suffix(']'))
        .and_then(|t| t.parse::<usize>().ok())
    {
        return state
            .transitions
            .get_mut(index)
            .is_some_and(|t| break_transition(t, how));
    }

    for (field, blocks) in [
        ("on_entry_blocks", &mut state.on_entry_blocks),
        ("on_exit_blocks", &mut state.on_exit_blocks),
    ] {
        let prefix = format!("{field}[");
        if let Some(inner) = tail.strip_prefix(&prefix) {
            if let Some((b, j)) = parse_two_indices(inner) {
                return blocks
                    .get_mut(b)
                    .and_then(|block| block.get_mut(j))
                    .is_some_and(|a| break_action(a, how));
            }
        }
    }

    // `transitions[i].actions[j]`
    if let Some(inner) = tail.strip_prefix("transitions[") {
        if let Some((i, rest)) = inner.split_once("].actions[") {
            if let (Ok(i), Some(j)) = (
                i.parse::<usize>(),
                rest.strip_suffix(']').and_then(|j| j.parse::<usize>().ok()),
            ) {
                return state
                    .transitions
                    .get_mut(i)
                    .and_then(|t| t.actions.get_mut(j))
                    .is_some_and(|a| break_action(a, how));
            }
        }
    }

    false
}

/// `0][3]` -> (0, 3)
fn parse_two_indices(raw: &str) -> Option<(usize, usize)> {
    let (first, rest) = raw.split_once("][")?;
    Some((first.parse().ok()?, rest.strip_suffix(']')?.parse().ok()?))
}

#[test]
fn breaking_any_element_a_requirement_depends_on_moves_its_block() {
    const EXAMINED_FLOOR: usize = 40;

    let manifest = load_manifest();
    let base = base_model();
    let report = render(&base, &manifest, None);

    let breaks = [
        Break::TransitionEvent,
        Break::TransitionGuard,
        Break::TransitionTarget,
        Break::ActionDelay,
        Break::ActionEvent,
        Break::ActionSendId,
        Break::StateInitial,
        Break::StateFinal,
    ];

    let mut examined = 0usize;
    let mut unresolved_paths: Vec<String> = Vec::new();
    let mut unmoved: Vec<String> = Vec::new();
    let mut requirements_with_evidence = 0usize;

    for entry in &manifest.requirements {
        let deps = fragment(&base, &entry.id);
        if deps.is_empty() {
            continue;
        }
        requirements_with_evidence += 1;
        let before = block_of(&report, &entry.id);

        for dep in &deps {
            let mut resolved_any = false;
            for how in breaks {
                let mut mutant = base.clone();
                if !break_node_at(&mut mutant, &dep.node_path, how) {
                    continue;
                }
                resolved_any = true;
                examined += 1;

                let after_report = render(&mutant, &manifest, None);
                let after = block_of(&after_report, &entry.id);
                if after == before {
                    unmoved.push(format!(
                        "{}: {:?} on {} ({:?}) left block A byte-identical",
                        entry.id, how, dep.node_path, dep.relation
                    ));
                }
            }
            if !resolved_any {
                unresolved_paths.push(format!("{} ({:?})", dep.node_path, dep.relation));
            }
        }
    }

    println!(
        "broke {examined} element(s) across {requirements_with_evidence} requirement(s) \
         with evidence; {} left the block unchanged",
        unmoved.len()
    );

    assert!(
        unresolved_paths.is_empty(),
        "a dependency's node could not be reached by its own field path, so those \
         elements were never broken and this sweep silently measured less than it \
         reports:\n  {}",
        unresolved_paths.join("\n  ")
    );
    assert!(
        unmoved.is_empty(),
        "a violating change left the page byte-identical, so no reviewer could \
         catch it however carefully they read:\n  {}",
        unmoved.join("\n  ")
    );
    assert!(
        examined >= EXAMINED_FLOOR,
        "only {examined} element(s) broken; floor {EXAMINED_FLOOR}. A sweep that \
         examines nothing passes for the same reason a correct one does"
    );
}
