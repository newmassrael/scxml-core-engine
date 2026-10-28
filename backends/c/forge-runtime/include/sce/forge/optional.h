/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * A value that may be absent (docs/adr/0055): the C11 form of `optional<T>`.
 *
 * C has no optional type, and a pointer would need storage outside the value,
 * which a machine without an allocator does not have. So an optional value is one
 * struct of two parts, whether it is present and the value it is when it is:
 *
 *     typedef struct { bool present; T value; } sce_optional_<elem>_t;
 *
 * It is one value: an assignment, a parameter and a return carry both parts
 * together. It is built only by its two constructors, `sce_optional_<elem>_none()`
 * and `sce_optional_<elem>_some(v)`, and an absent one is not present and holds a
 * zero `value`, so two absent values are the same value (the padding between the
 * members is not part of it, and a compiler may leave it as it was). Generated code reads
 * `value` only where the build has proven the value present, and through
 * `sce_optional_<elem>_eq`, which compares two optional values as `===` does: equal
 * when both are absent, or both present and their values equal.
 *
 * One macro declares the type, its constructors and its `_eq` for one element
 * type, so each is spelled once:
 *
 *   - the numbers and `bool`, below, named by the type the document writes
 *     (`sce_optional_int64_t`, `sce_optional_float64_t`, `sce_optional_bool_t`);
 *   - a text of bound N, `SCE_OPTIONAL_DECLARE_STRING(N)`, written by the
 *     generator beside `sce_static_string_<N>_t` under the guard the two share
 *     (`sce_optional_string_<N>_t`);
 *   - a byte string of bound N, `SCE_OPTIONAL_DECLARE_BYTES(N)`, beside
 *     `sce_static_bytes_<N>_t` (`sce_optional_bytes_<N>_t`);
 *   - a value of an enum, `SCE_OPTIONAL_DECLARE(enum_<document>, <type>,
 *     SCE_OPTIONAL_SAME_VALUE)`, under the enum document's guard
 *     (`sce_optional_enum_<document>_t`);
 *   - a record, `SCE_OPTIONAL_DECLARE_RECORD(record_<type>, <type>)`, beside the
 *     struct it wraps and under a guard of its own (`sce_optional_record_<type>_t`),
 *     with no `_eq`: a record is read through a proven `.value` and written a
 *     field at a time, and two of them are no operand of `===`.
 *
 * A generated CBOR or content-line codec holds an entry or a parameter that may
 * be absent in the same struct, declared by its header under the same guards as a
 * machine's, so a value read from a codec is the value a statechart's variable of
 * that type holds.
 *
 * The read with a default, `x ?? d`, is a conditional over the flag, so `d` is
 * evaluated only when `x` is absent: a default that would fail fails only then
 * (`SCE_OPTIONAL_VALUE_OR` and its siblings). C11 has no portable way to name a
 * value inside an expression (a statement expression is a GNU extension), so `x`
 * is read twice, once for its flag and once for its value; a forge expression
 * writes nothing, so the two reads are the one value.
 *
 * Equality is read the same way: `SCE_OPTIONAL_SAME(a, b, same)` of two optional
 * values, the comparison every `_eq` makes, and `SCE_OPTIONAL_HOLDS(x, v)` of an
 * optional value and a present one, which needs no type of its own.
 *
 * Header-only, with no allocator: freestanding `<stdbool.h>`, `<stddef.h>`,
 * `<stdint.h>` and `<string.h>` only, and the runtime's byte view.
 */
#ifndef SCE_FORGE_OPTIONAL_H
#define SCE_FORGE_OPTIONAL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sce/forge/bytes.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Two present values of a number, a `bool` or an enum are the same value when `==`
 * says so, which is `===` of each of them (a real that is not a number equals no
 * value, itself included). */
#define SCE_OPTIONAL_SAME_VALUE(a, b) ((a) == (b))

/* Two present texts, each a buffer of its bound, are the same value when they hold
 * the same bytes up to their terminator. */
#define SCE_OPTIONAL_SAME_STRING(a, b) (strcmp((a).data, (b).data) == 0)

/* Two present byte strings, each a buffer and the length it holds, are the same
 * value when they hold the same bytes. */
#define SCE_OPTIONAL_SAME_BYTES(a, b) ((a).len == (b).len && memcmp((a).data, (b).data, (a).len) == 0)

/* `a === b` of two optional values: both absent, or both present and the same
 * value by `same`. */
#define SCE_OPTIONAL_SAME(a, b, same) ((a).present == (b).present && (!(a).present || same((a).value, (b).value)))

/* `x === v` of an optional value and a present value of a number, a `bool` or an
 * enum: `x` present and holding `v`. An absent value equals no present one. */
#define SCE_OPTIONAL_HOLDS(x, v) ((x).present && (x).value == (v))

/* `x === t` of an optional text and a present text. */
#define SCE_OPTIONAL_HOLDS_TEXT(x, t) ((x).present && strcmp((x).value.data, (t)) == 0)

