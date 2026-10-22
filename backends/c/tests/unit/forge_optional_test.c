// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value that may be absent (docs/adr/0055) as a `sce-static` machine generated as C
// holds it: one struct of a flag and a value (`sce/forge/optional.h`), built only by
// its two constructors, an absent one not present and holding zero, compared as `===` compares two
// optional values, and read with a default that is evaluated only when the value is
// absent. On the wire an absent value is JSON `null`, written with its key, and the
// payload reader tells a value, `null` and a missing key apart.
//
// Each check names what it holds; the line that breaks it is the one in the header
// that spells what the check names.

#include <sce/event_payload.h>
#include <sce/forge/optional.h>
#include <sce/forge/wire.h>

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

// The buffers a machine declares for a text and a byte string of one bound, as the
// generator writes them, and the optional values of each.
typedef struct {
    char data[8 + 1];
} sce_static_string_8_t;

typedef struct {
    uint8_t data[4];
    size_t len;
} sce_static_bytes_4_t;

SCE_OPTIONAL_DECLARE_STRING(8)
SCE_OPTIONAL_DECLARE_BYTES(4)

static int failures = 0;

static void expect(bool holds, const char *what) {
    if (!holds) {
        (void)fprintf(stderr, "FAIL: %s\n", what);
        failures++;
    }
}

static void expect_text(const char *what, const char *got, const char *want) {
    if (strcmp(got, want) != 0) {
        (void)fprintf(stderr, "FAIL: %s: `%s`, expected `%s`\n", what, got, want);
        failures++;
    }
}

// How many times a default was evaluated: a default is evaluated only where the
// value is absent.
static int defaults_evaluated = 0;

static int64_t counted_default(int64_t value) {
    defaults_evaluated++;
    return value;
}

static const char *counted_text(const char *text) {
    defaults_evaluated++;
    return text;
}

static void the_constructors(void) {
    const sce_optional_int64_t absent = sce_optional_int64_none();
    const sce_optional_int64_t present = sce_optional_int64_some(INT64_MIN);
    expect(!absent.present, "none() is absent");
    expect(absent.value == 0, "none() zeroes the value");
    expect(present.present && present.value == INT64_MIN, "some(v) is present and holds v");

    // Two absent values are the same value, whatever value a present one held before:
    // not present, and holding zero. What is compared is what the type says (its members),
    // not its bytes: the padding between them is not part of the value, and a compiler is
    // free to leave it as it was.
    sce_optional_int64_t cleared = sce_optional_int64_some(7);
    cleared = sce_optional_int64_none();
    const sce_optional_int64_t fresh = sce_optional_int64_none();
    expect(!cleared.present && !fresh.present, "an absent value is not present");
    expect(cleared.value == 0 && fresh.value == 0, "an absent value holds zero");
    expect(sce_optional_int64_eq(cleared, fresh), "two absent values are the same value");

    const sce_optional_bool_t yes = sce_optional_bool_some(true);
    const sce_optional_float64_t half = sce_optional_float64_some(0.5);
    const sce_optional_uint8_t top = sce_optional_uint8_some(UINT8_MAX);
    expect(yes.present && yes.value, "an optional bool holds true");
    expect(half.present && half.value == 0.5, "an optional float64 holds 0.5");
    expect(top.present && top.value == UINT8_MAX, "an optional uint8 holds 255");
}

