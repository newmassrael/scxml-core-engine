// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The AI supervision loop, driven through the C11 AOT engine.
//
// `examples/ai_loop/ai_loop.scxml` is a worked example: a statechart that
// supervises a long-running session, with <parallel> splitting the turn cycle
// from the liveness watch and the turn budget. The C++, Rust, Go, Kotlin and
// Python channels drive the same document; this is the sixth and last.
//
// Why six: a clause asserted in one channel is that engine's word for the
// document rather than the document's own, and the parallel defect that
// shipped in `1419a050ed` (a self-transition whose exit set swallowed the
// parallel root) was invisible to every W3C fixture because they are all one
// region deep. This document is three.
// `sce-build/tests/ai_loop_channel_parity.rs` holds every registered channel
// to the same scenario set BY NAME, so a scenario added here without its
// siblings fails there — which is the moment it is cheapest to fix. That
// pairing is why every `static int <name>(void)` below is a scenario and
// nothing else in this file has that shape.
//
// No sprag, no session, no pane: every effect the host would perform is
// replaced by the event that effect would have produced, so what is under test
// is the machine's topology rather than any driver's plumbing.
//
// Because the regions are orthogonal, a scenario asserts on the ACTIVE SET
// rather than on one state — "the cycle is working AND the budget is within"
// is the kind of claim a parallel machine makes. This backend makes that the
// only option and says so in its own header: `_get_current_state` does not
// exist here, because single-leaf semantics break for <parallel>.
//
// Fixture: examples/ai_loop/ai_loop.scxml, generated WITH
// `--host-processor x-sce-host` by `backends/c/tests/CMakeLists.txt`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "ai_loop_sm.h"

// The type the machine was generated for. `backends/c/tests/CMakeLists.txt`
// passes this same string to `--host-processor`; a driver registering a
// different one would measure nothing and pass.
#define DECLARED_TYPE "x-sce-host"

// ── Reporting ──────────────────────────────────────────────────────
//
// One line per failure, naming the scenario and what it saw, because a C
// runner has no assertion library to say it for us and a bare non-zero exit
// names nothing.

static int fail(const char *scenario, const char *message) {
    (void)fprintf(stderr, "ai_loop: FAIL [%s] - %s\n", scenario, message);
    return 1;
}

static int fail_num(const char *scenario, const char *what, long long got, long long want) {
    (void)fprintf(stderr, "ai_loop: FAIL [%s] - %s is %lld, expected %lld\n", scenario, what, got, want);
    return 1;
}

// The active set in the document's own words. Printed on the way out of a
// failing scenario: `[alive within working]` says where the machine is and a
// bitmap does not.
static void report_where(const char *scenario) {
    (void)fprintf(stderr, "ai_loop: [%s] active set follows\n", scenario);
}

static void print_active(const ai_loop_t *sm) {
    (void)fprintf(stderr, "  active:");
    for (int s = 0; s < (int)AI_LOOP_STATE_COUNT; s++) {
        if (ai_loop_in_state(sm, (ai_loop_state_t)s)) {
            (void)fprintf(stderr, " %s", ai_loop_state_name((ai_loop_state_t)s));
        }
    }
    (void)fprintf(stderr, "\n");
}

static int fail_where(const char *scenario, const char *message, const ai_loop_t *sm) {
    (void)fail(scenario, message);
    report_where(scenario);
    print_active(sm);
    return 1;
}

// ── Driving ────────────────────────────────────────────────────────

// What a recording host handler saw. The acts are the only witness for two
// scenarios: with a silent handler a <send> that lost its `type` behaves
// exactly like one that kept it.
typedef struct {
    int calls;
    char first_event[64];
    char first_text[SCE_MAX_DATA_LEN];
} recorder_t;

static void recording_handler(void *user_data, const sce_host_send_request_t *request,
                              sce_host_send_response_list_t *out) {
    recorder_t *rec = (recorder_t *)user_data;
    (void)out;
    if (rec->calls == 0) {
        (void)snprintf(rec->first_event, sizeof(rec->first_event), "%s",
                       request->event_name != NULL ? request->event_name : "");
        const char *text = sce_host_send_param(request, "text");
        (void)snprintf(rec->first_text, sizeof(rec->first_text), "%s", text != NULL ? text : "");
    }
    rec->calls++;
}

// W3C SCXML 6.2.5: the document declares its acts as sends a host serves, so
// one has to be registered or the first act raises `error.execution` instead
// of reaching anybody. This one performs nothing and reports nothing, which is
// deliberate: what these scenarios measure is the TOPOLOGY, and each supplies
// the events a host would have produced at exactly the point it wants them. A
// handler that answered would deliver the same events a second time.
static void silent_handler(void *user_data, const sce_host_send_request_t *request,
                           sce_host_send_response_list_t *out) {
    (void)user_data;
    (void)request;
    (void)out;
}

static void wire(sce_host_processor_registry_t *wiring, sce_host_send_handler_fn handler, void *user_data) {
    memset(wiring, 0, sizeof(*wiring));
    (void)sce_host_registry_register(wiring, DECLARED_TYPE, handler, user_data);
}

// A booted machine, sitting in `priming` with nothing prompted yet. The
// registry goes INTO `_init` because `priming` performs its act on entry — a
// handler installed afterwards would arrive one act too late.
static void boot(ai_loop_t *sm, sce_host_processor_registry_t *wiring) {
    wire(wiring, silent_handler, NULL);
    ai_loop_init_with_host_processors(sm, wiring);
    ai_loop_step(sm);
}

static void step(ai_loop_t *sm, ai_loop_event_t event) {
    ai_loop_event_with_meta_t evt;
    memset(&evt, 0, sizeof(evt));
    evt.event = event;
    ai_loop_raise_external(sm, &evt);
    ai_loop_step(sm);
}

