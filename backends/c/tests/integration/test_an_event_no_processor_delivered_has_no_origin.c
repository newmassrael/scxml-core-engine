// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 2): an event no Event I/O Processor
// delivered has no origin — a host-posted event and a host processor's reply
// that name none read both `_event.origin` and `_event.origintype` as the empty
// string — C11 AOT. This backend used to give every external event without an
// origin this session's own location and the SCXML processor's type; the
// invented origin is gone (section 3).
//
// Fixture:
// integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python / Swift channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(an_event_no_processor_delivered_has_no_origin ... HOST_PROCESSOR x-sce-host)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../require_started.h"
#include "an_event_no_processor_delivered_has_no_origin_sm.h"

typedef an_event_no_processor_delivered_has_no_origin_t sm_t;

// The type the build was told the host serves; the CMake registration passes
// this same string to `--host-processor`.
#define HOST_TYPE "x-sce-host"

// The host's processor answers the document's send with a reply that names no
// party, so the reply carries no origin.
static void replying_processor(void *user_data, const sce_host_send_request_t *request,
                               sce_host_send_response_list_t *out) {
    int *served = (int *)user_data;
    (void)request;
    ++*served;
    (void)sce_host_response_push(out, "host.reply", "");
}

static int check(const char *name, bool read, int64_t got) {
    if (read && got == 1) {
        return 1;
    }
    fprintf(stderr, "FAIL: %s = %lld%s, want 1: the event read an origin it was not given\n", name, (long long)got,
            read ? "" : " (unreadable)");
    return 0;
}

int main(void) {
    sm_t sm;
    int served = 0;
    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    if (!sce_host_registry_register(&wiring, HOST_TYPE, replying_processor, &served)) {
        fprintf(stderr, "FAIL: the host could not register its processor\n");
        return 1;
    }
    REQUIRE_STARTED(an_event_no_processor_delivered_has_no_origin_init_with_host_processors(&sm, &wiring),
                    AN_EVENT_NO_PROCESSOR_DELIVERED_HAS_NO_ORIGIN_START_OK);
    an_event_no_processor_delivered_has_no_origin_run(&sm);

    // The host raises an event of its own, naming no origin.
    an_event_no_processor_delivered_has_no_origin_event_with_meta_t posted;
    memset(&posted, 0, sizeof(posted));
    posted.event = AN_EVENT_NO_PROCESSOR_DELIVERED_HAS_NO_ORIGIN_EVENT_POSTED;
    an_event_no_processor_delivered_has_no_origin_raise_external(&sm, &posted);
    an_event_no_processor_delivered_has_no_origin_run(&sm);

    int ok = 1;
    if (served != 1) {
        fprintf(stderr, "FAIL: the host's processor was handed the document's send %d times, want 1\n", served);
        ok = 0;
    }
    if (!an_event_no_processor_delivered_has_no_origin_ended_in(
            &sm, AN_EVENT_NO_PROCESSOR_DELIVERED_HAS_NO_ORIGIN_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        ok = 0;
    }
    // Each value is read before it is judged: C leaves the order of a call's
    // arguments unspecified, so a reader and the value it writes cannot share one.
    int64_t posted_blank = -1;
    int64_t reply_blank = -1;
    const bool posted_read = an_event_no_processor_delivered_has_no_origin_posted_blank(&sm, &posted_blank);
    const bool reply_read = an_event_no_processor_delivered_has_no_origin_reply_blank(&sm, &reply_blank);
    ok &= check("postedBlank", posted_read, posted_blank);
    ok &= check("replyBlank", reply_read, reply_blank);
    an_event_no_processor_delivered_has_no_origin_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: an event no processor delivered had no origin\n");
    return 0;
}
