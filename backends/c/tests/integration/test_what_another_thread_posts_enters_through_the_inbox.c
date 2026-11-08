// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, ARCHITECTURE.md "Engine Threading": a generated C11 machine is
// confined to its owner, and what another thread has for it enters through its
// inbox, bounded at the capacity its deployment declares.
//
// The cases hold the inbox to its contract:
//
//   * many threads post endings and external events while the owner runs
//     macrosteps, and what the machine takes, and what its host is told, are
//     those of a single-threaded replay of the same posts;
//   * an ending posted for a start that was cancelled or started again is
//     refused by the token check, as a direct one is, and nothing posted is
//     applied before the owner reaches a macrostep boundary;
//   * a full inbox refuses a post, answers false and counts the refusal, and
//     takes posts again once its owner has applied what it held;
//   * a post made before the owner puts an event of its own on the external
//     queue is taken before that event (W3C test 201's shape), whether the
//     event is a host-run invocation's completion or the host's own raise;
//   * an ending posted for a child session is in that session's inbox when it
//     is posted, so the child takes it before an event its parent sends it
//     later in the same macrostep (ADR 0084, D2 as amended at step 10), and a
//     post naming a session the tree does not hold is refused and counted.
//
// Built once per row of the queue, as a deployment states its atomics
// (`SCE_INBOX_ATOMIC_RMW_WIDTH` 64, 32 and 0), each with `SCE_MAX_INBOX`
// declared (backends/c/tests/CMakeLists.txt).
//
// Fixtures: tests/integration/inbox_probe.scxml, the document the C++ engine's
// inbox test drives, generated with the host-invoker declaration `x-sce-host`.
// `taken` is what the machine took, in order, as `ping:<data>;`,
// `done:<data>;` and `failed:<data>;`. And
// sce-build/tests/fixtures/static_datamodel/static_invoke_ending_order.scxml, a
// tree whose child session `worker` runs a host invocation of its own.

// pthreads, sched_yield and clock_gettime are POSIX, and the target is built
// with C_EXTENSIONS OFF, so the feature-test macro precedes every include.
#define _POSIX_C_SOURCE 200809L

#include <pthread.h>
#include <sched.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

#include "inbox_probe_sm.h"
#include "static_invoke_ending_order_sm.h"

/* The type the fixture was generated for, and its one host-run invocation. */
#define TYPE "x-sce-host"
#define JOB "job"

/* Four threads post forty inputs each: ten times what the inbox holds, so the
   posters fill it and wait for the owner, and the owner drains it while they
   post. */
#define PRODUCERS 4
#define PER_PRODUCER 40
#define POSTS (PRODUCERS * PER_PRODUCER)

#define TAKEN_CAP 8192
#define LOG_CAP 8

/* The host's half: `start:<token>` and `cancel:<token>` in the order the machine
   asked, and the token of the latest start. It answers nothing on a start, so
   the work runs past the call. Called on the owner only. */
typedef struct {
    char entries[LOG_CAP][32];
    int count;
    uint64_t last_start;
} host_log_t;

static void serve(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    host_log_t *log = (host_log_t *)user_data;
    const bool start = event->phase == SCE_HOST_INVOKE_START;
    (void)out;
    if (start) {
        log->last_start = event->token;
    }
    if (log->count >= LOG_CAP) {
        return;
    }
    (void)snprintf(log->entries[log->count], sizeof(log->entries[0]), "%s:%llu", start ? "start" : "cancel",
                   (unsigned long long)event->token);
    log->count++;
}

/* Initialise `sm` with the host's invoker, on the calling thread, which makes it
   the owner. The initial macrostep starts `job`. */
static void boot(inbox_probe_t *sm, host_log_t *log) {
    sce_host_invoker_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    memset(log, 0, sizeof(*log));
    (void)sce_host_invoker_register(&wiring, TYPE, serve, log);
    inbox_probe_init_with_host_invokers(sm, &wiring);
}

/* `ping` with `"<label>"` as its data, as the document's own event. */
static void ping(inbox_probe_inbox_input_t *input, const char *label) {
    memset(input, 0, sizeof(*input));
    input->kind = INBOX_PROBE_INBOX_EXTERNAL_EVENT;
    (void)inbox_probe_resolve_event_by_name("ping", &input->as.event.event);
    (void)snprintf(input->as.event.data, sizeof(input->as.event.data), "\"%s\"", label);
}

/* `ping` with `"<label>"` as its data, named by text, as a peer or a host that
   knows no enumeration names it. */
