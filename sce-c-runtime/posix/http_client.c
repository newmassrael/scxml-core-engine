// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML C.2 — host-side HTTP/1.1 client + JSON response extractor
// for the C11 backend BasicHTTPEventProcessor corpus. See
// `http_client.h` for the contract; this file implements the surface
// against POSIX socket(2)/connect(2)/send(2)/recv(2) — no third-party
// dep. Mirrors the Go `net/http` and Rust `reqwest::blocking` shapes
// used by `sce-go-tests/harness/harness.go::SetupHTTPTest` and
// `sce-rust-tests/src/harness.rs::setup_http_test`, condensed to the
// W3C C.2 corpus's actual needs (POST only, plain HTTP/1.1, no
// chunked transfer, no TLS).
//
// Implementation choices forced by the corpus shape:
//   * `Connection: close` request header → server signals EOF via
//     socket close, no chunked-transfer parser needed.
//   * Status line + headers buffered into a stack array (4 KiB cap);
//     the Node standalone server's responses are well under that.
//   * Body read into a malloc'd buffer that grows by doubling;
//     pre-sized from `Content-Length` when present.
//   * JSON extractor is a hand-rolled top-level scanner — only `event`
//     and `data` keys are recognised; depth-counted object/array
//     spans for `data` are captured raw for downstream Lua re-parse.

#define _POSIX_C_SOURCE 200809L
/* `strncasecmp` is POSIX-only and lives in <strings.h>; the
   `_POSIX_C_SOURCE 200809L` define above gates the strict-stdc
   compiler off. */

#include <sce/http_client.h>

#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/types.h>
#include <unistd.h>

/* Internal-only: cap stack-allocated header read buffer. The Node
   standalone server emits ≤ 200 bytes of headers in practice; 4 KiB
   leaves substantial headroom while keeping the buffer stack-bound. */
#define SCE_HTTP_HEADER_CAP 4096

/* Internal-only: initial body buffer capacity when no Content-Length
   header is present. Doubles on demand inside the read loop. */
#define SCE_HTTP_BODY_INITIAL_CAP 1024

/* ── URL parsing ────────────────────────────────────────────────── */

bool sce_test_http_url_parse(const char *url, sce_test_http_url_t *out) {
    if (url == NULL || out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));

    /* Reject https:// — TLS not supported. The W3C C.2 corpus only
       targets plain HTTP. */
    static const char http_prefix[] = "http://";
    static const size_t http_prefix_len = sizeof(http_prefix) - 1u;
    if (strncmp(url, http_prefix, http_prefix_len) != 0) {
        return false;
    }
    const char *cursor = url + http_prefix_len;

    /* Host: bytes up to the first `:` (port) or `/` (path) or EOL. */
    const char *host_end = cursor;
    while (*host_end != '\0' && *host_end != ':' && *host_end != '/') {
        host_end++;
    }
    size_t host_len = (size_t)(host_end - cursor);
    if (host_len == 0u || host_len >= sizeof(out->host)) {
        return false;
    }
    memcpy(out->host, cursor, host_len);
    out->host[host_len] = '\0';
    cursor = host_end;

    /* Port: optional `:N` segment after the host. Defaults to 80. */
    if (*cursor == ':') {
        cursor++;
        char *port_end = NULL;
        long port_val = strtol(cursor, &port_end, 10);
        if (port_end == cursor || port_val <= 0 || port_val > 65535) {
            return false;
        }
        out->port = (uint16_t)port_val;
        cursor = port_end;
    } else {
        out->port = 80u;
    }

    /* Path: everything from the first `/` to the end. Empty path
       defaults to `/` (HTTP/1.1 requires a request-target). */
    if (*cursor == '\0') {
        out->path[0] = '/';
        out->path[1] = '\0';
        return true;
    }
    if (*cursor != '/') {
        return false;
    }
    size_t path_len = strlen(cursor);
    if (path_len >= sizeof(out->path)) {
        return false;
    }
    memcpy(out->path, cursor, path_len);
    out->path[path_len] = '\0';
    return true;
}

/* ── Socket helpers ─────────────────────────────────────────────── */

