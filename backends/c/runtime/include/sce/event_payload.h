/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
   SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

   Lifting an event's typed `_event.data` view out of the data it carries.

   NL→IR Item C1 Path A gives a schema'd event a typed payload that the
   natively lowered guards read. One producer fills it: the generated
   `<machine>_raise_<event>_typed` inject seam. Every other producer — `<send>`
   with `<param>`, namelist or `<content>`, an invoke forwarding an event either
   way, autoforward, BasicHTTP, mesh — fills the event record's `data`, the wire
   §scxml-5.10 describes and §scxml-B-2-8-1 reads.

   Until this header the two never met, so the same guard answered differently
   depending on where its event came from, and a typed payload could not cross
   an invoke boundary at all. No data, a missing field, or a value of another
   type is a delivery outside its type, which the machine refuses whole where it
   takes the event: no transition is selected for it, and an `error.execution`
   that carries the refusal is taken in its place (docs/adr/0086, section 4.2),
   where a guard that cannot be evaluated is only false (§scxml-3.13).

   ⚠ HEADER-ONLY, and that is a requirement rather than a convenience: the C11
   EventSchema gate links NO runtime library, because a native value path that
   links nothing is the MCU-clean proof the whole lowering exists for. A lift in
   a library would have taken that proof away from every machine that uses it.
   `static inline` keeps it in the one translation unit each machine already is.

   ⚠ No allocator and no copy of the payload either: the fields keep a pointer
   into the event record's own `data`, and each read scans it for one name. A
   payload is tens of bytes and a schema names a handful of fields, so the scan
   is bounded by the same small numbers either way. What it does take from the C
   library is `<stdlib.h>`'s `strtoull` / `strtod` and `<string.h>`'s `strncmp`,
   which every freestanding toolchain this backend targets provides.

   ⚠ Every refusal is a STATIC sentence about the field, with no name in it:
   the caller composes "`<event>` payload: '<field>' <reason>" from its own
   generation-time literals, because this backend has no allocator to build a
   message with. Beside the sentence a refusal carries what a statechart reads as
   data (docs/adr/0087, section 2.4): its code, which
   `sce_payload_refusal_code` answers for the sentence's own storage, so one text
   can stand for two codes; the place as a strict RFC 6901 pointer, which the
   caller composes as it composes the path (`sce_payload_pointer_join`); and the
   bound the value broke, which is the bound the caller handed the reader.

   Before any field is read, the whole text is held to the JSON grammar, and an
   object that repeats a name at any depth is refused, as every runtime's reader
   refuses it (tests/json_text/payload_refusal.json). The check keeps no stack:
   each container is walked once for its own members, and a container inside it
   is stepped over by counting brackets, so the memory is the same for any depth.

   Cross-language siblings: `sce_runtime.event_payload` (Python),
   `sce.LiftPayload` (Go), `SCE::Common::EventPayloadFields` (C++). */

#ifndef SCE_EVENT_PAYLOAD_H
#define SCE_EVENT_PAYLOAD_H

#include <errno.h>
#include <float.h>
#include <math.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "sce/number_text.h"
#include "sce/portability.h"
#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ── The refusals, spelled once ─────────────────────────────────────────── */

/* The class of a refusal: the enum `sce:std/lift/refusal_code.scxml` declares, each
   code at the number that document gives it (docs/adr/0087, section 2.3). */
typedef enum {
    SCE_PAYLOAD_CODE_NOT_JSON = 1,
    SCE_PAYLOAD_CODE_NOT_OBJECT = 2,
    SCE_PAYLOAD_CODE_TOO_DEEP = 3,
    SCE_PAYLOAD_CODE_MISSING = 4,
    SCE_PAYLOAD_CODE_NULL_NOT_OPTIONAL = 5,
    SCE_PAYLOAD_CODE_WRONG_KIND = 6,
    SCE_PAYLOAD_CODE_BAD_NUMBER = 7,
    SCE_PAYLOAD_CODE_OUT_OF_RANGE = 8,
    SCE_PAYLOAD_CODE_TOO_LONG = 9,
    SCE_PAYLOAD_CODE_TOO_MANY = 10,
    SCE_PAYLOAD_CODE_UNKNOWN_VARIANT = 11,
    SCE_PAYLOAD_CODE_NOT_BYTE_TEXT = 12,
    SCE_PAYLOAD_CODE_NUL_IN_TEXT = 13,
    /* A list short of the lower count its type states (docs/adr/0086, section 15). */
    SCE_PAYLOAD_CODE_TOO_FEW = 14
} sce_payload_code_t;

/* Each sentence is one array, so a refusal is told by the storage it points at and
   not by its text: where one sentence stands for two classes (a number that is no
   width's, or below zero for an unsigned one), it is two arrays of one text. */
SCE_C_UNUSED static const char sce_payload_refuse_no_data[] = "the event carries no data";
SCE_C_UNUSED static const char sce_payload_refuse_not_object[] = "the event's data is not an object of named fields";
SCE_C_UNUSED static const char sce_payload_refuse_not_object_nor_json[] =
    "the event's data is not an object of named fields";
SCE_C_UNUSED static const char sce_payload_refuse_not_json[] = "the event's data is not JSON";
SCE_C_UNUSED static const char sce_payload_refuse_trailing[] = "the event's data carries more than one JSON value";
SCE_C_UNUSED static const char sce_payload_refuse_missing[] = "is missing from the event's data";
SCE_C_UNUSED static const char sce_payload_refuse_not_number[] = "is not a number";
SCE_C_UNUSED static const char sce_payload_refuse_not_whole[] = "is not a whole number";
SCE_C_UNUSED static const char sce_payload_refuse_not_digits[] = "is not a whole number written as digits";
SCE_C_UNUSED static const char sce_payload_refuse_negative_zero[] = "is not a whole number (-0)";
SCE_C_UNUSED static const char sce_payload_refuse_past_2_53[] =
    "is a number past 2^53 - 1, which is not exact everywhere; a 64-bit integer that large is written as a text of "
    "its digits";
SCE_C_UNUSED static const char sce_payload_refuse_not_in_width[] = "does not fit the width its schema declares";
/* Below zero, for an unsigned type: no spelling of its numbers at all. */
SCE_C_UNUSED static const char sce_payload_refuse_below_zero[] = "does not fit the width its schema declares";
/* A 64-bit integer as a number past every width: past 2^53 - 1 as well. */
SCE_C_UNUSED static const char sce_payload_refuse_wide_past_width[] = "does not fit the width its schema declares";
SCE_C_UNUSED static const char sce_payload_refuse_not_truth[] = "is not a truth value";
SCE_C_UNUSED static const char sce_payload_refuse_not_text[] = "is not a text";
/* A text whose escapes name no character, or a byte string's text that is no UTF-8. */
SCE_C_UNUSED static const char sce_payload_refuse_text_malformed[] = "is not a text";
SCE_C_UNUSED static const char sce_payload_refuse_too_long[] = "is longer than this build reserved room for";
SCE_C_UNUSED static const char sce_payload_refuse_not_one_byte[] =
    "carries a character above U+00FF, which no single byte spells";
SCE_C_UNUSED static const char sce_payload_refuse_nul_in_text[] = "carries U+0000, which no text holds";
SCE_C_UNUSED static const char sce_payload_refuse_unknown_variant[] = "is not a variant of its enum";
SCE_C_UNUSED static const char sce_payload_refuse_null[] = "is null, and its type is not optional";
SCE_C_UNUSED static const char sce_payload_refuse_not_list[] = "is not a list";
SCE_C_UNUSED static const char sce_payload_refuse_not_record[] = "is not a record";
/* A union is one object, its tag beside its case's fields (docs/adr/0092, section 2). */
SCE_C_UNUSED static const char sce_payload_refuse_not_union[] = "is not a union";
SCE_C_UNUSED static const char sce_payload_refuse_past_count[] = "holds more elements than its type declares";
/* A list short of the lower count its type states (docs/adr/0086, section 15). */
SCE_C_UNUSED static const char sce_payload_refuse_short_count[] = "holds fewer elements than its type declares";
SCE_C_UNUSED static const char sce_payload_refuse_past_bound[] = "is more bytes than its type declares";
SCE_C_UNUSED static const char sce_payload_refuse_bad_defaults[] =
    "the schema's defaults are not a JSON object, which the generator never writes";
/* What a `<sce:lift>` refuses before it reads a field (docs/adr/0087, section 2.1). */
SCE_C_UNUSED static const char sce_payload_refuse_empty_text[] = "the text is empty";
SCE_C_UNUSED static const char sce_payload_refuse_too_deep[] =
    "the text opens a container deeper than a value of its schema holds";

#define SCE_PAYLOAD_REFUSE_NO_DATA sce_payload_refuse_no_data
#define SCE_PAYLOAD_REFUSE_NOT_OBJECT sce_payload_refuse_not_object
#define SCE_PAYLOAD_REFUSE_NOT_JSON sce_payload_refuse_not_json
#define SCE_PAYLOAD_REFUSE_TRAILING sce_payload_refuse_trailing
#define SCE_PAYLOAD_REFUSE_MISSING sce_payload_refuse_missing
#define SCE_PAYLOAD_REFUSE_NOT_NUMBER sce_payload_refuse_not_number
#define SCE_PAYLOAD_REFUSE_NOT_WHOLE sce_payload_refuse_not_whole
#define SCE_PAYLOAD_REFUSE_NOT_DIGITS sce_payload_refuse_not_digits
#define SCE_PAYLOAD_REFUSE_PAST_2_53 sce_payload_refuse_past_2_53
#define SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH sce_payload_refuse_not_in_width
#define SCE_PAYLOAD_REFUSE_NOT_TRUTH sce_payload_refuse_not_truth
#define SCE_PAYLOAD_REFUSE_NOT_TEXT sce_payload_refuse_not_text
#define SCE_PAYLOAD_REFUSE_TOO_LONG sce_payload_refuse_too_long
#define SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE sce_payload_refuse_not_one_byte
#define SCE_PAYLOAD_REFUSE_NUL_IN_TEXT sce_payload_refuse_nul_in_text
#define SCE_PAYLOAD_REFUSE_UNKNOWN_VARIANT sce_payload_refuse_unknown_variant
/* The refusals of a value of a composed type (docs/adr/0086, section 4): each is said
   of the value at its path, which the caller composes from its generation-time
   literals and the indices (`sce_payload_path_index`). */
