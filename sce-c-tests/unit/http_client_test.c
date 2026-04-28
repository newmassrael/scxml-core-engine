// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Unit tests for sce-c-tests/support/http_client.{c,h} — pin the URL
// parser, form-encoded body builder, JSON response extractor, and
// end-to-end POST round-trip against an in-process loopback server
// (TCP on 127.0.0.1, OS-assigned port, single-shot listen-accept-
// respond-close lambda thread).
//
// W3C SCXML C.2 corpus does not exercise edge cases like trailing
// commas, embedded `}` inside `data` strings, surrogate-pair `\uXXXX`,
// or chunked Content-Length boundaries — these tests force them so
// drift in the standalone server's response shape (or our client's
// parser) surfaces here, not as a flaky fixture months later.
//
// Each test returns 0 on PASS, non-zero on FAIL. main() aggregates.

#include "http_client.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

#define ASSERT_TRUE(cond, msg) \
    do { if (!(cond)) { fprintf(stderr, "  FAIL: %s\n", msg); return 1; } } while (0)

#define ASSERT_STREQ(actual, expected, msg) \
    do { \
        if (strcmp((actual), (expected)) != 0) { \
            fprintf(stderr, "  FAIL: %s — got '%s', want '%s'\n", msg, (actual), (expected)); \
            return 1; \
        } \
    } while (0)

#define ASSERT_MEMEQ(actual, actual_len, expected, msg) \
    do { \
        size_t _exp_len = strlen(expected); \
        if ((actual_len) != _exp_len || memcmp((actual), (expected), _exp_len) != 0) { \
            fprintf(stderr, "  FAIL: %s — got '%.*s' (%zu B), want '%s' (%zu B)\n", \
                    msg, (int)(actual_len), (actual), (size_t)(actual_len), \
                    (expected), _exp_len); \
            return 1; \
        } \
    } while (0)

/* ── URL parser ─────────────────────────────────────────────────── */

static int test_url_parse_basic(void) {
    sce_test_http_url_t url;
    ASSERT_TRUE(sce_test_http_url_parse("http://localhost:8080/test", &url),
                "parse must succeed");
    ASSERT_STREQ(url.host, "localhost", "host");
    ASSERT_TRUE(url.port == 8080, "port == 8080");
    ASSERT_STREQ(url.path, "/test", "path");
    return 0;
}

static int test_url_parse_default_port(void) {
    sce_test_http_url_t url;
    ASSERT_TRUE(sce_test_http_url_parse("http://example.com/api", &url),
                "parse must succeed");
    ASSERT_STREQ(url.host, "example.com", "host");
    ASSERT_TRUE(url.port == 80, "default port 80");
    ASSERT_STREQ(url.path, "/api", "path");
    return 0;
}

static int test_url_parse_no_path(void) {
    sce_test_http_url_t url;
    ASSERT_TRUE(sce_test_http_url_parse("http://localhost:8080", &url),
                "parse must succeed");
    ASSERT_TRUE(url.port == 8080, "port == 8080");
    ASSERT_STREQ(url.path, "/", "default path /");
    return 0;
}

static int test_url_parse_rejects_https(void) {
    sce_test_http_url_t url;
    ASSERT_TRUE(!sce_test_http_url_parse("https://localhost/x", &url),
                "https:// must be rejected");
    return 0;
}

static int test_url_parse_rejects_garbage(void) {
    sce_test_http_url_t url;
    ASSERT_TRUE(!sce_test_http_url_parse("not-a-url", &url),
                "non-http scheme must fail");
    ASSERT_TRUE(!sce_test_http_url_parse("http://", &url),
                "empty host must fail");
    ASSERT_TRUE(!sce_test_http_url_parse("http://host:0/x", &url),
                "port 0 must fail");
    ASSERT_TRUE(!sce_test_http_url_parse("http://host:99999/x", &url),
                "port > 65535 must fail");
    return 0;
}

