// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(399) invocation.

#include <stdio.h>

#include "test399_sm.h"

int main(void) {
    test399_t sm;
    test399_init(&sm);
    test399_run(&sm);

    int rc = test399_in_state(&sm, TEST399_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test399: FAIL — active = 0x%08x\n",
                (unsigned)test399_active_states(&sm));
    }
    test399_destroy(&sm);
    return rc;
}