static void ping_by_name(inbox_probe_inbox_input_t *input, const char *label) {
    memset(input, 0, sizeof(*input));
    input->kind = INBOX_PROBE_INBOX_EXTERNAL_EVENT_BY_NAME;
    (void)snprintf(input->as.by_name.event.name, sizeof(input->as.by_name.event.name), "%s", "ping");
    (void)snprintf(input->as.by_name.data, sizeof(input->as.by_name.data), "\"%s\"", label);
    (void)snprintf(input->as.by_name.event.type, sizeof(input->as.by_name.event.type), "%s", "external");
}

/* An ending of start `token` of `job`: a completion when `done`, a failure
   otherwise, with `"<label>"` as `_event.data`. */
static void ending(inbox_probe_inbox_input_t *input, uint64_t token, bool done, const char *label) {
    memset(input, 0, sizeof(*input));
    input->kind = INBOX_PROBE_INBOX_HOST_INVOKE_ENDING;
    (void)snprintf(input->as.ending.record.processor_type, sizeof(input->as.ending.record.processor_type), "%s", TYPE);
    (void)snprintf(input->as.ending.record.invoke_id, sizeof(input->as.ending.record.invoke_id), "%s", JOB);
    input->as.ending.record.token = token;
    input->as.ending.record.done = done;
    (void)snprintf(input->as.ending.data, sizeof(input->as.ending.data), "\"%s\"", label);
}

/* What the machine holds in `taken`, into `out`; false when it cannot be read. */
static bool taken_of(const inbox_probe_t *sm, char *out, size_t cap) {
    size_t len = 0u;
    if (!inbox_probe_taken(sm, out, cap, &len) || len >= cap) {
        out[0] = '\0';
        return false;
    }
    return true;
}

static int expect_taken(const char *what, const inbox_probe_t *sm, const char *want) {
    static char got[TAKEN_CAP];
    if (!taken_of(sm, got, sizeof(got))) {
        (void)fprintf(stderr, "FAIL: %s: `taken` could not be read\n", what);
        return 1;
    }
    if (strcmp(got, want) != 0) {
        (void)fprintf(stderr, "FAIL: %s:\n  taken `%s`\n  want  `%s`\n", what, got, want);
        return 1;
    }
    return 0;
}

static int expect_log(const char *what, const host_log_t *got, const char *const *want, int want_count) {
    int bad = got->count != want_count;
    for (int i = 0; !bad && i < want_count; i++) {
        bad = strcmp(got->entries[i], want[i]) != 0;
    }
    if (bad) {
        (void)fprintf(stderr, "FAIL: %s: the host was told", what);
        for (int i = 0; i < got->count; i++) {
            (void)fprintf(stderr, " %s", got->entries[i]);
        }
        (void)fprintf(stderr, "\n");
        return 1;
    }
    return 0;
}

/* Raise `name` on the owner, directly, and drive one macrostep. */
static void raise_and_step(inbox_probe_t *sm, const char *name) {
    inbox_probe_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    (void)inbox_probe_resolve_event_by_name(name, &meta.event);
    inbox_probe_raise_external(sm, &meta);
    inbox_probe_step(sm);
}

/* ── Posting from a thread that does not own the machine ───────────── */

typedef struct {
    inbox_probe_t *sm;
    const inbox_probe_inbox_input_t *input;
    bool taken;
} one_post_t;

static void *post_once(void *arg) {
    one_post_t *post = (one_post_t *)arg;
    post->taken = inbox_probe_inbox_post(post->sm, post->input);
    return NULL;
}

/* Post `input` from a thread of its own; whether the inbox took it. Joined
   before it returns, so the post happens-before what the caller does next. */
static bool post_from_another_thread(inbox_probe_t *sm, const inbox_probe_inbox_input_t *input) {
    one_post_t post = {sm, input, false};
    pthread_t poster;
    if (pthread_create(&poster, NULL, post_once, &post) != 0) {
        (void)fprintf(stderr, "FAIL: a posting thread could not be started\n");
        return false;
    }
    (void)pthread_join(poster, NULL);
    return post.taken;
}

/* ── Many threads, one owner, and a single-threaded replay ─────────── */

/* One post of the run, with what the machine records if it takes it. */
typedef struct {
    inbox_probe_inbox_input_t input;
    char recorded[48];
    bool is_ending;
} post_t;

/* What producer `producer` posts as its `k`th input: external events, typed and
   by name, and now and then an ending of `token`, completed or failed. Each
   data names its producer and its place, so the record says which post it came
   from. */