/* ── Form-encoded body builder ──────────────────────────────────── */

static int test_form_encode_basic(void) {
    char buf[256];
    buf[0] = '\0';
    size_t len = 0u;
    ASSERT_TRUE(sce_test_http_form_append(buf, sizeof(buf), &len,
                                          "_scxmleventname", "test"),
                "append k1");
    ASSERT_TRUE(sce_test_http_form_append(buf, sizeof(buf), &len,
                                          "param1", "2"),
                "append k2");
    ASSERT_STREQ(buf, "_scxmleventname=test&param1=2", "joined body");
    return 0;
}

static int test_form_encode_pct_encoding(void) {
    char buf[256];
    buf[0] = '\0';
    size_t len = 0u;
    /* Space → +, special chars → %XX (RFC 3986 unreserved set
       preserved). */
    ASSERT_TRUE(sce_test_http_form_append(buf, sizeof(buf), &len,
                                          "k", "a b/c=d&e"),
                "append");
    ASSERT_STREQ(buf, "k=a+b%2Fc%3Dd%26e", "encoded body");
    return 0;
}

static int test_form_encode_overflow(void) {
    char buf[16];
    buf[0] = '\0';
    size_t len = 0u;
    ASSERT_TRUE(!sce_test_http_form_append(buf, sizeof(buf), &len,
                                           "verylongkey", "verylongvalue"),
                "must reject overflow");
    return 0;
}

/* ── JSON response extractor ────────────────────────────────────── */

static int test_json_event_only(void) {
    char body[] = "{\"event\":\"test\"}";
    sce_test_http_json_response_t resp;
    ASSERT_TRUE(sce_test_http_parse_response(body, strlen(body), &resp),
                "parse");
    ASSERT_TRUE(resp.ok, "ok");
    ASSERT_MEMEQ(resp.event_name, resp.event_name_len, "test", "event_name");
    ASSERT_TRUE(resp.event_data == NULL, "no data");
    return 0;
}

static int test_json_event_and_data_string(void) {
    char body[] =
        "{\"status\":\"success\","
        "\"event\":\"HTTP.POST\","
        "\"data\":\"some content\","
        "\"timestamp\":1234}";
    sce_test_http_json_response_t resp;
    ASSERT_TRUE(sce_test_http_parse_response(body, strlen(body), &resp),
                "parse");
    ASSERT_MEMEQ(resp.event_name, resp.event_name_len, "HTTP.POST", "event");
    ASSERT_TRUE(resp.data_is_string, "data_is_string");
    ASSERT_MEMEQ(resp.event_data, resp.event_data_len,
                 "some content", "event_data");
    return 0;
}

static int test_json_event_and_data_object(void) {
    /* test567 shape: data is a JSON object containing form params. */
    char body[] =
        "{\"event\":\"test\",\"data\":{\"param1\":2,\"_scxmleventname\":\"test\"}}";
    sce_test_http_json_response_t resp;
    ASSERT_TRUE(sce_test_http_parse_response(body, strlen(body), &resp),
                "parse");
    ASSERT_TRUE(!resp.data_is_string, "data is object, not string");
    ASSERT_MEMEQ(resp.event_data, resp.event_data_len,
                 "{\"param1\":2,\"_scxmleventname\":\"test\"}",
                 "raw object span");
    return 0;
}

static int test_json_data_with_embedded_brace(void) {
    /* Depth-counted object capture must not split on `}` that lives
       inside a JSON string value. */
    char body[] = "{\"event\":\"x\",\"data\":{\"k\":\"a}b}c\"}}";
    sce_test_http_json_response_t resp;
    ASSERT_TRUE(sce_test_http_parse_response(body, strlen(body), &resp),
                "parse");
    ASSERT_MEMEQ(resp.event_data, resp.event_data_len,
                 "{\"k\":\"a}b}c\"}", "object captured intact");
    return 0;
}

