// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The standard host features (docs/SCE_HOST_FEATURES.md) as a generated C machine
// sees them through the reference host: every scenario of
// `tests/host_features/scenarios` replayed against the machine it names, with
// `sce_hf_invoker` registered as the invoker of every standard type.
//
// A scenario's `host` directives set what the host is like before a step; its
// `expect.host_invokes` and `expect.host_state` are what the host was asked and what
// it holds afterwards. The expected values are derived from the specification, not
// observed from the host.
//
// A C test has no JSON library to borrow, so this holds a small reader of its own: it
// reads the whole scenario into a tree and compares a value to the host's as a tree
// (an object by its members, an array by its elements, in order). The build raises
// SCE_MAX_DATA_LEN to what the largest result record needs
// (`tests/CMakeLists.txt`), because a record travels in one data slot of the machine.

#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sce/standard_host.h"
#include "static_host_clock_sm.h"
#include "static_host_http_sm.h"
#include "static_host_kv_sm.h"
#include "static_host_notify_sm.h"

#ifndef SCE_HOST_FEATURES_SCENARIO_DIR
#error "SCE_HOST_FEATURES_SCENARIO_DIR names the directory of the scenarios"
#endif

// ── A small JSON tree ────────────────────────────────────────────────────────

typedef enum { J_NULL, J_BOOL, J_NUM, J_STR, J_ARR, J_OBJ } jkind_t;

typedef struct jn {
    jkind_t kind;
    bool b;
    bool is_int;
    long long i;
    double d;
    char *s;
    size_t n;
    struct jn **items;
    char **keys;
    size_t count;
    size_t cap;
    // The text the value was written as, for a value the machine is handed whole.
    const char *raw;
    size_t raw_len;
} jn_t;

static void *must(void *p) {
    if (p == NULL) {
        abort();
    }
    return p;
}

static jn_t *jn_new(jkind_t kind) {
    jn_t *node = (jn_t *)must(calloc(1u, sizeof(jn_t)));
    node->kind = kind;
    return node;
}

static void jn_grow(jn_t *node) {
    if (node->count == node->cap) {
        node->cap = node->cap ? node->cap * 2u : 8u;
        node->items = (jn_t **)must(realloc(node->items, node->cap * sizeof(jn_t *)));
        if (node->kind == J_OBJ) {
            node->keys = (char **)must(realloc(node->keys, node->cap * sizeof(char *)));
        }
    }
}

static void jn_push(jn_t *arr, jn_t *item) {
    jn_grow(arr);
    arr->items[arr->count++] = item;
}

static void jn_set(jn_t *obj, const char *key, jn_t *item) {
    jn_grow(obj);
    obj->keys[obj->count] = (char *)must(strdup(key));
    obj->items[obj->count++] = item;
}

static jn_t *jn_str(const char *s) {
    jn_t *node = jn_new(J_STR);
    node->n = strlen(s);
    node->s = (char *)must(strdup(s));
    return node;
}

static jn_t *jn_int(long long i) {
    jn_t *node = jn_new(J_NUM);
    node->is_int = true;
    node->i = i;
    node->d = (double)i;
    return node;
}

static jn_t *jn_bool(bool b) {
    jn_t *node = jn_new(J_BOOL);
    node->b = b;
    return node;
}

static const jn_t *jn_get(const jn_t *obj, const char *key) {
    if (obj == NULL || obj->kind != J_OBJ) {
        return NULL;
    }
    for (size_t i = 0; i < obj->count; ++i) {
        if (strcmp(obj->keys[i], key) == 0) {
            return obj->items[i];
        }
    }
    return NULL;
}

static void jn_free(jn_t *node) {
    if (node == NULL) {
        return;
    }
    for (size_t i = 0; i < node->count; ++i) {
        jn_free(node->items[i]);
        if (node->kind == J_OBJ) {
            free(node->keys[i]);
        }
    }
    free(node->items);
    free(node->keys);
    free(node->s);
    free(node);
}

typedef struct {
    const char *p;
    const char *end;
    bool ok;
} parser_t;

static void skip(parser_t *ps) {
    while (ps->p < ps->end && (*ps->p == ' ' || *ps->p == '\n' || *ps->p == '\t' || *ps->p == '\r')) {
        ++ps->p;
    }
}

static size_t utf8(unsigned long code, char out[4]) {
    if (code < 0x80ul) {
        out[0] = (char)code;
        return 1u;
    }
    if (code < 0x800ul) {
        out[0] = (char)(0xC0ul | (code >> 6));
        out[1] = (char)(0x80ul | (code & 0x3Ful));
        return 2u;
    }
    if (code < 0x10000ul) {
        out[0] = (char)(0xE0ul | (code >> 12));
        out[1] = (char)(0x80ul | ((code >> 6) & 0x3Ful));
        out[2] = (char)(0x80ul | (code & 0x3Ful));
        return 3u;
    }
    out[0] = (char)(0xF0ul | (code >> 18));
    out[1] = (char)(0x80ul | ((code >> 12) & 0x3Ful));
    out[2] = (char)(0x80ul | ((code >> 6) & 0x3Ful));
    out[3] = (char)(0x80ul | (code & 0x3Ful));
    return 4u;
}

