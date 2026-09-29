// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.2: a delay is a property of the send, not of the
// processor it names — a delayed BasicHTTP send is POSTed when due, and
// <cancel> reaches it while it waits — C11 AOT.
//
// The other channels stand in for the HTTP transport with a recording
// callback. This channel's client makes the POST itself, so the transport
// here is a listener on the loopback address that records what arrives: the
// request line's path and the body, in the order they arrive. What it reads
// off the wire is therefore the request the machine built — the form, the
// reserved `_scxmleventname`, the `<param>` — and not a structure the machine
// handed over. The clock is manual, and the POST is synchronous, so a request
// is recorded before the call that made it returns: the order and the instant
// are the engine's, not the host's.
//
// Fixture:
// integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml
// (canonical, shared with the C++ AOT / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_delayed_http_send_is_posted_when_due ...)`
// in `backends/c/tests/CMakeLists.txt`.

// Sockets, poll and pthreads are POSIX, and the target is built with
// C_EXTENSIONS OFF — so the feature-test macro has to precede every include.
#define _POSIX_C_SOURCE 200809L

#include <arpa/inet.h>
#include <netinet/in.h>
#include <poll.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <unistd.h>

#include "a_delayed_http_send_is_posted_when_due_sm.h"

typedef a_delayed_http_send_is_posted_when_due_t sm_t;

// The port the fixture's targets name (`http://127.0.0.1:18081/...`).
#define LISTEN_PORT 18081

typedef struct {
    char path[64];
    char body[256];
} recorded_t;

static recorded_t g_requests[8];
static int g_count;
static pthread_mutex_t g_lock = PTHREAD_MUTEX_INITIALIZER;
static atomic_bool g_stop;
static int g_listen_fd = -1;

// The value of `key` in a form body, copied into `out`, or "" when absent.
static void form_value(const char *body, const char *key, char *out, size_t cap) {
    out[0] = '\0';
    const size_t key_len = strlen(key);
    const char *at = body;
    while (at != NULL && *at != '\0') {
        if (strncmp(at, key, key_len) == 0 && at[key_len] == '=') {
            const char *value = at + key_len + 1;
            const char *end = strchr(value, '&');
            size_t n = end != NULL ? (size_t)(end - value) : strlen(value);
            if (n >= cap) {
                n = cap - 1u;
            }
            memcpy(out, value, n);
            out[n] = '\0';
            return;
        }
        at = strchr(at, '&');
        if (at != NULL) {
            ++at;
        }
    }
}

// Read one HTTP request from `fd`: the request line's path and the body.
static bool read_request(int fd, recorded_t *out) {
    char buf[2048];
    size_t have = 0u;
    char *body_at = NULL;
    while (have < sizeof(buf) - 1u) {
        const ssize_t n = recv(fd, buf + have, sizeof(buf) - 1u - have, 0);
        if (n <= 0) {
            return false;
        }
        have += (size_t)n;
        buf[have] = '\0';
        body_at = strstr(buf, "\r\n\r\n");
        if (body_at != NULL) {
            break;
        }
    }
    if (body_at == NULL) {
        return false;
    }
    body_at += 4;
    size_t content_length = 0u;
    for (const char *line = strstr(buf, "\r\n"); line != NULL && line < body_at; line = strstr(line + 2, "\r\n")) {
        if (strncasecmp(line + 2, "Content-Length:", 15) == 0) {
            content_length = (size_t)strtoul(line + 17, NULL, 10);
        }
    }
    while ((size_t)(buf + have - body_at) < content_length && have < sizeof(buf) - 1u) {
        const ssize_t n = recv(fd, buf + have, sizeof(buf) - 1u - have, 0);
        if (n <= 0) {
            return false;
        }
        have += (size_t)n;
        buf[have] = '\0';
    }
    // "POST /path HTTP/1.1"
    const char *path = strchr(buf, ' ');
    const char *path_end = path != NULL ? strchr(path + 1, ' ') : NULL;
    if (path == NULL || path_end == NULL) {
        return false;
    }
    size_t path_len = (size_t)(path_end - (path + 1));
    if (path_len >= sizeof(out->path)) {
        path_len = sizeof(out->path) - 1u;
    }
    memcpy(out->path, path + 1, path_len);
    out->path[path_len] = '\0';
    size_t body_len = content_length;
    if (body_len >= sizeof(out->body)) {
        body_len = sizeof(out->body) - 1u;
    }
    memcpy(out->body, body_at, body_len);
    out->body[body_len] = '\0';
    return true;
}

