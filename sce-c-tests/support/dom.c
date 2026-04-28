// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML B.2 — host-side XML DOM tree for the C11 backend.
// cpp `XMLDOMWrapper.cpp` 1:1 algorithmic mirror, pugixml-free.

#include "dom.h"

#include <ctype.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct sce_xml_doc_s {
    sce_xml_node_t *root;
    char *error_msg;
};

// ─── Allocation helpers ─────────────────────────────────────────────

static char *sce_xml_dup_range(const char *src, size_t start, size_t end) {
    size_t n = end - start;
    char *dst = (char *)malloc(n + 1u);
    if (!dst) {
        return NULL;
    }
    memcpy(dst, src + start, n);
    dst[n] = '\0';
    return dst;
}

static char *sce_xml_dup_cstr(const char *s) {
    if (!s) {
        return NULL;
    }
    size_t n = strlen(s);
    char *dst = (char *)malloc(n + 1u);
    if (!dst) {
        return NULL;
    }
    memcpy(dst, s, n + 1u);
    return dst;
}

static sce_xml_node_t *sce_xml_node_new(sce_xml_node_type_t type) {
    sce_xml_node_t *n = (sce_xml_node_t *)calloc(1u, sizeof(*n));
    if (n) {
        n->type = type;
    }
    return n;
}

static sce_xml_attr_t *sce_xml_attr_new(void) {
    sce_xml_attr_t *a = (sce_xml_attr_t *)calloc(1u, sizeof(*a));
    return a;
}

// ─── Entity decoding ────────────────────────────────────────────────
//
// XML 1.0 §4.6 predefined entities + §4.1 character references.  The
// decoder is invoked for both attribute values and element text, mirroring
// pugixml's `parse_default` (which has both `parse_escapes` and
// `parse_eol` semantics on by default).  Encoding is UTF-8 (input is also
// UTF-8 per W3C SCXML).  An undecodable reference (unknown name, malformed
// numeric form) is left verbatim — pugixml does the same on
// `parse_eol | parse_escapes` failure.

static int sce_xml_utf8_encode(uint32_t codepoint, char out[4]) {
    if (codepoint < 0x80u) {
        out[0] = (char)codepoint;
        return 1;
    }
    if (codepoint < 0x800u) {
        out[0] = (char)(0xC0u | (codepoint >> 6));
        out[1] = (char)(0x80u | (codepoint & 0x3Fu));
        return 2;
    }
    if (codepoint < 0x10000u) {
        out[0] = (char)(0xE0u | (codepoint >> 12));
        out[1] = (char)(0x80u | ((codepoint >> 6) & 0x3Fu));
        out[2] = (char)(0x80u | (codepoint & 0x3Fu));
        return 3;
    }
    if (codepoint < 0x110000u) {
        out[0] = (char)(0xF0u | (codepoint >> 18));
        out[1] = (char)(0x80u | ((codepoint >> 12) & 0x3Fu));
        out[2] = (char)(0x80u | ((codepoint >> 6) & 0x3Fu));
        out[3] = (char)(0x80u | (codepoint & 0x3Fu));
        return 4;
    }
    return 0;  // invalid codepoint
}

