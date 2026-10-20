// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A <send> delay is read as one CSS2 time on every engine (ARCHITECTURE.md,
// "Durations (Single Source of Truth)"). The cases live in
// tests/durations/css2_time.json, read here with the event payload walker, and
// every case's `text` goes through sce_parse_delay_ms, which must answer the
// table's `ms` — or refuse it where `ms` is null.

#include <sce/event_payload.h>
#include <sce/types.h>

#include "shared_table.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_DURATION_TABLE
#error "SCE_DURATION_TABLE names tests/durations/css2_time.json; the build defines it"
#endif

enum { FIELD_CAP = 256, MIN_CASES = 20 };

// The case's `ms`: false in `*present` for null, otherwise the count.
static const char *read_ms(const char *object, bool *present, uint64_t *ms) {
    sce_payload_fields_t fields = {object, NULL};
    const char *p = sce_payload_find(&fields, "ms");
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    p = sce_payload_skip_space(p);
    if (strncmp(p, "null", 4) == 0) {
        *present = false;
        return NULL;
    }
    if (*p < '0' || *p > '9') {
        return SCE_PAYLOAD_REFUSE_NOT_NUMBER;
    }
    *present = true;
    *ms = (uint64_t)strtoull(p, NULL, 10);
    return NULL;
}

static int check_duration_case(const char *object, int index) {
    char name[FIELD_CAP];
    char text[FIELD_CAP];
    bool present = false;
    uint64_t want = 0u;
    const char *refusal = read_field(object, "name", name, sizeof(name), NULL);
    if (refusal == NULL) {
        refusal = read_field(object, "text", text, sizeof(text), NULL);
    }
    if (refusal == NULL) {
        refusal = read_ms(object, &present, &want);
    }
    if (refusal != NULL) {
        fprintf(stderr, "FAIL: case %d: the walker refused it: %s\n", index, refusal);
        return 1;
    }
    uint64_t got = 0u;
    const bool ok = sce_parse_delay_ms(text, &got);
    if (!present && ok) {
        fprintf(stderr, "FAIL: %s: read \"%s\" as %" PRIu64 " ms, the table says it is not a time\n", name, text, got);
        return 1;
    }
    if (present && !ok) {
        fprintf(stderr, "FAIL: %s: refused \"%s\", the table says %" PRIu64 " ms\n", name, text, want);
        return 1;
    }
    if (present && got != want) {
        fprintf(stderr, "FAIL: %s: read \"%s\" as %" PRIu64 " ms, the table says %" PRIu64 "\n", name, text, got, want);
        return 1;
    }
    return 0;
}

int main(void) {
    int failures = 0;
    const int cases = walk_table(SCE_DURATION_TABLE, check_duration_case, &failures);

    // A floor, not an equality: adding a case must not have to touch it, but
    // a table that stopped being read must not pass either.
    if (cases < MIN_CASES) {
        fprintf(stderr, "FAIL: the table produced only %d case(s)\n", cases);
        failures++;
    }
    if (failures != 0) {
        return 1;
    }
    printf("durations: %d case(s) read as one CSS2 time\n", cases);
    return 0;
}