static void post_of(int producer, int k, uint64_t token, post_t *out) {
    char label[32];
    (void)snprintf(label, sizeof(label), "p%d-%d", producer, k);
    out->is_ending = false;
    switch (k % 7) {
    case 3:
        ending(&out->input, token, true, label);
        (void)snprintf(out->recorded, sizeof(out->recorded), "done:%s;", label);
        out->is_ending = true;
        break;
    case 5:
        ending(&out->input, token, false, label);
        (void)snprintf(out->recorded, sizeof(out->recorded), "failed:%s;", label);
        out->is_ending = true;
        break;
    case 1:
        ping_by_name(&out->input, label);
        (void)snprintf(out->recorded, sizeof(out->recorded), "ping:%s;", label);
        break;
    default:
        ping(&out->input, label);
        (void)snprintf(out->recorded, sizeof(out->recorded), "ping:%s;", label);
        break;
    }
}

static inbox_probe_t g_sm;
static host_log_t g_host;

/* The order the inbox took the posts in: each post is made and recorded under
   this lock, so the record is the inbox's own order. */
static pthread_mutex_t g_order = PTHREAD_MUTEX_INITIALIZER;
static post_t g_posted[POSTS];
static int g_posted_count;
static uint32_t g_refusals;

static atomic_bool g_released;
static atomic_int g_running;

/* The waker the host installs: counts the posts the inbox took and wakes the
   owner. */
static pthread_mutex_t g_wake_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t g_woken = PTHREAD_COND_INITIALIZER;
static uint32_t g_wakes;

static void wake(void *context) {
    (void)context;
    (void)pthread_mutex_lock(&g_wake_lock);
    g_wakes++;
    (void)pthread_cond_signal(&g_woken);
    (void)pthread_mutex_unlock(&g_wake_lock);
}

typedef struct {
    int producer;
    uint64_t token;
} producer_t;

static void *produce(void *arg) {
    const producer_t *self = (const producer_t *)arg;
    post_t post;
    while (!atomic_load(&g_released)) {
        (void)sched_yield();
    }
    for (int k = 0; k < PER_PRODUCER; k++) {
        post_of(self->producer, k, self->token, &post);
        for (;;) {
            (void)pthread_mutex_lock(&g_order);
            const bool taken = inbox_probe_inbox_post(&g_sm, &post.input);
            if (taken) {
                g_posted[g_posted_count++] = post;
            } else {
                g_refusals++;
            }
            (void)pthread_mutex_unlock(&g_order);
            if (taken) {
                break;
            }
            /* The inbox is full: the owner has not reached a boundary since. */
            (void)sched_yield();
        }
    }
    (void)atomic_fetch_sub(&g_running, 1);
    (void)pthread_mutex_lock(&g_wake_lock);
    (void)pthread_cond_signal(&g_woken);
    (void)pthread_mutex_unlock(&g_wake_lock);
    return NULL;
}

/* What the machine must take from the posts, in the order the inbox took them:
   every event, and of the endings only the first, since it ends `job` and the
   token check refuses the rest. */
static void expected_from_posts(char *out, size_t cap) {
    bool ended = false;
    size_t len = 0u;
    out[0] = '\0';
    for (int i = 0; i < g_posted_count; i++) {
        if (g_posted[i].is_ending) {
            if (ended) {
                continue;
            }
            ended = true;
        }
        const size_t add = strlen(g_posted[i].recorded);
        if (len + add >= cap) {
            return;
        }
        memcpy(out + len, g_posted[i].recorded, add + 1u);
        len += add;
    }
}

/* Wait for a wake the owner has not seen, or for the producers to finish, for
   at most ten milliseconds, so a lost wake shows as a slow case and not a hung
   one; the count of wakes seen. */
static uint32_t wait_for_posts(uint32_t seen) {
    struct timespec until;
    (void)pthread_mutex_lock(&g_wake_lock);
    if (g_wakes == seen && atomic_load(&g_running) > 0) {
        (void)clock_gettime(CLOCK_REALTIME, &until);
        until.tv_nsec += 10L * 1000L * 1000L;
        if (until.tv_nsec >= 1000L * 1000L * 1000L) {
            until.tv_sec += 1;
            until.tv_nsec -= 1000L * 1000L * 1000L;
        }
        (void)pthread_cond_timedwait(&g_woken, &g_wake_lock, &until);
    }
    seen = g_wakes;
    (void)pthread_mutex_unlock(&g_wake_lock);
    return seen;
}

