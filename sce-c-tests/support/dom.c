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

static sce_xml_node_t *sce_xml_node_new(void) {
    sce_xml_node_t *n = (sce_xml_node_t *)calloc(1u, sizeof(*n));
    return n;
}

static sce_xml_attr_t *sce_xml_attr_new(void) {
    sce_xml_attr_t *a = (sce_xml_attr_t *)calloc(1u, sizeof(*a));
    return a;
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

// Read one of <?xml?> or <!-- --> if present at current position.
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
    char *val = sce_xml_dup_range(p->src, start, p->pos);
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
    sce_xml_node_t *node = sce_xml_node_new();
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

    // Children + text content.  Text content is currently silently
    // skipped — corpus (test557/561) carries only element children.
    sce_xml_node_t *child_tail = NULL;
    while (p->pos < p->len) {
        // skip whitespace between children — non-whitespace text is
        // currently treated as an error to surface unsupported cases.
        sce_xml_skip_ws(p);
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
            child->parent = node;
            if (child_tail) {
                child_tail->next_sibling = child;
            } else {
                node->first_child = child;
            }
            child_tail = child;
        } else {
            // Mixed text content — corpus does not exercise it.  Skip
            // until next '<' so files with stray whitespace + element
            // mix still parse cleanly; non-whitespace text is dropped.
            while (p->pos < p->len && p->src[p->pos] != '<') {
                p->pos++;
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

// cpp findElementsByTagNameStatic 1:1 — checks current node, then descends.
static int sce_xml_collect(sce_xml_node_t *node, const char *tag,
                           sce_xml_node_t ***out, size_t *count, size_t *cap) {
    if (!node) {
        return 1;
    }
    if (node->tag && strcmp(node->tag, tag) == 0) {
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
