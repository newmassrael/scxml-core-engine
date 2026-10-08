// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-C-2: the `<param>`s of a BasicHTTP `<send>` of a `datamodel="sce-static"`
// machine are read from its own fields when the send runs, and cross as the text a
// form carries (docs/adr/0005, decision 4) -- generated C.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_send_http.scxml, the
// document the Rust, Kotlin, Go, Python and C++ suites hold with a test of their
// own that records the request.
//
// The machine makes its POST through the HTTP client surface of
// `sce/http_client.h`, which a platform supplies (`sce_c_runtime_posix` is the POSIX
// one; the template says a bare-metal consumer provides the same symbols). This test
// is that consumer: it provides them itself, and keeps the request instead of making
// it, so the machine is observed where it hands its POST to the platform and no
// listener is involved. The test is linked WITHOUT `sce_c_runtime_posix`, so the
// double is the only definition of these symbols and the link is the proof.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <sce/http_client.h>

#include "static_send_http_sm.h"

// ---- the platform's HTTP client, as this consumer provides it -----------------

typedef struct {
    char host[128];
    unsigned port;
    char path[256];
    char content_type[64];
    char body[1024];
} posted_t;

static posted_t g_posted[4];
static int g_posted_count;

bool sce_test_http_url_parse(const char *url, sce_test_http_url_t *out) {
    // `scheme://host[:port]/path`, as far as the fixture's target needs.
    const char *rest = strstr(url, "://");
    if (rest == NULL) {
        return false;
    }
    rest += 3;
    const char *end = rest;
    while (*end != '\0' && *end != '/' && *end != ':') {
        ++end;
    }
    const size_t host_len = (size_t)(end - rest);
    if (host_len == 0u || host_len >= sizeof(out->host)) {
        return false;
    }
    memcpy(out->host, rest, host_len);
    out->host[host_len] = '\0';
    out->port = 80u;
    if (*end == ':') {
        out->port = (uint16_t)strtoul(end + 1, (char **)&end, 10);
    }
    (void)snprintf(out->path, sizeof(out->path), "%s", *end == '\0' ? "/" : end);
    return true;
}

// `key=value` pairs joined by `&`. The runtime's own appender percent-encodes; the
// values this fixture sends need none, and what is held here is which pairs the
// machine hands over.
bool sce_test_http_form_append(char *body, size_t cap, size_t *body_len, const char *key, const char *value) {
    const size_t need = strlen(key) + 1u + strlen(value) + (*body_len > 0u ? 1u : 0u);
    if (*body_len + need >= cap) {
        return false;
    }
    if (*body_len > 0u) {
        body[(*body_len)++] = '&';
    }
    *body_len += (size_t)snprintf(body + *body_len, cap - *body_len, "%s=%s", key, value);
    return true;
}

bool sce_test_http_post(const sce_test_http_url_t *url, const char *content_type, const char *body, size_t body_len,
                        int timeout_ms, sce_test_http_response_t *out) {
    (void)timeout_ms;
    if (g_posted_count < 4 && body_len < sizeof(g_posted[0].body)) {
        posted_t *p = &g_posted[g_posted_count];
        (void)snprintf(p->host, sizeof(p->host), "%s", url->host);
        p->port = url->port;
        (void)snprintf(p->path, sizeof(p->path), "%s", url->path);
        (void)snprintf(p->content_type, sizeof(p->content_type), "%s", content_type);
        memcpy(p->body, body, body_len);
        p->body[body_len] = '\0';
    }
    g_posted_count++;
    // An acknowledged request with an empty reply: it names no event.
    out->ok = true;
    out->status_code = 200;
    out->body = (char *)calloc(1u, 1u);
    out->body_len = 0u;
    return out->body != NULL;
}

void sce_test_http_response_free(sce_test_http_response_t *r) {
    free(r->body);
    r->body = NULL;
}

bool sce_test_http_parse_response(char *body, size_t body_len, sce_test_http_json_response_t *out) {
    (void)body;
    (void)body_len;
    (void)out;
    return false;
}

// ---- the cases -----------------------------------------------------------------

static void raise_event(static_send_http_t *sm, static_send_http_event_t event) {
    static_send_http_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    static_send_http_raise_external(sm, &meta);
    static_send_http_step(sm);
}

// Whether the form holds the pair `key=value` whole: between separators, so `count=4`
// is not found in `count=42` or in `recount=4`.
static bool form_holds(const char *body, const char *pair) {
    char wrapped[1100];
    char needle[160];
    (void)snprintf(wrapped, sizeof(wrapped), "&%s&", body);
    (void)snprintf(needle, sizeof(needle), "&%s&", pair);
    return strstr(wrapped, needle) != NULL;
}