static void step_with_data(ai_loop_t *sm, ai_loop_event_t event, const char *data) {
    ai_loop_event_with_meta_t evt;
    memset(&evt, 0, sizeof(evt));
    evt.event = event;
    (void)snprintf(evt.data, sizeof(evt.data), "%s", data);
    ai_loop_raise_external(sm, &evt);
    ai_loop_step(sm);
}

// A run whose first prompt has been sent — where every scenario starts.
static void start(ai_loop_t *sm, sce_host_processor_registry_t *wiring) {
    boot(sm, wiring);
    step(sm, AI_LOOP_EVENT_PROMPT_SENT);
}

// The verdict a completed turn is judged on.
//
// `judging` branches on `_event.data.done`, so `judge` is one of the two
// events this document requires a payload from — the host in
// `examples/ai_loop/ai_loop_example.cpp` composes exactly this JSON. Sending
// it bare is not a shortcut with the same meaning: `_event.data` is then
// absent, reading a field off it fails, and W3C SCXML 5.9.1 has a failed
// `cond` raise `error.execution` and be treated as false — so the run takes
// the same third transition a `done:false` verdict would while quietly
// counting an error per turn.
static void verdict(ai_loop_t *sm, bool done) {
    step_with_data(sm, AI_LOOP_EVENT_JUDGE, done ? "{\"done\":true}" : "{\"done\":false}");
}

// One completed turn: the work finished, and the loop decides what next.
static void turn(ai_loop_t *sm) {
    step(sm, AI_LOOP_EVENT_TURN_DONE);
    verdict(sm, false);
}

// ── Scenarios ──────────────────────────────────────────────────────