static unsigned long hex4(parser_t *ps) {
    unsigned long code = 0;
    for (int k = 0; k < 4; ++k) {
        if (ps->p >= ps->end || !strchr("0123456789abcdefABCDEF", *ps->p)) {
            ps->ok = false;
            return 0;
        }
        const char c = *ps->p++;
        code = code * 16u + (unsigned long)(c <= '9' ? c - '0' : (c | 0x20) - 'a' + 10);
    }
    return code;
}

static char *parse_string(parser_t *ps, size_t *len) {
    char *out = (char *)must(malloc((size_t)(ps->end - ps->p) + 1u));
    size_t n = 0;
    ++ps->p;  // the opening quote
    while (ps->p < ps->end && *ps->p != '"') {
        if (*ps->p != '\\') {
            out[n++] = *ps->p++;
            continue;
        }
        ++ps->p;
        if (ps->p >= ps->end) {
            break;
        }
        const char e = *ps->p++;
        switch (e) {
        case 'n':
            out[n++] = '\n';
            break;
        case 't':
            out[n++] = '\t';
            break;
        case 'r':
            out[n++] = '\r';
            break;
        case 'b':
            out[n++] = '\b';
            break;
        case 'f':
            out[n++] = '\f';
            break;
        case 'u': {
            unsigned long code = hex4(ps);
            if (code >= 0xD800ul && code <= 0xDBFFul && ps->p + 1 < ps->end && ps->p[0] == '\\' && ps->p[1] == 'u') {
                ps->p += 2;
                const unsigned long low = hex4(ps);
                code = 0x10000ul + ((code - 0xD800ul) << 10) + (low - 0xDC00ul);
            }
            n += utf8(code, out + n);
            break;
        }
        default:
            out[n++] = e;
            break;
        }
    }
    if (ps->p >= ps->end) {
        ps->ok = false;
    } else {
        ++ps->p;  // the closing quote
    }
    out[n] = '\0';
    *len = n;
    return out;
}

static jn_t *parse_value(parser_t *ps) {
    skip(ps);
    if (ps->p >= ps->end) {
        ps->ok = false;
        return NULL;
    }
    const char *start = ps->p;
    jn_t *node = NULL;
    if (*ps->p == '{' || *ps->p == '[') {
        const bool object = *ps->p == '{';
        node = jn_new(object ? J_OBJ : J_ARR);
        ++ps->p;
        skip(ps);
        if (ps->p < ps->end && *ps->p == (object ? '}' : ']')) {
            ++ps->p;
        } else {
            for (;;) {
                skip(ps);
                char *key = NULL;
                if (object) {
                    size_t key_len;
                    if (ps->p >= ps->end || *ps->p != '"') {
                        ps->ok = false;
                        break;
                    }
                    key = parse_string(ps, &key_len);
                    skip(ps);
                    if (ps->p >= ps->end || *ps->p != ':') {
                        free(key);
                        ps->ok = false;
                        break;
                    }
                    ++ps->p;
                }
                jn_t *item = parse_value(ps);
                if (item == NULL) {
                    free(key);
                    break;
                }
                if (object) {
                    jn_set(node, key, item);
                    free(key);
                } else {
                    jn_push(node, item);
                }
                skip(ps);
                if (ps->p < ps->end && *ps->p == ',') {
                    ++ps->p;
                    continue;
                }
                if (ps->p < ps->end && *ps->p == (object ? '}' : ']')) {
                    ++ps->p;
                    break;
                }
                ps->ok = false;
                break;
            }
        }
    } else if (*ps->p == '"') {
        node = jn_new(J_STR);
        node->s = parse_string(ps, &node->n);
    } else if (strncmp(ps->p, "true", 4u) == 0 || strncmp(ps->p, "false", 5u) == 0) {
        node = jn_bool(*ps->p == 't');
        ps->p += node->b ? 4 : 5;
    } else if (strncmp(ps->p, "null", 4u) == 0) {
        node = jn_new(J_NULL);
        ps->p += 4;
    } else {
        char *end = NULL;
        const double d = strtod(ps->p, &end);
        if (end == ps->p) {
            ps->ok = false;
            return NULL;
        }
        node = jn_new(J_NUM);
        node->d = d;
        node->is_int = memchr(ps->p, '.', (size_t)(end - ps->p)) == NULL &&
                       memchr(ps->p, 'e', (size_t)(end - ps->p)) == NULL &&
                       memchr(ps->p, 'E', (size_t)(end - ps->p)) == NULL;
        node->i = strtoll(ps->p, NULL, 10);
        ps->p = end;
    }
    node->raw = start;
    node->raw_len = (size_t)(ps->p - start);
    return node;
}

