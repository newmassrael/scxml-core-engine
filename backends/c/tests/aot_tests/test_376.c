// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test376 — C11 AOT runner.
//
// W3C SCXML 3.13: each `<onentry>` element is an independent execution block
// — block 1's `<send target="!invalid">` raises error.execution and stops
// subsequent actions in that block (W3C 5.10), but block 2's `<assign
// Var1=Var1+1>` still runs because block 2 is a separate block — Var1 reaches
// 2 → cond `Var1==2` routes s0→pass. The C11 mirror lifts each onentry block
// into its own static helper function (cpp's
// `EntryExitHelper::executeEntryBlocks` vector-of-lambdas dispatch,
// T3-inlined): a `return;` inside the helper terminates only that block, and
// the enclosing switch case sequentially calls the next helper.
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST376_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test376_sm.h"

int main(void) {
    test376_t sm;
    REQUIRE_STARTED(test376_init(&sm), TEST376_START_OK);
    test376_run(&sm);

    int rc = test376_ended_in(&sm, TEST376_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test376: FAIL — active = 0x%08x\n", (unsigned)test376_active_states(&sm));
    }
    test376_destroy(&sm);
    return rc;
}