static int posts_from_many_threads_are_taken_as_a_single_threaded_replay_takes_them(void) {
    static char taken[TAKEN_CAP];
    static char expected[TAKEN_CAP];
    static inbox_probe_t replay;
    host_log_t replay_host;
    pthread_t threads[PRODUCERS];
    producer_t producers[PRODUCERS];
    int bad = 0;

    boot(&g_sm, &g_host);
    /* Installed before any other thread can reach the machine. */
    inbox_probe_inbox_wake_with(&g_sm, wake, NULL);
    const uint64_t token = g_host.last_start;
    const char *const started[] = {g_host.entries[0]};
    if (g_host.count != 1) {
        (void)fprintf(stderr, "FAIL: the initial macrostep did not start `job` once\n");
        inbox_probe_destroy(&g_sm);
        return 1;
    }

    atomic_store(&g_released, false);
    atomic_store(&g_running, PRODUCERS);
    for (int p = 0; p < PRODUCERS; p++) {
        producers[p].producer = p;
        producers[p].token = token;
        if (pthread_create(&threads[p], NULL, produce, &producers[p]) != 0) {
            (void)fprintf(stderr, "FAIL: producer %d could not be started\n", p);
            return 1;
        }
    }
    atomic_store(&g_released, true);

    /* The owner: asleep until a post wakes it, then a drive, for as long as any
       producer runs. */
    uint32_t seen = 0u;
    while (atomic_load(&g_running) > 0) {
        seen = wait_for_posts(seen);
        inbox_probe_step(&g_sm);
    }
    for (int p = 0; p < PRODUCERS; p++) {
        (void)pthread_join(threads[p], NULL);
    }
    /* What the last producer posted after the owner's last drive. */
    inbox_probe_step(&g_sm);

    if (g_posted_count != POSTS) {
        (void)fprintf(stderr, "FAIL: %d of %d posts were taken\n", g_posted_count, POSTS);
        bad = 1;
    }
    if (inbox_probe_refused_inbox_posts(&g_sm) != g_refusals) {
        (void)fprintf(stderr, "FAIL: the inbox counted %u refusals, the posters met %u\n",
                      (unsigned)inbox_probe_refused_inbox_posts(&g_sm), (unsigned)g_refusals);
        bad = 1;
    }
    if (g_wakes != (uint32_t)g_posted_count) {
        (void)fprintf(stderr, "FAIL: the waker was called %u times for %d posts taken\n", (unsigned)g_wakes,
                      g_posted_count);
        bad = 1;
    }
    expected_from_posts(expected, sizeof(expected));
    bad |= expect_taken("the machine took the posts in the order the inbox took them", &g_sm, expected);
    /* The first ending ended `job`, so the host was told of its start and of
       nothing else. */
    bad |= expect_log("an ending was not applied, or a refused one was", &g_host, started, 1);

    /* The same posts, in the same order, from the owner's own thread, driving
       whenever the inbox is full. */
    boot(&replay, &replay_host);
    for (int i = 0; i < g_posted_count; i++) {
        while (!inbox_probe_inbox_post(&replay, &g_posted[i].input)) {
            inbox_probe_step(&replay);
        }
    }
    inbox_probe_step(&replay);
    if (!taken_of(&g_sm, taken, sizeof(taken))) {
        (void)fprintf(stderr, "FAIL: `taken` of the threaded run could not be read\n");
        bad = 1;
    }
    bad |= expect_taken("the run differs from a single-threaded replay of its posts", &replay, taken);
    const char *replayed[LOG_CAP] = {0};
    for (int i = 0; i < replay_host.count; i++) {
        replayed[i] = replay_host.entries[i];
    }
    bad |= expect_log("the invocation differs from a single-threaded replay of the posts", &g_host, replayed,
                      replay_host.count);

    inbox_probe_destroy(&replay);
    inbox_probe_destroy(&g_sm);
    return bad;
}

/* ── The token check ───────────────────────────────────────────────── */

/* What a cancelled process sends is ignored, and a restarted `<invoke>` is a
   different process under the same id (§scxml-6.4). A posted ending is judged
   when the owner applies it, by the check a direct one meets. */
