// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": an `<invoke>` the HOST runs is part of what a
// machine is doing (§scxml-6.4.1), and the process that ran it is gone with the process that saved
// it. A saved state holds each running host-run invocation with the request it was started with and
// the deadline it had left, and a restore starts it again from that request, never evaluating the
// element a second time, with `restarted` set, a new token and what was left of the deadline.
//
// `statechart_static_host_invoke.scxml` hands a job to the host from `working`. `done.invoke.h` and
// `error.invoke.h` each leave it and record a digit in `seen`, so the number says which arrived. The
// other halves are `ASavedHostInvocationIsStartedAgainAfterARestoreTest.cpp`,
// `a_saved_host_invocation_is_started_again_after_a_restore.rs` and the Kotlin, Go and Python
// twins.
//
// Driven entirely on the machine's manual clock: no case sleeps, and none can be decided by how
// loaded the build machine is. The shared instance `saved/statechart_static_host_invoke_running.json`
// is the format-2 text every backend writes (docs/adr/0045); the format-1 text of the same run, under
// `saved/format1/`, still restores.

// `strstr` and `snprintf` are C99; the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include "statechart_static_host_invoke_sm.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_HOST_SAVED_FIXTURES_DIR
#error "SCE_HOST_SAVED_FIXTURES_DIR names sce-build/tests/fixtures/host_processor/saved; the build defines it"
#endif

#define M statechart_static_host_invoke
#define HOST_TYPE "x-sce-host"

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

enum { TEXT_CAP = 8192, LOG_CAP = 8 };

// What the host was told, in order: a start or a cancel.
typedef struct {
    bool start;
    bool restarted;
    uint64_t token;
    char invoke_id[32];
    char src[64];
    char content[32];
    char event_data[128];
    char session_id[64];
    int param_count;
    char names[4][16];
    char values[4][32];
} entry_t;

typedef struct {
    entry_t log[LOG_CAP];
    int count;
} host_t;

static void invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    host_t *host = (host_t *)user_data;
    entry_t *entry;
    (void)out;
    if (host->count >= LOG_CAP) {
        return;
    }
    entry = &host->log[host->count++];
    memset(entry, 0, sizeof(*entry));
    entry->start = event->phase == SCE_HOST_INVOKE_START;
    entry->token = event->token;
    (void)snprintf(entry->invoke_id, sizeof(entry->invoke_id), "%s", event->invoke_id);
    if (entry->start) {
        entry->restarted = event->restarted;
        (void)snprintf(entry->src, sizeof(entry->src), "%s", event->src);
        (void)snprintf(entry->content, sizeof(entry->content), "%s", event->content);
        (void)snprintf(entry->event_data, sizeof(entry->event_data), "%s", event->event_data);
        (void)snprintf(entry->session_id, sizeof(entry->session_id), "%s", event->session_id);
        entry->param_count = event->param_count < 4 ? event->param_count : 4;
        for (int i = 0; i < entry->param_count; ++i) {
            (void)snprintf(entry->names[i], sizeof(entry->names[i]), "%s", event->params[i].name);
            (void)snprintf(entry->values[i], sizeof(entry->values[i]), "%s", event->params[i].value);
        }
    }
}

static int starts_of(const host_t *host) {
    int n = 0;
    for (int i = 0; i < host->count; ++i) {
        n += host->log[i].start ? 1 : 0;
    }
    return n;
}

static const entry_t *first_start(const host_t *host) {
    for (int i = 0; i < host->count; ++i) {
        if (host->log[i].start) {
            return &host->log[i];
        }
    }
    return NULL;
}

static bool read_trimmed(const char *path, char *out, size_t cap, size_t *len) {
    FILE *file = fopen(path, "rb");
    size_t n;
    if (file == NULL) {
        return false;
    }
    n = fread(out, 1u, cap - 1u, file);
    (void)fclose(file);
    while (n > 0u && (out[n - 1u] == '\n' || out[n - 1u] == '\r')) {
        --n;
    }
    out[n] = '\0';
    *len = n;
    return true;
}

static void shared(bool format1, char *out, size_t cap, size_t *len) {
    char path[1024];
    (void)snprintf(path, sizeof(path), "%s/%sstatechart_static_host_invoke_running.json", SCE_HOST_SAVED_FIXTURES_DIR,
                   format1 ? "format1/" : "");
    CHECK(read_trimmed(path, out, cap, len), "cannot read %s", path);
}

