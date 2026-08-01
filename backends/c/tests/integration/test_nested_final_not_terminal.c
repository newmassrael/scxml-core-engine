// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.7: only a top-level <final> ends the session — C11 AOT path.
//
// Appendix D enterStates sets running = false for a <final> only when
// isSCXMLElement(s.parent); otherwise it queues done.state.<parent> and the
// machine carries on. "Is this state a <final> element" is the structural
// question; it is not the completion criterion, and only the latter may gate
// completion, the completion callback, and the done.invoke.<id> a parent
// emits for this machine.
//
// The fixture rests in the nested final rather than passing through it: a
// machine that continues within the same macrostep is only ever sampled at
// the end, where a right and a wrong predicate agree.
//
// Fixture: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(nested_final_not_terminal ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "nested_final_not_terminal_sm.h"

int main(void) {
    nested_final_not_terminal_t sm;
    nested_final_not_terminal_init(&sm);
    nested_final_not_terminal_run(&sm);

    int rc = 0;
    if (!nested_final_not_terminal_in_state(&sm, NESTED_FINAL_NOT_TERMINAL_STATE_PHASEDONE)) {
        fprintf(stderr, "nested_final_not_terminal: FAIL - the fixture is supposed to "
                        "come to rest in the nested <final>; it did not, so nothing "
                        "below tests what it claims\n");
        rc = 1;
    } else if (nested_final_not_terminal_is_in_final_state(&sm)) {
        fprintf(stderr, "nested_final_not_terminal: FAIL - the engine reported "
                        "completion while resting in `phaseDone`, a <final> nested "
                        "inside `phase`. W3C SCXML Appendix D enterStates ends the "
                        "session only when the final's parent is the <scxml> element: "
                        "a nested one finishes its compound state and queues "
                        "done.state.phase, leaving the machine live.\n");
        rc = 1;
    } else {
        // The by-name entry point is emitted only for machines that host an
        // invoke; this fixture has none, so the event goes on by enum.
        nested_final_not_terminal_event_with_meta_t resume = {0};
        resume.event = NESTED_FINAL_NOT_TERMINAL_EVENT_RESUME;
        nested_final_not_terminal_raise_external(&sm, &resume);
        nested_final_not_terminal_run(&sm);
        if (!nested_final_not_terminal_in_state(&sm, NESTED_FINAL_NOT_TERMINAL_STATE_PASS)) {
            fprintf(stderr,
                    "nested_final_not_terminal: FAIL - `resume` did not carry "
                    "the machine out of the nested final to the top-level one. "
                    "Diagnostic: in_PASS=%d in_phaseDone=%d\n",
                    nested_final_not_terminal_in_state(&sm, NESTED_FINAL_NOT_TERMINAL_STATE_PASS),
                    nested_final_not_terminal_in_state(&sm, NESTED_FINAL_NOT_TERMINAL_STATE_PHASEDONE));
            rc = 1;
        }
    }

    nested_final_not_terminal_destroy(&sm);
    return rc;
}
