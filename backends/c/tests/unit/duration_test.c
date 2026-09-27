// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A <send> delay is read as one CSS2 time on every engine (ARCHITECTURE.md,
// "Durations (Single Source of Truth)"). The cases live in
// tests/durations/css2_time.json, read here with the event payload walker, and
// every case's `text` goes through sce_parse_delay_ms, which must answer the
// table's `ms` — or refuse it where `ms` is null.

#include <sce/event_payload.h>
#include <sce/types.h>

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_DURATION_TABLE
#error "SCE_DURATION_TABLE names tests/durations/css2_time.json; the build defines it"
#endif

enum { FIELD_CAP = 256, MIN_CASES = 20 };

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

// The case's `ms`: false in `*present` for null, otherwise the count.
static const char *read_ms(const char *object, bool *present, uint64_t *ms) {
    sce_payload_fields_t fields = {object};
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

static char *read_table(void) {
    FILE *f = fopen(SCE_DURATION_TABLE, "rb");
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
        fprintf(stderr, "FAIL: cannot read %s\n", SCE_DURATION_TABLE);
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
        char text[FIELD_CAP];
        bool present = false;
        uint64_t want = 0u;
        const char *refusal = read_text(p, "name", name);
        if (refusal == NULL) {
            refusal = read_text(p, "text", text);
        }
        if (refusal == NULL) {
            refusal = read_ms(p, &present, &want);
        }
        if (refusal != NULL) {
            fprintf(stderr, "FAIL: case %d: the walker refused it: %s\n", cases, refusal);
            failures++;
        } else {
            uint64_t got = 0u;
            const bool ok = sce_parse_delay_ms(text, &got);
            if (!present && ok) {
                fprintf(stderr, "FAIL: %s: read \"%s\" as %" PRIu64 " ms, the table says it is not a time\n", name,
                        text, got);
                failures++;
            } else if (present && !ok) {
                fprintf(stderr, "FAIL: %s: refused \"%s\", the table says %" PRIu64 " ms\n", name, text, want);
                failures++;
            } else if (present && got != want) {
                fprintf(stderr, "FAIL: %s: read \"%s\" as %" PRIu64 " ms, the table says %" PRIu64 "\n", name, text,
                        got, want);
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
    printf("durations: %d case(s) read as one CSS2 time\n", cases);
    return 0;
}