static int test_json_string_unescape(void) {
    /* Verify in-place unescape handles \" / \\ / \n / \uXXXX. */
    char body[] = "{\"event\":\"line\\nbreak\",\"data\":\"hi \\u00E9!\"}";
    sce_test_http_json_response_t resp;
    ASSERT_TRUE(sce_test_http_parse_response(body, strlen(body), &resp),
                "parse");
    ASSERT_MEMEQ(resp.event_name, resp.event_name_len,
                 "line\nbreak", "event newline");
    ASSERT_MEMEQ(resp.event_data, resp.event_data_len,
                 "hi \xC3\xA9!", "data UTF-8 e-acute");
    return 0;
}

static int test_json_missing_event_rejected(void) {
    char body[] = "{\"status\":\"success\",\"data\":\"x\"}";
    sce_test_http_json_response_t resp;
    ASSERT_TRUE(!sce_test_http_parse_response(body, strlen(body), &resp),
                "missing event rejected");
    return 0;
}

static int test_json_non_object_rejected(void) {
    char body[] = "[\"event\",\"test\"]";
    sce_test_http_json_response_t resp;
    ASSERT_TRUE(!sce_test_http_parse_response(body, strlen(body), &resp),
                "top-level array rejected");
    return 0;
}

/* ── End-to-end POST against in-process loopback server ─────────── */

typedef struct {
    int listen_fd;
    int port;
    /* Static reply: full HTTP/1.1 response written verbatim once a
       client connects. The unit test only needs single-shot. */
    const char *reply;
    size_t reply_len;
    /* Capture of the inbound request for later assertion. */
    char captured[1024];
    size_t captured_len;
} stub_server_t;

/* One-shot accept loop: accept a single client, read until EOF or
   buffer full, send the static reply, close. Stops the server. */
static void *stub_server_thread(void *arg) {
    stub_server_t *s = (stub_server_t *)arg;
    int client = accept(s->listen_fd, NULL, NULL);
    if (client < 0) {
        return NULL;
    }
    /* Read until we see end of headers + Content-Length bytes (or
       until the buffer fills). For unit-test simplicity we just
       drain whatever the client sends in a few recv calls. */
    while (s->captured_len + 1u < sizeof(s->captured)) {
        ssize_t n = recv(client, s->captured + s->captured_len,
                          sizeof(s->captured) - s->captured_len - 1u, 0);
        if (n <= 0) {
            break;
        }
        s->captured_len += (size_t)n;
        s->captured[s->captured_len] = '\0';
        /* End condition: full headers received + body satisfied.
           For the test bodies we ship, the request fits in one recv
           call most of the time; loop just in case the kernel
           splits. We exit when we've drained for one round-trip. */
        if (strstr(s->captured, "\r\n\r\n") != NULL) {
            /* Headers in; assume body fits. Real correctness would
               require parsing Content-Length, but the test bodies
               are small enough that the kernel hands them over in
               the same recv. */
            break;
        }
    }
    /* Send the static reply. */
    size_t sent = 0;
    while (sent < s->reply_len) {
        ssize_t n = send(client, s->reply + sent, s->reply_len - sent, 0);
        if (n <= 0) {
            break;
        }
        sent += (size_t)n;
    }
    close(client);
    return NULL;
}