static void equality_is_total(void) {
    const sce_optional_int64_t absent = sce_optional_int64_none();
    const sce_optional_int64_t seven = sce_optional_int64_some(7);
    const sce_optional_int64_t also_seven = sce_optional_int64_some(7);
    const sce_optional_int64_t zero = sce_optional_int64_some(0);
    expect(sce_optional_int64_eq(absent, sce_optional_int64_none()), "null === null");
    expect(sce_optional_int64_eq(seven, also_seven), "7 === 7");
    expect(!sce_optional_int64_eq(seven, zero), "7 !== 0");
    // An absent value equals no present one, the one its zeroed value would spell
    // included.
    expect(!sce_optional_int64_eq(absent, zero), "null !== 0");
    expect(!sce_optional_int64_eq(zero, absent), "0 !== null");

    sce_static_string_8_t soon = {"soon"};
    sce_static_string_8_t later = {"later"};
    const sce_optional_string_8_t title = sce_optional_string_8_some(soon);
    expect(sce_optional_string_8_eq(title, sce_optional_string_8_some(soon)), "'soon' === 'soon'");
    expect(!sce_optional_string_8_eq(title, sce_optional_string_8_some(later)), "'soon' !== 'later'");
    expect(!sce_optional_string_8_eq(title, sce_optional_string_8_none()), "'soon' !== null");
    // An empty text is a text, and not an absent one.
    sce_static_string_8_t empty = {""};
    expect(!sce_optional_string_8_eq(sce_optional_string_8_some(empty), sce_optional_string_8_none()), "'' !== null");

    sce_static_bytes_4_t ab = {{'a', 'b', 0u, 0u}, 2u};
    sce_static_bytes_4_t ab0 = {{'a', 'b', 0u, 0u}, 3u};
    expect(sce_optional_bytes_4_eq(sce_optional_bytes_4_some(ab), sce_optional_bytes_4_some(ab)), "bytes ab === ab");
    expect(!sce_optional_bytes_4_eq(sce_optional_bytes_4_some(ab), sce_optional_bytes_4_some(ab0)),
           "bytes ab !== ab 0x00, by length");
    expect(!sce_optional_bytes_4_eq(sce_optional_bytes_4_some(ab), sce_optional_bytes_4_none()), "bytes ab !== null");

    // The comparisons an expression spells with no type of its own.
    expect(SCE_OPTIONAL_SAME(seven, also_seven, SCE_OPTIONAL_SAME_VALUE), "SAME: 7 === 7");
    expect(SCE_OPTIONAL_SAME(absent, absent, SCE_OPTIONAL_SAME_VALUE), "SAME: null === null");
    expect(!SCE_OPTIONAL_SAME(absent, zero, SCE_OPTIONAL_SAME_VALUE), "SAME: null !== 0");
    expect(SCE_OPTIONAL_HOLDS(seven, 7), "HOLDS: 7 === 7");
    expect(!SCE_OPTIONAL_HOLDS(seven, 8), "HOLDS: 7 !== 8");
    expect(!SCE_OPTIONAL_HOLDS(absent, 0), "HOLDS: null !== 0");
    expect(SCE_OPTIONAL_HOLDS_TEXT(title, "soon"), "HOLDS_TEXT: 'soon' === 'soon'");
    expect(!SCE_OPTIONAL_HOLDS_TEXT(sce_optional_string_8_none(), ""), "HOLDS_TEXT: null !== ''");
}

static void the_default_is_evaluated_only_when_absent(void) {
    const sce_optional_int64_t absent = sce_optional_int64_none();
    const sce_optional_int64_t twelve = sce_optional_int64_some(12);

    defaults_evaluated = 0;
    expect(SCE_OPTIONAL_VALUE_OR(twelve, counted_default(0)) == 12, "12 ?? 0 is 12");
    expect(defaults_evaluated == 0, "the default of a present value is not evaluated");
    expect(SCE_OPTIONAL_VALUE_OR(absent, counted_default(5)) == 5, "null ?? 5 is 5");
    expect(defaults_evaluated == 1, "the default of an absent value is evaluated once");

    sce_static_string_8_t soon = {"soon"};
    const sce_optional_string_8_t title = sce_optional_string_8_some(soon);
    defaults_evaluated = 0;
    expect_text("'soon' ?? 'none'", SCE_OPTIONAL_TEXT_OR(title, counted_text("none")), "soon");
    expect(defaults_evaluated == 0, "the default of a present text is not evaluated");
    expect_text("null ?? 'none'", SCE_OPTIONAL_TEXT_OR(sce_optional_string_8_none(), counted_text("none")), "none");
    expect(defaults_evaluated == 1, "the default of an absent text is evaluated once");

    // A byte string's default is its view.
    sce_static_bytes_4_t ab = {{'a', 'b', 0u, 0u}, 2u};
    static const uint8_t none_bytes[] = {'n'};
    const sce_forge_bytes_view_t fallback = {none_bytes, sizeof(none_bytes)};
    // Held in a variable: the view borrows the buffer of the value it reads.
    const sce_optional_bytes_4_t present_ab = sce_optional_bytes_4_some(ab);
    const sce_forge_bytes_view_t held = SCE_OPTIONAL_BYTES_OR(present_ab, fallback);
    expect(held.len == 2u && held.data[0] == 'a' && held.data[1] == 'b', "bytes ab ?? 'n' is ab");
    const sce_forge_bytes_view_t defaulted = SCE_OPTIONAL_BYTES_OR(sce_optional_bytes_4_none(), fallback);
    expect(defaulted.len == 1u && defaulted.data[0] == 'n', "bytes null ?? 'n' is 'n'");

    // A default that may be absent too: the value is optional.
    const sce_optional_int64_t either = SCE_OPTIONAL_EITHER(absent, twelve);
    expect(either.present && either.value == 12, "null ?? 12 is the optional 12");
    const sce_optional_int64_t kept = SCE_OPTIONAL_EITHER(twelve, absent);
    expect(kept.present && kept.value == 12, "12 ?? null is 12");
    expect(!SCE_OPTIONAL_EITHER(absent, absent).present, "null ?? null is absent");
}

