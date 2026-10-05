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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_DOCUMENT_STEM_TABLE
#error "SCE_DOCUMENT_STEM_TABLE names tests/document_stem/document_stem.json; the build defines it"
#endif

enum { FIELD_CAP = 256, MIN_CASES = 15 };

// One string field of the case object at `object`, unescaped into `out`.
static const char *read_text(const char *object, const char *name, char *out) {
    sce_payload_fields_t fields = {object};
    const char *p = sce_payload_find(&fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    sce_payload_sink_t sink;
    memset(&sink, 0, sizeof(sink));
    sink.text = out;
    sink.cap = FIELD_CAP;
    const char *refusal = sce_payload_walk_text(p, &sink);
    out[sink.len < FIELD_CAP ? sink.len : FIELD_CAP - 1u] = '\0';
    return refusal;
}

static char *read_table(void) {
    FILE *f = fopen(SCE_DOCUMENT_STEM_TABLE, "rb");
    if (f == NULL) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    const long size = ftell(f);
    rewind(f);
    char *buf = size >= 0 ? malloc((size_t)size + 1u) : NULL;
    if (buf == NULL || fread(buf, 1, (size_t)size, f) != (size_t)size) {
        free(buf);
        fclose(f);
        return NULL;
    }
    buf[size] = '\0';
    fclose(f);
    return buf;
}

int main(void) {
    char *table = read_table();
    if (table == NULL) {
        fprintf(stderr, "FAIL: cannot read %s\n", SCE_DOCUMENT_STEM_TABLE);
        return 1;
    }
    const char *p = strstr(table, "\"cases\"");
    p = p != NULL ? strchr(p, '[') : NULL;
    if (p == NULL) {
        fprintf(stderr, "FAIL: the table has no cases\n");
        free(table);
        return 1;
    }
    p++;

    int cases = 0;
    int failures = 0;
    for (;;) {
        p = sce_payload_skip_space(p);
        if (*p == ']') {
            break;
        }
        if (*p != '{') {
            fprintf(stderr, "FAIL: a case is not an object\n");
            failures++;
            break;
        }
        char name[FIELD_CAP];
        char value[FIELD_CAP];
        char stem[FIELD_CAP];
        const char *refusal = read_text(p, "name", name);
        if (refusal == NULL) {
            refusal = read_text(p, "value", value);
        }
        if (refusal == NULL) {
            refusal = read_text(p, "stem", stem);
        }
        if (refusal != NULL) {
            fprintf(stderr, "FAIL: case %d: the walker refused it: %s\n", cases, refusal);
            failures++;
        } else {
            char got[FIELD_CAP];
            sce_document_stem(value, got, sizeof(got));
            if (strcmp(got, stem) != 0) {
                fprintf(stderr, "FAIL: %s: the stem of \"%s\" is \"%s\", the table says \"%s\"\n", name, value, got,
                        stem);
                failures++;
            }
        }
        cases++;
        p = sce_payload_skip_container(p);
        if (p == NULL) {
            fprintf(stderr, "FAIL: a case object never closes\n");
            failures++;
            break;
        }
        p = sce_payload_skip_space(p);
        if (*p == ',') {
            p++;
        }
    }
    free(table);

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
