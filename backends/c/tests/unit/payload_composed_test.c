// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A payload value of any bounded type is read at its path, or refused there before
// anything of it is kept (docs/adr/0086, sections 4 and 14.3): every element kind, at
// list depths 1 to 3, each bound 2, with an empty value, a maximal one, one past the
// bound at the innermost and the outermost list, a bad element, `null` where the
// element is not optional, an unknown key and an older revision. The oracle is the
// construction: each case is built with the value, or the path and the sentence, it
// must come to. A maximal value written by the wire writer is the text the case was
// built from, at exactly the size the calculus gives it; a bounded list refuses past
// its bound. Header-only code, no allocator.

#include <sce/event_payload.h>
#include <sce/forge/bounded_list.h>
#include <sce/forge/wire.h>

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

enum { BOUND = 2, TEXT_BOUND = 3, PATH_CAP = 64, HELD_CAP = 2048, JSON_CAP = 4096 };

static int failures = 0;

typedef enum {
    K_U8,
    K_U16,
    K_U32,
    K_U64,
    K_I8,
    K_I16,
    K_I32,
    K_I64,
    K_F32,
    K_F64,
    K_BOOL,
    K_ENUM,
    K_TEXT,
    K_BYTES,
    K_OPT,
    K_REC,
    K_REC_LIST,
    K_COUNT
} kind_t;

/* Each kind: its name, the JSON of its maximal value and how the model renders it,
 * a bad element with the path below it is refused at and the sentence, and the size of
 * its largest JSON text (docs/adr/0086, section 2.1). `K_REC` is `{a: uint8, b:
 * string[3]}`, whose schema's later revision added `c: uint8` with the default 9;
 * `K_REC_LIST` is `{tags: list<string[3]>[2]}`. */
typedef struct {
    const char *name;
    const char *max_json;
    const char *max_held;
    const char *bad_json;
    const char *bad_below;
    const char *bad_sentence;
    size_t largest;
} kind_info_t;

static const char ENUM_SENTENCE[] = "is not a variant of Level";

#define CONTROLS_JSON "\"\\u0001\\u0001\\u0001\""
#define CONTROLS_HELD "x010101"

static const kind_info_t KINDS[K_COUNT] = {
    {"uint8", "255", "255", "256000000000", "", SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH, 3},
    {"uint16", "65535", "65535", "256000000000", "", SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH, 5},
    {"uint32", "4294967295", "4294967295", "256000000000", "", SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH, 10},
    {"uint64", "\"18446744073709551615\"", "18446744073709551615", "\"-1\"", "", SCE_PAYLOAD_REFUSE_NOT_DIGITS, 22},
    {"int8", "-128", "-128", "1.5", "", SCE_PAYLOAD_REFUSE_NOT_WHOLE, 4},
    {"int16", "-32768", "-32768", "1.5", "", SCE_PAYLOAD_REFUSE_NOT_WHOLE, 6},
    {"int32", "-2147483648", "-2147483648", "1.5", "", SCE_PAYLOAD_REFUSE_NOT_WHOLE, 11},
    {"int64", "\"-9223372036854775808\"", "-9223372036854775808", "9007199254740992", "", SCE_PAYLOAD_REFUSE_PAST_2_53,
     22},
    /* Seventeen digits after `0.00000`: the longest spelling of a number, which only the decimal band from 1e-6 to
     * 1e-5 reaches. The single crosses as the double it widens to. */
    {"float32", "-0.0000012345677760094986", "-0.0000012345677760094986", "1e39", "", SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH,
     25},
    {"float64", "-0.0000012345678901234567", "-0.0000012345678901234567", "true", "", SCE_PAYLOAD_REFUSE_NOT_NUMBER,
     25},
    {"bool", "false", "false", "0", "", SCE_PAYLOAD_REFUSE_NOT_TRUTH, 5},
    {"enum", "\"medium\"", "medium", "\"lowest\"", "", ENUM_SENTENCE, 8},
    {"string[3]", CONTROLS_JSON, CONTROLS_HELD, "\"abcd\"", "", SCE_PAYLOAD_REFUSE_PAST_BOUND, 2 + 6 * TEXT_BOUND},
    {"bytes[3]", CONTROLS_JSON, CONTROLS_HELD, "\"\\u0100\"", "", SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE, 2 + 6 * TEXT_BOUND},
    /* `null` (4 bytes) is longer than any uint8's digits, so the maximal value is absent. */
    {"optional<uint8>", "null", "null", "256000000000", "", SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH, 4},
    {"record", "{\"a\":255,\"b\":" CONTROLS_JSON ",\"c\":255}", "{255," CONTROLS_HELD ",255}",
     "{\"a\":\"x\",\"b\":\"\"}", "/a", SCE_PAYLOAD_REFUSE_NOT_NUMBER,
     2 + (3 + 1 + 3) + (3 + 1 + 2 + 6 * TEXT_BOUND) + (3 + 1 + 3) + 2},
    {"record of a list", "{\"tags\":[" CONTROLS_JSON "," CONTROLS_JSON "]}", "{[" CONTROLS_HELD "," CONTROLS_HELD "]}",
     "{\"tags\":[\"\",\"\",\"x\"]}", "/tags", SCE_PAYLOAD_REFUSE_PAST_COUNT,
     2 + (6 + 1 + 2 + BOUND * (2 + 6 * TEXT_BOUND) + (BOUND - 1))},
};

/* Whether a read was refused with the sentence `want`: a sentence is compared by its
 * text, never by where the string lives. */
static bool refused_as(const char *refusal, const char *want) {
    return refusal != NULL && strcmp(refusal, want) == 0;
}

/* ── The model's text of a value, which a read renders and a case expects ── */

static void append(char *out, size_t cap, const char *text) {
    const size_t len = strlen(out);
    (void)snprintf(out + len, cap - len, "%s", text);
}

static void render_bytes(char *out, size_t cap, const uint8_t *bytes, size_t len) {
    append(out, cap, "x");
    for (size_t i = 0u; i < len; ++i) {
        char hex[3];
        (void)snprintf(hex, sizeof(hex), "%02x", (unsigned)bytes[i]);
        append(out, cap, hex);
    }
}

/* ── The reader of each kind, one call per constructor, as the generator writes it ── */

/* Read one value of `kind` at `at`, its path `path`, rendering it into `held`; on a
 * refusal, `refused_at` receives the path the sentence is said of. */
static const char *read_leaf(kind_t kind, const char *at, const char *path, char *held, char *refused_at);

