// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What every payload reader refuses, and how it names the refusal (docs/adr/0087,
// sections 2.3 and 2.4). The cases live in tests/json_text/payload_refusal.json, the
// table every runtime's reader is held to: each is read here through
// `sce/event_payload.h`, the code of each refusal told by `sce_payload_refusal_code`,
// its pointer composed as a generated reader composes it
// (`sce_payload_pointer_join`), and its bound the one the reader was handed. This
// reader took the first value of a repeated name, and held a text that carried
// U+0000 cut at that byte; the cases that do either hold it to refusing them.

#include <sce/event_payload.h>

#include "shared_table.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_JSON_PAYLOAD_REFUSAL_TABLE
#error "SCE_JSON_PAYLOAD_REFUSAL_TABLE names tests/json_text/payload_refusal.json; the build defines it"
#endif
#ifndef SCE_STDLIB_REFUSAL_CODE
#error "SCE_STDLIB_REFUSAL_CODE names stdlib/lift/refusal_code.scxml; the build defines it"
#endif

enum {
    FIELD_CAP = 256,
    POINTER_CAP = 64,
    NAME_CAP = 64,
    MIN_CASES = 40,
    LIST_CAP = 16,
    /* The most cases a union of the table declares. */
    UNION_CASES = 8,
    /* The room of the standard refusal record's `path` (sce:std/lift/refusal.scxml). */
    STANDARD_PATH_BOUND = 256
};

/* Where a refusal was made, and the bound it broke. */
typedef struct {
    char pointer[POINTER_CAP];
    uint32_t bound;
    bool has_bound;
} refused_at_t;

/* `refusal`, made at `pointer`, recorded in `at`: what every reader below answers. */
static const char *refuse_at(const char *refusal, const char *pointer, refused_at_t *at) {
    if (refusal != NULL) {
        snprintf(at->pointer, sizeof(at->pointer), "%s", pointer);
    }
    return refusal;
}

/* The value a descriptor member `key` holds, or NULL. */
static const char *member(const char *descriptor, const char *key) {
    sce_payload_fields_t fields = {descriptor, NULL};
    return sce_payload_find(&fields, key);
}

/* The bound a descriptor member `key` holds. */
static uint32_t bound_of(const char *descriptor, const char *key) {
    uint32_t bound = 0u;
    const char *value = member(descriptor, key);
    if (value == NULL || sce_payload_value_u32(value, &bound) != NULL) {
        fprintf(stderr, "FAIL: a descriptor's %s is no bound\n", key);
    }
    return bound;
}

/* The value at `value` read as the type the descriptor at `type` describes, one call
   per constructor, as a generated reader reads it; NULL when it reads. */