static int a_posted_ending_of_a_cancelled_or_restarted_start_is_refused_as_a_direct_one_is(void) {
    static inbox_probe_t sm;
    host_log_t host;
    inbox_probe_inbox_input_t input;
    char first_start[32];
    char first_cancel[32];
    char second_start[32];
    int bad = 0;

    boot(&sm, &host);
    const uint64_t first = host.last_start;
    raise_and_step(&sm, "leave");
    raise_and_step(&sm, "go");
    const uint64_t second = host.last_start;
    if (first == second) {
        (void)fprintf(stderr, "FAIL: a restart reused the first start's token\n");
        inbox_probe_destroy(&sm);
        return 1;
    }

    ending(&input, first, true, "stale");
    if (!post_from_another_thread(&sm, &input)) {
        (void)fprintf(stderr, "FAIL: the inbox refused a post; the token is the owner's to judge\n");
        bad = 1;
    }
    inbox_probe_step(&sm);
    bad |= expect_taken("the cancelled start's ending was taken", &sm, "");

    ending(&input, second, true, "fresh");
    if (!post_from_another_thread(&sm, &input)) {
        (void)fprintf(stderr, "FAIL: the inbox refused the running start's ending\n");
        bad = 1;
    }
    bad |= expect_taken("a post was applied before the owner reached a macrostep boundary", &sm, "");
    inbox_probe_step(&sm);
    bad |= expect_taken("the running start's ending", &sm, "done:fresh;");

    /* Ended once, so a later ending of the same start finds nothing to end. */
    ending(&input, second, false, "late");
    if (!post_from_another_thread(&sm, &input)) {
        (void)fprintf(stderr, "FAIL: the inbox refused a late ending\n");
        bad = 1;
    }
    inbox_probe_step(&sm);
    bad |= expect_taken("the same start ended twice", &sm, "done:fresh;");

    (void)snprintf(first_start, sizeof(first_start), "start:%llu", (unsigned long long)first);
    (void)snprintf(first_cancel, sizeof(first_cancel), "cancel:%llu", (unsigned long long)first);
    (void)snprintf(second_start, sizeof(second_start), "start:%llu", (unsigned long long)second);
    const char *const told[] = {first_start, first_cancel, second_start};
    bad |= expect_log("the invocation's lifecycle", &host, told, 3);
    inbox_probe_destroy(&sm);
    return bad;
}

/* ── A bounded inbox ───────────────────────────────────────────────── */

/* ADR 0084, D2: a bounded inbox refuses a post when full and says so, and
   nothing is dropped in silence: what it took is applied, in order, and what it
   refused is answered false and counted. */
static int a_full_inbox_refuses_a_post_and_reports_it(void) {
    static inbox_probe_t sm;
    static char want[TAKEN_CAP];
    host_log_t host;
    inbox_probe_inbox_input_t input;
    char label[32];
    size_t len = 0u;
    int bad = 0;

    boot(&sm, &host);
    want[0] = '\0';
    for (int i = 0; i < SCE_MAX_INBOX; i++) {
        (void)snprintf(label, sizeof(label), "f%d", i);
        ping(&input, label);
        if (!post_from_another_thread(&sm, &input)) {
            (void)fprintf(stderr, "FAIL: post %d of %d was refused by an inbox that holds %d\n", i + 1, SCE_MAX_INBOX,
                          SCE_MAX_INBOX);
            bad = 1;
        }
        len += (size_t)snprintf(want + len, sizeof(want) - len, "ping:%s;", label);
    }
    ping(&input, "over");
    if (post_from_another_thread(&sm, &input)) {
        (void)fprintf(stderr, "FAIL: a full inbox took a post\n");
        bad = 1;
    }
    if (inbox_probe_refused_inbox_posts(&sm) != 1u) {
        (void)fprintf(stderr, "FAIL: the refusal was counted %u times, want 1\n",
                      (unsigned)inbox_probe_refused_inbox_posts(&sm));
        bad = 1;
    }
    bad |= expect_taken("a post was applied before the owner reached a macrostep boundary", &sm, "");

    inbox_probe_step(&sm);
    bad |= expect_taken("what a full inbox took", &sm, want);

    /* Drained, it takes posts again. */
    ping(&input, "again");
    if (!post_from_another_thread(&sm, &input)) {
        (void)fprintf(stderr, "FAIL: the drained inbox refused a post\n");
        bad = 1;
    }
    inbox_probe_step(&sm);
    (void)snprintf(want + len, sizeof(want) - len, "ping:again;");
    bad |= expect_taken("a post after the drain", &sm, want);
    if (inbox_probe_refused_inbox_posts(&sm) != 1u) {
        (void)fprintf(stderr, "FAIL: a post the inbox took was counted as refused\n");
        bad = 1;
    }
    inbox_probe_destroy(&sm);
    return bad;
}

