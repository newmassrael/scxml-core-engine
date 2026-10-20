// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A 64-bit real in a typed event payload is written into the event's `data` the
// one way every engine writes a real (ARCHITECTURE.md, "JSON Number Text"):
// the fewest digits that read back, `5` for a whole value, and the exponent
// form from 1e21.
//
// The inject seam `..._raise_reading_taken_typed` writes the payload as the JSON
// text of the queued event's `data`, and the machine reads it back by `strtod`,
// so the machine behaves the same whichever digits were written: `0.1` and
// `0.10000000000000001` are one double. That is why this reads the TEXT, from the
// queue the seam wrote it into, and not what the machine did with it.
//
// MCU-clean acceptance: like its integer twin (test_event_schema_native.c) the
// target links NO runtime library, so a payload writer that needed one — or a
// header outside the runtime's include directory — would not link.
//
// The cases are tests/json_text/real_text.json, read at run time: each is a value as its
// IEEE 754 bits and the text every engine writes for it, so this backend is held to the
// same table as the others and carries no copy of it.
//
// Fixture: integration_resources/event_schema_real/event_schema_real.scxml
// (+ schema_reading.scxml), generated at CMake build time.

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "event_schema_real_sm.h"

#ifndef SCE_JSON_REAL_TEXT_TABLE
#error "SCE_JSON_REAL_TEXT_TABLE names tests/json_text/real_text.json; the build defines it"
#endif

enum { OBJECT_CAP = 256, FIELD_CAP = 64, MIN_CASES = 40 };

// The data text of the one event the seam queued.
static const char *queued_data(const event_schema_real_t *sm) {
    return sm->external_queue.buf[sm->external_queue.head].data;
}

// The table, whole, or NULL when it cannot be read. Read here and not through the
// runtime's payload reader: this target links no runtime library.
static char *read_table(const char *path) {
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        (void)fclose(f);
        return NULL;
    }
    const long size = ftell(f);
    rewind(f);
    char *buf = size >= 0 ? malloc((size_t)size + 1u) : NULL;
    if (buf == NULL || fread(buf, 1, (size_t)size, f) != (size_t)size) {
        free(buf);
        (void)fclose(f);
        return NULL;
    }
    buf[size] = '\0';
    (void)fclose(f);
    return buf;
}

// The string value of `key` in the flat case object `object`, copied into `out`.
// A case holds a name, the bits and the text, none of which needs an escape.
static int case_field(const char *object, const char *key, char *out) {
    char needle[16];
    (void)snprintf(needle, sizeof(needle), "\"%s\"", key);
    const char *p = strstr(object, needle);
    p = p != NULL ? strchr(p + strlen(needle), '"') : NULL;
    if (p == NULL) {
        return 0;
    }
    p++;
    const char *end = strchr(p, '"');
    if (end == NULL || (size_t)(end - p) >= FIELD_CAP) {
        return 0;
    }
    memcpy(out, p, (size_t)(end - p));
    out[end - p] = '\0';
    return 1;
}

// One case of the table through the seam: the value is the case's IEEE 754 bits, so
// no float parser decides it, and the data text must be the case's `text`.
static int check_case(const char *object, int index) {
    char name[FIELD_CAP];
    char bits[FIELD_CAP];
    char text[FIELD_CAP];
    if (!case_field(object, "name", name) || !case_field(object, "bits", bits) || !case_field(object, "text", text) ||
        strlen(bits) != 16u) {
        (void)fprintf(stderr, "FAIL: case %d of the table is not a name, sixteen hex digits and a text\n", index);
        return 1;
    }
    const uint64_t value_bits = strtoull(bits, NULL, 16);
    char want[FIELD_CAP + 16];
    (void)snprintf(want, sizeof(want), "{\"value\":%s}", text);

    event_schema_real_t sm;
    event_schema_real_init(&sm);
    event_schema_real_reading_taken_payload_t payload;
    memset(&payload, 0, sizeof(payload));
    memcpy(&payload.value, &value_bits, sizeof(payload.value));
    if (!event_schema_real_raise_reading_taken_typed(&sm, &payload)) {
        (void)fprintf(stderr, "FAIL: %s: the seam refused a finite value\n", name);
        return 1;
    }
    if (strcmp(queued_data(&sm), want) != 0) {
        (void)fprintf(stderr, "FAIL: %s: wrote %s, the table says %s\n", name, queued_data(&sm), want);
        return 1;
    }
    return 0;
}

int main(void) {
    int bad = 0;

    // Every case of tests/json_text/real_text.json, the table all engines are held to.
    char *table = read_table(SCE_JSON_REAL_TEXT_TABLE);
    const char *p = table != NULL ? strstr(table, "\"cases\"") : NULL;
    p = p != NULL ? strchr(p, '[') : NULL;
    if (p == NULL) {
        (void)fprintf(stderr, "FAIL: cannot read the cases of %s\n", SCE_JSON_REAL_TEXT_TABLE);
        free(table);
        return 1;
    }
    int cases = 0;
    for (p = strchr(p, '{'); p != NULL; p = strchr(p, '{')) {
        const char *end = strchr(p, '}');
        char object[OBJECT_CAP];
        if (end == NULL || (size_t)(end - p) >= sizeof(object)) {
            (void)fprintf(stderr, "FAIL: case %d of the table is not a flat object\n", cases);
            bad = 1;
            break;
        }
        memcpy(object, p, (size_t)(end - p));
        object[end - p] = '\0';
        bad |= check_case(object, cases);
        cases++;
        p = end + 1;
    }
    free(table);
    // A table that lost its cases must not read as a pass.
    if (cases < MIN_CASES) {
        (void)fprintf(stderr, "FAIL: %d cases read from the table, at least %d expected\n", cases, MIN_CASES);
        bad = 1;
    }

    // JSON has no spelling for a value that is not finite, so the seam refuses it
    // rather than write text no peer can read.
    {
        event_schema_real_t sm;
        event_schema_real_init(&sm);
        event_schema_real_reading_taken_payload_t payload;
        memset(&payload, 0, sizeof(payload));
        payload.value = INFINITY;
        if (event_schema_real_raise_reading_taken_typed(&sm, &payload)) {
            (void)fprintf(stderr, "FAIL: the seam wrote a value that is not finite\n");
            bad = 1;
        }
    }

    if (bad != 0) {
        return 1;
    }
    (void)printf("PASS: C11 typed payload writes a real as ECMAScript spells it\n");
    return 0;
}
