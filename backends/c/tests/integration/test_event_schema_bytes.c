// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RFC rfc-eventschema-bytes-guard.md §6 — C11 runtime-parity gate for the
// bytes-field EventSchema guard, the twin of the Rust / Python / Go bytes
// integration tests and the numeric `test_event_schema_native.c`.
//
// A transition guarded on `_event.data.raw === 'ack'` is lowered to a
// tagged-union comparison `raw_len == 3 && memcmp(raw, "ack", 3) == 0`
// against `pending_payload`, with NO script engine. The driver fills the
// no-alloc bounded-buffer payload (`uint8_t raw[CAP]; size_t raw_len;`) and
// injects it through the per-event `..._raise_signal_received_typed` seam —
// the entry the downstream consumer calls after decoding bytes — then
// asserts the guard fires for "ack" and misses for "no".
//
// MCU-clean acceptance: the harness target links NO runtime library
// (`sce_c_integration_event_schema_bytes` in backends/c/tests/CMakeLists.txt) —
// the bounded-buffer + memcmp value path is self-contained (only `sce/types.h`
// constants + freestanding `memcmp`), so a successful link is itself the
// no-script-engine / no-Lua proof for the bare-metal target.
//
// Fixture: integration_resources/event_schema_bytes/event_schema_bytes.scxml
//          (+ schema_signal_bytes.scxml). Regenerated at CMake build time —
// the build is the §6.2.6 freshness invariant; no committed tree for c11.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "event_schema_bytes_sm.h"

int main(void) {
    // Positive: raw == "ack" satisfies the native bytes guard, so the
    // signal.received transition fires waiting -> done.
    {
        event_schema_bytes_t sm;
        event_schema_bytes_init(&sm);
        if (!event_schema_bytes_in_state(&sm, EVENT_SCHEMA_BYTES_STATE_WAITING)) {
            fprintf(stderr, "FAIL: initial state is not waiting\n");
            return 1;
        }

        event_schema_bytes_signal_received_payload_t payload = {0};
        memcpy(payload.raw, "ack", 3);
        payload.raw_len = 3;
        event_schema_bytes_raise_signal_received_typed(&sm, &payload);
        event_schema_bytes_run(&sm);

        if (!event_schema_bytes_in_state(&sm, EVENT_SCHEMA_BYTES_STATE_DONE)) {
            fprintf(stderr, "FAIL: raw==\"ack\" should fire the bytes guard to done\n");
            return 1;
        }
    }

    // Negative: raw == "no" fails the guard, so the machine stays in
    // waiting (the same event name with a payload the guard rejects).
    {
        event_schema_bytes_t sm;
        event_schema_bytes_init(&sm);

        event_schema_bytes_signal_received_payload_t payload = {0};
        memcpy(payload.raw, "no", 2);
        payload.raw_len = 2;
        event_schema_bytes_raise_signal_received_typed(&sm, &payload);
        event_schema_bytes_run(&sm);

        if (!event_schema_bytes_in_state(&sm, EVENT_SCHEMA_BYTES_STATE_WAITING)) {
            fprintf(stderr, "FAIL: raw==\"no\" should leave the machine in waiting\n");
            return 1;
        }
    }

    printf("PASS: C11 EventSchema native bytes typed-payload guard\n");
    return 0;
}
