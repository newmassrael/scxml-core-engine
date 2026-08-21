/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

#ifndef SCE_W3C_BASIC_HTTP_TEST_ENDPOINT_H
#define SCE_W3C_BASIC_HTTP_TEST_ENDPOINT_H

/*
 * §scxml-C-2-3: where the harness's inbound BasicHTTP listener answers — the
 * ONE place that decides it, for every channel.
 *
 * The same address is handed to a state machine as the BasicHTTP processor's
 * published 'location', and the converted W3C documents read that entry to
 * address their sends. Bind parameters and published address are therefore one
 * fact: a document that posts somewhere the listener never claimed would fail
 * for a reason unrelated to what it tests.
 *
 * WHY THIS IS A C HEADER, AND WHY THE PORT IS NOT A CONSTANT
 * ---------------------------------------------------------
 * It used to be a C++ `constexpr` in BasicHttpTestEndpoint.h, which put the
 * fact out of reach of every other channel: the C11 AOT runners could not
 * include it, so each of them wrote "http://localhost:8080/test" again, and the
 * gates and CI workflows wrote the number a third and fourth time. One fact,
 * spelled independently in five places, is the shape this repository has been
 * bitten by before — so the fact lives here, in the one language every channel
 * can read, and C++ ergonomics are a thin wrapper in BasicHttpTestEndpoint.h.
 *
 * A constant also made the port a property of the SOURCE rather than of the
 * RUN. The listener is a machine-global resource: only one process on a host
 * can hold it, so two checkouts of this repository cannot test at the same time
 * while both compile the same number in. Reading it from the environment is
 * what lets a second tree be given a different one.
 *
 * A malformed value ABORTS rather than falling back to the default. A run that
 * quietly used 8080 after being told to use something else would bind the port
 * the other tree is using and report the collision as a test failure in
 * whichever tree lost -- the misattribution this harness exists to avoid.
 */

#include <stdio.h>
#include <stdlib.h>

/* The one name a host, a gate, or a CI job sets to move the endpoint. */
#define SCE_W3C_HTTP_PORT_ENV "SCE_W3C_HTTP_PORT"

/* What the endpoint is when nothing says otherwise. Kept as the historical
 * value so a tree that sets nothing behaves exactly as it did. */
#define SCE_W3C_HTTP_DEFAULT_PORT 8080
#define SCE_W3C_HTTP_TEST_PATH "/test"

/* Longest string sce_w3c_http_test_access_uri can produce, plus its NUL:
 * "http://localhost:" (17) + 5 port digits + "/test" (5) + 1. */
#define SCE_W3C_HTTP_URI_MAX 28

/* The port the fixture listener binds and the published location names. */
static inline int sce_w3c_http_test_port(void) {
    const char *raw = getenv(SCE_W3C_HTTP_PORT_ENV);
    char *end;
    long value;

    if (raw == NULL || raw[0] == '\0') {
        return SCE_W3C_HTTP_DEFAULT_PORT;
    }

    end = NULL;
    value = strtol(raw, &end, 10);
    if (end == NULL || *end != '\0' || value < 1 || value > 65535) {
        fprintf(stderr,
                "%s=\"%s\" is not a TCP port. The W3C BasicHTTP fixture "
                "endpoint is set by this variable and has no second opinion: "
                "continuing on the default would bind a port this run was told "
                "not to use.\n",
                SCE_W3C_HTTP_PORT_ENV, raw);
        abort();
    }
    return (int)value;
}

/* The path the fixture listener answers on. Fixed: the documents address it by
 * the published location, so nothing outside this header needs to vary it. */
static inline const char *sce_w3c_http_test_path(void) {
    return SCE_W3C_HTTP_TEST_PATH;
}

/* Writes the published BasicHTTP location into `buf` and returns it, so the
 * caller owns the storage and this header needs no allocator. `size` must be at
 * least SCE_W3C_HTTP_URI_MAX. */
static inline const char *sce_w3c_http_test_access_uri(char *buf, size_t size) {
    int written = snprintf(buf, size, "http://localhost:%d%s", sce_w3c_http_test_port(), sce_w3c_http_test_path());
    if (written < 0 || (size_t)written >= size) {
        fprintf(stderr,
                "sce_w3c_http_test_access_uri: %zu-byte buffer cannot hold the "
                "endpoint URI; SCE_W3C_HTTP_URI_MAX is %d\n",
                size, SCE_W3C_HTTP_URI_MAX);
        abort();
    }
    return buf;
}

#endif /* SCE_W3C_BASIC_HTTP_TEST_ENDPOINT_H */