#define SCE_PAYLOAD_REFUSE_NULL sce_payload_refuse_null
#define SCE_PAYLOAD_REFUSE_NOT_LIST sce_payload_refuse_not_list
#define SCE_PAYLOAD_REFUSE_NOT_RECORD sce_payload_refuse_not_record
#define SCE_PAYLOAD_REFUSE_NOT_UNION sce_payload_refuse_not_union
#define SCE_PAYLOAD_REFUSE_PAST_COUNT sce_payload_refuse_past_count
#define SCE_PAYLOAD_REFUSE_SHORT_COUNT sce_payload_refuse_short_count
#define SCE_PAYLOAD_REFUSE_PAST_BOUND sce_payload_refuse_past_bound
#define SCE_PAYLOAD_REFUSE_BAD_DEFAULTS sce_payload_refuse_bad_defaults

/* The most bytes a sentence above holds, its NUL not counted (docs/adr/0086, section 8.6 (b)). A
   generated machine quotes a refusal in the reason it raises, and the generator sizes the machine's
   slot of `_event.data` by this number (`PAYLOAD_REFUSAL` in `sce-build/src/forge/c11_reasons.rs`),
   which the machine asserts against the value it compiles with. Each sentence is held to it here, so
   a longer one fails this header rather than a machine's slot. */
#define SCE_PAYLOAD_REFUSAL_MAX_LEN 120u
#define SCE_PAYLOAD_REFUSAL_FITS(sentence) (sizeof(sentence) - 1u <= (size_t)SCE_PAYLOAD_REFUSAL_MAX_LEN)
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_no_data), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_object), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_object_nor_json), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_json), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_trailing), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_missing), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_number), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_whole), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_digits), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_negative_zero), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_past_2_53), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_in_width), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_below_zero), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_wide_past_width), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_truth), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_text), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_text_malformed), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_too_long), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_one_byte), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_nul_in_text), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_unknown_variant), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_null), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_list), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_record), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_not_union), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_past_count), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_short_count), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_past_bound), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_bad_defaults), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_empty_text), "a refusal is past its bound");
SCE_STATIC_ASSERT(SCE_PAYLOAD_REFUSAL_FITS(sce_payload_refuse_too_deep), "a refusal is past its bound");

/* The class of `refusal`, a sentence a reader of this header answered, told by its
   storage; 0 for NULL, which is no refusal, and for any other text. */
SCE_C_UNUSED static inline uint8_t sce_payload_refusal_code(const char *refusal) {
    static const struct {
        const char *sentence;
        uint8_t code;
    } classes[] = {
        {sce_payload_refuse_no_data, SCE_PAYLOAD_CODE_NOT_JSON},
        {sce_payload_refuse_not_object, SCE_PAYLOAD_CODE_NOT_OBJECT},
        {sce_payload_refuse_not_object_nor_json, SCE_PAYLOAD_CODE_NOT_JSON},
        {sce_payload_refuse_not_json, SCE_PAYLOAD_CODE_NOT_JSON},
        {sce_payload_refuse_trailing, SCE_PAYLOAD_CODE_NOT_JSON},
        {sce_payload_refuse_missing, SCE_PAYLOAD_CODE_MISSING},
        {sce_payload_refuse_not_number, SCE_PAYLOAD_CODE_WRONG_KIND},
        {sce_payload_refuse_not_whole, SCE_PAYLOAD_CODE_BAD_NUMBER},
        {sce_payload_refuse_not_digits, SCE_PAYLOAD_CODE_BAD_NUMBER},
        {sce_payload_refuse_negative_zero, SCE_PAYLOAD_CODE_BAD_NUMBER},
        {sce_payload_refuse_past_2_53, SCE_PAYLOAD_CODE_BAD_NUMBER},
        {sce_payload_refuse_not_in_width, SCE_PAYLOAD_CODE_OUT_OF_RANGE},
        {sce_payload_refuse_below_zero, SCE_PAYLOAD_CODE_BAD_NUMBER},
        {sce_payload_refuse_wide_past_width, SCE_PAYLOAD_CODE_BAD_NUMBER},
        {sce_payload_refuse_not_truth, SCE_PAYLOAD_CODE_WRONG_KIND},
        {sce_payload_refuse_not_text, SCE_PAYLOAD_CODE_WRONG_KIND},
        {sce_payload_refuse_text_malformed, SCE_PAYLOAD_CODE_NOT_JSON},
        {sce_payload_refuse_too_long, SCE_PAYLOAD_CODE_TOO_LONG},
        {sce_payload_refuse_not_one_byte, SCE_PAYLOAD_CODE_NOT_BYTE_TEXT},
        {sce_payload_refuse_nul_in_text, SCE_PAYLOAD_CODE_NUL_IN_TEXT},
        {sce_payload_refuse_unknown_variant, SCE_PAYLOAD_CODE_UNKNOWN_VARIANT},
        {sce_payload_refuse_null, SCE_PAYLOAD_CODE_NULL_NOT_OPTIONAL},
        {sce_payload_refuse_not_list, SCE_PAYLOAD_CODE_WRONG_KIND},
        {sce_payload_refuse_not_record, SCE_PAYLOAD_CODE_WRONG_KIND},
        {sce_payload_refuse_not_union, SCE_PAYLOAD_CODE_WRONG_KIND},
        {sce_payload_refuse_past_count, SCE_PAYLOAD_CODE_TOO_MANY},
        {sce_payload_refuse_short_count, SCE_PAYLOAD_CODE_TOO_FEW},
        {sce_payload_refuse_past_bound, SCE_PAYLOAD_CODE_TOO_LONG},
        {sce_payload_refuse_bad_defaults, SCE_PAYLOAD_CODE_NOT_JSON},
        {sce_payload_refuse_empty_text, SCE_PAYLOAD_CODE_NOT_JSON},
        {sce_payload_refuse_too_deep, SCE_PAYLOAD_CODE_TOO_DEEP},
    };

    for (size_t i = 0u; i < sizeof(classes) / sizeof(classes[0]); ++i) {
        if (classes[i].sentence == refusal) {
            return classes[i].code;
        }
    }
    return 0u;
}

/* The name the enum declares `code` by (`notJson`); "" for a number it does not declare. */
SCE_C_UNUSED static inline const char *sce_payload_code_name(uint8_t code) {
    static const char *const names[] = {
        "",          "notJson",    "notObject", "tooDeep", "missing",        "nullNotOptional", "wrongKind",
        "badNumber", "outOfRange", "tooLong",   "tooMany", "unknownVariant", "notByteText",     "nulInText",
        "tooFew"};
    return code < sizeof(names) / sizeof(names[0]) ? names[code] : "";
}

/* Whether a refusal of class `code` names the bound the value broke: the bound the
   caller handed the reader, for a text or byte string (bytes) or a list (elements: its
   bound past which it holds too many, or its lower count short of which too few). */
SCE_C_UNUSED static inline bool sce_payload_code_has_bound(uint8_t code) {
    return code == SCE_PAYLOAD_CODE_TOO_LONG || code == SCE_PAYLOAD_CODE_TOO_MANY || code == SCE_PAYLOAD_CODE_TOO_FEW ||
           code == SCE_PAYLOAD_CODE_TOO_DEEP;
}

/* The fields an event's data names — a borrowed view, valid only while the
   event record it came from is. */
typedef struct {
    const char *object;   /* at the '{' of the payload object */
    const char *defaults; /* at the '{' of the schema's defaults, or NULL */
} sce_payload_fields_t;

/* ── Walking one payload ────────────────────────────────────────────────── */

SCE_C_UNUSED static inline const char *sce_payload_skip_space(const char *p) {
    while (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n') {
        p++;
    }
    return p;
}

/* Past one JSON string, from its opening quote. NULL when it never closes. */
SCE_C_UNUSED static inline const char *sce_payload_skip_string(const char *p) {
    if (*p != '"') {
        return NULL;
    }
    p++;
    while (*p != '\0') {
        if (*p == '\\') {
            if (p[1] == '\0') {
                return NULL;
            }
            p += 2;
            continue;
        }
        if (*p == '"') {
            return p + 1;
        }
        p++;
    }
    return NULL;
}

/* Past one object or array, counting nesting. A payload may carry fields this
   document's schema does not name, and a nested one still has to be walked past
   to reach the fields that follow. */
