// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + 6.2: a <send>'s payload is the data of the event it sends,
// and a data-less event dequeued before it carries none — C11 AOT.
//
// Measured 2026-09-27, this channel staged the payload of an external
// <param>, namelist or <content> send in one Lua global that the next
// dequeue promoted, so each <raise> behind such a send was handed its data
// and the namelist and <content> sends arrived with none.
//
// Fixture:
// integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_payload_rides_on_its_own_event ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_payload_rides_on_its_own_event_sm.h"

typedef a_payload_rides_on_its_own_event_t sm_t;

static void report(const sm_t *sm) {
    int64_t got = -1, stolen = -1, plains = -1;
    (void)a_payload_rides_on_its_own_event_got(sm, &got);
    (void)a_payload_rides_on_its_own_event_stolen(sm, &stolen);
    (void)a_payload_rides_on_its_own_event_plains(sm, &plains);
    fprintf(stderr, "       got=%lld stolen=%lld plains=%lld (wanted 4 / 0 / 4)\n", (long long)got, (long long)stolen,
            (long long)plains);
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
    a_payload_rides_on_its_own_event_init(&sm);
    a_payload_rides_on_its_own_event_run(&sm);

    a_payload_rides_on_its_own_event_event_with_meta_t carrier = {0};
    carrier.event = A_PAYLOAD_RIDES_ON_ITS_OWN_EVENT_EVENT_FINISH;
    a_payload_rides_on_its_own_event_raise_external(&sm, &carrier);
    a_payload_rides_on_its_own_event_run(&sm);

    int ok = 1;
    if (!a_payload_rides_on_its_own_event_ended_in(&sm, A_PAYLOAD_RIDES_ON_ITS_OWN_EVENT_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        report(&sm);
        ok = 0;
    }
    ok &= expect_record(&sm, "got", a_payload_rides_on_its_own_event_got, 4,
                        "each payload event arrives carrying its own payload");
    ok &= expect_record(&sm, "stolen", a_payload_rides_on_its_own_event_stolen, 0,
                        "no data-less event arrives carrying a payload");
    ok &= expect_record(&sm, "plains", a_payload_rides_on_its_own_event_plains, 4,
                        "every data-less event arrives, and arrives empty");
    a_payload_rides_on_its_own_event_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: every payload arrived on its own event and no other\n");
    return 0;
}
