/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file from_canonical.h
 * @brief A value read back from the canonical form of the test log
 *        (docs/SCE_TEST_PROFILE.md §12.5; docs/SCE_TTCN3_LANE.md §10.3): the
 *        inverse of sce/test/canonical.h, which a recorded input trace's values
 *        are read with (`schemas/sce-test-trace.v1.schema.json`)
 *
 * Each reader takes one form and accepts its one spelling, so a value read here
 * and written back by sce/test/canonical.h is the same bytes:
 *
 *   - an integer is the text of its decimal digits, with no sign but a minus,
 *     no leading zero and no `-0`, within the range of the type it is read as;
 *   - a `bool` is a truth value;
 *   - a value of an enum is the name its document declares for the variant, or,
 *     for an enum of an open set, the text of a number no variant declares;
 *   - a text is a text of at most its bound in UTF-8 bytes;
 *   - a byte string is its octets in upper-case hex, two digits each;
 *   - a record is an object of its fields by their declared ids, an absent field
 *     omitted, no field it does not declare;
 *   - a list is an array of its elements within its bounds, an absent element of
 *     a list of `optional<T>` `null`.
 *
 * A value is a span of a text `sce_json_parse` judged (`sce_jv_t`,
 * sce/saved_json.h). One that does not belong to its type is refused, never
 * converted: the reader answers false and says why in `*err`, naming where the
 * value stands (`what`, then `: ` and the reason), in the words of the Rust
 * reader `sce_rust_runtime::test::from_canonical`, word for word. Which form a
 * value takes is decided by the types its document declares and not by its C
 * type, since a byte string and a list of `uint8` are both arrays of octets in
 * C: the generated reader of a record (`<configuration>_read_<record>` in the
 * unit of a test configuration) calls these by the walk its canonical writer is
 * spelled by (`forge::generator::test_template_walk::canonical_form`), and
 * writes each value read into the place the record's unit holds it in. The C11
 * port of `sce/include/test/FromCanonical.h`. C11's test profile matches no
 * union yet (`codegen_matrix::TEST_PROFILE_UNION_BACKENDS`), so no form of one
 * is read here.
 *
 * What is C's own: a value is read into storage the caller places, and a text
 * into the buffer of its bound, which ends at its first U+0000, so a text that
 * holds one is refused rather than cut. A text's UTF-8 is held to the one check
 * the forge runtime has (`sce_forge_is_valid_utf8`, sce/forge/codec.h), so a host
 * of this header has `backends/c/forge-runtime/include` on its include path, as
 * every host of a generated record has. An object that names a member twice is
 * refused where its text is judged (`sce_json_parse`), before any reader sees
 * it. Header-only and allocation-free.
 */

#ifndef SCE_TEST_FROM_CANONICAL_H
#define SCE_TEST_FROM_CANONICAL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/saved_json.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * An integer as the canonical form spells one: its sign and its magnitude.
 *
 * The text is read within the range of a 128-bit integer, the range the Rust
 * reader reads an integer's text in before it holds it to its type, so a text of
 * more digits is refused as no integer's on both. `wide` is set where the
 * magnitude is past what 64 bits hold, which no type of the profile holds.
 */
typedef struct sce_test_decimal_s {
    bool negative;
    bool wide;
    uint64_t magnitude;
} sce_test_decimal_t;

/* The magnitudes of the least and of the greatest 128-bit integer. */
#define SCE_TEST_LEAST_MAGNITUDE_ "170141183460469231731687303715884105728"
#define SCE_TEST_GREATEST_MAGNITUDE_ "170141183460469231731687303715884105727"

/**
 * The integer the `n` bytes at `digits` spell as the canonical form writes one,
 * in `*out`: an optional minus and decimal digits with no leading zero, never
 * `-0`, within the range of a 128-bit integer. False for any other text.
 */
