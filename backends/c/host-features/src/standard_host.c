/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif
#ifndef _DEFAULT_SOURCE
#define _DEFAULT_SOURCE 1
#endif

#include "sce/standard_host.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

const char *const sce_hf_types[SCE_HF_TYPE_COUNT] = {
    "sce-host-clock-now",
    "sce-host-clock-zone",
    "sce-host-kv-get",
    "sce-host-kv-put",
    "sce-host-kv-delete",
    "sce-host-kv-next-key",
    "sce-host-notify-permission",
    "sce-host-notify-request-permission",
    "sce-host-notify-post",
    "sce-host-notify-cancel",
    "sce-host-http-fetch",
};

/* The most bytes of response headers and of body a result record holds. */
#define HEADERS_BOUND 4096u
#define BODY_BOUND 16384u

/* ── Small owned containers ─────────────────────────────────────────── */

static void *must(void *p) {
    if (p == NULL) {
        abort();
    }
    return p;
}

static char *dup_n(const char *s, size_t n) {
    char *copy = (char *)must(malloc(n + 1u));
    memcpy(copy, s, n);
    copy[n] = '\0';
    return copy;
}

static char *dup_str(const char *s) {
    return dup_n(s, strlen(s));
}

typedef struct {
    char *p;
    size_t len;
    size_t cap;
} sb_t;

static void sb_reserve(sb_t *sb, size_t more) {
    if (sb->len + more + 1u > sb->cap) {
        size_t cap = sb->cap ? sb->cap : 64u;
        while (cap < sb->len + more + 1u) {
            cap *= 2u;
        }
        sb->p = (char *)must(realloc(sb->p, cap));
        sb->cap = cap;
    }
}

static void sb_putn(sb_t *sb, const char *s, size_t n) {
    sb_reserve(sb, n);
    memcpy(sb->p + sb->len, s, n);
    sb->len += n;
    sb->p[sb->len] = '\0';
}

static void sb_puts(sb_t *sb, const char *s) {
    sb_putn(sb, s, strlen(s));
}

/* A JSON string: quotes, backslashes and control characters escaped, the rest as it is. */
static void sb_json_string(sb_t *sb, const char *s, size_t n) {
    static const char hex[] = "0123456789abcdef";
    sb_puts(sb, "\"");
    for (size_t i = 0; i < n; ++i) {
        const unsigned char c = (unsigned char)s[i];
        if (c == '"') {
            sb_puts(sb, "\\\"");
        } else if (c == '\\') {
            sb_puts(sb, "\\\\");
        } else if (c == '\n') {
            sb_puts(sb, "\\n");
        } else if (c == '\r') {
            sb_puts(sb, "\\r");
        } else if (c == '\t') {
            sb_puts(sb, "\\t");
        } else if (c < 0x20u) {
            char esc[7] = {'\\', 'u', '0', '0', hex[c >> 4], hex[c & 0xfu], '\0'};
            sb_puts(sb, esc);
        } else {
            sb_putn(sb, (const char *)&c, 1u);
        }
    }
    sb_puts(sb, "\"");
}

typedef struct {
    char **items;
    size_t count;
    size_t cap;
} strlist_t;

static void strlist_push(strlist_t *list, const char *s) {
    if (list->count == list->cap) {
        list->cap = list->cap ? list->cap * 2u : 8u;
        list->items = (char **)must(realloc(list->items, list->cap * sizeof(char *)));
    }
    list->items[list->count++] = dup_str(s);
}

/* Remove every entry equal to `s`; whether any was there. */
static bool strlist_remove(strlist_t *list, const char *s) {
    bool removed = false;
    size_t kept = 0;
    for (size_t i = 0; i < list->count; ++i) {
        if (strcmp(list->items[i], s) == 0) {
            free(list->items[i]);
            removed = true;
        } else {
            list->items[kept++] = list->items[i];
        }
    }
    list->count = kept;
    return removed;
}

static void strlist_free(strlist_t *list) {
    for (size_t i = 0; i < list->count; ++i) {
        free(list->items[i]);
    }
    free(list->items);
    memset(list, 0, sizeof(*list));
}

typedef struct {
    char *key;
    char *value;
} entry_t;

typedef struct {
    char *operation;
    char *id;
    sce_host_send_param_t *params;
    size_t param_count;
} start_t;

typedef struct {
    char *operation;
    char *code;
} fault_t;

