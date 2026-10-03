// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated C:
// a host action whose arguments are typed expressions of the machine's own
// variables, called through the vtable the machine is initialised with, and what
// a failing computation costs.
//
// The two documents are the ones the C++ suite drives
// (`tests/integration/AStaticDatamodelRunsGeneratedCppTest.cpp`), and the
// expected calls are the ones it states:
//
//   * `static_host_call`: each argument is read when the call is made — one call
//     per entry of `idle`, with the datamodel as it stood.
//   * `static_host_call_arguments`: an argument that overflows is a failure, not
//     a wrapped value (SCE_FORGE.md §3.4.1), so the host is NOT called with it
//     and `error.execution` is raised in the call's place (SCE Accepted Subset
//     §2.15, E12 D5).
//
// Linked WITHOUT `sce_c_scripting` and `lua54`: the machines have no script
// engine, so the link is the proof.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_host_call_arguments_sm.h"
#include "static_host_call_sm.h"

// What a host recorded of the calls a machine made, as text.
typedef struct {
    char calls[8][32];
    int count;
} recorder_t;

static void record(recorder_t *r, const char *text) {
    if (r->count < 8) {
        (void)snprintf(r->calls[r->count], sizeof(r->calls[0]), "%s", text);
    }
    r->count++;
}

static void on_show_attempts(void *user_data, uint32_t count, bool exhausted) {
    char text[32];
    (void)snprintf(text, sizeof(text), "%u,%s", (unsigned)count, exhausted ? "true" : "false");
    record((recorder_t *)user_data, text);
}

static void on_report(void *user_data, uint8_t next) {
    char text[32];
    (void)snprintf(text, sizeof(text), "%u", (unsigned)next);
    record((recorder_t *)user_data, text);
}

// The calls a recorder holds, against the calls expected, in order.
static int calls_are(const char *scenario, const recorder_t *r, const char *const *want, int want_count) {
    int bad = 0;
    if (r->count != want_count) {
        (void)fprintf(stderr, "static_host_call: FAIL [%s] - %d calls, want %d\n", scenario, r->count, want_count);
        bad = 1;
    }
    for (int i = 0; i < want_count && i < r->count && i < 8; ++i) {
        if (strcmp(r->calls[i], want[i]) != 0) {
            (void)fprintf(stderr, "static_host_call: FAIL [%s] - call %d is \"%s\", want \"%s\"\n", scenario, i,
                          r->calls[i], want[i]);
            bad = 1;
        }
    }
    return bad;
}

// A host action takes the machine's variables as typed arguments, each read when
// the call is made: one call per entry of `idle`. The fourth retry finds
// `attempts < 3` false and re-enters nothing.
static int a_host_action_takes_typed_datamodel_arguments(void) {
    recorder_t r;
    memset(&r, 0, sizeof(r));
    static_host_call_actions_t host;
    memset(&host, 0, sizeof(host));
    host.show_attempts = on_show_attempts;
    host.user_data = &r;

    static_host_call_t sm;
    if (!static_host_call_init_with_actions(&sm, &host)) {
        (void)fprintf(stderr, "static_host_call: FAIL - a complete vtable was refused\n");
        return 1;
    }
    for (int i = 0; i < 4; ++i) {
        static_host_call_event_with_meta_t meta;
        memset(&meta, 0, sizeof(meta));
        meta.event = STATIC_HOST_CALL_EVENT_RETRY;
        static_host_call_raise_external(&sm, &meta);
        static_host_call_step(&sm);
    }
    const char *const want[] = {"0,false", "1,false", "2,false", "3,true"};
    return calls_are("typed arguments", &r, want, 4);
}

// An argument that cannot be computed is a failure, not a wrapped value: the
// host is not called, and `error.execution` is raised in the call's place.
static int an_argument_that_overflows_stops_the_call_and_raises_an_error(void) {
    recorder_t r;
    memset(&r, 0, sizeof(r));
    static_host_call_arguments_actions_t host;
    memset(&host, 0, sizeof(host));
    host.report = on_report;
    host.user_data = &r;

    static_host_call_arguments_t sm;
    if (!static_host_call_arguments_init_with_actions(&sm, &host)) {
        (void)fprintf(stderr, "static_host_call: FAIL - a complete vtable was refused\n");
        return 1;
    }
    int bad = 0;
    static_host_call_arguments_event_with_meta_t meta;

    memset(&meta, 0, sizeof(meta));
    meta.event = STATIC_HOST_CALL_ARGUMENTS_EVENT_FINE;
    static_host_call_arguments_raise_external(&sm, &meta);
    static_host_call_arguments_step(&sm);
    const char *const after_fine[] = {"251"};
    bad |= calls_are("250 + 1 fits a uint8", &r, after_fine, 1);
    if (static_host_call_arguments_get_errors(&sm) != 0) {
        (void)fprintf(stderr, "static_host_call: FAIL - an argument that fit raised an error\n");
        bad = 1;
    }

    memset(&meta, 0, sizeof(meta));
    meta.event = STATIC_HOST_CALL_ARGUMENTS_EVENT_OVERFLOW;
    static_host_call_arguments_raise_external(&sm, &meta);
    static_host_call_arguments_step(&sm);
    bad |= calls_are("250 + 10 does not fit, so the host is not called with 4", &r, after_fine, 1);
    if (static_host_call_arguments_get_errors(&sm) != 1) {
        (void)fprintf(stderr, "static_host_call: FAIL - error.execution was not raised and seen: errors is %u\n",
                      (unsigned)static_host_call_arguments_get_errors(&sm));
        bad = 1;
    }
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_host_action_takes_typed_datamodel_arguments();
    bad |= an_argument_that_overflows_stops_the_call_and_raises_an_error();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static host call: ok\n");
    return 0;
}