/* Set both SO_RCVTIMEO and SO_SNDTIMEO. Caller passes -1 to skip. */
static bool sce_http_set_timeouts(int fd, int timeout_ms) {
    if (timeout_ms <= 0) {
        return true;
    }
    struct timeval tv;
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
    if (setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv)) < 0) {
        return false;
    }
    if (setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv)) < 0) {
        return false;
    }
    return true;
}

/* Connect to (host, port) using getaddrinfo + the first result that
   accepts. Returns an open socket fd or -1. */
static int sce_http_connect(const char *host, uint16_t port, int timeout_ms) {
    char port_str[8];
    int written = snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    if (written < 0 || (size_t)written >= sizeof(port_str)) {
        return -1;
    }

    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_protocol = IPPROTO_TCP;

    struct addrinfo *result = NULL;
    int rc = getaddrinfo(host, port_str, &hints, &result);
    if (rc != 0 || result == NULL) {
        return -1;
    }

    int fd = -1;
    for (struct addrinfo *ai = result; ai != NULL; ai = ai->ai_next) {
        fd = socket(ai->ai_family, ai->ai_socktype, ai->ai_protocol);
        if (fd < 0) {
            continue;
        }
        if (!sce_http_set_timeouts(fd, timeout_ms)) {
            close(fd);
            fd = -1;
            continue;
        }
        if (connect(fd, ai->ai_addr, ai->ai_addrlen) == 0) {
            break;
        }
        close(fd);
        fd = -1;
    }
    freeaddrinfo(result);
    return fd;
}

/* Send the entire buffer; loop on partial writes. Returns false on
   error or EOF. Mirrors the cpp httplib internal `write_all` shape. */
static bool sce_http_send_all(int fd, const char *buf, size_t len) {
    size_t sent = 0;
    while (sent < len) {
        ssize_t n = send(fd, buf + sent, len - sent, 0);
        if (n <= 0) {
            if (n < 0 && errno == EINTR) {
                continue;
            }
            return false;
        }
        sent += (size_t)n;
    }
    return true;
}

/* ── HTTP/1.1 response reader ───────────────────────────────────── */

/* Read until `\r\n\r\n` (end of headers) or buffer overflow. Returns
   the number of bytes consumed for the header portion; the leftover
   bytes (which start the body) are copied into `*body_prefix`/
   `*body_prefix_len` so the body reader can pick up where the
   header reader stopped. */
static ssize_t sce_http_read_headers(int fd, char *header_buf, size_t header_cap, char **body_prefix,
                                     size_t *body_prefix_len) {
    size_t total = 0;
    while (total + 1 < header_cap) {
        ssize_t n = recv(fd, header_buf + total, header_cap - total - 1u, 0);
        if (n < 0) {
            if (errno == EINTR) {
                continue;
            }
            return -1;
        }
        if (n == 0) {
            /* EOF before headers ended — server closed prematurely. */
            return -1;
        }
        total += (size_t)n;
        header_buf[total] = '\0';
        char *end = strstr(header_buf, "\r\n\r\n");
        if (end != NULL) {
            size_t header_len = (size_t)(end - header_buf) + 4u;
            *body_prefix = header_buf + header_len;
            *body_prefix_len = total - header_len;
            return (ssize_t)header_len;
        }
    }
    /* Header section exceeded the cap — pathological response. */
    return -1;
}

/* Case-insensitive substring search restricted to one header line.
   Returns a pointer into `header_block` past the colon-and-space
   separator, or NULL when not found. Mutates nothing. */
static const char *sce_http_find_header(const char *header_block, size_t block_len, const char *name) {
    size_t name_len = strlen(name);
    const char *cursor = header_block;
    const char *end = header_block + block_len;

    while (cursor + name_len + 1u < end) {
        if (strncasecmp(cursor, name, name_len) == 0 && cursor[name_len] == ':') {
            const char *value = cursor + name_len + 1u;
            while (value < end && (*value == ' ' || *value == '\t')) {
                value++;
            }
            return value;
        }
        /* Skip to next line. */
        const char *eol = memchr(cursor, '\n', (size_t)(end - cursor));
        if (eol == NULL) {
            return NULL;
        }
        cursor = eol + 1;
    }
    return NULL;
}

/* Parse `HTTP/1.1 <code> <reason>` status line. Returns the integer
   code, or -1 on parse failure. */