static const char *read_value(kind_t kind, int depth, const char *at, const char *path, char *held, char *refused_at) {
    if (depth == 0) {
        return read_leaf(kind, at, path, held, refused_at);
    }
    sce_payload_list_t list;
    const char *refusal = sce_payload_list_open(at, BOUND, &list);
    if (refusal != NULL) {
        (void)snprintf(refused_at, PATH_CAP, "%s", path);
        return refusal;
    }
    append(held, HELD_CAP, "[");
    const char *element = NULL;
    uint32_t index = 0u;
    while (sce_payload_list_next(&list, &element)) {
        char element_path[PATH_CAP];
        if (!sce_payload_path_index(element_path, sizeof(element_path), path, index)) {
            return "the path does not fit its buffer";
        }
        if (index > 0u) {
            append(held, HELD_CAP, ",");
        }
        refusal = read_value(kind, depth - 1, element, element_path, held, refused_at);
        if (refusal != NULL) {
            return refusal;
        }
        index++;
    }
    append(held, HELD_CAP, "]");
    return NULL;
}

static const char *read_record_field_u8(const sce_payload_fields_t *fields, const char *path, const char *name,
                                        char *held, char *refused_at) {
    uint8_t n = 0;
    const char *refusal = sce_payload_read_u8(fields, name, &n);
    if (refusal != NULL) {
        (void)sce_payload_path_join(refused_at, PATH_CAP, path, name);
        return refusal;
    }
    char text[8];
    (void)snprintf(text, sizeof(text), "%u", (unsigned)n);
    append(held, HELD_CAP, text);
    return NULL;
}

static const char *read_leaf(kind_t kind, const char *at, const char *path, char *held, char *refused_at) {
    const char *refusal = NULL;
    char text[64] = {0};
    (void)snprintf(refused_at, PATH_CAP, "%s", path);
    switch (kind) {
    case K_U8: {
        uint8_t n = 0;
        refusal = sce_payload_value_u8(at, &n);
        (void)snprintf(text, sizeof(text), "%u", (unsigned)n);
        break;
    }
    case K_U16: {
        uint16_t n = 0;
        refusal = sce_payload_value_u16(at, &n);
        (void)snprintf(text, sizeof(text), "%u", (unsigned)n);
        break;
    }
    case K_U32: {
        uint32_t n = 0;
        refusal = sce_payload_value_u32(at, &n);
        (void)snprintf(text, sizeof(text), "%lu", (unsigned long)n);
        break;
    }
    case K_U64: {
        uint64_t n = 0;
        refusal = sce_payload_value_u64(at, &n);
        (void)snprintf(text, sizeof(text), "%llu", (unsigned long long)n);
        break;
    }
    case K_I8: {
        int8_t n = 0;
        refusal = sce_payload_value_i8(at, &n);
        (void)snprintf(text, sizeof(text), "%d", (int)n);
        break;
    }
    case K_I16: {
        int16_t n = 0;
        refusal = sce_payload_value_i16(at, &n);
        (void)snprintf(text, sizeof(text), "%d", (int)n);
        break;
    }
    case K_I32: {
        int32_t n = 0;
        refusal = sce_payload_value_i32(at, &n);
        (void)snprintf(text, sizeof(text), "%ld", (long)n);
        break;
    }
    case K_I64: {
        int64_t n = 0;
        refusal = sce_payload_value_i64(at, &n);
        (void)snprintf(text, sizeof(text), "%lld", (long long)n);
        break;
    }
    case K_F32: {
        float x = 0.0f;
        refusal = sce_payload_value_f32(at, &x);
        (void)sce_number_text((double)x, text, sizeof(text));
        break;
    }
    case K_F64: {
        double x = 0.0;
        refusal = sce_payload_value_f64(at, &x);
        (void)sce_number_text(x, text, sizeof(text));
        break;
    }
    case K_BOOL: {
        bool b = true;
        refusal = sce_payload_value_bool(at, &b);
        (void)snprintf(text, sizeof(text), "%s", b ? "true" : "false");
        break;
    }
    case K_ENUM: {
        /* The longest name the enum declares is the bound its reader reads with. */
        refusal = sce_payload_value_text(at, text, 6u);
        if (refused_as(refusal, SCE_PAYLOAD_REFUSE_PAST_BOUND) ||
            (refusal == NULL && strcmp(text, "low") != 0 && strcmp(text, "medium") != 0 && strcmp(text, "high") != 0)) {
            refusal = ENUM_SENTENCE;
        }
        break;
    }
    case K_TEXT: {
        char value[TEXT_BOUND + 1];
        refusal = sce_payload_value_text(at, value, TEXT_BOUND);
        if (refusal == NULL) {
            render_bytes(text, sizeof(text), (const uint8_t *)value, strlen(value));
        }
        break;
    }
    case K_BYTES: {
        uint8_t value[TEXT_BOUND];
        size_t len = 0u;
        refusal = sce_payload_value_bytes(at, value, TEXT_BOUND, &len);
        if (refusal == NULL) {
            render_bytes(text, sizeof(text), value, len);
        }
        break;
    }
    case K_OPT: {
        if (sce_payload_is_null(at)) {
            (void)snprintf(text, sizeof(text), "null");
            break;
        }
        uint8_t n = 0;
        refusal = sce_payload_value_u8(at, &n);
        (void)snprintf(text, sizeof(text), "%u", (unsigned)n);
        break;
    }
    case K_REC: {
        sce_payload_fields_t fields;
        refusal = sce_payload_record_open(at, "{\"c\":9}", &fields);
        if (refusal != NULL) {
            return refusal;
        }
        append(held, HELD_CAP, "{");
        refusal = read_record_field_u8(&fields, path, "a", held, refused_at);
        if (refusal != NULL) {
            return refusal;
        }
        append(held, HELD_CAP, ",");
        const char *b = NULL;
        char b_path[PATH_CAP];
        (void)sce_payload_path_join(b_path, sizeof(b_path), path, "b");
        refusal = sce_payload_field(&fields, "b", &b);
        if (refusal == NULL) {
            refusal = read_leaf(K_TEXT, b, b_path, held, refused_at);
        } else {
            (void)snprintf(refused_at, PATH_CAP, "%s", b_path);
        }
        if (refusal != NULL) {
            return refusal;
        }
        append(held, HELD_CAP, ",");
        refusal = read_record_field_u8(&fields, path, "c", held, refused_at);
        if (refusal != NULL) {
            return refusal;
        }
        append(held, HELD_CAP, "}");
        return NULL;
    }
    case K_REC_LIST: {
        sce_payload_fields_t fields;
        refusal = sce_payload_record_open(at, NULL, &fields);
        if (refusal != NULL) {
            return refusal;
        }
        char tags_path[PATH_CAP];
        (void)sce_payload_path_join(tags_path, sizeof(tags_path), path, "tags");
        const char *tags = NULL;
        refusal = sce_payload_field(&fields, "tags", &tags);
        if (refusal != NULL) {
            (void)snprintf(refused_at, PATH_CAP, "%s", tags_path);
            return refusal;
        }
        append(held, HELD_CAP, "{");
        refusal = read_value(K_TEXT, 1, tags, tags_path, held, refused_at);
        if (refusal != NULL) {
            return refusal;
        }
        append(held, HELD_CAP, "}");
        return NULL;
    }
    case K_COUNT:
        return "no such kind";
    }
    if (refusal == NULL) {
        append(held, HELD_CAP, text);
    }
    return refusal;
}

