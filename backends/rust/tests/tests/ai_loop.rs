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
    // §scxml-6.2.5: the document declares its acts as sends a host serves, so
    // one has to be registered or the first act raises `error.execution`
    // instead of reaching anybody. This one performs nothing and reports
    // nothing, which is deliberate: what these scenarios measure is the
    // TOPOLOGY, and each supplies the events a host would have produced at
    // exactly the point it wants them. A handler that answered would deliver
    // the same events a second time.
    //
    // `examples/ai_loop/ai_loop_example.cpp` registers the real one. Before
    // `initialize()`, because `priming` performs its act on entry.
    e.register_event_processor("x-sce-host", |_req| Vec::new());
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

/// The verdict a completed turn is judged on.
///
/// `judging` branches on `_event.data.done`, so `judge` is one of the two
/// events this document requires a payload from — the host in
/// `examples/ai_loop/ai_loop_example.cpp` composes exactly this JSON. Sending
/// the event bare is not a shortcut with the same meaning: `_event.data` is
/// then nil, indexing it raises `error.execution` (W3C SCXML 5.9.1 has a
/// failed `cond` raise and be treated as false), and the run takes the same
/// third transition it would have taken on `done:false` while quietly
/// counting an error per turn. Both channels drove it bare until 2026-08-23
/// and every scenario stayed green, which is why
/// `a_correctly_driven_run_reports_no_errors` now measures the count instead
/// of trusting the outcome.
fn verdict(e: &mut Engine<AiLoopPolicy>, done: bool) {
    e.raise_external(
        AiLoopEvent::Judge,
        if done {
            r#"{"done":true}"#
        } else {
            r#"{"done":false}"#
        },
        "",
    );
    e.step();
}