static inline bool sce_test_canonical_decimal(const char *digits, size_t n, sce_test_decimal_t *out) {
    size_t at = 0u;
    out->negative = false;
    out->wide = false;
    out->magnitude = 0u;
    if (n > 0u && digits[0] == '-') {
        out->negative = true;
        at = 1u;
    }
    const char *magnitude = digits + at;
    const size_t count = n - at;
    if (count == 0u) {
        return false;
    }
    for (size_t i = 0u; i < count; ++i) {
        if (magnitude[i] < '0' || magnitude[i] > '9') {
            return false;
        }
    }
    if (count > 1u && magnitude[0] == '0') {
        return false;
    }
    if (out->negative && magnitude[0] == '0') {
        return false;
    }
    const char *limit = out->negative ? SCE_TEST_LEAST_MAGNITUDE_ : SCE_TEST_GREATEST_MAGNITUDE_;
    const size_t limit_len = strlen(limit);
    if (count > limit_len || (count == limit_len && memcmp(magnitude, limit, count) > 0)) {
        return false;
    }
    for (size_t i = 0u; i < count && !out->wide; ++i) {
        const uint64_t digit = (uint64_t)(magnitude[i] - '0');
        if (out->magnitude > (UINT64_MAX - digit) / 10u) {
            out->wide = true;
        } else {
            out->magnitude = out->magnitude * 10u + digit;
        }
    }
    return true;
}

/** `*number` as a value of an unsigned type whose greatest value is `most`, in
 *  `*out`; false where it is past that range. */
static inline bool sce_test_decimal_unsigned(const sce_test_decimal_t *number, uint64_t most, uint64_t *out) {
    if (number->negative || number->wide || number->magnitude > most) {
        return false;
    }
    *out = number->magnitude;
    return true;
}

/** `*number` as a value of a signed type of the range `least` to `most`, in
 *  `*out`; false where it is past that range. `least` is at most 0 and `most`
 *  at least 0. */
static inline bool sce_test_decimal_signed(const sce_test_decimal_t *number, int64_t least, int64_t most,
                                           int64_t *out) {
    if (number->wide) {
        return false;
    }
    if (number->negative) {
        if (least >= 0) {
            return false;
        }
        /* The magnitude of `least`, held without negating it, which overflows
           for INT64_MIN. */
        const uint64_t most_magnitude = (uint64_t)(-(least + 1)) + 1u;
        if (number->magnitude > most_magnitude) {
            return false;
        }
        *out = number->magnitude == most_magnitude ? least : -(int64_t)number->magnitude;
        return true;
    }
    if (most < 0 || number->magnitude > (uint64_t)most) {
        return false;
    }
    *out = (int64_t)number->magnitude;
    return true;
}

/* The kind of a JSON value, as a refusal names it. */
static inline const char *sce_test_canonical_kind_(sce_jv_t value) {
    switch (sce_jv_kind(value)) {
    case SCE_JSON_NULL:
        return "null";
    case SCE_JSON_BOOL:
        return "a truth value";
    case SCE_JSON_NUMBER:
        return "a number";
    case SCE_JSON_STRING:
        return "a text";
    case SCE_JSON_ARRAY:
        return "an array";
    case SCE_JSON_OBJECT:
        return "an object";
    }
    return "an object";
}

/* The refusal of `value`, at `what`, which is not of the form `form`. */
static inline bool sce_test_canonical_not_a_(sce_jv_t value, const char *what, const char *form,
                                             sce_saved_error_t *err) {
    return sce_saved_refuse(err, "%s: %s is not %s", what, sce_test_canonical_kind_(value), form);
}

/* The text `value` spells, decoded into the `cap` bytes at `out` and cut where it
   is longer, in `*len`: what a refusal quotes, and what an integer's digits are
   read from, which no text past the buffer holds. */
static inline void sce_test_canonical_spelled_(sce_jv_t value, char *out, size_t cap, size_t *len) {
    size_t n = 0u;
    (void)sce_jv_string(value, out, cap, &n);
    if (len != NULL) {
        *len = n;
    }
}

/**
 * The integer the `n` bytes at `digits` spell, a text the caller ends with a
 * terminator, as a value of an unsigned type whose greatest value is `most`, in
 * `*out`; refused, naming `what`, where the text is not the canonical spelling
 * of an integer or the integer is past the range. The read of an integer's text
 * that a value of the canonical form and a module parameter's literal share.
 */
static inline bool sce_test_integer_text_unsigned(const char *digits, size_t n, uint64_t most, const char *what,
                                                  uint64_t *out, sce_saved_error_t *err) {
    sce_test_decimal_t number;
    if (!sce_test_canonical_decimal(digits, n, &number)) {
        return sce_saved_refuse(err, "%s: '%s' is not the text of an integer's decimal digits", what, digits);
    }
    if (!sce_test_decimal_unsigned(&number, most, out)) {
        return sce_saved_refuse(err, "%s: %s is past the range of its type", what, digits);
    }
    return true;
}

