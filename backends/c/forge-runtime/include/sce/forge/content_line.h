/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge — the content lines (RFC 5545 §3.1) a
 * `sce:encoding="content-line"` codec reads and writes (SCE_FORGE.md §4.6.4,
 * docs/adr/0010): `BEGIN:<component>`, properties — a name, `;`-separated
 * parameters, `:` and a value — and `END:<component>`, folded at 75 octets.
 *
 * Mirrors `backends/rust/forge-runtime/src/content_line.rs`, rule for rule. A
 * generated codec calls these; it spells no line grammar of its own. The rules
 * are written once in SCE_FORGE.md §4.6.4 and implemented once per backend
 * runtime.
 *
 * No allocation and no recursion, as the rest of this runtime: a string is
 * read into the caller's fixed array, and every refusal is a
 * `sce_forge_codec_status_t`.
 */

#ifndef SCE_FORGE_CONTENT_LINE_H
#define SCE_FORGE_CONTENT_LINE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sce/forge/codec.h"

#ifdef __cplusplus
extern "C" {
#endif

/* The most octets of one physical line (SCE_FORGE.md §4.6.4, Folding). */
enum { SCE_FORGE_CL_FOLD_WIDTH = 75 };

/* A control character a value never holds: below U+0020 but the tab, and
 * U+007F. */
static inline bool sce_forge_cl_is_control(int b) {
    return (b < 0x20 && b != 0x09) || b == 0x7F;
}

/* A byte of a property or parameter name: letters, digits and hyphens. */
static inline bool sce_forge_cl_is_name_byte(int b) {
    return (b >= '0' && b <= '9') || (b >= 'A' && b <= 'Z') || (b >= 'a' && b <= 'z') || b == '-';
}

static inline int sce_forge_cl_lower(int b) {
    return (b >= 'A' && b <= 'Z') ? b + 0x20 : b;
}

/* A walk over the bytes of one logical line, raw[pos, limit), with its folds
 * removed as it goes: a line break and the one space or tab after it are not
 * part of the text, wherever the sender cut. */
typedef struct {
    const uint8_t *raw;
    size_t limit;
    size_t pos;
} sce_forge_cl_scan_t;

static inline bool sce_forge_cl_scan_blank(const sce_forge_cl_scan_t *s, size_t i) {
    return i < s->limit && (s->raw[i] == ' ' || s->raw[i] == '\t');
}

/* Step over every fold at the current position. */
static inline void sce_forge_cl_scan_skip_folds(sce_forge_cl_scan_t *s) {
    for (;;) {
        size_t skipped = 0;
        if (s->pos + 1 < s->limit && s->raw[s->pos] == '\r' && s->raw[s->pos + 1] == '\n' &&
            sce_forge_cl_scan_blank(s, s->pos + 2)) {
            skipped = 3;
        } else if (s->pos < s->limit && s->raw[s->pos] == '\n' && sce_forge_cl_scan_blank(s, s->pos + 1)) {
            skipped = 2;
        }
        if (skipped == 0) {
            return;
        }
        s->pos += skipped;
    }
}

/* The next byte as 0..255 without taking it, or -1 at the end of the line. */
static inline int sce_forge_cl_scan_peek(sce_forge_cl_scan_t *s) {
    sce_forge_cl_scan_skip_folds(s);
    return s->pos < s->limit ? (int)s->raw[s->pos] : -1;
}

static inline int sce_forge_cl_scan_bump(sce_forge_cl_scan_t *s) {
    const int b = sce_forge_cl_scan_peek(s);
    if (b >= 0) {
        s->pos++;
    }
    return b;
}

/* Whether the unfolded text of raw[from, to) is `expected`, without regard to
 * case. */
static inline bool sce_forge_cl_unfolded_eq(const uint8_t *raw, size_t from, size_t to, const char *expected) {
    sce_forge_cl_scan_t scan;
    size_t i;
    scan.raw = raw;
    scan.limit = to;
    scan.pos = from;
    for (i = 0; expected[i] != '\0'; ++i) {
        const int b = sce_forge_cl_scan_bump(&scan);
        if (b < 0 || sce_forge_cl_lower(b) != sce_forge_cl_lower((unsigned char)expected[i])) {
            return false;
        }
    }
    return sce_forge_cl_scan_peek(&scan) < 0;
}

/* One logical line of the input, with its folds still inside it. */
typedef struct {
    size_t from;
    size_t to;
    /* Whether a line break ended it. The last line of an input that ends
     * without one may be cut short. */
    bool terminated;
} sce_forge_cl_rawline_t;

/* The name of a line and what follows it, as indexes into the input. */
typedef struct {
    size_t name_end;
    int separator;
    size_t rest_start;
} sce_forge_cl_head_t;

static inline bool sce_forge_cl_head_of(const uint8_t *raw, size_t from, size_t to, sce_forge_cl_head_t *head) {
    sce_forge_cl_scan_t scan;
    bool named = false;
    int separator;
    scan.raw = raw;
    scan.limit = to;
    scan.pos = from;
    for (;;) {
        const int b = sce_forge_cl_scan_peek(&scan);
        if (b >= 0 && sce_forge_cl_is_name_byte(b)) {
            (void)sce_forge_cl_scan_bump(&scan);
            named = true;
        } else {
            break;
        }
    }
    if (!named) {
        return false;
    }
    head->name_end = scan.pos;
    separator = sce_forge_cl_scan_peek(&scan);
    if (separator != ';' && separator != ':') {
        return false;
    }
    (void)sce_forge_cl_scan_bump(&scan);
    head->separator = separator;
    head->rest_start = scan.pos;
    return true;
}

/* ── Reading ───────────────────────────────────────────────────────── */

/* A reader over the properties of one component. */
typedef struct {
    const uint8_t *input;
    size_t size;
    const char *component;
    size_t pos;
    /* How many nested components the walk is inside (`VALARM` in `VEVENT`). */
    int depth;
} sce_forge_cl_reader_t;

/* The next logical line from the walk's position, or false at the end of the
 * input. A line ends at a line break (CRLF or LF) that no space or tab
 * follows. */
static inline bool sce_forge_cl_reader_next_raw_line(sce_forge_cl_reader_t *r, sce_forge_cl_rawline_t *line) {
    size_t i;
    const size_t start = r->pos;
    if (r->pos >= r->size) {
        return false;
    }
    for (i = start; i < r->size; ++i) {
        const bool folded = i + 1 < r->size && (r->input[i + 1] == ' ' || r->input[i + 1] == '\t');
        if (r->input[i] == '\n' && !folded) {
            line->from = start;
            line->to = (i > start && r->input[i - 1] == '\r') ? i - 1 : i;
            line->terminated = true;
            r->pos = i + 1;
            return true;
        }
    }
    line->from = start;
    line->to = r->size;
    line->terminated = false;
    r->pos = r->size;
    return true;
}

/* Skip lines up to the first `BEGIN:<component>` and stand after it. An input
 * that ends before it is SCE_FORGE_CODEC_NEED_MORE_BYTES. */
static inline sce_forge_codec_status_t sce_forge_cl_reader_begin(sce_forge_cl_reader_t *r, const uint8_t *input,
                                                                 size_t size, const char *component) {
    sce_forge_cl_rawline_t line;
    r->input = input;
    r->size = size;
    r->component = component;
    r->pos = 0;
    r->depth = 0;
    while (sce_forge_cl_reader_next_raw_line(r, &line)) {
        sce_forge_cl_head_t head = {0, 0, 0};
        if (!line.terminated) {
            break;
        }
        if (sce_forge_cl_head_of(input, line.from, line.to, &head) && head.separator == ':' &&
            sce_forge_cl_unfolded_eq(input, line.from, head.name_end, "BEGIN") &&
            sce_forge_cl_unfolded_eq(input, head.rest_start, line.to, component)) {
            return SCE_FORGE_CODEC_OK;
        }
    }
    return SCE_FORGE_CODEC_NEED_MORE_BYTES;
}

/* How many bytes of the input the walk has passed — after `END:<component>`
 * once `sce_forge_cl_reader_next` has answered `*has == false`. */
static inline size_t sce_forge_cl_reader_consumed(const sce_forge_cl_reader_t *r) {
    return r->pos;
}

/* Where a property stands in its line. */
enum {
    /* At the `;` that opens a parameter or the `:` that opens the value. */
    SCE_FORGE_CL_AT_SEPARATOR = 0,
    /* After a parameter's `=`, before its value. */
    SCE_FORGE_CL_PARAM_VALUE = 1,
    /* The value has been read. */
    SCE_FORGE_CL_DONE = 2
};

/* One property line the codec has been handed.
 *
 * Ask `sce_forge_cl_property_is` whether it is one the codec reads. Then, for a
 * property that declares parameters, loop on `sce_forge_cl_property_next_param`
 * and read the ones the codec declares; then read the value. A parameter no one
 * reads is skipped by the next call, and a property read by value alone skips
 * them all. */
typedef struct {
    const uint8_t *raw;
    size_t from;
    size_t name_end;
    sce_forge_cl_scan_t scan;
    size_t param_from;
    size_t param_to;
    bool has_param;
    int phase;
} sce_forge_cl_property_t;

/* The next property line of the component: `*has` is true with `*out` set, or
 * false after its `END:<component>`.
 *
 * A nested component is skipped through its `END:` without reading its lines.
 * An input that ends before `END:<component>` is
 * SCE_FORGE_CODEC_NEED_MORE_BYTES. */
static inline sce_forge_codec_status_t sce_forge_cl_reader_next(sce_forge_cl_reader_t *r, sce_forge_cl_property_t *out,
                                                                bool *has) {
    for (;;) {
        sce_forge_cl_rawline_t line;
        sce_forge_cl_head_t head = {0, 0, 0};
        bool named;
        sce_forge_codec_status_t cut;
        if (!sce_forge_cl_reader_next_raw_line(r, &line)) {
            return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        }
        named = sce_forge_cl_head_of(r->input, line.from, line.to, &head);
        if (r->depth > 0) {
            if (named && head.separator == ':') {
                if (sce_forge_cl_unfolded_eq(r->input, line.from, head.name_end, "BEGIN")) {
                    r->depth++;
                } else if (sce_forge_cl_unfolded_eq(r->input, line.from, head.name_end, "END")) {
                    r->depth--;
                }
            }
            continue;
        }
        /* A line with no name is cut short at the end of the input and
         * malformed anywhere else. */
        cut = line.terminated ? SCE_FORGE_CODEC_LINE_MALFORMED : SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (!named) {
            return cut;
        }
        if (head.separator == ':') {
            if (sce_forge_cl_unfolded_eq(r->input, line.from, head.name_end, "END")) {
                if (sce_forge_cl_unfolded_eq(r->input, head.rest_start, line.to, r->component)) {
                    *has = false;
                    return SCE_FORGE_CODEC_OK;
                }
                return cut;
            }
            if (line.terminated && sce_forge_cl_unfolded_eq(r->input, line.from, head.name_end, "BEGIN")) {
                r->depth = 1;
                continue;
            }
        }
        if (!line.terminated) {
            return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        }
        out->raw = r->input;
        out->from = line.from;
        out->name_end = head.name_end;
        out->scan.raw = r->input;
        out->scan.limit = line.to;
        out->scan.pos = head.name_end;
        out->param_from = 0;
        out->param_to = 0;
        out->has_param = false;
        out->phase = SCE_FORGE_CL_AT_SEPARATOR;
        *has = true;
        return SCE_FORGE_CODEC_OK;
    }
}

/* Whether this property is `name`, without regard to case. */
static inline bool sce_forge_cl_property_is(const sce_forge_cl_property_t *p, const char *name) {
    return sce_forge_cl_unfolded_eq(p->raw, p->from, p->name_end, name);
}

/* Whether the parameter `next_param` stands on is `name`, without regard to
 * case. */
static inline bool sce_forge_cl_property_param_is(const sce_forge_cl_property_t *p, const char *name) {
    return p->has_param && sce_forge_cl_unfolded_eq(p->raw, p->param_from, p->param_to, name);
}

/* Scan one parameter value — a quoted string, or text up to `;`, `:`, `,` or
 * `"` — storing each byte of it in out[0, max_size) (NULL: skipped) and its
 * length in `*len`. `*more` is whether another value follows a `,`. */
static inline sce_forge_codec_status_t sce_forge_cl_scan_param_value(sce_forge_cl_property_t *p, char *out,
                                                                     size_t max_size, size_t *len, bool *more) {
    size_t n = 0;
    int b;
    if (sce_forge_cl_scan_peek(&p->scan) == '"') {
        (void)sce_forge_cl_scan_bump(&p->scan);
        for (;;) {
            b = sce_forge_cl_scan_bump(&p->scan);
            if (b < 0) {
                return SCE_FORGE_CODEC_LINE_MALFORMED;
            }
            if (b == '"') {
                break;
            }
            if (out != NULL) {
                if (sce_forge_cl_is_control(b)) {
                    return SCE_FORGE_CODEC_LINE_BAD_VALUE;
                }
                if (n == max_size) {
                    return SCE_FORGE_CODEC_LINE_TOO_LONG;
                }
                out[n++] = (char)b;
            }
        }
    } else {
        for (;;) {
            b = sce_forge_cl_scan_peek(&p->scan);
            if (b < 0 || b == ';' || b == ':' || b == ',') {
                break;
            }
            if (b == '"') {
                return SCE_FORGE_CODEC_LINE_MALFORMED;
            }
            (void)sce_forge_cl_scan_bump(&p->scan);
            if (out != NULL) {
                if (sce_forge_cl_is_control(b)) {
                    return SCE_FORGE_CODEC_LINE_BAD_VALUE;
                }
                if (n == max_size) {
                    return SCE_FORGE_CODEC_LINE_TOO_LONG;
                }
                out[n++] = (char)b;
            }
        }
    }
    if (len != NULL) {
        *len = n;
    }
    b = sce_forge_cl_scan_peek(&p->scan);
    if (b == ',') {
        (void)sce_forge_cl_scan_bump(&p->scan);
        *more = true;
        return SCE_FORGE_CODEC_OK;
    }
    if (b == ';' || b == ':') {
        *more = false;
        return SCE_FORGE_CODEC_OK;
    }
    return SCE_FORGE_CODEC_LINE_MALFORMED;
}

static inline sce_forge_codec_status_t sce_forge_cl_skip_param_value(sce_forge_cl_property_t *p) {
    bool more = true;
    while (more) {
        const sce_forge_codec_status_t s = sce_forge_cl_scan_param_value(p, NULL, 0, NULL, &more);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
    }
    p->has_param = false;
    p->phase = SCE_FORGE_CL_AT_SEPARATOR;
    return SCE_FORGE_CODEC_OK;
}

/* Stand on the next parameter, skipping the value of one not read: `*more` is
 * true when there is one, false when the value is next. */
static inline sce_forge_codec_status_t sce_forge_cl_property_next_param(sce_forge_cl_property_t *p, bool *more) {
    int next;
    size_t start;
    size_t end;
    if (p->phase == SCE_FORGE_CL_PARAM_VALUE) {
        const sce_forge_codec_status_t s = sce_forge_cl_skip_param_value(p);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
    } else if (p->phase == SCE_FORGE_CL_DONE) {
        return SCE_FORGE_CODEC_LINE_MALFORMED;
    }
    next = sce_forge_cl_scan_peek(&p->scan);
    if (next == ':') {
        *more = false;
        return SCE_FORGE_CODEC_OK;
    }
    if (next != ';') {
        return SCE_FORGE_CODEC_LINE_MALFORMED;
    }
    (void)sce_forge_cl_scan_bump(&p->scan);
    sce_forge_cl_scan_skip_folds(&p->scan);
    start = p->scan.pos;
    for (;;) {
        const int b = sce_forge_cl_scan_peek(&p->scan);
        if (b >= 0 && sce_forge_cl_is_name_byte(b)) {
            (void)sce_forge_cl_scan_bump(&p->scan);
        } else {
            break;
        }
    }
    end = p->scan.pos;
    if (end == start || sce_forge_cl_scan_bump(&p->scan) != '=') {
        return SCE_FORGE_CODEC_LINE_MALFORMED;
    }
    p->param_from = start;
    p->param_to = end;
    p->has_param = true;
    p->phase = SCE_FORGE_CL_PARAM_VALUE;
    *more = true;
    return SCE_FORGE_CODEC_OK;
}

/* Read the value of the parameter `next_param` stands on, into out[0,
 * max_size), its length in `*len`. A second value is
 * SCE_FORGE_CODEC_LINE_BAD_VALUE. */
static inline sce_forge_codec_status_t
sce_forge_cl_property_read_param_string(sce_forge_cl_property_t *p, size_t max_size, char *out, size_t *len) {
    bool more = false;
    sce_forge_codec_status_t s;
    if (p->phase != SCE_FORGE_CL_PARAM_VALUE) {
        return SCE_FORGE_CODEC_LINE_MALFORMED;
    }
    s = sce_forge_cl_scan_param_value(p, out, max_size, len, &more);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (more) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    p->has_param = false;
    p->phase = SCE_FORGE_CL_AT_SEPARATOR;
    if (!sce_forge_is_valid_utf8((const uint8_t *)out, *len)) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Stand at the value: skip the parameters still unread and step over `:`. */
static inline sce_forge_codec_status_t sce_forge_cl_property_begin_value(sce_forge_cl_property_t *p) {
    bool more = true;
    while (more) {
        const sce_forge_codec_status_t s = sce_forge_cl_property_next_param(p, &more);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
    }
    (void)sce_forge_cl_scan_bump(&p->scan);
    p->phase = SCE_FORGE_CL_DONE;
    return SCE_FORGE_CODEC_OK;
}

/* Read the value as a string into out[0, max_size), its length in `*len`. With
 * `text`, `\\`, `\;`, `\,`, `\n` and `\N` are escapes; an unescaped `;` or `,`
 * is itself. */
static inline sce_forge_codec_status_t sce_forge_cl_property_read_string(sce_forge_cl_property_t *p, size_t max_size,
                                                                         bool text, char *out, size_t *len) {
    size_t n = 0;
    sce_forge_codec_status_t s = sce_forge_cl_property_begin_value(p);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    for (;;) {
        int value;
        const int b = sce_forge_cl_scan_bump(&p->scan);
        if (b < 0) {
            break;
        }
        value = b;
        if (text && b == '\\') {
            const int e = sce_forge_cl_scan_bump(&p->scan);
            if (e == '\\' || e == ';' || e == ',') {
                value = e;
            } else if (e == 'n' || e == 'N') {
                value = '\n';
            } else {
                return SCE_FORGE_CODEC_LINE_BAD_ESCAPE;
            }
        } else if (sce_forge_cl_is_control(b)) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        if (n == max_size) {
            return SCE_FORGE_CODEC_LINE_TOO_LONG;
        }
        out[n++] = (char)value;
    }
    *len = n;
    if (!sce_forge_is_valid_utf8((const uint8_t *)out, n)) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Read the value as a list of strings cut at `separator` (docs/adr/0014): at
 * most `max_values` of them, each at most `max_size` bytes. Part `i` is written
 * to `out + i * max_size`, its length to `lens[i]`, and the number of parts to
 * `*count`. The value is cut before it is unescaped: with `text` a separator
 * that a backslash precedes is part of the value, and with anything else every
 * separator cuts. The parts are judged left to right and the first failure is
 * the line's: a part past `max_values` is SCE_FORGE_CODEC_LINE_TOO_MANY, even
 * one that is empty or too long; an empty part is SCE_FORGE_CODEC_LINE_BAD_VALUE. */
static inline sce_forge_codec_status_t sce_forge_cl_property_read_strings(sce_forge_cl_property_t *p, char separator,
                                                                          size_t max_values, size_t max_size, bool text,
                                                                          char *out, size_t *lens, size_t *count) {
    size_t n = 0;
    size_t parts = 0;
    sce_forge_codec_status_t s = sce_forge_cl_property_begin_value(p);
    *count = 0;
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    for (;;) {
        int value;
        const int b = sce_forge_cl_scan_bump(&p->scan);
        if (b < 0 || b == (int)(unsigned char)separator) {
            if (n == 0) {
                return SCE_FORGE_CODEC_LINE_BAD_VALUE;
            }
            if (!sce_forge_is_valid_utf8((const uint8_t *)(out + parts * max_size), n)) {
                return SCE_FORGE_CODEC_LINE_BAD_VALUE;
            }
            lens[parts] = n;
            ++parts;
            *count = parts;
            n = 0;
            if (b < 0) {
                return SCE_FORGE_CODEC_OK;
            }
            /* A separator opens another part, which may not pass the bound. */
            if (parts >= max_values) {
                return SCE_FORGE_CODEC_LINE_TOO_MANY;
            }
            continue;
        }
        value = b;
        if (text && b == '\\') {
            const int e = sce_forge_cl_scan_bump(&p->scan);
            if (e == '\\' || e == ';' || e == ',') {
                value = e;
            } else if (e == 'n' || e == 'N') {
                value = '\n';
            } else {
                return SCE_FORGE_CODEC_LINE_BAD_ESCAPE;
            }
        } else if (sce_forge_cl_is_control(b)) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        if (n == max_size) {
            return SCE_FORGE_CODEC_LINE_TOO_LONG;
        }
        out[parts * max_size + n] = (char)value;
        ++n;
    }
}

/* The decimal the rest of the value is — an optional sign, digits — as
 * (`*negative`, `*magnitude`). A `-` is read only when `allow_minus`, so an
 * unsigned type refuses `-0` as well. */
static inline sce_forge_codec_status_t sce_forge_cl_property_read_decimal(sce_forge_cl_property_t *p, bool allow_minus,
                                                                          bool *negative, uint64_t *magnitude) {
    int next;
    int digits = 0;
    sce_forge_codec_status_t s = sce_forge_cl_property_begin_value(p);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    next = sce_forge_cl_scan_bump(&p->scan);
    *negative = next == '-';
    if (*negative && !allow_minus) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    if (*negative || next == '+') {
        next = sce_forge_cl_scan_bump(&p->scan);
    }
    *magnitude = 0;
    while (next >= 0) {
        uint64_t d;
        if (next < '0' || next > '9') {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        d = (uint64_t)(next - '0');
        if (*magnitude > (UINT64_MAX - d) / 10U) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        *magnitude = *magnitude * 10U + d;
        ++digits;
        next = sce_forge_cl_scan_bump(&p->scan);
    }
    if (digits == 0) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Read the value as an unsigned integer of at most `max`. */
static inline sce_forge_codec_status_t sce_forge_cl_property_read_uint(sce_forge_cl_property_t *p, uint64_t max,
                                                                       uint64_t *out) {
    bool negative = false;
    uint64_t magnitude = 0;
    const sce_forge_codec_status_t s = sce_forge_cl_property_read_decimal(p, false, &negative, &magnitude);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (magnitude > max) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    *out = magnitude;
    return SCE_FORGE_CODEC_OK;
}

/* Read the value as a signed integer within `min`..`max`. */
static inline sce_forge_codec_status_t sce_forge_cl_property_read_int(sce_forge_cl_property_t *p, int64_t min,
                                                                      int64_t max, int64_t *out) {
    bool negative = false;
    uint64_t magnitude = 0;
    int64_t value;
    const sce_forge_codec_status_t s = sce_forge_cl_property_read_decimal(p, true, &negative, &magnitude);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (negative) {
        if (magnitude > ((uint64_t)1 << 63)) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        /* 2^63 negated is the smallest int64; any other magnitude fits. */
        value = magnitude == 0 ? 0 : -(int64_t)(magnitude - 1U) - 1;
    } else {
        if (magnitude > (uint64_t)INT64_MAX) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        value = (int64_t)magnitude;
    }
    if (value < min || value > max) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    *out = value;
    return SCE_FORGE_CODEC_OK;
}

/* Read the value as `TRUE` or `FALSE`, in either case. */
static inline sce_forge_codec_status_t sce_forge_cl_property_read_bool(sce_forge_cl_property_t *p, bool *out) {
    char word[5];
    size_t n = 0;
    sce_forge_codec_status_t s = sce_forge_cl_property_begin_value(p);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    for (;;) {
        const int b = sce_forge_cl_scan_bump(&p->scan);
        if (b < 0) {
            break;
        }
        if (n == sizeof word) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        word[n++] = (char)sce_forge_cl_lower(b);
    }
    if (n == 4U && memcmp(word, "true", 4) == 0) {
        *out = true;
        return SCE_FORGE_CODEC_OK;
    }
    if (n == 5U && memcmp(word, "false", 5) == 0) {
        *out = false;
        return SCE_FORGE_CODEC_OK;
    }
    return SCE_FORGE_CODEC_LINE_BAD_VALUE;
}

/* ── Writing ───────────────────────────────────────────────────────── */

/* A writer of the lines of one component into a sink.
 *
 * Write `begin`; then a property by `property`, each present parameter by
 * `param`, and the value by one of the value functions, which ends the line;
 * then `finish`. Each returns the status that refused it, or
 * SCE_FORGE_CODEC_OK. */
typedef struct {
    sce_forge_writer_t *sink;
    const char *component;
    /* Octets already on the current physical line. */
    size_t column;
} sce_forge_cl_writer_t;

static inline sce_forge_cl_writer_t sce_forge_cl_writer_init(sce_forge_writer_t *sink, const char *component) {
    sce_forge_cl_writer_t w;
    w.sink = sink;
    w.component = component;
    w.column = 0;
    return w;
}

static inline sce_forge_codec_status_t sce_forge_cl_writer_raw(sce_forge_cl_writer_t *w, const char *text) {
    return sce_forge_writer_write_bytes(w->sink, (const uint8_t *)text, strlen(text));
}

static inline sce_forge_codec_status_t sce_forge_cl_writer_line_with(sce_forge_cl_writer_t *w, const char *head) {
    sce_forge_codec_status_t s = sce_forge_cl_writer_raw(w, head);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    s = sce_forge_cl_writer_raw(w, w->component);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return sce_forge_cl_writer_raw(w, "\r\n");
}

/* Write `BEGIN:<component>`. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_begin(sce_forge_cl_writer_t *w) {
    return sce_forge_cl_writer_line_with(w, "BEGIN:");
}

/* Write `END:<component>`. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_finish(sce_forge_cl_writer_t *w) {
    return sce_forge_cl_writer_line_with(w, "END:");
}

/* Write one unit — a character, or an escape — on the current line, after
 * cutting the line if it would pass SCE_FORGE_CL_FOLD_WIDTH octets. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_unit(sce_forge_cl_writer_t *w, const uint8_t *bytes,
                                                                size_t n) {
    sce_forge_codec_status_t s;
    if (w->column + n > (size_t)SCE_FORGE_CL_FOLD_WIDTH) {
        static const uint8_t fold[3] = {'\r', '\n', ' '};
        s = sce_forge_writer_write_bytes(w->sink, fold, sizeof fold);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
        w->column = 1;
    }
    s = sce_forge_writer_write_bytes(w->sink, bytes, n);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    w->column += n;
    return SCE_FORGE_CODEC_OK;
}

static inline sce_forge_codec_status_t sce_forge_cl_writer_unit_char(sce_forge_cl_writer_t *w, char c) {
    const uint8_t b = (uint8_t)c;
    return sce_forge_cl_writer_unit(w, &b, 1);
}

/* The units of ASCII `text`: one octet each. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_ascii_units(sce_forge_cl_writer_t *w, const char *text) {
    size_t i;
    for (i = 0; text[i] != '\0'; ++i) {
        const sce_forge_codec_status_t s = sce_forge_cl_writer_unit_char(w, text[i]);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* The units of the UTF-8 value: one character each, escaped when `text`. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_value_units(sce_forge_cl_writer_t *w, const char *value,
                                                                       size_t len, bool text) {
    const uint8_t *bytes = (const uint8_t *)value;
    size_t i = 0;
    while (i < len) {
        const uint8_t lead = bytes[i];
        size_t length = lead < 0x80U ? 1U : lead < 0xE0U ? 2U : lead < 0xF0U ? 3U : 4U;
        uint8_t escaped[2];
        bool is_escape = false;
        sce_forge_codec_status_t s;
        if (length > len - i) {
            length = len - i;
        }
        if (text && length == 1U) {
            escaped[0] = '\\';
            if (lead == '\\' || lead == ';' || lead == ',') {
                escaped[1] = lead;
                is_escape = true;
            } else if (lead == '\n') {
                escaped[1] = 'n';
                is_escape = true;
            }
        }
        s = is_escape ? sce_forge_cl_writer_unit(w, escaped, 2) : sce_forge_cl_writer_unit(w, bytes + i, length);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
        i += length;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Start a property's line with its name. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_property(sce_forge_cl_writer_t *w, const char *name) {
    w->column = 0;
    return sce_forge_cl_writer_ascii_units(w, name);
}

/* Write `;<name>=<value>`, quoting the value when it holds `:`, `;` or `,`. A
 * value past `max_size` is SCE_FORGE_CODEC_LINE_TOO_LONG; one with a control
 * character, a `"` or invalid UTF-8 is SCE_FORGE_CODEC_LINE_BAD_VALUE. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_param(sce_forge_cl_writer_t *w, const char *name,
                                                                 const char *value, size_t len, size_t max_size) {
    bool quoted = false;
    size_t i;
    sce_forge_codec_status_t s;
    if (len > max_size) {
        return SCE_FORGE_CODEC_LINE_TOO_LONG;
    }
    for (i = 0; i < len; ++i) {
        const int b = (unsigned char)value[i];
        if (b == '"' || sce_forge_cl_is_control(b)) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        quoted = quoted || b == ':' || b == ';' || b == ',';
    }
    if (!sce_forge_is_valid_utf8((const uint8_t *)value, len)) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    s = sce_forge_cl_writer_unit_char(w, ';');
    if (s == SCE_FORGE_CODEC_OK) {
        s = sce_forge_cl_writer_ascii_units(w, name);
    }
    if (s == SCE_FORGE_CODEC_OK) {
        s = sce_forge_cl_writer_unit_char(w, '=');
    }
    if (s == SCE_FORGE_CODEC_OK && quoted) {
        s = sce_forge_cl_writer_unit_char(w, '"');
    }
    if (s == SCE_FORGE_CODEC_OK) {
        s = sce_forge_cl_writer_value_units(w, value, len, false);
    }
    if (s == SCE_FORGE_CODEC_OK && quoted) {
        s = sce_forge_cl_writer_unit_char(w, '"');
    }
    return s;
}

static inline sce_forge_codec_status_t sce_forge_cl_writer_end_line(sce_forge_cl_writer_t *w) {
    w->column = 0;
    return sce_forge_cl_writer_raw(w, "\r\n");
}

/* Write `:<value>` and end the line. With `text`, `\`, `;`, `,` and a line feed
 * are written as escapes. A value past `max_size` is
 * SCE_FORGE_CODEC_LINE_TOO_LONG; one with a control character (but a TEXT's
 * line feed) or invalid UTF-8 is SCE_FORGE_CODEC_LINE_BAD_VALUE. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_string(sce_forge_cl_writer_t *w, const char *value,
                                                                  size_t len, bool text, size_t max_size) {
    size_t i;
    sce_forge_codec_status_t s;
    if (len > max_size) {
        return SCE_FORGE_CODEC_LINE_TOO_LONG;
    }
    for (i = 0; i < len; ++i) {
        const int b = (unsigned char)value[i];
        if (sce_forge_cl_is_control(b) && !(text && b == '\n')) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
    }
    if (!sce_forge_is_valid_utf8((const uint8_t *)value, len)) {
        return SCE_FORGE_CODEC_LINE_BAD_VALUE;
    }
    s = sce_forge_cl_writer_unit_char(w, ':');
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    s = sce_forge_cl_writer_value_units(w, value, len, text);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return sce_forge_cl_writer_end_line(w);
}

/* Write `:<value>{separator}<value>…` and end the line (docs/adr/0014). Value
 * `i` is `values + i * max_size`, `lens[i]` bytes. No values is
 * SCE_FORGE_CODEC_LINE_REQUIRED_MISSING and more than `max_values` is
 * SCE_FORGE_CODEC_LINE_TOO_MANY. A value past `max_size` is
 * SCE_FORGE_CODEC_LINE_TOO_LONG; one with a control character (but a TEXT's line
 * feed), invalid UTF-8, an empty one, and one that is not a TEXT and holds the
 * separator are SCE_FORGE_CODEC_LINE_BAD_VALUE, because a reader would cut or
 * refuse them. Every value is held before any of the line is written. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_strings(sce_forge_cl_writer_t *w, const char *values,
                                                                   const size_t *lens, size_t count, char separator,
                                                                   bool text, size_t max_size, size_t max_values) {
    size_t i;
    size_t k;
    sce_forge_codec_status_t s;
    if (count == 0) {
        return SCE_FORGE_CODEC_LINE_REQUIRED_MISSING;
    }
    if (count > max_values) {
        return SCE_FORGE_CODEC_LINE_TOO_MANY;
    }
    for (i = 0; i < count; ++i) {
        const char *value = values + i * max_size;
        if (lens[i] > max_size) {
            return SCE_FORGE_CODEC_LINE_TOO_LONG;
        }
        if (lens[i] == 0) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
        for (k = 0; k < lens[i]; ++k) {
            const int b = (unsigned char)value[k];
            if (sce_forge_cl_is_control(b) && !(text && b == '\n')) {
                return SCE_FORGE_CODEC_LINE_BAD_VALUE;
            }
            if (!text && value[k] == separator) {
                return SCE_FORGE_CODEC_LINE_BAD_VALUE;
            }
        }
        if (!sce_forge_is_valid_utf8((const uint8_t *)value, lens[i])) {
            return SCE_FORGE_CODEC_LINE_BAD_VALUE;
        }
    }
    s = sce_forge_cl_writer_unit_char(w, ':');
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    for (i = 0; i < count; ++i) {
        if (i > 0) {
            s = sce_forge_cl_writer_unit_char(w, separator);
            if (s != SCE_FORGE_CODEC_OK) {
                return s;
            }
        }
        s = sce_forge_cl_writer_value_units(w, values + i * max_size, lens[i], text);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
    }
    return sce_forge_cl_writer_end_line(w);
}

static inline sce_forge_codec_status_t sce_forge_cl_writer_digits(sce_forge_cl_writer_t *w, const char *text) {
    sce_forge_codec_status_t s = sce_forge_cl_writer_unit_char(w, ':');
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    s = sce_forge_cl_writer_ascii_units(w, text);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return sce_forge_cl_writer_end_line(w);
}

/* Write `:<value>` as decimal digits, `-` first when `negative` (with the
 * magnitude in `value`), and end the line. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_decimal(sce_forge_cl_writer_t *w, bool negative,
                                                                   uint64_t value) {
    char buf[22];
    size_t at = sizeof buf - 1U;
    buf[at] = '\0';
    do {
        buf[--at] = (char)('0' + (int)(value % 10U));
        value /= 10U;
    } while (value != 0U);
    if (negative) {
        buf[--at] = '-';
    }
    return sce_forge_cl_writer_digits(w, buf + at);
}

/* Write `:<value>` as decimal digits and end the line. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_uint(sce_forge_cl_writer_t *w, uint64_t value) {
    return sce_forge_cl_writer_decimal(w, false, value);
}

/* Write `:<value>` as decimal digits, `-` first when negative, and end the
 * line. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_int(sce_forge_cl_writer_t *w, int64_t value) {
    /* The magnitude of the smallest int64 is 2^63, which -value would
     * overflow, so it is taken in unsigned arithmetic. */
    const uint64_t magnitude = value < 0 ? (uint64_t)0 - (uint64_t)value : (uint64_t)value;
    return sce_forge_cl_writer_decimal(w, value < 0, magnitude);
}

/* Write `:TRUE` or `:FALSE` and end the line. */
static inline sce_forge_codec_status_t sce_forge_cl_writer_bool(sce_forge_cl_writer_t *w, bool value) {
    return sce_forge_cl_writer_digits(w, value ? "TRUE" : "FALSE");
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_CONTENT_LINE_H */
