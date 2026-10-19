// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A generated machine held to the transition function generated beside it.
//!
//! A document that declares `sce:transition-function="emit"` is generated with
//! a pure function of (state, event) beside its machine
//! (`docs/SCE_TRANSITION_FUNCTION.md`). The function is computed at build time
//! by the selection rule the engine follows at run time, so the two are one fact
//! only if every pair agrees, and this is what asks: a fresh machine per pair,
//! driven to the state by the shortest event path the function itself gives,
//! then sent the event. What the engine reaches, and whether it took the event
//! or discarded it, must be what the function says.
//!
//! The drivers of the integration stems that declare a function share it, so
//! the two stems ask the same question the same way.

use std::collections::BTreeSet;

use sce_rust_runtime::{Engine, StatePolicy};

/// A generated transition function, read through its accessors: the module
/// is the generator's, so the drive is handed what it needs of it by name.
pub struct TransitionFunction<S: 'static, E: 'static> {
    pub states: &'static [S],
    pub events: &'static [E],
    pub initial: S,
    pub transition: fn(S, E) -> Option<S>,
    pub is_final: fn(S) -> bool,
    pub configuration: fn(S) -> &'static [&'static str],
    pub state_name: fn(S) -> &'static str,
    pub event_name: fn(E) -> &'static str,
}

/// The shortest sequence of events that takes `function` from its initial
/// state to `target`, read from the function alone; `None` when no sequence
/// does.
pub fn shortest_path<S, E>(function: &TransitionFunction<S, E>, target: S) -> Option<Vec<E>>
where
    S: Copy + PartialEq,
    E: Copy,
{
    let mut frontier: Vec<(S, Vec<E>)> = vec![(function.initial, Vec::new())];
    let mut seen = vec![function.initial];
    let mut next = 0;
    while next < frontier.len() {
        let (state, path) = frontier[next].clone();
        next += 1;
        if state == target {
            return Some(path);
        }
        for &event in function.events {
            if let Some(to) = (function.transition)(state, event) {
                if !seen.contains(&to) {
                    seen.push(to);
                    let mut longer = path.clone();
                    longer.push(event);
                    frontier.push((to, longer));
                }
            }
        }
    }
    None
}

/// Drive a machine `fresh` builds, already initialized, to every state of
/// `function`, send it every event there, and assert that the configuration
/// it reaches, and whether it took the event or discarded it, is what the
/// function says. Returns how many pairs it checked, which a caller holds to
/// the number of states times the number of events.
pub fn expect_every_pair_agrees<S, E, P>(
    function: &TransitionFunction<S, E>,
    fresh: impl Fn() -> Engine<P>,
) -> usize
where
    S: Copy + PartialEq,
    E: Copy,
    P: StatePolicy,
{
    let active = |engine: &Engine<P>| -> BTreeSet<&'static str> {
        engine
            .get_active_states()
            .into_iter()
            .map(P::get_state_name)
            .collect()
    };
    let configuration = |state: S| -> BTreeSet<&'static str> {
        (function.configuration)(state).iter().copied().collect()
    };
    let send = |engine: &mut Engine<P>, event: E| {
        engine.raise_external_by_name((function.event_name)(event), "");
        engine.step();
    };
    let terminal = |engine: &Engine<P>| engine.terminal_state().map(P::get_state_name);

    assert_eq!(
        active(&fresh()),
        configuration(function.initial),
        "the machine does not start where the function does"
    );
    let mut pairs = 0;
    for &state in function.states {
        let name = (function.state_name)(state);
        let path = shortest_path(function, state)
            .unwrap_or_else(|| panic!("{name} is not reached from the initial state"));
        for &event in function.events {
            let pair = format!("({name}, {})", (function.event_name)(event));
            let mut engine = fresh();
            for &step in &path {
                send(&mut engine, step);
            }
            pairs += 1;
            if (function.is_final)(state) {
                assert_eq!(
                    terminal(&engine),
                    Some(name),
                    "{pair}: the path does not end the machine there"
                );
                continue;
            }
            assert_eq!(
                active(&engine),
                configuration(state),
                "{pair}: the path leaves the machine elsewhere"
            );
            let discarded = engine.discarded_external_events();
            send(&mut engine, event);
            match (function.transition)(state, event) {
                None => {
                    assert_eq!(
                        engine.discarded_external_events(),
                        discarded + 1,
                        "{pair}: the function refuses it and the machine took it"
                    );
                    assert_eq!(
                        active(&engine),
                        configuration(state),
                        "{pair}: the machine moved on an event it discarded"
                    );
                }
                Some(to) => {
                    assert_eq!(
                        engine.discarded_external_events(),
                        discarded,
                        "{pair}: the function takes it and the machine discarded it"
                    );
                    if (function.is_final)(to) {
                        assert_eq!(
                            terminal(&engine),
                            Some((function.state_name)(to)),
                            "{pair}: the machine did not end where the function says"
                        );
                    } else {
                        assert_eq!(
                            active(&engine),
                            configuration(to),
                            "{pair}: the machine is not where the function says"
                        );
                    }
                }
            }
        }
    }
    pairs
}