static const char *read_described(const char *type, const char *value, const char *pointer, refused_at_t *at) {
    if (*type == '"') {
        char scalar[NAME_CAP];
        (void)sce_payload_text_into(type, scalar, sizeof(scalar), false);
        const char *refusal = NULL;
        if (strcmp(scalar, "uint8") == 0) {
            uint8_t n = 0u;
            refusal = sce_payload_value_u8(value, &n);
        } else if (strcmp(scalar, "uint16") == 0) {
            uint16_t n = 0u;
            refusal = sce_payload_value_u16(value, &n);
        } else if (strcmp(scalar, "uint32") == 0) {
            uint32_t n = 0u;
            refusal = sce_payload_value_u32(value, &n);
        } else if (strcmp(scalar, "uint64") == 0) {
            uint64_t n = 0u;
            refusal = sce_payload_value_u64(value, &n);
        } else if (strcmp(scalar, "int8") == 0) {
            int8_t n = 0;
            refusal = sce_payload_value_i8(value, &n);
        } else if (strcmp(scalar, "int16") == 0) {
            int16_t n = 0;
            refusal = sce_payload_value_i16(value, &n);
        } else if (strcmp(scalar, "int32") == 0) {
            int32_t n = 0;
            refusal = sce_payload_value_i32(value, &n);
        } else if (strcmp(scalar, "int64") == 0) {
            int64_t n = 0;
            refusal = sce_payload_value_i64(value, &n);
        } else if (strcmp(scalar, "float32") == 0) {
            float x = 0.0f;
            refusal = sce_payload_value_f32(value, &x);
        } else if (strcmp(scalar, "float64") == 0) {
            double x = 0.0;
            refusal = sce_payload_value_f64(value, &x);
        } else if (strcmp(scalar, "bool") == 0) {
            bool b = false;
            refusal = sce_payload_value_bool(value, &b);
        } else if (strcmp(scalar, "string") == 0) {
            char text[FIELD_CAP];
            refusal = sce_payload_text_into(value, text, sizeof(text), false);
        } else {
            fprintf(stderr, "FAIL: the table names no type %s\n", scalar);
            refusal = SCE_PAYLOAD_REFUSE_NOT_TEXT;
        }
        return refuse_at(refusal, pointer, at);
    }
    if (*type == '[') {
        char name[NAME_CAP];
        const char *refusal = sce_payload_text_into(value, name, sizeof(name), false);
        if (refusal != NULL) {
            return refuse_at(refusal, pointer, at);
        }
        sce_payload_list_t names;
        const char *declared = NULL;
        (void)sce_payload_list_open(type, LIST_CAP, &names);
        while (sce_payload_list_next(&names, &declared)) {
            char variant[NAME_CAP];
            if (sce_payload_text_into(declared, variant, sizeof(variant), false) == NULL &&
                strcmp(variant, name) == 0) {
                return NULL;
            }
        }
        return refuse_at(SCE_PAYLOAD_REFUSE_UNKNOWN_VARIANT, pointer, at);
    }
    const char *inner = member(type, "optional");
    if (inner != NULL) {
        return sce_payload_is_null(value) ? NULL : read_described(inner, value, pointer, at);
    }
    if (member(type, "string") != NULL) {
        const uint32_t bound = bound_of(type, "string");
        char text[FIELD_CAP];
        const char *refusal = sce_payload_value_text(value, text, bound);
        at->bound = bound;
        return refuse_at(refusal, pointer, at);
    }
    if (member(type, "bytes") != NULL) {
        const uint32_t bound = bound_of(type, "bytes");
        uint8_t bytes[FIELD_CAP];
        size_t len = 0u;
        const char *refusal = sce_payload_value_bytes(value, bytes, bound, &len);
        at->bound = bound;
        return refuse_at(refusal, pointer, at);
    }
    inner = member(type, "list");
    if (inner != NULL) {
        const uint32_t bound = bound_of(type, "max");
        /* The lower count, where the case states one (docs/adr/0086, section 15). */
        const uint32_t least = member(type, "min") != NULL ? bound_of(type, "min") : 0u;
        sce_payload_list_t list;
        const char *refusal = sce_payload_list_open_within(value, least, bound, &list);
        if (refusal != NULL) {
            /* The bound a refusal names is the one it broke: the lower count for a
               list too short, the bound for one too long. */
            at->bound = (refusal == SCE_PAYLOAD_REFUSE_SHORT_COUNT) ? least : bound;
            return refuse_at(refusal, pointer, at);
        }
        const char *element = NULL;
        for (uint32_t index = 0u; sce_payload_list_next(&list, &element); ++index) {
            char below[POINTER_CAP];
            (void)sce_payload_pointer_index(below, sizeof(below), pointer, index);
            refusal = read_described(inner, element, below, at);
            if (refusal != NULL) {
                return refusal;
            }
        }
        return NULL;
    }
    const char *record = member(type, "record");
    if (record != NULL) {
        sce_payload_fields_t fields;
        const char *refusal = sce_payload_record_open(value, NULL, &fields);
        if (refusal != NULL) {
            return refuse_at(refusal, pointer, at);
        }
        const char *name = sce_payload_skip_space(record + 1);
        for (; name != NULL && *name == '"'; name = sce_payload_next_member(name)) {
            char id[NAME_CAP];
            (void)sce_payload_text_into(name, id, sizeof(id), false);
            char below[POINTER_CAP];
            (void)sce_payload_pointer_join(below, sizeof(below), pointer, id);
            const char *field = NULL;
            refusal = sce_payload_field(&fields, id, &field);
            if (refusal != NULL) {
                return refuse_at(refusal, below, at);
            }
            const char *field_type = sce_payload_skip_space(sce_payload_past_string(name));
            refusal = read_described(sce_payload_skip_space(field_type + 1), field, below, at);
            if (refusal != NULL) {
                return refusal;
            }
        }
        return NULL;
    }
    const char *described = member(type, "union");
    if (described != NULL) {
        /* The tag's enum is the cases' names, in the table's order; an empty case is
           null, any other the record it holds (docs/adr/0092, section 4). */
        char tag[NAME_CAP];
        (void)sce_payload_text_into(member(described, "tag"), tag, sizeof(tag), false);
        const char *cases = member(described, "cases");
        char held[UNION_CASES][NAME_CAP];
        const char *names[UNION_CASES] = {NULL};
        const char *records[UNION_CASES] = {NULL};
        size_t count = 0u;
        for (const char *name = sce_payload_skip_space(cases + 1); name != NULL && *name == '"' && count < UNION_CASES;
             name = sce_payload_next_member(name)) {
            (void)sce_payload_text_into(name, held[count], sizeof(held[count]), false);
            names[count] = held[count];
            const char *colon = sce_payload_skip_space(sce_payload_past_string(name));
            records[count] = sce_payload_skip_space(colon + 1);
            count++;
        }
        size_t index = 0u;
        bool at_tag = false;
        const char *refusal = sce_payload_union_open(value, tag, names, count, &index, &at_tag);
        if (refusal != NULL) {
            char below[POINTER_CAP];
            (void)sce_payload_pointer_join(below, sizeof(below), pointer, tag);
            return refuse_at(refusal, at_tag ? below : pointer, at);
        }
        return sce_payload_is_null(records[index]) ? NULL : read_described(records[index], value, pointer, at);
    }
    fprintf(stderr, "FAIL: a type of the table names no constructor\n");
    return refuse_at(SCE_PAYLOAD_REFUSE_NOT_TEXT, pointer, at);
}