static int sce_http_parse_status(const char *header_block) {
    if (strncmp(header_block, "HTTP/1.", 7) != 0) {
        return -1;
    }
    const char *space = strchr(header_block, ' ');
    if (space == NULL) {
        return -1;
    }
    char *end = NULL;
    long code = strtol(space + 1, &end, 10);
    if (end == space + 1 || code < 100 || code > 999) {
        return -1;
    }
    return (int)code;
}

/* Read the response body into a malloc'd buffer. Honours Content-
   Length when present; otherwise drains until EOF. `prefix` carries
   bytes already read by the header reader and prepended to the
   buffer before any further recv(). */
static bool sce_http_read_body(int fd, const char *prefix, size_t prefix_len, long content_length, /* -1 when unknown */
                               char **out_body, size_t *out_len) {
    size_t cap = (content_length > 0) ? (size_t)content_length + 1u : SCE_HTTP_BODY_INITIAL_CAP;
    char *buf = malloc(cap);
    if (buf == NULL) {
        return false;
    }
    size_t len = 0;
    if (prefix_len > 0) {
        memcpy(buf, prefix, prefix_len);
        len = prefix_len;
    }

    while (true) {
        if (content_length >= 0 && len >= (size_t)content_length) {
            break;
        }
        if (len + 1u >= cap) {
            size_t new_cap = cap * 2u;
            char *grown = realloc(buf, new_cap);
            if (grown == NULL) {
                free(buf);
                return false;
            }
            buf = grown;
            cap = new_cap;
        }
        ssize_t n = recv(fd, buf + len, cap - len - 1u, 0);
        if (n < 0) {
            if (errno == EINTR) {
                continue;
            }
            free(buf);
            return false;
        }
        if (n == 0) {
            /* EOF — Connection: close or Content-Length boundary. */
            break;
        }
        len += (size_t)n;
    }
    buf[len] = '\0';
    *out_body = buf;
    *out_len = len;
    return true;
}

/* Decode RFC 7230 §4.1 chunked transfer encoding in place. Input is
   the raw byte stream `<size-hex>\r\n<chunk>\r\n...0\r\n\r\n` already
   accumulated into `buf` (length `*len`). Output is the concatenated
   chunk payload, length re-written into `*len`. The Node.js
   standalone server emits chunked because Express does not buffer
   the JSON response — the client side has to handle it.

   Returns false on truncation / malformed-size / missing CRLF; the
   caller treats false as transport failure. The buffer is mutated
   in place (output is strictly ≤ input size, so no re-allocation). */
static bool sce_http_dechunk_inplace(char *buf, size_t *len) {
    char *read = buf;
    char *write = buf;
    char *end = buf + *len;

    while (read < end) {
        /* Parse chunk size as hex. RFC 7230: extension `<size>;<ext>`
           is allowed but the standalone server does not emit it; we
           tolerate by stopping at the first non-hex char and skipping
           to the CRLF. */
        char *size_end = read;
        while (size_end < end && ((*size_end >= '0' && *size_end <= '9') || (*size_end >= 'a' && *size_end <= 'f') ||
                                  (*size_end >= 'A' && *size_end <= 'F'))) {
            size_end++;
        }
        if (size_end == read) {
            return false;
        }
        char saved = *size_end;
        *size_end = '\0';
        long chunk_size = strtol(read, NULL, 16);
        *size_end = saved;
        if (chunk_size < 0) {
            return false;
        }
        /* Skip optional `;ext` and the trailing `\r\n`. */
        char *crlf = memchr(size_end, '\n', (size_t)(end - size_end));
        if (crlf == NULL) {
            return false;
        }
        read = crlf + 1;
        if ((size_t)chunk_size == 0u) {
            /* Last chunk — trailing `\r\n` (and any trailers) ignored. */
            break;
        }
        if (read + chunk_size > end) {
            return false;
        }
        memmove(write, read, (size_t)chunk_size);
        write += chunk_size;
        read += chunk_size;
        /* Consume the `\r\n` separator after the chunk payload. */
        if (read + 1 < end && read[0] == '\r' && read[1] == '\n') {
            read += 2;
        } else if (read < end && read[0] == '\n') {
            read += 1;
        } else {
            return false;
        }
    }
    *len = (size_t)(write - buf);
    buf[*len] = '\0';
    return true;
}