struct sce_hf_host_s {
    sce_hf_clock_t clock;
    sce_hf_transport_t transport;
    /* The store, in ascending order of the keys' bytes. */
    entry_t *store;
    size_t store_count;
    size_t store_cap;
    bool has_quota;
    size_t quota;
    char *permission;
    char *user_decision;
    strlist_t waiting;
    strlist_t shown;
    fault_t *faults;
    size_t fault_count;
    strlist_t held;
    strlist_t cancelled;
    bool monotonic_set;
    int64_t monotonic_floor;
    start_t *starts;
    size_t start_count;
    size_t start_cap;
};

sce_hf_host_t *sce_hf_host_new(sce_hf_clock_t clock, sce_hf_transport_t transport) {
    sce_hf_host_t *host = (sce_hf_host_t *)must(calloc(1u, sizeof(*host)));
    host->clock = clock;
    host->transport = transport;
    host->permission = dup_str("undetermined");
    host->user_decision = dup_str("none");
    return host;
}

static void clear_faults(sce_hf_host_t *host) {
    for (size_t i = 0; i < host->fault_count; ++i) {
        free(host->faults[i].operation);
        free(host->faults[i].code);
    }
    free(host->faults);
    host->faults = NULL;
    host->fault_count = 0;
}

void sce_hf_clear_starts(sce_hf_host_t *host) {
    for (size_t i = 0; i < host->start_count; ++i) {
        start_t *start = &host->starts[i];
        free(start->operation);
        free(start->id);
        for (size_t p = 0; p < start->param_count; ++p) {
            free((char *)start->params[p].name);
            free((char *)start->params[p].value);
        }
        free(start->params);
    }
    free(host->starts);
    host->starts = NULL;
    host->start_count = 0;
    host->start_cap = 0;
}

void sce_hf_host_free(sce_hf_host_t *host) {
    if (host == NULL) {
        return;
    }
    for (size_t i = 0; i < host->store_count; ++i) {
        free(host->store[i].key);
        free(host->store[i].value);
    }
    free(host->store);
    free(host->permission);
    free(host->user_decision);
    strlist_free(&host->waiting);
    strlist_free(&host->shown);
    clear_faults(host);
    strlist_free(&host->held);
    strlist_free(&host->cancelled);
    sce_hf_clear_starts(host);
    free(host);
}

void sce_hf_outcome_free(sce_hf_outcome_t *outcome) {
    free(outcome->data);
    outcome->data = NULL;
}

void sce_hf_http_response_free(sce_hf_http_response_t *response) {
    for (size_t i = 0; i < response->header_count; ++i) {
        free(response->headers[i].name);
        free(response->headers[i].value);
    }
    free(response->headers);
    free(response->body);
    memset(response, 0, sizeof(*response));
}

/* ── Configuration and what a scenario reads back ───────────────────── */

void sce_hf_set_fault(sce_hf_host_t *host, const char *operation, const char *code) {
    host->faults = (fault_t *)must(realloc(host->faults, (host->fault_count + 1u) * sizeof(fault_t)));
    host->faults[host->fault_count].operation = dup_str(operation);
    host->faults[host->fault_count].code = dup_str(code);
    host->fault_count++;
}

void sce_hf_clear_faults(sce_hf_host_t *host) {
    clear_faults(host);
}

void sce_hf_set_quota(sce_hf_host_t *host, size_t keys) {
    host->has_quota = true;
    host->quota = keys;
}

void sce_hf_set_permission(sce_hf_host_t *host, const char *state) {
    free(host->permission);
    host->permission = dup_str(state);
}

void sce_hf_set_user_decision(sce_hf_host_t *host, const char *decision) {
    free(host->user_decision);
    host->user_decision = dup_str(decision);
}

size_t sce_hf_store_count(const sce_hf_host_t *host) {
    return host->store_count;
}

void sce_hf_store_entry(const sce_hf_host_t *host, size_t index, const char **key, const char **value) {
    *key = host->store[index].key;
    *value = host->store[index].value;
}

size_t sce_hf_waiting_count(const sce_hf_host_t *host) {
    return host->waiting.count;
}

const char *sce_hf_waiting_id(const sce_hf_host_t *host, size_t index) {
    return host->waiting.items[index];
}

size_t sce_hf_shown_count(const sce_hf_host_t *host) {
    return host->shown.count;
}

const char *sce_hf_shown_id(const sce_hf_host_t *host, size_t index) {
    return host->shown.items[index];
}

