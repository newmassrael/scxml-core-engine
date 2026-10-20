// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The value a hybrid <invoke srcexpr> computes is reduced to the stem of the
// document it names (docs/SCE_ACCEPTED_SUBSET.md §2.13), and every engine
// reduces it the same way. The cases live in tests/document_stem/document_stem.json,
// read here with the event payload walker, and every case's `value` goes through
// sce_document_stem, which must answer the table's `stem`.

#include <sce/event_payload.h>
#include <sce/invoke.h>

#include "shared_table.h"

#include <stdio.h>
#include <string.h>

#ifndef SCE_DOCUMENT_STEM_TABLE
#error "SCE_DOCUMENT_STEM_TABLE names tests/document_stem/document_stem.json; the build defines it"
#endif

enum { FIELD_CAP = 256, MIN_CASES = 15 };

static int check_stem_case(const char *object, int index) {
    char name[FIELD_CAP];
    char value[FIELD_CAP];
    char stem[FIELD_CAP];
    const char *refusal = read_field(object, "name", name, sizeof(name), NULL);
    if (refusal == NULL) {
        refusal = read_field(object, "value", value, sizeof(value), NULL);
    }
    if (refusal == NULL) {
        refusal = read_field(object, "stem", stem, sizeof(stem), NULL);
    }
    if (refusal != NULL) {
        fprintf(stderr, "FAIL: case %d: the walker refused it: %s\n", index, refusal);
        return 1;
    }
    char got[FIELD_CAP];
    sce_document_stem(value, got, sizeof(got));
    if (strcmp(got, stem) != 0) {
        fprintf(stderr, "FAIL: %s: the stem of \"%s\" is \"%s\", the table says \"%s\"\n", name, value, got, stem);
        return 1;
    }
    return 0;
}

int main(void) {
    int failures = 0;
    const int cases = walk_table(SCE_DOCUMENT_STEM_TABLE, check_stem_case, &failures);

    // A floor, not an equality: adding a case must not have to touch it, but
    // a table that stopped being read must not pass either.
    if (cases < MIN_CASES) {
        fprintf(stderr, "FAIL: the table produced only %d case(s)\n", cases);
        failures++;
    }
    if (failures != 0) {
        return 1;
    }
    printf("document stem: %d case(s) reduced to one stem\n", cases);
    return 0;
}