static int form_pair_count(const char *body) {
    int count = body[0] == '\0' ? 0 : 1;
    for (const char *c = body; *c != '\0'; ++c) {
        count += *c == '&' ? 1 : 0;
    }
    return count;
}

static int fail(const char *what) {
    (void)fprintf(stderr, "static_send_http: FAIL - %s\n", what);
    return 1;
}

static int expect_pairs(const char *scenario, const char *const *want, int want_count) {
    int bad = 0;
    if (g_posted_count != 1) {
        (void)fprintf(stderr, "static_send_http: FAIL [%s] - %d requests were made, want one\n", scenario,
                      g_posted_count);
        return 1;
    }
    const posted_t *p = &g_posted[0];
    if (strcmp(p->host, "example.invalid") != 0 || strcmp(p->path, "/hook") != 0) {
        (void)fprintf(stderr, "static_send_http: FAIL [%s] - the request went to %s%s\n", scenario, p->host, p->path);
        bad = 1;
    }
    if (strcmp(p->content_type, "application/x-www-form-urlencoded") != 0) {
        (void)fprintf(stderr, "static_send_http: FAIL [%s] - the body is %s, not a form\n", scenario, p->content_type);
        bad = 1;
    }
    for (int i = 0; i < want_count; ++i) {
        if (!form_holds(p->body, want[i])) {
            (void)fprintf(stderr, "static_send_http: FAIL [%s] - the form `%s` does not hold `%s`\n", scenario, p->body,
                          want[i]);
            bad = 1;
        }
    }
    // The pairs asked for and the event's name, and nothing else: a pair that
    // should have been left out is told from one that is present.
    if (form_pair_count(p->body) != want_count) {
        (void)fprintf(stderr, "static_send_http: FAIL [%s] - the form `%s` has %d pairs, want %d\n", scenario, p->body,
                      form_pair_count(p->body), want_count);
        bad = 1;
    }
    return bad;
}

// Each value is the text it spells, read when the send runs: `bump` makes the fields
// 4, true and "busy" before `go` reads them.
static int a_send_over_http_carries_the_text_the_fields_hold_when_it_runs(void) {
    g_posted_count = 0;
    static_send_http_t sm;
    static_send_http_init(&sm);
    raise_event(&sm, STATIC_SEND_HTTP_EVENT_BUMP);
    raise_event(&sm, STATIC_SEND_HTTP_EVENT_GO);
    const char *const want[] = {
        "_scxmleventname=note", "count=4", "ready=true", "label=busy", "twice=8", "delta=-5", "ratio=1.5"};
    return expect_pairs("after bump", want, 7);
}

// The control: nothing has written a variable, so the fields hold what `<data expr>`
// gave them and the request carries those, not the values `bump` would have set.
static int the_pairs_are_the_fields_as_they_stand_and_not_a_copy_from_start_up(void) {
    g_posted_count = 0;
    static_send_http_t sm;
    static_send_http_init(&sm);
    raise_event(&sm, STATIC_SEND_HTTP_EVENT_GO);
    const char *const want[] = {
        "_scxmleventname=note", "count=3", "ready=false", "label=idle", "twice=6", "delta=-5", "ratio=1.5"};
    return expect_pairs("before any bump", want, 7);
}

// `big` is `count * 2000000000`, which a 32-bit field cannot hold: its pair is left
// out, not carried as a zero, the request still goes with the pair that could be
// read, and the failure is counted as error.execution once (§scxml-5.7.1).
static int a_param_that_cannot_be_read_is_left_out_and_the_request_still_goes(void) {
    g_posted_count = 0;
    static_send_http_t sm;
    static_send_http_init(&sm);
    raise_event(&sm, STATIC_SEND_HTTP_EVENT_BUMP);
    raise_event(&sm, STATIC_SEND_HTTP_EVENT_BOOM);
    const char *const want[] = {"_scxmleventname=note", "count=4"};
    int bad = expect_pairs("a param that overflows", want, 2);
    if (g_posted_count == 1 && strstr(g_posted[0].body, "big=") != NULL) {
        bad |= fail("the pair whose value failed was carried");
    }
    if (static_send_http_get_errors(&sm) != 1u) {
        (void)fprintf(stderr, "static_send_http: FAIL - errors is %u, want 1\n",
                      (unsigned)static_send_http_get_errors(&sm));
        bad = 1;
    }
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_send_over_http_carries_the_text_the_fields_hold_when_it_runs();
    bad |= the_pairs_are_the_fields_as_they_stand_and_not_a_copy_from_start_up();
    bad |= a_param_that_cannot_be_read_is_left_out_and_the_request_still_goes();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static send http: ok\n");
    return 0;
}