typedef statechart_static_host_invoke_t machine_t;

static bool raise_named(machine_t *sm, const char *name) {
    statechart_static_host_invoke_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    if (!statechart_static_host_invoke_resolve_event_by_name(name, &meta.event)) {
        return false;
    }
    statechart_static_host_invoke_raise_external(sm, &meta);
    statechart_static_host_invoke_step(sm);
    return true;
}

static void drive(machine_t *sm, uint64_t ms) {
    statechart_static_host_invoke_advance_time_ms(sm, ms);
    statechart_static_host_invoke_run(sm);
}

// A machine on a manual clock that the host's invoker is registered on, which `init` starts.
static void boot(machine_t *sm, host_t *host) {
    sce_host_invoker_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    CHECK(sce_host_invoker_register(&wiring, HOST_TYPE, invoker, host), "the invoker registers");
    statechart_static_host_invoke_init_with_host_invokers(sm, &wiring);
}

// A machine standing in `working`, its invocation started with token 0.
static void working(machine_t *sm, host_t *host) {
    boot(sm, host);
    CHECK(raise_named(sm, "start"), "the document names start");
}

// `text` restored `elapsed_ms` of wall-clock time after it was saved, its host invoker registered the
// way a host does it: in the arguments of the restore. The clock is the test's own.
static bool restore_with(machine_t *sm, host_t *host, const char *text, int64_t elapsed_ms, sce_saved_error_t *err) {
    sce_host_invoker_registry_t wiring;
    statechart_static_host_invoke_restore_args_t args;
    memset(&wiring, 0, sizeof(wiring));
    memset(&args, 0, sizeof(args));
    (void)sce_host_invoker_register(&wiring, HOST_TYPE, invoker, host);
    args.clock = sce_clock_manual(0u);
    args.host_invokers = &wiring;
    return statechart_static_host_invoke_restore(sm, text, strlen(text), SAVED_AT_MS + elapsed_ms, &args, err);
}

static bool standing_in(const machine_t *sm, const char *state) {
    statechart_static_host_invoke_state_t s;
    return statechart_static_host_invoke_state_from_name(state, &s) &&
           (sm->active & ((uint32_t)1u << (uint32_t)s)) != 0u;
}

static uint32_t seen_of(const machine_t *sm) {
    return statechart_static_host_invoke_get_seen(sm);
}

// ── The text ────────────────────────────────────────────────────────────────

static void a_machine_is_saved_with_the_request_its_invocation_was_started_with(void) {
    static machine_t sm;
    static char saved[TEXT_CAP];
    static char text[TEXT_CAP];
    size_t len = 0u;
    size_t want_len = 0u;
    host_t host;
    sce_saved_error_t err;
    const entry_t *first;
    memset(&host, 0, sizeof(host));
    working(&sm, &host);
    first = first_start(&host);
    CHECK(first != NULL && !first->restarted, "the document's own start is not a restart");
    CHECK(first != NULL && first->token == 0u, "the first start has token 0");
    CHECK(statechart_static_host_invoke_save(&sm, SAVED_AT_MS, saved, sizeof(saved), &len, &err), "%s", err.message);
    // The text every backend writes for the same machine, byte for byte, but for the id of the
    // session, which this one drew and the shared text gives its root by hand.
    shared(false, text, sizeof(text), &want_len);
    {
        char *drawn = strstr(saved, "\"sessionid\":\"");
        char *end = drawn != NULL ? strchr(drawn + 13, '"') : NULL;
        static char same[TEXT_CAP];
        CHECK(drawn != NULL && end != NULL, "the text names its session");
        if (drawn != NULL && end != NULL) {
            (void)snprintf(same, sizeof(same), "%.*s\"sessionid\":\"fixture-root%s", (int)(drawn - saved), saved, end);
            CHECK(strcmp(same, text) == 0, "saved as\n%s\nthe shared text is\n%s", same, text);
        }
    }
}

static void a_restore_asks_nothing_of_the_host_until_the_machine_is_driven(void) {
    for (int format1 = 0; format1 < 2; ++format1) {
        static machine_t sm;
        static char text[TEXT_CAP];
        size_t len = 0u;
        host_t host;
        sce_saved_error_t err;
        memset(&host, 0, sizeof(host));
        shared(format1 != 0, text, sizeof(text), &len);
        CHECK(restore_with(&sm, &host, text, 1000, &err), "%s", err.message);
        CHECK(host.count == 0, "the restore itself calls nobody");
        CHECK(standing_in(&sm, "working"), "the machine stands where it stood");
        // The first macrostep starts it, where an invocation the document entered would have started.
        drive(&sm, 0u);
        CHECK(starts_of(&host) == 1, "format %d: the first drive starts it, %d starts", format1 + 1, starts_of(&host));
    }
}