SCE_C_UNUSED static inline const char *sce_payload_skip_container(const char *p) {
    const char open = *p;
    const char close = (open == '{') ? '}' : ']';
    int depth = 0;
    while (*p != '\0') {
        if (*p == '"') {
            const char *after = sce_payload_skip_string(p);
            if (after == NULL) {
                return NULL;
            }
            p = after;
            continue;
        }
        if (*p == open) {
            depth++;
        } else if (*p == close) {
            depth--;
            if (depth == 0) {
                return p + 1;
            }
        }
        p++;
    }
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_skip_value(const char *p) {
    p = sce_payload_skip_space(p);
    if (*p == '"') {
        return sce_payload_skip_string(p);
    }
    if (*p == '{' || *p == '[') {
        return sce_payload_skip_container(p);
    }
    /* A number, or one of the three keywords. Either way it ends where the next
       member does. */
    while (*p != '\0' && *p != ',' && *p != '}' && *p != ']') {
        p++;
    }
    return p;
}

/* The next character of a JSON string's body, as a code point, and past one JSON
   string in the grammar; defined with the grammar's walk below. */
SCE_C_UNUSED static inline unsigned long sce_payload_next_character(const char **p);
SCE_C_UNUSED static inline const char *sce_payload_past_string(const char *p);

/* Whether the JSON string starting at `p` spells exactly `name`, an ASCII name of
   a schema's field. A name is the text it spells: a sender may write any character
   of a name as an escape, and every runtime reads the name the escapes spell
   (tests/json_text/payload_refusal.json). */
SCE_C_UNUSED static inline bool sce_payload_string_is(const char *p, const char *name) {
    /* Held to the grammar first, so that no escape is walked past the end of the text. */
    if (sce_payload_past_string(p) == NULL) {
        return false;
    }
    p++; /* past the opening quote */
    while (*name != '\0') {
        if (*p == '\0' || *p == '"') {
            return false;
        }
        if (sce_payload_next_character(&p) != (unsigned long)(unsigned char)*name) {
            return false;
        }
        name++;
    }
    return *p == '"';
}

/* The value `name` carries in the object at `object`, or NULL when it names none. */
SCE_C_UNUSED static inline const char *sce_payload_find_in(const char *object, const char *name) {
    if (object == NULL) {
        return NULL;
    }
    const char *p = sce_payload_skip_space(object + 1); /* past '{' */
    while (*p != '\0' && *p != '}') {
        if (*p != '"') {
            return NULL;
        }
        const bool matched = sce_payload_string_is(p, name);
        const char *after_key = sce_payload_skip_string(p);
        if (after_key == NULL) {
            return NULL;
        }
        p = sce_payload_skip_space(after_key);
        if (*p != ':') {
            return NULL;
        }
        p = sce_payload_skip_space(p + 1);
        if (matched) {
            return p;
        }
        const char *after_value = sce_payload_skip_value(p);
        if (after_value == NULL) {
            return NULL;
        }
        p = sce_payload_skip_space(after_value);
        if (*p == ',') {
            p = sce_payload_skip_space(p + 1);
            continue;
        }
        break;
    }
    return NULL;
}

/* ── Holding a text to the JSON grammar (RFC 8259) ──────────────────────── */

/* Four hex digits at `p`, or -1 when they are not; defined with the readers below. */
SCE_C_UNUSED static inline long sce_payload_hex4(const char *p);

/* Past one JSON string at `p` in RFC 8259's grammar (sections 7 and 8.2): closed, no
   character below U+0020 standing unescaped, every escape one of the eight short ones
   or `\u` and four hex digits, and a `\u` escape of the first half of a UTF-16 pair
   followed at once by one of the second half, a second half never alone. NULL when
   `p` spells none: a half of a character names no character, so the text is no JSON
   text every runtime reads alike (tests/json_text/payload_refusal.json). */
SCE_C_UNUSED static inline const char *sce_payload_past_string(const char *p) {
    if (*p != '"') {
        return NULL;
    }
    p++;
    while (*p != '"') {
        if ((unsigned char)*p < 0x20u) {
            return NULL;
        }
        if (*p == '\\') {
            p++;
            if (*p == 'u') {
                const long unit = sce_payload_hex4(p + 1);
                if (unit < 0) {
                    return NULL;
                }
                p += 4;
                if (unit >= 0xD800 && unit <= 0xDBFF) {
                    const long low = (p[1] == '\\' && p[2] == 'u') ? sce_payload_hex4(p + 3) : -1;
                    if (low < 0xDC00 || low > 0xDFFF) {
                        return NULL;
                    }
                    p += 6;
                } else if (unit >= 0xDC00 && unit <= 0xDFFF) {
                    return NULL;
                }
            } else if (*p == '\0' || strchr("\"\\/bfnrt", *p) == NULL) {
                return NULL;
            }
        }
        p++;
    }
    return p + 1;
}

/* Past one JSON number at `p` in RFC 8259's grammar (section 6): `-`, then `0` or a
   digit that is not `0` and more digits, then a fraction and an exponent, each with at
   least one digit. NULL when `p` spells none. */
SCE_C_UNUSED static inline const char *sce_payload_past_number(const char *p) {
    if (*p == '-') {
        p++;
    }
    if (*p == '0') {
        p++;
    } else if (*p >= '1' && *p <= '9') {
        while (*p >= '0' && *p <= '9') {
            p++;
        }
    } else {
        return NULL;
    }
    if (*p == '.') {
        p++;
        if (*p < '0' || *p > '9') {
            return NULL;
        }
        while (*p >= '0' && *p <= '9') {
            p++;
        }
    }
    if (*p == 'e' || *p == 'E') {
        p++;
        if (*p == '+' || *p == '-') {
            p++;
        }
        if (*p < '0' || *p > '9') {
            return NULL;
        }
        while (*p >= '0' && *p <= '9') {
            p++;
        }
    }
    return p;
}

/* Past one JSON value at `p`: a string, a number or a keyword by its grammar, and a
   container to the bracket that closes it, counting both kinds of bracket, its strings
   held to their grammar. What a container holds is held to the grammar by its own walk
   (`sce_payload_container_is_well_formed`). NULL when `p` spells no value. */
SCE_C_UNUSED static inline const char *sce_payload_past_value(const char *p) {
    if (*p == '"') {
        return sce_payload_past_string(p);
    }
    if (*p == '{' || *p == '[') {
        unsigned long depth = 0u;
        while (*p != '\0') {
            if (*p == '"') {
                p = sce_payload_past_string(p);
                if (p == NULL) {
                    return NULL;
                }
                continue;
            }
            if (*p == '{' || *p == '[') {
                depth++;
            } else if (*p == '}' || *p == ']') {
                depth--;
                if (depth == 0u) {
                    return p + 1;
                }
            }
            p++;
        }
        return NULL;
    }
    if (strncmp(p, "true", 4) == 0 || strncmp(p, "null", 4) == 0) {
        return p + 4;
    }
    if (strncmp(p, "false", 5) == 0) {
        return p + 5;
    }
    return sce_payload_past_number(p);
}

/* The next member's name of an object after the member whose name is at `name`, or
   NULL after the last; the object has been held to the grammar. */
SCE_C_UNUSED static inline const char *sce_payload_next_member(const char *name) {
    const char *p = sce_payload_past_string(name);
    if (p == NULL) {
        return NULL;
    }
    p = sce_payload_skip_space(p);
    if (*p != ':') {
        return NULL;
    }
    p = sce_payload_past_value(sce_payload_skip_space(p + 1));
    if (p == NULL) {
        return NULL;
    }
    p = sce_payload_skip_space(p);
    return (*p == ',') ? sce_payload_skip_space(p + 1) : NULL;
}

/* The next character of the JSON string whose body `*p` is in, as a code point,
   `*p` moved past it: an escape is the character it names, a pair of halves the one
   character they name together, and UTF-8 the character it spells. A byte that begins
   no character is itself, so two names compare as their bytes where they are no text. */
SCE_C_UNUSED static inline unsigned long sce_payload_next_character(const char **p) {
    const unsigned char c = (unsigned char)**p;
    if (c == '\\') {
        const char e = (*p)[1];
        *p += 2;
        switch (e) {
        case 'b':
            return '\b';
        case 'f':
            return '\f';
        case 'n':
            return '\n';
        case 'r':
            return '\r';
        case 't':
            return '\t';
        case 'u': {
            unsigned long code = (unsigned long)sce_payload_hex4(*p);
            *p += 4;
            if (code >= 0xD800u && code <= 0xDBFFu && (*p)[0] == '\\' && (*p)[1] == 'u') {
                const long low = sce_payload_hex4(*p + 2);
                if (low >= 0xDC00 && low <= 0xDFFF) {
                    code = 0x10000u + ((code - 0xD800u) << 10) + ((unsigned long)low - 0xDC00u);
                    *p += 6;
                }
            }
            return code;
        }
        default:
            return (unsigned char)e;
        }
    }
    size_t n = 0u;
    unsigned long code = c;
    if (c >= 0xF0u && c <= 0xF4u) {
        n = 3u;
        code = c & 0x07u;
    } else if (c >= 0xE0u) {
        n = 2u;
        code = c & 0x0Fu;
    } else if (c >= 0xC2u) {
        n = 1u;
        code = c & 0x1Fu;
    }
    for (size_t i = 1u; i <= n; ++i) {
        const unsigned char more = (unsigned char)(*p)[i];
        if ((more & 0xC0u) != 0x80u) {
            *p += 1;
            return c;
        }
        code = (code << 6) | (more & 0x3Fu);
    }
    *p += 1u + n;
    return code;
}

/* Whether the JSON strings at `a` and `b` spell the same name. */
SCE_C_UNUSED static inline bool sce_payload_same_name(const char *a, const char *b) {
    a++;
    b++;
    while (*a != '"' && *b != '"') {
        if (sce_payload_next_character(&a) != sce_payload_next_character(&b)) {
            return false;
        }
    }
    return *a == '"' && *b == '"';
}

/* Whether the container at `p` is well formed in what it holds itself: its members
   or elements separated by commas, each name a string followed by a colon, each value
   one JSON value, the bracket of its own kind at its end, and in an object no name
   twice. A container inside it is stepped over, and held to the grammar by its own
   walk. */
SCE_C_UNUSED static inline bool sce_payload_container_is_well_formed(const char *p) {
    const bool object = (*p == '{');
    const char close = object ? '}' : ']';
    const char *q = sce_payload_skip_space(p + 1);
    if (*q == close) {
        return true;
    }
    for (;;) {
        if (object) {
            q = sce_payload_past_string(q);
            if (q == NULL) {
                return false;
            }
            q = sce_payload_skip_space(q);
            if (*q != ':') {
                return false;
            }
            q = sce_payload_skip_space(q + 1);
        }
        q = sce_payload_past_value(q);
        if (q == NULL) {
            return false;
        }
        q = sce_payload_skip_space(q);
        if (*q == ',') {
            q = sce_payload_skip_space(q + 1);
            continue;
        }
        if (*q == close) {
            break;
        }
        return false;
    }
    if (object) {
        /* An object whose names repeat has no single meaning: a reader that takes the
           first and one that takes the last would read two values from one text. */
        for (const char *a = sce_payload_skip_space(p + 1); a != NULL; a = sce_payload_next_member(a)) {
            for (const char *b = sce_payload_next_member(a); b != NULL; b = sce_payload_next_member(b)) {
                if (sce_payload_same_name(a, b)) {
                    return false;
                }
            }
        }
    }
    return true;
}

/* Why the text at `text` is not exactly one JSON value, or NULL when it is: every
   container in it walked once for what it holds itself, in the order its bracket
   opens, so no call is made per level and nothing is kept per level. */
SCE_C_UNUSED static inline const char *sce_payload_check_text(const char *text) {
    const char *p = sce_payload_skip_space(text);
    const char *end = sce_payload_past_value(p);
    if (end == NULL) {
        return SCE_PAYLOAD_REFUSE_NOT_JSON;
    }
    for (const char *q = p; q < end;) {
        if (*q == '"') {
            q = sce_payload_past_string(q);
            continue;
        }
        if ((*q == '{' || *q == '[') && !sce_payload_container_is_well_formed(q)) {
            return SCE_PAYLOAD_REFUSE_NOT_JSON;
        }
        q++;
    }
    return (*sce_payload_skip_space(end) == '\0') ? NULL : SCE_PAYLOAD_REFUSE_TRAILING;
}

/* The value `name` carries: in the delivery, else in the schema's defaults when
   the decode was given them, else NULL. */
SCE_C_UNUSED static inline const char *sce_payload_find(const sce_payload_fields_t *fields, const char *name) {
    if (fields == NULL) {
        return NULL;
    }
    const char *delivered = sce_payload_find_in(fields->object, name);
    return (delivered != NULL) ? delivered : sce_payload_find_in(fields->defaults, name);
}

/* Read an event's data as the fields it names.

   Returns NULL when `out` was filled, else a static sentence saying why not.
   The JSON read is §scxml-B-2-8-1's second rung, the same one the script engine
   takes for the same string. */
SCE_C_UNUSED static inline const char *sce_payload_decode(const char *data, sce_payload_fields_t *out) {
    if (out == NULL) {
        return SCE_PAYLOAD_REFUSE_NOT_OBJECT;
    }
    out->object = NULL;
    out->defaults = NULL;
    if (data == NULL) {
        return SCE_PAYLOAD_REFUSE_NO_DATA;
    }
    const char *p = sce_payload_skip_space(data);
    if (*p == '\0') {
        return SCE_PAYLOAD_REFUSE_NO_DATA;
    }
    const char *not_json = sce_payload_check_text(p);
    if (*p != '{') {
        /* §scxml-B-2-8-1's other rungs: a bare value or a space-normalized
           string names no fields. One sentence, and two classes: one JSON value
           that is not an object, and a text that is not JSON at all. */
        return (not_json == NULL) ? SCE_PAYLOAD_REFUSE_NOT_OBJECT : sce_payload_refuse_not_object_nor_json;
    }
    if (not_json != NULL) {
        return not_json;
    }
    out->object = p;
    return NULL;
}

/* `sce_payload_decode` for a schema a later revision of which added fields:
   `defaults` is the JSON object of those fields' defaults, a literal the
   generated machine holds, and a field the delivery lacks is read out of it
   instead (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows").

   A delivery with no data at all lacks every field, so it is read as one that
   names none: a field of the first revision is then refused by name, and a
   schema all of whose fields came later reads its defaults. A field the
   delivery does carry is read as it came, and held to its type as any other
   value is. */
SCE_C_UNUSED static inline const char *sce_payload_decode_with_defaults(const char *data, const char *defaults,
                                                                        sce_payload_fields_t *out) {
    if (out == NULL) {
        return SCE_PAYLOAD_REFUSE_NOT_OBJECT;
    }
    out->object = NULL;
    out->defaults = NULL;
    if (data != NULL && *sce_payload_skip_space(data) != '\0') {
        const char *refusal = sce_payload_decode(data, out);
        if (refusal != NULL) {
            return refusal;
        }
    }
    if (defaults != NULL) {
        const char *d = sce_payload_skip_space(defaults);
        if (*d == '{') {
            out->defaults = d;
        }
    }
    return NULL;
}

/* ── Reading one value at its declared type ─────────────────────────────── */

/* Every reader comes in two forms. `sce_payload_value_<kind>(at, ...)` reads the
   value whose text begins at `at`, at whatever depth of the payload it stands: an
   element a list cursor handed out, a field of a record value. The
   `sce_payload_read_<kind>(fields, name, ...)` form finds the field `name` of an
   object first, and refuses a delivery that lacks it. */

/* Whether the value at `p` is the keyword `null`, and not a longer word that
   starts with it. */
SCE_C_UNUSED static inline bool sce_payload_is_null(const char *p) {
    if (strncmp(p, "null", 4) != 0) {
        return false;
    }
    const char after = p[4];
    return after == '\0' || after == ',' || after == '}' || after == ']' || after == ' ' || after == '\t' ||
           after == '\r' || after == '\n';
}

/* The refusal of `null` where the type is not optional, or `otherwise` of any
   other value of the wrong kind. */
SCE_C_UNUSED static inline const char *sce_payload_wrong_kind(const char *p, const char *otherwise) {
    return sce_payload_is_null(p) ? SCE_PAYLOAD_REFUSE_NULL : otherwise;
}

/* A whole number, as wide as this backend reads them, with the refusals the
   width-specific readers narrow further.

   ⚠ A truth value is not a number here. JSON spells both, and a schema that
   declared `uint32` and received `true` has been handed something its own type
   says cannot occur. */
SCE_C_UNUSED static inline const char *sce_payload_value_whole(const char *p, bool *negative, uint64_t *magnitude) {
    if (*p != '-' && (*p < '0' || *p > '9')) {
        return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_NUMBER);
    }
    *negative = (*p == '-');
    const char *digits = *negative ? p + 1 : p;
    if (*digits < '0' || *digits > '9') {
        return SCE_PAYLOAD_REFUSE_NOT_NUMBER;
    }
    errno = 0;
    char *end = NULL;
    const unsigned long long value = strtoull(digits, &end, 10);
    if (end == digits || errno == ERANGE) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    /* A fraction or an exponent is a different value, and rounding it would
       answer a guard for something nobody sent. */
    if (*end == '.' || *end == 'e' || *end == 'E') {
        return SCE_PAYLOAD_REFUSE_NOT_WHOLE;
    }
    /* `-0` is a JSON number and no spelling of a whole number: every runtime refuses
       it, as it refuses `"-0"` for a 64-bit integer. */
    if (*negative && value == 0u) {
        return sce_payload_refuse_negative_zero;
    }
    *magnitude = (uint64_t)value;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_whole(const sce_payload_fields_t *fields, const char *name,
                                                              bool *negative, uint64_t *magnitude) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_whole(p, negative, magnitude);
}