static void an_absent_value_crosses_as_null(void) {
    char json[64];
    expect(sce_forge_wire_json(sce_forge_wire_null(), json, sizeof(json)), "null fits its buffer");
    expect_text("an absent value as JSON", json, "null");

    // The key is written, its value `null`, beside a present value.
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, json, sizeof(json));
    sce_forge_wire_pair(&w, "\"due\"", sce_forge_wire_null());
    sce_forge_wire_pair(&w, "\"title\"", sce_forge_wire_string("later"));
    expect(sce_forge_wire_end(&w), "the object fits its buffer");
    expect_text("an object with an absent value", json, "{\"due\":null,\"title\":\"later\"}");

    // Text has no spelling for absence: refused, never written as some text.
    char text[16] = "x";
    expect(!sce_forge_wire_text(sce_forge_wire_null(), text, sizeof(text)), "an absent value is refused as text");
    expect_text("an absent value refused as text", text, "");
}

static void the_reader_tells_a_value_null_and_a_missing_key_apart(void) {
    static const char delivery[] = "{\"id\":2,\"due\":null,\"title\":\"later\",\"status\": null }";
    sce_payload_fields_t fields = {delivery, NULL};
    bool present = true;

    expect(sce_payload_read_presence(&fields, "due", &present) == NULL && !present, "`due: null` is absent");
    present = true;
    expect(sce_payload_read_presence(&fields, "status", &present) == NULL && !present,
           "`status: null` before a space and the end is absent");
    present = false;
    expect(sce_payload_read_presence(&fields, "title", &present) == NULL && present, "`title: \"later\"` is present");
    int64_t id = 0;
    present = false;
    expect(sce_payload_read_presence(&fields, "id", &present) == NULL && present &&
               sce_payload_read_i64(&fields, "id", &id) == NULL && id == 2,
           "`id: 2` is present and reads 2");

    // A missing key is a sender whose revision does not have the field: refused.
    const char *missing = sce_payload_read_presence(&fields, "kept", &present);
    expect(missing != NULL && strcmp(missing, SCE_PAYLOAD_REFUSE_MISSING) == 0, "a missing key is refused");

    // The text `null` is a present text, and a word that starts with `null` is no
    // `null`.
    static const char texts[] = "{\"title\":\"null\",\"word\":nullish}";
    sce_payload_fields_t text_fields = {texts, NULL};
    present = false;
    expect(sce_payload_read_presence(&text_fields, "title", &present) == NULL && present,
           "the text \"null\" is present");
    present = false;
    expect(sce_payload_read_presence(&text_fields, "word", &present) == NULL && present, "`nullish` is not `null`");

    // A field a later revision added with no default reads the `null` the schema's
    // defaults hold for it, and a delivery that carries it is read as it came.
    static const char older[] = "{\"id\":1}";
    static const char defaults[] = "{\"due\":null}";
    sce_payload_fields_t revised = {older, defaults};
    present = true;
    expect(sce_payload_read_presence(&revised, "due", &present) == NULL && !present,
           "an added field with no default reads absent");
    static const char newer[] = "{\"id\":1,\"due\":4}";
    sce_payload_fields_t carried = {newer, defaults};
    int64_t due = 0;
    present = false;
    expect(sce_payload_read_presence(&carried, "due", &present) == NULL && present &&
               sce_payload_read_i64(&carried, "due", &due) == NULL && due == 4,
           "an added field that is carried reads its value");
}

int main(void) {
    the_constructors();
    equality_is_total();
    the_default_is_evaluated_only_when_absent();
    an_absent_value_crosses_as_null();
    the_reader_tells_a_value_null_and_a_missing_key_apart();
    if (failures != 0) {
        (void)fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    return 0;
}