/// One completed turn: the work finished, and the loop decides what next.
fn turn(e: &mut Engine<AiLoopPolicy>) {
    step(e, AiLoopEvent::TurnDone);
    verdict(e, false);
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

/// What a reflection writes is what the restarted session is primed with.
///
/// This is the loop's whole reason for having a restart state: `reflecting`
/// rewrites the prompts and `restarting` replaces the session so a fresh one
/// reads them. Both halves are invisible to an outcome — a run converges just
/// the same whether the text it sent afterwards was the reflection's, the
/// author's, or empty — so the C++ sibling reads the prompts it actually sent
/// (`the restarted session was not primed with an empty prompt`) and this side
/// reads what the machine holds.
///
/// It is asserted because the example was wrong here: its host wrote
/// `{"start_prompt":"","turn_prompt":"","milestone":"refined"}`, so the
/// document came back holding two empty strings and the fresh session was
/// primed with nothing at all, under a scenario titled "restarts into the
/// improved prompts". Measured 2026-08-15 in the example's own output.
#[test]
fn what_a_reflection_writes_is_what_the_machine_then_holds() {
    let mut e = started();

    let authored = e
        .policy()
        .start_prompt()
        .expect("a started loop can read its opening prompt");

    for _ in 1..=8 {
        turn(&mut e);
    }
    assert!(
        holds(&e, AiLoopState::Reflecting),
        "the document sets `reflect_every` to 8, so the eighth completed turn reflects; \
         active: {:?}",
        active(&e)
    );

    e.raise_external(
        AiLoopEvent::ReflectApplied,
        r#"{"start_prompt":"Resuming. Milestone: refined","turn_prompt":"Continue toward: refined","milestone":"refined"}"#,
        "",
    );
    e.step();

    assert_eq!(
        e.policy().milestone(),
        Some("refined".to_string()),
        "⚠ the reflection's milestone did not reach the datamodel, so the restart it is \
         about to pay for improves nothing"
    );
    let after = e
        .policy()
        .start_prompt()
        .expect("⚠ the prompt a restarted session is primed with must still be readable");
    assert_eq!(
        after, "Resuming. Milestone: refined",
        "⚠ the machine is not holding what the reflection wrote"
    );
    assert_ne!(
        after, authored,
        "the reflection has to have changed something, or this test would pass against a \
         machine that ignored it"
    );
    assert!(
        !after.is_empty(),
        "⚠ an empty prompt is what a host sends when reflection erased it, and the run \
         still converges — which is why this is asserted rather than watched"
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

/// The outcome the loop exists to reach, and the report it asks for first.
///
/// The document's opening comment claims the outcomes are enumerated, and
/// five finals spell them. Measured 2026-08-23: `converged` — the one a
/// successful run ends in — was reached by no scenario in either channel, and
/// neither was the `closing` state on the way to it. Both suites were green
/// on nineteen clauses about a loop that had never been seen finishing.
///
/// `closing` is asserted separately from the terminal because it is the whole
/// reason the document does not send `judge` straight to a final: the session
/// is asked for a closing report, and only the turn that answers it ends the
/// run.
/// A machine that jumped from the verdict to `converged` would satisfy a
/// terminal-only check and lose the report.
#[test]
fn the_run_converges_through_a_closing_report() {
    let mut e = started();
    step(&mut e, AiLoopEvent::TurnDone);
    verdict(&mut e, true);

    assert!(
        holds(&e, AiLoopState::Closing),
        "a `done` verdict asks for the closing report before ending the run; active: {:?}",
        active(&e)
    );

    step(&mut e, AiLoopEvent::TurnDone);

    assert!(
        holds(&e, AiLoopState::Converged),
        "the turn that answers the closing report reaches `reported`, whose `<raise>` is \
         what takes all three regions out at once; active: {:?}",
        active(&e)
    );
}

/// §scxml-5.9.1: a host that forgets the verdict can find out.
///
/// `judging` reads `_event.data.done`. A `judge` that carries nothing leaves
/// `_event.data` nil, indexing it fails, and the clause says a failed `cond`
/// raises `error.execution` and is treated as false — so the run does exactly
/// what a `done:false` verdict would do and heads into another turn. The two
/// deliveries are indistinguishable from the configuration, from the datamodel
/// and from the outcome: a loop driven this way never converges, however
/// finished the session reports itself to be, and nothing says why.
///
/// What tells them apart is the engine's own count. This is the same shape as
/// `unhandled_error_is_observable` and `undecodable_payload_is_reported`: the
/// behaviour is correct per the spec, and the defect would be that it is
/// unobservable.
#[test]
fn a_verdict_without_its_payload_is_reported() {
    let mut e = started();
    step(&mut e, AiLoopEvent::TurnDone);

    step(&mut e, AiLoopEvent::Judge);

    assert!(
        holds(&e, AiLoopState::Working),
        "a `cond` that could not be evaluated is treated as false, so the cycle takes the \
         unconditional third transition and works another turn; active: {:?}",
        active(&e)
    );
    assert_eq!(
        e.unhandled_error_events(),
        1,
        "the payload-less verdict raised no error a host could count, so a run that will \
         never converge looks exactly like one that has not converged yet"
    );
    assert_eq!(
        e.last_unhandled_error(),
        Some(AiLoopEvent::ErrorExecution),
        "the count has to name what it counted; a host reading only a number cannot tell a \
         failed `cond` from a failed action"
    );
}

/// The floor that makes the count above a measurement.
///
/// A counter asserted only where it is expected to move measures half of what
/// it claims: `a_verdict_without_its_payload_is_reported` would pass just as
/// well against an engine that raised `error.execution` on every event. So the
/// same run, driven the way `ai_loop_example.cpp` drives it, has to raise
/// nothing at all — through the reflection and the restart it pays for, which
/// is where the document's other payload-carrying event lands.
#[test]
fn a_correctly_driven_run_reports_no_errors() {
    let mut e = started();
    for _ in 1..=8 {
        turn(&mut e);
    }
    assert!(
        holds(&e, AiLoopState::Reflecting),
        "the eighth completed turn reflects; active: {:?}",
        active(&e)
    );

    e.raise_external(
        AiLoopEvent::ReflectApplied,
        r#"{"start_prompt":"Resuming. Milestone: refined","turn_prompt":"Continue toward: refined","milestone":"refined"}"#,
        "",
    );
    e.step();
    step(&mut e, AiLoopEvent::SessionReady);
    step(&mut e, AiLoopEvent::PromptSent);
    turn(&mut e);

    assert_eq!(
        e.unhandled_error_events(),
        0,
        "a run driven the way the document's own host drives it raises nothing; an error \
         here means the two are not asking the machine the same thing, and the channel \
         would be asserting clauses about a path no deployment takes"
    );
}

/// Rebuilding more often than the author allowed is a spent budget, not a
/// broken document.
///
/// `max_restarts` bounds how many times a session may be replaced. Measured
/// 2026-08-23: neither channel named it, so `stuck` — one of the two states
/// that reach `exhausted` — was reachable only in prose. The budget region's
/// `max_turns` had a witness; this one had none, and the two are different
/// mechanisms that happen to share a terminal.
///
/// A lost session is the cheap way in: `drive` answers `session.lost` with a
/// restart from wherever the cycle is, which is the same door reflection uses
/// and the one a real deployment hits when a process dies.
#[test]
fn a_session_replaced_past_its_budget_reports_stuck() {
    let mut e = started();
    let allowed = e
        .policy()
        .max_restarts()
        .expect("the document declares a restart budget");

    for n in 1..=allowed {
        step(&mut e, AiLoopEvent::SessionLost);
        step(&mut e, AiLoopEvent::SessionReady);
        assert!(
            holds(&e, AiLoopState::Priming),
            "replacement {n} of {allowed} is within the budget, so the fresh session is \
             primed with whatever the loop has written by now; active: {:?}",
            active(&e)
        );
    }

    step(&mut e, AiLoopEvent::SessionLost);
    step(&mut e, AiLoopEvent::SessionReady);

    assert!(
        holds(&e, AiLoopState::Exhausted),
        "the replacement past `max_restarts` reaches `stuck`, which reports the run as \
         exhausted rather than failed; active: {:?}",
        active(&e)
    );
}

/// §scxml-6.2.5: the document tells a host what to do, in its own words.
///
/// Every scenario above registers a handler that answers nothing, because what
/// they measure is the topology and each supplies its own events. That makes
/// them blind to the thing this scenario asserts: with a silent handler, a
/// `<send>` that LOST its `type="x-sce-host"` behaves exactly like one that
/// kept it — nothing is delivered either way — so the whole conversion could
/// rot back to targetless sends with both channels green.
///
/// So this one records instead of ignoring. It pins that entering `priming`
/// asks the host to prompt, and that the prompt text rides ON the act: the
/// host is TOLD what to send rather than reaching into the datamodel behind
/// the machine to find out, which is the difference the conversion bought.
#[test]
fn the_document_declares_its_acts_to_the_host() {
    let seen: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = std::sync::Arc::clone(&seen);

    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = Engine::new(AiLoopPolicy::new(script_engine));
    e.register_event_processor(
        "x-sce-host",
        move |req: sce_rust_runtime::HostSendRequest| {
            let text = req
                .params
                .get("text")
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_default();
            recorder
                .lock()
                .expect("handler log")
                .push((req.event_name.clone(), text));
            Vec::new()
        },
    );
    e.initialize();

    let acts = seen.lock().expect("handler log");
    assert_eq!(
        acts.first().map(|(name, _)| name.as_str()),
        Some("prompt.start"),
        "entering `priming` did not ask the host to prompt; the acts seen were {acts:?}"
    );
    assert!(
        acts[0].1.contains("North star:"),
        "the act carried no prompt, so a host would have to reach past the machine for one: {:?}",
        acts[0].1
    );
}

/// The sibling of `one_cancel_reaches_every_region`.
///
/// The document writes `fail` and `cancel` once each on the `<parallel>` and
/// says so in a comment — one transition rather than one per region, because a
/// run ends as a whole. Only `cancel` was asserted, and the two are not the
/// same claim: they are separate transitions to separate terminals, and a
/// consumer distinguishing "the run broke" from "somebody stopped it" reads
/// which final it ended in.
#[test]
fn a_failure_ends_the_whole_run() {
    let mut e = started();

    step(&mut e, AiLoopEvent::Fail);

    assert!(
        holds(&e, AiLoopState::Failed),
        "`fail` is written on the `<parallel>` itself, so one event takes all three regions \
         to `failed` — a different outcome from `cancelled`, which is what tells a broken \
         run from a stopped one; active: {:?}",
        active(&e)
    );
}