/** The integer the text `digits` spells as a value of a signed type of the range
 *  `least` to `most`, in `*out`: `sce_test_integer_text_unsigned` of a signed
 *  type. */
static inline bool sce_test_integer_text_signed(const char *digits, size_t n, int64_t least, int64_t most,
                                                const char *what, int64_t *out, sce_saved_error_t *err) {
    sce_test_decimal_t number;
    if (!sce_test_canonical_decimal(digits, n, &number)) {
        return sce_saved_refuse(err, "%s: '%s' is not the text of an integer's decimal digits", what, digits);
    }
    if (!sce_test_decimal_signed(&number, least, most, out)) {
        return sce_saved_refuse(err, "%s: %s is past the range of its type", what, digits);
    }
    return true;
}

/** An integer of an unsigned type whose greatest value is `most`, in `*out`: the
 *  text of its decimal digits. */
static inline bool sce_test_from_canonical_unsigned(sce_jv_t value, uint64_t most, const char *what, uint64_t *out,
                                                    sce_saved_error_t *err) {
    char digits[SCE_SAVED_ERROR_LEN];
    size_t n = 0u;
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_test_canonical_not_a_(value, what, "an integer, which is the text of its decimal digits", err);
    }
    sce_test_canonical_spelled_(value, digits, sizeof(digits), &n);
    return sce_test_integer_text_unsigned(digits, n, most, what, out, err);
}

/** An integer of a signed type of the range `least` to `most`, in `*out`: the
 *  text of its decimal digits. */
static inline bool sce_test_from_canonical_signed(sce_jv_t value, int64_t least, int64_t most, const char *what,
                                                  int64_t *out, sce_saved_error_t *err) {
    char digits[SCE_SAVED_ERROR_LEN];
    size_t n = 0u;
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_test_canonical_not_a_(value, what, "an integer, which is the text of its decimal digits", err);
    }
    sce_test_canonical_spelled_(value, digits, sizeof(digits), &n);
    return sce_test_integer_text_signed(digits, n, least, most, what, out, err);
}

/** A `bool`, in `*out`. */
static inline bool sce_test_from_canonical_boolean(sce_jv_t value, const char *what, bool *out,
                                                   sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_BOOL) {
        return sce_test_canonical_not_a_(value, what, "a truth value", err);
    }
    *out = sce_jv_as_bool(value);
    return true;
}

/**
 * A text of at most `max` bytes of UTF-8, the bound its type declares, into the
 * `cap` bytes at `out`, the buffer of that bound, terminated.
 */
static inline bool sce_test_from_canonical_text(sce_jv_t value, size_t max, const char *what, char *out, size_t cap,
                                                sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_test_canonical_not_a_(value, what, "a text", err);
    }
    const size_t count = sce_jv_string_len(value);
    if (count > max) {
        return sce_saved_refuse(err, "%s: a text of %lu bytes is past the %lu its type holds", what,
                                (unsigned long)count, (unsigned long)max);
    }
    if (count >= cap) {
        return sce_saved_refuse(err, "%s: a text of %lu bytes is past what its type holds", what, (unsigned long)count);
    }
    size_t n = 0u;
    (void)sce_jv_string(value, out, cap, &n);
    if (!sce_forge_is_valid_utf8((const uint8_t *)out, n)) {
        return sce_saved_refuse(err, "%s: a text of %lu bytes is not well-formed UTF-8", what, (unsigned long)n);
    }
    if (memchr(out, '\0', n) != NULL) {
        return sce_saved_refuse(err, "%s: a text of %lu bytes holds U+0000, at which a C text of its type ends", what,
                                (unsigned long)n);
    }
    return true;
}

/* The value of one upper-case hex digit, or -1. */
static inline int sce_test_canonical_hex_digit_(unsigned char digit) {
    if (digit >= '0' && digit <= '9') {
        return digit - '0';
    }
    if (digit >= 'A' && digit <= 'F') {
        return digit - 'A' + 10;
    }
    return -1;
}

/**
 * A byte string of at most `max` octets, into the `cap` bytes at `out`, its
 * count in `*len`: the text of its octets in upper-case hex, two digits each.
 */
