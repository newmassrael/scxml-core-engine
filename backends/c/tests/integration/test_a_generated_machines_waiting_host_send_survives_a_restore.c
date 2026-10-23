// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a delayed `<send>` the HOST serves is a wait
// a saved machine carries, with the request it will make when the wait ends, measured through the
// machine a generator wrote.
//
// `statechart_static_delayed_host_send.scxml` is a `datamodel="sce-static"` document, generated
// with the processor declaration: the send is armed by the code the generator emitted, with its
// two typed `<param>`s read from the machine's fields, and saved and restored by the `_save` and
// `_restore` the generator emitted. The text this backend writes is the format-2
// `saved/statechart_static_delayed_host_send_waiting.json` (docs/adr/0045), its root given the id
// the fixtures name it by; the format-1 text of the same run, under `saved/format1/`, still
// restores. A C11 root draws its own id, so the text a run saves is held to the shared one once
// the id it drew is replaced by the shared one.
//
// The run `start`, `bump` changes `job` after the send was armed, so the request the wait carries
// (`job` 7) is told from one evaluated again when the wait ends (`job` 8): a restore hands the
// host the request the document made, and the machine itself carries on with the `job` it had.
//
// Driven entirely on a manual clock: no case sleeps, and none can be decided by how loaded the
// build machine is.
//
// The Rust, Kotlin, Go and Python halves are `a_generated_machines_waiting_host_send_survives_a_restore.rs`,
// `AGeneratedMachinesWaitingHostSendSurvivesARestoreTest.kt`,
// `a_generated_machines_waiting_host_send_survives_a_restore_test.go` and
// `test_a_generated_machines_waiting_host_send_survives_a_restore.py`.

// `snprintf` is C99; the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include "statechart_static_delayed_host_send_sm.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_HOST_SAVED_FIXTURES_DIR
#error "SCE_HOST_SAVED_FIXTURES_DIR names sce-build/tests/fixtures/host_processor/saved; the build defines it"
#endif

#define DECLARED_TYPE "x-sce-host"
#define SHARED_ROOT "fixture-root"

// 2023-11-14T22:13:20Z, the wall-clock moment the shared instance was saved at.
#define SAVED_AT_MS INT64_C(1700000000000)
// How long the document's send waits, and how much of that has gone by when the machine is saved.
#define WAIT_MS 500u
#define WAITED_MS 100u

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

enum { TEXT_CAP = 4096, MAX_CALLS = 8 };

typedef statechart_static_delayed_host_send_t machine_t;
#define M(name) statechart_static_delayed_host_send_##name

// What the host was asked: the engine's reading of "now" at the moment, and the request.
typedef struct {
    uint64_t at;
    char event[32];
    char target[32];
    char send_id[16];
    char event_data[128];
    char job[16];
} call_t;

typedef struct {
    call_t calls[MAX_CALLS];
    int count;
    uint64_t now;
} host_t;

static void handler(void *user_data, const sce_host_send_request_t *request, sce_host_send_response_list_t *out) {
    host_t *host = (host_t *)user_data;
    const char *job = sce_host_send_param(request, "job");
    (void)out;
    if (host->count >= MAX_CALLS) {
        return;
    }
    call_t *call = &host->calls[host->count++];
    call->at = host->now;
    (void)snprintf(call->event, sizeof(call->event), "%s", request->event_name);
    (void)snprintf(call->target, sizeof(call->target), "%s", request->target);
    (void)snprintf(call->send_id, sizeof(call->send_id), "%s", request->send_id);
    (void)snprintf(call->event_data, sizeof(call->event_data), "%s", request->event_data);
    (void)snprintf(call->job, sizeof(call->job), "%s", job != NULL ? job : "(absent)");
}

