// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: the value a srcexpr computes is the child that runs — C11
// AOT path, over the candidate set sce:candidates declares.
//
// C11 embeds each child by value, so a declared set is a slot per candidate
// and at most one is ever active. That shape is why this channel is worth
// asking separately: a slot left active, or the wrong one initialised, is a
// mistake the other backends' pointers would not make the same way.
//
//   ran chosen     -> from.chosen     -> pass
//   ran other      -> from.other      -> wrongChild
//   loaded nothing -> error.execution -> noChild
//   ran a stub     -> no event at all -> rests in probe
//
// Fixture: integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(invoke_candidate_selects_the_child ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "invoke_candidate_selects_the_child_sm.h"

int main(void) {
    invoke_candidate_selects_the_child_t sm;
    invoke_candidate_selects_the_child_init(&sm);
    invoke_candidate_selects_the_child_run(&sm);

    int rc = 0;
    if (!invoke_candidate_selects_the_child_in_state(&sm, INVOKE_CANDIDATE_SELECTS_THE_CHILD_STATE_PASS)) {
        fprintf(stderr, "invoke_candidate_selects_the_child: FAIL - the machine did not reach "
                        "`pass`. W3C SCXML 6.4.3 makes the evaluated value name the child that "
                        "runs; `wrongChild` means the other candidate ran, `noChild` means "
                        "nothing loaded, and resting in `probe` means no child spoke at all.\n");
        rc = 1;
    }

    invoke_candidate_selects_the_child_destroy(&sm);
    return rc;
}
