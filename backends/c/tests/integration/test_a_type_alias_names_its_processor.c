// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 5.10.1 (docs/adr/0090, section 4): a `typeexpr` that
// evaluates to the alias `scxml` names the SCXML processor, and the event it
// delivers reads the processor's URI as its origintype; a spelling that is no
// alias is still refused — C11 AOT. This backend used to refuse the alias.
//
// Fixture:
// backends/c/tests/integration_resources/a_type_alias_names_its_processor/a_type_alias_names_its_processor.scxml
// (a document of this channel alone).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_type_alias_names_its_processor ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "../require_started.h"
#include "a_type_alias_names_its_processor_sm.h"

typedef a_type_alias_names_its_processor_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    REQUIRE_STARTED(a_type_alias_names_its_processor_init(&sm), A_TYPE_ALIAS_NAMES_ITS_PROCESSOR_START_OK);
    a_type_alias_names_its_processor_run(&sm);

    int ok = 1;
    if (!a_type_alias_names_its_processor_ended_in(&sm, A_TYPE_ALIAS_NAMES_ITS_PROCESSOR_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"aliasOk", a_type_alias_names_its_processor_alias_ok, 1},
        {"aliasRefused", a_type_alias_names_its_processor_alias_refused, 0},
        {"otherRefused", a_type_alias_names_its_processor_other_refused, 1},
        {"strayed", a_type_alias_names_its_processor_strayed, 0},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_type_alias_names_its_processor_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: a type alias named its processor\n");
    return 0;
}