static int all_three_regions_are_live_at_once(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_WORKING) || !ai_loop_in_state(&sm, AI_LOOP_STATE_ALIVE) ||
        !ai_loop_in_state(&sm, AI_LOOP_STATE_WITHIN)) {
        bad = fail_where(
            "all_three_regions_are_live_at_once",
            "the cycle, the liveness watch and the budget are orthogonal regions and must all be active at once", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int reflection_fires_on_schedule(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int at = 0;
    for (int n = 1; n <= 10; n++) {
        turn(&sm);
        if (ai_loop_in_state(&sm, AI_LOOP_STATE_REFLECTING)) {
            at = n;
            break;
        }
    }
    int bad = 0;
    if (at != 8) {
        bad = fail_num("reflection_fires_on_schedule", "the turn the document's `reflect_every` of 8 should reflect on",
                       at, 8);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int reflection_goes_through_a_restart_and_the_loop_re_primes(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);
    for (int n = 0; n < 8; n++) {
        turn(&sm);
    }

    int bad = 0;
    step(&sm, AI_LOOP_EVENT_REFLECT_APPLIED);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_RESTARTING)) {
        bad |= fail_where(
            "reflection_goes_through_a_restart_and_the_loop_re_primes",
            "a session reads its context once, when it starts, so applying a reflection has to REPLACE it", &sm);
    }

    step(&sm, AI_LOOP_EVENT_SESSION_READY);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PRIMING)) {
        bad |= fail_where("reflection_goes_through_a_restart_and_the_loop_re_primes",
                          "a replaced session starts empty and must be primed with the current prompts", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int the_budget_ends_the_run_from_wherever_the_cycle_is(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    for (int n = 0; n < 60; n++) {
        if (ai_loop_in_state(&sm, AI_LOOP_STATE_REFLECTING)) {
            step(&sm, AI_LOOP_EVENT_REFLECT_NONE);
        }
        if (ai_loop_in_state(&sm, AI_LOOP_STATE_EXHAUSTED)) {
            break;
        }
        turn(&sm);
    }

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_EXHAUSTED)) {
        bad = fail_where(
            "the_budget_ends_the_run_from_wherever_the_cycle_is",
            "the budget is its own region precisely so the turn count is not something `judging` has to check", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int a_standing_instruction_answers_without_waking_anybody(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    step(&sm, AI_LOOP_EVENT_TURN_BLOCKED);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_SCREENING)) {
        bad |= fail_where("a_standing_instruction_answers_without_waking_anybody",
                          "a dialog is screened against the rules the person wrote in advance before anyone is woken",
                          &sm);
    }

    step(&sm, AI_LOOP_EVENT_SCREEN_MATCHED);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_WORKING) || ai_loop_in_state(&sm, AI_LOOP_STATE_PAUSED)) {
        bad |= fail_where(
            "a_standing_instruction_answers_without_waking_anybody",
            "a matched rule is a decision the person already made, so the run carries on and nobody is woken", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int an_unmatched_dialog_wakes_the_person_who_answers(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    step(&sm, AI_LOOP_EVENT_TURN_BLOCKED);
    step(&sm, AI_LOOP_EVENT_SCREEN_NONE);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PAUSED)) {
        bad |= fail_where("an_unmatched_dialog_wakes_the_person_who_answers",
                          "the loop answers only what the person decided in advance; anything else stops it and waits",
                          &sm);
    }

    step(&sm, AI_LOOP_EVENT_TURN_DONE);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_JUDGING)) {
        bad |= fail_where("an_unmatched_dialog_wakes_the_person_who_answers",
                          "once the person has answered, the turn completes where it left off", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

// A person answering does not re-introduce the session to itself.
//
// `paused` is a sibling of `running`, so answering targets `judging` and
// enters `running` on the way — as an ANCESTOR. W3C SCXML Appendix D
// addAncestorStatesToEnter adds such a state without its default initial
// child, and here the default is `priming`, whose <onentry> sends the opening
// prompt. An engine that gives every entered compound state its default leaves
// the cycle in two states at once and the host, reading the configuration,
// sends the start prompt again — measured 2026-08-15 on both AOT engines, with
// every W3C fixture green.
static int answering_a_question_does_not_re_prime_the_session(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_TURN_BLOCKED);
    step(&sm, AI_LOOP_EVENT_SCREEN_NONE);
    step(&sm, AI_LOOP_EVENT_TURN_DONE);

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_JUDGING)) {
        bad |= fail_where("answering_a_question_does_not_re_prime_the_session",
                          "the answered turn has to land in `judging`", &sm);
    }
    if (ai_loop_in_state(&sm, AI_LOOP_STATE_PRIMING)) {
        bad |= fail_where("answering_a_question_does_not_re_prime_the_session",
                          "`running` has two children active at once, so a host would re-send the opening prompt", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int hold_and_resume_return_to_exactly_where_the_cycle_was(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);
    turn(&sm);

    int bad = 0;
    step(&sm, AI_LOOP_EVENT_HOLD);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PAUSED)) {
        bad |= fail_where("hold_and_resume_return_to_exactly_where_the_cycle_was",
                          "a person looking at the work holds the cycle", &sm);
    }

    step(&sm, AI_LOOP_EVENT_RESUME);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_WORKING)) {
        bad |= fail_where("hold_and_resume_return_to_exactly_where_the_cycle_was",
                          "resuming puts the cycle back to work rather than ending the run", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

// `<history id="where">` declares `<transition target="working"/>` as its
// default, so a hold taken while the cycle is in `working` resumes there
// whether history recorded anything or not — the scenario above cannot tell a
// working history from one that records nothing. `priming` is the one place
// the two answers differ.
static int resume_returns_somewhere_the_history_default_does_not(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    boot(&sm, &wiring);

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PRIMING)) {
        bad |= fail_where("resume_returns_somewhere_the_history_default_does_not",
                          "the run starts with a session that exists and has not been prompted", &sm);
    }

    step(&sm, AI_LOOP_EVENT_HOLD);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PAUSED)) {
        bad |= fail_where("resume_returns_somewhere_the_history_default_does_not",
                          "a person can take over before the first prompt as readily as after one", &sm);
    }

    step(&sm, AI_LOOP_EVENT_RESUME);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PRIMING) || ai_loop_in_state(&sm, AI_LOOP_STATE_WORKING)) {
        bad |= fail_where(
            "resume_returns_somewhere_the_history_default_does_not",
            "`<history>` must restore the state the cycle was in; landing in `working` is the default answering", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int the_person_interrupts_the_inner_session_by_hand(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    step(&sm, AI_LOOP_EVENT_TURN_INTERRUPTED);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PAUSED) || ai_loop_in_state(&sm, AI_LOOP_STATE_SCREENING)) {
        bad |= fail_where("the_person_interrupts_the_inner_session_by_hand",
                          "a person typing into the session is not a dialog to screen; the loop stays out of the way",
                          &sm);
    }

    step(&sm, AI_LOOP_EVENT_TURN_INTERRUPTED);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PAUSED)) {
        bad |= fail_where("the_person_interrupts_the_inner_session_by_hand",
                          "further interruptions keep it paused rather than fighting the person for the session", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int nobody_comes(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    step(&sm, AI_LOOP_EVENT_TURN_BLOCKED);
    step(&sm, AI_LOOP_EVENT_SCREEN_NONE);
    step(&sm, AI_LOOP_EVENT_UNATTENDED);

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_BLOCKED)) {
        bad =
            fail_where("nobody_comes", "a question nobody answers ends the run in an outcome the document names", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int a_pane_that_dies_mid_turn_is_noticed_and_rebuilt(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    // The cycle is sitting in `working`, waiting for a turn that will never
    // come because the process is gone. `watch` is the region that sees it.
    step(&sm, AI_LOOP_EVENT_SESSION_LOST);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_RESTARTING) || !ai_loop_in_state(&sm, AI_LOOP_STATE_REBUILDING)) {
        bad |= fail_where("a_pane_that_dies_mid_turn_is_noticed_and_rebuilt",
                          "a dead session has to be noticed independently of where the turn cycle is", &sm);
    }

    step(&sm, AI_LOOP_EVENT_SESSION_READY);
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PRIMING) || !ai_loop_in_state(&sm, AI_LOOP_STATE_ALIVE)) {
        bad |= fail_where("a_pane_that_dies_mid_turn_is_noticed_and_rebuilt",
                          "both regions recover together: the run re-primes and the watch goes back to alive", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

static int one_cancel_reaches_every_region(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    step(&sm, AI_LOOP_EVENT_CANCEL);

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_CANCELLED)) {
        bad = fail_where(
            "one_cancel_reaches_every_region",
            "cancel is one transition on the `<parallel>` itself, so a single event ends all three regions", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

// W3C SCXML 5.3: the machine answers what its own datamodel holds.
//
// A host supervising this loop has to size its own work against the budget the
// document declares. The half that decides the shape is `turns`: it is
// authored 0 and assigned on every completed turn, so an accessor that
// answered the AUTHORED literal would keep saying 0 for the whole run.
static int the_machine_answers_what_its_own_datamodel_holds(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    int64_t value = 0;
    if (!ai_loop_max_turns(&sm, &value) || value != 40) {
        bad |= fail_num("the_machine_answers_what_its_own_datamodel_holds", "max_turns", (long long)value, 40);
    }
    if (!ai_loop_reflect_every(&sm, &value) || value != 8) {
        bad |= fail_num("the_machine_answers_what_its_own_datamodel_holds", "reflect_every", (long long)value, 8);
    }
    bool permissions = true;
    if (!ai_loop_screen_permissions(&sm, &permissions) || permissions) {
        bad |= fail("the_machine_answers_what_its_own_datamodel_holds",
                    "a standing answer to permission dialogs must be readable, and it is authored false");
    }

    if (!ai_loop_turns(&sm, &value) || value != 0) {
        bad |= fail_num("the_machine_answers_what_its_own_datamodel_holds", "turns before any turn completed",
                        (long long)value, 0);
    }
    turn(&sm);
    if (!ai_loop_turns(&sm, &value) || value != 1) {
        bad |= fail_num("the_machine_answers_what_its_own_datamodel_holds",
                        "turns after one completed — the accessor must report what the datamodel HOLDS",
                        (long long)value, 1);
    }
    ai_loop_destroy(&sm);
    return bad;
}

// The strategy a host edits is the strategy it can read back.
//
// `start_prompt` is asserted through its parts rather than as one literal,
// because it is a concatenation: it exists to prove that a value the document
// COMPUTES from its strings is readable too, not only the ones it spells out.
static int the_strategy_a_host_edits_is_the_strategy_it_can_read_back(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    char buf[512];
    size_t len = 0;

    if (!ai_loop_done_marker(&sm, buf, sizeof(buf), &len) || strcmp(buf, "MILESTONE REACHED") != 0) {
        bad |= fail("the_strategy_a_host_edits_is_the_strategy_it_can_read_back",
                    "the marker that decides when the run has converged must be readable off the machine");
    }
    if (!ai_loop_north_star(&sm, buf, sizeof(buf), &len) ||
        strcmp(buf, "(edit me) the outcome this loop exists to reach") != 0) {
        bad |= fail("the_strategy_a_host_edits_is_the_strategy_it_can_read_back",
                    "the goal the author edits is the first thing a supervisor displays");
    }
    if (!ai_loop_milestone(&sm, buf, sizeof(buf), &len) ||
        strcmp(buf, "(edit me) the next checkpoint on the way there") != 0) {
        bad |= fail("the_strategy_a_host_edits_is_the_strategy_it_can_read_back",
                    "so is the checkpoint it is working toward");
    }

    if (!ai_loop_start_prompt(&sm, buf, sizeof(buf), &len)) {
        bad |= fail("the_strategy_a_host_edits_is_the_strategy_it_can_read_back",
                    "the prompt the loop sends into a fresh session must be readable before it is sent");
    } else if (strstr(buf, "(edit me) the outcome this loop exists to reach") == NULL ||
               strstr(buf, "Report what you did") == NULL) {
        (void)fprintf(stderr,
                      "ai_loop: FAIL [the_strategy_a_host_edits_is_the_strategy_it_can_read_back] - "
                      "the composed prompt lost the authored strings it was built from: %s\n",
                      buf);
        bad |= 1;
    }
    ai_loop_destroy(&sm);
    return bad;
}

// The standing instructions are readable, which is what makes them standing.
//
// A decision written down where nobody can read it back is indistinguishable
// from the loop deciding on its own authority. The parts asserted are the ones
// a reader acts on, so reformatting the block inside the document does not
// fail this.
static int the_standing_instructions_can_be_read_back_off_the_machine(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    char buf[1024];
    size_t len = 0;

    if (!ai_loop_screen_rules(&sm, buf, sizeof(buf), &len)) {
        bad |= fail("the_standing_instructions_can_be_read_back_off_the_machine",
                    "the standing-instruction table must be readable off the machine");
    } else {
        if (buf[0] != '[') {
            bad |= fail("the_standing_instructions_can_be_read_back_off_the_machine",
                        "the block is authored as an array and must come back as one");
        }
        // The buffer is larger than the block, so a truncated read is a
        // reader defect rather than a caller one — asserted because the parts
        // below are all near the front and would survive a short answer.
        if (len >= sizeof(buf)) {
            bad |= fail("the_standing_instructions_can_be_read_back_off_the_machine",
                        "the table did not fit the buffer this scenario provided");
        }
        static const char *const questions[] = {"design-decision", "design-proposal", "multiple-choice"};
        for (size_t i = 0; i < sizeof(questions) / sizeof(questions[0]); i++) {
            if (strstr(buf, questions[i]) == NULL) {
                (void)fprintf(stderr,
                              "ai_loop: FAIL [the_standing_instructions_can_be_read_back_off_the_machine] - "
                              "`%s` is screened by the document but absent from what the machine reports\n",
                              questions[i]);
                bad |= 1;
            }
        }
        if (strstr(buf, "Rethink for the most durable answer") == NULL) {
            bad |= fail("the_standing_instructions_can_be_read_back_off_the_machine",
                        "the reply a screened question receives is the half a person most needs to see");
        }
    }
    ai_loop_destroy(&sm);
    return bad;
}

// A structured variable answers with what it is holding, not with what it was
// declared as.
//
// The write goes through the Lua C API rather than `luaL_dostring`, and that
// is the same choice the five sibling channels made for the same reason: their
// engines take a VALUE through `set_variable` while `evaluate_expression`
// takes the engine's language, and a test written in source text would be
// asserting about the transpiler rather than about the reader. This backend
// embeds Lua directly and publishes the state on the struct, so the value is
// pushed here.
static int a_structured_read_follows_the_assignment_and_refuses_another_type(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    char buf[1024];
    size_t len = 0;

    // `[{"when":"later"}]` as a value: an array of one object.
    lua_newtable(sm.L);
    lua_newtable(sm.L);
    lua_pushstring(sm.L, "later");
    lua_setfield(sm.L, -2, "when");
    lua_rawseti(sm.L, -2, 1);
    lua_setglobal(sm.L, "screen_rules");

    if (!ai_loop_screen_rules(&sm, buf, sizeof(buf), &len)) {
        bad |= fail("a_structured_read_follows_the_assignment_and_refuses_another_type",
                    "a reassigned structured variable is still readable");
    } else if (strstr(buf, "later") == NULL || strstr(buf, "design-decision") != NULL) {
        (void)fprintf(stderr,
                      "ai_loop: FAIL [a_structured_read_follows_the_assignment_and_refuses_another_type] - "
                      "the reader answered with the authored table after the session was assigned another: %s\n",
                      buf);
        bad |= 1;
    }

    lua_pushinteger(sm.L, 5);
    lua_setglobal(sm.L, "screen_rules");
    if (ai_loop_screen_rules(&sm, buf, sizeof(buf), &len)) {
        bad |= fail("a_structured_read_follows_the_assignment_and_refuses_another_type",
                    "a variable declared structured and now holding a number must report that it cannot answer");
    }
    ai_loop_destroy(&sm);
    return bad;
}

// What a reflection writes is what the restarted session is primed with.
//
// Both halves are invisible to an outcome — a run converges just the same
// whether the text it sent afterwards was the reflection's, the author's, or
// empty. It is asserted because the example was wrong here: its host wrote two
// empty strings and the fresh session was primed with nothing at all, under a
// scenario titled "restarts into the improved prompts".
static int what_a_reflection_writes_is_what_the_machine_then_holds(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    char authored[512];
    char after[512];
    size_t len = 0;

    if (!ai_loop_start_prompt(&sm, authored, sizeof(authored), &len)) {
        bad |= fail("what_a_reflection_writes_is_what_the_machine_then_holds",
                    "a started loop can read its opening prompt");
    }

    for (int n = 0; n < 8; n++) {
        turn(&sm);
    }
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_REFLECTING)) {
        bad |= fail_where("what_a_reflection_writes_is_what_the_machine_then_holds",
                          "the eighth completed turn reflects", &sm);
    }

    step_with_data(&sm, AI_LOOP_EVENT_REFLECT_APPLIED,
                   "{\"start_prompt\":\"Resuming. Milestone: refined\","
                   "\"turn_prompt\":\"Continue toward: refined\",\"milestone\":\"refined\"}");

    char milestone[512];
    if (!ai_loop_milestone(&sm, milestone, sizeof(milestone), &len) || strcmp(milestone, "refined") != 0) {
        bad |= fail("what_a_reflection_writes_is_what_the_machine_then_holds",
                    "the reflection's milestone did not reach the datamodel, so the restart improves nothing");
    }
    if (!ai_loop_start_prompt(&sm, after, sizeof(after), &len)) {
        bad |= fail("what_a_reflection_writes_is_what_the_machine_then_holds",
                    "the prompt a restarted session is primed with must still be readable");
    } else {
        if (strcmp(after, "Resuming. Milestone: refined") != 0) {
            bad |= fail("what_a_reflection_writes_is_what_the_machine_then_holds",
                        "the machine is not holding what the reflection wrote");
        }
        if (strcmp(after, authored) == 0) {
            bad |= fail("what_a_reflection_writes_is_what_the_machine_then_holds",
                        "the reflection has to have changed something, or this scenario would pass against a machine "
                        "that ignored it");
        }
        if (after[0] == '\0') {
            bad |= fail("what_a_reflection_writes_is_what_the_machine_then_holds",
                        "an empty prompt is what a host sends when reflection erased it, and the run still converges");
        }
    }
    ai_loop_destroy(&sm);
    return bad;
}

// A machine that has not been booted cannot answer, and says so.
//
// The failure this refuses is the one a default-valued field would produce: a
// freshly constructed machine reporting the document's literal as though a
// session had been created and initialised it.
static int an_uninitialised_machine_says_it_cannot_answer(void) {
    ai_loop_t sm;
    memset(&sm, 0, sizeof(sm));

    int64_t value = -1;
    int bad = 0;
    if (ai_loop_max_turns(&sm, &value)) {
        bad = fail(
            "an_uninitialised_machine_says_it_cannot_answer",
            "before _init there is no datamodel, and answering 40 would be a claim about a run that never started");
    }
    // No `_destroy`: nothing was initialised, and destroying a zeroed struct
    // is a different scenario from this one.
    return bad;
}

// The outcome the loop exists to reach, and the report it asks for first.
//
// `closing` is asserted separately from the terminal because it is the whole
// reason the document does not send `judge` straight to a final: the session is
// asked for a closing report, and only the turn that answers it ends the run.
static int the_run_converges_through_a_closing_report(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_TURN_DONE);
    verdict(&sm, true);

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_CLOSING)) {
        bad |= fail_where("the_run_converges_through_a_closing_report",
                          "a `done` verdict asks for the closing report before ending the run", &sm);
    }

    step(&sm, AI_LOOP_EVENT_TURN_DONE);

    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_CONVERGED)) {
        bad |= fail_where(
            "the_run_converges_through_a_closing_report",
            "the turn that answers the closing report reaches `reported`, whose <raise> ends all three regions", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

// W3C SCXML 5.9.1: a host that forgets the verdict can find out.
//
// The two deliveries are indistinguishable from the configuration, from the
// datamodel and from the outcome: a loop driven this way never converges,
// however finished the session reports itself to be, and nothing says why.
// What tells them apart is the engine's own count.
static int a_verdict_without_its_payload_is_reported(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_TURN_DONE);

    step(&sm, AI_LOOP_EVENT_JUDGE);

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_WORKING)) {
        bad |= fail_where("a_verdict_without_its_payload_is_reported",
                          "a `cond` that could not be evaluated is treated as false, so the cycle works another turn",
                          &sm);
    }
    uint32_t errors = ai_loop_unhandled_error_events(&sm);
    if (errors != 1u) {
        bad |= fail_num("a_verdict_without_its_payload_is_reported",
                        "unhandled error events after a payload-less verdict", (long long)errors, 1);
    }
    ai_loop_event_t last = AI_LOOP_EVENT_UNATTENDED;
    if (!ai_loop_last_unhandled_error(&sm, &last) || last != AI_LOOP_EVENT_ERROR_EXECUTION) {
        bad |= fail("a_verdict_without_its_payload_is_reported",
                    "the count has to name what it counted; a host reading a number cannot tell a failed `cond` from a "
                    "failed action");
    }
    ai_loop_destroy(&sm);
    return bad;
}