static int check_case(const char *object, int index) {
    char name[FIELD_CAP];
    char data[FIELD_CAP];
    if (read_field(object, "name", name, sizeof(name), NULL) != NULL ||
        read_field(object, "data", data, sizeof(data), NULL) != NULL) {
        fprintf(stderr, "FAIL: case %d has no name or data\n", index);
        return 1;
    }
    const char *type = member(object, "type");
    refused_at_t at;
    memset(&at, 0, sizeof(at));
    sce_payload_fields_t fields;
    const char *refusal = NULL;
    if (member(object, "depth") != NULL) {
        /* A case that gives a depth is read as a `<sce:lift>` of the record of the one
           field `v` reads it (docs/adr/0087, section 2.1), its pointer held to `pathBound`
           bytes, the standard refusal's 256 where the case gives none. */
        const uint32_t depth = bound_of(object, "depth");
        const uint32_t path_bound = member(object, "pathBound") != NULL ? bound_of(object, "pathBound") : 256u;
        char lifted_at[STANDARD_PATH_BOUND + 1];
        const size_t cap = path_bound < STANDARD_PATH_BOUND ? path_bound + 1u : sizeof(lifted_at);
        refusal = refuse_at(sce_payload_lift(data, depth, NULL, &fields, lifted_at, cap), lifted_at, &at);
        at.bound = depth;
    } else {
        refusal = refuse_at(sce_payload_decode(data, &fields), "", &at);
    }
    const char *value = NULL;
    if (refusal == NULL) {
        refusal = refuse_at(sce_payload_field(&fields, "v", &value), "/v", &at);
    }
    if (refusal == NULL) {
        refusal = read_described(type, value, "/v", &at);
    }
    const char *want = member(object, "refusal");
    if (want == NULL) {
        if (refusal != NULL) {
            fprintf(stderr, "FAIL: %s: refused (%s), the table reads it\n", name, refusal);
            return 1;
        }
        return 0;
    }
    if (refusal == NULL) {
        fprintf(stderr, "FAIL: %s: read, the table refuses it\n", name);
        return 1;
    }
    char code[NAME_CAP];
    char pointer[POINTER_CAP];
    (void)read_field(want, "code", code, sizeof(code), NULL);
    (void)read_field(want, "pointer", pointer, sizeof(pointer), NULL);
    const char *bound = member(want, "bound");
    const bool want_bound = bound != NULL && !sce_payload_is_null(bound);
    uint32_t wanted = 0u;
    if (want_bound) {
        (void)sce_payload_value_u32(bound, &wanted);
    }
    const uint8_t got = sce_payload_refusal_code(refusal);
    const bool has_bound = sce_payload_code_has_bound(got);
    if (strcmp(sce_payload_code_name(got), code) != 0 || strcmp(at.pointer, pointer) != 0 || has_bound != want_bound ||
        (has_bound && at.bound != wanted)) {
        fprintf(stderr, "FAIL: %s: refused %s at '%s' bound %u (%s), the table says %s at '%s' bound %u\n", name,
                sce_payload_code_name(got), at.pointer, has_bound ? (unsigned)at.bound : 0u, refusal, code, pointer,
                (unsigned)wanted);
        return 1;
    }
    return 0;
}

/* The codes are the variants sce:std/lift/refusal_code.scxml declares, by the same names and
   at the same numbers. */
