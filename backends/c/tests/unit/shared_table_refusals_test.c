// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reader every C unit test of a shared table uses (shared_table.h) refuses what
// it cannot read: a value that is not an unsigned integer, a name that is not a
// text or cannot be held or quoted, a pair that is not `[name, value]`, a `params`
// that is not an array, a case that is not in the table, a table that cannot be
// opened. The real tables hold none of these (their values are integers, their
// cases are there), so a test that reads them never takes a refusal path, and a
// refusal that stopped refusing would be read as a table that agrees with the code.
// Here the tables are text in memory, each one wrong in the one way its case names,
// and the reader must refuse it; a well formed case must still be read whole.

#include "shared_table.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

enum { MAX_PAIRS = 4, BUILT_CAP = 512, LONG_NAME_LEN = 40, UNHOLDABLE_NAME_LEN = 70 };

// The reader's answer for the case object `object`, which must be a refusal: the
// message the reader gave, and one failure when it read the case instead.
static int expect_refused(const char *what, const char *object) {
    table_pair_t pairs[MAX_PAIRS];
    size_t count = 99u;
    const char *refusal = table_read_params(object, pairs, MAX_PAIRS, &count);
    if (refusal == NULL) {
        (void)fprintf(stderr, "FAIL: %s: read %zu pair(s) of %s, which must be refused\n", what, count, object);
        return 1;
    }
    if (refusal[0] == '\0') {
        (void)fprintf(stderr, "FAIL: %s: the refusal of %s says nothing\n", what, object);
        return 1;
    }
    if (count != 0u) {
        (void)fprintf(stderr, "FAIL: %s: a refused case reports %zu pair(s), not 0\n", what, count);
        return 1;
    }
    return 0;
}

// A pair of the well formed case, as read.
static int expect_pair(const char *what, const table_pair_t *pair, const char *name, const char *key, uint64_t value) {
    int bad = 0;
    if (strcmp(pair->name, name) != 0) {
        (void)fprintf(stderr, "FAIL: %s: the name is `%s`, expected `%s`\n", what, pair->name, name);
        bad = 1;
    }
    if (strcmp(pair->key, key) != 0) {
        (void)fprintf(stderr, "FAIL: %s: the quoted name is `%s`, expected `%s`\n", what, pair->key, key);
        bad = 1;
    }
    if (pair->value != value) {
        (void)fprintf(stderr, "FAIL: %s: `%s` holds %llu, expected %llu\n", what, name, (unsigned long long)pair->value,
                      (unsigned long long)value);
        bad = 1;
    }
    return bad;
}

// What the reader must accept: the pairs in the order the case declares them, a
// name that repeats kept as it came, and the widest unsigned integer.
static int a_well_formed_case_is_read_whole(void) {
    const char *what = "a well formed case";
    const char *object = "{\"name\":\"c\",\"params\":[[\"k\",1],[\"j\",0],\n [\"k\", 18446744073709551615 ]]}";
    table_pair_t pairs[MAX_PAIRS];
    size_t count = 0u;
    const char *refusal = table_read_params(object, pairs, MAX_PAIRS, &count);
    if (refusal != NULL) {
        (void)fprintf(stderr, "FAIL: %s: refused with `params %s`\n", what, refusal);
        return 1;
    }
    if (count != 3u) {
        (void)fprintf(stderr, "FAIL: %s: read %zu pair(s), expected 3\n", what, count);
        return 1;
    }
    int bad = 0;
    bad |= expect_pair("a well formed case, first pair", &pairs[0], "k", "\"k\"", 1u);
    bad |= expect_pair("a well formed case, second pair", &pairs[1], "j", "\"j\"", 0u);
    bad |= expect_pair("a well formed case, third pair", &pairs[2], "k", "\"k\"", UINT64_MAX);

    // An empty `params` is a case with no pairs, not a refusal.
    refusal = table_read_params("{\"params\":[]}", pairs, MAX_PAIRS, &count);
    if (refusal != NULL || count != 0u) {
        (void)fprintf(stderr, "FAIL: an empty params: refused with `%s` or counted %zu pair(s), expected none\n",
                      refusal != NULL ? refusal : "", count);
        bad = 1;
    }
    return bad;
}

// A value of a kind a pair does not hold: only an unsigned integer is a value.
static int a_value_that_is_not_an_unsigned_integer_is_refused(void) {
    int bad = 0;
    bad |= expect_refused("a string value", "{\"params\":[[\"a\",\"1\"]]}");
    bad |= expect_refused("a negative number", "{\"params\":[[\"a\",-1]]}");
    bad |= expect_refused("a fraction", "{\"params\":[[\"a\",1.5]]}");
    bad |= expect_refused("an object value", "{\"params\":[[\"a\",{\"b\":1}]]}");
    bad |= expect_refused("an array value", "{\"params\":[[\"a\",[1]]]}");
    bad |= expect_refused("a value that overflows uint64", "{\"params\":[[\"a\",18446744073709551616]]}");
    bad |= expect_refused("a pair with no value", "{\"params\":[[\"a\"]]}");
    return bad;
}