/* ── The cases ───────────────────────────────────────────────────────────── */

/* The JSON of the maximal value of `kind` at `depth`, with the leaf at the end of the
 * last index at every depth written as `last_leaf` where it is given, and one more
 * element in the list at `past_level` along the last index (1 the outermost, 0 none). */
static void list_json(kind_t kind, int depth, const char *last_leaf, int past_level, char *out, size_t cap) {
    if (depth == 0) {
        append(out, cap, last_leaf != NULL ? last_leaf : KINDS[kind].max_json);
        return;
    }
    append(out, cap, "[");
    list_json(kind, depth - 1, NULL, 0, out, cap);
    append(out, cap, ",");
    list_json(kind, depth - 1, last_leaf, past_level > 1 ? past_level - 1 : 0, out, cap);
    if (past_level == 1) {
        append(out, cap, ",");
        list_json(kind, depth - 1, NULL, 0, out, cap);
    }
    append(out, cap, "]");
}

/* The model's text of the maximal value, its last leaf `last_leaf` where given. */
static void list_held(kind_t kind, int depth, const char *last_leaf, char *out, size_t cap) {
    if (depth == 0) {
        append(out, cap, last_leaf != NULL ? last_leaf : KINDS[kind].max_held);
        return;
    }
    append(out, cap, "[");
    list_held(kind, depth - 1, NULL, out, cap);
    append(out, cap, ",");
    list_held(kind, depth - 1, last_leaf, out, cap);
    append(out, cap, "]");
}

static void innermost(int depth, char *out, size_t cap) {
    (void)snprintf(out, cap, "v");
    for (int i = 1; i < depth; ++i) {
        append(out, cap, "/1");
    }
}

static int cases = 0;

/* Read `list` (the JSON of the field `v`) as `kind` at `depth`, beside a key no schema
 * declares, with the schema's `defaults` where given, and hold the result to `want`
 * (the model's text) or to a refusal at `want_path` saying `want_sentence`. */
static void expect(const char *label, kind_t kind, int depth, const char *data, const char *defaults, const char *want,
                   const char *want_path, const char *want_sentence) {
    cases++;
    sce_payload_fields_t fields;
    const char *refusal = (defaults != NULL) ? sce_payload_decode_with_defaults(data, defaults, &fields)
                                             : sce_payload_decode(data, &fields);
    const char *v = NULL;
    if (refusal == NULL) {
        refusal = sce_payload_field(&fields, "v", &v);
    }
    char held[HELD_CAP] = {0};
    char refused_at[PATH_CAP] = {0};
    if (refusal == NULL) {
        refusal = read_value(kind, depth, v, "v", held, refused_at);
    }
    if (want != NULL) {
        if (refusal != NULL || strcmp(held, want) != 0) {
            fprintf(stderr, "FAIL: %s at depth %d, %s: read `%s` (%s), the oracle says `%s`\n", KINDS[kind].name, depth,
                    label, held, refusal != NULL ? refusal : "no refusal", want);
            failures++;
        }
        return;
    }
    if (refusal == NULL || strcmp(refusal, want_sentence) != 0 || strcmp(refused_at, want_path) != 0) {
        fprintf(stderr, "FAIL: %s at depth %d, %s: refused at '%s' as `%s`, the oracle says '%s' `%s`\n",
                KINDS[kind].name, depth, label, refused_at, refusal != NULL ? refusal : "(read)", want_path,
                want_sentence);
        failures++;
    }
}

static void wrap(const char *list, char *out, size_t cap) {
    (void)snprintf(out, cap, "{\"v\":%s,\"zz\":{\"unknown\":[1,null]}}", list);
}