/* ── Public API: synchronous POST ───────────────────────────────── */

bool sce_test_http_post(const sce_test_http_url_t *url, const char *content_type, const char *body, size_t body_len,
                        int timeout_ms, sce_test_http_response_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    if (url == NULL || content_type == NULL || (body == NULL && body_len > 0)) {
        return false;
    }

    int fd = sce_http_connect(url->host, url->port, timeout_ms);
    if (fd < 0) {
        return false;
    }

    /* Build request head. The W3C C.2 corpus's largest expected
       Content-Length is the JSON+content shape from test520
       (`<content>this is some content</content>` = ~20 bytes); the
       request head fits easily in 1 KiB. */
    char head[1024];
    int head_len = snprintf(head, sizeof(head),
                            "POST %s HTTP/1.1\r\n"
                            "Host: %s:%u\r\n"
                            "Content-Type: %s\r\n"
                            "Content-Length: %zu\r\n"
                            "Connection: close\r\n"
                            "\r\n",
                            url->path, url->host, (unsigned)url->port, content_type, body_len);
    if (head_len < 0 || (size_t)head_len >= sizeof(head)) {
        close(fd);
        return false;
    }

    if (!sce_http_send_all(fd, head, (size_t)head_len)) {
        close(fd);
        return false;
    }
    if (body_len > 0 && !sce_http_send_all(fd, body, body_len)) {
        close(fd);
        return false;
    }

    /* Read headers into stack buffer. */
    char header_buf[SCE_HTTP_HEADER_CAP];
    char *body_prefix = NULL;
    size_t body_prefix_len = 0u;
    ssize_t header_len = sce_http_read_headers(fd, header_buf, sizeof(header_buf), &body_prefix, &body_prefix_len);
    if (header_len < 0) {
        close(fd);
        return false;
    }

    int status_code = sce_http_parse_status(header_buf);
    if (status_code < 0) {
        close(fd);
        return false;
    }
    out->status_code = status_code;

    /* Two body-framing modes per RFC 7230 §3.3.3:
         (a) Content-Length: <N> → exactly N body bytes
         (b) Transfer-Encoding: chunked → `<hex-size>\r\n<chunk>\r\n...0\r\n\r\n`
       The Node.js standalone server picks chunked for JSON responses
       (Express auto-chunks because the body is computed in pieces);
       the unit-test stub server picks Content-Length. Both shapes
       must round-trip cleanly. */
    long content_length = -1;
    const char *cl = sce_http_find_header(header_buf, (size_t)header_len, "Content-Length");
    if (cl != NULL) {
        char *end = NULL;
        long val = strtol(cl, &end, 10);
        if (end != cl && val >= 0) {
            content_length = val;
        }
    }
    bool is_chunked = false;
    const char *te = sce_http_find_header(header_buf, (size_t)header_len, "Transfer-Encoding");
    if (te != NULL) {
        /* The header value may carry a list (`chunked, gzip` etc.);
           we only honour the bare `chunked` token. The standalone
           server emits `chunked` alone; gzip/deflate isn't in the
           host-helper R3 budget. */
        const char *eol = memchr(te, '\r', (size_t)((header_buf + header_len) - te));
        size_t te_len = (eol != NULL) ? (size_t)(eol - te) : (size_t)((header_buf + header_len) - te);
        for (size_t i = 0u; i + 7u <= te_len; i++) {
            if (strncasecmp(te + i, "chunked", 7u) == 0) {
                is_chunked = true;
                break;
            }
        }
    }

    char *body_buf = NULL;
    size_t body_buf_len = 0u;
    /* Drain raw bytes first; chunked uses size-prefixed framing inside
       the body so Content-Length is meaningless. When neither header
       is set, drain-until-EOF (server closes per `Connection: close`). */
    if (!sce_http_read_body(fd, body_prefix, body_prefix_len, is_chunked ? -1 : content_length, &body_buf,
                            &body_buf_len)) {
        close(fd);
        return false;
    }
    close(fd);

    if (is_chunked) {
        if (!sce_http_dechunk_inplace(body_buf, &body_buf_len)) {
            free(body_buf);
            return false;
        }
    }

    out->ok = true;
    out->body = body_buf;
    out->body_len = body_buf_len;
    return true;
}

