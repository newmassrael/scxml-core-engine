// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.3 + Appendix D: a state entered only because the target lies
// inside it takes no default child — C11 AOT.
//
// Appendix D asks two different questions with two functions.
// `addDescendantStatesToEnter` gives a compound state its default child and is
// called for the transition's TARGET; `addAncestorStatesToEnter` walks the
// states between the target and the LCCA and adds them WITHOUT defaults. Its
// one exception is a parallel ancestor, whose other regions do get theirs.
//
// Measured 2026-08-15 on the worked example `examples/ai_loop/ai_loop.scxml`,
// where the wrongly-entered state's `<onentry>` sends a prompt: the supervised
// session was re-sent its opening prompt every time a person answered a dialog.
//
// The document is driven twice on purpose. `cross` enters the `<parallel>`
// itself, so `run` is a parallel ancestor and `drive`/`outer` are compound
// ones; `again` runs with the parallel already active, so only `outer` is
// entered. Those are different branches of the generated entry walk.
//
// Fixture: integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(ancestor_entry_is_not_default_entry ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "ancestor_entry_is_not_default_entry_sm.h"

/// The counters, as the machine holds them (W3C SCXML 5.3 accessors).
///
/// The configuration cannot tell the failures apart on its own: a compound
/// state holds one child at a time, so a spurious entry has been REPLACED by
/// the target before anyone looks while its `<onentry>` has already run. What
/// survives is the effect, so the effect is what a failure prints.
static void report_counters(const ancestor_entry_is_not_default_entry_t *sm) {
    int64_t defaulted = -1, lobbied = -1, idled = -1, targeted = -1;
    (void)ancestor_entry_is_not_default_entry_defaulted(sm, &defaulted);
    (void)ancestor_entry_is_not_default_entry_lobbied(sm, &lobbied);
    (void)ancestor_entry_is_not_default_entry_idled(sm, &idled);
    (void)ancestor_entry_is_not_default_entry_targeted(sm, &targeted);
    fprintf(stderr,
            "       counters: defaulted=%lld lobbied=%lld idled=%lld targeted=%lld "
            "(wanted 0 / 1 / 1 / 2)\n",
            (long long)defaulted, (long long)lobbied, (long long)idled, (long long)targeted);
}

static void send(ancestor_entry_is_not_default_entry_t *sm, ancestor_entry_is_not_default_entry_event_t event) {
    ancestor_entry_is_not_default_entry_event_with_meta_t carrier = {0};
    carrier.event = event;
    ancestor_entry_is_not_default_entry_raise_external(sm, &carrier);
    ancestor_entry_is_not_default_entry_run(sm);
}

int main(void) {
    ancestor_entry_is_not_default_entry_t sm;
    ancestor_entry_is_not_default_entry_init(&sm);
    ancestor_entry_is_not_default_entry_run(&sm);

    if (!ancestor_entry_is_not_default_entry_in_state(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_AWAY)) {
        fprintf(stderr, "FAIL: the run has to start OUTSIDE the `<parallel>` for the first pass "
                        "to be testing anything — a source already inside it leaves the "
                        "ancestors active and the entry chain never reaches their defaults\n");
        return 1;
    }

    // Pass one: the parallel is not active, so `run` is entered as a parallel
    // ancestor and `drive` and `outer` as compound ones.
    send(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_EVENT_CROSS);

    if (!ancestor_entry_is_not_default_entry_in_state(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_CHOSEN)) {
        fprintf(stderr, "FAIL: the transition named `chosen` and the machine did not enter it\n");
        return 1;
    }
    // The symptom, named where it happens. The extra child is a SIBLING of the
    // target, so every ancestor test still holds and the current leaf is right.
    if (ancestor_entry_is_not_default_entry_in_state(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_BY_DEFAULT)) {
        fprintf(stderr, "FAIL: `outer` has two children active at once. `by_default` is what "
                        "`initial` names, and nothing targeted it — it was entered because the "
                        "engine gave `outer` its default child while entering `outer` merely as "
                        "an ancestor of `chosen`\n");
        return 1;
    }
    if (!ancestor_entry_is_not_default_entry_in_state(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_IDLE)) {
        fprintf(stderr, "FAIL: the region no entering state is inside must still be entered with "
                        "its default — Appendix D's one exception for a parallel ancestor\n");
        return 1;
    }

    // Pass two: the parallel is already active now, so `run` and `drive` are
    // skipped and only `outer` is entered. That is a different branch of the
    // entry walk, and it is the one a running machine takes.
    send(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_EVENT_BACK);
    send(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_EVENT_AGAIN);

    if (ancestor_entry_is_not_default_entry_in_state(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_BY_DEFAULT)) {
        fprintf(stderr, "FAIL: `outer` took its default child on the second pass, where the "
                        "`<parallel>` was already active and only `outer` itself was entered — "
                        "the shape the worked example hits every time a person answers a "
                        "dialog\n");
        return 1;
    }

    send(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_EVENT_CHECK);

    if (!ancestor_entry_is_not_default_entry_in_state(&sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_SETTLED)) {
        // The document checks its four clauses in document order and lands each
        // in a `<final>` of its own, so which one it stopped at names the defect.
        const char *stopped = "no final at all — `check` was not answered";
        if (ancestor_entry_is_not_default_entry_in_state(&sm,
                                                         ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_FAILDEFAULTED)) {
            stopped = "failDefaulted — a default child nobody targeted was entered";
        } else if (ancestor_entry_is_not_default_entry_in_state(
                       &sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_FAILLOBBIED)) {
            stopped = "failLobbied — `drive`'s default was taken while it was only an ancestor";
        } else if (ancestor_entry_is_not_default_entry_in_state(&sm,
                                                                ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_FAILIDLED)) {
            stopped = "failIdled — the untouched region did not get its default, or got it twice";
        } else if (ancestor_entry_is_not_default_entry_in_state(
                       &sm, ANCESTOR_ENTRY_IS_NOT_DEFAULT_ENTRY_STATE_FAILTARGETED)) {
            stopped = "failTargeted — a pass never reached the target";
        }
        fprintf(stderr, "FAIL: `check` did not carry the machine to `settled`. It stopped at %s\n", stopped);
        report_counters(&sm);
        return 1;
    }

    printf("PASS: an ancestor entered on the way to a target took no default child\n");
    return 0;
}
