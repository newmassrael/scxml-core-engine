// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0147, D3: the door of docs/adr/0084 is a concurrent object, and each run
// of a generated C11 machine's inbox is a history. This program runs the inbox of
// tests/integration/inbox_probe.scxml on real threads, records every post each
// poster made and every event the owner put on the machine's external queue, with
// the end of the machine's run, and writes each run in the form every runtime's
// inbox history is written in (tests/forge/conformance/inbox_history.schema.json).
// It judges nothing: `sce-codegen check-inbox-history` holds each history
// linearizable to docs/formal/InboxService.tla, as it holds every runtime's
// (ctest `c11_inbox_history_check_<row>`, which runs after this program).
//
// What enters the external queue is told by the machine: it is built with its
// inbox port on (`SCE_INBOX_PORT`), so it calls `inbox_probe_external_enqueue`
// below immediately before and immediately after each event enters, saying
// whether its inbox held the event. The drain, the order it keeps and the inbox
// are the machine's; this program reads the clock around them.
//
// A run: a first poster posts one input more than the inbox holds while the owner
// stands between two drives, so one post is refused as full. Then several threads
// post typed and named `ping`s, and now and then an ending naming a session the
// tree does not hold, while the owner drives the machine, raises `ping`s of its
// own and stops it once half the posts are made. A refused post is answered and
// its poster goes on with its next input. Every input and every own event carries
// its value as its data, which no other of the run has, and the port names each
// event it is told by that value.
//
// The machine's inbox never closes: a post taken after the stop is applied where
// the owner next drives the machine, and every history says `closes: false`.
//
// Built once per row of the queue, as a deployment states its atomics
// (`SCE_INBOX_ATOMIC_RMW_WIDTH` 64, 32 and 0), each with an inbox of four
// inputs (backends/c/tests/CMakeLists.txt).
//
//   test_a_run_of_the_inbox_is_written_as_its_history <directory>

// pthreads, sched_yield and mkdir are POSIX, and the target is built with
// C_EXTENSIONS OFF, so the feature-test macro precedes every include.
#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "../require_started.h"
#include "inbox_probe_sm.h"

/* The type the fixture was generated for. */
#define TYPE "x-sce-host"

/* Threads that post, beside the owner, and the posts each makes in one run. */
#define POSTERS 4
#define PER_POSTER 48
#define TOTAL_POSTS (POSTERS * PER_POSTER)
/* Runs per row: the schedules that matter are a small fraction of those a machine
   gives. */
#define RUNS 8
/* Own events the owner raises at most in one run. */
#define OWN_PER_RUN 24
/* The first value of the owner's own events, past every input a poster posts. */
#define OWN_BASE 1000000u
/* The number the history names the machine's session by, and the session the
   tree does not hold. */
#define SESSION 0u
#define ABSENT 1u
#define ABSENT_ID "inbox.history.absent"
/* Drives the owner makes at most, once the posters are joined, to apply what the
   inbox still holds. */
#define FINAL_DRIVES 64

/* ── The clock and the record ──────────────────────────────────────── */

/* The one clock every thread of a run reads, immediately before and immediately
   after each operation it records: one read-modify-write chain on one atomic
   orders the readings with the calls between them, so "returned before invoked"
   in the record means it in the run, and no two readings are equal. */
static _Atomic uint64_t g_clock;

static uint64_t tick(void) {
    return atomic_fetch_add_explicit(&g_clock, 1u, memory_order_acq_rel) + 1u;
}

typedef enum { OP_POST, OP_ENQUEUE, OP_END } op_kind_t;

typedef enum { POST_TAKEN, POST_REFUSED_FULL, POST_REFUSED_UNKNOWN } post_outcome_t;

/* One operation a thread observed. A post names its session, its input and what
   the inbox answered; an enqueue names its item and whether the owner raised it
   itself; an end names its session. */
typedef struct {
    op_kind_t kind;
    uint64_t session;
    uint64_t value;
    post_outcome_t outcome;
    bool own;
    uint64_t invoked;
    uint64_t returned;
} op_t;

typedef struct {
    op_t *ops;
    size_t count;
    size_t reserved;
} op_log_t;

static void log_add(op_log_t *log, const op_t *op) {
    if (log->count == log->reserved) {
        log->reserved = log->reserved == 0u ? 256u : log->reserved * 2u;
        log->ops = (op_t *)realloc(log->ops, log->reserved * sizeof(op_t));
        if (log->ops == NULL) {
            (void)fprintf(stderr, "inbox history: out of memory recording a history\n");
            exit(2);
        }
    }
    log->ops[log->count++] = *op;
}