static bool jn_equal(const jn_t *a, const jn_t *b) {
    if (a->kind != b->kind) {
        return false;
    }
    switch (a->kind) {
    case J_NULL:
        return true;
    case J_BOOL:
        return a->b == b->b;
    case J_NUM:
        return a->is_int && b->is_int ? a->i == b->i : a->d == b->d;
    case J_STR:
        return a->n == b->n && memcmp(a->s, b->s, a->n) == 0;
    case J_ARR:
        if (a->count != b->count) {
            return false;
        }
        for (size_t i = 0; i < a->count; ++i) {
            if (!jn_equal(a->items[i], b->items[i])) {
                return false;
            }
        }
        return true;
    default:
        if (a->count != b->count) {
            return false;
        }
        for (size_t i = 0; i < a->count; ++i) {
            const jn_t *other = jn_get(b, a->keys[i]);
            if (other == NULL || !jn_equal(a->items[i], other)) {
                return false;
            }
        }
        return true;
    }
}

static void dump(const jn_t *node, FILE *out) {
    switch (node->kind) {
    case J_NULL:
        (void)fputs("null", out);
        break;
    case J_BOOL:
        (void)fputs(node->b ? "true" : "false", out);
        break;
    case J_NUM:
        if (node->is_int) {
            (void)fprintf(out, "%lld", node->i);
        } else {
            (void)fprintf(out, "%g", node->d);
        }
        break;
    case J_STR:
        (void)fprintf(out, "\"%.200s\"", node->s);
        break;
    default: {
        const bool object = node->kind == J_OBJ;
        (void)fputc(object ? '{' : '[', out);
        for (size_t i = 0; i < node->count; ++i) {
            if (i > 0u) {
                (void)fputc(',', out);
            }
            if (object) {
                (void)fprintf(out, "\"%s\":", node->keys[i]);
            }
            dump(node->items[i], out);
        }
        (void)fputc(object ? '}' : ']', out);
    }
    }
}

// ── The host a scenario sets ─────────────────────────────────────────────────

typedef struct {
    bool set;
    bool hold;
    bool fail;
    int status;
    sce_hf_header_t *headers;
    size_t header_count;
    unsigned char *body;
    size_t body_len;
    // What the transport was given, one entry for each exchange.
    jn_t *log;
} script_t;

typedef struct {
    sce_hf_host_t *host;
    script_t transport;
    sce_hf_reading_t reading;
    sce_hf_zone_t zone;
} rig_t;

static bool scripted_read(void *user_data, sce_hf_reading_t *out) {
    *out = ((rig_t *)user_data)->reading;
    return true;
}

static bool scripted_zone(void *user_data, sce_hf_zone_t *out) {
    *out = ((rig_t *)user_data)->zone;
    return true;
}

static sce_hf_exchange_t scripted_exchange(void *user_data, const sce_hf_http_request_t *request,
                                           sce_hf_http_response_t *out) {
    script_t *script = (script_t *)user_data;
    jn_t *entry = jn_new(J_OBJ);
    jn_set(entry, "method", jn_str(request->method));
    jn_set(entry, "url", jn_str(request->url));
    jn_t *headers = jn_new(J_ARR);
    for (size_t i = 0; i < request->header_count; ++i) {
        jn_t *header = jn_new(J_OBJ);
        jn_set(header, "name", jn_str(request->headers[i].name));
        jn_set(header, "value", jn_str(request->headers[i].value));
        jn_push(headers, header);
    }
    jn_set(entry, "headers", headers);
    jn_set(entry, "body", jn_str(request->body));
    jn_push(script->log, entry);
    if (script->hold) {
        return SCE_HF_EXCHANGE_HOLD;
    }
    if (script->fail || !script->set) {
        return SCE_HF_EXCHANGE_FAILURE;
    }
    out->status = script->status;
    out->header_count = script->header_count;
    out->headers = (sce_hf_header_t *)must(calloc(script->header_count + 1u, sizeof(sce_hf_header_t)));
    for (size_t i = 0; i < script->header_count; ++i) {
        out->headers[i].name = (char *)must(strdup(script->headers[i].name));
        out->headers[i].value = (char *)must(strdup(script->headers[i].value));
    }
    out->body_len = script->body_len;
    out->body = (unsigned char *)must(malloc(script->body_len + 1u));
    memcpy(out->body, script->body, script->body_len);
    return SCE_HF_EXCHANGE_RESPONSE;
}

static void script_clear_response(script_t *script) {
    for (size_t i = 0; i < script->header_count; ++i) {
        free(script->headers[i].name);
        free(script->headers[i].value);
    }
    free(script->headers);
    free(script->body);
    script->headers = NULL;
    script->header_count = 0;
    script->body = NULL;
    script->body_len = 0;
}