void sce_test_http_response_free(sce_test_http_response_t *r) {
    if (r == NULL) {
        return;
    }
    free(r->body);
    memset(r, 0, sizeof(*r));
}

/* ── form-urlencoded body builder ───────────────────────────────── */

static bool sce_http_should_pct_encode(unsigned char c) {
    /* RFC 3986 unreserved set + spaces (handled separately). */
    if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9')) {
        return false;
    }
    if (c == '-' || c == '_' || c == '.' || c == '~') {
        return false;
    }
    return true;
}

/* Append `s` to `body[*body_len .. cap]`, percent-encoding per RFC
   3986 unreserved-set rules. Spaces become `+` (Go/Rust harness
   parity). Returns false on overflow without partial-writing. */
static bool sce_http_form_pct_append(char *body, size_t cap, size_t *body_len, const char *s) {
    static const char hex[] = "0123456789ABCDEF";
    size_t pos = *body_len;
    for (size_t i = 0; s[i] != '\0'; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c == ' ') {
            if (pos + 1u >= cap) {
                return false;
            }
            body[pos++] = '+';
            continue;
        }
        if (!sce_http_should_pct_encode(c)) {
            if (pos + 1u >= cap) {
                return false;
            }
            body[pos++] = (char)c;
            continue;
        }
        if (pos + 3u >= cap) {
            return false;
        }
        body[pos++] = '%';
        body[pos++] = hex[c >> 4];
        body[pos++] = hex[c & 0x0F];
    }
    *body_len = pos;
    body[pos] = '\0';
    return true;
}

bool sce_test_http_form_append(char *body, size_t cap, size_t *body_len, const char *key, const char *value) {
    if (body == NULL || body_len == NULL || key == NULL || value == NULL) {
        return false;
    }
    size_t pos = *body_len;
    if (pos > 0u) {
        if (pos + 1u >= cap) {
            return false;
        }
        body[pos++] = '&';
    }
    *body_len = pos;
    if (!sce_http_form_pct_append(body, cap, body_len, key)) {
        return false;
    }
    pos = *body_len;
    if (pos + 1u >= cap) {
        return false;
    }
    body[pos++] = '=';
    *body_len = pos;
    return sce_http_form_pct_append(body, cap, body_len, value);
}

/* ── JSON extractor (top-level event/data only) ─────────────────── */

/* Skip ASCII whitespace including \r\n. Returns the new cursor. */
static const char *sce_json_skip_ws(const char *p, const char *end) {
    while (p < end && (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n')) {
        p++;
    }
    return p;
}

/* Hex digit → 0-15 or -1. */
static int sce_json_hex(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return 10 + (c - 'a');
    }
    if (c >= 'A' && c <= 'F') {
        return 10 + (c - 'A');
    }
    return -1;
}

/* Encode codepoint as UTF-8 into `dst`, returning byte count
   written. Codepoints above U+10FFFF or surrogate-half values are
   clamped to U+FFFD. */
static size_t sce_json_utf8_encode(uint32_t cp, char *dst) {
    if (cp >= 0xD800 && cp <= 0xDFFF) {
        cp = 0xFFFD;
    } else if (cp > 0x10FFFF) {
        cp = 0xFFFD;
    }
    if (cp < 0x80) {
        dst[0] = (char)cp;
        return 1u;
    }
    if (cp < 0x800) {
        dst[0] = (char)(0xC0 | (cp >> 6));
        dst[1] = (char)(0x80 | (cp & 0x3F));
        return 2u;
    }
    if (cp < 0x10000) {
        dst[0] = (char)(0xE0 | (cp >> 12));
        dst[1] = (char)(0x80 | ((cp >> 6) & 0x3F));
        dst[2] = (char)(0x80 | (cp & 0x3F));
        return 3u;
    }
    dst[0] = (char)(0xF0 | (cp >> 18));
    dst[1] = (char)(0x80 | ((cp >> 12) & 0x3F));
    dst[2] = (char)(0x80 | ((cp >> 6) & 0x3F));
    dst[3] = (char)(0x80 | (cp & 0x3F));
    return 4u;
}