static void log_clear(op_log_t *log) {
    log->count = 0u;
}

/* The owner's operations, in its program order: written only on the owner. */
static op_log_t g_owner;
/* Whether the owner records what the port tells it. */
static bool g_recording;
/* The clock reading the port took immediately before the event it is telling
   entered the queue. */
static uint64_t g_enqueue_invoked;

/* The value an event carries as its data: the digits of `"<value>"`; 0, which
   no input or own event of a run has, when it carries none. */
static uint64_t value_of(const char *data) {
    if (data == NULL) {
        return 0u;
    }
    while (*data != '\0' && (*data < '0' || *data > '9')) {
        data++;
    }
    return *data == '\0' ? 0u : (uint64_t)strtoull(data, NULL, 10);
}

/* docs/adr/0147, D3: the machine's inbox port. */
void inbox_probe_external_enqueue(const inbox_probe_t *sm, const inbox_probe_event_with_meta_t *evt, bool posted,
                                  bool entered) {
    op_t op;
    (void)sm;
    if (!g_recording) {
        return;
    }
    if (!entered) {
        g_enqueue_invoked = tick();
        return;
    }
    op.kind = OP_ENQUEUE;
    op.session = SESSION;
    op.value = value_of(evt->data);
    op.outcome = POST_TAKEN;
    op.own = !posted;
    op.invoked = g_enqueue_invoked;
    op.returned = tick();
    log_add(&g_owner, &op);
}

/* ── The machine and its posts ─────────────────────────────────────── */

static inbox_probe_t g_sm;

/* The host's half: it answers nothing on a start, so the work runs past the
   call, and keeps nothing. Called on the owner only. */
static void serve(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    (void)user_data;
    (void)event;
    (void)out;
}

/* docs/adr/0086, section 8.5 (a): `inbox_probe` has a script engine, so a post
   hands over a text the poster allocated with the function that releases it. */
static void release_text(void *context, const char *text) {
    (void)context;
    free((void *)(uintptr_t)text);
}

/* `"<value>"`, in storage the caller hands over with `release_text`. */
static const char *handed_value(uint64_t value) {
    char quoted[32];
    (void)snprintf(quoted, sizeof(quoted), "\"%llu\"", (unsigned long long)value);
    const size_t size = strlen(quoted) + 1u;
    char *text = (char *)malloc(size);
    if (text == NULL) {
        (void)fprintf(stderr, "inbox history: out of memory making a post\n");
        exit(2);
    }
    memcpy(text, quoted, size);
    return text;
}

/* Take back the text of `input`, a post the inbox refused, which the poster
   therefore still owns. */
static void take_back(const inbox_probe_inbox_input_t *input) {
    if (input->kind == INBOX_PROBE_INBOX_EXTERNAL_EVENT) {
        release_text(NULL, input->as.event.record.data);
    } else if (input->kind == INBOX_PROBE_INBOX_EXTERNAL_EVENT_BY_NAME) {
        release_text(NULL, input->as.by_name.event.data);
    } else if (input->kind == INBOX_PROBE_INBOX_HOST_INVOKE_ENDING) {
        release_text(NULL, input->as.ending.record.data);
    }
}

/* `ping` carrying `value`, as the document's own event. */
static void ping(inbox_probe_inbox_input_t *input, uint64_t value) {
    memset(input, 0, sizeof(*input));
    input->kind = INBOX_PROBE_INBOX_EXTERNAL_EVENT;
    (void)inbox_probe_resolve_event_by_name("ping", &input->as.event.record.event);
    input->as.event.record.data = handed_value(value);
    input->as.event.release = release_text;
    input->as.event.release_context = NULL;
}

/* `ping` carrying `value`, named by text, as a host that knows no enumeration
   names it. */
static void ping_by_name(inbox_probe_inbox_input_t *input, uint64_t value) {
    memset(input, 0, sizeof(*input));
    input->kind = INBOX_PROBE_INBOX_EXTERNAL_EVENT_BY_NAME;
    (void)snprintf(input->as.by_name.event.name, sizeof(input->as.by_name.event.name), "%s", "ping");
    input->as.by_name.event.data = handed_value(value);
    (void)snprintf(input->as.by_name.event.type, sizeof(input->as.by_name.event.type), "%s", "external");
    input->as.by_name.release = release_text;
    input->as.by_name.release_context = NULL;
}

/* An ending naming the session the tree does not hold, carrying `value`. No run
   here starts an invocation of that session, so were it taken it would raise
   nothing, and its input would never enter. */
