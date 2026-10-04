// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Child sessions")
// under generated C: an `<invoke type="scxml">` starts a child session, hands it
// the values of its `<param>`s and `namelist` before it enters its initial
// configuration, and the child's end reaches its parent as `done.invoke`.
//
// Three documents: the first two are the ones the Rust, Kotlin and Go suites
// drive, run live here — their restore half has no C counterpart, since a C
// machine is not saved — and the third is this suite's own:
//
//   * `static_invoke`        a child that takes `a` then `b`, which its parent
//                            forwards (autoforward); `abort` leaves the state that
//                            invokes it, which cancels the child and raises nothing.
//   * `static_invoke_params` children handed values once, when they start: one
//                            that ends only on the 7 and the true it is handed,
//                            one handed nothing that never ends, and one handed 7
//                            that waits for an 8 a later `bump` makes of the
//                            parent's field and does not reach it.
//   * `static_invoke_entry`  the values are in the child's variables when its
//                            `<onentry>` runs, a send to the parent from that
//                            `<onentry>` arrives, and a value that cannot be
//                            computed is left out and reported, the child still
//                            starting with the one its `<data>` gave it.
//
// Linked WITHOUT `sce_c_scripting` and `lua54`: the machines have no script
// engine, so the link is the proof.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_invoke_entry_sm.h"
#include "static_invoke_params_sm.h"
#include "static_invoke_sm.h"

// A machine's own door for an event named as a document names it: the event is
// raised on its external queue and the macrostep it starts is run.
#define STATIC_MACHINE_SEND(M)                                                                                         \
    static bool M##_send(M##_t *sm, const char *name) {                                                                \
        M##_event_t event;                                                                                             \
        if (!M##_resolve_event_by_name(name, &event)) {                                                                \
            return false;                                                                                              \
        }                                                                                                              \
        M##_event_with_meta_t meta;                                                                                    \
        memset(&meta, 0, sizeof(meta));                                                                                \
        meta.event = event;                                                                                            \
        M##_raise_external(sm, &meta);                                                                                 \
        M##_step(sm);                                                                                                  \
        return true;                                                                                                   \
    }

STATIC_MACHINE_SEND(static_invoke)
STATIC_MACHINE_SEND(static_invoke_params)

static int expect(bool holds, const char *what) {
    if (!holds) {
        (void)fprintf(stderr, "FAIL: %s\n", what);
        return 1;
    }
    return 0;
}

static int expect_count(uint32_t got, uint32_t want, const char *what) {
    if (got != want) {
        (void)fprintf(stderr, "FAIL: %s: %u, want %u\n", what, (unsigned)got, (unsigned)want);
        return 1;
    }
    return 0;
}

// `worker` takes `a` then `b` through its parent and ends: `done.invoke.worker`
// counts the run and leaves the parent in `working`, with no child running.
static int a_child_takes_what_its_parent_forwards(void) {
    static static_invoke_t sm;
    static_invoke_init(&sm);
    int bad = expect_count(static_invoke_get_completed(&sm), 0u, "before any event");
    bad |= expect(static_invoke_send(&sm, "a"), "`a` is an event of the machine");
    bad |= expect_count(static_invoke_get_completed(&sm), 0u, "after `a`, the child is at `second`");
    bad |= expect(static_invoke_send(&sm, "b"), "`b` is an event of the machine");
    bad |= expect_count(static_invoke_get_completed(&sm), 1u, "after `b`, the child has ended");
    bad |= expect(static_invoke_in_state(&sm, STATIC_INVOKE_STATE_WORKING), "the parent stays in `working`");
    (void)static_invoke_send(&sm, "finish");
    bad |= expect(static_invoke_ended_in(&sm, STATIC_INVOKE_STATE_FINISHED), "`finish` leaves for `finished`");
    static_invoke_destroy(&sm);
    return bad;
}

