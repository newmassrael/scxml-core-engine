/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file canonical.h
 * @brief The canonical form of a message's value in the test log
 *        (docs/SCE_TEST_PROFILE.md §12.5), and the record type a message
 *        carries it by
 *
 * One JSON value per value of the profile's types, built from texts, truth
 * values, arrays and objects alone, so that the JSON Canonicalization Scheme
 * (`sce_jcs_t`, sce/saved_json.h) writes two equal values as the same bytes on
 * every runtime:
 *
 *   - an integer of any width is the text of its decimal digits, since an
 *     `int64` or a `uint64` is past what a JSON reader that holds numbers as
 *     doubles reads exactly;
 *   - a `bool` is a truth value;
 *   - a value of an enum is the name its document declares for its variant;
 *   - a text is a text;
 *   - a byte string is the text of its octets in upper-case hex, two digits
 *     each;
 *   - a record is an object of its fields by their declared ids, a field that
 *     is absent omitted, as TTCN-3's `omit` leaves it out (a member named with
 *     `sce_jcs_key` that no value follows);
 *   - a list is an array of its elements in order, an element of
 *     `optional<T>` that is absent `null`, the one value no other form takes
 *     (`sce_test_canonical_absent_element`).
 *
 * The generated unit of a test template over a record writes its
 * `<name>_canonical` with these, by the types its document declares
 * (`forge/c/test_template.h.jinja2`): a C type alone does not say which form a
 * value takes. A byte string, of one entry or inside a list, is held with its
 * length, which `sce_test_canonical_bytes` is given. C11 lowers no union, so no
 * form of one is written here. The C11 port of `sce/include/test/Canonical.h` and
 * `sce_rust_runtime::test::canonical`. Header-only and allocation-free.
 */

#ifndef SCE_TEST_CANONICAL_H
#define SCE_TEST_CANONICAL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "sce/saved_json.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Writes the value at `value`, of the type the writer is for, in its
 *  canonical form. */
typedef void (*sce_test_canonical_fn)(sce_jcs_t *jcs, const void *value);

/**
 * The record type of a message in the test log: its name and the writer of its
 * value's canonical form.
 *
 * `name` is the record's name as the types module of the suite's TTCN-3 export
 * names it (docs/SCE_TEST_PROFILE.md §9.5): the unit name of its `event-schema`
 * document. A value in TCI always carries its type, and so does a message of
 * the test log. The unit of a template over the record gives one
 * (`<name>_record_type`), of static storage, and a send carries it beside the
 * value (`sce_test_message_value_t`, sce/test/port.h); the C11 form of Rust's
 * `MessageValue` and C++'s `SCE::Test::MessageValue`.
 */
typedef struct sce_test_record_type_s {
    const char *name;
    sce_test_canonical_fn canonical;
} sce_test_record_type_t;

/** A `bool`. */
static inline void sce_test_canonical_boolean(sce_jcs_t *jcs, bool value) {
    sce_jcs_bool(jcs, value);
}

/** A signed integer: the text of its decimal digits. */
static inline void sce_test_canonical_signed(sce_jcs_t *jcs, int64_t value) {
    char digits[24];
    const int n = snprintf(digits, sizeof(digits), "%lld", (long long)value);
    sce_jcs_string_n(jcs, digits, n > 0 ? (size_t)n : 0u);
}

/** An unsigned integer: the text of its decimal digits. */
static inline void sce_test_canonical_unsigned(sce_jcs_t *jcs, uint64_t value) {
    char digits[24];
    const int n = snprintf(digits, sizeof(digits), "%llu", (unsigned long long)value);
    sce_jcs_string_n(jcs, digits, n > 0 ? (size_t)n : 0u);
}

/* An integer read as both: unsigned where it is positive, which holds every
   positive value of every width, and signed where it is not. */
