// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML C.2 — JSON-to-Lua C API push for the host-side HTTP
// helper. See `http_lua_binding.h` for the contract; this file is
// the recursive-descent walker that visits a JSON document and emits
// the corresponding Lua value via the standard `lua_push*` API.
//
// Implementation notes:
//
//   * Non-allocating where possible: numeric literals go through
//     strtoll / strtod, strings collect into a small stack buffer
//     that grows on demand via realloc + lua_pushlstring at end.
//   * Stack discipline: each helper pushes exactly one value on
//     success, leaves the stack untouched on failure. `push_value`
//     records the entry depth and rewinds on error.
//   * UTF-8 + surrogate pair decode mirrors the in-place unescape in
//     `http_client.c::sce_json_unescape_string` so behaviour is
//     uniform across the two consumers; logic is duplicated rather
//     than shared because http_client.c is Lua-free and exporting a
//     decoder API would cross the layering boundary unnecessarily.

#include "http_lua_binding.h"

#include <ctype.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include <lua.h>
#include <lauxlib.h>

/* ── Internal state ─────────────────────────────────────────────── */

typedef struct {
    const char *p;          /* cursor */
    const char *end;        /* one-past-end */
} sce_json_state_t;

/* Forward declarations. */
static bool push_value(lua_State *L, sce_json_state_t *s);

/* ── Whitespace + character primitives ──────────────────────────── */

static void skip_ws(sce_json_state_t *s) {
    while (s->p < s->end &&
           (*s->p == ' ' || *s->p == '\t' ||
            *s->p == '\r' || *s->p == '\n')) {
        s->p++;
    }
}

static int hex_digit(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return 10 + (c - 'a');
    if (c >= 'A' && c <= 'F') return 10 + (c - 'A');
    return -1;
}

/* UTF-8 encode `cp` into `dst`; returns byte count written
   (1..4). Codepoints in the surrogate range or above U+10FFFF are
   substituted with U+FFFD per RFC 8259. */
static size_t utf8_encode(uint32_t cp, char *dst) {
    if ((cp >= 0xD800 && cp <= 0xDFFF) || cp > 0x10FFFFu) {
        cp = 0xFFFDu;
    }
    if (cp < 0x80u) {
        dst[0] = (char)cp;
        return 1u;
    }
    if (cp < 0x800u) {
        dst[0] = (char)(0xC0u | (cp >> 6));
        dst[1] = (char)(0x80u | (cp & 0x3Fu));
        return 2u;
    }
    if (cp < 0x10000u) {
        dst[0] = (char)(0xE0u | (cp >> 12));
        dst[1] = (char)(0x80u | ((cp >> 6) & 0x3Fu));
        dst[2] = (char)(0x80u | (cp & 0x3Fu));
        return 3u;
    }
    dst[0] = (char)(0xF0u | (cp >> 18));
    dst[1] = (char)(0x80u | ((cp >> 12) & 0x3Fu));
    dst[2] = (char)(0x80u | ((cp >> 6) & 0x3Fu));
    dst[3] = (char)(0x80u | (cp & 0x3Fu));
    return 4u;
}

/* ── JSON string → Lua string (out-of-place decode) ─────────────── */

/* Stack-buffer-first decode: most fixture strings fit in 256 bytes;
   only spill to malloc when overflow. The buffer is owned by the
   caller's automatic frame so no free() in the happy path. */
typedef struct {
    char inline_buf[256];
    char *heap;       /* malloc'd when inline overflows; NULL otherwise */
    size_t cap;       /* cap of whichever buffer is active */
    size_t len;       /* bytes written */
    char *active;     /* points into inline_buf or heap */
} decode_buf_t;

static void decode_buf_init(decode_buf_t *b) {
    b->heap = NULL;
    b->cap = sizeof(b->inline_buf);
    b->len = 0u;
    b->active = b->inline_buf;
}

static bool decode_buf_reserve(decode_buf_t *b, size_t need) {
    if (b->len + need <= b->cap) {
        return true;
    }
    size_t new_cap = b->cap * 2u;
    while (new_cap < b->len + need) {
        new_cap *= 2u;
    }
    if (b->heap == NULL) {
        b->heap = malloc(new_cap);
        if (b->heap == NULL) {
            return false;
        }
        memcpy(b->heap, b->inline_buf, b->len);
    } else {
        char *grown = realloc(b->heap, new_cap);
        if (grown == NULL) {
            return false;
        }
        b->heap = grown;
    }
    b->active = b->heap;
    b->cap = new_cap;
    return true;
}

static void decode_buf_free(decode_buf_t *b) {
    if (b->heap != NULL) {
        free(b->heap);
        b->heap = NULL;
    }
}