static void check_cross_product(void) {
    for (int k = 0; k < K_COUNT; ++k) {
        const kind_t kind = (kind_t)k;
        for (int depth = 1; depth <= 3; ++depth) {
            char list[JSON_CAP];
            char data[JSON_CAP];
            char want[HELD_CAP];
            char inner[PATH_CAP];
            char leaf[PATH_CAP];
            innermost(depth, inner, sizeof(inner));
            /* The last element of the innermost list: the path one index below it. */
            innermost(depth + 1, leaf, sizeof(leaf));

            expect("empty", kind, depth, "{\"v\":[],\"zz\":1}", NULL, "[]", NULL, NULL);

            list[0] = '\0';
            list_json(kind, depth, NULL, 0, list, sizeof(list));
            wrap(list, data, sizeof(data));
            want[0] = '\0';
            list_held(kind, depth, NULL, want, sizeof(want));
            expect("maximal", kind, depth, data, NULL, want, NULL, NULL);
            /* Every key of a record element and the field beside the list are read
             * around a key no schema declares, which the data carries. */
            expect("an unknown key", kind, depth, data, NULL, want, NULL, NULL);

            list[0] = '\0';
            list_json(kind, depth, NULL, depth, list, sizeof(list));
            wrap(list, data, sizeof(data));
            expect("one past the innermost bound", kind, depth, data, NULL, NULL, inner, SCE_PAYLOAD_REFUSE_PAST_COUNT);

            list[0] = '\0';
            list_json(kind, depth, NULL, 1, list, sizeof(list));
            wrap(list, data, sizeof(data));
            expect("one past the outermost bound", kind, depth, data, NULL, NULL, "v", SCE_PAYLOAD_REFUSE_PAST_COUNT);

            if (kind == K_OPT) {
                /* The maximal list of optionals is every element absent; a present
                 * one among them is read as present. */
                list[0] = '\0';
                list_json(kind, depth, "7", 0, list, sizeof(list));
                wrap(list, data, sizeof(data));
                want[0] = '\0';
                list_held(kind, depth, "7", want, sizeof(want));
                expect("a present element among absent ones", kind, depth, data, NULL, want, NULL, NULL);
            } else {
                list[0] = '\0';
                list_json(kind, depth, "null", 0, list, sizeof(list));
                wrap(list, data, sizeof(data));
                expect("null where the element is not optional", kind, depth, data, NULL, NULL, leaf,
                       SCE_PAYLOAD_REFUSE_NULL);
            }

            list[0] = '\0';
            list_json(kind, depth, KINDS[kind].bad_json, 0, list, sizeof(list));
            wrap(list, data, sizeof(data));
            char bad_at[PATH_CAP];
            (void)snprintf(bad_at, sizeof(bad_at), "%s%s", leaf, KINDS[kind].bad_below);
            expect("a bad element", kind, depth, data, NULL, NULL, bad_at, KINDS[kind].bad_sentence);

            if (kind == K_REC) {
                /* A record element of an older revision reads the field its schema's
                 * later revision added from that schema's defaults; a key no schema
                 * declares inside an element is stepped over. */
                list[0] = '\0';
                list_json(kind, depth, "{\"a\":1,\"b\":\"\"}", 0, list, sizeof(list));
                wrap(list, data, sizeof(data));
                want[0] = '\0';
                list_held(kind, depth, "{1,x,9}", want, sizeof(want));
                expect("an element of an older revision", kind, depth, data, NULL, want, NULL, NULL);
            } else {
                /* A delivery of the revision before the list reads it empty. */
                expect("a delivery of the revision before the list", kind, depth, "{\"zz\":1}", "{\"v\":[]}", "[]",
                       NULL, NULL);
            }
        }
    }
    /* A key no schema declares inside a record element. */
    char data[JSON_CAP];
    wrap("[{\"zz\":[{\"q\":1}],\"c\":255,\"b\":" CONTROLS_JSON ",\"a\":255},{\"a\":255,\"b\":" CONTROLS_JSON
         ",\"c\":255}]",
         data, sizeof(data));
    expect("an unknown key inside an element", K_REC, 1, data, NULL,
           "[{255," CONTROLS_HELD ",255},{255," CONTROLS_HELD ",255}]", NULL, NULL);
    if (cases != K_COUNT * 3 * 8 + 1) {
        fprintf(stderr, "FAIL: the cross product holds %d cases\n", cases);
        failures++;
    }
}

/* ── The writer: a maximal value is the text the cases were built from ───── */

static void write_leaf(sce_forge_wire_t *w, kind_t kind) {
    switch (kind) {
    case K_U8:
        sce_forge_wire_element(w, sce_forge_wire_uint(UINT8_MAX));
        break;
    case K_U16:
        sce_forge_wire_element(w, sce_forge_wire_uint(UINT16_MAX));
        break;
    case K_U32:
        sce_forge_wire_element(w, sce_forge_wire_uint(UINT32_MAX));
        break;
    case K_U64:
        sce_forge_wire_element(w, sce_forge_wire_wide_uint(UINT64_MAX));
        break;
    case K_I8:
        sce_forge_wire_element(w, sce_forge_wire_int(INT8_MIN));
        break;
    case K_I16:
        sce_forge_wire_element(w, sce_forge_wire_int(INT16_MIN));
        break;
    case K_I32:
        sce_forge_wire_element(w, sce_forge_wire_int(INT32_MIN));
        break;
    case K_I64:
        sce_forge_wire_element(w, sce_forge_wire_wide_int(INT64_MIN));
        break;
    case K_F32:
        sce_forge_wire_element(w, sce_forge_wire_real((double)-1.2345677760094986e-06f));
        break;
    case K_F64:
        sce_forge_wire_element(w, sce_forge_wire_real(-1.2345678901234567e-06));
        break;
    case K_BOOL:
        sce_forge_wire_element(w, sce_forge_wire_bool(false));
        break;
    case K_ENUM:
        sce_forge_wire_element(w, sce_forge_wire_string("medium"));
        break;
    case K_TEXT:
        sce_forge_wire_element(w, sce_forge_wire_string("\x01\x01\x01"));
        break;
    case K_BYTES: {
        static const uint8_t bytes[TEXT_BOUND] = {1, 1, 1};
        sce_forge_bytes_view_t view = {bytes, TEXT_BOUND};
        sce_forge_wire_element(w, sce_forge_wire_bytes(view));
        break;
    }
    case K_OPT:
        sce_forge_wire_element(w, sce_forge_wire_null());
        break;
    case K_REC:
        sce_forge_wire_element_open(w, '{');
        sce_forge_wire_member(w, "\"a\"", sce_forge_wire_uint(255));
        sce_forge_wire_member(w, "\"b\"", sce_forge_wire_string("\x01\x01\x01"));
        sce_forge_wire_member(w, "\"c\"", sce_forge_wire_uint(255));
        sce_forge_wire_close(w);
        break;
    case K_REC_LIST:
        sce_forge_wire_element_open(w, '{');
        sce_forge_wire_member_open(w, "\"tags\"", '[');
        sce_forge_wire_element(w, sce_forge_wire_string("\x01\x01\x01"));
        sce_forge_wire_element(w, sce_forge_wire_string("\x01\x01\x01"));
        sce_forge_wire_close(w);
        sce_forge_wire_close(w);
        break;
    case K_COUNT:
        break;
    }
}

static void write_list(sce_forge_wire_t *w, kind_t kind, int depth) {
    for (int i = 0; i < BOUND; ++i) {
        if (depth == 1) {
            write_leaf(w, kind);
        } else {
            sce_forge_wire_element_open(w, '[');
            write_list(w, kind, depth - 1);
            sce_forge_wire_close(w);
        }
    }
}

static void check_maximal_sizes(void) {
    for (int k = 0; k < K_COUNT; ++k) {
        const kind_t kind = (kind_t)k;
        size_t size = KINDS[kind].largest;
        for (int depth = 1; depth <= 3; ++depth) {
            size = 2u + BOUND * size + (BOUND - 1u);
            char buf[JSON_CAP];
            sce_forge_wire_t w;
            sce_forge_wire_begin(&w, buf, sizeof(buf));
            sce_forge_wire_pair_open(&w, "\"v\"", '[');
            write_list(&w, kind, depth);
            sce_forge_wire_close(&w);
            if (!sce_forge_wire_end(&w)) {
                fprintf(stderr, "FAIL: %s at depth %d: the maximal value did not fit\n", KINDS[kind].name, depth);
                failures++;
                continue;
            }
            char want[JSON_CAP] = "{\"v\":";
            list_json(kind, depth, NULL, 0, want, sizeof(want));
            append(want, sizeof(want), "}");
            if (strcmp(buf, want) != 0) {
                fprintf(stderr, "FAIL: %s at depth %d: written `%s`, built `%s`\n", KINDS[kind].name, depth, buf, want);
                failures++;
            }
            /* The value is the text between `{"v":` and the closing `}`. */
            const size_t written = strlen(buf) - 6u;
            if (written != size) {
                fprintf(stderr, "FAIL: %s at depth %d: %zu bytes, the calculus says %zu\n", KINDS[kind].name, depth,
                        written, size);
                failures++;
            }
        }
    }
}

