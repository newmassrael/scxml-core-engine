// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The link-time clock of a test that must run on a manual clock: reading it ends the test.
//
// A saved state carries the deadlines a machine had left, and the saved-state tests compare what a
// machine writes with the shared text every backend writes, byte for byte. That comparison holds
// only while the machine reads time from the test's manual clock. A machine started through an
// `_init` that installs `sce_clock_monotonic()` reads `_sce_clock_now_ms()` instead, and then a
// deadline saved a moment after it was set is shorter by whatever the build machine took between the
// two statements: equal on most runs, a millisecond short on a few. Measured on 2026-10-23 as
// `c11_integration_static_host_invoke_saved` failing in CI on one run, after passing on the one before
// with no change to the C11 tree.
//
// So these tests link this definition in place of the POSIX one in `runtime/posix/clock.c`, the
// seam that file names for a host that brings its own time. A machine that reads the wall clock here
// stops at the first reading, on every run, and says why.

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

uint64_t _sce_clock_now_ms(void) {
    (void)fputs("this test runs on a manual clock, and a machine in it read the wall clock: start it "
                "with an `_init_with_clock...` door and `sce_clock_manual`, or restore it with "
                "`args.clock = sce_clock_manual(...)`\n",
                stderr);
    abort();
}