/* ── Arrival order on the external queue ───────────────────────────── */

/* The host of W3C test 201's shape: on a start it has a thread of its own post
   `ping` to the machine and waits for it, so the post happens-before what the
   owner does next, and then completes the start at once. */
typedef struct {
    host_log_t log;
    inbox_probe_t *sm;
    bool posted;
} posting_host_t;

static void serve_posting(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    posting_host_t *host = (posting_host_t *)user_data;
    inbox_probe_inbox_input_t input;
    serve(&host->log, event, out);
    if (event->phase != SCE_HOST_INVOKE_START) {
        return;
    }
    ping(&input, "posted");
    host->posted = post_from_another_thread(host->sm, &input);
    (void)sce_host_invoke_answer_done(out, "\"own\"");
}

/* ADR 0084, D2 as amended at step 8. Inside the owner's macrostep a host call
   waits for another thread to post `ping`, and the owner then puts an event of
   its own on the external queue: the `done.invoke.job` of the start the host
   completed at once. The post arrived first, so it is taken first. */
static int a_post_made_before_the_owner_queues_its_own_event_is_taken_before_it(void) {
    static inbox_probe_t sm;
    static posting_host_t host;
    sce_host_invoker_registry_t wiring;
    int bad = 0;

    memset(&wiring, 0, sizeof(wiring));
    memset(&host, 0, sizeof(host));
    host.sm = &sm;
    (void)sce_host_invoker_register(&wiring, TYPE, serve_posting, &host);
    inbox_probe_init_with_host_invokers(&sm, &wiring);
    inbox_probe_step(&sm);
    if (!host.posted) {
        (void)fprintf(stderr, "FAIL: the inbox refused the post made during the start\n");
        bad = 1;
    }
    bad |= expect_taken("the owner's own event was taken ahead of a post that arrived before it", &sm,
                        "ping:posted;done:own;");
    inbox_probe_destroy(&sm);
    return bad;
}

/* A host's own raise on the owner puts an event of the owner's on the external
   queue as well: what another thread posted before it is taken before it. */
static int a_post_made_before_the_hosts_own_raise_is_taken_before_it(void) {
    static inbox_probe_t sm;
    host_log_t host;
    inbox_probe_inbox_input_t input;
    inbox_probe_event_with_meta_t raised;
    int bad = 0;

    boot(&sm, &host);
    ping(&input, "posted");
    if (!post_from_another_thread(&sm, &input)) {
        (void)fprintf(stderr, "FAIL: the inbox refused a post\n");
        bad = 1;
    }
    memset(&raised, 0, sizeof(raised));
    (void)inbox_probe_resolve_event_by_name("ping", &raised.event);
    /* A view of the data, which the machine's queue copies. */
    raised.data = "\"raised\"";
    inbox_probe_raise_external(&sm, &raised);
    inbox_probe_step(&sm);
    bad |= expect_taken("the host's raise was taken ahead of a post that arrived before it", &sm,
                        "ping:posted;ping:raised;");
    inbox_probe_destroy(&sm);
    return bad;
}

/* ── A child session's own inbox ───────────────────────────────────── */

/* `static_invoke_ending_order`: the child session `worker` asks the host for the
   time (`clock`, an invocation of `sce-host-clock-now`) and the host does not
   answer at once. The parent's `ask` calls the host (`beforeQuestion`) and then
   sends `question` to `worker`; `worker` reports `stamped` when the clock's
   ending reaches it and `answer` when `question` does, and the parent numbers
   what it hears in `stamped_at` and `answer_at`. */
#define ORDER_CLOCK_TYPE "sce-host-clock-now"
#define ORDER_CLOCK_ID "clock"
#define ORDER_CLOCK_NOW "{\"error\":\"\",\"utc_ms\":1700000000000,\"monotonic_ms\":5}"

/* The host of the tree: it serves the clock, keeping the session and the token
   of the start without answering, and performs `beforeQuestion` by having a
   thread of its own post the clock's ending, naming `worker`'s session, joined
   before the action returns, so the post happens-before the parent's send. */
typedef struct {
    static_invoke_ending_order_actions_t table;
    static_invoke_ending_order_t *sm;
    int starts;
    uint64_t token;
    char session[SCE_MAX_SESSION_ID_LEN];
    bool asked;
    bool posted;
} ordering_host_t;

