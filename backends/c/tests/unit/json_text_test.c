// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string written into JSON text is written one way on every engine
// (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). The cases live in
// tests/json_text/string_escape.json. This backend both reads and writes the
// text an event's data carries, so the table measures both halves here: each
// case's `text` is read with the payload walker, and written back with the
// payload quoter, and what it writes must be the table's `escaped`.
//
// A float is written one way too (ARCHITECTURE.md, "JSON Number Text (Single
// Source of Truth)"). The cases live in tests/json_text/real_text.json, each
// value as its IEEE 754 bits, and the `sce-static` wire writer is held to them
// three ways: the number's own text, a pair of a data object, and the text a
// host request carries a param as.

#include <sce/event_payload.h>
#include <sce/forge/wire.h>

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_JSON_TEXT_TABLE
#error "SCE_JSON_TEXT_TABLE names tests/json_text/string_escape.json; the build defines it"
#endif
#ifndef SCE_JSON_REAL_TEXT_TABLE
#error "SCE_JSON_REAL_TEXT_TABLE names tests/json_text/real_text.json; the build defines it"
#endif

enum { FIELD_CAP = 256, MIN_CASES = 8, MIN_REAL_CASES = 40 };

// One string field of the case object at `object`, unescaped into `out`, its
// length in `len`: read through the walker rather than sce_payload_read_text
// because a case holds NUL, and a terminated string would end there. `out` is
// terminated for a reader that wants a C string.
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
    out[sink.len < FIELD_CAP ? sink.len : FIELD_CAP - 1u] = '\0';
    return refusal;
}