static int hex_value(char c) {
    return c <= '9' ? c - '0' : (c | 0x20) - 'a' + 10;
}

// What an `http` directive says the transport answers.
static void script_directive(script_t *script, const jn_t *d) {
    script_clear_response(script);
    script->set = true;
    const jn_t *hold = jn_get(d, "hold");
    script->hold = hold != NULL && hold->b;
    script->fail = jn_get(d, "fail") != NULL;
    if (script->hold || script->fail) {
        return;
    }
    script->status = (int)jn_get(d, "status")->i;
    const jn_t *headers = jn_get(d, "headers");
    script->header_count = headers->count;
    script->headers = (sce_hf_header_t *)must(calloc(headers->count + 1u, sizeof(sce_hf_header_t)));
    for (size_t i = 0; i < headers->count; ++i) {
        script->headers[i].name = (char *)must(strdup(headers->items[i]->items[0]->s));
        script->headers[i].value = (char *)must(strdup(headers->items[i]->items[1]->s));
    }
    const jn_t *body = jn_get(d, "body");
    const jn_t *hex = jn_get(d, "body_hex");
    const jn_t *repeat = jn_get(d, "body_repeat");
    if (body != NULL) {
        script->body = (unsigned char *)must(malloc(body->n + 1u));
        memcpy(script->body, body->s, body->n);
        script->body_len = body->n;
    } else if (hex != NULL) {
        script->body_len = hex->n / 2u;
        script->body = (unsigned char *)must(malloc(script->body_len + 1u));
        for (size_t i = 0; i < script->body_len; ++i) {
            script->body[i] = (unsigned char)(hex_value(hex->s[2u * i]) * 16 + hex_value(hex->s[2u * i + 1u]));
        }
    } else if (repeat != NULL) {
        const size_t unit = repeat->items[0]->n;
        const size_t times = (size_t)repeat->items[1]->i;
        script->body_len = unit * times;
        script->body = (unsigned char *)must(malloc(script->body_len + 1u));
        for (size_t i = 0; i < times; ++i) {
            memcpy(script->body + i * unit, repeat->items[0]->s, unit);
        }
    } else {
        script->body = (unsigned char *)must(malloc(1u));
    }
}

static int rig_directives(rig_t *rig, const jn_t *directives) {
    for (size_t i = 0; i < directives->count; ++i) {
        const char *name = directives->keys[i];
        const jn_t *value = directives->items[i];
        if (strcmp(name, "clock") == 0) {
            const jn_t *utc = jn_get(value, "utc_ms");
            const jn_t *mono = jn_get(value, "monotonic_ms");
            const jn_t *id = jn_get(value, "zone_id");
            const jn_t *offset = jn_get(value, "utc_offset_s");
            if (utc != NULL) {
                rig->reading.utc_ms = utc->i;
            }
            if (mono != NULL) {
                rig->reading.monotonic_ms = mono->i;
            }
            if (id != NULL) {
                (void)snprintf(rig->zone.id, sizeof(rig->zone.id), "%s", id->s);
            }
            if (offset != NULL) {
                rig->zone.utc_offset_s = (int32_t)offset->i;
            }
        } else if (strcmp(name, "permission") == 0) {
            sce_hf_set_permission(rig->host, value->s);
        } else if (strcmp(name, "user_decision") == 0) {
            sce_hf_set_user_decision(rig->host, value->s);
        } else if (strcmp(name, "quota") == 0) {
            sce_hf_set_quota(rig->host, (size_t)value->i);
        } else if (strcmp(name, "fail") == 0) {
            sce_hf_clear_faults(rig->host);
            for (size_t k = 0; k < value->count; ++k) {
                sce_hf_set_fault(rig->host, value->keys[k], value->items[k]->s);
            }
        } else if (strcmp(name, "http") == 0) {
            script_directive(&rig->transport, value);
        } else {
            (void)fprintf(stderr, "host_features: `%s` is not a host directive\n", name);
            return 1;
        }
    }
    return 0;
}

// What the host was started with since the last step, as a scenario writes it: the
// request as its record, `at_ms` being the one integer field.
static jn_t *rig_starts(rig_t *rig) {
    jn_t *out = jn_new(J_ARR);
    for (size_t i = 0; i < sce_hf_start_count(rig->host); ++i) {
        jn_t *request = jn_new(J_OBJ);
        for (size_t p = 0; p < sce_hf_start_param_count(rig->host, i); ++p) {
            const char *name;
            const char *value;
            sce_hf_start_param(rig->host, i, p, &name, &value);
            jn_set(request, name, strcmp(name, "at_ms") == 0 ? jn_int(strtoll(value, NULL, 10)) : jn_str(value));
        }
        jn_t *start = jn_new(J_OBJ);
        jn_set(start, "type", jn_str(sce_hf_start_operation(rig->host, i)));
        jn_set(start, "id", jn_str(sce_hf_start_id(rig->host, i)));
        jn_set(start, "request", request);
        jn_push(out, start);
    }
    sce_hf_clear_starts(rig->host);
    return out;
}