/* A 64-bit whole number: written as a text of its digits, or as a JSON number
   when it is exact in a double (|n| <= 2^53).

   A 64-bit integer crosses as a text because a reader that takes a JSON number
   through a double carries one past 2^53 with its low bits wrong and no error; a
   number past that is refused here on every engine, so the same payload gives
   the same field wherever it is read. */
SCE_C_UNUSED static inline const char *sce_payload_value_wide(const char *p, bool is_signed, bool *negative,
                                                              uint64_t *magnitude) {
    if (*p != '"') {
        const char *refusal = sce_payload_value_whole(p, negative, magnitude);
        if (refusal == NULL && *magnitude > (((uint64_t)1 << 53) - 1u)) {
            return SCE_PAYLOAD_REFUSE_PAST_2_53;
        }
        /* A 64-bit integer as a number is held to 2^53 - 1 however large the number
           is: one past every width is no spelling of it either. */
        return (refusal == SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH) ? sce_payload_refuse_wide_past_width : refusal;
    }
    p++;
    *negative = (*p == '-');
    if (*negative && !is_signed) {
        return SCE_PAYLOAD_REFUSE_NOT_DIGITS;
    }
    const char *digits = *negative ? p + 1 : p;
    uint64_t value = 0;
    const char *d = digits;
    for (; *d >= '0' && *d <= '9'; d++) {
        const uint64_t digit = (uint64_t)(*d - '0');
        if (value > (UINT64_MAX - digit) / 10u) {
            return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
        }
        value = value * 10u + digit;
    }
    /* Canonical: at least one digit, no leading zero, no `-0`, then the closing quote. */
    if (d == digits || *d != '"' || (*digits == '0' && d != digits + 1) || (*negative && value == 0u)) {
        return SCE_PAYLOAD_REFUSE_NOT_DIGITS;
    }
    *magnitude = value;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_wide(const sce_payload_fields_t *fields, const char *name,
                                                             bool is_signed, bool *negative, uint64_t *magnitude) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_wide(p, is_signed, negative, magnitude);
}

SCE_C_UNUSED static inline const char *sce_payload_fit_signed(bool negative, uint64_t magnitude, int64_t min,
                                                              int64_t max, int64_t *out) {
    if (negative) {
        /* -(min) does not fit an int64_t when min is its floor, so the
           comparison is made on the magnitude instead. */
        const uint64_t floor_magnitude = (uint64_t)(-(min + 1)) + 1u;
        if (magnitude > floor_magnitude) {
            return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
        }
        *out = (magnitude == floor_magnitude) ? min : -(int64_t)magnitude;
        return NULL;
    }
    if (magnitude > (uint64_t)max) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    *out = (int64_t)magnitude;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_value_signed(const char *p, int64_t min, int64_t max, int64_t *out) {
    bool negative = false;
    uint64_t magnitude = 0;
    const char *refusal = sce_payload_value_whole(p, &negative, &magnitude);
    if (refusal != NULL) {
        return refusal;
    }
    return sce_payload_fit_signed(negative, magnitude, min, max, out);
}

SCE_C_UNUSED static inline const char *sce_payload_read_signed(const sce_payload_fields_t *fields, const char *name,
                                                               int64_t min, int64_t max, int64_t *out) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_signed(p, min, max, out);
}

SCE_C_UNUSED static inline const char *sce_payload_value_unsigned(const char *p, uint64_t max, uint64_t *out) {
    bool negative = false;
    uint64_t magnitude = 0;
    const char *refusal = sce_payload_value_whole(p, &negative, &magnitude);
    if (refusal != NULL) {
        return refusal;
    }
    if (negative) {
        /* Below zero is no spelling of an unsigned number at any width. */
        return sce_payload_refuse_below_zero;
    }
    if (magnitude > max) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    *out = magnitude;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_unsigned(const sce_payload_fields_t *fields, const char *name,
                                                                 uint64_t max, uint64_t *out) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_unsigned(p, max, out);
}