static inline void sce_test_canonical_integer_(sce_jcs_t *jcs, bool positive, uint64_t as_unsigned, int64_t as_signed) {
    if (positive) {
        sce_test_canonical_unsigned(jcs, as_unsigned);
    } else {
        sce_test_canonical_signed(jcs, as_signed);
    }
}

/** An integer of any width the profile has, signed or not: the text of its
 *  decimal digits. A macro, since C has no overload by the width and the
 *  signedness of a field; `value` is read three times, so it is a place, never
 *  an expression with an effect. */
#define SCE_TEST_CANONICAL_INTEGER(jcs, value)                                                                         \
    sce_test_canonical_integer_((jcs), (value) > 0, (uint64_t)(value), (int64_t)(value))

/** A text. */
static inline void sce_test_canonical_text(sce_jcs_t *jcs, const char *value) {
    sce_jcs_string(jcs, value);
}

/** An element of a list of `optional<T>` that holds no value: `null` in its
 *  place, where a present one is written as its value. An element is not
 *  omitted as an absent field is, since the array names it by its position,
 *  which leaving it out would move for every element after it. The TTCN-3
 *  export refuses a list of `optional<T>` (rule E1, docs/SCE_TEST_PROFILE.md
 *  §6.3), so only a native log holds this form. */
static inline void sce_test_canonical_absent_element(sce_jcs_t *jcs) {
    sce_jcs_null(jcs);
}

/** A byte string: its `len` octets in upper-case hex, two digits each, as
 *  TTCN-3 writes an `octetstring` (`'00FF'O`) without its quotes and its `O`.
 *  Hex digits need no escape, so the text is written as it is built. */
static inline void sce_test_canonical_bytes(sce_jcs_t *jcs, const uint8_t *octets, size_t len) {
    static const char hex[] = "0123456789ABCDEF";
    sce_jcs_place_(jcs);
    sce_jw_put_(&jcs->w, "\"", 1u);
    for (size_t i = 0u; i < len; ++i) {
        char pair[2];
        pair[0] = hex[octets[i] >> 4];
        pair[1] = hex[octets[i] & 0x0Fu];
        sce_jw_put_(&jcs->w, pair, 2u);
    }
    sce_jw_put_(&jcs->w, "\"", 1u);
}

/** A value of a closed enum: `declared`, the name its document declares for the
 *  variant (`<enum>_declared_name`). A closed enum holds only its declared
 *  numbers, so a value no variant names (`declared` NULL) is not a value of its
 *  type, and refuses the text rather than being written. */
static inline void sce_test_canonical_enumerated(sce_jcs_t *jcs, const char *declared) {
    if (declared == NULL) {
        sce_jcs_refuse(jcs, "a value of a closed enum that no variant declares has no canonical form");
        return;
    }
    sce_jcs_string(jcs, declared);
}

/* A value of an enum of an open set: its declared name, or its number. */
static inline void sce_test_canonical_open_enumerated_(sce_jcs_t *jcs, const char *declared, bool positive,
                                                       uint64_t as_unsigned, int64_t as_signed) {
    if (declared != NULL) {
        sce_jcs_string(jcs, declared);
    } else {
        sce_test_canonical_integer_(jcs, positive, as_unsigned, as_signed);
    }
}

/** A value of an enum of an open set (`sce:strict-variants="false"`): the name
 *  its document declares for it, `declared`, or, for a number no variant
 *  declares (`declared` NULL), the text of that number's decimal digits. No
 *  variant's name is such a text, since a name is an identifier. The TTCN-3
 *  export refuses an enum of an open set (docs/SCE_TEST_PROFILE.md §9.5), so
 *  only a native log holds this form. `value` is read three times. */
#define SCE_TEST_CANONICAL_OPEN_ENUMERATED(jcs, declared, value)                                                       \
    sce_test_canonical_open_enumerated_((jcs), (declared), (value) > 0, (uint64_t)(value), (int64_t)(value))

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_CANONICAL_H */