static int check_codes(void) {
    char *document = read_table(SCE_STDLIB_REFUSAL_CODE);
    if (document == NULL) {
        fprintf(stderr, "FAIL: cannot read %s\n", SCE_STDLIB_REFUSAL_CODE);
        return 1;
    }
    int failures = 0;
    int declared = 0;
    for (const char *at = strstr(document, "<sce:variant "); at != NULL; at = strstr(at + 1, "<sce:variant ")) {
        declared++;
    }
    /* SCE_PAYLOAD_CODE_TOO_FEW is the last code (docs/adr/0086, section 15.11). */
    if (declared != SCE_PAYLOAD_CODE_TOO_FEW) {
        fprintf(stderr, "FAIL: the enum declares %d variants\n", declared);
        failures++;
    }
    for (uint8_t code = SCE_PAYLOAD_CODE_NOT_JSON; code <= SCE_PAYLOAD_CODE_TOO_FEW; ++code) {
        char variant[NAME_CAP];
        snprintf(variant, sizeof(variant), "<sce:variant name=\"%s\" value=\"%u\"/>", sce_payload_code_name(code),
                 (unsigned)code);
        if (strstr(document, variant) == NULL) {
            fprintf(stderr, "FAIL: the enum does not declare %s\n", variant);
            failures++;
        }
    }
    free(document);
    return failures;
}

/* RFC 6901: `/` before each token, `~` written `~0` and `/` written `~1`, and a pointer
   that does not fit leaves the prefix alone. */
static int check_pointers(void) {
    int failures = 0;
    char out[16];

    const struct {
        const char *prefix;
        const char *name;
        const char *want;
    } cases[] = {{"", "items", "/items"},
                 {"/items", "2", "/items/2"},
                 {"", "a~b/c", "/a~0b~1c"},
                 {"", "~1", "/~01"},
                 {"", "", "/"}};

    for (size_t i = 0u; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        if (!sce_payload_pointer_join(out, sizeof(out), cases[i].prefix, cases[i].name) ||
            strcmp(out, cases[i].want) != 0) {
            fprintf(stderr, "FAIL: the pointer to '%s' below '%s' reads '%s'\n", cases[i].name, cases[i].prefix, out);
            failures++;
        }
    }
    if (!sce_payload_pointer_index(out, sizeof(out), "/v", 12u) || strcmp(out, "/v/12") != 0) {
        fprintf(stderr, "FAIL: the pointer to an index reads '%s'\n", out);
        failures++;
    }
    if (sce_payload_pointer_join(out, 8u, "/items", "tags") || strcmp(out, "/items") != 0) {
        fprintf(stderr, "FAIL: a pointer past its buffer reads '%s'\n", out);
        failures++;
    }
    return failures;
}

/* A lift refuses a container past its depth over the brackets alone, before the text is
   read as JSON, and names the container it refused, held to the room of the refusal's
   path by whole tokens (docs/adr/0087, sections 2.1 and 2.4). */
static int check_lift_depth(void) {
    const struct {
        const char *text;
        uint32_t depth;
        uint32_t path_bound;
        const char *pointer; /* NULL: no refusal */
    } cases[] = {
        {"{\"title\":\"a\",\"count\":1}", 1u, 256u, NULL},
        {"{\"title\":[\"a\"]}", 1u, 256u, "/title"},
        {"{\"extra\":{\"a\":1}}", 1u, 256u, "/extra"},
        {"[[1]]", 1u, 256u, "/0"},
        {"[1,2,[3]]", 1u, 256u, "/2"},
        {"{\"extra\":[[[[[[[", 1u, 256u, "/extra"},
        {"{\"title\":\"[[{\\\"[\",\"count\":1}", 1u, 256u, NULL},
        {"{\"title\":{\"a\":1}}", 2u, 256u, NULL},
        {"{\"title\":\"[[", 1u, 256u, NULL},
        {"{\"title\":\"x\",\"extra\":[1]}", 1u, 256u, "/extra"},
        {"{\"a\":{\"b\":[1]}}", 2u, 256u, "/a/b"},
        {"{\"a\":[0,{\"b\":[1]}]}", 3u, 256u, "/a/1/b"},
        {"{\"a/b~c\":[1]}", 1u, 256u, "/a~1b~0c"},
        {"{[1]}", 1u, 256u, ""},
        {"{\"a\":{\"abcdef\":[1]}}", 2u, 4u, "/a"},
        {"{\"a\":{\"bc\":[1]}}", 2u, 5u, "/a/bc"},
    };

    int failures = 0;
    for (size_t i = 0u; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        char pointer[STANDARD_PATH_BOUND + 1];
        const size_t cap = cases[i].path_bound + 1u;
        const char *refusal = sce_payload_lift_too_deep(cases[i].text, cases[i].depth, pointer, cap);
        if (cases[i].pointer == NULL) {
            if (refusal != NULL) {
                fprintf(stderr, "FAIL: %s: refused as too deep\n", cases[i].text);
                failures++;
            }
            continue;
        }
        if (sce_payload_refusal_code(refusal) != SCE_PAYLOAD_CODE_TOO_DEEP || strcmp(pointer, cases[i].pointer) != 0) {
            fprintf(stderr, "FAIL: %s: refused %s at '%s', the case says tooDeep at '%s'\n", cases[i].text,
                    sce_payload_code_name(sce_payload_refusal_code(refusal)), pointer, cases[i].pointer);
            failures++;
        }
    }
    return failures;
}