/* A container left open, a member in an array, an element in an object and a pair
 * inside a container are each an object that does not fit, written as `{}`. */
static void check_the_writer_refuses_a_shape_it_cannot_close(void) {
    char buf[64];
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair_open(&w, "\"v\"", '[');
    if (sce_forge_wire_end(&w) || strcmp(buf, "{}") != 0) {
        fprintf(stderr, "FAIL: a container left open was written as `%s`\n", buf);
        failures++;
    }
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair_open(&w, "\"v\"", '[');
    sce_forge_wire_member(&w, "\"a\"", sce_forge_wire_uint(1));
    sce_forge_wire_close(&w);
    if (sce_forge_wire_end(&w)) {
        fprintf(stderr, "FAIL: a member was written into an array\n");
        failures++;
    }
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair_open(&w, "\"v\"", '{');
    sce_forge_wire_element(&w, sce_forge_wire_uint(1));
    sce_forge_wire_close(&w);
    if (sce_forge_wire_end(&w)) {
        fprintf(stderr, "FAIL: an element was written into an object\n");
        failures++;
    }
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair_open(&w, "\"v\"", '{');
    sce_forge_wire_pair(&w, "\"a\"", sce_forge_wire_uint(1));
    sce_forge_wire_close(&w);
    if (sce_forge_wire_end(&w)) {
        fprintf(stderr, "FAIL: a pair was written inside a container\n");
        failures++;
    }
    /* A scalar pair and a container pair of one object, in the order given. */
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"a\"", sce_forge_wire_uint(1));
    sce_forge_wire_pair_open(&w, "\"b\"", '[');
    sce_forge_wire_close(&w);
    sce_forge_wire_pair(&w, "\"c\"", sce_forge_wire_null());
    if (!sce_forge_wire_end(&w) || strcmp(buf, "{\"a\":1,\"b\":[],\"c\":null}") != 0) {
        fprintf(stderr, "FAIL: a pair beside a container pair was written as `%s`\n", buf);
        failures++;
    }
}

/* A pair whose value a machine's writer writes part by part (docs/adr/0086, section
 * 5.3): the key named first, then the value bare, beside a scalar pair; a pair of a
 * name that repeats is never collected with it; a key inside a container is a shape
 * that does not fit; and a value refused by its writer leaves the object unwritten. */
static void check_a_composed_pair_is_written_part_by_part(void) {
    char buf[96];
    sce_forge_wire_t w;
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"a\"", sce_forge_wire_uint(1));
    sce_forge_wire_pair_key(&w, "\"b\"");
    sce_forge_wire_open_container(&w, '[');
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_open_container(&w, '{');
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_put(&w, "\"k\":", 4u);
    sce_forge_wire_put_value(&w, sce_forge_wire_string("x"));
    sce_forge_wire_close(&w);
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_put_value(&w, sce_forge_wire_null());
    sce_forge_wire_close(&w);
    sce_forge_wire_pair_key(&w, "\"c\"");
    sce_forge_wire_put_value(&w, sce_forge_wire_bool(true));
    if (!sce_forge_wire_end(&w) || strcmp(buf, "{\"a\":1,\"b\":[{\"k\":\"x\"},null],\"c\":true}") != 0) {
        fprintf(stderr, "FAIL: a composed pair written part by part reads `%s`\n", buf);
        failures++;
    }
    /* The pair before a composed one is a value of its own, not an array it joins. */
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"a\"", sce_forge_wire_uint(1));
    sce_forge_wire_pair_key(&w, "\"a\"");
    sce_forge_wire_put_value(&w, sce_forge_wire_uint(2));
    if (!sce_forge_wire_end(&w) || strcmp(buf, "{\"a\":1,\"a\":2}") != 0) {
        fprintf(stderr, "FAIL: a composed pair was collected with the name before it: `%s`\n", buf);
        failures++;
    }
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair_open(&w, "\"v\"", '[');
    sce_forge_wire_pair_key(&w, "\"k\"");
    sce_forge_wire_close(&w);
    if (sce_forge_wire_end(&w) || strcmp(buf, "{}") != 0) {
        fprintf(stderr, "FAIL: a pair's key inside a container was written as `%s`\n", buf);
        failures++;
    }
    /* The text of one composed value alone, read back from the object it was
       written into, as a send a host serves carries each pair as text. */
    char text[32];
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair(&w, "\"a\"", sce_forge_wire_uint(1));
    sce_forge_wire_pair_key(&w, "\"b\"");
    const size_t mark = sce_forge_wire_mark(&w);
    sce_forge_wire_open_container(&w, '[');
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_put_value(&w, sce_forge_wire_uint(7));
    sce_forge_wire_close(&w);
    if (!sce_forge_wire_copy_since(&w, mark, text, sizeof(text)) || strcmp(text, "[7]") != 0) {
        fprintf(stderr, "FAIL: the text of a composed value reads `%s`\n", text);
        failures++;
    }
    if (sce_forge_wire_copy_since(&w, mark, text, 3u) || text[0] != '\0') {
        fprintf(stderr, "FAIL: the text of a composed value was cut to `%s`\n", text);
        failures++;
    }
    sce_forge_wire_refuse(&w);
    if (sce_forge_wire_copy_since(&w, mark, text, sizeof(text)) || text[0] != '\0') {
        fprintf(stderr, "FAIL: the text of a refused object was read as `%s`\n", text);
        failures++;
    }
    sce_forge_wire_begin(&w, buf, sizeof(buf));
    sce_forge_wire_pair_key(&w, "\"v\"");
    sce_forge_wire_open_container(&w, '[');
    sce_forge_wire_refuse(&w);
    sce_forge_wire_close(&w);
    if (sce_forge_wire_end(&w) || strcmp(buf, "{}") != 0) {
        fprintf(stderr, "FAIL: a refused value left the object written as `%s`\n", buf);
        failures++;
    }
}