size_t sce_hf_cancelled_count(const sce_hf_host_t *host) {
    return host->cancelled.count;
}

const char *sce_hf_cancelled_id(const sce_hf_host_t *host, size_t index) {
    return host->cancelled.items[index];
}

size_t sce_hf_start_count(const sce_hf_host_t *host) {
    return host->start_count;
}

const char *sce_hf_start_operation(const sce_hf_host_t *host, size_t index) {
    return host->starts[index].operation;
}

const char *sce_hf_start_id(const sce_hf_host_t *host, size_t index) {
    return host->starts[index].id;
}

size_t sce_hf_start_param_count(const sce_hf_host_t *host, size_t index) {
    return host->starts[index].param_count;
}

void sce_hf_start_param(const sce_hf_host_t *host, size_t index, size_t param, const char **name, const char **value) {
    *name = host->starts[index].params[param].name;
    *value = host->starts[index].params[param].value;
}

/* ── The system clock ───────────────────────────────────────────────── */

static int64_t timespec_ms(const struct timespec *t) {
    return (int64_t)t->tv_sec * 1000 + (int64_t)(t->tv_nsec / 1000000L);
}

static int64_t system_origin_ms(void) {
    static int64_t origin;
    static bool set;
    if (!set) {
        struct timespec t;
        (void)clock_gettime(CLOCK_MONOTONIC, &t);
        origin = timespec_ms(&t);
        set = true;
    }
    return origin;
}

static bool system_read(void *user_data, sce_hf_reading_t *out) {
    (void)user_data;
    struct timespec utc;
    struct timespec mono;
    if (clock_gettime(CLOCK_REALTIME, &utc) != 0 || clock_gettime(CLOCK_MONOTONIC, &mono) != 0) {
        return false;
    }
    out->utc_ms = timespec_ms(&utc);
    out->monotonic_ms = timespec_ms(&mono) - system_origin_ms();
    return true;
}

/* The offset in force now, and the identifier the TZ variable names when it
   names a zone, which the C library has no other way to report. */
static bool system_zone(void *user_data, sce_hf_zone_t *out) {
    (void)user_data;
    const time_t now = time(NULL);
    struct tm local;
    if (localtime_r(&now, &local) == NULL) {
        return false;
    }
    out->id[0] = '\0';
    const char *tz = getenv("TZ");
    if (tz != NULL) {
        if (tz[0] == ':') {
            ++tz;
        }
        if ((strchr(tz, '/') != NULL || strcmp(tz, "UTC") == 0) && strlen(tz) < sizeof(out->id)) {
            (void)snprintf(out->id, sizeof(out->id), "%s", tz);
        }
    }
    out->utc_offset_s = (int32_t)local.tm_gmtoff;
    return true;
}

sce_hf_clock_t sce_hf_system_clock(void) {
    (void)system_origin_ms();
    sce_hf_clock_t clock = {system_read, system_zone, NULL};
    return clock;
}

/* ── Result records ─────────────────────────────────────────────────── */

typedef struct {
    sb_t sb;
} record_t;

/* `{"error":<error>` — more fields follow, then `record_end`. */
static void record_begin(record_t *r, const char *error) {
    memset(r, 0, sizeof(*r));
    sb_puts(&r->sb, "{\"error\":");
    sb_json_string(&r->sb, error, strlen(error));
}

static void record_name(record_t *r, const char *name) {
    sb_puts(&r->sb, ",");
    sb_json_string(&r->sb, name, strlen(name));
    sb_puts(&r->sb, ":");
}

static void record_text(record_t *r, const char *name, const char *value, size_t n) {
    record_name(r, name);
    sb_json_string(&r->sb, value, n);
}

static void record_bool(record_t *r, const char *name, bool value) {
    record_name(r, name);
    sb_puts(&r->sb, value ? "true" : "false");
}

static void record_int(record_t *r, const char *name, long long value) {
    char digits[32];
    (void)snprintf(digits, sizeof(digits), "%lld", value);
    record_name(r, name);
    sb_puts(&r->sb, digits);
}

static void record_end(record_t *r, sce_hf_outcome_t *out) {
    sb_puts(&r->sb, "}");
    out->kind = SCE_HF_DONE;
    out->data = r->sb.p;
}

/* The failure `code` of `operation`: its result record with the fields at
   their zero and `error` the code, so a machine reads the failure as a typed
   field. */