/* Walk a JSON string starting at `*cursor` (which points to the
   opening `"`). Unescapes in place into the same buffer (output is
   strictly ≤ input length, so no allocation needed). Advances
   `*cursor` past the closing `"`. Sets `*value_start` and `*value_len`
   to the unescaped span. Returns false on syntax error or unexpected
   EOF. */
static bool sce_json_unescape_string(char **cursor, char *end, char **value_start, size_t *value_len) {
    char *p = *cursor;
    if (p >= end || *p != '"') {
        return false;
    }
    p++;
    char *write = p;
    *value_start = p;

    while (p < end && *p != '"') {
        if (*p == '\\') {
            if (p + 1 >= end) {
                return false;
            }
            char esc = p[1];
            switch (esc) {
            case '"':
                *write++ = '"';
                p += 2;
                break;
            case '\\':
                *write++ = '\\';
                p += 2;
                break;
            case '/':
                *write++ = '/';
                p += 2;
                break;
            case 'b':
                *write++ = '\b';
                p += 2;
                break;
            case 'f':
                *write++ = '\f';
                p += 2;
                break;
            case 'n':
                *write++ = '\n';
                p += 2;
                break;
            case 'r':
                *write++ = '\r';
                p += 2;
                break;
            case 't':
                *write++ = '\t';
                p += 2;
                break;
            case 'u': {
                if (p + 6 > end) {
                    return false;
                }
                int h0 = sce_json_hex(p[2]);
                int h1 = sce_json_hex(p[3]);
                int h2 = sce_json_hex(p[4]);
                int h3 = sce_json_hex(p[5]);
                if (h0 < 0 || h1 < 0 || h2 < 0 || h3 < 0) {
                    return false;
                }
                uint32_t cp = (uint32_t)((h0 << 12) | (h1 << 8) | (h2 << 4) | h3);
                /* High surrogate followed by `\uDxxx` low surrogate
                   pairs into a supplementary plane codepoint. */
                if (cp >= 0xD800 && cp <= 0xDBFF && p + 12 <= end && p[6] == '\\' && p[7] == 'u') {
                    int l0 = sce_json_hex(p[8]);
                    int l1 = sce_json_hex(p[9]);
                    int l2 = sce_json_hex(p[10]);
                    int l3 = sce_json_hex(p[11]);
                    if (l0 < 0 || l1 < 0 || l2 < 0 || l3 < 0) {
                        return false;
                    }
                    uint32_t low = (uint32_t)((l0 << 12) | (l1 << 8) | (l2 << 4) | l3);
                    if (low >= 0xDC00 && low <= 0xDFFF) {
                        cp = 0x10000u + ((cp - 0xD800u) << 10) + (low - 0xDC00u);
                        p += 12;
                    } else {
                        p += 6;
                    }
                } else {
                    p += 6;
                }
                write += sce_json_utf8_encode(cp, write);
                break;
            }
            default:
                return false;
            }
        } else {
            *write++ = *p++;
        }
    }
    if (p >= end) {
        return false;
    }
    *value_len = (size_t)(write - *value_start);
    /* In-place unescape collapses ≤ input bytes; pad the gap with
       NUL so callers reading past `value_len` see a terminated
       string. The original buffer was malloc'd by the HTTP reader
       with a trailing NUL slot, so this is safe up to body_len + 1
       bytes. */
    if (write < p) {
        *write = '\0';
    }
    *cursor = p + 1; /* past closing `"` */
    return true;
}

/* Capture the bytes from `*cursor` (pointing at `{` or `[`) through
   the matching closer, depth-counted, ignoring matched characters
   inside JSON strings. Returns false on truncation/syntax error.
   Sets `*value_start` and `*value_len` to span the raw bytes
   including both delimiters. */
static bool sce_json_capture_object(char **cursor, char *end, char **value_start, size_t *value_len) {
    char *p = *cursor;
    *value_start = p;
    char open = *p;
    char close = (open == '{') ? '}' : ']';
    int depth = 1;
    p++;
    while (p < end && depth > 0) {
        if (*p == '"') {
            /* Skip a JSON string without unescaping (we just want to
               find its terminator). */
            p++;
            while (p < end && *p != '"') {
                if (*p == '\\' && p + 1 < end) {
                    p += 2;
                } else {
                    p++;
                }
            }
            if (p >= end) {
                return false;
            }
            p++;
            continue;
        }
        if (*p == '{' || *p == '[') {
            depth++;
        } else if (*p == '}' || *p == ']') {
            depth--;
            if (depth == 0) {
                if (*p != close) {
                    return false;
                }
                p++;
                *value_len = (size_t)(p - *value_start);
                *cursor = p;
                return true;
            }
        }
        p++;
    }
    return false;
}

