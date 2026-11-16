/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
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
 * A malformed value ABORTS rather than falling back to a guess. A run that
 * quietly used another port after being told one would bind the port the other
 * tree is using and report the collision as a test failure in whichever tree
 * lost -- the misattribution this harness exists to avoid.
 *
 * WHO NEEDS A PORT NAMED, AND WHO DOES NOT
 * ----------------------------------------
 * A listener in the SAME PROCESS as the machine it serves (the C++ runner's
 * W3CHttpTestServer, for the Interpreter and the C++ AOT runs) has no reason to
 * be told a port. When SCE_W3C_HTTP_PORT is unset it binds one the kernel
 * assigns (sce_w3c_http_in_process_port answers 0), and the location the
 * machine is handed is read from that listener after it binds
 * (W3CHttpTestServer::accessUri), never recomputed from the environment. The
 * Kotlin harness's W3CHttpTestServer and the Python conftest's listener are
 * such listeners too, and bind the same way. Such a run cannot collide with
 * another tree, or with another account's process on the host: measured
 * 2026-11-05, an account holding 8080 made the C++ runner's every BasicHTTP
 * test fail to start. When the variable IS set, that port is bound exactly, as
 * before.
 *
 * A listener in ANOTHER PROCESS is the Node server (standalone_http_server.js)
 * that the C11 AOT runners (sce_w3c_http_test_access_uri), the Rust and Go
 * harnesses and the WASM run post to, started by a ctest fixture, a gate, the
 * `verify` action or the WASM host. It has no default port either. Whoever
 * starts it asks for port 0, unless SCE_W3C_HTTP_PORT names one; the server
 * names the port it bound on its `[READY] port=<n>` line, and whoever started it
 * hands <n> to the processes that post to it:
 *   - through SCE_W3C_HTTP_PORT, when it starts them afterwards (a gate, the
 *     `verify` action, the WASM host, which sets it in its own environment
 *     before the program's main());
 *   - through a file holding the port, named by SCE_W3C_HTTP_PORT_FILE, when
 *     their environment was fixed before the server started: ctest fixes each
 *     test's environment when the tree is configured, so the C11 fixture writes
 *     the port it was told into that file (tests/w3c/http_server_fixture.sh).
 * A consumer told neither ABORTS, naming both (sce_w3c_http_test_port): there
 * is no port it could assume, since one it assumed is one another program on
 * the host may hold, and a fixed default made every lane need that port free.
 * The port file wins over SCE_W3C_HTTP_PORT, because it names the listener this
 * run started, where the variable may be left over from the caller's shell.
 */

#include <stdio.h>
#include <stdlib.h>

/* The one name a host, a gate, or a CI job sets to move the endpoint. */
#define SCE_W3C_HTTP_PORT_ENV "SCE_W3C_HTTP_PORT"

/* The name of a file holding the port an out-of-process listener bound, for a
 * consumer whose environment was fixed before that listener started. */
#define SCE_W3C_HTTP_PORT_FILE_ENV "SCE_W3C_HTTP_PORT_FILE"

#define SCE_W3C_HTTP_TEST_PATH "/test"

/* The host every listener binds and every published location names. */
#define SCE_W3C_HTTP_TEST_HOST "localhost"

/* The published location, from a port and a path: one format for the listener
 * that publishes its bound port and for the one whose port was agreed. */
#define SCE_W3C_HTTP_URI_FORMAT "http://" SCE_W3C_HTTP_TEST_HOST ":%d%s"

/* Longest string sce_w3c_http_test_access_uri can produce, plus its NUL:
 * "http://localhost:" (17) + 5 port digits + "/test" (5) + 1. */
#define SCE_W3C_HTTP_URI_MAX 28

/* `raw` read as a TCP port, or an abort naming `source`, where it came from. */
static inline int sce_w3c_http_parse_port(const char *raw, const char *source) {
    char *end = NULL;
    long value = strtol(raw, &end, 10);

    if (end == raw || end == NULL || *end != '\0' || value < 1 || value > 65535) {
        fprintf(stderr,
                "%s holds \"%s\", which is not a TCP port. The W3C BasicHTTP "
                "fixture endpoint is set there and has no second opinion: "
                "continuing on another port would post where no listener of "
                "this run answers.\n",
                source, raw);
        abort();
    }
    return (int)value;
}

/* The port SCE_W3C_HTTP_PORT names, or `when_unset` when it names none. */
static inline int sce_w3c_http_named_port(int when_unset) {
    const char *raw = getenv(SCE_W3C_HTTP_PORT_ENV);

    if (raw == NULL || raw[0] == '\0') {
        return when_unset;
    }
    return sce_w3c_http_parse_port(raw, SCE_W3C_HTTP_PORT_ENV);
}

/* The port written in the file at `path`: digits, and at most one trailing
 * newline. An unreadable or malformed file aborts: its listener said where it
 * answers, and a consumer that cannot read it cannot know. */
static inline int sce_w3c_http_port_in_file(const char *path) {
    char text[16];
    size_t length;
    FILE *file = fopen(path, "r");

    if (file == NULL) {
        fprintf(stderr,
                "%s names %s, which cannot be read: the out-of-process W3C "
                "BasicHTTP listener did not write the port it bound there, so "
                "it did not start for this run.\n",
                SCE_W3C_HTTP_PORT_FILE_ENV, path);
        abort();
    }
    length = fread(text, 1, sizeof text - 1, file);
    fclose(file);
    text[length] = '\0';
    if (length > 0 && text[length - 1] == '\n') {
        text[length - 1] = '\0';
    }
    return sce_w3c_http_parse_port(text, path);
}

/* The port an out-of-process fixture listener bound and the published location
 * names: the one the port file holds, or the one SCE_W3C_HTTP_PORT names. A
 * consumer told neither aborts: there is no default to assume. */
static inline int sce_w3c_http_test_port(void) {
    const char *file = getenv(SCE_W3C_HTTP_PORT_FILE_ENV);
    int named;

    if (file != NULL && file[0] != '\0') {
        return sce_w3c_http_port_in_file(file);
    }
    named = sce_w3c_http_named_port(0);
    if (named == 0) {
        fprintf(stderr,
                "neither %s nor %s says where the W3C BasicHTTP fixture "
                "listener answers. Start it with "
                "`node tests/w3c/standalone_http_server.js 0` and set %s to the "
                "port its `[READY] port=<n>` line names. It has no default "
                "port: one assumed is one another program on this host may "
                "hold.\n",
                SCE_W3C_HTTP_PORT_FILE_ENV, SCE_W3C_HTTP_PORT_ENV, SCE_W3C_HTTP_PORT_ENV);
        abort();
    }
    return named;
}

/* The port an in-process fixture listener binds: the one SCE_W3C_HTTP_PORT
 * names, or 0 for one the kernel assigns. The location it publishes is read
 * from the listener after it binds, not from here. */
static inline int sce_w3c_http_in_process_port(void) {
    return sce_w3c_http_named_port(0);
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
    int written = snprintf(buf, size, SCE_W3C_HTTP_URI_FORMAT, sce_w3c_http_test_port(), sce_w3c_http_test_path());
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