static bool decode_buf_push(decode_buf_t *b, char c) {
    if (!decode_buf_reserve(b, 1u)) {
        return false;
    }
    b->active[b->len++] = c;
    return true;
}

static bool decode_string(sce_json_state_t *s, decode_buf_t *out) {
    if (s->p >= s->end || *s->p != '"') {
        return false;
    }
    s->p++;
    while (s->p < s->end && *s->p != '"') {
        if (*s->p == '\\') {
            if (s->p + 1 >= s->end) {
                return false;
            }
            char esc = s->p[1];
            switch (esc) {
            case '"':  if (!decode_buf_push(out, '"')) return false; s->p += 2; break;
            case '\\': if (!decode_buf_push(out, '\\')) return false; s->p += 2; break;
            case '/':  if (!decode_buf_push(out, '/')) return false; s->p += 2; break;
            case 'b':  if (!decode_buf_push(out, '\b')) return false; s->p += 2; break;
            case 'f':  if (!decode_buf_push(out, '\f')) return false; s->p += 2; break;
            case 'n':  if (!decode_buf_push(out, '\n')) return false; s->p += 2; break;
            case 'r':  if (!decode_buf_push(out, '\r')) return false; s->p += 2; break;
            case 't':  if (!decode_buf_push(out, '\t')) return false; s->p += 2; break;
            case 'u': {
                if (s->p + 6 > s->end) {
                    return false;
                }
                int h0 = hex_digit(s->p[2]);
                int h1 = hex_digit(s->p[3]);
                int h2 = hex_digit(s->p[4]);
                int h3 = hex_digit(s->p[5]);
                if (h0 < 0 || h1 < 0 || h2 < 0 || h3 < 0) {
                    return false;
                }
                uint32_t cp = (uint32_t)((h0 << 12) | (h1 << 8) |
                                          (h2 << 4) | h3);
                if (cp >= 0xD800u && cp <= 0xDBFFu &&
                    s->p + 12 <= s->end &&
                    s->p[6] == '\\' && s->p[7] == 'u') {
                    int l0 = hex_digit(s->p[8]);
                    int l1 = hex_digit(s->p[9]);
                    int l2 = hex_digit(s->p[10]);
                    int l3 = hex_digit(s->p[11]);
                    if (l0 < 0 || l1 < 0 || l2 < 0 || l3 < 0) {
                        return false;
                    }
                    uint32_t low = (uint32_t)((l0 << 12) | (l1 << 8) |
                                                (l2 << 4) | l3);
                    if (low >= 0xDC00u && low <= 0xDFFFu) {
                        cp = 0x10000u +
                             ((cp - 0xD800u) << 10) +
                             (low - 0xDC00u);
                        s->p += 12;
                    } else {
                        s->p += 6;
                    }
                } else {
                    s->p += 6;
                }
                if (!decode_buf_reserve(out, 4u)) {
                    return false;
                }
                size_t written = utf8_encode(cp, out->active + out->len);
                out->len += written;
                break;
            }
            default:
                return false;
            }
        } else {
            if (!decode_buf_push(out, *s->p)) {
                return false;
            }
            s->p++;
        }
    }
    if (s->p >= s->end) {
        return false;
    }
    s->p++;  /* past closing `"` */
    return true;
}

static bool push_string(lua_State *L, sce_json_state_t *s) {
    decode_buf_t buf;
    decode_buf_init(&buf);
    if (!decode_string(s, &buf)) {
        decode_buf_free(&buf);
        return false;
    }
    lua_pushlstring(L, buf.active, buf.len);
    decode_buf_free(&buf);
    return true;
}

/* ── JSON number → Lua integer/number ───────────────────────────── */

static bool push_number(lua_State *L, sce_json_state_t *s) {
    const char *start = s->p;
    bool has_fraction = false;
    if (*s->p == '-') {
        s->p++;
    }
    while (s->p < s->end &&
           ((*s->p >= '0' && *s->p <= '9') ||
            *s->p == '.' || *s->p == 'e' || *s->p == 'E' ||
            *s->p == '+' || *s->p == '-')) {
        if (*s->p == '.' || *s->p == 'e' || *s->p == 'E') {
            has_fraction = true;
        }
        s->p++;
    }
    if (s->p == start) {
        return false;
    }
    /* Parse a NUL-terminated copy via strto*; we lack a length-bounded
       variant in standard C. The source span is at most
       SCE_HTTP_HEADER_CAP (4 KiB) bytes from an http response, so a
       64-byte stack buffer is far over the longest plausible JSON
       number literal (RFC 8259 lacks a hard cap; practically ≤ 30
       chars). */
    char buf[64];
    size_t n = (size_t)(s->p - start);
    if (n >= sizeof(buf)) {
        return false;
    }
    memcpy(buf, start, n);
    buf[n] = '\0';
    char *parsed_end = NULL;
    if (!has_fraction) {
        long long iv = strtoll(buf, &parsed_end, 10);
        if (parsed_end != buf + n) {
            return false;
        }
        lua_pushinteger(L, (lua_Integer)iv);
    } else {
        double dv = strtod(buf, &parsed_end);
        if (parsed_end != buf + n) {
            return false;
        }
        lua_pushnumber(L, dv);
    }
    return true;
}