// The text of the shared instance without its closing newline.
static void shared_text(bool format1, char *out, size_t cap) {
    char path[1024];
    size_t n;
    FILE *file;
    (void)snprintf(path, sizeof(path), "%s/%sstatechart_static_delayed_host_send_waiting.json",
                   SCE_HOST_SAVED_FIXTURES_DIR, format1 ? "format1/" : "");
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

static void drive(machine_t *sm, const char *event) {
    M(event_with_meta_t) meta;
    memset(&meta, 0, sizeof(meta));
    CHECK(M(resolve_event_by_name)(event, &meta.event), "the document names %s", event);
    M(raise_external)(sm, &meta);
    M(run)(sm);
}

static void advance_to(host_t *host, machine_t *sm, uint64_t to_ms) {
    CHECK(to_ms >= host->now, "time does not run backwards in these cases");
    const uint64_t from_ms = host->now;
    // The host's reading of "now" is the moment the clock reaches, which is what the handler sees
    // when the wait ends during the advance.
    host->now = to_ms;
    M(advance_time_ms)(sm, to_ms - from_ms);
    M(run)(sm);
}

// A machine that armed its send with `job` 7, saw `bump`, and has waited WAITED_MS of the WAIT_MS.
static void waiting(machine_t *sm) {
    static host_t host;
    sce_host_processor_registry_t wiring;
    memset(&host, 0, sizeof(host));
    memset(&wiring, 0, sizeof(wiring));
    CHECK(sce_host_registry_register(&wiring, DECLARED_TYPE, handler, &host), "the processor registers");
    M(init_with_clock_and_host_processors)(sm, sce_clock_manual(0u), &wiring);
    drive(sm, "start");
    drive(sm, "bump");
    M(advance_time_ms)(sm, WAITED_MS);
    M(run)(sm);
    CHECK(M(in_state)(sm, STATECHART_STATIC_DELAYED_HOST_SEND_STATE_WAITING), "the machine stands in `waiting`");
}

static bool saved_as_shared(const machine_t *sm, char *out, size_t cap) {
    static char text[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    if (!M(save)(sm, SAVED_AT_MS, text, sizeof(text), &len, &err)) {
        CHECK(false, "the machine saves: %s", err.message);
        return false;
    }
    replace_all(text, M(session_id)(sm), SHARED_ROOT, out, cap);
    return true;
}

// `text` restored `elapsed_ms` after it was saved, its host serving DECLARED_TYPE.
static bool restored(machine_t *sm, host_t *host, const char *text, int64_t elapsed_ms, sce_saved_error_t *err) {
    sce_host_processor_registry_t wiring;
    M(restore_args_t) args;
    memset(host, 0, sizeof(*host));
    memset(&wiring, 0, sizeof(wiring));
    memset(&args, 0, sizeof(args));
    CHECK(sce_host_registry_register(&wiring, DECLARED_TYPE, handler, host), "the processor registers");
    args.clock = sce_clock_manual(0u);
    args.host_processors = &wiring;
    return M(restore)(sm, text, strlen(text), SAVED_AT_MS + elapsed_ms, &args, err);
}

static void restores(machine_t *sm, host_t *host, const char *text, int64_t elapsed_ms) {
    sce_saved_error_t err;
    CHECK(restored(sm, host, text, elapsed_ms, &err), "the restore was refused: %s", err.message);
}

// ── What a waiting send is saved with ───────────────────────────────────────

static void a_waiting_send_is_saved_with_the_request_it_was_armed_with(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    static char want[TEXT_CAP];
    waiting(&sm);
    if (!saved_as_shared(&sm, text, sizeof(text))) {
        return;
    }
    // Armed with `job` 7: `bump` ran after, and the request does not follow it. The machine itself
    // carries the `job` it has now (8).
    CHECK(strstr(text, "\"due\":\"1700000000400\"") != NULL, "%s", text);
    CHECK(strstr(text, "\"params\":{\"job\":[\"7\"],\"label\":[\"report\"]}") != NULL, "%s", text);
    CHECK(strstr(text, "\"variables\":{\"job\":8,\"label\":\"report\"}") != NULL, "%s", text);
    // The text every backend writes for the same machine, byte for byte.
    shared_text(false, want, sizeof(want));
    CHECK(strcmp(text, want) == 0, "saved as\n%s\nthe shared text is\n%s", text, want);
    M(destroy)(&sm);
}

static void a_restored_send_is_made_when_its_moment_comes_with_the_job_it_was_armed_with(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    host_t host;
    shared_text(false, text, sizeof(text));
    // Back 50 ms after the save: the send has 350 ms left of the 400 it had.
    restores(&sm, &host, text, 50);
    CHECK(M(in_state)(&sm, STATECHART_STATIC_DELAYED_HOST_SEND_STATE_WAITING), "standing in `waiting`");
    advance_to(&host, &sm, 349u);
    CHECK(host.count == 0, "nothing is due yet: %d calls", host.count);
    advance_to(&host, &sm, 350u);
    CHECK(host.count == 1, "the send is made once: %d calls", host.count);
    if (host.count == 1) {
        const call_t *made = &host.calls[0];
        CHECK(made->at == 350u, "at the moment it came due: %lu", (unsigned long)made->at);
        CHECK(strcmp(made->event, "audit") == 0 && strcmp(made->target, "job://report") == 0 &&
                  strcmp(made->send_id, "a") == 0,
              "the request is %s to %s as %s", made->event, made->target, made->send_id);
        CHECK(strcmp(made->job, "7") == 0,
              "the request the document made, not one evaluated again with the job the machine holds now: %s",
              made->job);
        CHECK(strcmp(made->event_data, "{\"job\":7,\"label\":\"report\"}") == 0, "event data is %s", made->event_data);
    }
    M(destroy)(&sm);
}

static void a_waiting_send_saved_in_format_one_is_made_when_its_moment_comes(void) {
    // The format-1 text of the same run, which a backend wrote before trees.
    static machine_t sm;
    static char text[TEXT_CAP];
    host_t host;
    shared_text(true, text, sizeof(text));
    restores(&sm, &host, text, 50);
    advance_to(&host, &sm, 350u);
    CHECK(host.count == 1 && strcmp(host.calls[0].job, "7") == 0, "%d calls, job %s", host.count,
          host.count > 0 ? host.calls[0].job : "(none)");
    M(destroy)(&sm);
}

static void a_send_already_due_when_the_machine_comes_back_is_made_when_it_is_driven(void) {
    // A minute away: the wait ran out while the process was dead.
    static machine_t sm;
    static char text[TEXT_CAP];
    host_t host;
    shared_text(false, text, sizeof(text));
    restores(&sm, &host, text, 60000);
    CHECK(host.count == 0, "restoring makes no send");
    advance_to(&host, &sm, 0u);
    CHECK(host.count == 1 && host.calls[0].at == 0u, "%d calls", host.count);
    M(destroy)(&sm);
}

static void a_restored_send_can_still_be_cancelled_by_its_id(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    host_t host;
    shared_text(false, text, sizeof(text));
    restores(&sm, &host, text, 0);
    drive(&sm, "cancel");
    advance_to(&host, &sm, 10000u);
    CHECK(host.count == 0, "the cancel found the send the saved state carried: %d calls", host.count);
    M(destroy)(&sm);
}

static void a_restored_machine_carries_on_with_the_job_it_had(void) {
    // `job` is 8 in the saved state; a `bump` after the restore makes it 9, and a new wait armed
    // from `idle` reads the machine's own field. The restored wait is over (at 400 ms) before the
    // second is armed: a send under an id still waiting is replaced by one backend and left beside
    // it by another, and this case is not about that.
    static machine_t sm;
    static char text[TEXT_CAP];
    host_t host;
    shared_text(false, text, sizeof(text));
    restores(&sm, &host, text, 0);
    advance_to(&host, &sm, 400u);
    drive(&sm, "stop");
    drive(&sm, "bump");
    drive(&sm, "start");
    advance_to(&host, &sm, 10000u);
    CHECK(host.count == 2 && strcmp(host.calls[0].job, "7") == 0 && strcmp(host.calls[1].job, "9") == 0,
          "the restored wait first, then the one armed with the job the restored field held: %d calls, jobs %s %s",
          host.count, host.count > 0 ? host.calls[0].job : "-", host.count > 1 ? host.calls[1].job : "-");
    M(destroy)(&sm);
}

static void a_saved_state_round_trips_through_its_own_text(void) {
    static machine_t sm;
    static char want[TEXT_CAP];
    static char again[TEXT_CAP];
    host_t host;
    size_t len = 0u;
    sce_saved_error_t err;
    shared_text(false, want, sizeof(want));
    restores(&sm, &host, want, 0);
    if (M(save)(&sm, SAVED_AT_MS, again, sizeof(again), &len, &err)) {
        CHECK(strcmp(again, want) == 0, "saved as\n%s\nwant\n%s", again, want);
    } else {
        CHECK(false, "saves: %s", err.message);
    }
    M(destroy)(&sm);
}

int main(void) {
    a_waiting_send_is_saved_with_the_request_it_was_armed_with();
    a_restored_send_is_made_when_its_moment_comes_with_the_job_it_was_armed_with();
    a_waiting_send_saved_in_format_one_is_made_when_its_moment_comes();
    a_send_already_due_when_the_machine_comes_back_is_made_when_it_is_driven();
    a_restored_send_can_still_be_cancelled_by_its_id();
    a_restored_machine_carries_on_with_the_job_it_had();
    a_saved_state_round_trips_through_its_own_text();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("a waiting host send survives a restore: ok");
    return 0;
}
