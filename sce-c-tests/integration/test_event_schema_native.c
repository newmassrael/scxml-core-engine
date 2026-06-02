// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A (EventSchema MCU native lowering, RFC §10.4 step 5)
// — C11 runtime-parity gate for the watching-zenoh value path.
//
// Proves the C11 backend's native typed `_event.data` lowering end-to-end:
// a transition guarded on `_event.data.elapsed_ms === 0` is lowered to a
// tagged-union comparison against `pending_payload` with NO script engine
// (the Lua `lua_eval_guard` path is unreachable on an MCU). The driver
// injects a typed payload through the per-event `..._raise_job_completed_typed`
// seam — the entry the watching-zenoh consumer calls after decoding bytes,
// which binds the event name + payload tag + union member in one site — and
// asserts the guard fires for `elapsed_ms == 0` and misses for a non-zero
// value, the C11 twin of the Rust slice runtime test.
//
// MCU-clean acceptance: the harness target links NO runtime library
// (`sce_c_integration_event_schema_native` in sce-c-tests/CMakeLists.txt) —
// the datamodel-less native path is fully self-contained (only `sce/types.h`
// constants), so a successful link is itself the no-script-engine / no-Lua
// proof for the bare-metal target.
//
// Fixture: integration_resources/event_schema_native/event_schema_native.scxml
//          (+ schema_job_completed.scxml). Regeneration is automatic at
// CMake build time — the build is the §6.2.6 freshness invariant; there is
// no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "event_schema_native_sm.h"

int main(void) {
    // Positive: elapsed_ms == 0 satisfies the native guard, so the
    // job.completed transition fires waiting -> done.
    {
        event_schema_native_t sm;
        event_schema_native_init(&sm);
        if (!event_schema_native_in_state(&sm, EVENT_SCHEMA_NATIVE_STATE_WAITING)) {
            fprintf(stderr, "FAIL: initial state is not waiting\n");
            return 1;
        }

        event_schema_native_job_completed_payload_t payload = {0};
        payload.elapsed_ms = 0;
        event_schema_native_raise_job_completed_typed(&sm, &payload);
        event_schema_native_run(&sm);

        if (!event_schema_native_in_state(&sm, EVENT_SCHEMA_NATIVE_STATE_DONE)) {
            fprintf(stderr, "FAIL: elapsed_ms==0 should fire the guard to done\n");
            return 1;
        }
    }

    // Negative: elapsed_ms != 0 fails the guard, so the machine stays in
    // waiting (the same event name with a payload the guard rejects).
    {
        event_schema_native_t sm;
        event_schema_native_init(&sm);

        event_schema_native_job_completed_payload_t payload = {0};
        payload.elapsed_ms = 5;
        event_schema_native_raise_job_completed_typed(&sm, &payload);
        event_schema_native_run(&sm);

        if (!event_schema_native_in_state(&sm, EVENT_SCHEMA_NATIVE_STATE_WAITING)) {
            fprintf(stderr, "FAIL: elapsed_ms==5 should leave the machine in waiting\n");
            return 1;
        }
    }

    printf("PASS: C11 EventSchema native typed-payload guard\n");
    return 0;
}