static jn_t *id_list(size_t count, const char *(*id)(const sce_hf_host_t *, size_t), const sce_hf_host_t *host) {
    jn_t *out = jn_new(J_ARR);
    for (size_t i = 0; i < count; ++i) {
        jn_push(out, jn_str(id(host, i)));
    }
    return out;
}

static jn_t *rig_state(rig_t *rig, const jn_t *wanted) {
    jn_t *state = jn_new(J_OBJ);
    for (size_t i = 0; i < wanted->count; ++i) {
        const char *name = wanted->keys[i];
        if (strcmp(name, "store") == 0) {
            jn_t *store = jn_new(J_OBJ);
            for (size_t k = 0; k < sce_hf_store_count(rig->host); ++k) {
                const char *key;
                const char *value;
                sce_hf_store_entry(rig->host, k, &key, &value);
                jn_set(store, key, jn_str(value));
            }
            jn_set(state, name, store);
        } else if (strcmp(name, "waiting") == 0) {
            jn_set(state, name, id_list(sce_hf_waiting_count(rig->host), sce_hf_waiting_id, rig->host));
        } else if (strcmp(name, "shown") == 0) {
            jn_set(state, name, id_list(sce_hf_shown_count(rig->host), sce_hf_shown_id, rig->host));
        } else if (strcmp(name, "cancelled") == 0) {
            jn_set(state, name, id_list(sce_hf_cancelled_count(rig->host), sce_hf_cancelled_id, rig->host));
        } else if (strcmp(name, "http") == 0) {
            // A copy: the log is the transport's and is read again by a later step.
            jn_t *log = jn_new(J_ARR);
            for (size_t k = 0; k < rig->transport.log->count; ++k) {
                jn_push(log, rig->transport.log->items[k]);
            }
            jn_set(state, name, log);
        }
    }
    return state;
}

static void free_state(jn_t *state, const jn_t *transport_log) {
    // The "http" member shares the transport log's entries: they are not this tree's to free.
    for (size_t i = 0; i < state->count; ++i) {
        if (strcmp(state->keys[i], "http") == 0) {
            state->items[i]->count = 0;
        }
    }
    (void)transport_log;
    jn_free(state);
}

// ── A machine as the replay reads it ─────────────────────────────────────────

typedef struct {
    const char *name;
    // A string, a truth value or a number, by the kind of the variable.
    jkind_t kind;
    const char *(*text)(const void *sm);
    long long (*number)(const void *sm);
} variable_t;

typedef struct {
    const char *name;
    // Starts the machine with `registry` as its host invokers, on a manual clock.
    void (*start)(void *sm, const sce_host_invoker_registry_t *registry);
    bool (*raise)(void *sm, const char *event, const char *data);
    void (*advance)(void *sm, uint64_t ms);
    void (*settle)(void *sm);
    // The atomic state the machine is in, by its document name; NULL for none or several.
    const char *(*state)(const void *sm);
    const variable_t *variables;
    size_t variable_count;
    size_t size;
} machine_t;

#define TEXT_VAR(M, VAR)                                                                                               \
    static const char *M##_text_##VAR(const void *sm) {                                                                \
        return M##_get_##VAR((const M##_t *)sm);                                                                       \
    }
#define NUMBER_VAR(M, VAR)                                                                                             \
    static long long M##_number_##VAR(const void *sm) {                                                                \
        return (long long)M##_get_##VAR((const M##_t *)sm);                                                            \
    }
#define TEXT_ROW(M, VAR) {#VAR, J_STR, M##_text_##VAR, NULL}
#define NUMBER_ROW(M, KIND, VAR) {#VAR, KIND, NULL, M##_number_##VAR}

// A machine that waits on no clock is started with its invokers and driven by `_step`; one that does (the HTTP
// exchange carries a deadline) is started on a manual clock the scenario moves on and driven by `_tick`.
#define START_PLAIN(M) M##_init_with_host_invokers((M##_t *)sm, registry)
#define START_SCHEDULED(M) M##_init_with_clock_and_host_invokers((M##_t *)sm, sce_clock_manual(0u), registry)
#define ADVANCE_PLAIN(M)                                                                                               \
    (void)sm;                                                                                                          \
    (void)ms
#define ADVANCE_SCHEDULED(M) M##_advance_time_ms((M##_t *)sm, ms)
#define SETTLE_PLAIN(M) M##_step((M##_t *)sm)
#define SETTLE_SCHEDULED(M)                                                                                            \
    for (int round = 0; round < 5; ++round) {                                                                          \
        M##_tick((M##_t *)sm);                                                                                         \
    }