/* One reader per declared width, in both forms. A value the width cannot hold
   is refused rather than wrapped — silently truncating is how a guard on a
   boundary answers for a value the document never received. */
#define SCE_PAYLOAD_SIGNED_READER(suffix, type, min, max)                                                              \
    SCE_C_UNUSED static inline const char *sce_payload_value_##suffix(const char *p, type *out) {                      \
        int64_t wide = 0;                                                                                              \
        const char *refusal = sce_payload_value_signed(p, (min), (max), &wide);                                        \
        if (refusal != NULL) {                                                                                         \
            return refusal;                                                                                            \
        }                                                                                                              \
        *out = (type)wide;                                                                                             \
        return NULL;                                                                                                   \
    }                                                                                                                  \
    SCE_C_UNUSED static inline const char *sce_payload_read_##suffix(const sce_payload_fields_t *fields,               \
                                                                     const char *name, type *out) {                    \
        const char *p = sce_payload_find(fields, name);                                                                \
        return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_##suffix(p, out);                          \
    }

#define SCE_PAYLOAD_UNSIGNED_READER(suffix, type, max)                                                                 \
    SCE_C_UNUSED static inline const char *sce_payload_value_##suffix(const char *p, type *out) {                      \
        uint64_t wide = 0;                                                                                             \
        const char *refusal = sce_payload_value_unsigned(p, (max), &wide);                                             \
        if (refusal != NULL) {                                                                                         \
            return refusal;                                                                                            \
        }                                                                                                              \
        *out = (type)wide;                                                                                             \
        return NULL;                                                                                                   \
    }                                                                                                                  \
    SCE_C_UNUSED static inline const char *sce_payload_read_##suffix(const sce_payload_fields_t *fields,               \
                                                                     const char *name, type *out) {                    \
        const char *p = sce_payload_find(fields, name);                                                                \
        return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_##suffix(p, out);                          \
    }

SCE_PAYLOAD_SIGNED_READER(i8, int8_t, INT8_MIN, INT8_MAX)
SCE_PAYLOAD_SIGNED_READER(i16, int16_t, INT16_MIN, INT16_MAX)
SCE_PAYLOAD_SIGNED_READER(i32, int32_t, INT32_MIN, INT32_MAX)
SCE_PAYLOAD_UNSIGNED_READER(u8, uint8_t, UINT8_MAX)
SCE_PAYLOAD_UNSIGNED_READER(u16, uint16_t, UINT16_MAX)
SCE_PAYLOAD_UNSIGNED_READER(u32, uint32_t, UINT32_MAX)

/* The 64-bit readers read the text of the digits, or a number exact in a double
   (`sce_payload_read_wide`). */
SCE_C_UNUSED static inline const char *sce_payload_value_i64(const char *p, int64_t *out) {
    bool negative = false;
    uint64_t magnitude = 0;
    const char *refusal = sce_payload_value_wide(p, true, &negative, &magnitude);
    if (refusal != NULL) {
        return refusal;
    }
    return sce_payload_fit_signed(negative, magnitude, INT64_MIN, INT64_MAX, out);
}

SCE_C_UNUSED static inline const char *sce_payload_read_i64(const sce_payload_fields_t *fields, const char *name,
                                                            int64_t *out) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_i64(p, out);
}

SCE_C_UNUSED static inline const char *sce_payload_value_u64(const char *p, uint64_t *out) {
    bool negative = false;
    uint64_t magnitude = 0;
    const char *refusal = sce_payload_value_wide(p, false, &negative, &magnitude);
    if (refusal != NULL) {
        return refusal;
    }
    if (negative) {
        return sce_payload_refuse_below_zero;
    }
    *out = magnitude;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_u64(const sce_payload_fields_t *fields, const char *name,
                                                            uint64_t *out) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_u64(p, out);
}

SCE_C_UNUSED static inline const char *sce_payload_value_floating(const char *p, double *out) {
    if (*p != '-' && (*p < '0' || *p > '9')) {
        return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_NUMBER);
    }
    errno = 0;
    char *end = NULL;
    const double value = strtod(p, &end);
    if (end == p) {
        return SCE_PAYLOAD_REFUSE_NOT_NUMBER;
    }
    if (errno == ERANGE) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    *out = value;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_floating(const sce_payload_fields_t *fields, const char *name,
                                                                 double *out) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_floating(p, out);
}

SCE_C_UNUSED static inline const char *sce_payload_value_f32(const char *p, float *out) {
    double wide = 0.0;
    const char *refusal = sce_payload_value_floating(p, &wide);
    if (refusal != NULL) {
        return refusal;
    }
    if (wide > (double)FLT_MAX || wide < -(double)FLT_MAX) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    *out = (float)wide;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_f32(const sce_payload_fields_t *fields, const char *name,
                                                            float *out) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_f32(p, out);
}

SCE_C_UNUSED static inline const char *sce_payload_value_f64(const char *p, double *out) {
    return sce_payload_value_floating(p, out);
}

SCE_C_UNUSED static inline const char *sce_payload_read_f64(const sce_payload_fields_t *fields, const char *name,
                                                            double *out) {
    return sce_payload_read_floating(fields, name, out);
}

SCE_C_UNUSED static inline const char *sce_payload_value_bool(const char *p, bool *out) {
    if (strncmp(p, "true", 4) == 0) {
        *out = true;
        return NULL;
    }
    if (strncmp(p, "false", 5) == 0) {
        *out = false;
        return NULL;
    }
    return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_TRUTH);
}

SCE_C_UNUSED static inline const char *sce_payload_read_bool(const sce_payload_fields_t *fields, const char *name,
                                                             bool *out) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_value_bool(p, out);
}

/* Whether the field `name` of an optional type (docs/adr/0055) carries a value:
   `*present` is false for `null`, the one spelling of an absent value, and true
   for anything else, which the reader of the field's type then reads and holds to
   that type. A delivery that lacks the key, where no default of a later revision
   names it either, is refused as missing: a missing key is a sender whose revision
   does not have the field, never an absent value. A field a later revision added
   with no default reads the `null` the schema's defaults hold for it, and so is
   absent. */