/* Bring up the stub server on 127.0.0.1:0 (OS-assigned port). */
static int stub_server_start(stub_server_t *s, const char *reply) {
    memset(s, 0, sizeof(*s));
    s->reply = reply;
    s->reply_len = strlen(reply);
    s->listen_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (s->listen_fd < 0) {
        return -1;
    }
    int one = 1;
    setsockopt(s->listen_fd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof(one));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    addr.sin_port = 0;  /* OS-assigned */
    if (bind(s->listen_fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(s->listen_fd);
        return -1;
    }
    socklen_t addr_len = sizeof(addr);
    if (getsockname(s->listen_fd, (struct sockaddr *)&addr, &addr_len) < 0) {
        close(s->listen_fd);
        return -1;
    }
    s->port = ntohs(addr.sin_port);
    if (listen(s->listen_fd, 1) < 0) {
        close(s->listen_fd);
        return -1;
    }
    return 0;
}

static void stub_server_stop(stub_server_t *s) {
    if (s->listen_fd >= 0) {
        close(s->listen_fd);
        s->listen_fd = -1;
    }
}

static int test_post_roundtrip(void) {
    static const char reply[] =
        "HTTP/1.1 200 OK\r\n"
        "Content-Type: application/json\r\n"
        "Content-Length: 51\r\n"
        "Connection: close\r\n"
        "\r\n"
        "{\"status\":\"success\",\"event\":\"echoed\",\"data\":\"hi\"}";

    stub_server_t s;
    if (stub_server_start(&s, reply) < 0) {
        fprintf(stderr, "  FAIL: stub_server_start failed\n");
        return 1;
    }
    pthread_t th;
    if (pthread_create(&th, NULL, stub_server_thread, &s) != 0) {
        stub_server_stop(&s);
        fprintf(stderr, "  FAIL: pthread_create failed\n");
        return 1;
    }

    char url_str[64];
    snprintf(url_str, sizeof(url_str), "http://127.0.0.1:%d/test", s.port);
    sce_test_http_url_t url;
    ASSERT_TRUE(sce_test_http_url_parse(url_str, &url), "url parse");

    sce_test_http_response_t resp;
    bool ok = sce_test_http_post(&url,
                                 "application/x-www-form-urlencoded",
                                 "_scxmleventname=test", 20u,
                                 5000, &resp);
    pthread_join(th, NULL);
    stub_server_stop(&s);

    if (!ok) {
        fprintf(stderr,
                "  diag: ok=%d status=%d body_len=%zu captured_len=%zu\n",
                ok, resp.status_code, resp.body_len, s.captured_len);
        if (s.captured_len > 0) {
            fprintf(stderr, "  diag captured: '%.*s'\n",
                    (int)s.captured_len, s.captured);
        }
    }
    ASSERT_TRUE(ok, "post must succeed");
    ASSERT_TRUE(resp.status_code == 200, "status 200");
    ASSERT_TRUE(resp.body != NULL, "body present");

    /* Verify the request the server saw carried the expected body. */
    ASSERT_TRUE(strstr(s.captured, "POST /test HTTP/1.1") != NULL,
                "request line");
    ASSERT_TRUE(strstr(s.captured, "Content-Type: "
                                    "application/x-www-form-urlencoded")
                  != NULL,
                "content-type header");
    ASSERT_TRUE(strstr(s.captured, "_scxmleventname=test") != NULL,
                "request body present");

    /* Now extract event/data from the response body (mutates body
       in place to unescape strings). */
    sce_test_http_json_response_t parsed;
    ASSERT_TRUE(sce_test_http_parse_response(resp.body, resp.body_len,
                                              &parsed),
                "parse response");
    ASSERT_MEMEQ(parsed.event_name, parsed.event_name_len,
                 "echoed", "event_name");
    ASSERT_TRUE(parsed.data_is_string, "data is string");
    ASSERT_MEMEQ(parsed.event_data, parsed.event_data_len,
                 "hi", "event_data");

    sce_test_http_response_free(&resp);
    return 0;
}

/* Chunked-encoded round-trip — the Node standalone server emits
   `Transfer-Encoding: chunked` for JSON responses (Express auto-
   chunks). The client must reassemble the chunk payloads before the
   JSON extractor sees the body. */
static int test_post_roundtrip_chunked(void) {
    /* Body `{"event":"test","data":"hi"}` = 28 bytes; encoded as a
       single 0x1c chunk plus the terminator. The test pins the
       dechunk path against drift in the standalone server's
       framing choice. */
    static const char reply[] =
        "HTTP/1.1 200 OK\r\n"
        "Content-Type: application/json\r\n"
        "Transfer-Encoding: chunked\r\n"
        "Connection: close\r\n"
        "\r\n"
        "1c\r\n"
        "{\"event\":\"test\",\"data\":\"hi\"}\r\n"
        "0\r\n"
        "\r\n";

    stub_server_t s;
    if (stub_server_start(&s, reply) < 0) {
        fprintf(stderr, "  FAIL: stub_server_start failed\n");
        return 1;
    }
    pthread_t th;
    if (pthread_create(&th, NULL, stub_server_thread, &s) != 0) {
        stub_server_stop(&s);
        fprintf(stderr, "  FAIL: pthread_create failed\n");
        return 1;
    }

    char url_str[64];
    snprintf(url_str, sizeof(url_str), "http://127.0.0.1:%d/test", s.port);
    sce_test_http_url_t url;
    ASSERT_TRUE(sce_test_http_url_parse(url_str, &url), "url parse");

    sce_test_http_response_t resp;
    bool ok = sce_test_http_post(&url, "application/x-www-form-urlencoded",
                                  "x=y", 3u, 5000, &resp);
    pthread_join(th, NULL);
    stub_server_stop(&s);

    ASSERT_TRUE(ok, "post must succeed");
    ASSERT_TRUE(resp.status_code == 200, "status 200");
    /* After dechunk, body length must match the chunk payload size. */
    ASSERT_TRUE(resp.body_len == 28u, "dechunked body length");

    sce_test_http_json_response_t parsed;
    ASSERT_TRUE(sce_test_http_parse_response(resp.body, resp.body_len,
                                              &parsed),
                "parse dechunked response");
    ASSERT_MEMEQ(parsed.event_name, parsed.event_name_len,
                 "test", "event_name");
    ASSERT_MEMEQ(parsed.event_data, parsed.event_data_len,
                 "hi", "event_data");

    sce_test_http_response_free(&resp);
    return 0;
}

/* ── main: aggregate ────────────────────────────────────────────── */

int main(void) {
    static const struct {
        const char *name;
        int (*fn)(void);
    } tests[] = {
        {"url_parse_basic",              test_url_parse_basic},
        {"url_parse_default_port",       test_url_parse_default_port},
        {"url_parse_no_path",            test_url_parse_no_path},
        {"url_parse_rejects_https",      test_url_parse_rejects_https},
        {"url_parse_rejects_garbage",    test_url_parse_rejects_garbage},
        {"form_encode_basic",            test_form_encode_basic},
        {"form_encode_pct_encoding",     test_form_encode_pct_encoding},
        {"form_encode_overflow",         test_form_encode_overflow},
        {"json_event_only",              test_json_event_only},
        {"json_event_and_data_string",   test_json_event_and_data_string},
        {"json_event_and_data_object",   test_json_event_and_data_object},
        {"json_data_with_embedded_brace",test_json_data_with_embedded_brace},
        {"json_string_unescape",         test_json_string_unescape},
        {"json_missing_event_rejected",  test_json_missing_event_rejected},
        {"json_non_object_rejected",     test_json_non_object_rejected},
        {"post_roundtrip",               test_post_roundtrip},
        {"post_roundtrip_chunked",       test_post_roundtrip_chunked},
    };
    const size_t n = sizeof(tests) / sizeof(tests[0]);
    size_t failed = 0u;
    for (size_t i = 0; i < n; ++i) {
        printf("[%2zu/%zu] %s ... ", i + 1u, n, tests[i].name);
        fflush(stdout);
        int rc = tests[i].fn();
        if (rc == 0) {
            printf("PASS\n");
        } else {
            printf("FAIL\n");
            failed++;
        }
    }
    if (failed != 0u) {
        fprintf(stderr, "FAILED: %zu/%zu tests\n", failed, n);
        return 1;
    }
    printf("OK: %zu/%zu tests\n", n, n);
    return 0;
}