static void failure(const char *operation, const char *code, sce_hf_outcome_t *out) {
    record_t r;
    record_begin(&r, code);
    if (strcmp(operation, "sce-host-clock-now") == 0) {
        record_text(&r, "utc_ms", "0", 1u);
        record_text(&r, "monotonic_ms", "0", 1u);
    } else if (strcmp(operation, "sce-host-clock-zone") == 0) {
        record_text(&r, "zone_id", "", 0u);
        record_int(&r, "utc_offset_s", 0);
    } else if (strcmp(operation, "sce-host-kv-get") == 0) {
        record_bool(&r, "found", false);
        record_text(&r, "value", "", 0u);
    } else if (strcmp(operation, "sce-host-kv-put") == 0) {
        record_bool(&r, "replaced", false);
    } else if (strcmp(operation, "sce-host-kv-delete") == 0 || strcmp(operation, "sce-host-notify-cancel") == 0) {
        record_bool(&r, "existed", false);
    } else if (strcmp(operation, "sce-host-kv-next-key") == 0) {
        record_bool(&r, "found", false);
        record_text(&r, "key", "", 0u);
    } else if (strcmp(operation, "sce-host-notify-permission") == 0 ||
               strcmp(operation, "sce-host-notify-request-permission") == 0) {
        record_text(&r, "state", "", 0u);
    } else if (strcmp(operation, "sce-host-notify-post") == 0) {
        record_text(&r, "status", "", 0u);
    } else {
        record_int(&r, "status", 0);
        record_text(&r, "headers", "", 0u);
        record_text(&r, "body", "", 0u);
    }
    record_end(&r, out);
}

/* ── Clock ──────────────────────────────────────────────────────────── */

static void clock_now(sce_hf_host_t *host, sce_hf_outcome_t *out) {
    sce_hf_reading_t reading;
    if (!host->clock.read(host->clock.user_data, &reading)) {
        failure("sce-host-clock-now", "unavailable", out);
        return;
    }
    /* No answer goes below one already given. */
    if (!host->monotonic_set || reading.monotonic_ms > host->monotonic_floor) {
        host->monotonic_floor = reading.monotonic_ms;
        host->monotonic_set = true;
    }
    char utc[32];
    char mono[32];
    (void)snprintf(utc, sizeof(utc), "%" PRId64, reading.utc_ms);
    (void)snprintf(mono, sizeof(mono), "%" PRId64, host->monotonic_floor);
    record_t r;
    record_begin(&r, "");
    record_text(&r, "utc_ms", utc, strlen(utc));
    record_text(&r, "monotonic_ms", mono, strlen(mono));
    record_end(&r, out);
}

static void clock_zone(sce_hf_host_t *host, sce_hf_outcome_t *out) {
    sce_hf_zone_t zone;
    memset(&zone, 0, sizeof(zone));
    if (!host->clock.zone(host->clock.user_data, &zone)) {
        failure("sce-host-clock-zone", "unavailable", out);
        return;
    }
    record_t r;
    record_begin(&r, "");
    record_text(&r, "zone_id", zone.id, strlen(zone.id));
    record_int(&r, "utc_offset_s", zone.utc_offset_s);
    record_end(&r, out);
}

/* ── Key-value store ────────────────────────────────────────────────── */

/* strcmp compares bytes as unsigned characters, which for UTF-8 is the order
   the specification gives. */
static size_t store_find(const sce_hf_host_t *host, const char *key, bool *found) {
    size_t at = 0;
    while (at < host->store_count && strcmp(host->store[at].key, key) < 0) {
        ++at;
    }
    *found = at < host->store_count && strcmp(host->store[at].key, key) == 0;
    return at;
}

static void kv_get(sce_hf_host_t *host, const char *key, sce_hf_outcome_t *out) {
    bool found;
    const size_t at = store_find(host, key, &found);
    record_t r;
    record_begin(&r, "");
    record_bool(&r, "found", found);
    record_text(&r, "value", found ? host->store[at].value : "", found ? strlen(host->store[at].value) : 0u);
    record_end(&r, out);
}

