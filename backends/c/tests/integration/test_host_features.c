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
#include "static_host_model_sm.h"
#include "static_host_notify_sm.h"
#include "static_model_harness_sm.h"

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
    sce_hf_scripted_provider_t *model;
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

// What a `model` directive says the scripted provider is like: each member it names. An answer is `text`, or
// `text_repeat` as a text and a count, with `finish`, `stop` when not written.
static void model_directive(sce_hf_scripted_provider_t *model, const jn_t *d) {
    const jn_t *answers = jn_get(d, "answers");
    if (answers != NULL) {
        sce_hf_scripted_clear_answers(model);
        for (size_t i = 0; i < answers->count; ++i) {
            const jn_t *answer = answers->items[i];
            const jn_t *text = jn_get(answer, "text");
            const jn_t *finish = jn_get(answer, "finish");
            const char *ends = finish != NULL ? finish->s : "stop";
            if (text != NULL) {
                sce_hf_scripted_push_answer(model, text->s, text->n, ends);
                continue;
            }
            const jn_t *repeat = jn_get(answer, "text_repeat");
            const size_t unit = repeat->items[0]->n;
            const size_t times = (size_t)repeat->items[1]->i;
            char *joined = (char *)must(malloc(unit * times + 1u));
            for (size_t k = 0; k < times; ++k) {
                memcpy(joined + k * unit, repeat->items[0]->s, unit);
            }
            joined[unit * times] = '\0';
            sce_hf_scripted_push_answer(model, joined, unit * times, ends);
            free(joined);
        }
    }
    const jn_t *constrained = jn_get(d, "constrained");
    if (constrained != NULL) {
        sce_hf_scripted_set_constrained(model, constrained->b);
    }
    const jn_t *name = jn_get(d, "name");
    if (name != NULL) {
        sce_hf_scripted_set_name(model, name->s);
    }
    const jn_t *context = jn_get(d, "context");
    if (context != NULL) {
        sce_hf_scripted_set_context(model, (uint64_t)context->i);
    }
    const jn_t *hold = jn_get(d, "hold");
    if (hold != NULL) {
        sce_hf_scripted_set_hold(model, hold->b);
    }
}

// The request fields a scenario writes as numbers; every other is text.
static bool integer_field(const char *name) {
    return strcmp(name, "at_ms") == 0 || strcmp(name, "max_tokens") == 0 || strcmp(name, "temperature_milli") == 0;
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
        } else if (strcmp(name, "model") == 0) {
            model_directive(rig->model, value);
        } else {
            (void)fprintf(stderr, "host_features: `%s` is not a host directive\n", name);
            return 1;
        }
    }
    return 0;
}

// What the host was started with since the last step, as a scenario writes it: the
// request as its record, the integer fields as numbers.
static jn_t *rig_starts(rig_t *rig) {
    jn_t *out = jn_new(J_ARR);
    for (size_t i = 0; i < sce_hf_start_count(rig->host); ++i) {
        jn_t *request = jn_new(J_OBJ);
        for (size_t p = 0; p < sce_hf_start_param_count(rig->host, i); ++p) {
            const char *name;
            const char *value;
            sce_hf_start_param(rig->host, i, p, &name, &value);
            jn_set(request, name, integer_field(name) ? jn_int(strtoll(value, NULL, 10)) : jn_str(value));
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
    // A string, a truth value, a number, or a value of any shape (J_OBJ) read whole, by the kind of the
    // variable.
    jkind_t kind;
    const char *(*text)(const void *sm);
    long long (*number)(const void *sm);
    jn_t *(*value)(const void *sm);
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
    // Hands the host the machine's table of answer schemas; NULL for a machine that names none.
    void (*answer_schemas)(sce_hf_host_t *host);
} machine_t;

#define TEXT_VAR(M, VAR)                                                                                               \
    static const char *M##_text_##VAR(const void *sm) {                                                                \
        return M##_get_##VAR((const M##_t *)sm);                                                                       \
    }
#define NUMBER_VAR(M, VAR)                                                                                             \
    static long long M##_number_##VAR(const void *sm) {                                                                \
        return (long long)M##_get_##VAR((const M##_t *)sm);                                                            \
    }
