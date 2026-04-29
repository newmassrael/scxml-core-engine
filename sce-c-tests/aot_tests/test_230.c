// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test230 — C11 AOT runner.
//
// W3C SCXML 6.4: `<invoke autoforward="true">` requires the parent to
// forward an exact copy of every external event to the active child,
// preserving every `_event` field (name/type/sendid/origin/origintype/
// invokeid/data). The W3C corpus marks this manual because the field
// equality is observed via two side-by-side `<log>` blocks in parent
// and child for human inspection. The automated success criterion
// mirrors cpp `Test230.h` — reach the `final` state (any divergence
// routes to `fail`).
//
// Fixture flow (resources/230/test230.txml):
//   parent s0/s01 onentry: <send event=timeout delay=3s>
//   child sub0 onentry: <send target=#_parent event=childToParent> +
//                       <send event=timeout delay=2s> (intra-child)
//   parent s01 catches childToParent → s02 (logs all _event fields)
//   parent's autoforwarder ships childToParent back to the child;
//   child sub0 catches childToParent → subFinal (logs all _event
//   fields), child raises done.invoke; parent s02 catches done.invoke
//   → final.  3 s `timeout` is the safety net.
//
// Reaches `final` only if (a) the child's onentry-send arrives at the
// parent (test191 init-with-parent ordering), (b) the parent forwards
// it back (test229 forward_to_autoforward_children), and (c) the
// child's done.invoke reaches the parent before the wildcard catches
// any divergent event. Any of these failing routes the parent into
// `fail` via `<transition event="*"/>`.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test230_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test230_t sm;
    test230_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test230_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test230: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test230_active_states(&sm));
            test230_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test230_tick(&sm);
    }

    int rc = test230_in_state(&sm, TEST230_STATE_FINAL) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test230: FAIL — active = 0x%08x\n",
                (unsigned)test230_active_states(&sm));
    }
    test230_destroy(&sm);
    return rc;
}