// The floor that makes the count above a measurement.
//
// A counter asserted only where it is expected to move measures half of what
// it claims. So the same run, driven the way `ai_loop_example.cpp` drives it,
// has to raise nothing at all — through the reflection and the restart it pays
// for, which is where the document's other payload-carrying event lands.
static int a_correctly_driven_run_reports_no_errors(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);
    for (int n = 0; n < 8; n++) {
        turn(&sm);
    }

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_REFLECTING)) {
        bad |= fail_where("a_correctly_driven_run_reports_no_errors", "the eighth completed turn reflects", &sm);
    }

    step_with_data(&sm, AI_LOOP_EVENT_REFLECT_APPLIED,
                   "{\"start_prompt\":\"Resuming. Milestone: refined\","
                   "\"turn_prompt\":\"Continue toward: refined\",\"milestone\":\"refined\"}");
    step(&sm, AI_LOOP_EVENT_SESSION_READY);
    step(&sm, AI_LOOP_EVENT_PROMPT_SENT);
    turn(&sm);

    uint32_t errors = ai_loop_unhandled_error_events(&sm);
    if (errors != 0u) {
        bad |= fail_num("a_correctly_driven_run_reports_no_errors",
                        "unhandled error events on the path the document's own host takes", (long long)errors, 0);
    }
    ai_loop_destroy(&sm);
    return bad;
}

