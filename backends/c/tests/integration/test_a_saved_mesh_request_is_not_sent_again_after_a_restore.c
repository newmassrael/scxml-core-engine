// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a Mesh request (`<invoke type="sce:mesh-rpc">`)
// has one request in flight, and the peer it reached may already have acted on it. A saved state
// holds the call as it was sent; a restore does NOT send it again, because the peer would act on
// it twice, and the router that carried it is gone with the process that saved it. The document is
// told on the first macrostep that the call was interrupted (`error.invoke.<id>` carrying
// `"interrupted"`) and sends a new request if it means to ask again.
//
// `statechart_static_mesh_request.scxml` asks from `asking`; `done.invoke.ask` and
// `error.invoke.ask` each leave it, counting in `answered` and `failed`. The shared instance
// `saved/statechart_static_mesh_request_asking.json` is the format-2 text every backend writes
// (docs/adr/0045); `saved/format1/` holds the format-1 text of the same run. A C11 root draws its
// own id, so the text a run saves is held to the shared one once the id it drew is replaced by
// the shared one.
//
// The Rust, Kotlin, Go and Python halves are `a_saved_mesh_request_is_not_sent_again_after_a_restore.rs`,
// `ASavedMeshRequestIsNotSentAgainAfterARestoreTest.kt`,
// `a_saved_mesh_request_is_not_sent_again_after_a_restore_test.go` and
// `test_a_saved_mesh_request_is_not_sent_again_after_a_restore.py`.

// `snprintf` is C99; the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include "statechart_static_mesh_request_sm.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_HOST_SAVED_FIXTURES_DIR
#error "SCE_HOST_SAVED_FIXTURES_DIR names sce-build/tests/fixtures/host_processor/saved; the build defines it"
#endif

#define SHARED_ROOT "fixture-root"

// 2023-11-14T22:13:20Z, the wall-clock moment the shared instance was saved at.
#define SAVED_AT_MS INT64_C(1700000000000)