static inline bool sce_test_from_canonical_bytes(sce_jv_t value, size_t max, const char *what, uint8_t *out, size_t cap,
                                                 size_t *len, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_test_canonical_not_a_(value, what, "a byte string, which is a text of hex", err);
    }
    sce_jstr_t hex;
    unsigned char high = 0u;
    unsigned char low = 0u;
    size_t octets = 0u;
    bool upper = true;
    sce_jstr_init(&hex, value);
    while (sce_jstr_next(&hex, &high)) {
        if (!sce_jstr_next(&hex, &low)) {
            upper = false;
            break;
        }
        const int high_value = sce_test_canonical_hex_digit_(high);
        const int low_value = sce_test_canonical_hex_digit_(low);
        if (high_value < 0 || low_value < 0) {
            upper = false;
            break;
        }
        if (octets < cap) {
            out[octets] = (uint8_t)((high_value << 4) | low_value);
        }
        ++octets;
    }
    if (!upper) {
        char spelled[SCE_SAVED_ERROR_LEN];
        sce_test_canonical_spelled_(value, spelled, sizeof(spelled), NULL);
        return sce_saved_refuse(err, "%s: '%s' is not octets in upper-case hex, two digits each", what, spelled);
    }
    if (octets > max) {
        return sce_saved_refuse(err, "%s: a byte string of %lu octets is past the %lu its type holds", what,
                                (unsigned long)octets, (unsigned long)max);
    }
    if (octets > cap) {
        return sce_saved_refuse(err, "%s: a byte string of %lu octets is past what its type holds", what,
                                (unsigned long)octets);
    }
    *len = octets;
    return true;
}

/**
 * A value of an enum of a closed set: the index in `names` of the variant whose
 * declared name the text is, in `*index`. `names` holds the `count` names its
 * document declares, in the order of its variants.
 */
static inline bool sce_test_from_canonical_enumerated(sce_jv_t value, const char *const *names, size_t count,
                                                      const char *what, size_t *index, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_test_canonical_not_a_(value, what, "a value of an enum, which is its name", err);
    }
    for (size_t i = 0u; i < count; ++i) {
        if (sce_jv_string_is(value, names[i])) {
            *index = i;
            return true;
        }
    }
    char spelled[SCE_SAVED_ERROR_LEN];
    sce_test_canonical_spelled_(value, spelled, sizeof(spelled), NULL);
    return sce_saved_refuse(err, "%s: '%s' names no variant of its enum", what, spelled);
}

/**
 * A value of an enum of an open set (`sce:strict-variants="false"`) whose
 * carrier is unsigned, its greatest value `most`: the number of the variant
 * whose declared name the text is, or the number the text spells where no
 * variant declares it, in `*out`. `names` and `numbers` hold the `count`
 * variants its document declares. A declared number is written by its
 * variant's name, so its digits are refused as the second spelling of one
 * value.
 */
static inline bool sce_test_from_canonical_open_unsigned(sce_jv_t value, const char *const *names,
                                                         const uint64_t *numbers, size_t count, uint64_t most,
                                                         const char *what, uint64_t *out, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_test_canonical_not_a_(value, what, "a value of an enum, which is its name", err);
    }
    for (size_t i = 0u; i < count; ++i) {
        if (sce_jv_string_is(value, names[i])) {
            *out = numbers[i];
            return true;
        }
    }
    char spelled[SCE_SAVED_ERROR_LEN];
    size_t n = 0u;
    sce_test_decimal_t number;
    uint64_t raw = 0u;
    sce_test_canonical_spelled_(value, spelled, sizeof(spelled), &n);
    if (!sce_test_canonical_decimal(spelled, n, &number)) {
        return sce_saved_refuse(err, "%s: '%s' names no variant of its enum, and is no number", what, spelled);
    }
    if (!sce_test_decimal_unsigned(&number, most, &raw)) {
        return sce_saved_refuse(err, "%s: %s is past the range of its enum", what, spelled);
    }
    for (size_t i = 0u; i < count; ++i) {
        if (numbers[i] == raw) {
            return sce_saved_refuse(err, "%s: %s is the number of a declared variant, which is written by its name",
                                    what, spelled);
        }
    }
    *out = raw;
    return true;
}