static void a_restored_invocation_is_started_again_from_the_request_it_was_saved_with(void) {
    static machine_t original_sm;
    static machine_t sm;
    static char text[TEXT_CAP];
    size_t len = 0u;
    host_t original;
    host_t host;
    sce_saved_error_t err;
    const entry_t *before;
    const entry_t *again;
    memset(&original, 0, sizeof(original));
    memset(&host, 0, sizeof(host));
    working(&original_sm, &original);
    before = first_start(&original);
    shared(false, text, sizeof(text), &len);
    CHECK(restore_with(&sm, &host, text, 1000, &err), "%s", err.message);
    drive(&sm, 0u);
    again = first_start(&host);
    CHECK(again != NULL && before != NULL, "both runs start it");
    if (again == NULL || before == NULL) {
        return;
    }
    CHECK(again->restarted, "the host is told this is a resumption");
    CHECK(again->token == 1u, "the next token the saved state carried on: %lu", (unsigned long)again->token);
    CHECK(strcmp(again->session_id, "fixture-root") == 0, "the session that started it: %s", again->session_id);
    CHECK(strcmp(again->invoke_id, before->invoke_id) == 0, "the same id");
    CHECK(strcmp(again->src, before->src) == 0, "the same src: %s", again->src);
    CHECK(strcmp(again->event_data, before->event_data) == 0, "the same event data: %s", again->event_data);
    CHECK(strcmp(again->content, before->content) == 0, "the same request, not one evaluated again");
    CHECK(again->param_count == before->param_count, "the same params");
    for (int i = 0; i < again->param_count && i < before->param_count; ++i) {
        CHECK(strcmp(again->names[i], before->names[i]) == 0 && strcmp(again->values[i], before->values[i]) == 0,
              "param %d: %s=%s, was %s=%s", i, again->names[i], again->values[i], before->names[i], before->values[i]);
    }
}