static void kv_put(sce_hf_host_t *host, const char *key, const char *value, sce_hf_outcome_t *out) {
    bool replaced;
    const size_t at = store_find(host, key, &replaced);
    if (!replaced && host->has_quota && host->store_count >= host->quota) {
        failure("sce-host-kv-put", "quota_exceeded", out);
        return;
    }
    if (replaced) {
        free(host->store[at].value);
        host->store[at].value = dup_str(value);
    } else {
        if (host->store_count == host->store_cap) {
            host->store_cap = host->store_cap ? host->store_cap * 2u : 16u;
            host->store = (entry_t *)must(realloc(host->store, host->store_cap * sizeof(entry_t)));
        }
        memmove(&host->store[at + 1u], &host->store[at], (host->store_count - at) * sizeof(entry_t));
        host->store[at].key = dup_str(key);
        host->store[at].value = dup_str(value);
        host->store_count++;
    }
    record_t r;
    record_begin(&r, "");
    record_bool(&r, "replaced", replaced);
    record_end(&r, out);
}

static void kv_delete(sce_hf_host_t *host, const char *key, sce_hf_outcome_t *out) {
    bool existed;
    const size_t at = store_find(host, key, &existed);
    if (existed) {
        free(host->store[at].key);
        free(host->store[at].value);
        memmove(&host->store[at], &host->store[at + 1u], (host->store_count - at - 1u) * sizeof(entry_t));
        host->store_count--;
    }
    record_t r;
    record_begin(&r, "");
    record_bool(&r, "existed", existed);
    record_end(&r, out);
}

/* The first key, in ascending order of UTF-8 bytes, that is greater than
   `after` and begins with `prefix`; `after` is excluded, and an empty `after`
   starts at the least key. */
static void kv_next_key(sce_hf_host_t *host, const char *after, const char *prefix, sce_hf_outcome_t *out) {
    const size_t prefix_len = strlen(prefix);
    record_t r;
    record_begin(&r, "");
    for (size_t i = 0; i < host->store_count; ++i) {
        const char *key = host->store[i].key;
        if ((after[0] == '\0' || strcmp(key, after) > 0) && strncmp(key, prefix, prefix_len) == 0) {
            record_bool(&r, "found", true);
            record_text(&r, "key", key, strlen(key));
            record_end(&r, out);
            return;
        }
    }
    record_bool(&r, "found", false);
    record_text(&r, "key", "", 0u);
    record_end(&r, out);
}

/* ── Notifications ──────────────────────────────────────────────────── */

static void notify_state(sce_hf_host_t *host, sce_hf_outcome_t *out) {
    record_t r;
    record_begin(&r, "");
    record_text(&r, "state", host->permission, strlen(host->permission));
    record_end(&r, out);
}

/* Remove what `id` names from both lists; whether it named anything. */
static bool notify_withdraw(sce_hf_host_t *host, const char *id) {
    const bool waiting = strlist_remove(&host->waiting, id);
    const bool shown = strlist_remove(&host->shown, id);
    return waiting || shown;
}

static void notify_request_permission(sce_hf_host_t *host, sce_hf_outcome_t *out) {
    if (strcmp(host->permission, "undetermined") == 0) {
        /* A host with no user to ask answers denied. */
        sce_hf_set_permission(host, strcmp(host->user_decision, "granted") == 0 ? "granted" : "denied");
    }
    notify_state(host, out);
}

static void notify_post(sce_hf_host_t *host, const char *id, const char *at_ms, sce_hf_outcome_t *out) {
    record_t r;
    if (strcmp(host->permission, "granted") != 0) {
        record_begin(&r, "");
        record_text(&r, "status", "denied", 6u);
        record_end(&r, out);
        return;
    }
    sce_hf_reading_t reading;
    if (!host->clock.read(host->clock.user_data, &reading)) {
        failure("sce-host-notify-post", "unavailable", out);
        return;
    }
    char *end = NULL;
    const long long at = strtoll(at_ms, &end, 10);
    notify_withdraw(host, id);
    record_begin(&r, "");
    if (end != at_ms && at > (long long)reading.utc_ms) {
        strlist_push(&host->waiting, id);
        record_text(&r, "status", "scheduled", 9u);
    } else {
        strlist_push(&host->shown, id);
        record_text(&r, "status", "posted", 6u);
    }
    record_end(&r, out);
}

static void notify_cancel(sce_hf_host_t *host, const char *id, sce_hf_outcome_t *out) {
    record_t r;
    record_begin(&r, "");
    record_bool(&r, "existed", notify_withdraw(host, id));
    record_end(&r, out);
}

/* ── HTTP ───────────────────────────────────────────────────────────── */

static const char *const methods[] = {"GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"};

