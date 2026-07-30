// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// §scxml-C-2 — host-side HTTP client for the C11 backend's
// BasicHTTPEventProcessor conformance corpus (test201/509/510/513/518/
// 520/522/531/532/534/567).
//
// Client-only design: the fixture binary issues a synchronous HTTP
// POST against the existing
// `tests/w3c/standalone_http_server.js` Node.js server (already used by
// the Go and Rust harnesses + cpp WASM lane). The server JSON-echoes
// each inbound request as
//   `{"status":"success","event":"<name>","data":<json|string>,
//    "timestamp":<num>}`
// and the client extracts the `event` + `data` keys for re-injection
// into the SM via `<sm>_raise_external`. No in-process server is
// required, so this header carries client-side primitives only.
//
// Host-side helper, isolated to the `sce_c_test_http_support` STATIC
// archive consumed only by fixture binaries. sce-c-runtime stays
// untouched (zero-deps profile preserved bit-exact).
//
// Surface mirrors what `setup_http_test()` does in
// `backends/rust/tests/src/harness.rs:130-189` and
// `backends/go/tests/harness/harness.go::SetupHTTPTest`, reimplemented in
// pure C with POSIX socket + recursive-descent JSON extractor. No
// libcurl / cpp-httplib dep (cpp-httplib is C++ only; libcurl is too
// heavy for a test-fixture-only helper).

#ifndef SCE_C_TESTS_SUPPORT_HTTP_CLIENT_H
#define SCE_C_TESTS_SUPPORT_HTTP_CLIENT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── URL parsing ────────────────────────────────────────────────── */

/**
 * Parsed `http://<host>:<port><path>` URL components.
 *
 * Inline buffers (no malloc) because the §scxml-C-2 corpus uses
 * literal `http://localhost:8080/test` exclusively (TXMLConverter.h:
 * `HTTP_TEST_SERVER_URL`); 128/256-byte caps are far above the
 * fixture URLs while keeping the struct stack-allocatable inside a
 * generated `_perform_http_send_*` block.
 */
typedef struct {
    char host[128];
    uint16_t port;
    char path[256];
} sce_test_http_url_t;

/**
 * Parse an `http://` URL into host/port/path components. `https://`
 * is rejected — the W3C C.2 corpus never targets TLS, and embedding a
 * TLS stack would defeat the helper's minimal-footprint design.
 *
 * @return true on success; false on scheme/syntax error or buffer
 *         overflow (host > 127 bytes, path > 255 bytes).
 */
bool sce_test_http_url_parse(const char *url, sce_test_http_url_t *out);

/* ── Synchronous HTTP/1.1 POST ──────────────────────────────────── */

/**
 * Result of an HTTP POST. `body` is malloc'd; pass to
 * `sce_test_http_response_free` regardless of the `ok` flag (the
 * struct is initialised even on transport failure so the cleanup
 * path is uniform).
 */
typedef struct {
    bool ok;         /* transport-level success — set even on 4xx/5xx */
    int status_code; /* parsed `HTTP/1.1 <code>` status; 0 if untransmitted */
    char *body;      /* malloc'd, may be empty string but not NULL when ok */
    size_t body_len; /* bytes in body, excluding the trailing NUL */
} sce_test_http_response_t;

/**
 * Issue a synchronous HTTP/1.1 POST.
 *
 * Builds the request as
 *   POST <path> HTTP/1.1\r\n
 *   Host: <host>:<port>\r\n
 *   Content-Type: <content_type>\r\n
 *   Content-Length: <body_len>\r\n
 *   Connection: close\r\n
 *   \r\n
 *   <body>
 *
 * The `Connection: close` header lets the server signal end-of-body
 * by closing the socket — simplifies the body reader to "drain until
 * EOF" without ever needing chunked-transfer parsing (the standalone
 * Node server emits Content-Length on every response anyway, so the
 * reader honours that header when present and falls back to EOF).
 *
 * @param url           target host/port/path
 * @param content_type  MIME for the request body
 *                      (e.g. `application/x-www-form-urlencoded` or
 *                      `text/plain`)
 * @param body          POST body bytes (may be NULL when body_len == 0)
 * @param body_len      number of bytes in `body`
 * @param timeout_ms    socket-level timeout for connect/send/recv;
 *                      W3C C.2 corpus uses <send delay="3s"/> safety
 *                      nets, so 5000 ms is the textbook choice
 *                      (mirrors cpp `HttpEventTarget` default)
 * @param out           response sink — populated even on `false`
 *                      return so the cleanup contract is uniform
 *
 * @return true when a response was received and parsed; false on
 *         socket error, parse failure, or timeout. `status_code` is
 *         set whenever the status line was decoded, regardless of
 *         the 2xx/non-2xx split.
 */