/* A lift's record reads what a delivery's would, a field it lacks out of the schema's
   defaults, and refuses defaults that are no object and a text that is empty. */
static int check_lift_defaults(void) {
    int failures = 0;
    sce_payload_fields_t fields;
    char pointer[POINTER_CAP];
    uint8_t w = 0u;
    const char *refusal = sce_payload_lift("{\"v\":1}", 1u, "{\"w\":7}", &fields, pointer, sizeof(pointer));
    if (refusal != NULL || sce_payload_read_u8(&fields, "w", &w) != NULL || w != 7u) {
        fprintf(stderr, "FAIL: a lift did not read its field out of the defaults (%s)\n",
                refusal != NULL ? refusal : "read");
        failures++;
    }
    refusal = sce_payload_lift("{\"v\":1}", 1u, "[]", &fields, pointer, sizeof(pointer));
    if (refusal != SCE_PAYLOAD_REFUSE_BAD_DEFAULTS || fields.object != NULL) {
        fprintf(stderr, "FAIL: a lift took defaults that are no object\n");
        failures++;
    }
    refusal = sce_payload_lift(NULL, 1u, NULL, &fields, pointer, sizeof(pointer));
    if (sce_payload_refusal_code(refusal) != SCE_PAYLOAD_CODE_NOT_JSON || strcmp(pointer, "") != 0) {
        fprintf(stderr, "FAIL: a lift of no text was not refused as notJson\n");
        failures++;
    }
    return failures;
}

/* An enum's name is compared as the characters it spells, at any length. */
static int check_variants(void) {
    static const char *const names[] = {"task", "note"};
    int failures = 0;
    size_t index = 99u;
    if (sce_payload_value_variant("\"note\"", names, 2u, &index) != NULL || index != 1u) {
        fprintf(stderr, "FAIL: the variant note was not read\n");
        failures++;
    }
    if (sce_payload_value_variant("\"\\u0074ask\"", names, 2u, &index) != NULL || index != 0u) {
        fprintf(stderr, "FAIL: a variant spelled with an escape was not read\n");
        failures++;
    }
    if (sce_payload_value_variant("\"a name longer than any variant declares\"", names, 2u, &index) !=
        SCE_PAYLOAD_REFUSE_UNKNOWN_VARIANT) {
        fprintf(stderr, "FAIL: a long name was not refused as no variant\n");
        failures++;
    }
    if (sce_payload_value_variant("null", names, 2u, &index) != SCE_PAYLOAD_REFUSE_NULL ||
        sce_payload_value_variant("1", names, 2u, &index) != SCE_PAYLOAD_REFUSE_NOT_TEXT ||
        sce_payload_value_variant("\"\\u0000\"", names, 2u, &index) != SCE_PAYLOAD_REFUSE_NUL_IN_TEXT) {
        fprintf(stderr, "FAIL: a value that is no name was not refused by its kind\n");
        failures++;
    }
    return failures;
}

int main(void) {
    int failures = 0;
    const int cases = walk_table(SCE_JSON_PAYLOAD_REFUSAL_TABLE, check_case, &failures);
    // A floor, not an equality: adding a case must not have to touch it, but a table
    // that stopped being read must not pass either.
    if (cases < MIN_CASES) {
        fprintf(stderr, "FAIL: the table produced only %d case(s)\n", cases);
        failures++;
    }
    failures += check_codes();
    failures += check_pointers();
    failures += check_lift_depth();
    failures += check_lift_defaults();
    failures += check_variants();
    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    printf("payload refusal: %d case(s) pass\n", cases);
    return 0;
}
