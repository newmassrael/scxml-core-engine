// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `<param>` name that repeats collects its values, in document order, into one
// array of the event's data on every engine (ARCHITECTURE.md, "JSON Object Key
// Order"; W3C SCXML test178). A `sce-static` machine generated as C writes the
// pairs of a `<send>`, a `<donedata>` and an `<invoke>` through the header-only
// wire writer of the forge runtime, which lists them sorted by name with a name's
// values in document order, so the writer collects a key it was just given again.
//
// The first case is the one `tests/json_text/object_key_order.json` holds every
// engine's writer to ("a name that repeats is one array in declaration order"):
// `k`, `j`, `k` with the values 1, 0, 2 is `{"j":0,"k":[1,2]}`. It is read from the
// table, its params and its expected text, and not copied here. The others are what a
// machine can do to it that the table cannot say: a value that failed to compute
// is left out, so a name with one value left is that value and not an array of
// one; a run of three; an array in the middle and at the end of the object; a
// string with an escape in an array; and an object that does not fit, which is
// `{}` and never a truncated array.

#include <sce/event_payload.h>
#include <sce/forge/wire.h>

#include "shared_table.h"

#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_JSON_KEY_ORDER_TABLE
#error "SCE_JSON_KEY_ORDER_TABLE names tests/json_text/object_key_order.json; the build defines it"
#endif

// The case of the table this test writes through the wire writer.
static const char TABLE_CASE_NAME[] = "a name that repeats is one array in declaration order";

enum { MAX_PAIRS = 8, PAIR_NAME_CAP = 64, EXPECTED_CAP = 256 };

// One element of the case's `params`: a name and a whole number.
typedef struct {
    // The name unescaped, which the pairs are ordered by.
    char name[PAIR_NAME_CAP];
    // The JSON text of the name, quotes included: what the writer is given. The
    // writer keeps the pointer until the object ends, so the pair outlives it.
    char key[PAIR_NAME_CAP];
    uint64_t value;
} table_pair_t;

static int expect_text(const char *what, const char *got, const char *want) {
    if (strcmp(got, want) != 0) {
        (void)fprintf(stderr, "%s: wrote `%s`, expected `%s`\n", what, got, want);
        return 1;
    }
    return 0;
}

// A param of the table case this test cannot write: said once, naming the case,
// and -1 so that the reader returns it. A negative `index` is the array itself.
static int refuse_param(int index, const char *why) {
    if (index < 0) {
        (void)fprintf(stderr, "table case \"%s\": params %s\n", TABLE_CASE_NAME, why);
    } else {
        (void)fprintf(stderr, "table case \"%s\": params[%d] %s\n", TABLE_CASE_NAME, index, why);
    }
    return -1;
}

// The case's `params` as pairs, in the order the table declares them: each
// element is `["name", <unsigned integer>]`, and a value of another kind (the
// table holds objects, arrays and strings in other cases) is refused rather than
// guessed at. The number of pairs, or -1 after saying why not.
static int read_params(const char *object, table_pair_t *pairs, int cap) {
    sce_payload_fields_t fields = {object, NULL};
    const char *p = sce_payload_find(&fields, "params");
    if (p == NULL || *p != '[') {
        return refuse_param(-1, "is missing or is not an array");
    }
    p = sce_payload_skip_space(p + 1);

    int count = 0;
    while (*p != ']') {
        if (count == cap) {
            return refuse_param(-1, "holds more pairs than this test has room for");
        }
        if (*p != '[') {
            return refuse_param(count, "is not a [name, value] pair");
        }
        const char *after = sce_payload_skip_container(p);
        if (after == NULL) {
            return refuse_param(count, "never closes");
        }

        table_pair_t *pair = &pairs[count];
        const char *q = sce_payload_skip_space(p + 1);
        sce_payload_sink_t sink;
        memset(&sink, 0, sizeof(sink));
        sink.text = pair->name;
        sink.cap = sizeof(pair->name);
        if (sce_payload_walk_text(q, &sink) != NULL) {
            return refuse_param(count, "has a name that is not a text this test can hold");
        }
        pair->name[sink.len] = '\0';
        if (strlen(pair->name) != sink.len) {
            return refuse_param(count, "has a name with a NUL in it, which no C string key holds");
        }
        if (!sce_payload_quote_span((const unsigned char *)pair->name, sink.len, false, pair->key, sizeof(pair->key))) {
            return refuse_param(count, "has a name this test cannot quote");
        }

        q = sce_payload_skip_string(q);
        q = q != NULL ? sce_payload_skip_space(q) : NULL;
        if (q == NULL || *q != ',') {
            return refuse_param(count, "has no value after its name");
        }
        q = sce_payload_skip_space(q + 1);
        if (*q < '0' || *q > '9') {
            return refuse_param(count, "has a value that is not an unsigned integer");
        }
        char *end = NULL;
        errno = 0;
        pair->value = (uint64_t)strtoull(q, &end, 10);
        if (errno != 0 || *sce_payload_skip_space(end) != ']') {
            return refuse_param(count, "has a value that is not an unsigned integer");
        }

        count++;
        p = sce_payload_skip_space(after);
        if (*p == ',') {
            p = sce_payload_skip_space(p + 1);
        }
    }
    return count;
}

// The pairs ordered by name, by the bytes of the name, and a name's values left in
// the order they were declared in: the order the generator lists them in. An
// insertion sort, which does not move a pair past one that is equal to it.
static void order_by_name(table_pair_t *pairs, int count) {
    for (int i = 1; i < count; ++i) {
        const table_pair_t held = pairs[i];
        int j = i;
        while (j > 0 && strcmp(pairs[j - 1].name, held.name) > 0) {
            pairs[j] = pairs[j - 1];
            --j;
        }
        pairs[j] = held;
    }
}

