// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: an <invoke> naming its target through an expression
// evaluates that expression at invoke-fire time, and a failure to evaluate
// raises error.execution — C11 AOT path.
//
// The clause puts two obligations on the Processor and this fixture is about
// the second only. The evaluated string does not select the child on this
// path: codegen fixes the child at build time and writes an immediate-<final>
// stub for it (docs/SCE_ACCEPTED_SUBSET.md §2.13), which C11 reaches by
// #including the stub's header from the parent translation unit. A fixture
// built on the value would therefore measure nothing here; one built on the
// FAILURE measures what every channel still owes.
//
// Fixture: integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(invoke_expression_failure_is_reported ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "invoke_expression_failure_is_reported_sm.h"

int main(void) {
    invoke_expression_failure_is_reported_t sm;
    invoke_expression_failure_is_reported_init(&sm);
    invoke_expression_failure_is_reported_run(&sm);

    int rc = 0;
    if (!invoke_expression_failure_is_reported_in_state(&sm, INVOKE_EXPRESSION_FAILURE_IS_REPORTED_STATE_PASS)) {
        fprintf(stderr, "invoke_expression_failure_is_reported: FAIL - the machine did not reach "
                        "`pass`. W3C SCXML 6.4.3 requires the expression to be evaluated when the "
                        "<invoke> fires and error.execution raised when it cannot be; reaching "
                        "`fail` means the child started on an expression nothing evaluated.\n");
        rc = 1;
    }

    invoke_expression_failure_is_reported_destroy(&sm);
    return rc;
}
