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

/// A person answering does not re-introduce the session to itself.
///
/// `paused` is a sibling of `running`, so answering targets `judging` and
/// enters `running` on the way — as an ANCESTOR. §scxml-D-addAncestorStatesToEnter
/// adds such a state without its default initial child, and here the default
/// is `priming`, whose `<onentry>` sends the opening prompt. An engine that
/// gives every entered compound state its default leaves the cycle in two
/// states at once and the host, reading the configuration, sends the start
/// prompt again — measured 2026-08-15 on both AOT engines, with every W3C
/// fixture green and this file's other seventeen tests green with it.
///
/// The clause itself is pinned across all seven channels by
/// `integration_resources/ancestor_entry_is_not_default_entry/`. This test is
/// the worked example's own stake in it: the document that made the defect
/// visible asserts the shape it was found in, so a regression here fails as a
/// supervision bug rather than as an abstract entry-set one.
#[test]
fn answering_a_question_does_not_re_prime_the_session() {
    let mut e = started();
    step(&mut e, AiLoopEvent::TurnBlocked);
    step(&mut e, AiLoopEvent::ScreenNone);
    step(&mut e, AiLoopEvent::TurnDone);

    assert!(
        holds(&e, AiLoopState::Judging),
        "the answered turn has to land in `judging`; active: {:?}",
        active(&e)
    );
    assert!(
        !holds(&e, AiLoopState::Priming),
        "⚠ `running` has two children active at once: {:?}. `priming` sends `prompt.start`, \
         so a host driving this configuration re-sends the opening prompt every time a \
         person answers a dialog",
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

/// §scxml-5.3: the machine answers what its own datamodel holds.
///
/// A host supervising this loop has to size its own work against the budget
/// the document declares. Without an accessor the only readable copy is the
/// script engine's, reached with an engine handle, a session id and the
/// variable's name spelled as a string — three things a consumer should not
/// need, none of them checked by a compiler.
///
/// The half that decides the shape is `turns`: it is authored `0` and
/// assigned on every completed turn, so an accessor that answered the
/// AUTHORED literal would keep saying `0` for the whole run. What a consumer
/// asks for is the value the machine is holding now, which is why the read
/// goes to whoever owns the datamodel rather than to a copy taken at
/// generation time.
#[test]
fn the_machine_answers_what_its_own_datamodel_holds() {
    let mut e = started();

    assert_eq!(
        e.policy().max_turns(),
        Some(40),
        "the authored budget must be readable off the machine itself, in the \
         host's own type"
    );
    assert_eq!(
        e.policy().reflect_every(),
        Some(8),
        "so must the reflection cadence"
    );
    assert_eq!(
        e.policy().screen_permissions(),
        Some(false),
        "a standing answer to permission dialogs is a promise about what the loop \
         may do unattended, and a host must be able to inspect it"
    );

    assert_eq!(
        e.policy().turns(),
        Some(0),
        "no turn has completed yet, so the bookkeeping still reads its authored value"
    );
    turn(&mut e);
    assert_eq!(
        e.policy().turns(),
        Some(1),
        "⚠ the accessor must report what the datamodel HOLDS, not what the document \
         authored — a value frozen at generation time would still say 0 here, and \
         `max_turns` itself is assigned in the consumer's own copy of this loop"
    );
}

/// The strategy a host edits is the strategy it can read back.
///
/// The budget above is the numeric half of the datamodel. This is the other
/// half, and it is the half the example's own comment calls editable: the
/// north star, the milestone, the prompts built from them, the marker that
/// ends the run. A supervisor that is going to send `start_prompt` has to be
/// able to see what it is about to send, and a UI over this loop has nothing
/// to display without these.
///
/// They were unreadable for the same reason none of them looked unusual: the
/// document spells its strings with `'…'`, and the classifier deciding which
/// variables get an accessor tested for `"`. Eight of the sixteen declarations
/// were silently untyped, so this file could assert the budget and pass while
/// the strategy was not reachable at all.
///
/// `start_prompt` is asserted through its parts rather than as one literal,
/// because it is a concatenation: it exists to prove that a value the document
/// COMPUTES from its strings is readable too, not only the ones it spells out.
#[test]
fn the_strategy_a_host_edits_is_the_strategy_it_can_read_back() {
    let e = started();

    assert_eq!(
        e.policy().done_marker(),
        Some("MILESTONE REACHED".to_string()),
        "⚠ the marker that decides when the run has converged must be readable \
         off the machine — a host matching the session's report against it \
         cannot ask the document"
    );
    assert_eq!(
        e.policy().north_star(),
        Some("(edit me) the outcome this loop exists to reach".to_string()),
        "the goal the author edits is the first thing a supervisor displays"
    );
    assert_eq!(
        e.policy().milestone(),
        Some("(edit me) the next checkpoint on the way there".to_string()),
        "so is the checkpoint it is working toward"
    );

    let start = e.policy().start_prompt().expect(
        "⚠ the prompt the loop sends into a fresh session must be \
                 readable before it is sent",
    );
    assert!(
        start.contains("(edit me) the outcome this loop exists to reach")
            && start.contains("Report what you did"),
        "the composed prompt must carry the authored strings it was built from, \
         so a host reading it sees what the session will receive: {start:?}"
    );
}

/// The standing instructions are readable, which is what makes them
/// standing.
///
/// `screen_rules` is the block that decides when a person is NOT woken. The
/// document keeps it in the authored half deliberately — its own comment says
/// the loop is carrying out a decision made in advance and written down — and
/// a decision written down where nobody can read it back is indistinguishable
/// from the loop deciding on its own authority. A supervisor showing a human
/// "these three questions are being answered for you" has to get the list from
/// the machine.
///
/// It was the one declaration in the example with no reader, because the
/// accessor set stopped at the three scalar types. The parts asserted here are
/// the ones a reader acts on — which question is matched and what answer it
/// gets — rather than the whole text, so that reformatting the block inside
/// the document does not fail this.
#[test]
fn the_standing_instructions_can_be_read_back_off_the_machine() {
    let e = started();

    let rules = e.policy().screen_rules().expect(
        "⚠ the standing-instruction table must be readable off the machine — a \
         host that cannot list it cannot show anyone which questions are being \
         answered without them",
    );

    assert!(
        rules.starts_with('['),
        "the block is authored as an array and must come back as one: {rules:?}"
    );
    for question in ["design-decision", "design-proposal", "multiple-choice"] {
        assert!(
            rules.contains(question),
            "⚠ `{question}` is screened by the document but absent from what the \
             machine reports: {rules:?}"
        );
    }
    assert!(
        rules.contains("Rethink for the most durable answer"),
        "the reply a screened question receives is the half a person most needs \
         to see, and it is what distinguishes carrying out a decision from \
         making one: {rules:?}"
    );
}

/// A structured variable answers with what it is holding, not with what it
/// was declared as.
///
/// The scalar readers refuse a value of another type, and this asserts the
/// json one does too — from both directions. A write into the session must be
/// visible, because a reader frozen at generation time would answer the
/// document's literal for the whole run; and a scalar written into a variable
/// declared structured must read as "cannot answer" rather than as the
/// scalar's own JSON.
///
/// The writes go through `set_variable`, which takes a value rather than
/// source text. That is the half of the engine interface that is the same
/// whichever engine a deployment injected — `evaluate_expression` takes the
/// ENGINE's language, and this runtime is given a Lua one — so a test written
/// in either language would be asserting about the injection rather than
/// about the reader.
#[test]
fn a_structured_read_follows_the_assignment_and_refuses_another_type() {
    use sce_rust_runtime::scripting::ScriptValue;

    let e = started();

    let engine = e.policy().script_engine.clone();
    let sid = e
        .policy()
        .session_id
        .clone()
        .expect("a started machine holds a session");

    let mut later = std::collections::HashMap::new();
    later.insert("when".to_string(), ScriptValue::String("later".to_string()));
    engine
        .set_variable(
            &sid,
            "screen_rules",
            ScriptValue::Array(vec![ScriptValue::Object(later)]),
        )
        .expect("the session takes a structured value");

    let after = e
        .policy()
        .screen_rules()
        .expect("a reassigned structured variable is still readable");
    assert!(
        after.contains("later") && !after.contains("design-decision"),
        "⚠ the reader answered with the authored table after the session was \
         assigned another one: {after:?}"
    );

    engine
        .set_variable(&sid, "screen_rules", ScriptValue::Int(5))
        .expect("the session takes a scalar too");
    assert_eq!(
        e.policy().screen_rules(),
        None,
        "⚠ a variable declared structured and now holding a number must report \
         that the machine cannot answer. `5` is valid JSON, so a reader that \
         forwarded whatever the serializer produced would hand a consumer a \
         document shape that no longer exists."
    );
}

/// A machine that has not been booted cannot answer, and says so.
///
/// The failure this refuses is the one a default-valued field would produce: a
/// freshly constructed machine reporting the document's literal as though a
/// session had been created and initialised it. Nothing has read the document
/// at this point, so `None` is the only honest answer.
#[test]
fn an_uninitialised_machine_says_it_cannot_answer() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = AiLoopPolicy::new(script_engine);

    assert_eq!(
        policy.max_turns(),
        None,
        "before initialize() there is no session holding a datamodel, and answering \
         40 would be a claim about a run that has not started"
    );
}
