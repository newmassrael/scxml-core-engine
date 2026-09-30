// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a <send> reaches what its target names, carries
// its payload there, and a target that names nothing reachable is reported —
// C11 AOT.
//
// Fixture:
// integration_resources/a_send_reaches_only_what_its_target_names/a_send_reaches_only_what_its_target_names.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_send_reaches_only_what_its_target_names ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>
#include <string.h>

#include "a_send_reaches_only_what_its_target_names_sm.h"

typedef a_send_reaches_only_what_its_target_names_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    int ok = 1;

    /* W3C SCXML 6.2.4: this document sends to `#_parent`, so a host that runs
       machines as roots and asks for the refusal gets it. The plain `_init`
       below runs the same machine; the refusal is opt-in. */
    const char *refusal = a_send_reaches_only_what_its_target_names_root_start_refusal();
    if (refusal == NULL ||
        strcmp(refusal, "the machine sends to #_parent and was started with no parent session") != 0) {
        fprintf(stderr, "FAIL: root_start_refusal = %s, want the needs-parent sentence\n", refusal ? refusal : "NULL");
        ok = 0;
    }

    sm_t sm;
    a_send_reaches_only_what_its_target_names_init(&sm);
    a_send_reaches_only_what_its_target_names_run(&sm);

    if (!a_send_reaches_only_what_its_target_names_ended_in(&sm,
                                                            A_SEND_REACHES_ONLY_WHAT_ITS_TARGET_NAMES_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"execErrors", a_send_reaches_only_what_its_target_names_exec_errors, 1},
        {"commErrors", a_send_reaches_only_what_its_target_names_comm_errors, 5},
        {"afterRefused", a_send_reaches_only_what_its_target_names_after_refused, 0},
        {"afterNobody", a_send_reaches_only_what_its_target_names_after_nobody, 0},
        {"afterStranger", a_send_reaches_only_what_its_target_names_after_stranger, 0},
        {"afterOrphan", a_send_reaches_only_what_its_target_names_after_orphan, 0},
        {"afterOrphanExpr", a_send_reaches_only_what_its_target_names_after_orphan_expr, 0},
        {"afterOrphanExprLater", a_send_reaches_only_what_its_target_names_after_orphan_expr_later, 0},
        {"bareArrived", a_send_reaches_only_what_its_target_names_bare_arrived, 1},
        {"pongOk", a_send_reaches_only_what_its_target_names_pong_ok, 1},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_send_reaches_only_what_its_target_names_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each <send> reached what its target named\n");
    return 0;
}