// Rebuilding more often than the author allowed is a spent budget, not a
// broken document. `max_restarts` bounds how many times a session may be
// replaced, and `stuck` — one of the two states that reach `exhausted` — was
// reachable only in prose until a channel named it.
static int a_session_replaced_past_its_budget_reports_stuck(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    int bad = 0;
    int64_t allowed = 0;
    if (!ai_loop_max_restarts(&sm, &allowed)) {
        bad |= fail("a_session_replaced_past_its_budget_reports_stuck", "the document declares a restart budget");
        ai_loop_destroy(&sm);
        return bad;
    }

    for (int64_t n = 1; n <= allowed; n++) {
        step(&sm, AI_LOOP_EVENT_SESSION_LOST);
        step(&sm, AI_LOOP_EVENT_SESSION_READY);
        if (!ai_loop_in_state(&sm, AI_LOOP_STATE_PRIMING)) {
            bad |= fail_where("a_session_replaced_past_its_budget_reports_stuck",
                              "a replacement within the budget primes the fresh session", &sm);
        }
    }

    step(&sm, AI_LOOP_EVENT_SESSION_LOST);
    step(&sm, AI_LOOP_EVENT_SESSION_READY);

    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_EXHAUSTED)) {
        bad |= fail_where(
            "a_session_replaced_past_its_budget_reports_stuck",
            "the replacement past `max_restarts` reaches `stuck`, which reports exhausted rather than failed", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

// W3C SCXML 6.2.5: the document tells a host what to do, in its own words.
//
// Every scenario above registers a handler that answers nothing, which makes
// them blind to the thing this one asserts: with a silent handler, a <send>
// that LOST its `type="x-sce-host"` behaves exactly like one that kept it. So
// this one records, and pins that the prompt text rides ON the act.
static int the_document_declares_its_acts_to_the_host(void) {
    recorder_t rec;
    memset(&rec, 0, sizeof(rec));

    sce_host_processor_registry_t wiring;
    wire(&wiring, recording_handler, &rec);

    ai_loop_t sm;
    ai_loop_init_with_host_processors(&sm, &wiring);

    int bad = 0;
    if (rec.calls == 0) {
        bad |= fail("the_document_declares_its_acts_to_the_host",
                    "entering `priming` asked the host to perform nothing at all");
    } else {
        if (strcmp(rec.first_event, "prompt.start") != 0) {
            (void)fprintf(stderr,
                          "ai_loop: FAIL [the_document_declares_its_acts_to_the_host] - "
                          "entering `priming` asked for `%s`, not `prompt.start`\n",
                          rec.first_event);
            bad |= 1;
        }
        if (strstr(rec.first_text, "North star:") == NULL) {
            (void)fprintf(stderr,
                          "ai_loop: FAIL [the_document_declares_its_acts_to_the_host] - "
                          "the act carried no prompt, so a host would reach past the machine for one: %s\n",
                          rec.first_text);
            bad |= 1;
        }
    }
    ai_loop_destroy(&sm);
    return bad;
}

// The sibling of `one_cancel_reaches_every_region`. `fail` and `cancel` are
// separate transitions to separate terminals, and a consumer distinguishing
// "the run broke" from "somebody stopped it" reads which final it ended in.
static int a_failure_ends_the_whole_run(void) {
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;
    start(&sm, &wiring);

    step(&sm, AI_LOOP_EVENT_FAIL);

    int bad = 0;
    if (!ai_loop_in_state(&sm, AI_LOOP_STATE_FAILED)) {
        bad = fail_where(
            "a_failure_ends_the_whole_run",
            "`fail` is written on the `<parallel>` itself, so one event takes all three regions to `failed`", &sm);
    }
    ai_loop_destroy(&sm);
    return bad;
}

// ══════════════════════════════════════════════════════════════════
// A run that outlived its process
//
// `_init_at` takes states, and nothing that crosses a process boundary can
// carry one: a journal, a wire and a file all carry STRINGS. `_state_name`
// writes that record and `_state_from_name` reads it back, and until the
// second existed the door could be called and its argument could not be built
// — a supervisor coming back had to `_init` instead, which is a replay rather
// than a resume: `priming` performs its prompt on entry, so the restored loop
// typed the first prompt again.
//
// This backend's door differs from the other five in one way worth naming: it
// takes the configuration alone. There is no `current` argument because there
// is no current-state shadow to restore — the header says so, and the reason
// is that single-leaf semantics break for <parallel>. So the half the Python
// channel spells as "the recorded leaf is among the restored leaves" has no
// spelling here at all, and this scenario asserts what this engine publishes.
// ══════════════════════════════════════════════════════════════════

static int a_run_journalled_as_names_resumes_where_it_stopped(void) {
    ai_loop_t ran;
    sce_host_processor_registry_t ran_wiring;
    start(&ran, &ran_wiring);
    turn(&ran);
    turn(&ran);

    // Everything a host can persist. Not states, not a bitmap: text.
    const char *journal[AI_LOOP_STATE_COUNT];
    size_t journalled = 0;
    for (int s = 0; s < (int)AI_LOOP_STATE_COUNT; s++) {
        if (ai_loop_in_state(&ran, (ai_loop_state_t)s)) {
            journal[journalled++] = ai_loop_state_name((ai_loop_state_t)s);
        }
    }
    ai_loop_destroy(&ran);

    int bad = 0;
    bool saw_working = false;
    for (size_t i = 0; i < journalled; i++) {
        if (strcmp(journal[i], "working") == 0) {
            saw_working = true;
        }
    }
    if (!saw_working) {
        bad |= fail("a_run_journalled_as_names_resumes_where_it_stopped",
                    "the journal is meant to be taken mid-run, with the cycle at work");
    }

    // A new process, holding nothing but those strings.
    ai_loop_state_t configuration[AI_LOOP_STATE_COUNT];
    size_t count = 0;
    for (size_t i = 0; i < journalled; i++) {
        ai_loop_state_t state;
        if (!ai_loop_state_from_name(journal[i], &state)) {
            (void)fprintf(stderr,
                          "ai_loop: FAIL [a_run_journalled_as_names_resumes_where_it_stopped] - "
                          "`%s` is a name this document published and it did not read back\n",
                          journal[i]);
            return 1;
        }
        configuration[count++] = state;
    }

    recorder_t rec;
    memset(&rec, 0, sizeof(rec));
    sce_host_processor_registry_t wiring;
    wire(&wiring, recording_handler, &rec);

    ai_loop_t resumed;
    ai_loop_configuration_rejection_t verdict_ = ai_loop_init_at(&resumed, configuration, count, "", &wiring);
    if (verdict_ != AI_LOOP_CONFIG_NONE) {
        bad |= fail_num("a_run_journalled_as_names_resumes_where_it_stopped",
                        "the rejection a configuration this document published was refused with", (long long)verdict_,
                        (long long)AI_LOOP_CONFIG_NONE);
        return bad;
    }

    for (size_t i = 0; i < count; i++) {
        if (!ai_loop_in_state(&resumed, configuration[i])) {
            bad |= fail_where("a_run_journalled_as_names_resumes_where_it_stopped",
                              "the machine came back somewhere other than where the journal said it was", &resumed);
            break;
        }
    }
    uint32_t restored = ai_loop_active_states(&resumed);
    uint32_t expected = 0u;
    for (size_t i = 0; i < count; i++) {
        expected |= (uint32_t)(1u << (unsigned)configuration[i]);
    }
    if (restored != expected) {
        bad |= fail_where("a_run_journalled_as_names_resumes_where_it_stopped",
                          "the restored configuration holds states the journal did not name", &resumed);
    }
    if (ai_loop_is_in_final_state(&resumed)) {
        bad |= fail("a_run_journalled_as_names_resumes_where_it_stopped",
                    "a machine put back into a mid-run configuration is not a finished one");
    }
    if (rec.calls != 0) {
        bad |=
            fail_num("a_run_journalled_as_names_resumes_where_it_stopped",
                     "acts performed while resuming — the replay `_init_at` exists to avoid", (long long)rec.calls, 0);
    }
    ai_loop_destroy(&resumed);
    return bad;
}

static int every_state_a_run_reaches_reads_back_from_its_own_name(void) {
    uint32_t seen = 0u;
    ai_loop_t sm;
    sce_host_processor_registry_t wiring;

    // Every outcome the document names, walked rather than listed: a state is
    // recorded here only because a run actually stood in it, and a written-out
    // list of states is what `_state_from_name` exists to replace.
    start(&sm, &wiring);
    seen |= ai_loop_active_states(&sm);
    for (int n = 0; n < 60; n++) {
        if (ai_loop_in_state(&sm, AI_LOOP_STATE_REFLECTING)) {
            seen |= ai_loop_active_states(&sm);
            step(&sm, AI_LOOP_EVENT_REFLECT_APPLIED);
            seen |= ai_loop_active_states(&sm);
            step(&sm, AI_LOOP_EVENT_SESSION_READY);
        }
        if (ai_loop_in_state(&sm, AI_LOOP_STATE_EXHAUSTED)) {
            break;
        }
        turn(&sm);
        seen |= ai_loop_active_states(&sm);
    }
    seen |= ai_loop_active_states(&sm);
    ai_loop_destroy(&sm);

    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_TURN_DONE);
    // Recorded here, before the verdict, because `judging` is where a completed
    // turn WAITS — the only state in the cycle a host reaches by sending
    // nothing. Every other branch of this walk records after driving the machine
    // on, and that is exactly how `judging` stayed unvisited while the floor
    // below read as satisfied.
    seen |= ai_loop_active_states(&sm);
    verdict(&sm, true);
    seen |= ai_loop_active_states(&sm);
    step(&sm, AI_LOOP_EVENT_TURN_DONE);
    seen |= ai_loop_active_states(&sm);
    ai_loop_destroy(&sm);

    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_TURN_BLOCKED);
    seen |= ai_loop_active_states(&sm);
    step(&sm, AI_LOOP_EVENT_SCREEN_NONE);
    seen |= ai_loop_active_states(&sm);
    step(&sm, AI_LOOP_EVENT_UNATTENDED);
    seen |= ai_loop_active_states(&sm);
    ai_loop_destroy(&sm);

    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_HOLD);
    seen |= ai_loop_active_states(&sm);
    step(&sm, AI_LOOP_EVENT_RESUME);
    seen |= ai_loop_active_states(&sm);
    ai_loop_destroy(&sm);

    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_SESSION_LOST);
    seen |= ai_loop_active_states(&sm);
    step(&sm, AI_LOOP_EVENT_SESSION_READY);
    seen |= ai_loop_active_states(&sm);
    ai_loop_destroy(&sm);

    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_CANCEL);
    seen |= ai_loop_active_states(&sm);
    ai_loop_destroy(&sm);

    start(&sm, &wiring);
    step(&sm, AI_LOOP_EVENT_FAIL);
    seen |= ai_loop_active_states(&sm);
    ai_loop_destroy(&sm);

    int bad = 0;
    int reached = 0;
    for (int s = 0; s < (int)AI_LOOP_STATE_COUNT; s++) {
        if ((seen & (uint32_t)(1u << (unsigned)s)) != 0u) {
            reached++;
        }
    }
    // A floor, not a target: without one, a table that had lost every arm but
    // the first would pass this by being asked about a single state.
    //
    // 21 is measured rather than chosen. The document declares 25 states and
    // the four below are unreachable to any reader of the configuration, so a
    // floor of 25 would retire this test and the 20 it used to hold understated
    // the walk by one.
    if (reached < 21) {
        bad |= fail_num("every_state_a_run_reaches_reads_back_from_its_own_name", "states these scenarios stood in",
                        (long long)reached, 21);
    }

    // The other side of that ratchet, and the reason the number above is a
    // measurement: these four are inner <final>s whose <onentry> is a <raise>
    // that ends the run in the SAME macrostep — `reported` raises
    // `run.converged`, `stuck` and `spent` raise `run.exhausted`, `abandoned`
    // raises `run.blocked` — so a configuration read taken between macrosteps
    // can never stand in one. Nothing else in the document is like that.
    //
    // Asserting their ABSENCE is what keeps 21 honest from above: make one of
    // them observable, or extend the walk to reach it, and this fails until the
    // floor is raised with it.
    static const char *const unobservable[] = {"abandoned", "reported", "spent", "stuck"};
    for (size_t i = 0; i < sizeof(unobservable) / sizeof(unobservable[0]); i++) {
        ai_loop_state_t state;
        if (!ai_loop_state_from_name(unobservable[i], &state)) {
            (void)fprintf(stderr,
                          "ai_loop: FAIL [every_state_a_run_reaches_reads_back_from_its_own_name] - "
                          "`%s` is a state this document declares\n",
                          unobservable[i]);
            bad |= 1;
            continue;
        }
        if ((seen & (uint32_t)(1u << (unsigned)state)) != 0u) {
            (void)fprintf(stderr,
                          "ai_loop: FAIL [every_state_a_run_reaches_reads_back_from_its_own_name] - "
                          "`%s` was reached, so the ceiling this test documents has moved: raise the floor\n",
                          unobservable[i]);
            bad |= 1;
        }
    }

    for (int s = 0; s < (int)AI_LOOP_STATE_COUNT; s++) {
        if ((seen & (uint32_t)(1u << (unsigned)s)) == 0u) {
            continue;
        }
        const char *name = ai_loop_state_name((ai_loop_state_t)s);
        ai_loop_state_t back;
        if (!ai_loop_state_from_name(name, &back) || back != (ai_loop_state_t)s) {
            (void)fprintf(stderr,
                          "ai_loop: FAIL [every_state_a_run_reaches_reads_back_from_its_own_name] - "
                          "`%s` did not read back as the state that published it\n",
                          name);
            bad |= 1;
        }
    }

    // The other half of the contract: a name the document does not carry is
    // refused rather than guessed at. A table that answers anyway turns a stale
    // journal into a plausible-looking resume, which is the one outcome a host
    // has no way to detect afterwards.
    ai_loop_state_t out;
    if (ai_loop_state_from_name("no-such-state", &out) || ai_loop_state_from_name("", &out)) {
        bad |= fail("every_state_a_run_reaches_reads_back_from_its_own_name",
                    "a name this document does not carry read back as a state");
    }
    if (ai_loop_state_from_name("turn.done", &out)) {
        bad |= fail("every_state_a_run_reaches_reads_back_from_its_own_name",
                    "an event name is not a state name; the two tables are separate on purpose");
    }
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= all_three_regions_are_live_at_once();
    bad |= reflection_fires_on_schedule();
    bad |= reflection_goes_through_a_restart_and_the_loop_re_primes();
    bad |= the_budget_ends_the_run_from_wherever_the_cycle_is();
    bad |= a_standing_instruction_answers_without_waking_anybody();
    bad |= an_unmatched_dialog_wakes_the_person_who_answers();
    bad |= answering_a_question_does_not_re_prime_the_session();
    bad |= hold_and_resume_return_to_exactly_where_the_cycle_was();
    bad |= resume_returns_somewhere_the_history_default_does_not();
    bad |= the_person_interrupts_the_inner_session_by_hand();
    bad |= nobody_comes();
    bad |= a_pane_that_dies_mid_turn_is_noticed_and_rebuilt();
    bad |= one_cancel_reaches_every_region();
    bad |= the_machine_answers_what_its_own_datamodel_holds();
    bad |= the_strategy_a_host_edits_is_the_strategy_it_can_read_back();
    bad |= the_standing_instructions_can_be_read_back_off_the_machine();
    bad |= a_structured_read_follows_the_assignment_and_refuses_another_type();
    bad |= what_a_reflection_writes_is_what_the_machine_then_holds();
    bad |= an_uninitialised_machine_says_it_cannot_answer();
    bad |= the_run_converges_through_a_closing_report();
    bad |= a_verdict_without_its_payload_is_reported();
    bad |= a_correctly_driven_run_reports_no_errors();
    bad |= a_session_replaced_past_its_budget_reports_stuck();
    bad |= the_document_declares_its_acts_to_the_host();
    bad |= a_failure_ends_the_whole_run();
    bad |= a_run_journalled_as_names_resumes_where_it_stopped();
    bad |= every_state_a_run_reaches_reads_back_from_its_own_name();

    if (bad != 0) {
        (void)fprintf(stderr, "ai_loop: FAIL - see the scenario(s) named above\n");
        return 1;
    }
    (void)printf("ai_loop: PASS - 27 scenarios\n");
    return 0;
}
