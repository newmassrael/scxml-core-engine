// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The machine altered in each way of a closed list (docs/adr/0115, D15), so
//! the judge can ask which guarantees notice each alteration: a guarantee no
//! alteration breaks says nothing of what the machine does (Chockler,
//! Kupferman and Vardi 2001), and an alteration no guarantee notices is
//! something the properties do not hold the machine to.
//!
//! The list, over every transition and every block of executable content the
//! machine writes, in document order:
//!
//! - a transition's condition made false, which takes the transition out: a
//!   transition never enabled is one the selection passes over, as it passes
//!   over one that is not written;
//! - its condition made true, for a transition that has one;
//! - an action taken out;
//! - the event an action sends or raises changed to one no transition names;
//! - the value an `<assign>` writes changed: an integer by one more, a
//!   boolean to its negation.
//!
//! Only the machine is altered, never its environment, which it does not
//! control: the exploration and the assumptions stay as they are.

use std::collections::BTreeMap;

use crate::model::{Action, SCXMLModel};

use super::document_model::{self, Ty};

/// Where an action is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// The executable content of transition `index` of `state`.
    Transition { state: String, index: usize },
    /// The `<onentry>` numbered `block` of `state`, from 0.
    OnEntry { state: String, block: usize },
    /// The `<onexit>` numbered `block` of `state`, from 0.
    OnExit { state: String, block: usize },
    /// The content of the initial transition of `state`.
    Initial { state: String },
}

impl Block {
    fn describe(&self) -> String {
        match self {
            Block::Transition { state, index } => {
                format!("transition {index} of state `{state}`")
            }
            Block::OnEntry { state, block } => {
                format!("the <onentry> {block} of state `{state}`")
            }
            Block::OnExit { state, block } => format!("the <onexit> {block} of state `{state}`"),
            Block::Initial { state } => format!("the <initial> of state `{state}`"),
        }
    }

    /// The actions of the block in `model`, to alter.
    fn actions_mut<'m>(&self, model: &'m mut SCXMLModel) -> Option<&'m mut Vec<Action>> {
        match self {
            Block::Transition { state, index } => model
                .states
                .get_mut(state)?
                .transitions
                .get_mut(*index)
                .map(|transition| &mut transition.actions),
            Block::OnEntry { state, block } => {
                model.states.get_mut(state)?.on_entry_blocks.get_mut(*block)
            }
            Block::OnExit { state, block } => {
                model.states.get_mut(state)?.on_exit_blocks.get_mut(*block)
            }
            Block::Initial { state } => model
                .states
                .get_mut(state)
                .map(|state| &mut state.initial_transition_actions),
        }
    }
}

/// One alteration of the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alteration {
    /// The condition of transition `index` of `state` made false, which
    /// takes the transition out.
    TakenOut { state: String, index: usize },
    /// The condition of transition `index` of `state` made true.
    AlwaysEnabled { state: String, index: usize },
    /// Action `at` of `block` taken out.
    ActionOut { block: Block, at: usize },
    /// The event action `at` of `block` sends or raises, `from`, changed to
    /// `to`.
    EventChanged {
        block: Block,
        at: usize,
        from: String,
        to: String,
    },
    /// The value action `at` of `block` writes to `location`, `from`,
    /// changed to `to`.
    ValueChanged {
        block: Block,
        at: usize,
        location: String,
        from: String,
        to: String,
    },
}

impl Alteration {
    /// The alteration, as a report and a record name it.
    pub fn describe(&self) -> String {
        match self {
            Alteration::TakenOut { state, index } => {
                format!("transition {index} of state `{state}` taken out")
            }
            Alteration::AlwaysEnabled { state, index } => {
                format!("the condition of transition {index} of state `{state}` made true")
            }
            Alteration::ActionOut { block, at } => {
                format!("action {at} of {} taken out", block.describe())
            }
            Alteration::EventChanged {
                block,
                at,
                from,
                to,
            } => format!(
                "the event `{from}` made by action {at} of {} changed to `{to}`",
                block.describe()
            ),
            Alteration::ValueChanged {
                block,
                at,
                location,
                from,
                to,
            } => format!(
                "the value `{from}` written to `{location}` by action {at} of {} changed to `{to}`",
                block.describe()
            ),
        }
    }