SCE_C_UNUSED static inline const char *sce_payload_read_presence(const sce_payload_fields_t *fields, const char *name,
                                                                 bool *present) {
    const char *p = sce_payload_find(fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    *present = !sce_payload_is_null(p);
    return NULL;
}

/* Where one JSON string's characters go as they are unescaped. `emit` answers
   false when it has no room left, or when the character has no one-byte
   spelling and the sink is a byte string. */
typedef struct {
    char *text;     /* set for a text field */
    uint8_t *bytes; /* set for a byte-string field */
    size_t cap;
    size_t len;
    bool above_one_byte;
    /* Whether `cap` is the bound the value's type declares (a text's bound and its
       terminator, a byte string's bound), so running out of room is a value past
       its bound and not a buffer this build sized too small. */
    bool bounded;
} sce_payload_sink_t;

SCE_C_UNUSED static inline bool sce_payload_emit(sce_payload_sink_t *sink, unsigned long code) {
    if (sink->bytes != NULL) {
        if (code > 0xFFu) {
            sink->above_one_byte = true;
            return false;
        }
        if (sink->len >= sink->cap) {
            return false;
        }
        sink->bytes[sink->len++] = (uint8_t)code;
        return true;
    }
    if (sink->len + 1u >= sink->cap) {
        return false;
    }
    sink->text[sink->len++] = (char)(code & 0xFFu);
    return true;
}

/* A code point a `\u` escape named, into a text sink as UTF-8 — a text field
   holds UTF-8, and a code point above U+007F is more than one byte of it. A
   byte-string sink takes it as the one byte it must be. */
SCE_C_UNUSED static inline bool sce_payload_emit_code_point(sce_payload_sink_t *sink, unsigned long code) {
    if (sink->bytes != NULL || code < 0x80u) {
        return sce_payload_emit(sink, code);
    }
    unsigned char utf8[4];
    size_t n;
    if (code < 0x800u) {
        utf8[0] = (unsigned char)(0xC0u | (code >> 6));
        utf8[1] = (unsigned char)(0x80u | (code & 0x3Fu));
        n = 2u;
    } else if (code < 0x10000u) {
        utf8[0] = (unsigned char)(0xE0u | (code >> 12));
        utf8[1] = (unsigned char)(0x80u | ((code >> 6) & 0x3Fu));
        utf8[2] = (unsigned char)(0x80u | (code & 0x3Fu));
        n = 3u;
    } else {
        utf8[0] = (unsigned char)(0xF0u | (code >> 18));
        utf8[1] = (unsigned char)(0x80u | ((code >> 12) & 0x3Fu));
        utf8[2] = (unsigned char)(0x80u | ((code >> 6) & 0x3Fu));
        utf8[3] = (unsigned char)(0x80u | (code & 0x3Fu));
        n = 4u;
    }
    for (size_t i = 0u; i < n; ++i) {
        if (!sce_payload_emit(sink, utf8[i])) {
            return false;
        }
    }
    return true;
}

/* The four hex digits after a `\u`, at `p`, or -1 when they are not four hex
   digits. Read digit by digit: `strtoul` would also take a sign or a leading
   space, which JSON does not. */
SCE_C_UNUSED static inline long sce_payload_hex4(const char *p) {
    long code = 0;
    for (int i = 0; i < 4; i++) {
        const char c = p[i];
        int digit;
        if (c >= '0' && c <= '9') {
            digit = c - '0';
        } else if (c >= 'a' && c <= 'f') {
            digit = c - 'a' + 10;
        } else if (c >= 'A' && c <= 'F') {
            digit = c - 'A' + 10;
        } else {
            return -1;
        }
        code = code * 16 + digit;
    }
    return code;
}

/* One JSON string's characters, unescaped, into `sink`. */
SCE_C_UNUSED static inline const char *sce_payload_walk_text(const char *p, sce_payload_sink_t *sink) {
    if (*p != '"') {
        return SCE_PAYLOAD_REFUSE_NOT_TEXT;
    }
    p++;
    while (*p != '\0' && *p != '"') {
        unsigned long code;
        /* Whether `code` is a code point a `\u` escape named, rather than one
           byte of the text as it came. */
        bool named = false;
        if (*p == '\\') {
            p++;
            switch (*p) {
            case '"':
                code = '"';
                break;
            case '\\':
                code = '\\';
                break;
            case '/':
                code = '/';
                break;
            case 'b':
                code = '\b';
                break;
            case 'f':
                code = '\f';
                break;
            case 'n':
                code = '\n';
                break;
            case 'r':
                code = '\r';
                break;
            case 't':
                code = '\t';
                break;
            case 'u': {
                const long unit = sce_payload_hex4(p + 1);
                if (unit < 0) {
                    return sce_payload_refuse_text_malformed;
                }
                p += 4;
                code = (unsigned long)unit;
                /* RFC 8259 §7: a character outside the Basic Multilingual
                   Plane is written as its UTF-16 surrogate pair, two escapes
                   that name one code point together. A half without its
                   other half names no character at all. */
                if (code >= 0xD800u && code <= 0xDBFFu) {
                    const long low = (p[1] == '\\' && p[2] == 'u') ? sce_payload_hex4(p + 3) : -1;
                    if (low < 0xDC00 || low > 0xDFFF) {
                        return sce_payload_refuse_text_malformed;
                    }
                    code = 0x10000u + ((code - 0xD800u) << 10) + ((unsigned long)low - 0xDC00u);
                    p += 6;
                } else if (code >= 0xDC00u && code <= 0xDFFFu) {
                    return sce_payload_refuse_text_malformed;
                }
                named = true;
                break;
            }
            default:
                return sce_payload_refuse_text_malformed;
            }
            p++;
        } else {
            code = (unsigned char)*p;
            p++;
            if (sink->bytes != NULL && code >= 0x80u) {
                /* A character the text wrote as itself is its UTF-8, and is the
                   one byte of the byte string it spells when it lies within
                   U+00FF: `C3 BF` is the byte 0xFF, as its escape is. One past
                   U+00FF is no byte; what is not UTF-8 is no text. */
                if ((code == 0xC2u || code == 0xC3u) && ((unsigned char)*p & 0xC0u) == 0x80u) {
                    code = ((code & 0x03u) << 6) | ((unsigned char)*p & 0x3Fu);
                    p++;
                } else if (code >= 0xC4u && code <= 0xF4u) {
                    sink->above_one_byte = true;
                    return SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE;
                } else {
                    return sce_payload_refuse_text_malformed;
                }
            }
        }
        if (!(named ? sce_payload_emit_code_point(sink, code) : sce_payload_emit(sink, code))) {
            if (sink->above_one_byte) {
                return SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE;
            }
            return sink->bounded ? SCE_PAYLOAD_REFUSE_PAST_BOUND : SCE_PAYLOAD_REFUSE_TOO_LONG;
        }
    }
    if (*p != '"') {
        return sce_payload_refuse_text_malformed;
    }
    return NULL;
}

/* A text field, copied into a caller-owned buffer of `cap` bytes including the
   terminator. A text longer than the buffer is refused rather than truncated: a
   guard comparing a cut-off text answers for a value nobody sent. */
SCE_C_UNUSED static inline const char *sce_payload_text_into(const char *p, char *out, size_t cap, bool bounded) {
    if (*p != '"') {
        return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_TEXT);
    }
    if (cap == 0u) {
        return bounded ? SCE_PAYLOAD_REFUSE_PAST_BOUND : SCE_PAYLOAD_REFUSE_TOO_LONG;
    }
    sce_payload_sink_t sink;
    memset(&sink, 0, sizeof(sink));
    sink.text = out;
    sink.cap = cap;
    sink.bounded = bounded;
    const char *refusal = sce_payload_walk_text(p, &sink);
    if (refusal != NULL) {
        return refusal;
    }
    /* A text holding U+0000 would end at it here, where a text is held
       NUL-terminated, so no runtime holds one and every reader refuses it
       (docs/adr/0087, section 2.3). A byte string may hold the byte zero. */
    if (memchr(out, '\0', sink.len) != NULL) {
        return SCE_PAYLOAD_REFUSE_NUL_IN_TEXT;
    }
    out[sink.len] = '\0';
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_text(const sce_payload_fields_t *fields, const char *name,
                                                             char *out, size_t cap) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_text_into(p, out, cap, false);
}

/* A text of at most `bound` UTF-8 bytes (docs/adr/0086, section 4.4), into `out`,
   which holds `bound + 1` bytes for the text and its terminator. One past the
   bound is refused at whatever depth it stands, and never cut. */
SCE_C_UNUSED static inline const char *sce_payload_value_text(const char *p, char *out, size_t bound) {
    return sce_payload_text_into(p, out, bound + 1u, true);
}

/* A byte-string field, copied into a caller-owned buffer of `cap` bytes, with
   the length written to `out_len`.

   JSON has no byte string, so the wire carries the byte-exact Latin-1 text the
   inject seam writes, and this reads it back the same way: every one of the 256
   values is one character and back. Printable ASCII — what a bytes guard
   compares — is the same bytes under either reading. */
SCE_C_UNUSED static inline const char *sce_payload_bytes_into(const char *p, uint8_t *out, size_t cap, bool bounded,
                                                              size_t *out_len) {
    if (*p != '"') {
        return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_TEXT);
    }
    sce_payload_sink_t sink;
    memset(&sink, 0, sizeof(sink));
    sink.bytes = out;
    sink.cap = cap;
    sink.bounded = bounded;
    const char *refusal = sce_payload_walk_text(p, &sink);
    if (refusal != NULL) {
        return refusal;
    }
    *out_len = sink.len;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_bytes(const sce_payload_fields_t *fields, const char *name,
                                                              uint8_t *out, size_t cap, size_t *out_len) {
    const char *p = sce_payload_find(fields, name);
    return (p == NULL) ? SCE_PAYLOAD_REFUSE_MISSING : sce_payload_bytes_into(p, out, cap, false, out_len);
}

/* A byte string of at most `bound` bytes, into `out`, which holds `bound` bytes;
   its length is written to `out_len`. One past the bound is refused. */
SCE_C_UNUSED static inline const char *sce_payload_value_bytes(const char *p, uint8_t *out, size_t bound,
                                                               size_t *out_len) {
    return sce_payload_bytes_into(p, out, bound, true, out_len);
}

/* ── Walking a composed value (docs/adr/0086, section 4.3) ──────────────── */

/* A cursor over the elements of a list value, handed out one at a time with no
   copy and no allocation. The elements are counted when the cursor is opened,
   and a list longer than its bound is refused then, before any element is read
   (ADR 0077, section 2). */
typedef struct {
    const char *next; /* at the next element, or at the closing ']' */
    uint32_t count;   /* how many elements the list holds */
    uint32_t index;   /* the index of the next element */
} sce_payload_list_t;

/* Open the list value at `p`, of at least `least` and at most `bound` elements
   (docs/adr/0086, section 15). Both counts are held when the elements are counted,
   before any is read: one past the bound is SCE_PAYLOAD_REFUSE_PAST_COUNT, one short
   of the lower count SCE_PAYLOAD_REFUSE_SHORT_COUNT, and the cursor hands out no
   element after either. A value that is no array, or an array that never closes, is
   not a list. */
SCE_C_UNUSED static inline const char *sce_payload_list_open_within(const char *p, uint32_t least, uint32_t bound,
                                                                    sce_payload_list_t *out) {
    /* A cursor whose list was refused hands out no element, whichever refusal
       it was: it is emptied before the first one. */
    out->next = p;
    out->count = 0u;
    out->index = 0u;
    if (*p != '[') {
        return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_LIST);
    }
    out->next = sce_payload_skip_space(p + 1);
    const char *q = out->next;
    if (*q == ']') {
        /* The empty list: no value of a type that needs an element. */
        return (least > 0u) ? SCE_PAYLOAD_REFUSE_SHORT_COUNT : NULL;
    }
    uint32_t count = 0u;
    for (;;) {
        if (count == UINT32_MAX) {
            return SCE_PAYLOAD_REFUSE_PAST_COUNT;
        }
        count++;
        if (count > bound) {
            /* Counted, never decoded: the walk stops at the first element past
               the bound. */
            return SCE_PAYLOAD_REFUSE_PAST_COUNT;
        }
        q = sce_payload_skip_value(q);
        if (q == NULL) {
            return SCE_PAYLOAD_REFUSE_NOT_LIST;
        }
        q = sce_payload_skip_space(q);
        if (*q == ',') {
            q = sce_payload_skip_space(q + 1);
            continue;
        }
        if (*q != ']') {
            return SCE_PAYLOAD_REFUSE_NOT_LIST;
        }
        if (count < least) {
            /* Counted, never decoded: the cursor hands out none of them. */
            return SCE_PAYLOAD_REFUSE_SHORT_COUNT;
        }
        out->count = count;
        return NULL;
    }
}

/* Open the list value at `p`, of at most `bound` elements: a list that states no
   lower count. A value that is no array, or an array that never closes, is not a
   list. */
SCE_C_UNUSED static inline const char *sce_payload_list_open(const char *p, uint32_t bound, sce_payload_list_t *out) {
    return sce_payload_list_open_within(p, 0u, bound, out);
}

/* The next element of an open list: `*element` is at its text, and false is
   answered once every element has been handed out. */
SCE_C_UNUSED static inline bool sce_payload_list_next(sce_payload_list_t *list, const char **element) {
    if (list->index >= list->count) {
        return false;
    }
    *element = list->next;
    list->index++;
    const char *after = sce_payload_skip_space(sce_payload_skip_value(list->next));
    list->next = (*after == ',') ? sce_payload_skip_space(after + 1) : after;
    return true;
}

/* Open the record value at `p` as the fields its schema's reader reads with the
   `sce_payload_read_*` readers. `defaults`, for a record whose schema a later
   revision added fields to, is the JSON object of their defaults, a literal the
   generated machine holds, and a field the value lacks is read out of it
   (docs/adr/0086, section 10.3); NULL for a schema with none. A key the schema
   does not declare is never asked for, so it is stepped over. */
