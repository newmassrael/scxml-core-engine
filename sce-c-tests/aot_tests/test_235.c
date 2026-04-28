// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test235 — C11 AOT runner.
//
// Fixture (resources/235/test235.txml):
//   <state id="s0">
//     <invoke type="http://www.w3.org/TR/scxml/" id="foo">
//       <content>...inline child reaching final immediately...</content>
//     </invoke>
//     <transition event="done.invoke.foo" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C 6.3.1 specific event: when the parent has a `done.invoke.<id>`
// transition and the spawned invoke's id matches, the platform raises
// the specific event variant rather than the generic `done.invoke`.
// The codegen-time `use_specific_event` flag is set in the parser by
// matching the explicit invoke id against the parent's event set, and
// `invoke_methods.jinja2` lowers the spawn-time done event to the
// specific enum (here `TEST235_EVENT_DONE_INVOKE_FOO`). The wildcard
// arm catches any drift — if the generic `done.invoke` were raised
// instead, the parent would route to fail because the specific
// transition would never fire and `event="*"` would catch it first.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test235_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test235_t sm;
    test235_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test235_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test235: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test235_active_states(&sm));
            test235_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test235_tick(&sm);
    }

    int rc = test235_in_state(&sm, TEST235_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test235: FAIL — active = 0x%08x\n",
                (unsigned)test235_active_states(&sm));
    }
    test235_destroy(&sm);
    return rc;
}