static void naming_the_absent_session(inbox_probe_inbox_input_t *input, uint64_t value) {
    memset(input, 0, sizeof(*input));
    input->kind = INBOX_PROBE_INBOX_HOST_INVOKE_ENDING;
    (void)snprintf(input->as.ending.record.session_id, sizeof(input->as.ending.record.session_id), "%s", ABSENT_ID);
    (void)snprintf(input->as.ending.record.processor_type, sizeof(input->as.ending.record.processor_type), "%s", TYPE);
    (void)snprintf(input->as.ending.record.invoke_id, sizeof(input->as.ending.record.invoke_id), "%s", "job");
    input->as.ending.record.token = 0u;
    input->as.ending.record.done = true;
    input->as.ending.record.data = handed_value(value);
    input->as.ending.release = release_text;
    input->as.ending.release_context = NULL;
}

/* Post `input`, input `value` of session `session`, recorded in `log` with the
   clock read immediately around it. The inbox answers a post with whether it took
   it; a post to the machine's own inbox is refused only for room, and one naming
   the session the tree does not hold only for that. */
static void record_post(op_log_t *log, uint64_t session, uint64_t value, inbox_probe_inbox_input_t *input) {
    op_t op;
    op.kind = OP_POST;
    op.session = session;
    op.value = value;
    op.own = false;
    op.invoked = tick();
    const bool taken = inbox_probe_inbox_post(&g_sm, input);
    op.returned = tick();
    if (taken) {
        op.outcome = POST_TAKEN;
    } else {
        op.outcome = session == ABSENT ? POST_REFUSED_UNKNOWN : POST_REFUSED_FULL;
        take_back(input);
    }
    log_add(log, &op);
}

/* ── One run ───────────────────────────────────────────────────────── */

/* The first poster's posts and each later poster's. */
static op_log_t g_filler;
static op_log_t g_posters[POSTERS];

static atomic_bool g_released;
static atomic_int g_running;
static atomic_int g_made;

/* The values of a run's posts: the first poster's from 1, each later poster's
   past them, `PER_POSTER` apart. */
#define FILLER_POSTS (SCE_MAX_INBOX + 1)

static uint64_t input_of(int poster, int k) {
    return (uint64_t)FILLER_POSTS + 1u + (uint64_t)poster * (uint64_t)PER_POSTER + (uint64_t)k;
}

static void *fill(void *arg) {
    (void)arg;
    for (int k = 0; k < FILLER_POSTS; k++) {
        inbox_probe_inbox_input_t input;
        const uint64_t value = (uint64_t)k + 1u;
        ping(&input, value);
        record_post(&g_filler, SESSION, value, &input);
    }
    return NULL;
}

static void *post(void *arg) {
    const int poster = *(const int *)arg;
    op_log_t *log = &g_posters[poster];
    while (!atomic_load(&g_released)) {
        (void)sched_yield();
    }
    for (int k = 0; k < PER_POSTER; k++) {
        inbox_probe_inbox_input_t input;
        const uint64_t value = input_of(poster, k);
        switch (k % 6) {
        case 5:
            naming_the_absent_session(&input, value);
            record_post(log, ABSENT, value, &input);
            break;
        case 1:
        case 3:
            ping_by_name(&input, value);
            record_post(log, SESSION, value, &input);
            break;
        default:
            ping(&input, value);
            record_post(log, SESSION, value, &input);
            break;
        }
        (void)atomic_fetch_add(&g_made, 1);
        (void)sched_yield();
    }
    (void)atomic_fetch_sub(&g_running, 1);
    return NULL;
}

/* Raise the own event `value` on the owner, as the host's own raise does. */
static void raise_own(uint64_t value) {
    char data[32];
    inbox_probe_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    (void)inbox_probe_resolve_event_by_name("ping", &meta.event);
    (void)snprintf(data, sizeof(data), "\"%llu\"", (unsigned long long)value);
    /* A view of the data, which the machine's queue copies. */
    meta.data = data;
    inbox_probe_raise_external(&g_sm, &meta);
}

/* Stop the machine, recorded as the end of its run with the clock read
   immediately around it. */
static void record_end(void) {
    op_t op;
    op.kind = OP_END;
    op.session = SESSION;
    op.value = 0u;
    op.outcome = POST_TAKEN;
    op.own = false;
    op.invoked = tick();
    inbox_probe_stop(&g_sm);
    op.returned = tick();
    log_add(&g_owner, &op);
}