/* One composed value written alone into a buffer of its own, with no object around
 * it, as a request that carries each pair as text only is handed one: the JSON of
 * the value, terminated; and the empty string, never a value cut short, for one
 * that was refused, left a container open or did not fit. */
static void check_a_value_is_written_alone(void) {
    char buf[32];
    sce_forge_wire_t w;
    sce_forge_wire_begin_value(&w, buf, sizeof(buf));
    sce_forge_wire_open_container(&w, '[');
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_put_value(&w, sce_forge_wire_uint(7));
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_open_container(&w, '{');
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_put(&w, "\"k\":", 4u);
    sce_forge_wire_put_value(&w, sce_forge_wire_string("x"));
    sce_forge_wire_close(&w);
    sce_forge_wire_close(&w);
    if (!sce_forge_wire_end_value(&w) || strcmp(buf, "[7,{\"k\":\"x\"}]") != 0) {
        fprintf(stderr, "FAIL: a value written alone reads `%s`\n", buf);
        failures++;
    }
    sce_forge_wire_begin_value(&w, buf, sizeof(buf));
    sce_forge_wire_put_value(&w, sce_forge_wire_bool(true));
    if (!sce_forge_wire_end_value(&w) || strcmp(buf, "true") != 0) {
        fprintf(stderr, "FAIL: a scalar written alone reads `%s`\n", buf);
        failures++;
    }
    sce_forge_wire_begin_value(&w, buf, sizeof(buf));
    sce_forge_wire_open_container(&w, '[');
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_put_value(&w, sce_forge_wire_uint(7));
    if (sce_forge_wire_end_value(&w) || buf[0] != '\0') {
        fprintf(stderr, "FAIL: a value with a container left open was written as `%s`\n", buf);
        failures++;
    }
    sce_forge_wire_begin_value(&w, buf, sizeof(buf));
    sce_forge_wire_open_container(&w, '[');
    sce_forge_wire_refuse(&w);
    sce_forge_wire_close(&w);
    if (sce_forge_wire_end_value(&w) || buf[0] != '\0') {
        fprintf(stderr, "FAIL: a refused value was written as `%s`\n", buf);
        failures++;
    }
    /* Five bytes of room: `[7,8]` needs a sixth for its end. */
    sce_forge_wire_begin_value(&w, buf, 5u);
    sce_forge_wire_open_container(&w, '[');
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_put_value(&w, sce_forge_wire_uint(7));
    sce_forge_wire_next_in_container(&w);
    sce_forge_wire_put_value(&w, sce_forge_wire_uint(8));
    sce_forge_wire_close(&w);
    if (sce_forge_wire_end_value(&w) || buf[0] != '\0') {
        fprintf(stderr, "FAIL: a value past its buffer was cut to `%s`\n", buf);
        failures++;
    }
}

/* The path a refusal names a value by, joined from the prefix and each segment. */
static void check_paths(void) {
    char path[16];
    if (!sce_payload_path_join(path, sizeof(path), "items", "tags") || strcmp(path, "items/tags") != 0) {
        fprintf(stderr, "FAIL: a joined path reads `%s`\n", path);
        failures++;
    }
    if (!sce_payload_path_index(path, sizeof(path), path, 2u) || strcmp(path, "items/tags/2") != 0) {
        fprintf(stderr, "FAIL: a path joined in place reads `%s`\n", path);
        failures++;
    }
    if (!sce_payload_path_index(path, sizeof(path), "", 4294967295u) || strcmp(path, "4294967295") != 0) {
        fprintf(stderr, "FAIL: the widest index reads `%s`\n", path);
        failures++;
    }
    if (sce_payload_path_join(path, 8u, "items", "tags") || strcmp(path, "items") != 0) {
        fprintf(stderr, "FAIL: a path past its buffer reads `%s`\n", path);
        failures++;
    }
}

/* A top-level text and byte string are held to their bound when read, and a record
 * that is not an object, or whose schema's defaults are not one, is refused. */