static void *serve(void *unused) {
    (void)unused;
    while (!atomic_load(&g_stop)) {
        struct pollfd wait = {.fd = g_listen_fd, .events = POLLIN, .revents = 0};
        if (poll(&wait, 1, 50) <= 0) {
            continue;
        }
        const int fd = accept(g_listen_fd, NULL, NULL);
        if (fd < 0) {
            continue;
        }
        recorded_t request;
        if (read_request(fd, &request)) {
            pthread_mutex_lock(&g_lock);
            if (g_count < (int)(sizeof(g_requests) / sizeof(g_requests[0]))) {
                g_requests[g_count++] = request;
            }
            pthread_mutex_unlock(&g_lock);
            // Answer as the W3C harness server does: the event the request
            // named, echoed back, so the reply path is exercised too.
            char event[64];
            form_value(request.body, "_scxmleventname", event, sizeof(event));
            char json[160];
            const int json_len = snprintf(json, sizeof(json), "{\"status\":\"success\",\"event\":\"%s\",\"data\":{}}",
                                          event[0] != '\0' ? event : "event1");
            char reply[384];
            const int reply_len = snprintf(reply, sizeof(reply),
                                           "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n"
                                           "Content-Length: %d\r\nConnection: close\r\n\r\n%s",
                                           json_len, json);
            (void)send(fd, reply, (size_t)reply_len, MSG_NOSIGNAL);
        }
        close(fd);
    }
    return NULL;
}

static bool start_listener(pthread_t *thread) {
    g_listen_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (g_listen_fd < 0) {
        return false;
    }
    const int yes = 1;
    (void)setsockopt(g_listen_fd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(LISTEN_PORT);
    (void)inet_pton(AF_INET, "127.0.0.1", &addr.sin_addr);
    if (bind(g_listen_fd, (struct sockaddr *)&addr, sizeof(addr)) != 0 || listen(g_listen_fd, 8) != 0) {
        return false;
    }
    return pthread_create(thread, NULL, serve, NULL) == 0;
}

static int g_ok = 1;

// The requests recorded so far must be exactly `want`, in order, each as the
// path and body the machine should have sent.
static void expect(const char *when, int want_count, const recorded_t *want) {
    pthread_mutex_lock(&g_lock);
    if (g_count != want_count) {
        fprintf(stderr, "FAIL: %s: %d request(s) POSTed, want %d\n", when, g_count, want_count);
        g_ok = 0;
    } else {
        for (int i = 0; i < want_count; ++i) {
            if (strcmp(g_requests[i].path, want[i].path) != 0 || strcmp(g_requests[i].body, want[i].body) != 0) {
                fprintf(stderr, "FAIL: %s: request %d was POST %s body '%s', want %s '%s'\n", when, i,
                        g_requests[i].path, g_requests[i].body, want[i].path, want[i].body);
                g_ok = 0;
            }
        }
    }
    pthread_mutex_unlock(&g_lock);
}

int main(void) {
    pthread_t listener;
    if (!start_listener(&listener)) {
        fprintf(stderr, "FAIL: cannot listen on 127.0.0.1:%d — the fixture's targets name it\n", LISTEN_PORT);
        return 1;
    }

    static const recorded_t all[] = {
        {"/now", "_scxmleventname=now"},
        {"/later", "_scxmleventname=later"},
        {"/dynamic", "_scxmleventname=dynamic&k=v"},
    };

    sm_t sm;
    a_delayed_http_send_is_posted_when_due_init_with_clock(&sm, sce_clock_manual(0u));
    expect("at once", 1, all);

    a_delayed_http_send_is_posted_when_due_advance_time_ms(&sm, 99u);
    expect("at 99ms (a send delayed 100ms is not due)", 1, all);

    a_delayed_http_send_is_posted_when_due_advance_time_ms(&sm, 1u);
    expect("at 100ms (due now; the cancelled one never is)", 2, all);

    a_delayed_http_send_is_posted_when_due_advance_time_ms(&sm, 100u);
    expect("at 200ms", 3, all);

    a_delayed_http_send_is_posted_when_due_advance_time_ms(&sm, 100u);
    expect("at 300ms", 3, all);
    if (!a_delayed_http_send_is_posted_when_due_ended_in(&sm, A_DELAYED_HTTP_SEND_IS_POSTED_WHEN_DUE_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        g_ok = 0;
    }

    a_delayed_http_send_is_posted_when_due_destroy(&sm);
    atomic_store(&g_stop, true);
    (void)pthread_join(listener, NULL);
    (void)close(g_listen_fd);
    if (!g_ok) {
        return 1;
    }
    printf("PASS: a delayed BasicHTTP <send> is POSTed when due\n");
    return 0;
}
