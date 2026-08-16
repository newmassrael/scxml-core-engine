// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + B.2: a payload a HOST injects reaches the datamodel as a
// value — C11 AOT.
//
// The edge nothing measured. Every other integration fixture leaves the
// carrier's `data` field at its zero value — measured 2026-08-16, on every
// channel — so the host-to-datamodel boundary was covered by no test at all.
// The W3C suite does not reach it either: its payloads originate inside the
// document (`<send><content>`, `<param>`, `<donedata>`), which this backend
// lowers on a separate path from the one an embedder calls.
//
// It is the edge an embedder actually uses. `watching-zenoh` ships this
// backend on an MCU and hands it payloads from the wire; `examples/ai_loop`
// answers its machine with `{"done":true}` and selects on `_event.data.done`.
//
// Fixture: integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(event_data_arrives_as_sent ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>
#include <string.h>

#include "event_data_arrives_as_sent_sm.h"

static void send_with_payload(event_data_arrives_as_sent_t *sm, event_data_arrives_as_sent_event_t event,
                              const char *payload) {
    event_data_arrives_as_sent_event_with_meta_t carrier = {0};
    carrier.event = event;
    // The host's own bounded copy, exactly as an embedder writes it.
    size_t len = strlen(payload);
    if (len >= sizeof(carrier.data)) {
        len = sizeof(carrier.data) - 1u;
    }
    memcpy(carrier.data, payload, len);
    carrier.data[len] = '\0';
    event_data_arrives_as_sent_raise_external(sm, &carrier);
    event_data_arrives_as_sent_run(sm);
}

int main(void) {
    event_data_arrives_as_sent_t sm;
    event_data_arrives_as_sent_init(&sm);
    event_data_arrives_as_sent_run(&sm);

    if (!event_data_arrives_as_sent_in_state(&sm, EVENT_DATA_ARRIVES_AS_SENT_STATE_WAITING)) {
        fprintf(stderr, "FAIL: the fixture is supposed to start in `waiting`; it did not, so "
                        "nothing below is testing what it claims\n");
        return 1;
    }

    // A JSON object, the shape an embedder has when it holds structured data
    // and a state machine to give it to.
    send_with_payload(&sm, EVENT_DATA_ARRIVES_AS_SENT_EVENT_PAYLOAD, "{\"milestone\":\"refined\",\"turns\":2}");

    if (event_data_arrives_as_sent_in_state(&sm, EVENT_DATA_ARRIVES_AS_SENT_STATE_MANGLED)) {
        fprintf(stderr, "FAIL: the host sent {\"milestone\":\"refined\",\"turns\":2} and the guard "
                        "`_event.data.milestone === 'refined' && _event.data.turns === 2` did not hold, "
                        "so the payload did not arrive as an object with those properties\n");
        return 1;
    }
    if (!event_data_arrives_as_sent_in_state(&sm, EVENT_DATA_ARRIVES_AS_SENT_STATE_HEARD)) {
        fprintf(stderr, "FAIL: the payload guard neither matched nor mismatched — the machine is "
                        "not in `heard`\n");
        return 1;
    }

    // Text that is not JSON. The same call, and it must NOT be parsed into
    // something else: `hold the line` is the value the document compares
    // against, character for character.
    send_with_payload(&sm, EVENT_DATA_ARRIVES_AS_SENT_EVENT_NOTE, "hold the line");

    if (!event_data_arrives_as_sent_in_state(&sm, EVENT_DATA_ARRIVES_AS_SENT_STATE_SETTLED)) {
        fprintf(stderr,
                "FAIL: the host sent the text `hold the line` and `_event.data === 'hold the "
                "line'` did not hold, so a payload that is not JSON did not arrive as the string "
                "it was sent as. Still inside: garbled=%d heard=%d\n",
                event_data_arrives_as_sent_in_state(&sm, EVENT_DATA_ARRIVES_AS_SENT_STATE_GARBLED),
                event_data_arrives_as_sent_in_state(&sm, EVENT_DATA_ARRIVES_AS_SENT_STATE_HEARD));
        return 1;
    }

    printf("PASS: a host payload reached the datamodel as an object, and as text\n");
    return 0;
}
