// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.5 + docs/adr/0090, sections 1 and 6: `<finalize>` runs for an
// event the invoked child sent and never for one the host posted, even when
// the host spells the child's own SCXML pair as its origin — C11 AOT.
//
// This backend reads an event's sender back out of its pair rather than storing
// it, so it is the provenance mark (`from_session`, set only where the SCXML
// processor stamps the pair) that tells a child's send from a host's event
// naming the child. Without it the host's `poke` would run the child's finalize,
// which the C++ engines, whose host-posted events carry no sender, never do.
//
// Fixture:
// backends/c/tests/integration_resources/a_host_pair_runs_no_finalize/a_host_pair_runs_no_finalize.scxml
// (a document of this channel alone).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_host_pair_runs_no_finalize ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../require_started.h"
#include "a_host_pair_runs_no_finalize_sm.h"

typedef a_host_pair_runs_no_finalize_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

// Each counter the document keeps must read `want`.
static int expect(const sm_t *sm, const char *phase, const char *name, reader_t read, int64_t want) {
    int64_t got = -1;
    if (read(sm, &got) && got == want) {
        return 1;
    }
    fprintf(stderr, "FAIL [%s]: %s = %lld, want %lld\n", phase, name, (long long)got, (long long)want);
    return 0;
}

int main(void) {
    sm_t sm;
    REQUIRE_STARTED(a_host_pair_runs_no_finalize_init(&sm), A_HOST_PAIR_RUNS_NO_FINALIZE_START_OK);
    a_host_pair_runs_no_finalize_run(&sm);

    // The child's own send runs the finalize: the check below is not vacuous.
    int ok = 1;
    ok &= expect(&sm, "child", "heard", a_host_pair_runs_no_finalize_heard, 1);
    ok &= expect(&sm, "child", "finalized", a_host_pair_runs_no_finalize_finalized, 1);

    // The host posts an event naming the running child's own pair as its origin.
    a_host_pair_runs_no_finalize_event_with_meta_t poke;
    memset(&poke, 0, sizeof(poke));
    poke.event = A_HOST_PAIR_RUNS_NO_FINALIZE_EVENT_POKE;
    if (!sce_event_origin_scxml(&poke.origin, sm.child_kid.session_id) || sce_event_origin_is_none(&poke.origin)) {
        fprintf(stderr, "FAIL: the host could not spell the child's pair (child session '%s')\n",
                sm.child_kid.session_id);
        ok = 0;
    }
    a_host_pair_runs_no_finalize_raise_external(&sm, &poke);
    a_host_pair_runs_no_finalize_run(&sm);

    ok &= expect(&sm, "host", "poked", a_host_pair_runs_no_finalize_poked, 1);
    ok &= expect(&sm, "host", "finalized (the host's event ran the child's finalize)",
                 a_host_pair_runs_no_finalize_finalized, 1);

    a_host_pair_runs_no_finalize_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: a host's pair ran no finalize\n");
    return 0;
}