static void check_top_level_bounds_and_records(void) {
    static const char data[] = "{\"note\":\"abcde\",\"blob\":\"abcde\",\"n\":null,\"s\":5,\"r\":{\"a\":1}}";
    sce_payload_fields_t fields;
    if (sce_payload_decode(data, &fields) != NULL) {
        fprintf(stderr, "FAIL: the payload did not read\n");
        failures++;
        return;
    }
    const char *v = NULL;
    char text[5];
    uint8_t bytes[4];
    size_t len = 0u;
    (void)sce_payload_field(&fields, "note", &v);
    if (!refused_as(sce_payload_value_text(v, text, 4u), SCE_PAYLOAD_REFUSE_PAST_BOUND)) {
        fprintf(stderr, "FAIL: a text past its bound was not refused as past it\n");
        failures++;
    }
    (void)sce_payload_field(&fields, "blob", &v);
    if (!refused_as(sce_payload_value_bytes(v, bytes, 4u, &len), SCE_PAYLOAD_REFUSE_PAST_BOUND)) {
        fprintf(stderr, "FAIL: a byte string past its bound was not refused as past it\n");
        failures++;
    }
    /* The buffer a build reserved is not a bound the type declares, and says so. */
    if (!refused_as(sce_payload_read_text(&fields, "note", text, sizeof(text)), SCE_PAYLOAD_REFUSE_TOO_LONG)) {
        fprintf(stderr, "FAIL: a text past a reserved buffer was not refused as too long\n");
        failures++;
    }
    int64_t n = 0;
    if (!refused_as(sce_payload_read_i64(&fields, "n", &n), SCE_PAYLOAD_REFUSE_NULL)) {
        fprintf(stderr, "FAIL: null where the type is not optional was not refused as null\n");
        failures++;
    }
    sce_payload_fields_t record;
    (void)sce_payload_field(&fields, "s", &v);
    if (!refused_as(sce_payload_record_open(v, NULL, &record), SCE_PAYLOAD_REFUSE_NOT_RECORD)) {
        fprintf(stderr, "FAIL: a number was opened as a record\n");
        failures++;
    }
    (void)sce_payload_field(&fields, "r", &v);
    if (!refused_as(sce_payload_record_open(v, "[]", &record), SCE_PAYLOAD_REFUSE_BAD_DEFAULTS)) {
        fprintf(stderr, "FAIL: defaults that are no object were taken\n");
        failures++;
    }
    sce_payload_list_t list;
    if (!refused_as(sce_payload_list_open(v, 2u, &list), SCE_PAYLOAD_REFUSE_NOT_LIST)) {
        fprintf(stderr, "FAIL: an object was opened as a list\n");
        failures++;
    }
    if (!refused_as(sce_payload_list_open("[1,2", 2u, &list), SCE_PAYLOAD_REFUSE_NOT_LIST)) {
        fprintf(stderr, "FAIL: an array that never closes was opened as a list\n");
        failures++;
    }
    /* A cursor whose list was refused hands out no element, whatever it counted. */
    if (!refused_as(sce_payload_list_open("[1,2,3]", 2u, &list), SCE_PAYLOAD_REFUSE_PAST_COUNT) ||
        sce_payload_list_next(&list, &v)) {
        fprintf(stderr, "FAIL: a list past its bound handed out an element\n");
        failures++;
    }
    if (!refused_as(sce_payload_field(&fields, "gone", &v), SCE_PAYLOAD_REFUSE_MISSING)) {
        fprintf(stderr, "FAIL: a missing field was found\n");
        failures++;
    }
    /* A list short of its lower count is refused when it is counted, before an
       element is handed out (docs/adr/0086, section 15.5), and classed tooFew. */
    if (!refused_as(sce_payload_list_open_within("[\"x\"]", 2u, 3u, &list), SCE_PAYLOAD_REFUSE_SHORT_COUNT) ||
        sce_payload_list_next(&list, &v) ||
        sce_payload_refusal_code(SCE_PAYLOAD_REFUSE_SHORT_COUNT) != SCE_PAYLOAD_CODE_TOO_FEW ||
        !sce_payload_code_has_bound(SCE_PAYLOAD_CODE_TOO_FEW)) {
        fprintf(stderr, "FAIL: a list short of its lower count handed out an element or was not tooFew\n");
        failures++;
    }
    if (!refused_as(sce_payload_list_open_within("[]", 1u, 3u, &list), SCE_PAYLOAD_REFUSE_SHORT_COUNT)) {
        fprintf(stderr, "FAIL: the empty list was opened where one element is needed\n");
        failures++;
    }
    if (sce_payload_list_open_within("[1,2]", 2u, 3u, &list) != NULL || list.count != 2u) {
        fprintf(stderr, "FAIL: a list that holds its lower count was refused\n");
        failures++;
    }
    if (!refused_as(sce_payload_list_open_within("[1,2,3,4]", 2u, 3u, &list), SCE_PAYLOAD_REFUSE_PAST_COUNT)) {
        fprintf(stderr, "FAIL: a list past its bound with a lower count was not refused past it\n");
        failures++;
    }
    if (sce_payload_list_open("[]", 3u, &list) != NULL) {
        fprintf(stderr, "FAIL: the empty list was refused where no element is needed\n");
        failures++;
    }
}

SCE_FORGE_BOUNDED_LIST_DECLARE(sce_list_2_uint8, uint8_t, 2)
SCE_FORGE_BOUNDED_LIST_DECLARE(sce_list_3_list_2_uint8, sce_list_2_uint8_t, 3)

/* A bounded list holds at most its bound, at every depth, and its layout is its
 * size at build time. */
static void check_bounded_list(void) {
    sce_list_2_uint8_t inner = {0};
    if (!sce_list_2_uint8_push(&inner, 1) || !sce_list_2_uint8_push(&inner, 2) || sce_list_2_uint8_push(&inner, 3) ||
        inner.len != 2u || inner.items[1] != 2) {
        fprintf(stderr, "FAIL: a list of two took a third element or lost one\n");
        failures++;
    }
    sce_list_3_list_2_uint8_t outer = {0};
    for (int i = 0; i < 3; ++i) {
        (void)sce_list_3_list_2_uint8_push(&outer, inner);
    }
    if (sce_list_3_list_2_uint8_push(&outer, inner) || !sce_list_3_list_2_uint8_within(&outer)) {
        fprintf(stderr, "FAIL: a list of three took a fourth element\n");
        failures++;
    }
    /* A value a host built past a bound, at any depth, is what an encode refuses. */
    outer.items[1].len = 3u;
    if (sce_list_2_uint8_within(&outer.items[1])) {
        fprintf(stderr, "FAIL: an inner list past its bound was held within it\n");
        failures++;
    }
    outer.len = 4u;
    if (sce_list_3_list_2_uint8_within(&outer)) {
        fprintf(stderr, "FAIL: an outer list past its bound was held within it\n");
        failures++;
    }
    /* A value written into a place whose type states a lower count is held to it
       (docs/adr/0086, section 15.6). */
    inner.len = 1u;
    if (!sce_list_2_uint8_holds_at_least(&inner, 1u) || sce_list_2_uint8_holds_at_least(&inner, 2u)) {
        fprintf(stderr, "FAIL: a list was not held to a lower count\n");
        failures++;
    }
    if (sizeof(sce_list_3_list_2_uint8_t) < 3u * sizeof(sce_list_2_uint8_t)) {
        fprintf(stderr, "FAIL: a list does not hold its bound in place\n");
        failures++;
    }
}

/* A union over `dot`, `circle` and `disc`, the last two holding one record (`radius`
   uint16), read as a generated reader reads it: the tag first, then the case's record
   from the same object (docs/adr/0092, section 4). Answers the refusal, `*at_tag` where
   it stands, and the case and radius read. */
static const char *read_shape(const char *data, size_t *kind, uint16_t *radius, bool *at_tag) {
    static const char *const kinds[] = {"dot", "circle", "disc"};
    sce_payload_fields_t fields;
    const char *s = NULL;
    *at_tag = false;
    *radius = 0u;
    const char *refusal = sce_payload_decode(data, &fields);
    if (refusal == NULL) {
        refusal = sce_payload_field(&fields, "s", &s);
    }
    if (refusal == NULL) {
        refusal = sce_payload_union_open(s, "kind", kinds, 3u, kind, at_tag);
    }
    if (refusal != NULL || *kind == 0u) {
        return refusal;
    }
    *at_tag = false;
    sce_payload_fields_t record;
    refusal = sce_payload_record_open(s, NULL, &record);
    return refusal != NULL ? refusal : sce_payload_read_u16(&record, "radius", radius);
}

/* A union's tag is read first, wherever it stands, and names the case whose record is
   read from the same object: two cases that hold one record stay apart by their tag, a
   key or a `null` the case does not declare is stepped over, and a refusal stands at the
   union, at its tag, or at the case's field. */