#define MACHINE_COMMON(M, KIND)                                                                                        \
    static void M##_start(void *sm, const sce_host_invoker_registry_t *registry) {                                     \
        START_##KIND(M);                                                                                               \
    }                                                                                                                  \
    static bool M##_raise_by_name(void *sm, const char *name, const char *data) {                                      \
        M##_event_t event;                                                                                             \
        if (!M##_resolve_event_by_name(name, &event)) {                                                                \
            return false;                                                                                              \
        }                                                                                                              \
        M##_event_with_meta_t meta;                                                                                    \
        memset(&meta, 0, sizeof(meta));                                                                                \
        meta.event = event;                                                                                            \
        if (data != NULL) {                                                                                            \
            if (strlen(data) >= sizeof(meta.data)) {                                                                   \
                return false;                                                                                          \
            }                                                                                                          \
            memcpy(meta.data, data, strlen(data) + 1u);                                                                \
        }                                                                                                              \
        M##_raise_external((M##_t *)sm, &meta);                                                                        \
        return true;                                                                                                   \
    }                                                                                                                  \
    static void M##_advance(void *sm, uint64_t ms) {                                                                   \
        ADVANCE_##KIND(M);                                                                                             \
    }                                                                                                                  \
    static void M##_settle(void *sm) {                                                                                 \
        SETTLE_##KIND(M);                                                                                              \
    }                                                                                                                  \
    static const char *M##_state(const void *sm) {                                                                     \
        const char *found = NULL;                                                                                      \
        for (int s = 0; s < (int)M##_STATE_COUNT_VALUE; ++s) {                                                         \
            if (M##_in_state((const M##_t *)sm, (M##_state_t)s)) {                                                     \
                if (found != NULL) {                                                                                   \
                    return NULL;                                                                                       \
                }                                                                                                      \
                found = M##_state_name((M##_state_t)s);                                                                \
            }                                                                                                          \
        }                                                                                                              \
        return found;                                                                                                  \
    }

#define static_host_kv_STATE_COUNT_VALUE STATIC_HOST_KV_STATE_COUNT
#define static_host_clock_STATE_COUNT_VALUE STATIC_HOST_CLOCK_STATE_COUNT
#define static_host_notify_STATE_COUNT_VALUE STATIC_HOST_NOTIFY_STATE_COUNT
#define static_host_http_STATE_COUNT_VALUE STATIC_HOST_HTTP_STATE_COUNT

MACHINE_COMMON(static_host_kv, PLAIN)
TEXT_VAR(static_host_kv, outcome)
NUMBER_VAR(static_host_kv, found)
TEXT_VAR(static_host_kv, got)
NUMBER_VAR(static_host_kv, replaced)
NUMBER_VAR(static_host_kv, existed)
NUMBER_VAR(static_host_kv, next_found)
TEXT_VAR(static_host_kv, next)
static const variable_t kv_variables[] = {
    TEXT_ROW(static_host_kv, outcome),
    NUMBER_ROW(static_host_kv, J_BOOL, found),
    TEXT_ROW(static_host_kv, got),
    NUMBER_ROW(static_host_kv, J_BOOL, replaced),
    NUMBER_ROW(static_host_kv, J_BOOL, existed),
    NUMBER_ROW(static_host_kv, J_BOOL, next_found),
    TEXT_ROW(static_host_kv, next),
};

MACHINE_COMMON(static_host_clock, PLAIN)
TEXT_VAR(static_host_clock, outcome)
NUMBER_VAR(static_host_clock, utc_ms)
NUMBER_VAR(static_host_clock, monotonic_ms)
TEXT_VAR(static_host_clock, zone_id)
NUMBER_VAR(static_host_clock, utc_offset_s)
static const variable_t clock_variables[] = {
    TEXT_ROW(static_host_clock, outcome),
    NUMBER_ROW(static_host_clock, J_NUM, utc_ms),
    NUMBER_ROW(static_host_clock, J_NUM, monotonic_ms),
    TEXT_ROW(static_host_clock, zone_id),
    NUMBER_ROW(static_host_clock, J_NUM, utc_offset_s),
};

MACHINE_COMMON(static_host_notify, PLAIN)
TEXT_VAR(static_host_notify, outcome)
TEXT_VAR(static_host_notify, permission)
TEXT_VAR(static_host_notify, status)
NUMBER_VAR(static_host_notify, existed)
static const variable_t notify_variables[] = {
    TEXT_ROW(static_host_notify, outcome),
    TEXT_ROW(static_host_notify, permission),
    TEXT_ROW(static_host_notify, status),
    NUMBER_ROW(static_host_notify, J_BOOL, existed),
};

MACHINE_COMMON(static_host_http, SCHEDULED)
TEXT_VAR(static_host_http, outcome)
NUMBER_VAR(static_host_http, status)
TEXT_VAR(static_host_http, response_headers)
TEXT_VAR(static_host_http, response_body)
static const variable_t http_variables[] = {
    TEXT_ROW(static_host_http, outcome),
    NUMBER_ROW(static_host_http, J_NUM, status),
    TEXT_ROW(static_host_http, response_headers),
    TEXT_ROW(static_host_http, response_body),
};