/** `sce_test_from_canonical_open_unsigned` of an enum whose carrier is signed, of
 *  the range `least` to `most`. */
static inline bool sce_test_from_canonical_open_signed(sce_jv_t value, const char *const *names, const int64_t *numbers,
                                                       size_t count, int64_t least, int64_t most, const char *what,
                                                       int64_t *out, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_test_canonical_not_a_(value, what, "a value of an enum, which is its name", err);
    }
    for (size_t i = 0u; i < count; ++i) {
        if (sce_jv_string_is(value, names[i])) {
            *out = numbers[i];
            return true;
        }
    }
    char spelled[SCE_SAVED_ERROR_LEN];
    size_t n = 0u;
    sce_test_decimal_t number;
    int64_t raw = 0;
    sce_test_canonical_spelled_(value, spelled, sizeof(spelled), &n);
    if (!sce_test_canonical_decimal(spelled, n, &number)) {
        return sce_saved_refuse(err, "%s: '%s' names no variant of its enum, and is no number", what, spelled);
    }
    if (!sce_test_decimal_signed(&number, least, most, &raw)) {
        return sce_saved_refuse(err, "%s: %s is past the range of its enum", what, spelled);
    }
    for (size_t i = 0u; i < count; ++i) {
        if (numbers[i] == raw) {
            return sce_saved_refuse(err, "%s: %s is the number of a declared variant, which is written by its name",
                                    what, spelled);
        }
    }
    *out = raw;
    return true;
}

/**
 * A record whose fields are the `count` ids `fields`: an object, each member of
 * which names one of them. Its fields are read by `sce_test_from_canonical_field`
 * and `sce_test_from_canonical_optional_field`.
 */
static inline bool sce_test_from_canonical_record(sce_jv_t value, const char *const *fields, size_t count,
                                                  const char *what, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_OBJECT) {
        return sce_test_canonical_not_a_(value, what, "a record, which is an object of its fields", err);
    }
    sce_jiter_t members;
    sce_jv_t key;
    sce_jv_t member;
    (void)sce_jv_iter(value, &members);
    while (sce_jiter_member(&members, &key, &member)) {
        bool declared = false;
        for (size_t i = 0u; i < count && !declared; ++i) {
            declared = sce_jv_string_is(key, fields[i]);
        }
        if (!declared) {
            char name[SCE_SAVED_ERROR_LEN];
            sce_test_canonical_spelled_(key, name, sizeof(name), NULL);
            return sce_saved_refuse(err, "%s: '%s' is no field of its record", what, name);
        }
    }
    return true;
}

/** The field `id` of the record `record`, whose type is not optional, so it is
 *  present, in `*out`. `what` names the field. */
static inline bool sce_test_from_canonical_field(sce_jv_t record, const char *id, const char *what, sce_jv_t *out,
                                                 sce_saved_error_t *err) {
    if (!sce_jv_get(record, id, out)) {
        return sce_saved_refuse(err, "%s: the field is absent, and its type is not optional", what);
    }
    return true;
}

/** The field `id` of the record `record`, whose type is optional, in `*out`:
 *  false where it is absent, which the canonical form writes by omitting it. */
static inline bool sce_test_from_canonical_optional_field(sce_jv_t record, const char *id, sce_jv_t *out) {
    return sce_jv_get(record, id, out);
}

/** The elements of a list of at least `min` and at most `max` elements, as the
 *  walk of its array in `*items`, which `sce_jiter_element` steps. */
static inline bool sce_test_from_canonical_elements(sce_jv_t value, size_t min, size_t max, const char *what,
                                                    sce_jiter_t *items, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_ARRAY) {
        return sce_test_canonical_not_a_(value, what, "a list, which is an array", err);
    }
    const size_t count = sce_jv_count(value);
    if (count < min || count > max) {
        return sce_saved_refuse(err, "%s: a list of %lu elements is outside its bounds, %lu to %lu", what,
                                (unsigned long)count, (unsigned long)min, (unsigned long)max);
    }
    (void)sce_jv_iter(value, items);
    return true;
}

/** Whether an element of a list of `optional<T>` holds a value: not JSON `null`,
 *  which the canonical form writes in the place of an absent element. */
static inline bool sce_test_from_canonical_present(sce_jv_t value) {
    return !sce_jv_is_null(value);
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_FROM_CANONICAL_H */