static void check_union(void) {
    const struct {
        const char *data;
        size_t kind;
        uint16_t radius;
    } readable[] = {
        {"{\"s\":{\"kind\":\"dot\"}}", 0u, 0u},
        {"{\"s\":{\"radius\":2,\"kind\":\"disc\"}}", 2u, 2u},
        {"{\"s\":{\"kind\":\"circle\",\"radius\":2,\"width\":null,\"text\":\"x\"}}", 1u, 2u},
        {"{\"s\":{\"kind\":\"dot\",\"radius\":null}}", 0u, 0u},
    };

    for (size_t i = 0u; i < sizeof(readable) / sizeof(readable[0]); ++i) {
        size_t kind = 99u;
        uint16_t radius = 0u;
        bool at_tag = false;
        const char *refusal = read_shape(readable[i].data, &kind, &radius, &at_tag);
        if (refusal != NULL || kind != readable[i].kind || radius != readable[i].radius) {
            fprintf(stderr, "FAIL: %s read case %u radius %u (%s)\n", readable[i].data, (unsigned)kind,
                    (unsigned)radius, refusal != NULL ? refusal : "read");
            failures++;
        }
    }

    const struct {
        const char *data;
        const char *refusal;
        bool at_tag;
    } refused[] = {
        {"{\"s\":[]}", SCE_PAYLOAD_REFUSE_NOT_UNION, false},
        {"{\"s\":null}", SCE_PAYLOAD_REFUSE_NULL, false},
        {"{\"s\":{\"radius\":2}}", SCE_PAYLOAD_REFUSE_MISSING, true},
        {"{\"s\":{\"kind\":null,\"radius\":null}}", SCE_PAYLOAD_REFUSE_NULL, true},
        {"{\"s\":{\"kind\":3,\"radius\":\"x\"}}", SCE_PAYLOAD_REFUSE_NOT_TEXT, true},
        {"{\"s\":{\"kind\":\"oval\",\"radius\":2}}", SCE_PAYLOAD_REFUSE_UNKNOWN_VARIANT, true},
        {"{\"s\":{\"kind\":\"circle\",\"width\":2}}", SCE_PAYLOAD_REFUSE_MISSING, false},
        {"{\"s\":{\"kind\":\"disc\",\"radius\":null}}", SCE_PAYLOAD_REFUSE_NULL, false},
    };

    for (size_t i = 0u; i < sizeof(refused) / sizeof(refused[0]); ++i) {
        size_t kind = 0u;
        uint16_t radius = 0u;
        bool at_tag = !refused[i].at_tag;
        const char *refusal = read_shape(refused[i].data, &kind, &radius, &at_tag);
        if (refusal != refused[i].refusal || at_tag != refused[i].at_tag) {
            fprintf(stderr, "FAIL: %s refused (%s) at the %s, the case says (%s) at the %s\n", refused[i].data,
                    refusal != NULL ? refusal : "nothing", at_tag ? "tag" : "union or field", refused[i].refusal,
                    refused[i].at_tag ? "tag" : "union or field");
            failures++;
        }
    }
}

/* A lift of a value read whole (docs/adr/0092, section 6.1 item 5): the object the text is
   handed to the reader of its type, which reads a union's tag first; the refusals before
   it are a lift's, with their pointer; and the path a reader leaves its refusal at is the
   refusal record's strict pointer, written whole token by token. */
static void check_a_value_is_lifted_whole(void) {
    static const char *const kinds[] = {"dot", "circle", "disc"};
    char pointer[16];
    const char *value = NULL;
    size_t kind = 99u;
    bool at_tag = false;
    const char *refusal =
        sce_payload_lift_value(" {\"kind\":\"disc\",\"radius\":4}", 2u, &value, pointer, sizeof(pointer));
    if (refusal != NULL || value == NULL || *value != '{' ||
        sce_payload_union_open(value, "kind", kinds, 3u, &kind, &at_tag) != NULL || kind != 2u) {
        fprintf(stderr, "FAIL: a union's text was not handed whole to its reader (%s)\n",
                refusal != NULL ? refusal : "read");
        failures++;
    }

    const struct {
        const char *text;
        const char *refusal;
        const char *pointer;
    } refused[] = {
        {"[1]", SCE_PAYLOAD_REFUSE_NOT_OBJECT, ""},
        {"  ", sce_payload_refuse_empty_text, ""},
        {"{\"kind\":\"path\",\"points\":[[1]]}", sce_payload_refuse_too_deep, "/points/0"},
    };

    for (size_t i = 0u; i < sizeof(refused) / sizeof(refused[0]); ++i) {
        value = "unread";
        refusal = sce_payload_lift_value(refused[i].text, 2u, &value, pointer, sizeof(pointer));
        if (refusal != refused[i].refusal || value != NULL || strcmp(pointer, refused[i].pointer) != 0) {
            fprintf(stderr, "FAIL: %s lifted as (%s) at '%s', the case says (%s) at '%s'\n", refused[i].text,
                    refusal != NULL ? refusal : "nothing", pointer, refused[i].refusal, refused[i].pointer);
            failures++;
        }
    }

    const struct {
        const char *path;
        size_t cap;
        bool whole;
        const char *pointer;
    } paths[] = {
        {"", 4u, true, ""},
        {"text", 8u, true, "/text"},
        {"items/2/tags", 16u, true, "/items/2/tags"},
        {"a~b", 8u, true, "/a~0b"},
        {"items/2/tags", 9u, false, "/items/2"},
        {"text", 5u, false, ""},
    };

    for (size_t i = 0u; i < sizeof(paths) / sizeof(paths[0]); ++i) {
        const bool whole = sce_payload_pointer_of_path(pointer, paths[i].cap, paths[i].path);
        if (whole != paths[i].whole || strcmp(pointer, paths[i].pointer) != 0) {
            fprintf(stderr, "FAIL: the path '%s' in %u bytes is the pointer '%s' (%s), the case says '%s' (%s)\n",
                    paths[i].path, (unsigned)paths[i].cap, pointer, whole ? "whole" : "cut", paths[i].pointer,
                    paths[i].whole ? "whole" : "cut");
            failures++;
        }
    }
}

int main(void) {
    check_union();
    check_a_value_is_lifted_whole();
    check_cross_product();
    check_maximal_sizes();
    check_the_writer_refuses_a_shape_it_cannot_close();
    check_a_composed_pair_is_written_part_by_part();
    check_a_value_is_written_alone();
    check_paths();
    check_top_level_bounds_and_records();
    check_bounded_list();
    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    printf("payload_composed: %d cross-product cases and every writer, path and bound check passed\n", cases);
    return 0;
}
