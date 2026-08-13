// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The AI supervision loop, driven through the Rust AOT engine.
//
// `examples/ai_loop/ai_loop.scxml` is a worked example: a statechart that
// supervises a long-running session, with `<parallel>` splitting the turn
// cycle from the liveness watch and the turn budget. `examples/ai_loop/
// ai_loop_example.cpp` drives it through the C++ AOT engine; this file drives
// the same document through the Rust one. Two engines asserting one document
// is what makes a topology change fail loudly instead of half-landing — and
// the parallel defect that shipped in `1419a050ed` (a self-transition whose
// exit set swallowed the parallel root) was invisible to every W3C fixture
// because they are all one region deep. This document is three.
//
// No sprag, no session, no pane: every effect the host would perform is
// replaced by the event that effect would have produced, so what is under
// test is the machine's topology rather than any driver's plumbing.
//
// Because the regions are orthogonal, a scenario asserts on the ACTIVE SET
// rather than on one state — "the cycle is working AND the budget is within"
// is the kind of claim a parallel machine makes, and asserting a single
// current state cannot express it.
//
// Fixture: examples/ai_loop/ai_loop.scxml
//
// Regeneration (after example or template edit):
//   scripts/regen_ai_loop.sh

use sce_rust_runtime::Engine;
use sce_rust_tests::integration::ai_loop::{AiLoopEvent, AiLoopPolicy, AiLoopState};

/// Engine DI Parity RFC (Path B+): the document's prompts and standing rules
/// are datamodel values, so the policy takes a script engine rather than
/// reaching for a process-global one.
fn engine() -> Engine<AiLoopPolicy> {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = Engine::new(AiLoopPolicy::new(script_engine));
    e.initialize();
    e
}

/// Every state currently active, across all three regions.
fn active(e: &Engine<AiLoopPolicy>) -> Vec<AiLoopState> {
    e.get_active_states().to_vec()
}

fn holds(e: &Engine<AiLoopPolicy>, s: AiLoopState) -> bool {
    active(e).contains(&s)
}

fn step(e: &mut Engine<AiLoopPolicy>, ev: AiLoopEvent) {
    e.process_event(ev);
    e.step();
}

/// One completed turn: the work finished, and the loop decides what next.
fn turn(e: &mut Engine<AiLoopPolicy>) {
    step(e, AiLoopEvent::TurnDone);
    step(e, AiLoopEvent::Judge);
}

/// A run whose first prompt has been sent — the state every scenario below
/// starts from.
fn started() -> Engine<AiLoopPolicy> {
    let mut e = engine();
    step(&mut e, AiLoopEvent::PromptSent);
    e
}

#[test]
fn all_three_regions_are_live_at_once() {
    let e = started();
    let a = active(&e);
    assert!(
        a.contains(&AiLoopState::Working)
            && a.contains(&AiLoopState::Alive)
            && a.contains(&AiLoopState::Within),
        "the cycle, the liveness watch and the budget are orthogonal regions and \
         must all be active at once; got {a:?}"
    );
}

#[test]
fn reflection_fires_on_schedule() {
    let mut e = started();
    let mut at = None;
    for n in 1..=10 {
        turn(&mut e);
        if holds(&e, AiLoopState::Reflecting) {
            at = Some(n);
            break;
        }
    }
    assert_eq!(
        at,
        Some(8),
        "the document sets `reflect_every` to 8, so the eighth completed turn is \
         the one that reflects; reflection fired at {at:?}"
    );
}

