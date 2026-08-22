// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: a payload the datamodel could not read arrives as a
// space-normalized string, and the host that built it can find out - C11 AOT.
//
// The clause gives a payload three readings and names the third "otherwise".
// That word is where a belief leaves the system quietly. A host serializes
// `{"done":true}`, something truncates it to `{"done":`, and the clause is
// satisfied: the content becomes a string. The document then evaluates
// `_event.data.done`, finds nothing, and takes the transition it would have
// taken had the host sent a payload with no `done` field at all. Nothing is
// raised - the fallback is CORRECT behaviour, not an error - so before this
// fixture nothing anywhere said it had happened.
//
// This backend is where it matters most. An MCU consumer hands the payload in
// from outside the document and has no console to notice a quiet nil on, and
// this engine was the last of the six to route a host payload through a decoder
// at all (measured 2026-08-16, before which a JSON payload was pasted into Lua
// source and `_event.data` silently kept the PREVIOUS event's value).
//
// Fixture: integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(undecodable_payload_is_reported ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "undecodable_payload_is_reported_sm.h"

// Content that announces an object and stops. The shape a truncated write, a
// half-flushed buffer or a serializer that died mid-record actually produces.
#define TRUNCATED_OBJECT "{\"done\":"
// The same failure announced with `[`, under the other event name, so a channel
// that reports "the last event" rather than "the last event that lost a
// payload" cannot pass by accident.
#define TRUNCATED_ARRAY "[1,2"
// W3C test 562 sends exactly this shape and requires it to arrive as a string.
// Counting it would make the statistic fire on documents that are working.
#define PROSE "just a sentence"
// What the host meant to send.
#define INTACT_OBJECT "{\"done\":true}"

static void deliver(undecodable_payload_is_reported_t *sm, undecodable_payload_is_reported_event_t event,
                    const char *payload) {
    undecodable_payload_is_reported_event_with_meta_t carrier = {0};
    carrier.event = event;
    // The host's own bounded copy, exactly as an embedder writes it.
    size_t len = strlen(payload);
    if (len >= sizeof(carrier.data)) {
        len = sizeof(carrier.data) - 1u;
    }
    memcpy(carrier.data, payload, len);
    carrier.data[len] = '\0';
    undecodable_payload_is_reported_raise_external(sm, &carrier);
    undecodable_payload_is_reported_step(sm);
}