static size_t count_taken(const op_log_t *log) {
    size_t taken = 0u;
    for (size_t i = 0u; i < log->count; i++) {
        taken += log->ops[i].outcome == POST_TAKEN ? 1u : 0u;
    }
    return taken;
}

static size_t count_entered(bool own) {
    size_t entered = 0u;
    for (size_t i = 0u; i < g_owner.count; i++) {
        entered += g_owner.ops[i].kind == OP_ENQUEUE && g_owner.ops[i].own == own ? 1u : 0u;
    }
    return entered;
}

static size_t count_posts(post_outcome_t outcome) {
    size_t count = 0u;
    for (size_t i = 0u; i < g_filler.count; i++) {
        count += g_filler.ops[i].outcome == outcome ? 1u : 0u;
    }
    for (int p = 0; p < POSTERS; p++) {
        for (size_t i = 0u; i < g_posters[p].count; i++) {
            count += g_posters[p].ops[i].outcome == outcome ? 1u : 0u;
        }
    }
    return count;
}

/* Run once; false, saying why, when the run could not be made. */
static bool run_once(void) {
    sce_host_invoker_registry_t wiring;
    pthread_t filler;
    pthread_t threads[POSTERS];
    int indices[POSTERS];
    bool ended = false;

    log_clear(&g_owner);
    log_clear(&g_filler);
    for (int p = 0; p < POSTERS; p++) {
        log_clear(&g_posters[p]);
    }
    atomic_store(&g_clock, 0u);
    memset(&wiring, 0, sizeof(wiring));
    (void)sce_host_invoker_register(&wiring, TYPE, serve, NULL);
    REQUIRE_STARTED(inbox_probe_init_with_host_invokers(&g_sm, &wiring), INBOX_PROBE_START_OK);
    inbox_probe_step(&g_sm);
    g_recording = true;

    /* One input more than the inbox holds, while the owner stands between two
       drives: the last is refused as full. */
    if (pthread_create(&filler, NULL, fill, NULL) != 0) {
        (void)fprintf(stderr, "FAIL: the first poster could not be started\n");
        return false;
    }
    (void)pthread_join(filler, NULL);
    /* An event of the owner's own behind what the first poster's posts left in
       the inbox, so every run enters one. */
    uint64_t own = OWN_BASE;
    raise_own(own);
    own++;

    atomic_store(&g_released, false);
    atomic_store(&g_running, POSTERS);
    atomic_store(&g_made, 0);
    for (int p = 0; p < POSTERS; p++) {
        indices[p] = p;
        if (pthread_create(&threads[p], NULL, post, &indices[p]) != 0) {
            (void)fprintf(stderr, "FAIL: poster %d could not be started\n", p);
            return false;
        }
    }
    atomic_store(&g_released, true);

    /* The owner: a drive, an own event now and then, and the stop once half the
       posts are made. */
    unsigned round = 0u;
    while (atomic_load(&g_running) > 0) {
        inbox_probe_step(&g_sm);
        round++;
        if (!ended && round % 3u == 0u && own < OWN_BASE + OWN_PER_RUN) {
            raise_own(own);
            own++;
        }
        if (!ended && atomic_load(&g_made) * 2 >= TOTAL_POSTS) {
            record_end();
            ended = true;
        }
        (void)sched_yield();
    }
    for (int p = 0; p < POSTERS; p++) {
        (void)pthread_join(threads[p], NULL);
    }
    if (!ended) {
        record_end();
    }
    /* What the last posts left in the inbox, applied where the owner drives the
       machine again. */
    size_t taken = count_taken(&g_filler);
    for (int p = 0; p < POSTERS; p++) {
        taken += count_taken(&g_posters[p]);
    }
    for (int i = 0; i < FINAL_DRIVES && count_entered(false) < taken; i++) {
        inbox_probe_step(&g_sm);
    }
    g_recording = false;
    inbox_probe_destroy(&g_sm);

    if (count_entered(false) != taken) {
        (void)fprintf(stderr, "FAIL: %zu inputs entered the queue of the %zu posts taken\n", count_entered(false),
                      taken);
        return false;
    }
    if (count_posts(POST_REFUSED_FULL) == 0u || count_posts(POST_REFUSED_UNKNOWN) == 0u || count_entered(true) == 0u) {
        (void)fprintf(stderr,
                      "FAIL: the run was refused as full %zu times and as unknown %zu times, and entered %zu "
                      "events of the owner's own, where each was due at least once\n",
                      count_posts(POST_REFUSED_FULL), count_posts(POST_REFUSED_UNKNOWN), count_entered(true));
        return false;
    }
    return true;
}

