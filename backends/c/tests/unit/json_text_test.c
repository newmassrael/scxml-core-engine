// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string written into JSON text is written one way on every engine
// (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). The cases live in
// tests/json_text/string_escape.json. This backend both reads and writes the
// text an event's data carries, so the table measures both halves here: each
// case's `text` is read with the payload walker, and written back with the
// payload quoter, and what it writes must be the table's `escaped`.

#include <sce/event_payload.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_JSON_TEXT_TABLE
#error "SCE_JSON_TEXT_TABLE names tests/json_text/string_escape.json; the build defines it"
#endif

enum { FIELD_CAP = 256, MIN_CASES = 8 };

// One string field of the case object at `object`, unescaped into `out`, its
// length in `len`: read through the walker rather than sce_payload_read_text
// because a case holds NUL, and a terminated string would end there.
static const char *read_field(const char *object, const char *name, char *out, size_t *len) {
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
    *len = sink.len;
    return refusal;
}

static char *read_table(void) {
    FILE *f = fopen(SCE_JSON_TEXT_TABLE, "rb");
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
        fprintf(stderr, "FAIL: cannot read %s\n", SCE_JSON_TEXT_TABLE);
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
        char escaped[FIELD_CAP];
        size_t name_len = 0u;
        size_t text_len = 0u;
        size_t escaped_len = 0u;
        const char *refusal = read_field(p, "name", name, &name_len);
        if (refusal == NULL) {
            refusal = read_field(p, "text", text, &text_len);
        }
        if (refusal == NULL) {
            refusal = read_field(p, "escaped", escaped, &escaped_len);
        }
        name[name_len < FIELD_CAP ? name_len : FIELD_CAP - 1u] = '\0';
        escaped[escaped_len < FIELD_CAP ? escaped_len : FIELD_CAP - 1u] = '\0';
        if (refusal != NULL) {
            fprintf(stderr, "FAIL: case %d: the walker refused it: %s\n", cases, refusal);
            failures++;
        } else {
            // Room for a whole field and the two quotes around it.
            char written[FIELD_CAP + 2];
            char expected[FIELD_CAP + 2];
            snprintf(expected, sizeof(expected), "\"%s\"", escaped);
            if (!sce_payload_quote_span((const unsigned char *)text, text_len, false, written, sizeof(written))) {
                fprintf(stderr, "FAIL: %s: the quoter refused it\n", name);
                failures++;
            } else if (strcmp(written, expected) != 0) {
                fprintf(stderr, "FAIL: %s: wrote %s, the table says %s\n", name, written, expected);
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
    printf("json_text: %d case(s) written in the one form\n", cases);
    return 0;
}
