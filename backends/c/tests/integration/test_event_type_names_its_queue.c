// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1: `_event.type` names the queue an event was taken from —
// C11 AOT.
//
// The document queues an external event first and two internal ones after
// it. Measured 2026-09-26, this channel already seeded the type at each pop;
// the fixture pins it.
//
// Fixture: integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(event_type_names_its_queue ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "event_type_names_its_queue_sm.h"

typedef event_type_names_its_queue_t sm_t;

/// What the handlers recorded (W3C SCXML 5.3 accessors).
static void report(const sm_t *sm) {
    int64_t int_code = -1, send_code = -1, ext_code = -1;
    (void)event_type_names_its_queue_int_code(sm, &int_code);
    (void)event_type_names_its_queue_send_code(sm, &send_code);
    (void)event_type_names_its_queue_ext_code(sm, &ext_code);
    fprintf(stderr, "       intCode=%lld sendCode=%lld extCode=%lld (1 internal, 2 external; wanted 1 / 1 / 2)\n",
            (long long)int_code, (long long)send_code, (long long)ext_code);
}

static int expect_record(const sm_t *sm, const char *name, bool (*reader)(const sm_t *, int64_t *), int64_t want,
                         const char *why) {
    int64_t value = -1;
    if (!reader(sm, &value) || value != want) {
        fprintf(stderr, "FAIL: %s = %lld, want %lld: %s\n", name, (long long)value, (long long)want, why);
        report(sm);
        return 0;
    }
    return 1;
}

int main(void) {
    sm_t sm;
    event_type_names_its_queue_init(&sm);
    // The document queues its own events; the run needs nothing from the host.
    event_type_names_its_queue_run(&sm);

    int ok = 1;
    if (!event_type_names_its_queue_ended_in(&sm, EVENT_TYPE_NAMES_ITS_QUEUE_STATE_DONE)) {
        fprintf(stderr, "FAIL: `ext` must carry the run to `done`\n");
        report(&sm);
        ok = 0;
    }
    ok &= expect_record(&sm, "intCode", event_type_names_its_queue_int_code, 1,
                        "`int` came off the internal queue while `ext` waited on the external one");
    ok &= expect_record(&sm, "sendCode", event_type_names_its_queue_send_code, 1,
                        "a `<send target=\"#_internal\">` with a payload rides the internal queue too");
    ok &= expect_record(&sm, "extCode", event_type_names_its_queue_ext_code, 2, "`ext` came off the external queue");
    event_type_names_its_queue_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: every event was typed by the queue it was taken from\n");
    return 0;
}
