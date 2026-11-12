// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test399 — C11 AOT runner.
//
// W3C SCXML 5.9.3 event descriptor matching across all six descriptor
// shapes the spec admits:
//   1. exact name        — `event="timeout"` matches only `timeout`
//   2. multi-token       — `event="foo bar"` matches `foo` OR `bar`
//   3. token prefix      — `foo` matches `foo` AND `foo.zoo` (dot boundary)
//   4. token boundary    — `foo` does NOT match `foos` (no `.` separator),
//                          so the doc-order-following `event="foos"`
//                          transition is the live target out of s04
//   5. prefix-suffix `*` — `event="foo.*"` matches `foo.zoo`
//   6. universal `*`     — matches every non-eventless event
// Each chained state s01..s06 forces one shape, and the final transition
// out of s06 (`event="*"`) closes the chain on `pass`. The 2 s
// `<send delay="2s" event="timeout"/>` from s0 is a safety-net guard
// whose `<transition event="timeout" target="fail"/>` only fires if the
// match-chain stalls — in the success path the eventless internal-queue
// drain reaches `pass` before any external `_tick` poll is ever needed
// (mirrors test403c's safety-net composition).
//
// W3C SCXML 5.9.3: event descriptor matching covers exact match, multi-token
// (`foo bar`), token-prefix dot boundary (`foo` matches `foo.zoo` but not
// `foos`), prefix-suffix wildcard (`foo.*`), and universal wildcard (`*`);
// analyzer.rs precomputes the per-transition matching set so C11 emits
// codegen-time OR lists in place of cpp/rust's runtime EventMatchingHelper
// call

#include <stdio.h>

#include "../require_started.h"
#include "test399_sm.h"

int main(void) {
    test399_t sm;
    REQUIRE_STARTED(test399_init(&sm), TEST399_START_OK);
    test399_run(&sm);

    int rc = test399_ended_in(&sm, TEST399_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test399: FAIL — active = 0x%08x\n", (unsigned)test399_active_states(&sm));
    }
    test399_destroy(&sm);
    return rc;
}