/* An RFC 9110 token: what a header name is made of. */
static bool is_token(const char *s, size_t n) {
    if (n == 0u) {
        return false;
    }
    for (size_t i = 0; i < n; ++i) {
        const unsigned char c = (unsigned char)s[i];
        const bool alnum = (c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
        if (!alnum && strchr("!#$%&'*+-.^_`|~", (int)c) == NULL) {
            return false;
        }
    }
    return true;
}

static void free_headers(sce_hf_header_t *headers, size_t count) {
    for (size_t i = 0; i < count; ++i) {
        free(headers[i].name);
        free(headers[i].value);
    }
    free(headers);
}

/* The header lines of a request, split into name and value; false for a line
   that is not one. Empty lines are ignored, and the value is without the
   spaces and tabs around it. */
static bool header_lines(const char *text, sce_hf_header_t **out, size_t *count) {
    sce_hf_header_t *headers = NULL;
    size_t n = 0;
    const char *line = text;
    for (;;) {
        const char *end = strchr(line, '\n');
        size_t len = end != NULL ? (size_t)(end - line) : strlen(line);
        if (len > 0u && line[len - 1u] == '\r') {
            --len;
        }
        if (len > 0u) {
            const char *colon = memchr(line, ':', len);
            if (colon == NULL || !is_token(line, (size_t)(colon - line))) {
                free_headers(headers, n);
                return false;
            }
            const char *value = colon + 1;
            const char *value_end = line + len;
            while (value < value_end && (*value == ' ' || *value == '\t')) {
                ++value;
            }
            while (value_end > value && (value_end[-1] == ' ' || value_end[-1] == '\t')) {
                --value_end;
            }
            headers = (sce_hf_header_t *)must(realloc(headers, (n + 1u) * sizeof(sce_hf_header_t)));
            headers[n].name = dup_n(line, (size_t)(colon - line));
            headers[n].value = dup_n(value, (size_t)(value_end - value));
            ++n;
        }
        if (end == NULL) {
            break;
        }
        line = end + 1;
    }
    *out = headers;
    *count = n;
    return true;
}

/* An absolute `http` or `https` URL with a host, and no ASCII space or control
   character (U+0000 to U+0020 and U+007F) in it. */
static bool is_http_url(const char *url) {
    const char *rest;
    if (strncmp(url, "http://", 7u) == 0) {
        rest = url + 7;
    } else if (strncmp(url, "https://", 8u) == 0) {
        rest = url + 8;
    } else {
        return false;
    }
    const size_t authority = strcspn(rest, "/?#");
    if (authority == 0u || rest[0] == ':') {
        return false;
    }
    for (const unsigned char *c = (const unsigned char *)url; *c != '\0'; ++c) {
        if (*c <= 0x20u || *c == 0x7Fu) {
            return false;
        }
    }
    return true;
}

static bool http_acceptable(const char *method, const char *url, const char *headers) {
    bool listed = false;
    for (size_t i = 0; i < sizeof(methods) / sizeof(methods[0]); ++i) {
        listed = listed || strcmp(method, methods[i]) == 0;
    }
    if (!listed || !is_http_url(url)) {
        return false;
    }
    sce_hf_header_t *parsed = NULL;
    size_t count = 0;
    if (!header_lines(headers, &parsed, &count)) {
        return false;
    }
    free_headers(parsed, count);
    return true;
}

/* Whether `bytes` is well-formed UTF-8: no overlong form, no surrogate, nothing
   past U+10FFFF. */
static bool valid_utf8(const unsigned char *bytes, size_t n) {
    size_t i = 0;
    while (i < n) {
        const unsigned char b = bytes[i];
        size_t extra;
        uint32_t code;
        if (b < 0x80u) {
            ++i;
            continue;
        }
        if (b >= 0xC2u && b <= 0xDFu) {
            extra = 1u;
            code = b & 0x1Fu;
        } else if (b >= 0xE0u && b <= 0xEFu) {
            extra = 2u;
            code = b & 0x0Fu;
        } else if (b >= 0xF0u && b <= 0xF4u) {
            extra = 3u;
            code = b & 0x07u;
        } else {
            return false;
        }
        if (i + extra >= n) {
            return false;
        }
        for (size_t k = 1u; k <= extra; ++k) {
            if ((bytes[i + k] & 0xC0u) != 0x80u) {
                return false;
            }
            code = (code << 6) | (bytes[i + k] & 0x3Fu);
        }
        if ((extra == 2u && code < 0x800u) || (extra == 3u && code < 0x10000u) || code > 0x10FFFFu ||
            (code >= 0xD800u && code <= 0xDFFFu)) {
            return false;
        }
        i += extra + 1u;
    }
    return true;
}

static void http_fetch(sce_hf_host_t *host, const char *id, const char *method, const char *url, const char *headers,
                       const char *body, sce_hf_outcome_t *out) {
    sce_hf_header_t *lines = NULL;
    size_t line_count = 0;
    (void)header_lines(headers, &lines, &line_count);
    const sce_hf_http_request_t request = {method, url, lines, line_count, body};
    sce_hf_http_response_t response;
    memset(&response, 0, sizeof(response));
    const sce_hf_exchange_t exchange = host->transport.exchange(host->transport.user_data, &request, &response);
    free_headers(lines, line_count);
    if (exchange == SCE_HF_EXCHANGE_FAILURE) {
        failure("sce-host-http-fetch", "network", out);
        return;
    }
    if (exchange == SCE_HF_EXCHANGE_HOLD) {
        strlist_push(&host->held, id);
        out->kind = SCE_HF_HOLD;
        out->data = NULL;
        return;
    }
    sb_t joined;
    memset(&joined, 0, sizeof(joined));
    for (size_t i = 0; i < response.header_count; ++i) {
        if (i > 0u) {
            sb_puts(&joined, "\n");
        }
        for (const char *c = response.headers[i].name; *c != '\0'; ++c) {
            const char lower = (*c >= 'A' && *c <= 'Z') ? (char)(*c - 'A' + 'a') : *c;
            sb_putn(&joined, &lower, 1u);
        }
        sb_puts(&joined, ": ");
        const char *value = response.headers[i].value;
        const char *value_end = value + strlen(value);
        while (*value == ' ' || *value == '\t') {
            ++value;
        }
        while (value_end > value && (value_end[-1] == ' ' || value_end[-1] == '\t')) {
            --value_end;
        }
        sb_putn(&joined, value, (size_t)(value_end - value));
    }
    /* A response to HEAD has no body, whatever the transport carried. */
    const size_t body_len = strcmp(method, "HEAD") == 0 ? 0u : response.body_len;
    if (joined.len > HEADERS_BOUND || body_len > BODY_BOUND) {
        failure("sce-host-http-fetch", "response_too_large", out);
    } else if (!valid_utf8(response.body, body_len)) {
        failure("sce-host-http-fetch", "invalid_body", out);
    } else {
        record_t r;
        record_begin(&r, "");
        record_int(&r, "status", response.status);
        record_text(&r, "headers", joined.p != NULL ? joined.p : "", joined.len);
        record_text(&r, "body", body_len > 0u ? (const char *)response.body : "", body_len);
        record_end(&r, out);
    }
    free(joined.p);
    sce_hf_http_response_free(&response);
}

/* ── Start ──────────────────────────────────────────────────────────── */

static const char *param_of(const sce_host_send_param_t *params, size_t count, const char *name) {
    for (size_t i = 0; i < count; ++i) {
        if (strcmp(params[i].name, name) == 0) {
            return params[i].value;
        }
    }
    return "";
}

static void log_start(sce_hf_host_t *host, const char *operation, const char *id, const sce_host_send_param_t *params,
                      size_t count) {
    if (host->start_count == host->start_cap) {
        host->start_cap = host->start_cap ? host->start_cap * 2u : 16u;
        host->starts = (start_t *)must(realloc(host->starts, host->start_cap * sizeof(start_t)));
    }
    start_t *start = &host->starts[host->start_count++];
    start->operation = dup_str(operation);
    start->id = dup_str(id);
    start->param_count = count;
    start->params = (sce_host_send_param_t *)must(calloc(count > 0u ? count : 1u, sizeof(sce_host_send_param_t)));
    for (size_t i = 0; i < count; ++i) {
        start->params[i].name = dup_str(params[i].name);
        start->params[i].value = dup_str(params[i].value);
    }
}

bool sce_hf_start(sce_hf_host_t *host, const char *operation, const char *invoke_id,
                  const sce_host_send_param_t *params, size_t param_count, sce_hf_outcome_t *out) {
    bool known = false;
    for (size_t i = 0; i < SCE_HF_TYPE_COUNT; ++i) {
        known = known || strcmp(operation, sce_hf_types[i]) == 0;
    }
    out->kind = SCE_HF_HOLD;
    out->data = NULL;
    if (!known) {
        return false;
    }
    log_start(host, operation, invoke_id, params, param_count);
#define ARG(name) param_of(params, param_count, name)
    bool refused = false;
    if (strcmp(operation, "sce-host-kv-get") == 0 || strcmp(operation, "sce-host-kv-delete") == 0 ||
        strcmp(operation, "sce-host-kv-put") == 0) {
        refused = ARG("key")[0] == '\0';
    } else if (strcmp(operation, "sce-host-notify-post") == 0 || strcmp(operation, "sce-host-notify-cancel") == 0) {
        refused = ARG("id")[0] == '\0';
    } else if (strcmp(operation, "sce-host-http-fetch") == 0) {
        refused = !http_acceptable(ARG("method"), ARG("url"), ARG("headers"));
    }
    if (refused) {
        out->kind = SCE_HF_REFUSE;
        out->data = dup_str("{\"reason\":\"invalid_argument\"}");
        return true;
    }
    for (size_t i = 0; i < host->fault_count; ++i) {
        if (strcmp(host->faults[i].operation, operation) == 0) {
            failure(operation, host->faults[i].code, out);
            return true;
        }
    }
    if (strcmp(operation, "sce-host-clock-now") == 0) {
        clock_now(host, out);
    } else if (strcmp(operation, "sce-host-clock-zone") == 0) {
        clock_zone(host, out);
    } else if (strcmp(operation, "sce-host-kv-get") == 0) {
        kv_get(host, ARG("key"), out);
    } else if (strcmp(operation, "sce-host-kv-put") == 0) {
        kv_put(host, ARG("key"), ARG("value"), out);
    } else if (strcmp(operation, "sce-host-kv-delete") == 0) {
        kv_delete(host, ARG("key"), out);
    } else if (strcmp(operation, "sce-host-kv-next-key") == 0) {
        kv_next_key(host, ARG("after"), ARG("prefix"), out);
    } else if (strcmp(operation, "sce-host-notify-permission") == 0) {
        notify_state(host, out);
    } else if (strcmp(operation, "sce-host-notify-request-permission") == 0) {
        notify_request_permission(host, out);
    } else if (strcmp(operation, "sce-host-notify-post") == 0) {
        notify_post(host, ARG("id"), ARG("at_ms"), out);
    } else if (strcmp(operation, "sce-host-notify-cancel") == 0) {
        notify_cancel(host, ARG("id"), out);
    } else {
        http_fetch(host, invoke_id, ARG("method"), ARG("url"), ARG("headers"), ARG("body"), out);
    }
#undef ARG
    return true;
}

void sce_hf_cancel(sce_hf_host_t *host, const char *invoke_id) {
    (void)strlist_remove(&host->held, invoke_id);
    strlist_push(&host->cancelled, invoke_id);
}

/* ── The engine's side ──────────────────────────────────────────────── */

void sce_hf_invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    sce_hf_host_t *host = (sce_hf_host_t *)user_data;
    if (event->phase == SCE_HOST_INVOKE_CANCEL) {
        sce_hf_cancel(host, event->invoke_id);
        return;
    }
    sce_hf_outcome_t outcome;
    if (!sce_hf_start(host, event->processor_type, event->invoke_id, event->params, (size_t)event->param_count,
                      &outcome)) {
        return;
    }
    if (outcome.kind == SCE_HF_DONE) {
        if (strlen(outcome.data) >= (size_t)SCE_MAX_DATA_LEN) {
            /* The machine's data slot cannot carry this result: the host declines to start what it could not
               answer. A build that serves the operation sets SCE_MAX_DATA_LEN to at least SCE_HF_MIN_DATA_LEN. */
            out->has_refusal = true;
            (void)snprintf(out->refusal, sizeof(out->refusal), "{\"reason\":\"data_slot\"}");
        } else {
            out->has_done_data = true;
            (void)snprintf(out->done_data, sizeof(out->done_data), "%s", outcome.data);
        }
    } else if (outcome.kind == SCE_HF_REFUSE) {
        out->has_refusal = true;
        (void)snprintf(out->refusal, sizeof(out->refusal), "%s", outcome.data);
    }
    sce_hf_outcome_free(&outcome);
}

bool sce_hf_register(sce_host_invoker_registry_t *registry, sce_hf_host_t *host) {
    for (size_t i = 0; i < SCE_HF_TYPE_COUNT; ++i) {
        if (!sce_host_invoker_register(registry, sce_hf_types[i], sce_hf_invoker, host)) {
            return false;
        }
    }
    return true;
}