/* Skip a JSON value (string / number / true/false/null / object /
   array). Returns false on syntax error. */
static bool sce_json_skip_value(char **cursor, char *end) {
    char *p = *cursor;
    if (p >= end) {
        return false;
    }
    if (*p == '"') {
        char *unused_start = NULL;
        size_t unused_len = 0u;
        return sce_json_unescape_string(cursor, end, &unused_start, &unused_len);
    }
    if (*p == '{' || *p == '[') {
        char *unused_start = NULL;
        size_t unused_len = 0u;
        return sce_json_capture_object(cursor, end, &unused_start, &unused_len);
    }
    /* Number / true / false / null: scan to whitespace, comma, `}`,
       `]`. */
    while (p < end && *p != ',' && *p != '}' && *p != ']' && *p != ' ' && *p != '\t' && *p != '\r' && *p != '\n') {
        p++;
    }
    *cursor = p;
    return true;
}

bool sce_test_http_parse_response(char *body, size_t body_len, sce_test_http_json_response_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    if (body == NULL) {
        return false;
    }

    char *p = body;
    char *end = body + body_len;
    p = (char *)sce_json_skip_ws(p, end);
    if (p >= end || *p != '{') {
        return false;
    }
    p++;

    bool seen_event = false;

    while (p < end) {
        p = (char *)sce_json_skip_ws(p, end);
        if (p < end && *p == '}') {
            break;
        }

        char *key_start = NULL;
        size_t key_len = 0u;
        if (!sce_json_unescape_string(&p, end, &key_start, &key_len)) {
            return false;
        }
        p = (char *)sce_json_skip_ws(p, end);
        if (p >= end || *p != ':') {
            return false;
        }
        p++;
        p = (char *)sce_json_skip_ws(p, end);
        if (p >= end) {
            return false;
        }

        bool match_event = (key_len == 5u && memcmp(key_start, "event", 5u) == 0);
        bool match_data = (key_len == 4u && memcmp(key_start, "data", 4u) == 0);

        if (match_event) {
            if (*p != '"') {
                return false;
            }
            char *val_start = NULL;
            size_t val_len = 0u;
            if (!sce_json_unescape_string(&p, end, &val_start, &val_len)) {
                return false;
            }
            out->event_name = val_start;
            out->event_name_len = val_len;
            seen_event = true;
        } else if (match_data) {
            if (*p == '"') {
                char *val_start = NULL;
                size_t val_len = 0u;
                if (!sce_json_unescape_string(&p, end, &val_start, &val_len)) {
                    return false;
                }
                out->event_data = val_start;
                out->event_data_len = val_len;
                out->data_is_string = true;
            } else if (*p == '{' || *p == '[') {
                char *val_start = NULL;
                size_t val_len = 0u;
                if (!sce_json_capture_object(&p, end, &val_start, &val_len)) {
                    return false;
                }
                out->event_data = val_start;
                out->event_data_len = val_len;
                out->data_is_string = false;
            } else {
                /* Number / true/false/null — capture raw span by
                   scanning to value end, then take the slice. */
                char *val_start = p;
                if (!sce_json_skip_value(&p, end)) {
                    return false;
                }
                out->event_data = val_start;
                out->event_data_len = (size_t)(p - val_start);
                out->data_is_string = false;
            }
        } else {
            /* Unknown key — skip the value. */
            if (!sce_json_skip_value(&p, end)) {
                return false;
            }
        }

        p = (char *)sce_json_skip_ws(p, end);
        if (p < end && *p == ',') {
            p++;
            continue;
        }
        if (p < end && *p == '}') {
            break;
        }
        if (p >= end) {
            return false;
        }
    }

    if (!seen_event) {
        return false;
    }
    out->ok = true;
    return true;
}