// The case is read from the table: its params are written through the wire writer
// in the order the generator lists them, sorted by name, so that the table's `k`,
// `j`, `k` is `j` and then the two `k`s, in the order they were declared in, and
// what the writer wrote must be the case's `data`.
static int the_table_case(void) {
    char *table = read_table(SCE_JSON_KEY_ORDER_TABLE);
    if (table == NULL) {
        (void)fprintf(stderr, "table case \"%s\": cannot read %s\n", TABLE_CASE_NAME, SCE_JSON_KEY_ORDER_TABLE);
        return 1;
    }
    const char *object = table_find_case(table, TABLE_CASE_NAME);
    if (object == NULL) {
        (void)fprintf(stderr, "table case \"%s\": %s has no case of that name\n", TABLE_CASE_NAME,
                      SCE_JSON_KEY_ORDER_TABLE);
        free(table);
        return 1;
    }

    char expected[EXPECTED_CAP];
    const char *refusal = read_field(object, "data", expected, sizeof(expected), NULL);
    if (refusal != NULL) {
        (void)fprintf(stderr, "table case \"%s\": data %s\n", TABLE_CASE_NAME, refusal);
        free(table);
        return 1;
    }
    table_pair_t pairs[MAX_PAIRS];
    const int count = read_params(object, pairs, MAX_PAIRS);
    free(table);
    if (count < 0) {
        return 1;
    }

    order_by_name(pairs, count);
    char buf[EXPECTED_CAP];
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    for (int i = 0; i < count; ++i) {
        sce_forge_wire_pair(&w, pairs[i].key, sce_forge_wire_uint(pairs[i].value));
    }
    int bad = sce_forge_wire_end(&w) ? 0 : 1;
    return bad | expect_text("a name that repeats is one array", buf, expected);
}

// A pair whose value failed to compute is left out and the others still go: when
// the first of a name's values failed, the one that is left is that value alone.
static int a_name_with_one_value_left_is_that_value(void) {
    char buf[64];
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"a\"", sce_forge_wire_uint(1u));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_uint(2u));
    sce_forge_wire_pair(&w, "\"z\"", sce_forge_wire_uint(3u));
    int bad = sce_forge_wire_end(&w) ? 0 : 1;
    return bad | expect_text("one value of a name", buf, "{\"a\":1,\"k\":2,\"z\":3}");
}

static int a_run_of_three_is_one_array_in_the_middle_and_at_the_end(void) {
    char buf[96];
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"a\"", sce_forge_wire_bool(true));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_uint(1u));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_uint(2u));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_uint(3u));
    sce_forge_wire_pair(&w, "\"m\"", sce_forge_wire_int(-4));
    sce_forge_wire_pair(&w, "\"z\"", sce_forge_wire_uint(5u));
    sce_forge_wire_pair(&w, "\"z\"", sce_forge_wire_uint(6u));
    int bad = sce_forge_wire_end(&w) ? 0 : 1;
    return bad | expect_text("runs of three and two", buf, "{\"a\":true,\"k\":[1,2,3],\"m\":-4,\"z\":[5,6]}");
}

// An array holds the values as each is written: a string with a quote in it, a real
// as ECMAScript spells it, and a value of another kind than the one before it.
static int an_array_holds_each_value_as_it_is_written(void) {
    char buf[96];
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_string("a\"b"));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_real(1.5));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_bool(false));
    int bad = sce_forge_wire_end(&w) ? 0 : 1;
    return bad | expect_text("mixed values", buf, "{\"k\":[\"a\\\"b\",1.5,false]}");
}

// Two names that only share a prefix are two names, not a repeat.
static int a_prefix_is_not_a_repeat(void) {
    char buf[64];
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"a\"", sce_forge_wire_uint(1u));
    sce_forge_wire_pair(&w, "\"ab\"", sce_forge_wire_uint(2u));
    int bad = sce_forge_wire_end(&w) ? 0 : 1;
    return bad | expect_text("a name and one that extends it", buf, "{\"a\":1,\"ab\":2}");
}

// An object that does not fit is `{}`: turning the first value of a name into an
// array takes two bytes more than the pair took, the `[` and the `]`, and a buffer
// that had no byte to give leaves no truncated array behind.
static int an_object_that_does_not_fit_is_empty(void) {
    int bad = 0;
    // `{"k":[1,2]}` is 11 bytes and its terminator: a buffer of 12 holds it, and
    // 11 and 10 do not.
    for (size_t cap = 10u; cap <= 12u; ++cap) {
        char buf[16];
        memset(buf, 'x', sizeof(buf));
        sce_forge_wire_t w;
        sce_forge_wire_begin(&w, buf, cap);
        sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_uint(1u));
        sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_uint(2u));
        const bool fits = sce_forge_wire_end(&w);
        if (cap == 12u) {
            bad |= fits ? 0 : 1;
            bad |= expect_text("a buffer that holds the array", buf, "{\"k\":[1,2]}");
        } else {
            bad |= fits ? 1 : 0;
            bad |= expect_text("a buffer that does not", buf, "{}");
        }
    }
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= the_table_case();
    bad |= a_name_with_one_value_left_is_that_value();
    bad |= a_run_of_three_is_one_array_in_the_middle_and_at_the_end();
    bad |= an_array_holds_each_value_as_it_is_written();
    bad |= a_prefix_is_not_a_repeat();
    bad |= an_object_that_does_not_fit_is_empty();
    if (bad != 0) {
        return 1;
    }
    (void)printf("forge wire repeat: ok\n");
    return 0;
}
