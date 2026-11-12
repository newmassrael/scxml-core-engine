// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0096: every `_init*` of a generated C11 machine answers whether the
// machine started, as `<machine>_start_status_t`, and the answer is the
// caller's to read (`SCE_WARN_UNUSED`; GCC does not let a `(void)` cast drop it).
// A test that starts a machine to observe what it does has nothing to observe
// when the start is refused, so the refusal fails the test where it happened:
//
//     REQUIRE_STARTED(my_machine_init(&sm), MY_MACHINE_START_OK);
//
// The run ends with status 1, which CTest reads as the failure, and the file,
// the line and the call that was refused are written to the standard error.
// A test that asks whether a start is refused (a host table that is incomplete,
// a script session that cannot be established) compares the answer itself.

#ifndef SCE_C_TESTS_REQUIRE_STARTED_H
#define SCE_C_TESTS_REQUIRE_STARTED_H

#include <stdio.h>
#include <stdlib.h>

// The `_START_OK` of a machine that a macro names by a pasted token, and so
// cannot spell in upper case: it is 0 on every machine (the generated header's
// `<machine>_start_status_t` says so).
#define PASTED_START_OK 0

#define REQUIRE_STARTED(start, started)                                                                                \
    do {                                                                                                               \
        if ((start) != (started)) {                                                                                    \
            (void)fprintf(stderr, "%s:%d: FAIL - the machine did not start: %s\n", __FILE__, __LINE__, #start);        \
            exit(1);                                                                                                   \
        }                                                                                                              \
    } while (0)

#endif
