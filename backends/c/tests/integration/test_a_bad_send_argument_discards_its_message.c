// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 4.9: a <send> whose own argument cannot be evaluated raises
// error.execution, discards the message, and ends its block — C11 AOT.
//
// The machine's clock is advanced a full minute before `finish`: a channel that
// scheduled the message whose `delayexpr` failed, under some default delay,
// delivers it within that minute and moves `sent`.
//
// Fixture:
// integration_resources/a_bad_send_argument_discards_its_message/a_bad_send_argument_discards_its_message.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_bad_send_argument_discards_its_message ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_bad_send_argument_discards_its_message_sm.h"

typedef a_bad_send_argument_discards_its_message_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    a_bad_send_argument_discards_its_message_init(&sm);
    a_bad_send_argument_discards_its_message_run(&sm);
    a_bad_send_argument_discards_its_message_advance_time_ms(&sm, 60000u);

    a_bad_send_argument_discards_its_message_event_with_meta_t carrier = {0};
    carrier.event = A_BAD_SEND_ARGUMENT_DISCARDS_ITS_MESSAGE_EVENT_FINISH;
    a_bad_send_argument_discards_its_message_raise_external(&sm, &carrier);
    a_bad_send_argument_discards_its_message_run(&sm);

    int ok = 1;
    if (!a_bad_send_argument_discards_its_message_ended_in(&sm, A_BAD_SEND_ARGUMENT_DISCARDS_ITS_MESSAGE_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"errors", a_bad_send_argument_discards_its_message_errors, 6},
        {"sent", a_bad_send_argument_discards_its_message_sent, 0},
        {"after", a_bad_send_argument_discards_its_message_after, 0},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_bad_send_argument_discards_its_message_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each bad send argument discarded its message\n");
    return 0;
}
