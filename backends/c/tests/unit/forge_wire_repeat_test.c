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
// `k`, `j`, `k` with the values 1, 0, 2 is `{"j":0,"k":[1,2]}`. The others are what a
// machine can do to it that the table cannot say: a value that failed to compute
// is left out, so a name with one value left is that value and not an array of
// one; a run of three; an array in the middle and at the end of the object; a
// string with an escape in an array; and an object that does not fit, which is
// `{}` and never a truncated array.

#include <sce/forge/wire.h>

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int expect_text(const char *what, const char *got, const char *want) {
    if (strcmp(got, want) != 0) {
        (void)fprintf(stderr, "%s: wrote `%s`, expected `%s`\n", what, got, want);
        return 1;
    }
    return 0;
}

// The pairs are written in the order the generator lists them: sorted by name, so
// the table's `k`, `j`, `k` is `j` and then the two `k`s, in the order they were
// declared in.
static int the_table_case(void) {
    char buf[64];
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"j\"", sce_forge_wire_uint(0u));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_uint(1u));
    sce_forge_wire_pair(&w, "\"k\"", sce_forge_wire_uint(2u));
    int bad = sce_forge_wire_end(&w) ? 0 : 1;
    return bad | expect_text("a name that repeats is one array", buf, "{\"j\":0,\"k\":[1,2]}");
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