// The child is at its beginning, where `a` is the event it waits for: `b` alone
// leaves it there.
static int a_child_waits_for_its_first_event(void) {
    static static_invoke_t sm;
    static_invoke_init(&sm);
    (void)static_invoke_send(&sm, "b");
    int bad = expect_count(static_invoke_get_completed(&sm), 0u, "`b` before `a`");
    static_invoke_destroy(&sm);
    return bad;
}

// Leaving the state that invokes the child cancels it, and a cancelled child
// raises nothing (W3C SCXML 6.4): `b` later reaches no one.
static int leaving_the_state_cancels_the_child(void) {
    static static_invoke_t sm;
    static_invoke_init(&sm);
    (void)static_invoke_send(&sm, "a");
    (void)static_invoke_send(&sm, "abort");
    (void)static_invoke_send(&sm, "b");
    int bad = expect(static_invoke_in_state(&sm, STATIC_INVOKE_STATE_IDLE), "`abort` leaves for `idle`");
    bad |= expect_count(static_invoke_get_completed(&sm), 0u, "a cancelled child raises no `done.invoke`");
    static_invoke_destroy(&sm);
    return bad;
}

// The values the children are handed are the ones the parent holds when the
// invoke EXECUTES: `working`'s `<onentry>` adds 3 to `base`, so `worker` is
// handed 7, which it ends on with the true `namelist` hands it. `control`,
// handed nothing, keeps its 0 and false and never ends; `watcher` is handed 7
// and waits for 8.
static int the_children_are_handed_what_the_parent_holds_when_they_start(void) {
    static static_invoke_params_t sm;
    static_invoke_params_init(&sm);
    int bad = expect(static_invoke_params_in_state(&sm, STATIC_INVOKE_PARAMS_STATE_PLAIN),
                     "`worker` ended, so the parent moved to `plain`");
    bad |= expect_count(static_invoke_params_get_completed(&sm), 1u,
                        "`worker` alone has ended: `control` is handed nothing and `watcher` is handed 7");
    // A child is handed its values once. `base` is 8 now, and `watcher` waits
    // for 8, but it was handed 7 and is not handed again.
    (void)static_invoke_params_send(&sm, "bump");
    bad |= expect_count(static_invoke_params_get_completed(&sm), 1u,
                        "a field changed while `watcher` runs does not reach it");
    bad |= expect(static_invoke_params_in_state(&sm, STATIC_INVOKE_PARAMS_STATE_PLAIN), "the parent stays in `plain`");
    static_invoke_params_destroy(&sm);
    return bad;
}

// `first` raises `ready` from its `<onentry>` only when it reads the 8 it was
// handed, so it ends only if the value was there when it entered. `second` is
// handed a value that overflows: that value is left out, `error.execution`
// counts it, and `second` still starts, holding the 5 its `<data>` gave it,
// which is what its own `<onentry>` reads. `first` also sends its parent
// `hello` from that `<onentry>`, which reaches it only if the routing to the
// parent was in place before the entry walk.
static int the_values_are_there_when_the_child_enters(void) {
    static static_invoke_entry_t sm;
    static_invoke_entry_init(&sm);
    int bad = expect(static_invoke_entry_in_state(&sm, STATIC_INVOKE_ENTRY_STATE_OVERFLOWING),
                     "`first` ended, so the parent moved to `overflowing`");
    bad |= expect_count(static_invoke_entry_get_completed(&sm), 2u, "both children ended");
    bad |= expect_count(static_invoke_entry_get_errors(&sm), 1u, "the overflowing value raised `error.execution` once");
    bad |= expect_count(static_invoke_entry_get_greetings(&sm), 1u, "`first` greeted its parent from its `<onentry>`");
    static_invoke_entry_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_child_takes_what_its_parent_forwards();
    bad |= a_child_waits_for_its_first_event();
    bad |= leaving_the_state_cancels_the_child();
    bad |= the_children_are_handed_what_the_parent_holds_when_they_start();
    bad |= the_values_are_there_when_the_child_enters();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static invoke: ok\n");
    return 0;
}