#define MACHINE_ROW(M, VARIABLES)                                                                                      \
    {#M,           M##_start, M##_raise_by_name, M##_advance,                                                          \
     M##_settle,   M##_state, VARIABLES,         sizeof(VARIABLES) / sizeof((VARIABLES)[0]),                           \
     sizeof(M##_t)}

static const machine_t machines[] = {
    MACHINE_ROW(static_host_kv, kv_variables),
    MACHINE_ROW(static_host_clock, clock_variables),
    MACHINE_ROW(static_host_notify, notify_variables),
    MACHINE_ROW(static_host_http, http_variables),
};

// ── The replay ───────────────────────────────────────────────────────────────

static char *read_file(const char *path) {
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return NULL;
    }
    (void)fseek(file, 0, SEEK_END);
    const long size = ftell(file);
    (void)fseek(file, 0, SEEK_SET);
    char *text = (char *)must(malloc((size_t)size + 1u));
    const size_t got = fread(text, 1u, (size_t)size, file);
    text[got] = '\0';
    (void)fclose(file);
    return text;
}

static int fail(const char *machine, int step, const char *note, const char *what, const jn_t *got, const jn_t *want) {
    (void)fprintf(stderr, "host_features: FAIL [%s] step %d (%s): %s", machine, step, note, what);
    if (got != NULL) {
        (void)fputs(" - got ", stderr);
        dump(got, stderr);
    }
    if (want != NULL) {
        (void)fputs(", want ", stderr);
        dump(want, stderr);
    }
    (void)fputc('\n', stderr);
    return 1;
}

static int replay(const machine_t *machine) {
    char path[512];
    (void)snprintf(path, sizeof(path), "%s/%s.json", SCE_HOST_FEATURES_SCENARIO_DIR, machine->name);
    char *text = read_file(path);
    if (text == NULL) {
        (void)fprintf(stderr, "host_features: the scenario %s is not readable\n", path);
        return 1;
    }
    parser_t ps = {text, text + strlen(text), true};
    jn_t *scenario = parse_value(&ps);
    if (scenario == NULL || !ps.ok) {
        (void)fprintf(stderr, "host_features: the scenario %s is not JSON\n", path);
        free(text);
        return 1;
    }
    int bad = 0;
    const jn_t *steps = jn_get(scenario, "steps");
    if (steps == NULL || steps->count == 0u) {
        (void)fprintf(stderr, "host_features: a scenario with no steps judges nothing\n");
        bad = 1;
    }

    rig_t rig;
    memset(&rig, 0, sizeof(rig));
    rig.transport.log = jn_new(J_ARR);
    sce_hf_clock_t clock = {scripted_read, scripted_zone, &rig};
    sce_hf_transport_t transport = {scripted_exchange, &rig.transport};
    rig.host = sce_hf_host_new(clock, transport);
    sce_host_invoker_registry_t registry;
    memset(&registry, 0, sizeof(registry));
    if (!sce_hf_register(&registry, rig.host)) {
        (void)fprintf(stderr, "host_features: the registry cannot hold the standard types (SCE_MAX_HOST_INVOKERS)\n");
        bad = 1;
    }
    void *sm = calloc(1u, machine->size);
    machine->start(sm, &registry);
    machine->settle(sm);

    for (size_t n = 0; !bad && steps != NULL && n < steps->count; ++n) {
        const jn_t *step = steps->items[n];
        const jn_t *note_node = jn_get(step, "note");
        const char *note = note_node != NULL ? note_node->s : "";
        const jn_t *host = jn_get(step, "host");
        if (host != NULL) {
            bad |= rig_directives(&rig, host);
        }
        const jn_t *advance = jn_get(step, "advance_ms");
        if (advance != NULL) {
            machine->advance(sm, (uint64_t)advance->i);
            machine->settle(sm);
        }
        const jn_t *event = jn_get(step, "event");
        if (event != NULL) {
            const jn_t *data = jn_get(step, "data");
            char *payload = data != NULL ? (char *)must(strndup(data->raw, data->raw_len)) : NULL;
            if (!machine->raise(sm, event->s, payload)) {
                bad |= fail(machine->name, (int)n, note, "the machine's events do not match the step's", NULL, NULL);
            }
            free(payload);
            machine->settle(sm);
        }
        const jn_t *expect = jn_get(step, "expect");
        jn_t *started = rig_starts(&rig);
        const jn_t *want_starts = jn_get(expect, "host_invokes");
        if (want_starts != NULL && !jn_equal(started, want_starts)) {
            bad |= fail(machine->name, (int)n, note, "what the host was asked", started, want_starts);
        }
        jn_free(started);
        const jn_t *want_state = jn_get(expect, "host_state");
        if (want_state != NULL) {
            jn_t *got = rig_state(&rig, want_state);
            if (!jn_equal(got, want_state)) {
                bad |= fail(machine->name, (int)n, note, "what the host holds", got, want_state);
            }
            free_state(got, rig.transport.log);
        }
        const jn_t *state = jn_get(expect, "state");
        if (state != NULL) {
            const char *atomic = machine->state(sm);
            if (atomic == NULL || strcmp(atomic, state->s) != 0) {
                (void)fprintf(stderr, "host_features: FAIL [%s] step %zu (%s): the current state is %s, want %s\n",
                              machine->name, n, note, atomic != NULL ? atomic : "(none)", state->s);
                bad = 1;
            }
        }
        const jn_t *variables = jn_get(expect, "variables");
        for (size_t v = 0; variables != NULL && v < variables->count; ++v) {
            const variable_t *row = NULL;
            for (size_t k = 0; k < machine->variable_count; ++k) {
                if (strcmp(machine->variables[k].name, variables->keys[v]) == 0) {
                    row = &machine->variables[k];
                }
            }
            if (row == NULL) {
                (void)fprintf(stderr, "host_features: the driver reads no variable `%s`\n", variables->keys[v]);
                bad = 1;
                continue;
            }
            jn_t *got = row->kind == J_STR    ? jn_str(row->text(sm))
                        : row->kind == J_BOOL ? jn_bool(row->number(sm) != 0)
                                              : jn_int(row->number(sm));
            if (!jn_equal(got, variables->items[v])) {
                char what[96];
                (void)snprintf(what, sizeof(what), "variable `%s`", variables->keys[v]);
                bad |= fail(machine->name, (int)n, note, what, got, variables->items[v]);
            }
            jn_free(got);
        }
    }
    free(sm);
    jn_free(rig.transport.log);
    script_clear_response(&rig.transport);
    sce_hf_host_free(rig.host);
    jn_free(scenario);
    free(text);
    return bad;
}

// What the host holds apart from the scenarios: its system clock, what it does not serve, and the order of
// its keys. A C string holds no U+0000, so a key with one cannot arrive.
static bool serves_only_the_standard_set_and_a_monotonic_clock(void) {
    bool ok = true;
    sce_hf_transport_t transport = {scripted_exchange, NULL};
    script_t script;
    memset(&script, 0, sizeof(script));
    script.log = jn_new(J_ARR);
    transport.user_data = &script;
    sce_hf_host_t *host = sce_hf_host_new(sce_hf_system_clock(), transport);
    sce_hf_outcome_t outcome;
    if (sce_hf_start(host, "x-app-host", "id", NULL, 0u, &outcome)) {
        (void)fprintf(stderr, "host_features: a type outside the standard set was served\n");
        ok = false;
    }
    long long last = -1;
    for (int i = 0; i < 50; ++i) {
        if (!sce_hf_start(host, "sce-host-clock-now", "now", NULL, 0u, &outcome) || outcome.kind != SCE_HF_DONE) {
            ok = false;
            break;
        }
        const char *at = strstr(outcome.data, "\"monotonic_ms\":\"");
        const long long mono = at != NULL ? strtoll(at + strlen("\"monotonic_ms\":\""), NULL, 10) : -2;
        if (mono < last) {
            (void)fprintf(stderr, "host_features: the monotonic reading went from %lld to %lld\n", last, mono);
            ok = false;
        }
        last = mono;
        sce_hf_outcome_free(&outcome);
    }
    const sce_host_send_param_t emoji = {"key", "\xF0\x9F\x98\x80"};
    const sce_host_send_param_t wide = {"key", "\xEF\xBD\x9E"};
    const sce_host_send_param_t value = {"value", "v"};
    const sce_host_send_param_t one[2] = {emoji, value};
    const sce_host_send_param_t two[2] = {wide, value};
    (void)sce_hf_start(host, "sce-host-kv-put", "put", one, 2u, &outcome);
    sce_hf_outcome_free(&outcome);
    (void)sce_hf_start(host, "sce-host-kv-put", "put", two, 2u, &outcome);
    sce_hf_outcome_free(&outcome);
    const char *first_key;
    const char *first_value;
    sce_hf_store_entry(host, 0u, &first_key, &first_value);
    if (strcmp(first_key, "\xEF\xBD\x9E") != 0) {
        (void)fprintf(stderr, "host_features: the keys are not in the order of their UTF-8 bytes\n");
        ok = false;
    }
    sce_hf_host_free(host);
    jn_free(script.log);
    return ok;
}

int main(void) {
    int bad = 0;
    if (!serves_only_the_standard_set_and_a_monotonic_clock()) {
        bad = 1;
    }
    for (size_t i = 0; i < sizeof(machines) / sizeof(machines[0]); ++i) {
        const int one = replay(&machines[i]);
        (void)printf("host_features: %s %s\n", machines[i].name, one == 0 ? "ok" : "FAILED");
        bad |= one;
    }
    return bad;
}