static int failures = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        if (!(condition)) {                                                                                            \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

enum { TEXT_CAP = 4096 };

typedef statechart_static_mesh_request_t machine_t;
#define M(name) statechart_static_mesh_request_##name

// What the router was handed.
typedef struct {
    int starts;
} router_t;

static void router(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    router_t *log = (router_t *)user_data;
    (void)out;
    if (event->phase == SCE_HOST_INVOKE_START) {
        log->starts++;
    }
}

static void shared_text(bool format1, char *out, size_t cap) {
    char path[1024];
    size_t n;
    FILE *file;
    (void)snprintf(path, sizeof(path), "%s/%sstatechart_static_mesh_request_asking.json", SCE_HOST_SAVED_FIXTURES_DIR,
                   format1 ? "format1/" : "");
    file = fopen(path, "rb");
    CHECK(file != NULL, "cannot read %s", path);
    if (file == NULL) {
        out[0] = '\0';
        return;
    }
    n = fread(out, 1u, cap - 1u, file);
    (void)fclose(file);
    while (n > 0u && (out[n - 1u] == '\n' || out[n - 1u] == '\r')) {
        --n;
    }
    out[n] = '\0';
}

// `text` with every `from` made `to`.
static void replace_all(const char *text, const char *from, const char *to, char *out, size_t cap) {
    const size_t from_len = strlen(from);
    const size_t to_len = strlen(to);
    size_t at = 0u;
    while (*text != '\0' && at + 1u < cap) {
        if (strncmp(text, from, from_len) == 0) {
            if (at + to_len + 1u >= cap) {
                break;
            }
            memcpy(out + at, to, to_len);
            at += to_len;
            text += from_len;
        } else {
            out[at++] = *text++;
        }
    }
    out[at] = '\0';
}

// A machine whose request to `#motor` is in flight, the router having been handed it.
static void asking(machine_t *sm, router_t *log) {
    sce_host_invoker_registry_t wiring;
    M(event_with_meta_t) meta;
    memset(log, 0, sizeof(*log));
    memset(&wiring, 0, sizeof(wiring));
    memset(&meta, 0, sizeof(meta));
    CHECK(sce_host_invoker_register_mesh_rpc_invoker(&wiring, router, log), "the mesh-rpc door takes the router");
    M(init_with_host_invokers)(sm, &wiring);
    M(run)(sm);
    CHECK(M(resolve_event_by_name)("go", &meta.event), "the document names `go`");
    M(raise_external)(sm, &meta);
    M(step)(sm);
}

// `text` restored `elapsed_ms` after it was saved, the router registered the way a host does it.
static bool restored(machine_t *sm, router_t *log, const char *text, int64_t elapsed_ms, sce_saved_error_t *err) {
    sce_host_invoker_registry_t wiring;
    M(restore_args_t) args;
    memset(log, 0, sizeof(*log));
    memset(&wiring, 0, sizeof(wiring));
    memset(&args, 0, sizeof(args));
    CHECK(sce_host_invoker_register_mesh_rpc_invoker(&wiring, router, log), "the mesh-rpc door takes the router");
    args.host_invokers = &wiring;
    return M(restore)(sm, text, strlen(text), SAVED_AT_MS + elapsed_ms, &args, err);
}

static void restores(machine_t *sm, router_t *log, const char *text, int64_t elapsed_ms) {
    sce_saved_error_t err;
    CHECK(restored(sm, log, text, elapsed_ms, &err), "restores: %s", err.message);
}

// ── What a request in flight is saved with ──────────────────────────────────

static void a_request_in_flight_is_saved_as_the_call_it_was(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    static char again[TEXT_CAP];
    static char want[TEXT_CAP];
    router_t log;
    size_t len = 0u;
    sce_saved_error_t err;
    asking(&sm, &log);
    CHECK(log.starts == 1, "the router was handed the request: %d", log.starts);
    if (!M(save)(&sm, SAVED_AT_MS, text, sizeof(text), &len, &err)) {
        CHECK(false, "saves: %s", err.message);
        return;
    }
    replace_all(text, M(session_id)(&sm), SHARED_ROOT, again, sizeof(again));
    // One call in flight, with no deadline of its own: the router owns the deadline of a Mesh
    // request. The text every backend writes for the same machine, byte for byte.
    shared_text(false, want, sizeof(want));
    CHECK(strcmp(again, want) == 0, "saved as\n%s\nthe shared text is\n%s", again, want);
    M(destroy)(&sm);
}

static void a_restore_asks_nothing_of_the_router_until_the_machine_is_driven(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    router_t log;
    shared_text(false, text, sizeof(text));
    restores(&sm, &log, text, 1000);
    CHECK(log.starts == 0, "the restore itself calls nobody");
    CHECK(M(in_state)(&sm, STATECHART_STATIC_MESH_REQUEST_STATE_ASKING), "standing in `asking`");
    M(destroy)(&sm);
}

static void a_restored_request_is_not_sent_again_and_the_document_is_told_it_was_interrupted(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    router_t log;
    shared_text(false, text, sizeof(text));
    restores(&sm, &log, text, 1000);
    M(step)(&sm);
    CHECK(log.starts == 0, "the peer may have acted on the first request already, so no second one leaves: %d",
          log.starts);
    CHECK(M(get_failed)(&sm) == 1u && M(get_answered)(&sm) == 0u && M(get_refused)(&sm) == 0u,
          "error.invoke.ask arrived: failed %u, answered %u, refused %u", (unsigned)M(get_failed)(&sm),
          (unsigned)M(get_answered)(&sm), (unsigned)M(get_refused)(&sm));
    CHECK(M(ended_in)(&sm, STATECHART_STATIC_MESH_REQUEST_STATE_DONE), "the run ended in `done`");
    M(destroy)(&sm);
}

static void a_call_is_interrupted_whenever_the_machine_comes_back(void) {
    // There is no moment after which the call is still worth sending again.
    static const int64_t elapsed[] = {0, 250, 60000};
    for (size_t i = 0u; i < sizeof(elapsed) / sizeof(elapsed[0]); ++i) {
        static machine_t sm;
        static char text[TEXT_CAP];
        router_t log;
        shared_text(false, text, sizeof(text));
        restores(&sm, &log, text, elapsed[i]);
        M(step)(&sm);
        CHECK(log.starts == 0, "%ld ms: %d starts", (long)elapsed[i], log.starts);
        CHECK(M(get_failed)(&sm) == 1u, "%ld ms after the save: failed is %u", (long)elapsed[i],
              (unsigned)M(get_failed)(&sm));
        M(destroy)(&sm);
    }
}

static void a_format_one_request_is_interrupted_the_same_way(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    router_t log;
    shared_text(true, text, sizeof(text));
    restores(&sm, &log, text, 1000);
    M(step)(&sm);
    CHECK(log.starts == 0 && M(get_failed)(&sm) == 1u, "%d starts, failed %u", log.starts,
          (unsigned)M(get_failed)(&sm));
    M(destroy)(&sm);
}

static void a_restored_machine_saved_again_before_it_was_driven_keeps_the_call(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    static char again[TEXT_CAP];
    router_t log;
    size_t len = 0u;
    sce_saved_error_t err;
    shared_text(false, text, sizeof(text));
    restores(&sm, &log, text, 1000);
    if (M(save)(&sm, SAVED_AT_MS + 1000, again, sizeof(again), &len, &err)) {
        CHECK(strstr(again, "\"hostinvokes\":[{\"type\":\"sce:mesh-rpc\",\"id\":\"ask\"") != NULL,
              "the call is still held: %s", again);
    } else {
        CHECK(false, "saves: %s", err.message);
    }
    M(destroy)(&sm);
}

int main(void) {
    a_request_in_flight_is_saved_as_the_call_it_was();
    a_restore_asks_nothing_of_the_router_until_the_machine_is_driven();
    a_restored_request_is_not_sent_again_and_the_document_is_told_it_was_interrupted();
    a_call_is_interrupted_whenever_the_machine_comes_back();
    a_format_one_request_is_interrupted_the_same_way();
    a_restored_machine_saved_again_before_it_was_driven_keeps_the_call();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("a saved mesh request: ok");
    return 0;
}