SCE_C_UNUSED static inline const char *sce_payload_record_open(const char *p, const char *defaults,
                                                               sce_payload_fields_t *out) {
    out->object = NULL;
    out->defaults = NULL;
    if (*p != '{') {
        return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_RECORD);
    }
    if (defaults != NULL) {
        const char *d = sce_payload_skip_space(defaults);
        if (*d != '{') {
            return SCE_PAYLOAD_REFUSE_BAD_DEFAULTS;
        }
        out->defaults = d;
    }
    out->object = p;
    return NULL;
}

/* The value of the field `name` of `fields`, at `*value`: what a reader of a
   field of a composed type walks from. */
SCE_C_UNUSED static inline const char *sce_payload_field(const sce_payload_fields_t *fields, const char *name,
                                                         const char **value) {
    const char *p = sce_payload_find(fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    *value = p;
    return NULL;
}

/* The path below `prefix` of the element at `index`, or of the field `name`,
   written into `out` of `cap` bytes: what a refusal names a value at any depth by
   (`items/2/tags/0`). False when it does not fit, and `out` then holds the prefix
   alone, never a path cut in the middle of a segment. */
SCE_C_UNUSED static inline bool sce_payload_path_join(char *out, size_t cap, const char *prefix, const char *name) {
    if (cap == 0u) {
        return false;
    }
    const size_t head = (prefix != NULL) ? strlen(prefix) : 0u;
    const size_t tail = strlen(name);
    const size_t slash = (head > 0u) ? 1u : 0u;
    if (head + slash + tail + 1u > cap) {
        if (head < cap && prefix != NULL && prefix != out) {
            memmove(out, prefix, head);
            out[head] = '\0';
        }
        return false;
    }
    if (prefix != NULL && prefix != out) {
        memmove(out, prefix, head);
    }
    if (slash != 0u) {
        out[head] = '/';
    }
    memcpy(out + head + slash, name, tail);
    out[head + slash + tail] = '\0';
    return true;
}

/* The decimal digits of `position`, terminated, into `segment` of 11 bytes. */
SCE_C_UNUSED static inline void sce_payload_index_digits(char segment[11], uint32_t position) {
    char digits[11];
    size_t n = 0u;
    do {
        digits[n++] = (char)('0' + (position % 10u));
        position /= 10u;
    } while (position != 0u);
    for (size_t i = 0u; i < n; ++i) {
        segment[i] = digits[n - 1u - i];
    }
    segment[n] = '\0';
}

SCE_C_UNUSED static inline bool sce_payload_path_index(char *out, size_t cap, const char *prefix, uint32_t position) {
    char segment[11];
    sce_payload_index_digits(segment, position);
    return sce_payload_path_join(out, cap, prefix, segment);
}

/* The strict RFC 6901 pointer to the field `name` below the pointer `prefix` (empty or
   NULL for the whole text), written into `out` of `cap` bytes: `/` and the name with
   `~` written `~0` and `/` written `~1` (RFC 6901, section 3), what a refusal names a
   value by beside its sentence (`/items/2/tags/0`). False when it does not fit, and
   `out` then holds the prefix alone, never a pointer cut in the middle of a token. */
SCE_C_UNUSED static inline bool sce_payload_pointer_join(char *out, size_t cap, const char *prefix, const char *name) {
    if (cap == 0u) {
        return false;
    }
    const size_t head = (prefix != NULL) ? strlen(prefix) : 0u;
    size_t tail = 0u;
    for (const char *c = name; *c != '\0'; ++c) {
        tail += (*c == '~' || *c == '/') ? 2u : 1u;
    }
    if (head < cap && prefix != NULL && prefix != out) {
        memmove(out, prefix, head);
    }
    if (head + 1u + tail + 1u > cap) {
        if (head < cap) {
            out[head] = '\0';
        } else if (prefix != out) {
            out[0] = '\0';
        }
        return false;
    }
    size_t at = head;
    out[at++] = '/';
    for (const char *c = name; *c != '\0'; ++c) {
        if (*c == '~' || *c == '/') {
            out[at++] = '~';
            out[at++] = (*c == '~') ? '0' : '1';
        } else {
            out[at++] = *c;
        }
    }
    out[at] = '\0';
    return true;
}

/* The strict RFC 6901 pointer to the element at `position` below `prefix`. */
SCE_C_UNUSED static inline bool sce_payload_pointer_index(char *out, size_t cap, const char *prefix,
                                                          uint32_t position) {
    char segment[11];
    sce_payload_index_digits(segment, position);
    return sce_payload_pointer_join(out, cap, prefix, segment);
}

/* ── Reading a text as a record: `<sce:lift>` (docs/adr/0087, section 2.1) ── */

/* The place, in the container open at `level` when the scan of a lift reaches `refused`,
   that the scan stands at: whether the container is an array, the elements it has passed,
   and the text of the member a `:` named (NULL for none). */
typedef struct {
    bool array;
    uint64_t index;
    const char *member;
} sce_payload_scan_place_t;

/* The place at `level` (1 for the outermost container) when the scan of `text` reaches
   `refused`, walked as `sce_payload_lift_too_deep` walks it: the container at `level` is
   the last opened there, a text closed at that level is the name a `:` after it takes, and
   a `,` there passes an element. */
SCE_C_UNUSED static inline void sce_payload_scan_place(const char *text, const char *refused, uint32_t level,
                                                       sce_payload_scan_place_t *place) {
    uint32_t open = 0u;
    const char *last_text = NULL;
    place->array = false;
    place->index = 0u;
    place->member = NULL;
    for (const char *p = text; p < refused; ++p) {
        if (*p == '"') {
            const char *start = p;
            ++p;
            while (*p != '\0' && *p != '"') {
                p += (*p == '\\' && p[1] != '\0') ? 2 : 1;
            }
            if (open == level) {
                last_text = start;
            }
        } else if (*p == ':' && open == level) {
            place->member = last_text;
            last_text = NULL;
        } else if (*p == ',' && open == level) {
            place->member = NULL;
            last_text = NULL;
            place->index++;
        } else if (*p == '{' || *p == '[') {
            ++open;
            if (open == level) {
                place->array = (*p == '[');
                place->index = 0u;
                place->member = NULL;
                last_text = NULL;
            }
        } else if ((*p == '}' || *p == ']') && open > 0u) {
            --open;
        }
    }
}

/* The code point of the UTF-8 sequence at `*p`, `*p` moved past it; false when the bytes
   are no UTF-8: a stray continuation, an overlong form, a surrogate, or past U+10FFFF. */
SCE_C_UNUSED static inline bool sce_payload_utf8_at(const char **p, unsigned long *code) {
    const unsigned char lead = (unsigned char)**p;
    size_t more = 0u;
    unsigned long value = lead;
    if (lead >= 0xF0u && lead <= 0xF4u) {
        more = 3u;
        value = lead & 0x07u;
    } else if (lead >= 0xE0u && lead <= 0xEFu) {
        more = 2u;
        value = lead & 0x0Fu;
    } else if (lead >= 0xC2u && lead <= 0xDFu) {
        more = 1u;
        value = lead & 0x1Fu;
    } else if (lead >= 0x80u) {
        return false;
    }
    for (size_t i = 1u; i <= more; ++i) {
        const unsigned char next = (unsigned char)(*p)[i];
        if ((next & 0xC0u) != 0x80u) {
            return false;
        }
        value = (value << 6) | (next & 0x3Fu);
    }
    if ((more == 2u && (value < 0x800u || (value >= 0xD800u && value <= 0xDFFFu))) ||
        (more == 3u && (value < 0x10000u || value > 0x10FFFFu))) {
        return false;
    }
    *p += more + 1u;
    *code = value;
    return true;
}

/* `/` and the name the JSON string at `name` spells, as an RFC 6901 token (`~` written `~0`,
   `/` written `~1`), after the `*len` bytes of `pointer`, which holds `cap` bytes with its
   terminator; `*len` moves past it. False, and `pointer` cut back to its `*len` bytes, when
   the name is no text (no JSON string, not UTF-8, or holding U+0000) or the token does not
   fit. */
SCE_C_UNUSED static inline bool sce_payload_pointer_append_name(char *pointer, size_t cap, size_t *len,
                                                                const char *name) {
    size_t at = *len;
    bool written = sce_payload_past_string(name) != NULL && at + 1u < cap;
    if (written) {
        pointer[at++] = '/';
    }
    for (const char *c = name + 1; written && *c != '"';) {
        unsigned long code = 0u;
        if ((unsigned char)*c >= 0x80u) {
            written = sce_payload_utf8_at(&c, &code);
        } else {
            code = sce_payload_next_character(&c);
        }
        char bytes[4];
        size_t count = 0u;
        if (code == '~' || code == '/') {
            bytes[count++] = '~';
            bytes[count++] = (code == '~') ? '0' : '1';
        } else if (code < 0x80u) {
            bytes[count++] = (char)code;
        } else if (code < 0x800u) {
            bytes[count++] = (char)(0xC0u | (code >> 6));
            bytes[count++] = (char)(0x80u | (code & 0x3Fu));
        } else if (code < 0x10000u) {
            bytes[count++] = (char)(0xE0u | (code >> 12));
            bytes[count++] = (char)(0x80u | ((code >> 6) & 0x3Fu));
            bytes[count++] = (char)(0x80u | (code & 0x3Fu));
        } else {
            bytes[count++] = (char)(0xF0u | (code >> 18));
            bytes[count++] = (char)(0x80u | ((code >> 12) & 0x3Fu));
            bytes[count++] = (char)(0x80u | ((code >> 6) & 0x3Fu));
            bytes[count++] = (char)(0x80u | (code & 0x3Fu));
        }
        written = written && code != 0u && at + count < cap;
        for (size_t i = 0u; written && i < count; ++i) {
            pointer[at++] = bytes[i];
        }
    }
    if (cap > 0u) {
        pointer[written ? at : *len] = '\0';
    }
    if (written) {
        *len = at;
    }
    return written;
}

/* The pointer to the container `sce_payload_lift_too_deep` refused at `refused`, which
   stands in `levels` containers, written into `pointer` of `cap` bytes as that function
   says. */
SCE_C_UNUSED static inline void sce_payload_scan_pointer(const char *text, const char *refused, uint32_t levels,
                                                         char *pointer, size_t cap) {
    size_t len = 0u;
    if (cap == 0u) {
        return;
    }
    pointer[0] = '\0';
    for (uint32_t level = 1u; level <= levels; ++level) {
        sce_payload_scan_place_t place;
        sce_payload_scan_place(text, refused, level, &place);
        if (place.array) {
            char digits[21];
            size_t n = 0u;
            uint64_t index = place.index;
            do {
                digits[n++] = (char)('0' + (int)(index % 10u));
                index /= 10u;
            } while (index != 0u);
            if (len + 1u + n >= cap) {
                break;
            }
            pointer[len++] = '/';
            while (n > 0u) {
                pointer[len++] = digits[--n];
            }
            pointer[len] = '\0';
        } else if (place.member == NULL || !sce_payload_pointer_append_name(pointer, cap, &len, place.member)) {
            break;
        }
    }
}

/* The refusal of a container `text` opens deeper than `depth`, the deepest a value of
   a lift's schema holds (docs/adr/0087, section 2.1, item 4), or NULL when it opens
   none. The bound of the refusal is `depth`.

   Decided over the brackets alone, before the text is read as JSON, so that every
   runtime decides it alike and no reader descends past the type: a text is skipped from
   its opening quote to the next quote no backslash escapes, each `{` and `[` outside one
   opens a level and each `}` and `]` closes one, and the first opened past `depth` is
   refused, whatever the text holds after it. A text that never closes ends the walk with
   nothing refused, and the reading of the text as JSON refuses it.

   `pointer`, of `cap` bytes, receives the place of the container refused (docs/adr/0087,
   section 2.4): for each container it stands in, outermost first, the element's index in
   an array (the `,` the scan met at that level since the array opened) or the member's
   name in an object (the last text closed at that level before a `:`, since the object
   opened or its last `,`), each an RFC 6901 token. It stops before the first place it
   cannot write: a member with no name before its colon, a name that is no text (no JSON
   string, not UTF-8, or holding U+0000), or a token that would take the pointer past
   `cap - 1` bytes, the room of the refusal record's `path`.

   No allocator and no stack of levels: the scan counts brackets, and once it refuses, the
   place at each level is found by walking the text again up to the refused bracket
   (`sce_payload_scan_place`), so the memory is the same for any depth and the work is
   bounded by `depth` walks of the text. */
SCE_C_UNUSED static inline const char *sce_payload_lift_too_deep(const char *text, uint32_t depth, char *pointer,
                                                                 size_t cap) {
    uint32_t open = 0u;
    if (cap > 0u) {
        pointer[0] = '\0';
    }
    for (const char *p = text; *p != '\0'; ++p) {
        if (*p == '"') {
            ++p;
            while (*p != '\0' && *p != '"') {
                p += (*p == '\\' && p[1] != '\0') ? 2 : 1;
            }
            if (*p == '\0') {
                return NULL;
            }
        } else if (*p == '{' || *p == '[') {
            if (open >= depth) {
                sce_payload_scan_pointer(text, p, open, pointer, cap);
                return sce_payload_refuse_too_deep;
            }
            ++open;
        } else if ((*p == '}' || *p == ']') && open > 0u) {
            --open;
        }
    }
    return NULL;
}

/* Read the text a `<sce:lift>` reads as the fields of a record of a schema whose deepest
   value holds `depth` levels of containers. `defaults`, for a schema a later revision of
   which added fields, is the JSON object of their defaults, a literal the generated machine
   holds; NULL for none. Answers NULL when `out` was filled, else a static sentence of the
   refusal, `pointer` (of `cap` bytes, the room of the refusal record's `path` and its
   terminator) holding its place: "" but for a container past the depth
   (`sce_payload_lift_too_deep`).

   Three refusals come before the fields are read, in this order: a text that is NULL,
   empty or only whitespace (`notJson`); a container opened deeper than `depth`
   (`sce_payload_lift_too_deep`, `tooDeep`); and a text that is not one JSON object, as a
   delivery is refused (`sce_payload_decode`). */
SCE_C_UNUSED static inline const char *sce_payload_lift(const char *text, uint32_t depth, const char *defaults,
                                                        sce_payload_fields_t *out, char *pointer, size_t cap) {
    out->object = NULL;
    out->defaults = NULL;
    if (cap > 0u) {
        pointer[0] = '\0';
    }
    if (text == NULL || *sce_payload_skip_space(text) == '\0') {
        return sce_payload_refuse_empty_text;
    }
    const char *refusal = sce_payload_lift_too_deep(text, depth, pointer, cap);
    if (refusal == NULL) {
        refusal = sce_payload_decode(text, out);
    }
    if (refusal == NULL && defaults != NULL) {
        const char *d = sce_payload_skip_space(defaults);
        if (*d != '{') {
            out->object = NULL;
            return SCE_PAYLOAD_REFUSE_BAD_DEFAULTS;
        }
        out->defaults = d;
    }
    return refusal;
}

/* The variant of an enum the value at `p` names, as its index in the `count` names of
   `names` into `*index`: a text, compared with each declared name as the characters it
   spells, so a name of any length is compared in place and one no variant declares is
   `unknownVariant`, as on every runtime. `null` is refused as a value of a type that is
   not optional, any other value as not a text, and a text holding U+0000 as no text. */
SCE_C_UNUSED static inline const char *sce_payload_value_variant(const char *p, const char *const *names, size_t count,
                                                                 size_t *index) {
    if (*p != '"') {
        return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_TEXT);
    }
    for (const char *c = p + 1; *c != '"' && *c != '\0';) {
        if (sce_payload_next_character(&c) == 0u) {
            return SCE_PAYLOAD_REFUSE_NUL_IN_TEXT;
        }
    }
    for (size_t i = 0u; i < count; ++i) {
        if (sce_payload_string_is(p, names[i])) {
            *index = i;
            return NULL;
        }
    }
    return SCE_PAYLOAD_REFUSE_UNKNOWN_VARIANT;
}