    /// `model` with this alteration made.
    pub fn apply(&self, model: &SCXMLModel) -> SCXMLModel {
        let mut altered = model.clone();
        let condition = |altered: &mut SCXMLModel, state: &str, index: usize, cond: &str| {
            if let Some(transition) = altered
                .states
                .get_mut(state)
                .and_then(|state| state.transitions.get_mut(index))
            {
                transition.cond = cond.to_string();
            }
        };
        match self {
            Alteration::TakenOut { state, index } => {
                condition(&mut altered, state.as_str(), *index, "false");
            }
            Alteration::AlwaysEnabled { state, index } => {
                condition(&mut altered, state.as_str(), *index, "true");
            }
            Alteration::ActionOut { block, at } => {
                if let Some(actions) = block.actions_mut(&mut altered) {
                    if *at < actions.len() {
                        actions.remove(*at);
                    }
                }
            }
            Alteration::EventChanged { block, at, to, .. } => {
                if let Some(action) = block
                    .actions_mut(&mut altered)
                    .and_then(|actions| actions.get_mut(*at))
                {
                    action.event = to.clone();
                }
            }
            Alteration::ValueChanged { block, at, to, .. } => {
                if let Some(action) = block
                    .actions_mut(&mut altered)
                    .and_then(|actions| actions.get_mut(*at))
                {
                    action.expr = to.clone();
                }
            }
        }
        altered
    }
}

/// Every alteration of `model` the closed list names, in document order.
pub fn alterations(model: &SCXMLModel) -> Vec<Alteration> {
    let types: BTreeMap<String, Ty> =
        document_model::write_with_facts(model, "", document_model::Options::default())
            .map(|(_, facts)| facts.data.into_iter().collect())
            .unwrap_or_default();
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    let mut found = Vec::new();
    for state in states {
        let id = &state.id;
        for (index, transition) in state.transitions.iter().enumerate() {
            found.push(Alteration::TakenOut {
                state: id.clone(),
                index,
            });
            if !transition.cond.trim().is_empty() {
                found.push(Alteration::AlwaysEnabled {
                    state: id.clone(),
                    index,
                });
            }
        }
        let mut blocks: Vec<(Block, &Vec<Action>)> = Vec::new();
        for (block, actions) in state.on_entry_blocks.iter().enumerate() {
            blocks.push((
                Block::OnEntry {
                    state: id.clone(),
                    block,
                },
                actions,
            ));
        }
        for (block, actions) in state.on_exit_blocks.iter().enumerate() {
            blocks.push((
                Block::OnExit {
                    state: id.clone(),
                    block,
                },
                actions,
            ));
        }
        blocks.push((
            Block::Initial { state: id.clone() },
            &state.initial_transition_actions,
        ));
        for (index, transition) in state.transitions.iter().enumerate() {
            blocks.push((
                Block::Transition {
                    state: id.clone(),
                    index,
                },
                &transition.actions,
            ));
        }
        for (block, actions) in blocks {
            for (at, action) in actions.iter().enumerate() {
                found.extend(of_action(&block, at, action, &types));
            }
        }
    }
    found
}