// A name the reader cannot use as a key: not a text, longer than it holds, with a
// NUL in it, or too wide to quote into the room of a key.
static int a_name_that_cannot_be_a_key_is_refused(void) {
    int bad = 0;
    bad |= expect_refused("a name that is not text", "{\"params\":[[1,2]]}");

    char built[BUILT_CAP];
    int n = snprintf(built, sizeof(built), "{\"params\":[[\"");
    for (int i = 0; i < UNHOLDABLE_NAME_LEN; ++i) {
        built[n++] = 'a';
    }
    n += snprintf(built + n, sizeof(built) - (size_t)n, "\",1]]}");
    bad |= expect_refused("a name longer than the reader holds", built);

    // A name of quotes, each one escaped by the writer of the key: it is held
    // (LONG_NAME_LEN bytes) and it does not fit its quoted form.
    n = snprintf(built, sizeof(built), "{\"params\":[[\"");
    for (int i = 0; i < LONG_NAME_LEN; ++i) {
        n += snprintf(built + n, sizeof(built) - (size_t)n, "\\\"");
    }
    n += snprintf(built + n, sizeof(built) - (size_t)n, "\",1]]}");
    bad |= expect_refused("a name too long to quote", built);

    bad |= expect_refused("a name with a NUL", "{\"params\":[[\"a\\u0000b\",1]]}");
    return bad;
}

// The shape of the pair and of the array it is in.
static int a_pair_or_params_of_the_wrong_shape_is_refused(void) {
    int bad = 0;
    bad |= expect_refused("a pair with three elements", "{\"params\":[[\"a\",1,2]]}");
    bad |= expect_refused("a pair that is not an array", "{\"params\":[\"a\"]}");
    bad |= expect_refused("a pair that is an object", "{\"params\":[{\"a\":1}]}");
    bad |= expect_refused("a pair that never closes", "{\"params\":[[\"a\",1");
    bad |= expect_refused("a params that is not an array", "{\"params\":{\"a\":1}}");
    bad |= expect_refused("a params that is a number", "{\"params\":7}");
    bad |= expect_refused("a missing params", "{\"name\":\"c\",\"data\":\"{}\"}");
    bad |= expect_refused("a params that never closes", "{\"params\":[[\"a\",1],");

    // More pairs than the caller made room for: refused, never written past `cap`.
    table_pair_t pairs[2];
    size_t count = 0u;
    const char *refusal = table_read_params("{\"params\":[[\"a\",1],[\"b\",2],[\"c\",3]]}", pairs,
                                            sizeof(pairs) / sizeof(pairs[0]), &count);
    if (refusal == NULL || count != 0u) {
        (void)fprintf(stderr, "FAIL: more pairs than the room for: read %zu pair(s) without a refusal\n", count);
        bad = 1;
    }
    return bad;
}

static const char TWO_CASES[] =
    "{\"cases\":[{\"name\":\"one\",\"params\":[]},{\"name\":\"tw\\u006f\",\"params\":[[\"a\",1]]}]}";

// A case is found by its unescaped name, or it is not found.
static int a_case_is_found_or_it_is_not(void) {
    int bad = 0;
    const char *absent = table_find_case(TWO_CASES, "three");
    if (absent != NULL) {
        (void)fprintf(stderr, "FAIL: an absent case name: found a case at offset %td\n", absent - TWO_CASES);
        bad = 1;
    }
    const char *prefix = table_find_case(TWO_CASES, "on");
    if (prefix != NULL) {
        (void)fprintf(stderr, "FAIL: a name that is a prefix of one: found a case at offset %td\n", prefix - TWO_CASES);
        bad = 1;
    }

    const char *first = table_find_case(TWO_CASES, "one");
    const char *first_want = strstr(TWO_CASES, "{\"name\":\"one\"");
    if (first == NULL || first != first_want) {
        (void)fprintf(stderr, "FAIL: a present case name (the first): not found at its object\n");
        bad = 1;
    }
    // The second is written with an escape, and found by what it spells.
    const char *second = table_find_case(TWO_CASES, "two");
    const char *second_want = strstr(TWO_CASES, "{\"name\":\"tw");
    if (second == NULL || second != second_want) {
        (void)fprintf(stderr,
                      "FAIL: a present case name (the second, written with an escape): not found at its object\n");
        bad = 1;
    }

    if (table_find_case("{\"name\":\"table without cases\"}", "one") != NULL) {
        (void)fprintf(stderr, "FAIL: a table with no cases: found a case\n");
        bad = 1;
    }
    if (table_find_case("{\"cases\":[7]}", "one") != NULL) {
        (void)fprintf(stderr, "FAIL: a case that is not an object: found a case\n");
        bad = 1;
    }
    return bad;
}

static int never_a_case(const char *object, int index) {
    (void)object;
    (void)index;
    (void)fprintf(stderr, "FAIL: a table that cannot be read handed a case to the check\n");
    return 1;
}

// A table that cannot be read is -1, not a table with no cases: the reader says so
// on stderr and gives the check no case. The message it prints is the reader's.
static int a_table_that_cannot_be_read_is_minus_one(void) {
    int failures = 0;
    const int cases = walk_table("/nonexistent/sce_shared_table_refusals/no_such_table.json", never_a_case, &failures);
    int bad = 0;
    if (cases != -1) {
        (void)fprintf(stderr, "FAIL: a path that does not exist: walk_table answered %d, expected -1\n", cases);
        bad = 1;
    }
    if (failures != 0) {
        (void)fprintf(stderr, "FAIL: a path that does not exist: walk_table counted %d failure(s) of cases\n",
                      failures);
        bad = 1;
    }
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_well_formed_case_is_read_whole();
    bad |= a_value_that_is_not_an_unsigned_integer_is_refused();
    bad |= a_name_that_cannot_be_a_key_is_refused();
    bad |= a_pair_or_params_of_the_wrong_shape_is_refused();
    bad |= a_case_is_found_or_it_is_not();
    bad |= a_table_that_cannot_be_read_is_minus_one();
    if (bad != 0) {
        return 1;
    }
    (void)printf("shared table refusals: ok\n");
    return 0;
}