int main(void) {
    int rc = 0;

    // The axis: content that asked for the structured reading and did not get
    // it is counted.
    {
        undecodable_payload_is_reported_t sm;
        undecodable_payload_is_reported_init(&sm);

        if (undecodable_payload_is_reported_undecodable_payloads(&sm) != 0u) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - something was counted "
                            "before the first event was delivered.\n");
            rc = 1;
        }

        // The lower bound, and the half a check like this usually forgets: an
        // accessor that always answers reads exactly like a working one from
        // the other side. The sentinel is `_EVENT_NOTE` rather than the enum's
        // zero value, so a `memset`-clean struct cannot hand back something
        // that looks deliberate.
        undecodable_payload_is_reported_event_t untouched = UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_NOTE;
        if (undecodable_payload_is_reported_last_undecodable_payload(&sm, &untouched)) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the machine named a lost "
                            "payload before anything had been delivered.\n");
            rc = 1;
        }
        if (untouched != UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_NOTE) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the accessor wrote to its "
                            "out-parameter while reporting that it had nothing to say.\n");
            rc = 1;
        }

        deliver(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER, TRUNCATED_OBJECT);

        int64_t answers = -1;
        (void)undecodable_payload_is_reported_answers(&sm, &answers);
        if (answers != 1) {
            fprintf(stderr,
                    "undecodable_payload_is_reported: FAIL - the `answer` transition did "
                    "not run (answers=%lld), so nothing here is measuring a delivery that "
                    "reached the document.\n",
                    (long long)answers);
            rc = 1;
        }
        if (undecodable_payload_is_reported_undecodable_payloads(&sm) != 1u) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the host sent `" TRUNCATED_OBJECT
                            "`, which announces an object and does not "
                            "parse as one. W3C SCXML B.2.8.1 correctly delivers it as a "
                            "string; the host that built it has no other way to learn its "
                            "payload stopped being structure.\n");
            rc = 1;
        }
        if (!undecodable_payload_is_reported_in_state(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_STATE_WAITING)) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the reading a payload "
                            "got changed which transition fired.\n");
            rc = 1;
        }

        undecodable_payload_is_reported_event_t named = UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_NOTE;
        if (!undecodable_payload_is_reported_last_undecodable_payload(&sm, &named) ||
            named != UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the machine counted a "
                            "lost payload but cannot say which delivery lost it.\n");
            rc = 1;
        }

        undecodable_payload_is_reported_destroy(&sm);
    }

    // The other half. A count that also counts success cannot be used to detect
    // failure, and the reading the clause calls "otherwise" is the NORMAL
    // outcome for a document whose author wrote prose.
    {
        undecodable_payload_is_reported_t sm;
        undecodable_payload_is_reported_init(&sm);

        deliver(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_NOTE, PROSE);
        int64_t notes = -1;
        (void)undecodable_payload_is_reported_notes(&sm, &notes);
        if (notes != 1) {
            fprintf(stderr,
                    "undecodable_payload_is_reported: FAIL - the `note` transition did not "
                    "run (notes=%lld).\n",
                    (long long)notes);
            rc = 1;
        }
        if (undecodable_payload_is_reported_undecodable_payloads(&sm) != 0u) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - `" PROSE "` is the "
                            "third reading working as W3C SCXML B.2.8.1 specifies and as "
                            "W3C test 562 requires. A diagnostic that fires when nothing "
                            "is wrong is one nobody reads.\n");
            rc = 1;
        }

        deliver(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER, INTACT_OBJECT);
        if (!undecodable_payload_is_reported_in_state(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_STATE_ACCEPTED)) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the guard "
                            "`_event.data.done` did not hold for `" INTACT_OBJECT "`, so "
                            "the structured reading did not happen and the check below "
                            "would be proving nothing.\n");
            rc = 1;
        }
        if (undecodable_payload_is_reported_undecodable_payloads(&sm) != 0u) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - a payload that parsed "
                            "was counted as one that did not.\n");
            rc = 1;
        }

        undecodable_payload_is_reported_destroy(&sm);
    }

    // Why the query has to exist: a lost payload and an absent field answer the
    // same through every accessor a host had.
    {
        undecodable_payload_is_reported_t broken;
        undecodable_payload_is_reported_init(&broken);
        deliver(&broken, UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER, TRUNCATED_OBJECT);

        undecodable_payload_is_reported_t intact;
        undecodable_payload_is_reported_init(&intact);
        // Valid JSON, and `done` is genuinely absent - the innocent explanation
        // an operator has to rule out.
        deliver(&intact, UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER, "{\"ready\":true}");

        int64_t broken_answers = -1;
        int64_t intact_answers = -2;
        (void)undecodable_payload_is_reported_answers(&broken, &broken_answers);
        (void)undecodable_payload_is_reported_answers(&intact, &intact_answers);

        if (undecodable_payload_is_reported_in_state(&broken, UNDECODABLE_PAYLOAD_IS_REPORTED_STATE_WAITING) !=
                undecodable_payload_is_reported_in_state(&intact, UNDECODABLE_PAYLOAD_IS_REPORTED_STATE_WAITING) ||
            broken_answers != intact_answers) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - this fixture exists "
                            "because a lost payload and an absent field are "
                            "indistinguishable through the accessors a host had; they "
                            "differ, so the fixture stopped measuring what it claims.\n");
            rc = 1;
        }

        if (undecodable_payload_is_reported_undecodable_payloads(&broken) != 1u ||
            undecodable_payload_is_reported_undecodable_payloads(&intact) != 0u) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the two runs agree on "
                            "everything else, so this count is the only thing that "
                            "separates a broken sender from a working one.\n");
            rc = 1;
        }

        undecodable_payload_is_reported_destroy(&intact);
        undecodable_payload_is_reported_destroy(&broken);
    }

    // A count says a payload was lost; a host debugging a stalled supervisor
    // needs to know which delivery lost it - and a delivery that succeeds must
    // move neither record.
    {
        undecodable_payload_is_reported_t sm;
        undecodable_payload_is_reported_init(&sm);

        deliver(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER, TRUNCATED_OBJECT);
        deliver(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_NOTE, TRUNCATED_ARRAY);

        if (undecodable_payload_is_reported_undecodable_payloads(&sm) != 2u) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the count is a count, "
                            "not a flag.\n");
            rc = 1;
        }
        undecodable_payload_is_reported_event_t named = UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER;
        if (!undecodable_payload_is_reported_last_undecodable_payload(&sm, &named) ||
            named != UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_NOTE) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the second loss arrived "
                            "under `note` and the machine still names the first.\n");
            rc = 1;
        }

        deliver(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER, INTACT_OBJECT);
        if (!undecodable_payload_is_reported_in_state(&sm, UNDECODABLE_PAYLOAD_IS_REPORTED_STATE_ACCEPTED)) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - the intact payload did "
                            "not take the guarded transition, so the two checks below are "
                            "not measuring a successful delivery.\n");
            rc = 1;
        }
        if (undecodable_payload_is_reported_undecodable_payloads(&sm) != 2u) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - a delivery that parsed "
                            "moved a count that belongs to ones that did not.\n");
            rc = 1;
        }
        named = UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_ANSWER;
        if (!undecodable_payload_is_reported_last_undecodable_payload(&sm, &named) ||
            named != UNDECODABLE_PAYLOAD_IS_REPORTED_EVENT_NOTE) {
            fprintf(stderr, "undecodable_payload_is_reported: FAIL - a delivery that parsed "
                            "moved a name that belongs to one that did not.\n");
            rc = 1;
        }

        undecodable_payload_is_reported_destroy(&sm);
    }

    if (rc == 0) {
        printf("undecodable_payload_is_reported: PASS\n");
    }
    return rc;
}