/* ── JSON literals: true / false / null ─────────────────────────── */

static bool push_literal(lua_State *L, sce_json_state_t *s,
                         const char *kw, size_t kw_len, int kind) {
    if (s->p + kw_len > s->end) {
        return false;
    }
    if (memcmp(s->p, kw, kw_len) != 0) {
        return false;
    }
    s->p += kw_len;
    /* kind: 0 = nil, 1 = true, 2 = false. */
    if (kind == 0) {
        lua_pushnil(L);
    } else if (kind == 1) {
        lua_pushboolean(L, 1);
    } else {
        lua_pushboolean(L, 0);
    }
    return true;
}

/* ── JSON object → Lua table ────────────────────────────────────── */

static bool push_object(lua_State *L, sce_json_state_t *s) {
    if (*s->p != '{') {
        return false;
    }
    s->p++;
    lua_newtable(L);
    int top = lua_gettop(L);

    skip_ws(s);
    if (s->p < s->end && *s->p == '}') {
        s->p++;
        return true;
    }

    while (s->p < s->end) {
        skip_ws(s);
        if (s->p >= s->end || *s->p != '"') {
            lua_settop(L, top - 1);
            return false;
        }

        decode_buf_t key;
        decode_buf_init(&key);
        if (!decode_string(s, &key)) {
            decode_buf_free(&key);
            lua_settop(L, top - 1);
            return false;
        }
        lua_pushlstring(L, key.active, key.len);
        decode_buf_free(&key);

        skip_ws(s);
        if (s->p >= s->end || *s->p != ':') {
            lua_settop(L, top - 1);
            return false;
        }
        s->p++;

        if (!push_value(L, s)) {
            lua_settop(L, top - 1);
            return false;
        }
        /* stack: ..., table, key, value */
        lua_rawset(L, top);

        skip_ws(s);
        if (s->p < s->end && *s->p == ',') {
            s->p++;
            continue;
        }
        if (s->p < s->end && *s->p == '}') {
            s->p++;
            return true;
        }
        lua_settop(L, top - 1);
        return false;
    }
    lua_settop(L, top - 1);
    return false;
}

/* ── JSON array → Lua array-table (1-based) ─────────────────────── */

static bool push_array(lua_State *L, sce_json_state_t *s) {
    if (*s->p != '[') {
        return false;
    }
    s->p++;
    lua_newtable(L);
    int top = lua_gettop(L);

    skip_ws(s);
    if (s->p < s->end && *s->p == ']') {
        s->p++;
        return true;
    }

    int idx = 1;
    while (s->p < s->end) {
        if (!push_value(L, s)) {
            lua_settop(L, top - 1);
            return false;
        }
        lua_rawseti(L, top, idx);
        idx++;

        skip_ws(s);
        if (s->p < s->end && *s->p == ',') {
            s->p++;
            continue;
        }
        if (s->p < s->end && *s->p == ']') {
            s->p++;
            return true;
        }
        lua_settop(L, top - 1);
        return false;
    }
    lua_settop(L, top - 1);
    return false;
}

/* ── Top-level value dispatcher ─────────────────────────────────── */

static bool push_value(lua_State *L, sce_json_state_t *s) {
    skip_ws(s);
    if (s->p >= s->end) {
        return false;
    }
    char ch = *s->p;
    if (ch == '"') {
        return push_string(L, s);
    }
    if (ch == '{') {
        return push_object(L, s);
    }
    if (ch == '[') {
        return push_array(L, s);
    }
    if (ch == '-' || (ch >= '0' && ch <= '9')) {
        return push_number(L, s);
    }
    if (ch == 't') {
        return push_literal(L, s, "true", 4u, 1);
    }
    if (ch == 'f') {
        return push_literal(L, s, "false", 5u, 2);
    }
    if (ch == 'n') {
        return push_literal(L, s, "null", 4u, 0);
    }
    return false;
}

bool sce_test_lua_push_json(lua_State *L, const char *json, size_t len) {
    if (L == NULL || json == NULL || len == 0u) {
        return false;
    }
    sce_json_state_t s;
    s.p = json;
    s.end = json + len;
    int entry_top = lua_gettop(L);
    if (!push_value(L, &s)) {
        lua_settop(L, entry_top);
        return false;
    }
    return true;
}