static void serve_clock(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    ordering_host_t *host = (ordering_host_t *)user_data;
    (void)out;
    if (event->phase != SCE_HOST_INVOKE_START) {
        return;
    }
    host->starts++;
    host->token = event->token;
    (void)snprintf(host->session, sizeof(host->session), "%s", event->session_id);
}

typedef struct {
    static_invoke_ending_order_t *sm;
    const static_invoke_ending_order_inbox_input_t *input;
    bool taken;
} ordering_post_t;

static void *post_ordering_once(void *arg) {
    ordering_post_t *post = (ordering_post_t *)arg;
    post->taken = static_invoke_ending_order_inbox_post(post->sm, post->input);
    return NULL;
}

/* Post the completion of the clock's start, naming `session`, from a thread of
   its own; whether the tree took it. Joined before it returns, so the post
   happens-before what the caller does next. */
static bool post_clock_ending(const ordering_host_t *host, const char *session) {
    static_invoke_ending_order_inbox_input_t input;
    ordering_post_t post;
    pthread_t poster;
    memset(&input, 0, sizeof(input));
    input.kind = STATIC_INVOKE_ENDING_ORDER_INBOX_HOST_INVOKE_ENDING;
    (void)snprintf(input.as.ending.record.session_id, sizeof(input.as.ending.record.session_id), "%s", session);
    (void)snprintf(input.as.ending.record.processor_type, sizeof(input.as.ending.record.processor_type), "%s",
                   ORDER_CLOCK_TYPE);
    (void)snprintf(input.as.ending.record.invoke_id, sizeof(input.as.ending.record.invoke_id), "%s", ORDER_CLOCK_ID);
    input.as.ending.record.token = host->token;
    input.as.ending.record.done = true;
    (void)snprintf(input.as.ending.data, sizeof(input.as.ending.data), "%s", ORDER_CLOCK_NOW);
    post.sm = host->sm;
    post.input = &input;
    post.taken = false;
    if (pthread_create(&poster, NULL, post_ordering_once, &post) != 0) {
        (void)fprintf(stderr, "FAIL: a posting thread could not be started\n");
        return false;
    }
    (void)pthread_join(poster, NULL);
    return post.taken;
}

static void before_question(void *user_data) {
    ordering_host_t *host = (ordering_host_t *)user_data;
    host->asked = true;
    host->posted = post_clock_ending(host, host->session);
}

/* Initialise `sm` with `host` as its invokers and its table, on the calling
   thread, which makes it the owner, and drive it until `worker` has started
   `clock`; false, with `sm` destroyed, when it did not start it once. */
static bool start_ordering_tree(static_invoke_ending_order_t *sm, ordering_host_t *host) {
    sce_host_invoker_registry_t wiring;
    memset(host, 0, sizeof(*host));
    memset(&wiring, 0, sizeof(wiring));
    host->sm = sm;
    host->table.before_question = before_question;
    host->table.user_data = host;
    (void)sce_host_invoker_register(&wiring, ORDER_CLOCK_TYPE, serve_clock, host);
    if (!static_invoke_ending_order_init_with_host_invokers(sm, &wiring, &host->table)) {
        (void)fprintf(stderr, "FAIL: the machine refused a host table that performs `beforeQuestion`\n");
        return false;
    }
    for (int i = 0; i < 16 && host->starts == 0; i++) {
        static_invoke_ending_order_step(sm);
    }
    if (host->starts != 1 || host->session[0] == '\0') {
        (void)fprintf(stderr, "FAIL: `worker` did not start `clock` once (%d starts)\n", host->starts);
        static_invoke_ending_order_destroy(sm);
        return false;
    }
    return true;
}

/* ADR 0084, D2 as amended at step 10: the address of a posted input is resolved
   when it is posted. The ending names `worker`, so it is on `worker`'s own inbox
   before the parent's `question` is sent, and `worker`'s chokepoint applies it
   before `question` joins its queue: the parent hears `stamped` first. */