bool sce_test_http_post(const sce_test_http_url_t *url, const char *content_type, const char *body, size_t body_len,
                        int timeout_ms, sce_test_http_response_t *out);

/** Release the `body` buffer of `r` and reset the struct to zero. */
void sce_test_http_response_free(sce_test_http_response_t *r);

/* ── form-urlencoded body builder ───────────────────────────────── */

/**
 * Append a `key=value` pair to a `application/x-www-form-urlencoded`
 * body buffer. Inserts a leading `&` separator when `*body_len > 0`,
 * percent-encodes both key and value per RFC 3986 reserved-set rules
 * (preserves `A-Za-z0-9-_.~`, `+` for spaces, `%XX` everywhere else
 * — matches Go `net/url.Values.Encode` and Rust `urlencoding` shape).
 *
 * `cap` is the capacity of `body`; on overflow returns false without
 * partial-writing. Caller-owned buffer; helper never realloc's.
 *
 * Used by the codegen-emitted `<send>` body builder to fold event
 * names (`_scxmleventname=<name>`) and `<param>` values into the
 * POST body the same way Go/Rust harnesses do.
 */
bool sce_test_http_form_append(char *body, size_t cap, size_t *body_len, const char *key, const char *value);

/* ── Standalone server response JSON extractor ──────────────────── */

/**
 * Parsed `{"event": "...", "data": ...}` projection of the
 * standalone server's JSON response. Pointers into the response body
 * — caller must keep that buffer alive for the lifetime of this
 * struct.
 *
 * `data_*` is NULL/0 when the JSON has no `data` key. When `data` is
 * a string ("data": "<text>"), `data_is_string` is true and the
 * JSON-string is unescaped in place into the response body buffer
 * before the pointer is set (the buffer is mutable: the caller passes
 * its own malloc'd bytes from `sce_test_http_post`).
 *
 * When `data` is an object/array, `data_is_string` is false and the
 * pointer/length spans the raw JSON-bytes (depth-balanced); the
 * fixture-side codegen lifts that span into a Lua table via
 * `_pending_donedata`.
 */
typedef struct {
    bool ok;
    const char *event_name; /* points into response_body */
    size_t event_name_len;
    const char *event_data; /* points into response_body, NULL if absent */
    size_t event_data_len;
    bool data_is_string; /* true when "data" was a JSON string */
} sce_test_http_json_response_t;

/**
 * Extract `event` (string) and `data` (string OR object) from the
 * top-level JSON object in `body`. Mutates the buffer in place when
 * `event` or `data` is a string (in-place unescape collapses
 * `\"`/`\\`/`\n`/`\t`/`\uXXXX` → UTF-8 + writes a NUL terminator the
 * caller can read past the returned `len`).
 *
 * `body` must be writable (the host-side `sce_test_http_response_t`
 * carries malloc'd bytes — that satisfies the contract). `body_len`
 * is the byte count excluding any trailing NUL.
 *
 * Tolerates arbitrary whitespace, ignores unknown top-level keys,
 * does not validate sibling-key ordering. Recovers via depth counting
 * on object/array values so `"data": {"x": "}}}"}` does not split
 * early on the embedded `}`.
 *
 * @return false when the body is not a top-level JSON object, or
 *         when `event` is missing — both indicate a server error
 *         that the fixture surface treats as transport failure.
 */
bool sce_test_http_parse_response(char *body, size_t body_len, sce_test_http_json_response_t *out);

#ifdef __cplusplus
}
#endif

#endif /* SCE_C_TESTS_SUPPORT_HTTP_CLIENT_H */