/* Open the union value at `p` (docs/adr/0092, section 4): one object holding its tag
   under the key `tag`, a name of the tag's enum, and beside it the fields of the case the
   tag names. The tag is read first, wherever it stands in the object, as
   `sce_payload_value_variant` reads an enum against the `count` names of `names`, and the
   index of the case it names is written to `*index`. The caller then opens the case's
   record at `p` with `sce_payload_record_open`, under that record's own defaults, and
   reads its fields from the same object; an empty case reads none. A key the case does
   not declare is never asked for, so it is stepped over.

   The refusals come in one order on every runtime: a value that is no object
   (SCE_PAYLOAD_REFUSE_NOT_UNION, or SCE_PAYLOAD_REFUSE_NULL for `null`); then the tag,
   absent (SCE_PAYLOAD_REFUSE_MISSING) or refused as the enum's reader refuses it. The
   caller composes the pointer, so `*at_tag` says where the refusal stands: true at the
   tag (the union's pointer joined with `tag`), false at the union itself. */
SCE_C_UNUSED static inline const char *sce_payload_union_open(const char *p, const char *tag, const char *const *names,
                                                              size_t count, size_t *index, bool *at_tag) {
    *at_tag = false;
    if (*p != '{') {
        return sce_payload_wrong_kind(p, SCE_PAYLOAD_REFUSE_NOT_UNION);
    }
    *at_tag = true;
    const char *tag_value = sce_payload_find_in(p, tag);
    if (tag_value == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    return sce_payload_value_variant(tag_value, names, count, index);
}

/* The short form JSON gives a control character, or 0 when it has none. */
SCE_C_UNUSED static inline char sce_payload_short_escape(unsigned char c) {
    switch (c) {
    case '"':
        return '"';
    case '\\':
        return '\\';
    case '\b':
        return 'b';
    case '\f':
        return 'f';
    case '\n':
        return 'n';
    case '\r':
        return 'r';
    case '\t':
        return 't';
    default:
        return 0;
    }
}

/* Quote an explicit span. Text is written in the one form every engine writes
   a string in (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"): `"`
   and `\` escaped, the five short forms, every other U+0000-U+001F as \u00xx
   in lowercase hex, everything else as it is, keeping its UTF-8 bytes. A byte
   payload uses Latin-1 code points, so a high byte is escaped as \u00xx
   rather than passed through a C string, which the reader above decodes back
   to that byte. Failure never exposes a partial JSON string. */
SCE_C_UNUSED static inline bool sce_payload_quote_span(const unsigned char *text, size_t len, bool latin1, char *out,
                                                       size_t cap) {
    static const char HEX[] = "0123456789abcdef";
    if (cap > 0u) {
        out[0] = '\0';
    }
    if (cap < 3u || (text == NULL && len != 0u)) {
        return false;
    }
    size_t at = 1u;
    out[0] = '"';
    for (size_t i = 0u; i < len; ++i) {
        const unsigned char c = text[i];
        const char short_form = sce_payload_short_escape(c);
        const bool unicode = short_form == 0 && (c < 0x20u || (latin1 && c >= 0x80u));
        const size_t needed = unicode ? 6u : (short_form != 0 ? 2u : 1u);
        /* Reserve both the closing quote and the terminator. Subtraction
           avoids overflow even when the caller passes a very large span. */
        if (needed > cap - at - 2u) {
            out[0] = '\0';
            return false;
        }
        if (unicode) {
            out[at++] = '\\';
            out[at++] = 'u';
            out[at++] = '0';
            out[at++] = '0';
            out[at++] = HEX[c >> 4];
            out[at++] = HEX[c & 0xFu];
        } else if (short_form != 0) {
            out[at++] = '\\';
            out[at++] = short_form;
        } else {
            out[at++] = (char)c;
        }
    }
    out[at++] = '"';
    out[at] = '\0';
    return true;
}

SCE_C_UNUSED static inline bool sce_payload_quote_bytes(const uint8_t *bytes, size_t len, char *out, size_t cap) {
    return sce_payload_quote_span(bytes, len, true, out, cap);
}

SCE_C_UNUSED static inline bool sce_payload_quote(const char *text, char *out, size_t cap) {
    return sce_payload_quote_span((const unsigned char *)text, text != NULL ? strlen(text) : 0u, false, out, cap);
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_EVENT_PAYLOAD_H */
