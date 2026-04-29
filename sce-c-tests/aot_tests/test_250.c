// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test250 — C11 AOT runner.
//
// W3C SCXML 6.4: when an `<invoke>` is cancelled (parent exits the
// invoking state), the cancelled child MUST run its onexit handlers
// before the session terminates. The W3C corpus marks this manual
// because the child's onexit emits `<log>` lines that no automated
// harness inspects, and once cancelled the child cannot signal the
// parent (per W3C cancellation-drop semantics on `parent_dispatch`).
// The automated success criterion mirrors cpp `Test250.h` —
// reach the `final` state.
//
// Fixture flow (resources/250/test250.txml):
//   s0 onentry:
//     <send event="foo"/>          immediate, lands on external queue
//     <invoke type=...>            child schedules 2 s timeout
//   s0 transition event="foo" target="final"   cancels the invoke
//
// Reaches `final` immediately on the foo dequeue → cancel-path
// `destroy_active_children` (test237/252 already pin this) NULLs the
// child's `parent_dispatch` and frees the SM. Child's `<onexit>` log
// emits run on the C11 cancel path because `_finalize_session` is
// gated on `child_has_send_to_parent` — test250's child only logs
// (no <send target="#_parent"> in onexit), so the gate skips the
// finalize walk and the cancel path runs the per-state onexit chain
// directly via `_destroy`. Either way the parent reaches `final`,
// which is the conformance bit.
//
// The parent has no `<send delay>` of its own — the cancellation is
// driven by the immediate `<send event="foo"/>` macrostep, so the
// runner uses `_run` (drain to quiescence) rather than the polling
// `_tick` loop required by delayed-send fixtures.

#include <stdio.h>

#include "test250_sm.h"

int main(void) {
    test250_t sm;
    test250_init(&sm);
    test250_run(&sm);

    int rc = test250_in_state(&sm, TEST250_STATE_FINAL) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test250: FAIL — active = 0x%08x\n",
                (unsigned)test250_active_states(&sm));
    }
    test250_destroy(&sm);
    return rc;
}
