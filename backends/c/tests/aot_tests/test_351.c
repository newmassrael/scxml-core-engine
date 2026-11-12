// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test351 — C11 AOT runner.
//
// W3C SCXML 5.10.1 (test351): the `<send>` static `id="..."` attribute MUST
// be propagated as `_event.sendid` to the receiver. The bare-external arm at
// state_machine.c.jinja2:334 now emits a strncpy when `action.id` is
// non-empty, mirroring cpp `EventWithMetadata` constructor sendId carry —
// when the author omitted `id`, the zero-initialised `send_id` field stays
// empty and `_event.sendid` reads as nil/undefined per the same spec clause's
// "MUST leave this attribute empty" rule. parser.rs:1153 stores the literal
// `id` attribute verbatim, so the strncpy operand is the exact string the
// author wrote. The s2 send (no id) and the s0 send (id="send1") share the
// same arm, gated by the `{% if action.id %}` jinja2 conditional.
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST351_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test351_sm.h"

int main(void) {
    test351_t sm;
    REQUIRE_STARTED(test351_init(&sm), TEST351_START_OK);
    test351_run(&sm);

    int rc = test351_ended_in(&sm, TEST351_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test351: FAIL — active = 0x%08x\n", (unsigned)test351_active_states(&sm));
    }
    test351_destroy(&sm);
    return rc;
}