#define TEXT_ROW(M, VAR) {#VAR, J_STR, M##_text_##VAR, NULL, NULL}
#define NUMBER_ROW(M, KIND, VAR) {#VAR, KIND, NULL, M##_number_##VAR, NULL}

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
        /* A view of the data; the machine's queue copies it, and data past the slot is refused here. */               \
        if (data != NULL && strlen(data) >= sizeof(((M##_queued_event_t *)0)->data)) {                                 \
            return false;                                                                                              \
        }                                                                                                              \
        meta.data = data;                                                                                              \
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
    {#M,                                                                                                               \
     M##_start,                                                                                                        \
     M##_raise_by_name,                                                                                                \
     M##_advance,                                                                                                      \
     M##_settle,                                                                                                       \
     M##_state,                                                                                                        \
     VARIABLES,                                                                                                        \
     sizeof(VARIABLES) / sizeof((VARIABLES)[0]),                                                                       \
     sizeof(M##_t),                                                                                                    \
     NULL}

// static_host_model: a generation under an answer schema, and the record read from its answer or the refusal
// of it. Both records may be absent; a record's text is its buffer's text, its byte string the Latin-1 text of
// its bytes, a 64-bit field the text of its digits, and an enum the name its document declares.
#define static_host_model_STATE_COUNT_VALUE STATIC_HOST_MODEL_STATE_COUNT
MACHINE_COMMON(static_host_model, SCHEDULED)
NUMBER_VAR(static_host_model, calls)
TEXT_VAR(static_host_model, outcome)
TEXT_VAR(static_host_model, finish)
NUMBER_VAR(static_host_model, constrained)
TEXT_VAR(static_host_model, answered_by)
NUMBER_VAR(static_host_model, prompt_tokens)
NUMBER_VAR(static_host_model, answer_tokens)

// The bytes of `data[0..len)` as the text whose characters are those bytes' code points, in UTF-8.
static jn_t *latin1_text(const uint8_t *data, size_t len) {
    char *text = (char *)must(malloc(2u * len + 1u));
    size_t n = 0;
    for (size_t i = 0; i < len; ++i) {
        if (data[i] < 0x80u) {
            text[n++] = (char)data[i];
        } else {
            text[n++] = (char)(0xC0u | (data[i] >> 6));
            text[n++] = (char)(0x80u | (data[i] & 0x3Fu));
        }
    }
    text[n] = '\0';
    jn_t *node = jn_str(text);
    free(text);
    return node;
}

static jn_t *static_host_model_value_answer(const void *sm) {
    const sce_optional_record_static_host_model_record_answer_t held =
        static_host_model_get_answer((const static_host_model_t *)sm);
    if (!held.present) {
        return jn_new(J_NULL);
    }
    char digits[32];
    (void)snprintf(digits, sizeof(digits), "%lld", (long long)held.value.amount);
    jn_t *answer = jn_new(J_OBJ);
    jn_set(answer, "id", jn_int((long long)held.value.id));
    jn_set(answer, "amount", jn_str(digits));
    jn_set(answer, "title", jn_str(held.value.title.data));
    jn_set(answer, "kind", jn_str(enum_lift_kind_declared_name(held.value.kind)));
    jn_set(answer, "note", held.value.note.present ? jn_str(held.value.note.value.data) : jn_new(J_NULL));
    jn_set(answer, "tag", latin1_text(held.value.tag.data, held.value.tag.len));
    jn_set(answer, "priority", jn_int((long long)held.value.priority));
    return answer;
}

static jn_t *static_host_model_value_why(const void *sm) {
    const sce_optional_record_static_host_model_record_lift_refusal_t held =
        static_host_model_get_why((const static_host_model_t *)sm);
    if (!held.present) {
        return jn_new(J_NULL);
    }
    jn_t *why = jn_new(J_OBJ);
    jn_set(why, "code", jn_str(refusal_code_declared_name(held.value.code)));
    jn_set(why, "path", jn_str(held.value.path.data));
    jn_set(why, "bound", held.value.bound.present ? jn_int((long long)held.value.bound.value) : jn_new(J_NULL));
    return why;
}

#define VALUE_ROW(M, VAR) {#VAR, J_OBJ, NULL, NULL, M##_value_##VAR}

static const variable_t model_variables[] = {
    NUMBER_ROW(static_host_model, J_NUM, calls),
    TEXT_ROW(static_host_model, outcome),
    TEXT_ROW(static_host_model, finish),
    NUMBER_ROW(static_host_model, J_BOOL, constrained),
    TEXT_ROW(static_host_model, answered_by),
    NUMBER_ROW(static_host_model, J_NUM, prompt_tokens),
    NUMBER_ROW(static_host_model, J_NUM, answer_tokens),
    VALUE_ROW(static_host_model, answer),
    VALUE_ROW(static_host_model, why),
};

static void static_host_model_answer_schemas_to(sce_hf_host_t *host) {
    sce_hf_set_answer_schemas(host, static_host_model_answer_schemas(), STATIC_HOST_MODEL_ANSWER_SCHEMA_COUNT);
}

static const machine_t machines[] = {
    MACHINE_ROW(static_host_kv, kv_variables),
    MACHINE_ROW(static_host_clock, clock_variables),
    MACHINE_ROW(static_host_notify, notify_variables),
    MACHINE_ROW(static_host_http, http_variables),
    {"static_host_model", static_host_model_start, static_host_model_raise_by_name, static_host_model_advance,
     static_host_model_settle, static_host_model_state, model_variables,
     sizeof(model_variables) / sizeof(model_variables[0]), sizeof(static_host_model_t),
     static_host_model_answer_schemas_to},
};

// static_model_harness: the reference harness (docs/adr/0087, section 6). Its run ends at a final inside its
// root, so its configuration holds a compound state beside the leaf; its leaf and its variables are read from
// its saved state, the text every backend saves alike, as the other lanes read them.

// A deep copy of `node`, which owns nothing of the tree it was read from.
static jn_t *jn_copy(const jn_t *node) {
    jn_t *copy = jn_new(node->kind);
    copy->b = node->b;
    copy->is_int = node->is_int;
    copy->i = node->i;
    copy->d = node->d;
    if (node->s != NULL) {
        copy->s = (char *)must(malloc(node->n + 1u));
        memcpy(copy->s, node->s, node->n);
        copy->s[node->n] = '\0';
        copy->n = node->n;
    }
    for (size_t i = 0; i < node->count; ++i) {
        if (node->kind == J_OBJ) {
            jn_set(copy, node->keys[i], jn_copy(node->items[i]));
        } else {
            jn_push(copy, jn_copy(node->items[i]));
        }
    }
    return copy;
}

// The harness's saved state as a tree, or NULL when it cannot be saved.
static jn_t *harness_saved(const void *sm) {
    sce_saved_error_t err;
    size_t len = 0;
    (void)static_model_harness_save((const static_model_harness_t *)sm, 0, NULL, 0u, &len, &err);
    char *text = (char *)must(malloc(len + 1u));
    if (!static_model_harness_save((const static_model_harness_t *)sm, 0, text, len + 1u, &len, &err)) {
        (void)fprintf(stderr, "host_features: the harness was not saved: %s\n", err.message);
        free(text);
        return NULL;
    }
    parser_t ps = {text, text + len, true};
    jn_t *saved = parse_value(&ps);
    jn_t *copy = saved != NULL && ps.ok ? jn_copy(saved) : NULL;
    jn_free(saved);
    free(text);
    return copy;
}

// The variable `name` of the harness's saved state; null when it cannot be read.
static jn_t *harness_saved_variable(const void *sm, const char *name) {
    jn_t *saved = harness_saved(sm);
    const jn_t *value = jn_get(jn_get(saved, "variables"), name);
    jn_t *copy = value != NULL ? jn_copy(value) : jn_new(J_NULL);
    jn_free(saved);
    return copy;
}

#define HARNESS_VARIABLE(VAR)                                                                                          \
    static jn_t *harness_value_##VAR(const void *sm) {                                                                 \
        return harness_saved_variable(sm, #VAR);                                                                       \
    }
HARNESS_VARIABLE(outcome)
HARNESS_VARIABLE(attempts)
HARNESS_VARIABLE(offered)
HARNESS_VARIABLE(refusals)
HARNESS_VARIABLE(answer)
HARNESS_VARIABLE(why)

static const variable_t harness_variables[] = {
    {"outcome", J_OBJ, NULL, NULL, harness_value_outcome}, {"attempts", J_OBJ, NULL, NULL, harness_value_attempts},
    {"offered", J_OBJ, NULL, NULL, harness_value_offered}, {"refusals", J_OBJ, NULL, NULL, harness_value_refusals},
    {"answer", J_OBJ, NULL, NULL, harness_value_answer},   {"why", J_OBJ, NULL, NULL, harness_value_why},
};

static void harness_start(void *sm, const sce_host_invoker_registry_t *registry) {
    static_model_harness_init_with_clock_and_host_invokers((static_model_harness_t *)sm, sce_clock_manual(0u),
                                                           registry);
}

static bool harness_raise(void *sm, const char *name, const char *data) {
    static_model_harness_event_t event;
    if (!static_model_harness_resolve_event_by_name(name, &event)) {
        return false;
    }
    static_model_harness_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    /* A view of the data; the machine's queue copies it, and data past the slot is refused here. */
    if (data != NULL && strlen(data) >= sizeof(((static_model_harness_queued_event_t *)0)->data)) {
        return false;
    }
    meta.data = data;
    static_model_harness_raise_external((static_model_harness_t *)sm, &meta);
    return true;
}

static void harness_advance(void *sm, uint64_t ms) {
    static_model_harness_advance_time_ms((static_model_harness_t *)sm, ms);
}

// A run may make its calls and take each completion within one event, a call a round.
static void harness_settle(void *sm) {
    for (int round = 0; round < 8; ++round) {
        static_model_harness_tick((static_model_harness_t *)sm);
    }
}

// The leaf the harness is in, by its document name, as its saved state's `current` holds it.
static const char *harness_leaf(const void *sm) {
    static char leaf[64];
    jn_t *saved = harness_saved(sm);
    const jn_t *current = jn_get(saved, "current");
    if (current == NULL || current->kind != J_STR || current->n >= sizeof(leaf)) {
        jn_free(saved);
        return NULL;
    }
    memcpy(leaf, current->s, current->n + 1u);
    jn_free(saved);
    return leaf;
}

static void harness_answer_schemas_to(sce_hf_host_t *host) {
    sce_hf_set_answer_schemas(host, static_model_harness_answer_schemas(), STATIC_MODEL_HARNESS_ANSWER_SCHEMA_COUNT);
}

static const machine_t harness_machine = {"static_model_harness",
                                          harness_start,
                                          harness_raise,
                                          harness_advance,
                                          harness_settle,
                                          harness_leaf,
                                          harness_variables,
                                          sizeof(harness_variables) / sizeof(harness_variables[0]),
                                          sizeof(static_model_harness_t),
                                          harness_answer_schemas_to};

// The cases of the reference harness's scenario (docs/adr/0087, section 9.1 item 8), one scenario each: a run
// of the harness ends at one of its finals, so each case is a run of its own.
static const char *const harness_scenarios[] = {
    "static_model_harness_accepted", "static_model_harness_exhausted", "static_model_harness_not_offered",
    "static_model_harness_repaired", "static_model_harness_timeout",
};

// The rows the transition observer of docs/adr/0089 is told in a run, in order. This target builds the harness
// with STATIC_MODEL_HARNESS_TRANSITION_OBSERVER=1 (backends/c/tests/CMakeLists.txt), so the machine calls this
// hook for each transition it takes, by the row of its emitted transition table.
#if !STATIC_MODEL_HARNESS_TRANSITION_OBSERVER
#error "this target builds static_model_harness with its transition observer (backends/c/tests/CMakeLists.txt)"
#endif

#define HARNESS_SEEN_MAX 256u

static static_model_harness_state_t harness_seen_source[HARNESS_SEEN_MAX];
static uint32_t harness_seen_index[HARNESS_SEEN_MAX];
static size_t harness_seen_count;

void static_model_harness_observe_transition(const static_model_harness_t *sm, static_model_harness_state_t source,
                                             uint32_t index) {
    (void)sm;
    if (harness_seen_count < HARNESS_SEEN_MAX) {
        harness_seen_source[harness_seen_count] = source;
        harness_seen_index[harness_seen_count] = index;
    }
    ++harness_seen_count;
}

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

// Replays the scenario `stem` against `machine`, the machine the scenario names: a machine whose run ends has
// one scenario a run (the reference harness's cases). Nonzero when the machine does not do what it says.
static int replay(const machine_t *machine, const char *stem) {
    char path[512];
    (void)snprintf(path, sizeof(path), "%s/%s.json", SCE_HOST_FEATURES_SCENARIO_DIR, stem);
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
    const jn_t *named = jn_get(scenario, "machine");
    if (named == NULL || named->kind != J_STR || strcmp(named->s, machine->name) != 0) {
        (void)fprintf(stderr, "host_features: the scenario %s names another machine than %s\n", stem, machine->name);
        bad = 1;
    }
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
    rig.model = sce_hf_scripted_provider_new();
    sce_hf_set_model_provider(rig.host, sce_hf_scripted_provider(rig.model));
    if (machine->answer_schemas != NULL) {
        machine->answer_schemas(rig.host);
    }
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
                bad |= fail(stem, (int)n, note, "the machine's events do not match the step's", NULL, NULL);
            }
            free(payload);
            machine->settle(sm);
        }
        const jn_t *expect = jn_get(step, "expect");
        jn_t *started = rig_starts(&rig);
        const jn_t *want_starts = jn_get(expect, "host_invokes");
        if (want_starts != NULL && !jn_equal(started, want_starts)) {
            bad |= fail(stem, (int)n, note, "what the host was asked", started, want_starts);
        }
        jn_free(started);
        const jn_t *want_state = jn_get(expect, "host_state");
        if (want_state != NULL) {
            jn_t *got = rig_state(&rig, want_state);
            if (!jn_equal(got, want_state)) {
                bad |= fail(stem, (int)n, note, "what the host holds", got, want_state);
            }
            free_state(got, rig.transport.log);
        }
        const jn_t *state = jn_get(expect, "state");
        if (state != NULL) {
            const char *atomic = machine->state(sm);
            if (atomic == NULL || strcmp(atomic, state->s) != 0) {
                (void)fprintf(stderr, "host_features: FAIL [%s] step %zu (%s): the current state is %s, want %s\n",
                              stem, n, note, atomic != NULL ? atomic : "(none)", state->s);
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
            jn_t *got = row->kind == J_OBJ    ? row->value(sm)
                        : row->kind == J_STR  ? jn_str(row->text(sm))
                        : row->kind == J_BOOL ? jn_bool(row->number(sm) != 0)
                                              : jn_int(row->number(sm));
            if (!jn_equal(got, variables->items[v])) {
                char what[96];
                (void)snprintf(what, sizeof(what), "variable `%s`", variables->keys[v]);
                bad |= fail(stem, (int)n, note, what, got, variables->items[v]);
            }
            jn_free(got);
        }
    }
    free(sm);
    jn_free(rig.transport.log);
    script_clear_response(&rig.transport);
    sce_hf_host_free(rig.host);
    sce_hf_scripted_provider_free(rig.model);
    jn_free(scenario);
    free(text);
    return bad;
}

// What the exhaustive provider answers for a class of the table.
static sce_hf_outcome_class_t outcome_class(const jn_t *cls) {
    sce_hf_outcome_class_t out = {SCE_HF_GENERATION_HOLD, "", "", ""};
    const char *end = jn_get(cls, "end")->s;
    if (strcmp(end, "answer") == 0) {
        out.kind = SCE_HF_GENERATION_ANSWER;
        out.text = jn_get(cls, "text")->s;
        out.finish = jn_get(cls, "finish")->s;
    } else if (strcmp(end, "failure") == 0) {
        out.kind = SCE_HF_GENERATION_FAILURE;
        out.code = jn_get(cls, "error")->s;
    } else if (strcmp(end, "refusal") == 0) {
        out.kind = SCE_HF_GENERATION_REFUSAL;
    }
    return out;
}

// Run static_host_model against the exhaustive provider answering the classes `sequence[0..length)` names, one
// call a class, and hold what the machine publishes after each call to what its class says. Nonzero when it
// does not.
static int run_sequence(const jn_t *table, const size_t *sequence, size_t length) {
    const machine_t *machine = &machines[sizeof(machines) / sizeof(machines[0]) - 1u];
    const jn_t *classes = jn_get(table, "classes");
    sce_hf_outcome_class_t provided[8];
    char named[256] = "";
    for (size_t i = 0; i < length; ++i) {
        provided[i] = outcome_class(classes->items[sequence[i]]);
        (void)strncat(named, jn_get(classes->items[sequence[i]], "class")->s, sizeof(named) - strlen(named) - 2u);
        (void)strncat(named, " ", sizeof(named) - strlen(named) - 1u);
    }
    rig_t rig;
    memset(&rig, 0, sizeof(rig));
    rig.transport.log = jn_new(J_ARR);
    sce_hf_clock_t clock = {scripted_read, scripted_zone, &rig};
    sce_hf_transport_t transport = {scripted_exchange, &rig.transport};
    rig.host = sce_hf_host_new(clock, transport);
    sce_hf_exhaustive_provider_t *provider = sce_hf_exhaustive_provider_new(provided, length);
    sce_hf_set_model_provider(rig.host, sce_hf_exhaustive_provider(provider));
    machine->answer_schemas(rig.host);
    sce_host_invoker_registry_t registry;
    memset(&registry, 0, sizeof(registry));
    int bad = sce_hf_register(&registry, rig.host) ? 0 : 1;
    void *sm = calloc(1u, machine->size);
    machine->start(sm, &registry);
    machine->settle(sm);
    const jn_t *ask = jn_get(table, "ask");
    const jn_t *data = jn_get(ask, "data");
    char *payload = (char *)must(strndup(data->raw, data->raw_len));
    // The record the last call that read one left, the table's own node; `none` before any.
    jn_t *none = jn_new(J_NULL);
    const jn_t *answer = none;
    for (size_t n = 0; !bad && n < length; ++n) {
        const jn_t *cls = classes->items[sequence[n]];
        if (!machine->raise(sm, jn_get(ask, "event")->s, payload)) {
            bad = fail(machine->name, (int)n, named, "the machine's events do not match the table's", NULL, NULL);
            break;
        }
        machine->settle(sm);
        const jn_t *then = jn_get(cls, "then");
        if (then != NULL && strcmp(then->s, "advance") == 0) {
            machine->advance(sm, (uint64_t)jn_get(table, "deadline_ms")->i);
            machine->settle(sm);
        } else if (then != NULL && strcmp(then->s, "leave") == 0) {
            (void)machine->raise(sm, jn_get(table, "leave")->s, NULL);
            machine->settle(sm);
        }
        const jn_t *expect = jn_get(cls, "expect");
        const jn_t *read = jn_get(expect, "answer");
        if (read != NULL) {
            answer = read;
        }
        const char *atomic = machine->state(sm);
        if (atomic == NULL || strcmp(atomic, "idle") != 0) {
            bad |= fail(machine->name, (int)n, named, "the machine is not idle", NULL, NULL);
        }
        const char *names[] = {"outcome", "why", "finish", "answer", "calls"};
        for (size_t v = 0; v < sizeof(names) / sizeof(names[0]); ++v) {
            const variable_t *row = NULL;
            for (size_t k = 0; k < machine->variable_count; ++k) {
                if (strcmp(machine->variables[k].name, names[v]) == 0) {
                    row = &machine->variables[k];
                }
            }
            if (row == NULL) {
                bad |= fail(machine->name, (int)n, named, "the driver reads no such variable", NULL, NULL);
                continue;
            }
            jn_t *got = row->kind == J_OBJ   ? row->value(sm)
                        : row->kind == J_STR ? jn_str(row->text(sm))
                                             : jn_int(row->number(sm));
            jn_t *calls = jn_int((long long)(n + 1u));
            const jn_t *want = strcmp(names[v], "answer") == 0  ? answer
                               : strcmp(names[v], "calls") == 0 ? calls
                                                                : jn_get(expect, names[v]);
            if (!jn_equal(got, want)) {
                char what[64];
                (void)snprintf(what, sizeof(what), "variable `%s`", names[v]);
                bad |= fail(machine->name, (int)n, named, what, got, want);
            }
            jn_free(calls);
            jn_free(got);
        }
    }
    jn_free(none);
    free(payload);
    free(sm);
    jn_free(rig.transport.log);
    script_clear_response(&rig.transport);
    sce_hf_host_free(rig.host);
    sce_hf_exhaustive_provider_free(provider);
    return bad;
}

// The machine around a model is tested exhaustively (docs/adr/0087, section 9.4): every sequence of outcome
// classes up to the table's bound, each call read as its class says, and every refusal code of the library
// either reached by a class or named as deferred with its reason.
static int every_outcome_class_sequence(void) {
    char path[512];
    (void)snprintf(path, sizeof(path), "%s/../model_outcome_classes.json", SCE_HOST_FEATURES_SCENARIO_DIR);
    char *text = read_file(path);
    if (text == NULL) {
        (void)fprintf(stderr, "host_features: the class table %s is not readable\n", path);
        return 1;
    }
    parser_t ps = {text, text + strlen(text), true};
    jn_t *table = parse_value(&ps);
    int bad = 0;
    const jn_t *classes = table != NULL && ps.ok ? jn_get(table, "classes") : NULL;
    const jn_t *bound = table != NULL && ps.ok ? jn_get(table, "bound") : NULL;
    if (classes == NULL || bound == NULL || classes->count == 0u || bound->i <= 0 || bound->i > 8) {
        (void)fprintf(stderr, "host_features: the class table names nothing to run\n");
        bad = 1;
    } else if (strcmp(jn_get(table, "answer_schema")->s, static_host_model_answer_schemas()[0].id) != 0) {
        (void)fprintf(stderr, "host_features: the class table names another answer schema\n");
        bad = 1;
    }
    size_t runs = 0;
    size_t expected = 0;
    size_t power = 1;
    for (size_t length = 1; !bad && length <= (size_t)bound->i; ++length) {
        power *= classes->count;
        expected += power;
        size_t sequence[8] = {0};
        for (;;) {
            bad |= run_sequence(table, sequence, length);
            ++runs;
            size_t position = length;
            while (position > 0u && sequence[position - 1u] == classes->count - 1u) {
                sequence[--position] = 0u;
            }
            if (position == 0u || bad) {
                break;
            }
            ++sequence[position - 1u];
        }
    }
    if (!bad && runs != expected) {
        (void)fprintf(stderr, "host_features: %zu sequences were run, not the %zu up to the bound\n", runs, expected);
        bad = 1;
    }
    if (!bad) {
        const char *codes[32];
        size_t code_count = 0;
        const jn_t *deferred = jn_get(table, "deferred");
        for (size_t i = 0; i < classes->count + deferred->count && code_count < 32u; ++i) {
            const jn_t *code = NULL;
            if (i < classes->count) {
                const jn_t *why = jn_get(jn_get(classes->items[i], "expect"), "why");
                code = why != NULL && why->kind == J_OBJ ? jn_get(why, "code") : NULL;
            } else {
                code = jn_get(deferred->items[i - classes->count], "code");
            }
            bool seen = code == NULL;
            for (size_t k = 0; !seen && k < code_count; ++k) {
                seen = strcmp(codes[k], code->s) == 0;
            }
            if (!seen) {
                codes[code_count++] = code->s;
            }
        }
        /* The fourteen codes, tooFew the last (docs/adr/0086, section 15.11). */
        if (code_count != 14u) {
            (void)fprintf(stderr, "host_features: %zu refusal codes are reached or deferred, not 14\n", code_count);
            bad = 1;
        }
    }
    jn_free(table);
    free(text);
    return bad;
}

// ── The reference harness, every sequence of outcome classes ─────────────────

#define HARNESS_NAME_MAX 64u
#define HARNESS_KEYS_MAX 64u

// The end of the host operation a class gives a call (docs/adr/0087, section 9.4), written into `out`: an
// answer by its `finish`, a failure by its code, the refusal, and a held call, whichever way it is left.
static void harness_outcome(const jn_t *cls, char *out, size_t cap) {
    const char *end = jn_get(cls, "end")->s;
    if (strcmp(end, "answer") == 0) {
        (void)snprintf(out, cap, "answer:%s", jn_get(cls, "finish")->s);
    } else if (strcmp(end, "failure") == 0) {
        (void)snprintf(out, cap, "%s", jn_get(cls, "error")->s);
    } else if (strcmp(end, "refusal") == 0) {
        (void)snprintf(out, cap, "invalid_argument");
    } else {
        (void)snprintf(out, cap, "hold");
    }
}

// The position of the row `(source, index)` in the harness's emitted transition table, or -1 for a pair that
// is no row of it.
static long harness_row_of(static_model_harness_state_t source, uint32_t index) {
    const static_model_harness_transition_t *rows = static_model_harness_transitions();
    for (size_t r = 0; r < (size_t)STATIC_MODEL_HARNESS_TRANSITION_COUNT; ++r) {
        if (rows[r].from == source && rows[r].index == index) {
            return (long)r;
        }
    }
    return -1;
}

// Whether `key` is one of `keys[0..count)`, adding it when it is not and there is room.
static bool harness_key_seen(char keys[][HARNESS_NAME_MAX], size_t *count, const char *key, bool add) {
    for (size_t i = 0; i < *count; ++i) {
        if (strcmp(keys[i], key) == 0) {
            return true;
        }
    }
    if (add && *count < HARNESS_KEYS_MAX) {
        (void)snprintf(keys[*count], HARNESS_NAME_MAX, "%s", key);
        ++*count;
    }
    return false;
}

// What one run of the harness against the exhaustive provider showed, beside the rows its observer was told
// (`harness_seen_*`).
typedef struct {
    size_t calls;
    char ended[HARNESS_NAME_MAX];
    char codes[8][HARNESS_NAME_MAX];
    size_t code_count;
} harness_run_t;

// Runs the reference harness against the exhaustive provider answering the classes `sequence[0..length)` names,
// one call a class: offers it the table's ids, starts it, and leaves each call it holds as the table's harness
// section says, until it ends. Nonzero when the run cannot be made.
static int harness_run(const jn_t *table, const size_t *sequence, size_t length, harness_run_t *out) {
    const jn_t *classes = jn_get(table, "classes");
    const jn_t *harness = jn_get(table, "harness");
    memset(out, 0, sizeof(*out));
    if (strcmp(jn_get(harness, "then")->s, "advance") != 0) {
        (void)fprintf(stderr, "host_features: a held call of the harness is left by its own deadline\n");
        return 1;
    }
    sce_hf_outcome_class_t provided[8];
    for (size_t i = 0; i < length; ++i) {
        provided[i] = outcome_class(classes->items[sequence[i]]);
    }
    harness_seen_count = 0;
    rig_t rig;
    memset(&rig, 0, sizeof(rig));
    rig.transport.log = jn_new(J_ARR);
    sce_hf_clock_t clock = {scripted_read, scripted_zone, &rig};
    sce_hf_transport_t transport = {scripted_exchange, &rig.transport};
    rig.host = sce_hf_host_new(clock, transport);
    sce_hf_exhaustive_provider_t *provider = sce_hf_exhaustive_provider_new(provided, length);
    sce_hf_set_model_provider(rig.host, sce_hf_exhaustive_provider(provider));
    harness_answer_schemas_to(rig.host);
    sce_host_invoker_registry_t registry;
    memset(&registry, 0, sizeof(registry));
    int bad = sce_hf_register(&registry, rig.host) ? 0 : 1;
    void *sm = calloc(1u, sizeof(static_model_harness_t));
    harness_start(sm, &registry);
    harness_settle(sm);
    const jn_t *offered = jn_get(harness, "offered");
    for (size_t i = 0; !bad && i < offered->count; ++i) {
        char data[64];
        (void)snprintf(data, sizeof(data), "{\"id\":%lld}", offered->items[i]->i);
        bad |= harness_raise(sm, jn_get(harness, "offer")->s, data) ? 0 : 1;
        harness_settle(sm);
    }
    const jn_t *data = jn_get(harness, "data");
    char *payload = (char *)must(strndup(data->raw, data->raw_len));
    bad |= harness_raise(sm, jn_get(harness, "start")->s, payload) ? 0 : 1;
    free(payload);
    harness_settle(sm);
    const long long bound = jn_get(harness, "bound")->i;
    for (long long waits = 0; !bad; ++waits) {
        const char *leaf = harness_leaf(sm);
        if (leaf != NULL && (strcmp(leaf, "accepted") == 0 || strcmp(leaf, "gave_up") == 0)) {
            break;
        }
        if (waits > bound) {
            (void)fprintf(stderr, "host_features: the harness neither ended nor held a call it could be moved past\n");
            bad = 1;
            break;
        }
        harness_advance(sm, (uint64_t)jn_get(harness, "deadline_ms")->i);
        harness_settle(sm);
    }
    out->calls = sce_hf_start_count(rig.host);
    const char *leaf = harness_leaf(sm);
    (void)snprintf(out->ended, sizeof(out->ended), "%s", leaf != NULL ? leaf : "(none)");
    jn_t *refusals = harness_saved_variable(sm, "refusals");
    for (size_t i = 0; i < refusals->count && out->code_count < 8u; ++i) {
        const jn_t *code = jn_get(refusals->items[i], "code");
        (void)snprintf(out->codes[out->code_count++], HARNESS_NAME_MAX, "%s", code != NULL ? code->s : "(none)");
    }
    jn_free(refusals);
    free(sm);
    jn_free(rig.transport.log);
    script_clear_response(&rig.transport);
    sce_hf_host_free(rig.host);
    sce_hf_exhaustive_provider_free(provider);
    return bad;
}

// The reference harness is tested exhaustively, and its coverage is measured (docs/adr/0087, section 9.4 item
// 1, Landing order item 7): every sequence of `bound` outcome classes is a run, the transition observer of
// docs/adr/0089 tells each row of the emitted transition table the run takes, and the runs together take every
// row of the table, reach every refusal code, and reach every host outcome at every call site (a row whose
// targets are the state whose invoke is the call). Nonzero, naming what was never reached, when they do not.
//
// The harness makes at most `bound` calls and the provider answers a call by its position alone, so every
// sequence of fewer classes the harness can consume is a prefix of a sequence run here, and none is asked past.
static int every_harness_sequence(void) {
    char path[512];
    (void)snprintf(path, sizeof(path), "%s/../model_outcome_classes.json", SCE_HOST_FEATURES_SCENARIO_DIR);
    char *text = read_file(path);
    if (text == NULL) {
        (void)fprintf(stderr, "host_features: the class table %s is not readable\n", path);
        return 1;
    }
    parser_t ps = {text, text + strlen(text), true};
    jn_t *table = parse_value(&ps);
    const jn_t *classes = table != NULL && ps.ok ? jn_get(table, "classes") : NULL;
    const jn_t *harness = table != NULL && ps.ok ? jn_get(table, "harness") : NULL;
    const jn_t *bound_node = harness != NULL ? jn_get(harness, "bound") : NULL;
    int bad = 0;
    if (classes == NULL || bound_node == NULL || classes->count == 0u || bound_node->i <= 0 || bound_node->i > 8 ||
        classes->count > HARNESS_KEYS_MAX) {
        (void)fprintf(stderr, "host_features: the class table names no harness run\n");
        jn_free(table);
        free(text);
        return 1;
    }
    if (strcmp(jn_get(harness, "machine")->s, "static_model_harness") != 0 ||
        strcmp(jn_get(table, "answer_schema")->s, static_model_harness_answer_schemas()[0].id) != 0) {
        (void)fprintf(stderr, "host_features: the class table names another harness or answer schema\n");
        bad = 1;
    }
    const size_t bound = (size_t)bound_node->i;
    const char *call = jn_get(harness, "call")->s;
    const static_model_harness_transition_t *rows = static_model_harness_transitions();
    const size_t row_count = (size_t)STATIC_MODEL_HARNESS_TRANSITION_COUNT;
    bool site[HARNESS_KEYS_MAX] = {false};
    bool taken[HARNESS_KEYS_MAX] = {false};
    size_t site_count = 0;
    for (size_t r = 0; r < row_count && r < HARNESS_KEYS_MAX; ++r) {
        site[r] = strcmp(rows[r].targets, call) == 0;
        site_count += site[r] ? 1u : 0u;
    }
    if (site_count == 0u || row_count > HARNESS_KEYS_MAX) {
        (void)fprintf(stderr, "host_features: no row of the harness's table enters `%s`\n", call);
        bad = 1;
    }
    char outcomes[HARNESS_KEYS_MAX][HARNESS_NAME_MAX];
    size_t outcome_count = 0;
    for (size_t i = 0; i < classes->count; ++i) {
        char outcome[HARNESS_NAME_MAX];
        harness_outcome(classes->items[i], outcome, sizeof(outcome));
        (void)harness_key_seen(outcomes, &outcome_count, outcome, true);
    }
    // reached[r][o]: the outcome `o` was given a call made through the row `r`.
    static bool reached[HARNESS_KEYS_MAX][HARNESS_KEYS_MAX];
    memset(reached, 0, sizeof(reached));
    char codes[HARNESS_KEYS_MAX][HARNESS_NAME_MAX];
    size_t code_count = 0;
    bool told_no_row = false;

    size_t runs = 0;
    size_t expected = 1;
    size_t sequence[8] = {0};
    for (size_t i = 0; i < bound; ++i) {
        expected *= classes->count;
    }
    while (!bad) {
        harness_run_t run;
        bad |= harness_run(table, sequence, bound, &run);
        if (bad) {
            break;
        }
        if (strcmp(run.ended, "accepted") != 0 && strcmp(run.ended, "gave_up") != 0) {
            (void)fprintf(stderr, "host_features: a harness run ended in `%s`, not a final\n", run.ended);
            bad = 1;
            break;
        }
        size_t called = 0;
        const size_t told = harness_seen_count < HARNESS_SEEN_MAX ? harness_seen_count : HARNESS_SEEN_MAX;
        for (size_t i = 0; i < told; ++i) {
            const long r = harness_row_of(harness_seen_source[i], harness_seen_index[i]);
            if (r < 0) {
                told_no_row = true;
                continue;
            }
            taken[r] = true;
            if (site[r]) {
                if (called < bound) {
                    char outcome[HARNESS_NAME_MAX];
                    harness_outcome(classes->items[sequence[called]], outcome, sizeof(outcome));
                    for (size_t o = 0; o < outcome_count; ++o) {
                        if (strcmp(outcomes[o], outcome) == 0) {
                            reached[r][o] = true;
                        }
                    }
                }
                ++called;
            }
        }
        if (called != run.calls || run.calls > bound) {
            (void)fprintf(stderr,
                          "host_features: a harness run made %zu calls, %zu of them by a row that enters `%s`\n",
                          run.calls, called, call);
            bad = 1;
            break;
        }
        for (size_t i = 0; i < run.code_count; ++i) {
            (void)harness_key_seen(codes, &code_count, run.codes[i], true);
        }
        ++runs;
        size_t position = bound;
        while (position > 0u && sequence[position - 1u] == classes->count - 1u) {
            sequence[--position] = 0u;
        }
        if (position == 0u) {
            break;
        }
        ++sequence[position - 1u];
    }
    if (!bad && runs != expected) {
        (void)fprintf(stderr, "host_features: %zu harness sequences were run, not the %zu of the bound\n", runs,
                      expected);
        bad = 1;
    }
    // Every gap is named, not only the first: a run that could not be made judges none of them.
    const int fatal = bad;
    if (!fatal && told_no_row) {
        (void)fprintf(stderr, "host_features: the observer was told a pair that is no row of the harness's table\n");
        bad = 1;
    }
    for (size_t r = 0; !fatal && r < row_count; ++r) {
        if (!taken[r]) {
            (void)fprintf(stderr, "host_features: row %s[%u] of the harness's transition table was never taken\n",
                          static_model_harness_state_name(rows[r].from), (unsigned)rows[r].index);
            bad = 1;
        }
    }
    if (!fatal) {
        // The fourteen lift codes, reached by a class or deferred, and the application's own.
        char expected_codes[HARNESS_KEYS_MAX][HARNESS_NAME_MAX];
        size_t expected_count = 0;
        char deferred[HARNESS_KEYS_MAX][HARNESS_NAME_MAX];
        size_t deferred_count = 0;
        (void)harness_key_seen(expected_codes, &expected_count, "notOffered", true);
        const jn_t *deferred_node = jn_get(table, "deferred");
        for (size_t i = 0; i < deferred_node->count; ++i) {
            const char *code = jn_get(deferred_node->items[i], "code")->s;
            (void)harness_key_seen(expected_codes, &expected_count, code, true);
            (void)harness_key_seen(deferred, &deferred_count, code, true);
        }
        for (size_t i = 0; i < classes->count; ++i) {
            const jn_t *why = jn_get(jn_get(classes->items[i], "expect"), "why");
            if (why != NULL && why->kind == J_OBJ) {
                (void)harness_key_seen(expected_codes, &expected_count, jn_get(why, "code")->s, true);
            }
        }
        if (expected_count != 15u) {
            (void)fprintf(stderr, "host_features: %zu refusal codes, not the fourteen lift codes and notOffered\n",
                          expected_count);
            bad = 1;
        }
        for (size_t i = 0; i < expected_count; ++i) {
            if (!harness_key_seen(codes, &code_count, expected_codes[i], false) &&
                !harness_key_seen(deferred, &deferred_count, expected_codes[i], false)) {
                (void)fprintf(stderr, "host_features: the refusal code %s was never reached by the harness\n",
                              expected_codes[i]);
                bad = 1;
            }
        }
        for (size_t i = 0; i < code_count; ++i) {
            if (!harness_key_seen(expected_codes, &expected_count, codes[i], false)) {
                (void)fprintf(stderr, "host_features: the harness kept the code %s, which no class reaches\n",
                              codes[i]);
                bad = 1;
            }
        }
    }
    for (size_t r = 0; !fatal && r < row_count; ++r) {
        for (size_t o = 0; site[r] && o < outcome_count; ++o) {
            if (!reached[r][o]) {
                (void)fprintf(stderr, "host_features: the host outcome %s was never reached at the call site %s[%u]\n",
                              outcomes[o], static_model_harness_state_name(rows[r].from), (unsigned)rows[r].index);
                bad = 1;
            }
        }
    }
    jn_free(table);
    free(text);
    return bad;
}

// What the host makes of a generation apart from the scenario: each argument it does not accept is refused
// before the provider is asked, an answer past its bound or a request past the context fails whole, and the
// exhaustive provider answers each class in turn.
static bool judges_a_generation_before_its_provider(void) {
    bool ok = true;
    script_t script;
    memset(&script, 0, sizeof(script));
    script.log = jn_new(J_ARR);
    sce_hf_transport_t transport = {scripted_exchange, &script};
    sce_hf_host_t *host = sce_hf_host_new(sce_hf_system_clock(), transport);
    static const sce_answer_schema_t schemas[] = {{"urn:sce:schema:answer:1", "{}", "root ::= \"{}\""}};
    sce_hf_set_answer_schemas(host, schemas, 1u);
    sce_hf_scripted_provider_t *scripted = sce_hf_scripted_provider_new();
    sce_hf_scripted_set_constrained(scripted, true);
    sce_hf_scripted_set_context(scripted, 10u);
    char *long_text = (char *)must(malloc(SCE_HF_ANSWER_MAX_BYTES + 2u));
    memset(long_text, 'a', SCE_HF_ANSWER_MAX_BYTES + 1u);
    long_text[SCE_HF_ANSWER_MAX_BYTES + 1u] = '\0';
    sce_hf_scripted_push_answer(scripted, long_text, SCE_HF_ANSWER_MAX_BYTES + 1u, "stop");
    sce_hf_scripted_push_answer(scripted, "kept", 4u, "stop");
    free(long_text);
    sce_hf_set_model_provider(host, sce_hf_scripted_provider(scripted));

    struct {
        const char *prompt, *schema, *max_tokens, *temperature, *expect;
    } const cases[] = {
        {"", "", "8", "0", NULL},
        {"p", "", "0", "0", NULL},
        {"p", "", "8", "2001", NULL},
        {"p", "urn:sce:schema:other:1", "8", "0", NULL},
        {"p", "", "8", "0", "\"error\":\"answer_too_large\""},
        {"abc", "", "8", "0", "\"error\":\"context_exceeded\""},
        {"abc", "urn:sce:schema:answer:1", "7", "2000", "\"text\":\"kept\""},
        {"p", "", "8", "0", "\"error\":\"unavailable\""},
    };

    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        const sce_host_send_param_t params[4] = {{"prompt", cases[i].prompt},
                                                 {"answer_schema", cases[i].schema},
                                                 {"max_tokens", cases[i].max_tokens},
                                                 {"temperature_milli", cases[i].temperature}};
        sce_hf_outcome_t outcome;
        (void)sce_hf_start(host, SCE_HF_MODEL_GENERATE, "gen", params, 4u, &outcome);
        const bool held = cases[i].expect == NULL
                              ? outcome.kind == SCE_HF_REFUSE
                              : outcome.kind == SCE_HF_DONE && strstr(outcome.data, cases[i].expect) != NULL;
        if (!held) {
            (void)fprintf(stderr, "host_features: generation case %zu answered %s\n", i,
                          outcome.data != NULL ? outcome.data : "(nothing)");
            ok = false;
        }
        sce_hf_outcome_free(&outcome);
    }
    const sce_hf_outcome_class_t sequence[] = {
        {SCE_HF_GENERATION_ANSWER, "t", "length", ""},
        {SCE_HF_GENERATION_FAILURE, "", "", "busy"},
        {SCE_HF_GENERATION_REFUSAL, "", "", ""},
        {SCE_HF_GENERATION_HOLD, "", "", ""},
    };
    sce_hf_exhaustive_provider_t *exhaustive = sce_hf_exhaustive_provider_new(sequence, 4u);
    sce_hf_set_model_provider(host, sce_hf_exhaustive_provider(exhaustive));
    const sce_hf_kind_t kinds[] = {SCE_HF_DONE, SCE_HF_DONE, SCE_HF_REFUSE, SCE_HF_HOLD, SCE_HF_DONE};
    const char *contains[] = {"\"model\":\"exhaustive\"", "\"error\":\"busy\"", NULL, NULL,
                              "\"error\":\"unavailable\""};
    for (size_t i = 0; i < 5u; ++i) {
        const sce_host_send_param_t params[4] = {
            {"prompt", "p"}, {"answer_schema", ""}, {"max_tokens", "8"}, {"temperature_milli", "0"}};
        sce_hf_outcome_t outcome;
        (void)sce_hf_start(host, SCE_HF_MODEL_GENERATE, "gen", params, 4u, &outcome);
        if (outcome.kind != kinds[i] || (contains[i] != NULL && strstr(outcome.data, contains[i]) == NULL)) {
            (void)fprintf(stderr, "host_features: exhaustive class %zu answered %s\n", i,
                          outcome.data != NULL ? outcome.data : "(nothing)");
            ok = false;
        }
        sce_hf_outcome_free(&outcome);
    }
    sce_hf_host_free(host);
    sce_hf_scripted_provider_free(scripted);
    sce_hf_exhaustive_provider_free(exhaustive);
    jn_free(script.log);
    return ok;
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
    if (!judges_a_generation_before_its_provider()) {
        bad = 1;
    }
    for (size_t i = 0; i < sizeof(machines) / sizeof(machines[0]); ++i) {
        const int one = replay(&machines[i], machines[i].name);
        (void)printf("host_features: %s %s\n", machines[i].name, one == 0 ? "ok" : "FAILED");
        bad |= one;
    }
    for (size_t i = 0; i < sizeof(harness_scenarios) / sizeof(harness_scenarios[0]); ++i) {
        const int one = replay(&harness_machine, harness_scenarios[i]);
        (void)printf("host_features: %s %s\n", harness_scenarios[i], one == 0 ? "ok" : "FAILED");
        bad |= one;
    }
    const int sequences = every_outcome_class_sequence();
    (void)printf("host_features: every outcome class sequence %s\n", sequences == 0 ? "ok" : "FAILED");
    bad |= sequences;
    const int harness_sequences = every_harness_sequence();
    (void)printf("host_features: every outcome class sequence of the reference harness %s\n",
                 harness_sequences == 0 ? "ok" : "FAILED");
    bad |= harness_sequences;
    return bad;
}