static void a_restored_invocation_keeps_the_deadline_it_had_left(void) {
    // Saved with 5000 ms to run, restored 1000 ms later: 4000 ms are left.
    static machine_t sm;
    static char text[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    sce_saved_error_t err;
    memset(&host, 0, sizeof(host));
    shared(false, text, sizeof(text), &len);
    CHECK(restore_with(&sm, &host, text, 1000, &err), "%s", err.message);
    drive(&sm, 0u);
    drive(&sm, 3999u);
    CHECK(seen_of(&sm) == 0u, "not due yet");
    CHECK(standing_in(&sm, "working"), "still working");
    drive(&sm, 1u);
    CHECK(seen_of(&sm) == 2u, "error.invoke.h, at 4000 ms: %u", (unsigned)seen_of(&sm));
    CHECK(standing_in(&sm, "failed"), "the failure left `working`");
    // The host is told to stop what it was given, with the token it was given.
    CHECK(host.count >= 2 && !host.log[host.count - 1].start && host.log[host.count - 1].token == 1u &&
              strcmp(host.log[host.count - 1].invoke_id, "h") == 0,
          "the host is told to stop it");
}

static void a_deadline_that_passed_while_the_machine_was_away_ends_the_invocation_unstarted(void) {
    // 6 s after the save, and the deadline was at 5 s.
    static machine_t sm;
    static char text[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    sce_saved_error_t err;
    memset(&host, 0, sizeof(host));
    shared(false, text, sizeof(text), &len);
    CHECK(restore_with(&sm, &host, text, 6000, &err), "%s", err.message);
    drive(&sm, 0u);
    CHECK(host.count == 0, "the host is not asked to begin what it would be told to stop");
    CHECK(seen_of(&sm) == 2u, "the document hears error.invoke.h: %u", (unsigned)seen_of(&sm));
    CHECK(standing_in(&sm, "failed"), "and leaves `working`");
}

static void a_restored_machine_owing_a_start_says_it_needs_a_tick_now(void) {
    static const int64_t elapsed[] = {1000, 6000};
    for (size_t i = 0u; i < sizeof(elapsed) / sizeof(elapsed[0]); ++i) {
        static machine_t sm;
        static char text[TEXT_CAP];
        size_t len = 0u;
        host_t host;
        sce_saved_error_t err;
        memset(&host, 0, sizeof(host));
        shared(false, text, sizeof(text), &len);
        CHECK(restore_with(&sm, &host, text, elapsed[i], &err), "%s", err.message);
        CHECK(statechart_static_host_invoke_time_until_next_scheduled_ms(&sm) == 0,
              "a restored invocation owes the machine a tick before anything else");
        drive(&sm, 0u);
    }
}

static void what_the_first_run_handed_a_host_cannot_answer_for_the_second(void) {
    // Token 0 belonged to the process that saved the machine. A late reply carrying it is a different
    // process's, and is ignored (§scxml-6.4).
    static machine_t sm;
    static char text[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    sce_saved_error_t err;
    uint64_t token;
    memset(&host, 0, sizeof(host));
    shared(false, text, sizeof(text), &len);
    CHECK(restore_with(&sm, &host, text, 1000, &err), "%s", err.message);
    drive(&sm, 0u);
    token = first_start(&host)->token;
    CHECK(!statechart_static_host_invoke_complete_host_invoke(&sm, HOST_TYPE, "h", 0u, "late"),
          "a reply of the first run is accepted");
    drive(&sm, 0u);
    CHECK(seen_of(&sm) == 0u && standing_in(&sm, "working"), "and changed nothing");
    CHECK(statechart_static_host_invoke_complete_host_invoke(&sm, HOST_TYPE, "h", token, "done"),
          "the reply of this run is refused");
    drive(&sm, 0u);
    CHECK(seen_of(&sm) == 1u && standing_in(&sm, "after"), "the completion of this run is taken");
}

static void an_invocation_cancelled_before_it_was_started_again_is_never_started(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    sce_saved_error_t err;
    memset(&host, 0, sizeof(host));
    shared(false, text, sizeof(text), &len);
    CHECK(restore_with(&sm, &host, text, 1000, &err), "%s", err.message);
    // Nothing was started, so there is nothing to tell the host to stop.
    CHECK(!statechart_static_host_invoke_cancel_host_invoke(&sm, HOST_TYPE, "h"), "there is no run to cancel");
    drive(&sm, 0u);
    CHECK(host.count == 0 || starts_of(&host) == 1, "it is started, when the machine is driven");
}

static void a_completion_still_queued_when_the_machine_was_saved_is_accepted_after_the_restore(void) {
    static machine_t sm;
    static machine_t again;
    static char saved[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    host_t again_host;
    sce_saved_error_t err;
    memset(&host, 0, sizeof(host));
    memset(&again_host, 0, sizeof(again_host));
    working(&sm, &host);
    // The host answers; the completion is on the external queue, stamped with the start's token, and
    // the machine has not been driven through it.
    CHECK(statechart_static_host_invoke_complete_host_invoke(&sm, HOST_TYPE, "h", 0u, "done"),
          "the completion is accepted");
    CHECK(statechart_static_host_invoke_save(&sm, SAVED_AT_MS, saved, sizeof(saved), &len, &err), "%s", err.message);
    CHECK(strstr(saved, "\"hostinvokes\":[]") != NULL, "the invocation ended: %s", saved);
    CHECK(strstr(saved, "\"hostinvoketoken\":\"0\"}") != NULL, "one queued completion carrying its token: %s", saved);
    CHECK(restore_with(&again, &again_host, saved, 0, &err), "%s", err.message);
    drive(&again, 0u);
    CHECK(seen_of(&again) == 1u && standing_in(&again, "after"), "taken once, not refused");
    CHECK(again_host.count == 0, "an ended invocation is not started again");
    CHECK(statechart_static_host_invoke_refused_host_invoke_completions(&again) == 0u, "the completion was refused");
}

static void a_machine_restored_and_saved_again_before_it_was_driven_keeps_its_invocation(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    static char again[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    sce_saved_error_t err;
    memset(&host, 0, sizeof(host));
    shared(false, text, sizeof(text), &len);
    CHECK(restore_with(&sm, &host, text, 1000, &err), "%s", err.message);
    // The same request, due at the same wall-clock moment, and the same counter.
    CHECK(statechart_static_host_invoke_save(&sm, SAVED_AT_MS + 1000, again, sizeof(again), &len, &err), "%s",
          err.message);
    CHECK(strcmp(again, text) == 0, "saved again as\n%s\nthe shared text is\n%s", again, text);
}

// ── A state the document could not have been in ─────────────────────────────

static bool restore_changed(const char *from, const char *to, const char *expect) {
    static machine_t sm;
    static char text[TEXT_CAP];
    static char changed[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    sce_saved_error_t err;
    const char *at;
    memset(&host, 0, sizeof(host));
    shared(false, text, sizeof(text), &len);
    at = strstr(text, from);
    CHECK(at != NULL, "the shared text holds %s", from);
    if (at == NULL) {
        return false;
    }
    (void)snprintf(changed, sizeof(changed), "%.*s%s%s", (int)(at - text), text, to, at + strlen(from));
    if (restore_with(&sm, &host, changed, 0, &err)) {
        CHECK(false, "the restore of %s was not refused", to);
        return false;
    }
    CHECK(strstr(err.message, expect) != NULL, "%s", err.message);
    return true;
}

static void a_saved_state_naming_a_host_invocation_the_document_cannot_have_run_is_refused(void) {
    (void)restore_changed("\"id\":\"h\"", "\"id\":\"stranger\"", "which the document does not have a host invoker run");
    (void)restore_changed("\"type\":\"x-sce-host\"", "\"type\":\"x-other\"",
                          "which the document does not have a host invoker run");
}

static void a_host_invocation_whose_state_the_configuration_is_not_in_is_refused(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    static char changed[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    sce_saved_error_t err;
    const char *a;
    const char *b;
    memset(&host, 0, sizeof(host));
    shared(false, text, sizeof(text), &len);
    a = strstr(text, "\"configuration\":[\"working\"]");
    b = strstr(text, "\"current\":\"working\"");
    CHECK(a != NULL && b != NULL && a < b, "the shared text names its state");
    if (a == NULL || b == NULL || a > b) {
        return;
    }
    (void)snprintf(changed, sizeof(changed), "%.*s\"configuration\":[\"idle\"],\"current\":\"idle\"%s", (int)(a - text),
                   text, b + strlen("\"current\":\"working\""));
    CHECK(!restore_with(&sm, &host, changed, 0, &err) &&
              strstr(err.message, "whose state the saved configuration does not stand in") != NULL,
          "%s", err.message);
}

static void an_invocation_named_twice_is_refused(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    static char twice[TEXT_CAP];
    size_t len = 0u;
    host_t host;
    sce_saved_error_t err;
    const char *open;
    const char *close;
    memset(&host, 0, sizeof(host));
    shared(false, text, sizeof(text), &len);
    open = strstr(text, "\"hostinvokes\":[");
    close = strstr(text, "],\"hostinvoketoken\"");
    CHECK(open != NULL && close != NULL, "the shared text holds the invocation");
    if (open == NULL || close == NULL) {
        return;
    }
    open += strlen("\"hostinvokes\":[");
    (void)snprintf(twice, sizeof(twice), "%.*s%.*s,%.*s%s", (int)(open - text), text, (int)(close - open), open,
                   (int)(close - open), open, close);
    CHECK(!restore_with(&sm, &host, twice, 0, &err) &&
              strstr(err.message, "which an earlier entry already names") != NULL,
          "%s", err.message);
}

int main(void) {
    a_machine_is_saved_with_the_request_its_invocation_was_started_with();
    a_restore_asks_nothing_of_the_host_until_the_machine_is_driven();
    a_restored_invocation_is_started_again_from_the_request_it_was_saved_with();
    a_restored_invocation_keeps_the_deadline_it_had_left();
    a_deadline_that_passed_while_the_machine_was_away_ends_the_invocation_unstarted();
    a_restored_machine_owing_a_start_says_it_needs_a_tick_now();
    what_the_first_run_handed_a_host_cannot_answer_for_the_second();
    an_invocation_cancelled_before_it_was_started_again_is_never_started();
    a_completion_still_queued_when_the_machine_was_saved_is_accepted_after_the_restore();
    a_machine_restored_and_saved_again_before_it_was_driven_keeps_its_invocation();
    a_saved_state_naming_a_host_invocation_the_document_cannot_have_run_is_refused();
    a_host_invocation_whose_state_the_configuration_is_not_in_is_refused();
    an_invocation_named_twice_is_refused();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("a saved host invocation: ok");
    return 0;
}