#[test]
fn reflection_goes_through_a_restart_and_the_loop_re_primes() {
    let mut e = started();
    for _ in 1..=8 {
        turn(&mut e);
    }

    step(&mut e, AiLoopEvent::ReflectApplied);
    assert!(
        holds(&e, AiLoopState::Restarting),
        "a session reads its context, MCP config and memory once, when it starts, \
         so applying a reflection has to REPLACE the session rather than \
         reconfigure it; active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::SessionReady);
    assert!(
        holds(&e, AiLoopState::Priming),
        "a replaced session starts empty and must be primed with the current \
         prompts before it can take a turn; active: {:?}",
        active(&e)
    );
}

#[test]
fn the_budget_ends_the_run_from_wherever_the_cycle_is() {
    let mut e = started();
    for _ in 1..=60 {
        if holds(&e, AiLoopState::Reflecting) {
            step(&mut e, AiLoopEvent::ReflectNone);
        }
        if holds(&e, AiLoopState::Exhausted) {
            break;
        }
        turn(&mut e);
    }
    assert!(
        holds(&e, AiLoopState::Exhausted),
        "the budget is its own region precisely so the turn count is not something \
         `judging` has to remember to check; active: {:?}",
        active(&e)
    );
}

#[test]
fn a_standing_instruction_answers_without_waking_anybody() {
    let mut e = started();

    step(&mut e, AiLoopEvent::TurnBlocked);
    assert!(
        holds(&e, AiLoopState::Screening),
        "a dialog is screened against the rules the person wrote in advance \
         before anyone is woken; active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::ScreenMatched);
    assert!(
        holds(&e, AiLoopState::Working) && !holds(&e, AiLoopState::Paused),
        "a matched rule is a decision the person already made, so the run carries \
         on and nobody is woken; active: {:?}",
        active(&e)
    );
}

#[test]
fn an_unmatched_dialog_wakes_the_person_who_answers() {
    let mut e = started();

    step(&mut e, AiLoopEvent::TurnBlocked);
    step(&mut e, AiLoopEvent::ScreenNone);
    assert!(
        holds(&e, AiLoopState::Paused),
        "the loop answers only what the person decided in advance; anything else \
         stops it and waits; active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::TurnDone);
    assert!(
        holds(&e, AiLoopState::Judging),
        "once the person has answered, the turn completes where it left off; \
         active: {:?}",
        active(&e)
    );
}

#[test]
fn hold_and_resume_return_to_exactly_where_the_cycle_was() {
    let mut e = started();
    turn(&mut e);

    step(&mut e, AiLoopEvent::Hold);
    assert!(
        holds(&e, AiLoopState::Paused),
        "a person looking at the work holds the cycle; active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::Resume);
    assert!(
        holds(&e, AiLoopState::Working),
        "resuming puts the cycle back to work rather than ending the run; \
         active: {:?}",
        active(&e)
    );
}

#[test]
fn resume_returns_somewhere_the_history_default_does_not() {
    // `<history id="where">` declares `<transition target="working"/>` as its
    // default, so a hold taken while the cycle is in `working` resumes there
    // whether history recorded anything or not — the test above cannot tell a
    // working history from one that records nothing. Measured: deleting the
    // recording filter left it green.
    //
    // `priming` is the one place the two answers differ. The machine comes up
    // there, `hold` is declared above the cycle so it reaches, and the history
    // default names `working` — so resuming into `priming` is only possible if
    // the configuration was really recorded.
    let mut e = engine();
    assert!(
        holds(&e, AiLoopState::Priming),
        "the run starts with a session that exists and has not been prompted; \
         active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::Hold);
    assert!(
        holds(&e, AiLoopState::Paused),
        "a person can take over before the first prompt as readily as after one; \
         active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::Resume);
    assert!(
        holds(&e, AiLoopState::Priming) && !holds(&e, AiLoopState::Working),
        "`<history>` must restore the state the cycle was actually in; landing in \
         `working` here is the history default answering instead, which is what a \
         history that records nothing looks like; active: {:?}",
        active(&e)
    );
}

#[test]
fn the_person_interrupts_the_inner_session_by_hand() {
    let mut e = started();

    step(&mut e, AiLoopEvent::TurnInterrupted);
    assert!(
        holds(&e, AiLoopState::Paused) && !holds(&e, AiLoopState::Screening),
        "a person typing into the session directly is not a dialog to screen — \
         the loop stops driving and stays out of the way; active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::TurnInterrupted);
    assert!(
        holds(&e, AiLoopState::Paused),
        "further interruptions keep it paused rather than fighting the person for \
         the session; active: {:?}",
        active(&e)
    );
}

#[test]
fn nobody_comes() {
    let mut e = started();

    step(&mut e, AiLoopEvent::TurnBlocked);
    step(&mut e, AiLoopEvent::ScreenNone);
    step(&mut e, AiLoopEvent::Unattended);
    assert!(
        holds(&e, AiLoopState::Blocked),
        "a question nobody answers ends the run in an outcome the document names, \
         rather than leaving it prompting into the dark; active: {:?}",
        active(&e)
    );
}

#[test]
fn a_pane_that_dies_mid_turn_is_noticed_and_rebuilt() {
    let mut e = started();

    // The cycle is sitting in `working`, waiting for a turn that will never
    // come because the process is gone. `watch` is the region that sees it.
    step(&mut e, AiLoopEvent::SessionLost);
    assert!(
        holds(&e, AiLoopState::Restarting) && holds(&e, AiLoopState::Rebuilding),
        "a dead session has to be noticed independently of where the turn cycle \
         happens to be, which is why the watch is its own region; active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::SessionReady);
    assert!(
        holds(&e, AiLoopState::Priming) && holds(&e, AiLoopState::Alive),
        "both regions recover together: the run re-primes and the watch goes back \
         to alive; active: {:?}",
        active(&e)
    );
}

#[test]
fn one_cancel_reaches_every_region() {
    let mut e = started();

    step(&mut e, AiLoopEvent::Cancel);
    assert!(
        holds(&e, AiLoopState::Cancelled),
        "cancel is one transition on the `<parallel>` itself rather than one per \
         region, so a single event ends all three; active: {:?}",
        active(&e)
    );
}