static int a_childs_ending_posted_before_its_parents_send_is_taken_before_it(void) {
    static static_invoke_ending_order_t sm;
    static ordering_host_t host;
    static_invoke_ending_order_event_with_meta_t ask;
    int bad = 0;

    if (!start_ordering_tree(&sm, &host)) {
        return 1;
    }

    memset(&ask, 0, sizeof(ask));
    (void)static_invoke_ending_order_resolve_event_by_name("ask", &ask.event);
    static_invoke_ending_order_raise_external(&sm, &ask);
    for (int i = 0; i < 16; i++) {
        static_invoke_ending_order_step(&sm);
    }

    if (!host.asked) {
        (void)fprintf(stderr, "FAIL: the parent took `ask` without calling `beforeQuestion`\n");
        bad = 1;
    }
    if (!host.posted) {
        (void)fprintf(stderr, "FAIL: the tree refused an ending posted for `worker`, a session it holds\n");
        bad = 1;
    }
    const uint32_t stamped_at = static_invoke_ending_order_get_stamped_at(&sm);
    const uint32_t answer_at = static_invoke_ending_order_get_answer_at(&sm);
    if (stamped_at != 1u || answer_at != 2u) {
        (void)fprintf(stderr,
                      "FAIL: `worker` took the parent's `question` ahead of the clock's ending posted for it "
                      "before the send, so the parent heard `stamped` as number %u and `answer` as number %u, "
                      "where 1 and 2 were due\n",
                      (unsigned)stamped_at, (unsigned)answer_at);
        bad = 1;
    }
    static_invoke_ending_order_destroy(&sm);
    return bad;
}

/* The waker the host of the tree installs on the root: counts the posts any
   session's inbox took. */
static void count_tree_wake(void *context) {
    (*(uint32_t *)context)++;
}

/* ADR 0084, D2 as amended at step 10: the tree's handle looks the named session
   up when the post is made. A session the tree does not hold is refused and the
   refusal counted, with nothing woken; one it holds is put in that session's
   inbox, which wakes the waker installed on the root, and the root's drive
   reaches the child that holds it. */
static int a_post_names_a_session_of_the_tree_or_is_refused(void) {
    static static_invoke_ending_order_t sm;
    static ordering_host_t host;
    static uint32_t wakes;
    int bad = 0;

    if (!start_ordering_tree(&sm, &host)) {
        return 1;
    }
    wakes = 0u;
    /* Installed before any other thread can reach the tree. */
    static_invoke_ending_order_inbox_wake_with(&sm, count_tree_wake, &wakes);

    if (post_clock_ending(&host, "no.such.session")) {
        (void)fprintf(stderr, "FAIL: the tree took an ending naming a session it does not hold\n");
        bad = 1;
    }
    if (static_invoke_ending_order_refused_inbox_posts(&sm) != 1u || wakes != 0u) {
        (void)fprintf(stderr,
                      "FAIL: an ending naming no session of the tree was counted %u times as refused and woke "
                      "the owner %u times, where 1 and 0 were due\n",
                      (unsigned)static_invoke_ending_order_refused_inbox_posts(&sm), (unsigned)wakes);
        bad = 1;
    }

    if (!post_clock_ending(&host, host.session)) {
        (void)fprintf(stderr, "FAIL: the tree refused an ending posted for `worker`, a session it holds\n");
        bad = 1;
    }
    if (wakes != 1u) {
        (void)fprintf(stderr, "FAIL: a post to `worker`'s inbox woke the root's waker %u times, where 1 was due\n",
                      (unsigned)wakes);
        bad = 1;
    }
    if (static_invoke_ending_order_get_stamped_at(&sm) != 0u) {
        (void)fprintf(stderr, "FAIL: a post was applied before the owner drove the tree\n");
        bad = 1;
    }
    static_invoke_ending_order_step(&sm);
    static_invoke_ending_order_step(&sm);
    if (static_invoke_ending_order_get_stamped_at(&sm) != 1u) {
        (void)fprintf(stderr, "FAIL: the root's drive did not reach `worker`, whose inbox held the clock's ending; "
                              "the parent never heard `stamped`\n");
        bad = 1;
    }
    static_invoke_ending_order_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= posts_from_many_threads_are_taken_as_a_single_threaded_replay_takes_them();
    bad |= a_posted_ending_of_a_cancelled_or_restarted_start_is_refused_as_a_direct_one_is();
    bad |= a_full_inbox_refuses_a_post_and_reports_it();
    bad |= a_post_made_before_the_owner_queues_its_own_event_is_taken_before_it();
    bad |= a_post_made_before_the_hosts_own_raise_is_taken_before_it();
    bad |= a_childs_ending_posted_before_its_parents_send_is_taken_before_it();
    bad |= a_post_names_a_session_of_the_tree_or_is_refused();
    if (bad == 0) {
        (void)printf("PASS: the inbox (SCE_INBOX_ATOMIC_RMW_WIDTH %d, SCE_MAX_INBOX %d)\n", SCE_INBOX_ATOMIC_RMW_WIDTH,
                     SCE_MAX_INBOX);
    }
    return bad;
}