static char *read_table(const char *path) {
    FILE *f = fopen(path, "rb");
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

// The failures one case of a table holds, given the case object and its place.
typedef int (*case_check_t)(const char *object, int index);

// Hand every case of the table at `path` to `check`. The number of cases, or -1
// when the table could not be read; the failures the cases hold are added to
// `failures`.
static int walk_table(const char *path, case_check_t check, int *failures) {
    char *table = read_table(path);
    if (table == NULL) {
        fprintf(stderr, "FAIL: cannot read %s\n", path);
        return -1;
    }
    const char *p = strstr(table, "\"cases\"");
    p = p != NULL ? strchr(p, '[') : NULL;
    if (p == NULL) {
        fprintf(stderr, "FAIL: %s has no cases\n", path);
        free(table);
        return -1;
    }
    p++;

    int cases = 0;
    for (;;) {
        p = sce_payload_skip_space(p);
        if (*p == ']') {
            break;
        }
        if (*p != '{') {
            fprintf(stderr, "FAIL: a case is not an object\n");
            (*failures)++;
            break;
        }
        *failures += check(p, cases);
        cases++;
        p = sce_payload_skip_container(p);
        if (p == NULL) {
            fprintf(stderr, "FAIL: a case object never closes\n");
            (*failures)++;
            break;
        }
        p = sce_payload_skip_space(p);
        if (*p == ',') {
            p++;
        }
    }
    free(table);
    return cases;
}

static int check_string_case(const char *object, int index) {
    char name[FIELD_CAP];
    char text[FIELD_CAP];
    char escaped[FIELD_CAP];
    size_t name_len = 0u;
    size_t text_len = 0u;
    size_t escaped_len = 0u;
    const char *refusal = read_field(object, "name", name, &name_len);
    if (refusal == NULL) {
        refusal = read_field(object, "text", text, &text_len);
    }
    if (refusal == NULL) {
        refusal = read_field(object, "escaped", escaped, &escaped_len);
    }
    if (refusal != NULL) {
        fprintf(stderr, "FAIL: case %d: the walker refused it: %s\n", index, refusal);
        return 1;
    }
    // Room for a whole field and the two quotes around it.
    char written[FIELD_CAP + 2];
    char expected[FIELD_CAP + 2];
    snprintf(expected, sizeof(expected), "\"%s\"", escaped);
    if (!sce_payload_quote_span((const unsigned char *)text, text_len, false, written, sizeof(written))) {
        fprintf(stderr, "FAIL: %s: the quoter refused it\n", name);
        return 1;
    }
    if (strcmp(written, expected) != 0) {
        fprintf(stderr, "FAIL: %s: wrote %s, the table says %s\n", name, written, expected);
        return 1;
    }
    return 0;
}

// What the wire writer writes for `value`, as the table says it spells: the
// number's own text, the pair of a data object that carries it, and the text a
// host request carries a param as.
static int check_real_written(const char *name, double value, const char *text) {
    int failures = 0;
    char written[64];
    // The table's text is a field of up to FIELD_CAP bytes, so the pair that
    // carries it holds that and the braces, the name and the colon around it.
    char expected_pair[FIELD_CAP + 8];
    if (sce_number_text(value, written, sizeof(written)) == 0u || strcmp(written, text) != 0) {
        fprintf(stderr, "FAIL: %s: the number text is %s, the table says %s\n", name, written, text);
        failures++;
    }

    char object[96];
    sce_forge_wire_t wire;
    sce_forge_wire_begin(&wire, object, sizeof(object));
    sce_forge_wire_pair(&wire, "\"x\"", sce_forge_wire_real(value));
    snprintf(expected_pair, sizeof(expected_pair), "{\"x\":%s}", text);
    if (!sce_forge_wire_end(&wire) || strcmp(object, expected_pair) != 0) {
        fprintf(stderr, "FAIL: %s: as a pair wrote %s, the table says %s\n", name, object, expected_pair);
        failures++;
    }

    if (!sce_forge_wire_text(sce_forge_wire_real(value), written, sizeof(written)) || strcmp(written, text) != 0) {
        fprintf(stderr, "FAIL: %s: as a param's text wrote %s, the table says %s\n", name, written, text);
        failures++;
    }
    return failures;
}

static int check_real_case(const char *object, int index) {
    char name[FIELD_CAP];
    char bits[FIELD_CAP];
    char text[FIELD_CAP];
    size_t name_len = 0u;
    size_t bits_len = 0u;
    size_t text_len = 0u;
    const char *refusal = read_field(object, "name", name, &name_len);
    if (refusal == NULL) {
        refusal = read_field(object, "bits", bits, &bits_len);
    }
    if (refusal == NULL) {
        refusal = read_field(object, "text", text, &text_len);
    }
    if (refusal != NULL) {
        fprintf(stderr, "FAIL: real case %d: the walker refused it: %s\n", index, refusal);
        return 1;
    }
    // The bits are the value: no engine's float parser is the one deciding.
    const uint64_t pattern = (uint64_t)strtoull(bits, NULL, 16);
    double value;
    memcpy(&value, &pattern, sizeof(value));
    return check_real_written(name, value, text);
}

// JSON has no spelling for a float that is not finite, so a pair carries `null`;
// a host request's text gives it the one ECMAScript does.
static int check_non_finite(void) {
    const struct {
        uint64_t bits;
        const char *text;
    } cases[] = {{UINT64_C(0x7FF8000000000000), "NaN"},
                 {UINT64_C(0x7FF0000000000000), "Infinity"},
                 {UINT64_C(0xFFF0000000000000), "-Infinity"}};

    int failures = 0;
    for (size_t i = 0u; i < sizeof(cases) / sizeof(cases[0]); i++) {
        double value;
        memcpy(&value, &cases[i].bits, sizeof(value));
        char object[32];
        sce_forge_wire_t wire;
        sce_forge_wire_begin(&wire, object, sizeof(object));
        sce_forge_wire_pair(&wire, "\"x\"", sce_forge_wire_real(value));
        if (!sce_forge_wire_end(&wire) || strcmp(object, "{\"x\":null}") != 0) {
            fprintf(stderr, "FAIL: %s as a pair wrote %s, want null\n", cases[i].text, object);
            failures++;
        }
        char written[32];
        if (!sce_forge_wire_text(sce_forge_wire_real(value), written, sizeof(written)) ||
            strcmp(written, cases[i].text) != 0) {
            fprintf(stderr, "FAIL: %s as a param's text wrote %s\n", cases[i].text, written);
            failures++;
        }
    }
    return failures;
}

int main(void) {
    int failures = 0;
    const int cases = walk_table(SCE_JSON_TEXT_TABLE, check_string_case, &failures);
    const int real_cases = walk_table(SCE_JSON_REAL_TEXT_TABLE, check_real_case, &failures);
    failures += check_non_finite();

    // A floor, not an equality: adding a case must not have to touch it, but
    // a table that stopped being read must not pass either.
    if (cases < MIN_CASES) {
        fprintf(stderr, "FAIL: the table produced only %d case(s)\n", cases);
        failures++;
    }
    if (real_cases < MIN_REAL_CASES) {
        fprintf(stderr, "FAIL: the real table produced only %d case(s)\n", real_cases);
        failures++;
    }
    if (failures != 0) {
        return 1;
    }
    printf("json_text: %d case(s) written in the one form, %d float(s) spelled as ECMAScript spells them\n", cases,
           real_cases);
    return 0;
}
