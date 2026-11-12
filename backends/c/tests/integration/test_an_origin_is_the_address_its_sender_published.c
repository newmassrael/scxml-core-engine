// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 + C.1 (docs/adr/0090, test 1): the origin of an event the
// SCXML Event I/O Processor delivers is the address its sender published, on a
// self-send, a child's send to its parent and a parent's send to its child, and
// a reply to it arrives — C11 AOT.
//
// The machine writes the pair where it builds the event (`sce_event_origin_scxml`,
// at the send sites and in the parent's dispatch shim), spelled by the one
// encoder its `_ioprocessors` entry is spelled by, and a child reaches its
// parent at the parent's published location as it reaches it at `#_parent`.
//
// Fixture:
// integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python / Swift channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(an_origin_is_the_address_its_sender_published ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "../require_started.h"
#include "an_origin_is_the_address_its_sender_published_sm.h"

typedef an_origin_is_the_address_its_sender_published_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    REQUIRE_STARTED(an_origin_is_the_address_its_sender_published_init(&sm),
                    AN_ORIGIN_IS_THE_ADDRESS_ITS_SENDER_PUBLISHED_START_OK);
    an_origin_is_the_address_its_sender_published_run(&sm);

    int ok = 1;
    if (!an_origin_is_the_address_its_sender_published_ended_in(
            &sm, AN_ORIGIN_IS_THE_ADDRESS_ITS_SENDER_PUBLISHED_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`; a reply sent to an origin that reached nobody leaves it "
                        "waiting\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"selfOk", an_origin_is_the_address_its_sender_published_self_ok, 1},
        {"selfReplied", an_origin_is_the_address_its_sender_published_self_replied, 1},
        {"childOk", an_origin_is_the_address_its_sender_published_child_ok, 1},
        {"childReplied", an_origin_is_the_address_its_sender_published_child_replied, 1},
        {"parentOk", an_origin_is_the_address_its_sender_published_parent_ok, 1},
        {"parentReplied", an_origin_is_the_address_its_sender_published_parent_replied, 1},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    an_origin_is_the_address_its_sender_published_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: every route's origin was the address its sender published\n");
    return 0;
}
