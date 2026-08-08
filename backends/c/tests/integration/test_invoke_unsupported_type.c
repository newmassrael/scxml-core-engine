// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1: an <invoke> naming an unsupported `type` places
// error.execution on the internal event queue — C11 AOT path.
//
// The spec defines the case, so the document is valid SCXML with exactly one
// observable: that raise. No child session starts and done.invoke.<id> never
// fires.
//
// Both engines were silent here in different ways before this landed — the
// Interpreter substituted an SCXML handler for the unknown type, and AOT
// dropped the <invoke> from the model entirely. A backend that renders this
// fixture without the raise reproduces the AOT form, and the machine then
// rests in `probe` instead of reaching `pass`.
//
// C11 reaches the raise through three gates the other backends do not share:
// the entry-action switch must emit a case for a state whose only content is
// the unsupported invoke, the `scxml_family` guards must admit the invoke
// includes and the pending queue, and `execute_pending_invokes` must carry an
// arm the `| scxml` filter would otherwise skip. Any one of them left closed
// yields a machine that compiles and rests in `probe`.
//
// Fixture: integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(invoke_unsupported_type ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "invoke_unsupported_type_sm.h"

int main(void) {
    invoke_unsupported_type_t sm;
    invoke_unsupported_type_init(&sm);
    invoke_unsupported_type_run(&sm);

    int rc = 0;
    if (!invoke_unsupported_type_in_state(&sm, INVOKE_UNSUPPORTED_TYPE_STATE_PASS)) {
        fprintf(stderr, "invoke_unsupported_type: FAIL - the machine did not reach `pass`. "
                        "W3C SCXML 6.4.1 requires an <invoke> whose `type` names no supported "
                        "processor to place error.execution on the internal event queue; "
                        "resting in `probe` means the <invoke> was dropped rather than lowered.\n");
        rc = 1;
    }

    invoke_unsupported_type_destroy(&sm);
    return rc;
}