/* ── The JSON form ─────────────────────────────────────────────────── */

static void write_post(FILE *out, const op_t *op) {
    (void)fprintf(out, "{\"call\": \"post\", \"session\": %llu, \"input\": %llu, ", (unsigned long long)op->session,
                  (unsigned long long)op->value);
    switch (op->outcome) {
    case POST_TAKEN:
        (void)fprintf(out, "\"outcome\": \"taken\", ");
        break;
    case POST_REFUSED_FULL:
        (void)fprintf(out, "\"outcome\": \"refused\", \"reason\": \"full\", ");
        break;
    case POST_REFUSED_UNKNOWN:
        (void)fprintf(out, "\"outcome\": \"refused\", \"reason\": \"unknown\", ");
        break;
    }
    (void)fprintf(out, "\"invoked\": %llu, \"returned\": %llu}", (unsigned long long)op->invoked,
                  (unsigned long long)op->returned);
}

static void write_posts(FILE *out, const op_log_t *log) {
    (void)fputc('[', out);
    for (size_t i = 0u; i < log->count; i++) {
        (void)fputs(i == 0u ? "\n      " : ",\n      ", out);
        write_post(out, &log->ops[i]);
    }
    (void)fputs("\n    ]", out);
}

static void write_owner_operation(FILE *out, const op_t *op) {
    if (op->kind == OP_END) {
        (void)fprintf(out, "{\"call\": \"end\", \"session\": %llu, ", (unsigned long long)op->session);
    } else {
        (void)fprintf(out, "{\"call\": \"enqueue\", \"session\": %llu, \"item\": %llu, \"source\": \"%s\", ",
                      (unsigned long long)op->session, (unsigned long long)op->value, op->own ? "own" : "posted");
    }
    (void)fprintf(out, "\"invoked\": %llu, \"returned\": %llu}", (unsigned long long)op->invoked,
                  (unsigned long long)op->returned);
}

/* Write the run as `<dir>/inbox_c11_<row>_run<run>.json`; false when it could
   not be written. */
static bool write_history(const char *dir, int run) {
    char path[1024];
    (void)snprintf(path, sizeof(path), "%s/inbox_c11_%d_run%02d.json", dir, SCE_INBOX_ATOMIC_RMW_WIDTH, run);
    FILE *out = fopen(path, "w");
    if (out == NULL) {
        (void)fprintf(stderr, "FAIL: %s could not be written\n", path);
        return false;
    }
    (void)fprintf(out, "{\n  \"version\": 2,\n  \"sessions\": [%u],\n  \"closes\": false,\n  \"capacity\": %d,\n",
                  SESSION, SCE_MAX_INBOX);
    (void)fputs("  \"owner\": [", out);
    for (size_t i = 0u; i < g_owner.count; i++) {
        (void)fputs(i == 0u ? "\n    " : ",\n    ", out);
        write_owner_operation(out, &g_owner.ops[i]);
    }
    (void)fputs("\n  ],\n  \"posters\": [\n    ", out);
    write_posts(out, &g_filler);
    for (int p = 0; p < POSTERS; p++) {
        (void)fputs(",\n    ", out);
        write_posts(out, &g_posters[p]);
    }
    (void)fputs("\n  ]\n}\n", out);
    return fclose(out) == 0;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        (void)fprintf(stderr, "usage: %s <directory the histories are written to>\n", argv[0]);
        return 2;
    }
    if (mkdir(argv[1], 0777) != 0 && errno != EEXIST) {
        (void)fprintf(stderr, "FAIL: %s could not be made\n", argv[1]);
        return 2;
    }
    int bad = 0;
    for (int run = 0; run < RUNS; run++) {
        if (!run_once() || !write_history(argv[1], run)) {
            (void)fprintf(stderr, "FAIL: run %d of the inbox (SCE_INBOX_ATOMIC_RMW_WIDTH %d, SCE_MAX_INBOX %d)\n", run,
                          SCE_INBOX_ATOMIC_RMW_WIDTH, SCE_MAX_INBOX);
            bad = 1;
        }
    }
    free(g_owner.ops);
    free(g_filler.ops);
    for (int p = 0; p < POSTERS; p++) {
        free(g_posters[p].ops);
    }
    if (bad == 0) {
        (void)printf("PASS: %d runs of the inbox written (SCE_INBOX_ATOMIC_RMW_WIDTH %d, SCE_MAX_INBOX %d)\n", RUNS,
                     SCE_INBOX_ATOMIC_RMW_WIDTH, SCE_MAX_INBOX);
    }
    return bad;
}