// Decode `&...;` references in `src` (length `len`) into a freshly
// malloc'd NUL-terminated string.  Returns NULL on out-of-memory; never
// rejects malformed references — those are passed through verbatim so a
// raw `&` in attribute / text is still observable (pugixml leaves
// unrecognised references in place under `parse_default`).
static char *sce_xml_decode_entities(const char *src, size_t len) {
    // Worst case: every char is a 1-byte literal — output ≤ input.
    char *out = (char *)malloc(len + 1u);
    if (!out) {
        return NULL;
    }
    size_t i = 0u;
    size_t o = 0u;
    while (i < len) {
        char c = src[i];
        if (c != '&') {
            out[o++] = c;
            i++;
            continue;
        }
        // Locate the terminating ';' within the next 32 bytes (XML
        // entity names are short; numeric refs cap at ~10 hex digits).
        size_t end = i + 1u;
        size_t cap = (i + 32u < len) ? (i + 32u) : len;
        while (end < cap && src[end] != ';') {
            end++;
        }
        if (end >= cap || src[end] != ';') {
            // No terminator — pass through.
            out[o++] = c;
            i++;
            continue;
        }
        size_t name_start = i + 1u;
        size_t name_len = end - name_start;
        int matched = 0;
        if (name_len >= 2u && src[name_start] == '#') {
            // Numeric character reference.
            uint32_t cp = 0u;
            int valid = 1;
            if (src[name_start + 1u] == 'x' || src[name_start + 1u] == 'X') {
                if (name_len < 3u) {
                    valid = 0;
                }
                for (size_t k = name_start + 2u; valid && k < end; ++k) {
                    char h = src[k];
                    cp <<= 4;
                    if (h >= '0' && h <= '9') {
                        cp |= (uint32_t)(h - '0');
                    } else if (h >= 'a' && h <= 'f') {
                        cp |= (uint32_t)(h - 'a' + 10);
                    } else if (h >= 'A' && h <= 'F') {
                        cp |= (uint32_t)(h - 'A' + 10);
                    } else {
                        valid = 0;
                    }
                }
            } else {
                for (size_t k = name_start + 1u; valid && k < end; ++k) {
                    char d = src[k];
                    if (d < '0' || d > '9') {
                        valid = 0;
                        break;
                    }
                    cp = cp * 10u + (uint32_t)(d - '0');
                }
            }
            if (valid) {
                char buf[4];
                int n = sce_xml_utf8_encode(cp, buf);
                if (n > 0) {
                    for (int k = 0; k < n; ++k) {
                        out[o++] = buf[k];
                    }
                    matched = 1;
                }
            }
        } else if (name_len == 2u && memcmp(src + name_start, "lt", 2) == 0) {
            out[o++] = '<';
            matched = 1;
        } else if (name_len == 2u && memcmp(src + name_start, "gt", 2) == 0) {
            out[o++] = '>';
            matched = 1;
        } else if (name_len == 3u && memcmp(src + name_start, "amp", 3) == 0) {
            out[o++] = '&';
            matched = 1;
        } else if (name_len == 4u && memcmp(src + name_start, "quot", 4) == 0) {
            out[o++] = '"';
            matched = 1;
        } else if (name_len == 4u && memcmp(src + name_start, "apos", 4) == 0) {
            out[o++] = '\'';
            matched = 1;
        }
        if (matched) {
            i = end + 1u;
        } else {
            // Unknown / malformed — pass through verbatim.
            out[o++] = c;
            i++;
        }
    }
    out[o] = '\0';
    return out;
}

// ─── Tree free ──────────────────────────────────────────────────────

static void sce_xml_free_attrs(sce_xml_attr_t *a) {
    while (a) {
        sce_xml_attr_t *next = a->next;
        free(a->name);
        free(a->value);
        free(a);
        a = next;
    }
}

static void sce_xml_free_node_recursive(sce_xml_node_t *n) {
    if (!n) {
        return;
    }
    sce_xml_node_t *c = n->first_child;
    while (c) {
        sce_xml_node_t *next = c->next_sibling;
        sce_xml_free_node_recursive(c);
        c = next;
    }
    sce_xml_free_attrs(n->attrs);
    free(n->tag);
    free(n->text);
    free(n);
}

void sce_xml_doc_free(sce_xml_doc_t *doc) {
    if (!doc) {
        return;
    }
    sce_xml_free_node_recursive(doc->root);
    free(doc->error_msg);
    free(doc);
}

// ─── Parser ─────────────────────────────────────────────────────────

typedef struct {
    const char *src;
    size_t pos;
    size_t len;
    int has_error;
    char error[160];
} sce_xml_parser_t;

static void sce_xml_parser_set_error(sce_xml_parser_t *p, const char *msg) {
    if (p->has_error) {
        return;
    }
    p->has_error = 1;
    snprintf(p->error, sizeof(p->error), "%s (at byte %zu)", msg, p->pos);
}

