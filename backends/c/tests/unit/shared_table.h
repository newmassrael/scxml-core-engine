// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reader every C unit test of a shared table uses (tests/json_text,
// tests/durations, tests/document_stem, ...). A table is a JSON object with a
// `"cases"` array of objects; each test reads the table from disk, walks the
// cases with the event payload walker, and holds its own code to what a case says.
// One reader here, so a table is read one way by every test that holds a C
// function to it, and a reader that is wrong is wrong in one place.
//
// Header-only and `static inline`, like the runtime headers it builds on: a test
// that uses only some of the functions does not warn about the others.

#ifndef SCE_C_TESTS_UNIT_SHARED_TABLE_H
#define SCE_C_TESTS_UNIT_SHARED_TABLE_H

#include <sce/event_payload.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// The whole file at `path` in a buffer from malloc, terminated; NULL when it
// cannot be read. The caller frees it.
SCE_C_UNUSED static inline char *read_table(const char *path) {
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

// One string field of the case object at `object`, unescaped into `out` of `cap`
// bytes (at least one), its length in `*len` when `len` is not NULL: read through
// the walker rather than sce_payload_read_text because a case holds NUL, and a
// terminated string would end there. `out` is terminated for a reader that wants
// a C string. The walker's refusal, or NULL when the field was read whole.
SCE_C_UNUSED static inline const char *read_field(const char *object, const char *name, char *out, size_t cap,
                                                  size_t *len) {
    sce_payload_fields_t fields = {object, NULL};
    const char *p = sce_payload_find(&fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    sce_payload_sink_t sink;
    memset(&sink, 0, sizeof(sink));
    sink.text = out;
    sink.cap = cap;
    const char *refusal = sce_payload_walk_text(p, &sink);
    if (len != NULL) {
        *len = sink.len;
    }
    out[sink.len < cap ? sink.len : cap - 1u] = '\0';
    return refusal;
}

// The first case object's place in `table`: the text past the `[` of its
// `"cases"` array, or NULL when it has none.
SCE_C_UNUSED static inline const char *table_cases_open(const char *table) {
    const char *p = strstr(table, "\"cases\"");
    p = p != NULL ? strchr(p, '[') : NULL;
    return p != NULL ? p + 1 : NULL;
}

// Where the case after the case object at `object` begins (or the `]` that ends
// the array); NULL when the object never closes.
SCE_C_UNUSED static inline const char *table_case_after(const char *object) {
    const char *p = sce_payload_skip_container(object);
    if (p == NULL) {
        return NULL;
    }
    p = sce_payload_skip_space(p);
    return *p == ',' ? p + 1 : p;
}

// The failures one case of a table holds, given the case object and its place.
typedef int (*case_check_t)(const char *object, int index);

// Hand every case of the table at `path` to `check`. The number of cases, or -1
// when the table could not be read; the failures the cases hold are added to
// `failures`.
SCE_C_UNUSED static inline int walk_table(const char *path, case_check_t check, int *failures) {
    char *table = read_table(path);
    if (table == NULL) {
        fprintf(stderr, "FAIL: cannot read %s\n", path);
        return -1;
    }
    const char *p = table_cases_open(table);
    if (p == NULL) {
        fprintf(stderr, "FAIL: %s has no cases\n", path);
        free(table);
        return -1;
    }

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
        p = table_case_after(p);
        if (p == NULL) {
            fprintf(stderr, "FAIL: a case object never closes\n");
            (*failures)++;
            break;
        }
    }
    free(table);
    return cases;
}

// Room for the `name` of a case that table_find_case looks for.
enum { TABLE_CASE_NAME_CAP = 256 };

// The case object (its `{`) in the text of a whole table whose `name` field is
// `case_name`, compared as the unescaped text; NULL when no case has that name or
// the cases cannot be walked. The object lives in `table`, which the caller keeps
// for as long as it reads the case.
SCE_C_UNUSED static inline const char *table_find_case(const char *table, const char *case_name) {
    const char *p = table_cases_open(table);
    while (p != NULL) {
        p = sce_payload_skip_space(p);
        if (*p != '{') {
            return NULL;
        }
        char name[TABLE_CASE_NAME_CAP];
        if (read_field(p, "name", name, sizeof(name), NULL) == NULL && strcmp(name, case_name) == 0) {
            return p;
        }
        p = table_case_after(p);
    }
    return NULL;
}

#endif /* SCE_C_TESTS_UNIT_SHARED_TABLE_H */