/* The optional value of the type `T`, named `<prefix>_t`, with its two constructors,
 * `<prefix>_none()` and `<prefix>_some(v)`, and no comparison: what every optional
 * type is made of, and the whole of a record's, which is no operand of `===`.
 *
 * It takes the name already joined (`sce_optional_int64`), and not the element: a
 * macro that passed the element on would expand it first, and `bool` is a macro
 * (`<stdbool.h>`) that must stay the word the element is named by. */
#define SCE_OPTIONAL_DECLARE_NAMED(prefix, T)                                                                          \
    typedef struct {                                                                                                   \
        bool present;                                                                                                  \
        T value;                                                                                                       \
    } prefix##_t;                                                                                                      \
                                                                                                                       \
    /* An absent value: not present, and its value zero. The members are what the type says; the                       \
     * padding between them is not part of the value, and a compiler may leave it as it was. */                        \
    static inline prefix##_t prefix##_none(void) {                                                                     \
        prefix##_t absent;                                                                                             \
        memset(&absent, 0, sizeof(absent));                                                                            \
        return absent;                                                                                                 \
    }                                                                                                                  \
                                                                                                                       \
    /* A present value, `value`. */                                                                                    \
    static inline prefix##_t prefix##_some(T value) {                                                                  \
        prefix##_t held = prefix##_none();                                                                             \
        held.present = true;                                                                                           \
        held.value = value;                                                                                            \
        return held;                                                                                                   \
    }

/* The optional value of the element type `T`, named `sce_optional_<elem>_t`, with
 * its two constructors and its `_eq`; `same(a, b)` answers whether two present
 * values of `T` are the same value. */
#define SCE_OPTIONAL_DECLARE(elem, T, same)                                                                            \
    SCE_OPTIONAL_DECLARE_NAMED(sce_optional_##elem, T)                                                                 \
                                                                                                                       \
    /* `a === b`: both absent, or both present and the same value. */                                                  \
    static inline bool sce_optional_##elem##_eq(sce_optional_##elem##_t a, sce_optional_##elem##_t b) {                \
        return SCE_OPTIONAL_SAME(a, b, same);                                                                          \
    }

/* A record of the struct type `T` that may be absent, named `sce_optional_<elem>_t`,
 * with its two constructors. A record is read through a proven `.value` and written
 * a field at a time (`x.value.f`), and two of them are no operand of `===`, so the
 * type has no `_eq`. */
#define SCE_OPTIONAL_DECLARE_RECORD(elem, T) SCE_OPTIONAL_DECLARE_NAMED(sce_optional_##elem, T)

/* A text of at most `capacity` UTF-8 bytes that may be absent, over the buffer
 * `sce_static_string_<capacity>_t` the machine declares first. */
#define SCE_OPTIONAL_DECLARE_STRING(capacity)                                                                          \
    SCE_OPTIONAL_DECLARE(string_##capacity, sce_static_string_##capacity##_t, SCE_OPTIONAL_SAME_STRING)

/* A byte string of at most `capacity` bytes that may be absent, over the buffer
 * `sce_static_bytes_<capacity>_t` the machine declares first. */
#define SCE_OPTIONAL_DECLARE_BYTES(capacity)                                                                           \
    SCE_OPTIONAL_DECLARE(bytes_##capacity, sce_static_bytes_##capacity##_t, SCE_OPTIONAL_SAME_BYTES)

SCE_OPTIONAL_DECLARE(bool, bool, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(int8, int8_t, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(int16, int16_t, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(int32, int32_t, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(int64, int64_t, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(uint8, uint8_t, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(uint16, uint16_t, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(uint32, uint32_t, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(uint64, uint64_t, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(float32, float, SCE_OPTIONAL_SAME_VALUE)
SCE_OPTIONAL_DECLARE(float64, double, SCE_OPTIONAL_SAME_VALUE)

/* The absent value, and a present one holding `v`, of the optional type `T`, where
 * the place a statement writes it to names the type and no constructor can: `T` is
 * `__typeof__(place)`. The constructors above are the spelling wherever the element
 * is named; these are the one for a place, as `place = SCE_OPTIONAL_PRESENT_OF(
 * __typeof__(place), v)`. */
#define SCE_OPTIONAL_ABSENT_OF(T) ((T){.present = false})
#define SCE_OPTIONAL_PRESENT_OF(T, v) ((T){.present = true, .value = (v)})

/* `x ?? d` of a number, a `bool` or an enum: the value of a present `x`, and `d`,
 * evaluated only then, of an absent one. */
#define SCE_OPTIONAL_VALUE_OR(x, d) ((x).present ? (x).value : (d))

/* `x ?? d` of a text: the text a present `x` holds, and `d` of an absent one. */
#define SCE_OPTIONAL_TEXT_OR(x, d) ((x).present ? (const char *)(x).value.data : (d))

/* `x ?? d` of a byte string: the view of the bytes a present `x` holds, and `d`
 * of an absent one. */
#define SCE_OPTIONAL_BYTES_OR(x, d) ((x).present ? (sce_forge_bytes_view_t){(x).value.data, (x).value.len} : (d))

/* `x ?? y` where the default `y` may be absent too: `x` when it is present, and
 * `y`, evaluated only then, when it is not. */
#define SCE_OPTIONAL_EITHER(x, y) ((x).present ? (x) : (y))

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_OPTIONAL_H */