static int sce_xml_is_name_start(int c) {
    return c == '_' || c == ':' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

static int sce_xml_is_name_char(int c) {
    return sce_xml_is_name_start(c) || c == '-' || c == '.' ||
           (c >= '0' && c <= '9');
}

static void sce_xml_skip_ws(sce_xml_parser_t *p) {
    while (p->pos < p->len) {
        char c = p->src[p->pos];
        if (c == ' ' || c == '\t' || c == '\r' || c == '\n') {
            p->pos++;
        } else {
            break;
        }
    }
}

static int sce_xml_match(sce_xml_parser_t *p, const char *lit) {
    size_t n = strlen(lit);
    if (p->pos + n > p->len) {
        return 0;
    }
    if (memcmp(p->src + p->pos, lit, n) != 0) {
        return 0;
    }
    p->pos += n;
    return 1;
}

// Skip optional <?xml ... ?> processing instruction.
static void sce_xml_skip_pi(sce_xml_parser_t *p) {
    if (sce_xml_match(p, "<?")) {
        while (p->pos + 1u < p->len) {
            if (p->src[p->pos] == '?' && p->src[p->pos + 1u] == '>') {
                p->pos += 2u;
                return;
            }
            p->pos++;
        }
        sce_xml_parser_set_error(p, "unterminated processing instruction");
    }
}

// Skip <!-- comment -->.
static void sce_xml_skip_comment(sce_xml_parser_t *p) {
    if (sce_xml_match(p, "<!--")) {
        while (p->pos + 2u < p->len) {
            if (p->src[p->pos] == '-' && p->src[p->pos + 1u] == '-' &&
                p->src[p->pos + 2u] == '>') {
                p->pos += 3u;
                return;
            }
            p->pos++;
        }
        sce_xml_parser_set_error(p, "unterminated comment");
    }
}

// Skip <!DOCTYPE name ...> with optional `[ internal subset ]`.  pugixml
// drops the doctype entirely on `parse_default`; we mirror that since
// W3C SCXML B.2 corpus never reads DOCTYPE-declared entities and we
// don't run DTD validation.  Internal subset is balanced on `[`/`]`.
static void sce_xml_skip_doctype(sce_xml_parser_t *p) {
    if (!sce_xml_match(p, "<!DOCTYPE")) {
        return;
    }
    int in_subset = 0;
    while (p->pos < p->len) {
        char c = p->src[p->pos];
        if (c == '[') {
            in_subset = 1;
        } else if (c == ']') {
            in_subset = 0;
        } else if (c == '>' && !in_subset) {
            p->pos++;
            return;
        }
        p->pos++;
    }
    sce_xml_parser_set_error(p, "unterminated DOCTYPE");
}

// Read one of <?xml?> / <!-- --> / <!DOCTYPE ...> if present at the
// current position.
static int sce_xml_skip_misc(sce_xml_parser_t *p) {
    sce_xml_skip_ws(p);
    if (p->pos + 1u < p->len && p->src[p->pos] == '<') {
        if (p->src[p->pos + 1u] == '?') {
            sce_xml_skip_pi(p);
            return 1;
        }
        if (p->pos + 3u < p->len && p->src[p->pos + 1u] == '!' &&
            p->src[p->pos + 2u] == '-' && p->src[p->pos + 3u] == '-') {
            sce_xml_skip_comment(p);
            return 1;
        }
        if (p->pos + 8u < p->len && p->src[p->pos + 1u] == '!' &&
            memcmp(p->src + p->pos + 2u, "DOCTYPE", 7) == 0) {
            sce_xml_skip_doctype(p);
            return 1;
        }
    }
    return 0;
}

static char *sce_xml_parse_name(sce_xml_parser_t *p) {
    sce_xml_skip_ws(p);
    if (p->pos >= p->len || !sce_xml_is_name_start((unsigned char)p->src[p->pos])) {
        sce_xml_parser_set_error(p, "expected name");
        return NULL;
    }
    size_t start = p->pos;
    p->pos++;
    while (p->pos < p->len && sce_xml_is_name_char((unsigned char)p->src[p->pos])) {
        p->pos++;
    }
    return sce_xml_dup_range(p->src, start, p->pos);
}

static char *sce_xml_parse_attr_value(sce_xml_parser_t *p) {
    if (p->pos >= p->len) {
        sce_xml_parser_set_error(p, "expected attribute value");
        return NULL;
    }
    char quote = p->src[p->pos];
    if (quote != '"' && quote != '\'') {
        sce_xml_parser_set_error(p, "attribute value missing quote");
        return NULL;
    }
    p->pos++;
    size_t start = p->pos;
    while (p->pos < p->len && p->src[p->pos] != quote) {
        p->pos++;
    }
    if (p->pos >= p->len) {
        sce_xml_parser_set_error(p, "unterminated attribute value");
        return NULL;
    }
    // Decode XML entity references in attribute values (pugixml's
    // parse_default has parse_escapes set, mirrors that).
    char *val = sce_xml_decode_entities(p->src + start, p->pos - start);
    if (!val) {
        sce_xml_parser_set_error(p, "out of memory");
        return NULL;
    }
    p->pos++;  // consume closing quote
    return val;
}

static int sce_xml_parse_attributes(sce_xml_parser_t *p, sce_xml_node_t *node) {
    sce_xml_attr_t *tail = NULL;
    while (1) {
        sce_xml_skip_ws(p);
        if (p->pos >= p->len) {
            sce_xml_parser_set_error(p, "unterminated start tag");
            return 0;
        }
        char c = p->src[p->pos];
        if (c == '/' || c == '>') {
            return 1;
        }
        char *name = sce_xml_parse_name(p);
        if (!name) {
            return 0;
        }
        sce_xml_skip_ws(p);
        if (p->pos >= p->len || p->src[p->pos] != '=') {
            free(name);
            sce_xml_parser_set_error(p, "expected '=' in attribute");
            return 0;
        }
        p->pos++;
        sce_xml_skip_ws(p);
        char *value = sce_xml_parse_attr_value(p);
        if (!value) {
            free(name);
            return 0;
        }
        sce_xml_attr_t *a = sce_xml_attr_new();
        if (!a) {
            free(name);
            free(value);
            sce_xml_parser_set_error(p, "out of memory");
            return 0;
        }
        a->name = name;
        a->value = value;
        a->next = NULL;
        if (tail) {
            tail->next = a;
        } else {
            node->attrs = a;
        }
        tail = a;
    }
}

// Append `child` to `parent`'s child list.  Caller owns lifecycle on
// failure (the freshly-parsed child is freed by the caller path).
static void sce_xml_append_child(sce_xml_node_t *parent, sce_xml_node_t *child,
                                 sce_xml_node_t **tail_inout) {
    child->parent = parent;
    if (*tail_inout) {
        (*tail_inout)->next_sibling = child;
    } else {
        parent->first_child = child;
    }
    *tail_inout = child;
}

// Parse a `<![CDATA[ ... ]]>` section into a CDATA-typed text child.
// Caller has already verified the `<![CDATA[` prefix is present.
static int sce_xml_parse_cdata(sce_xml_parser_t *p, sce_xml_node_t *parent,
                               sce_xml_node_t **tail_inout) {
    if (!sce_xml_match(p, "<![CDATA[")) {
        sce_xml_parser_set_error(p, "expected CDATA");
        return 0;
    }
    size_t start = p->pos;
    while (p->pos + 2u < p->len) {
        if (p->src[p->pos] == ']' && p->src[p->pos + 1u] == ']' &&
            p->src[p->pos + 2u] == '>') {
            char *body = sce_xml_dup_range(p->src, start, p->pos);
            if (!body) {
                sce_xml_parser_set_error(p, "out of memory");
                return 0;
            }
            sce_xml_node_t *node = sce_xml_node_new(SCE_XML_NODE_CDATA);
            if (!node) {
                free(body);
                sce_xml_parser_set_error(p, "out of memory");
                return 0;
            }
            node->text = body;
            sce_xml_append_child(parent, node, tail_inout);
            p->pos += 3u;  // consume ']]>'
            return 1;
        }
        p->pos++;
    }
    sce_xml_parser_set_error(p, "unterminated CDATA section");
    return 0;
}

// Capture text content up to the next `<` and append as PCDATA.  The
// raw bytes are entity-decoded so `&amp;` etc. round-trip to their
// literal form (parse_escapes default).  An empty text run produces no
// node — pugixml's `parse_ws_pcdata_single` would emit one, but we
// don't surface a runtime difference: the W3C corpus reads attribute
// values, not text content, and `getElementsByTagName` only collects
// element nodes anyway.
static int sce_xml_consume_text(sce_xml_parser_t *p, sce_xml_node_t *parent,
                                sce_xml_node_t **tail_inout) {
    size_t start = p->pos;
    while (p->pos < p->len && p->src[p->pos] != '<') {
        p->pos++;
    }
    if (p->pos == start) {
        return 1;
    }
    char *decoded = sce_xml_decode_entities(p->src + start, p->pos - start);
    if (!decoded) {
        sce_xml_parser_set_error(p, "out of memory");
        return 0;
    }
    sce_xml_node_t *node = sce_xml_node_new(SCE_XML_NODE_PCDATA);
    if (!node) {
        free(decoded);
        sce_xml_parser_set_error(p, "out of memory");
        return 0;
    }
    node->text = decoded;
    sce_xml_append_child(parent, node, tail_inout);
    return 1;
}

static sce_xml_node_t *sce_xml_parse_element(sce_xml_parser_t *p) {
    if (p->pos >= p->len || p->src[p->pos] != '<') {
        sce_xml_parser_set_error(p, "expected '<'");
        return NULL;
    }
    p->pos++;
    char *tag = sce_xml_parse_name(p);
    if (!tag) {
        return NULL;
    }
    sce_xml_node_t *node = sce_xml_node_new(SCE_XML_NODE_ELEMENT);
    if (!node) {
        free(tag);
        sce_xml_parser_set_error(p, "out of memory");
        return NULL;
    }
    node->tag = tag;

    if (!sce_xml_parse_attributes(p, node)) {
        sce_xml_free_node_recursive(node);
        return NULL;
    }

    sce_xml_skip_ws(p);
    if (p->pos < p->len && p->src[p->pos] == '/') {
        p->pos++;
        if (p->pos >= p->len || p->src[p->pos] != '>') {
            sce_xml_parser_set_error(p, "expected '>' after '/'");
            sce_xml_free_node_recursive(node);
            return NULL;
        }
        p->pos++;
        return node;  // self-closing element
    }

    if (p->pos >= p->len || p->src[p->pos] != '>') {
        sce_xml_parser_set_error(p, "expected '>' to close start tag");
        sce_xml_free_node_recursive(node);
        return NULL;
    }
    p->pos++;

    // Element body — interleaved children, CDATA sections, comments,
    // and mixed text.  Comments / PIs / DOCTYPE inside a body are
    // dropped by sce_xml_skip_misc (DOCTYPE in body is technically
    // ill-formed but pugixml tolerates it under parse_default).
    sce_xml_node_t *child_tail = NULL;
    while (p->pos < p->len) {
        // CDATA must be tested before generic comment / PI dispatch
        // because skip_misc does not handle `<![CDATA[`.
        if (p->pos + 8u < p->len && p->src[p->pos] == '<' &&
            p->src[p->pos + 1u] == '!' && p->src[p->pos + 2u] == '[' &&
            memcmp(p->src + p->pos + 3u, "CDATA[", 6) == 0) {
            if (!sce_xml_parse_cdata(p, node, &child_tail)) {
                sce_xml_free_node_recursive(node);
                return NULL;
            }
            continue;
        }
        if (sce_xml_skip_misc(p)) {
            continue;
        }
        if (p->pos >= p->len) {
            sce_xml_parser_set_error(p, "unterminated element body");
            sce_xml_free_node_recursive(node);
            return NULL;
        }
        if (p->src[p->pos] == '<') {
            if (p->pos + 1u < p->len && p->src[p->pos + 1u] == '/') {
                break;  // end tag
            }
            sce_xml_node_t *child = sce_xml_parse_element(p);
            if (!child) {
                sce_xml_free_node_recursive(node);
                return NULL;
            }
            sce_xml_append_child(node, child, &child_tail);
        } else {
            // Mixed text content — capture as PCDATA child.  Empty
            // runs (caller already at `<`) produce no node.
            if (!sce_xml_consume_text(p, node, &child_tail)) {
                sce_xml_free_node_recursive(node);
                return NULL;
            }
        }
    }

    if (!sce_xml_match(p, "</")) {
        sce_xml_parser_set_error(p, "expected end tag");
        sce_xml_free_node_recursive(node);
        return NULL;
    }
    char *end_tag = sce_xml_parse_name(p);
    if (!end_tag) {
        sce_xml_free_node_recursive(node);
        return NULL;
    }
    int names_match = (strcmp(end_tag, node->tag) == 0);
    free(end_tag);
    if (!names_match) {
        sce_xml_parser_set_error(p, "end tag name mismatch");
        sce_xml_free_node_recursive(node);
        return NULL;
    }
    sce_xml_skip_ws(p);
    if (p->pos >= p->len || p->src[p->pos] != '>') {
        sce_xml_parser_set_error(p, "expected '>' to close end tag");
        sce_xml_free_node_recursive(node);
        return NULL;
    }
    p->pos++;
    return node;
}

// ─── Public API ─────────────────────────────────────────────────────

sce_xml_doc_t *sce_xml_parse(const char *src) {
    sce_xml_doc_t *doc = (sce_xml_doc_t *)calloc(1u, sizeof(*doc));
    if (!doc) {
        return NULL;
    }
    if (!src) {
        doc->error_msg = sce_xml_dup_cstr("null source");
        return doc;
    }

    sce_xml_parser_t p;
    p.src = src;
    p.pos = 0u;
    p.len = strlen(src);
    p.has_error = 0;
    p.error[0] = '\0';

    // Optional prologue: PI and/or comment, with whitespace.
    while (sce_xml_skip_misc(&p)) {
        // continue skipping
    }
    sce_xml_skip_ws(&p);

    if (p.pos >= p.len || p.src[p.pos] != '<') {
        doc->error_msg = sce_xml_dup_cstr("Failed to parse XML content: missing root element");
        return doc;
    }
    sce_xml_node_t *root = sce_xml_parse_element(&p);
    if (!root) {
        char buf[224];
        snprintf(buf, sizeof(buf), "Failed to parse XML content: %s",
                 p.has_error ? p.error : "unknown");
        doc->error_msg = sce_xml_dup_cstr(buf);
        return doc;
    }

    // Trailing misc / whitespace OK, anything else flags an error so
    // callers see drift.
    while (sce_xml_skip_misc(&p)) {
        // continue
    }
    sce_xml_skip_ws(&p);
    if (p.pos != p.len) {
        sce_xml_free_node_recursive(root);
        doc->error_msg = sce_xml_dup_cstr("Failed to parse XML content: trailing data after root");
        return doc;
    }

    doc->root = root;
    return doc;
}

int sce_xml_doc_is_valid(const sce_xml_doc_t *doc) {
    return doc && doc->root != NULL;
}

const char *sce_xml_doc_error(const sce_xml_doc_t *doc) {
    if (!doc) {
        return "";
    }
    return doc->error_msg ? doc->error_msg : "";
}

sce_xml_node_t *sce_xml_doc_root(sce_xml_doc_t *doc) {
    if (!doc) {
        return NULL;
    }
    return doc->root;
}

const char *sce_xml_get_tag_name(const sce_xml_node_t *node) {
    if (!node || !node->tag) {
        return "";
    }
    return node->tag;
}

const char *sce_xml_get_attribute(const sce_xml_node_t *node, const char *attr) {
    if (!node || !attr) {
        return "";
    }
    sce_xml_attr_t *a = node->attrs;
    while (a) {
        if (strcmp(a->name, attr) == 0) {
            return a->value ? a->value : "";
        }
        a = a->next;
    }
    return "";
}

// cpp findElementsByTagNameStatic 1:1 — element-only match, then
// descend.  PCDATA / CDATA children carry no tag and are walked past.
static int sce_xml_collect(sce_xml_node_t *node, const char *tag,
                           sce_xml_node_t ***out, size_t *count, size_t *cap) {
    if (!node) {
        return 1;
    }
    if (node->type == SCE_XML_NODE_ELEMENT && node->tag &&
        strcmp(node->tag, tag) == 0) {
        if (*count == *cap) {
            size_t new_cap = *cap == 0u ? 4u : *cap * 2u;
            sce_xml_node_t **resized = (sce_xml_node_t **)realloc(*out, new_cap * sizeof(*resized));
            if (!resized) {
                return 0;
            }
            *out = resized;
            *cap = new_cap;
        }
        (*out)[*count] = node;
        (*count)++;
    }
    sce_xml_node_t *c = node->first_child;
    while (c) {
        if (!sce_xml_collect(c, tag, out, count, cap)) {
            return 0;
        }
        c = c->next_sibling;
    }
    return 1;
}

// cpp XMLDocument::getElementsByTagName — recursive from root (root included).
sce_xml_node_t **sce_xml_doc_get_elements_by_tag_name(
    sce_xml_doc_t *doc, const char *tag, size_t *out_count) {
    if (out_count) {
        *out_count = 0u;
    }
    if (!doc || !doc->root || !tag) {
        return NULL;
    }
    sce_xml_node_t **arr = NULL;
    size_t count = 0u;
    size_t cap = 0u;
    if (!sce_xml_collect(doc->root, tag, &arr, &count, &cap)) {
        free(arr);
        return NULL;
    }
    if (out_count) {
        *out_count = count;
    }
    return arr;
}

// cpp XMLElement::getElementsByTagName — descends children only (self
// not matched).  Uses the same recursive collector starting from each
// direct child.
sce_xml_node_t **sce_xml_node_get_elements_by_tag_name(
    sce_xml_node_t *node, const char *tag, size_t *out_count) {
    if (out_count) {
        *out_count = 0u;
    }
    if (!node || !tag) {
        return NULL;
    }
    sce_xml_node_t **arr = NULL;
    size_t count = 0u;
    size_t cap = 0u;
    sce_xml_node_t *c = node->first_child;
    while (c) {
        if (!sce_xml_collect(c, tag, &arr, &count, &cap)) {
            free(arr);
            return NULL;
        }
        c = c->next_sibling;
    }
    if (out_count) {
        *out_count = count;
    }
    return arr;
}

void sce_xml_free_node_array(sce_xml_node_t **arr) {
    free(arr);
}