/// The alterations of action `at` of `block`: taken out, and its event or
/// its value changed where it has one. `types` is each `<data>`'s type in
/// the machine's model.
fn of_action(
    block: &Block,
    at: usize,
    action: &Action,
    types: &BTreeMap<String, Ty>,
) -> Vec<Alteration> {
    let mut found = vec![Alteration::ActionOut {
        block: block.clone(),
        at,
    }];
    match action.action_type.as_str() {
        "send" | "raise" if !action.event.is_empty() => found.push(Alteration::EventChanged {
            block: block.clone(),
            at,
            from: action.event.clone(),
            to: format!("altered_{}", action.event.replace('.', "_")),
        }),
        "assign" if !action.expr.trim().is_empty() => {
            let location = action.location.trim();
            let changed = match types.get(location) {
                Some(Ty::Int) => Some(format!("({}) + 1", action.expr.trim())),
                Some(Ty::Bool) => Some(format!("!({})", action.expr.trim())),
                Some(Ty::Name) | None => None,
            };
            if let Some(to) = changed {
                found.push(Alteration::ValueChanged {
                    block: block.clone(),
                    at,
                    location: location.to_string(),
                    from: action.expr.trim().to_string(),
                    to,
                });
            }
        }
        _ => {}
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const TURNSTILE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="ecmascript" name="turnstile" initial="locked">
        <datamodel><data id="coins" expr="0"/></datamodel>
        <state id="locked">
            <transition event="coin" target="unlocked">
                <assign location="coins" expr="coins + 1"/>
            </transition>
        </state>
        <state id="unlocked">
            <onentry><send event="relock" delay="5s"/></onentry>
            <transition event="push" cond="coins &gt; 0" target="locked"/>
            <transition event="relock" target="locked"/>
        </state>
    </scxml>"#;

    fn turnstile() -> SCXMLModel {
        let mut model = crate::parser::SCXMLParser::new()
            .parse_string(TURNSTILE, "turnstile")
            .expect("the statechart parses");
        crate::analyzer::analyze(&mut model, "");
        model
    }

    /// Turns red if a way of the closed list is dropped, one is added that
    /// the list does not name, or the alterations leave document order.
    #[test]
    fn every_way_of_the_list_is_made_in_document_order() {
        let described: Vec<String> = alterations(&turnstile())
            .iter()
            .map(Alteration::describe)
            .collect();
        assert_eq!(
            described,
            [
                "transition 0 of state `locked` taken out",
                "action 0 of transition 0 of state `locked` taken out",
                "the value `coins + 1` written to `coins` by action 0 of transition 0 of state \
                 `locked` changed to `(coins + 1) + 1`",
                "transition 0 of state `unlocked` taken out",
                "the condition of transition 0 of state `unlocked` made true",
                "transition 1 of state `unlocked` taken out",
                "action 0 of the <onentry> 0 of state `unlocked` taken out",
                "the event `relock` made by action 0 of the <onentry> 0 of state `unlocked` \
                 changed to `altered_relock`",
            ]
        );
    }

    /// Turns red if an alteration changes another part of the machine than
    /// it names, or leaves the machine it was made of altered.
    #[test]
    fn an_alteration_changes_what_it_names_and_nothing_else() {
        let model = turnstile();
        let found = alterations(&model);
        let taken_out = found[0].apply(&model);
        assert_eq!(taken_out.states["locked"].transitions[0].cond, "false");
        assert_eq!(model.states["locked"].transitions[0].cond, "");
        let enabled = found[4].apply(&model);
        assert_eq!(enabled.states["unlocked"].transitions[0].cond, "true");
        let out = found[1].apply(&model);
        assert!(out.states["locked"].transitions[0].actions.is_empty());
        let valued = found[2].apply(&model);
        assert_eq!(
            valued.states["locked"].transitions[0].actions[0].expr,
            "(coins + 1) + 1"
        );
        let renamed = found[7].apply(&model);
        assert_eq!(
            renamed.states["unlocked"].on_entry_blocks[0][0].event,
            "altered_relock"
        );
        assert_eq!(
            model.states["unlocked"].on_entry_blocks[0][0].event,
            "relock"
        );
    }

    /// Turns red if an altered machine has no model where the machine has
    /// one: an alteration that cannot be judged shows nothing.
    #[test]
    fn every_altered_turnstile_has_a_model() {
        let model = turnstile();
        for alteration in alterations(&model) {
            let altered = alteration.apply(&model);
            assert!(
                document_model::write(
                    &altered,
                    "turnstile.scxml",
                    document_model::Options::default()
                )
                .is_ok(),
                "{} has no model",
                alteration.describe()
            );
        }
    }
}
