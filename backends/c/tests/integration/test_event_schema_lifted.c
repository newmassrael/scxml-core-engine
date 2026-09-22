// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A — the OTHER carrier, the C11 twin of the Rust
// `event_schema_native.rs` lifted cases, the Go `event_schema_lifted` package,
// the Kotlin `EventSchemaLiftedTest`, the Python `test_event_schema_lifted.py`
// and the C++ `EventSchemaLiftedAotTest`.
//
// A schema'd event's typed payload is filled by ONE producer: the generated
// `statechart_lifted_raise_job_completed_typed` seam. Every other producer —
// `<send>` with `<param>`, an invoke forwarding an event either way,
// autoforward, BasicHTTP, mesh — fills the event record's `data`, and until
// 2026-09-22 a natively lowered guard could read nothing but the typed union.
// The same guard therefore answered differently depending on where its event
// came from.
//
// What a refusal does is not a policy chosen here: it is what the SCRIPT
// ENGINE answers for the same guard on the same data (W3C SCXML 3.13, measured
// on this document). A native lowering that answered differently would make
// the optimisation observable, which is the one thing it may not be.
//
// MCU-clean acceptance, unchanged by the lift: this harness links NO runtime
// library. `sce/event_payload.h` is header-only for exactly that reason — a
// lift in a library would have taken that proof away from every machine that
// uses it.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "statechart_lifted_sm.h"

static int failures = 0;

static void expect_state(const char *what, const statechart_lifted_t *sm, statechart_lifted_state_t want) {
    if (!statechart_lifted_in_state(sm, want)) {
        fprintf(stderr, "FAIL: %s (want state %d)\n", what, (int)want);
        failures++;
    }
}

/* The event as every producer but the inject seam delivers it: its fields on
   the `data` wire, with no typed payload riding along. */
static void deliver_data(statechart_lifted_t *sm, const char *data) {
    statechart_lifted_event_with_meta_t evt;
    memset(&evt, 0, sizeof(evt));
    evt.event = STATECHART_LIFTED_EVENT_JOB_COMPLETED;
    snprintf(evt.data, sizeof(evt.data), "%s", data);
    statechart_lifted_raise_external(sm, &evt);
    statechart_lifted_run(sm);
}

int main(void) {
    /* A payload that arrived on the `data` wire must satisfy the same native
       guard the inject seam's typed payload does. */
    {
        statechart_lifted_t sm;
        statechart_lifted_init(&sm);
        expect_state("initial state is waiting", &sm, STATECHART_LIFTED_STATE_WAITING);
        deliver_data(&sm, "{\"elapsed_ms\":0}");
        expect_state("a payload on the data wire fires the same guard", &sm, STATECHART_LIFTED_STATE_DONE);
    }

    /* The seam the MCU consumer calls after decoding its bytes still fires the
       guard it was built for. */
    {
        statechart_lifted_t sm;
        statechart_lifted_init(&sm);
        statechart_lifted_job_completed_payload_t payload = {0};
        payload.elapsed_ms = 0;
        statechart_lifted_raise_job_completed_typed(&sm, &payload);
        statechart_lifted_run(&sm);
        expect_state("the inject seam still fires its own guard", &sm, STATECHART_LIFTED_STATE_DONE);
    }

    /* A text where the schema declares a number: error.execution, and the
       guard does not fire — what the script engine answers for this guard. */
    {
        statechart_lifted_t sm;
        statechart_lifted_init(&sm);
        deliver_data(&sm, "{\"elapsed_ms\":\"nought\"}");
        expect_state("a value of another type is refused", &sm, STATECHART_LIFTED_STATE_REFUSED);
    }

    /* An event carrying no data cannot answer a guard that reads a field of
       it. */
    {
        statechart_lifted_t sm;
        statechart_lifted_init(&sm);
        deliver_data(&sm, "");
        expect_state("an event with no data is refused the same way", &sm, STATECHART_LIFTED_STATE_REFUSED);
    }

    /* Data that names none of the schema's fields cannot answer the guard. */
    {
        statechart_lifted_t sm;
        statechart_lifted_init(&sm);
        deliver_data(&sm, "{\"other\":0}");
        expect_state("a field the data does not name is refused", &sm, STATECHART_LIFTED_STATE_REFUSED);
    }

    /* A well-typed payload the guard rejects is not an error: nothing failed,
       so nothing is raised and the machine waits. */
    {
        statechart_lifted_t sm;
        statechart_lifted_init(&sm);
        deliver_data(&sm, "{\"elapsed_ms\":5}");
        expect_state("a payload the guard rejects is not an error", &sm, STATECHART_LIFTED_STATE_WAITING);
    }

    if (failures != 0) {
        fprintf(stderr, "FAILED: %d case(s)\n", failures);
        return 1;
    }
    printf("PASS: C11 EventSchema typed payload lifted from the data wire\n");
    return 0;
}
