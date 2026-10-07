// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions" (docs/adr/0005, decision 6) under
// generated C: a child that declares `<sce:action>`s takes the table of the host
// that performs them when it is begun, because its first `<onentry>` can already
// perform an act — a table installed afterwards would arrive one act too late. So
// the table has to exist when the invocation starts, and the parent obtains it
// from its own: a function of the parent's table that answers a pointer to the
// child's, asked each time the invocation starts and copied before the entry walk.
//
// `static_child_host` invokes `worker`, which announces itself on entry
// (`started`) and reports its steps when it ends (`finished`). The parent declares
// no act: its table is there for the child alone. `static_child_host_hybrid`
// invokes whichever of two candidates its `srcexpr` names, each with an act of its
// own, and the table answered is the one for THAT candidate. A C machine is not
// saved, so no restore is read here; a table that answers NULL is C's own case — a
// start the child's table refuses is an invocation that does not happen.
//
// Linked WITHOUT `sce_c_scripting` and `lua54`: the machines have no script
// engine, so the link is the proof.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_child_host_hybrid_sm.h"
#include "static_child_host_sm.h"

#define CALLS_MAX 8
#define CHILDREN_MAX 4

typedef struct {
    char calls[CALLS_MAX][24];
    int count;
} recorder_t;

static void record(recorder_t *r, const char *text) {
    if (r->count < CALLS_MAX) {
        (void)snprintf(r->calls[r->count], sizeof(r->calls[0]), "%s", text);
    }
    r->count++;
}

static int expect(bool holds, const char *what) {
    if (!holds) {
        (void)fprintf(stderr, "static_child_host: FAIL - %s\n", what);
        return 1;
    }
    return 0;
}

static int expect_count(unsigned got, unsigned want, const char *what) {
    if (got != want) {
        (void)fprintf(stderr, "static_child_host: FAIL - %s: %u, want %u\n", what, got, want);
        return 1;
    }
    return 0;
}

// The calls a recorder holds, against the calls expected, in order.
static int calls_are(const char *what, const recorder_t *r, const char *const *want, int want_count) {
    int bad = expect_count((unsigned)r->count, (unsigned)want_count, what);
    for (int i = 0; i < want_count && i < r->count && i < CALLS_MAX; ++i) {
        if (strcmp(r->calls[i], want[i]) != 0) {
            (void)fprintf(stderr, "static_child_host: FAIL - %s: call %d is \"%s\", want \"%s\"\n", what, i,
                          r->calls[i], want[i]);
            bad = 1;
        }
    }
    return bad;
}

#define STATIC_MACHINE_SEND(M)                                                                                         \
    static void M##_send(M##_t *sm, const char *name) {                                                                \
        M##_event_t event;                                                                                             \
        if (!M##_resolve_event_by_name(name, &event)) {                                                                \
            (void)fprintf(stderr, "static_child_host: FAIL - no event %s\n", name);                                    \
            return;                                                                                                    \
        }                                                                                                              \
        M##_event_with_meta_t meta;                                                                                    \
        memset(&meta, 0, sizeof(meta));                                                                                \
        meta.event = event;                                                                                            \
        M##_raise_external(sm, &meta);                                                                                 \
        for (int i = 0; i < 40; ++i) {                                                                                 \
            M##_step(sm);                                                                                              \
        }                                                                                                              \
    }

STATIC_MACHINE_SEND(static_child_host)
STATIC_MACHINE_SEND(static_child_host_hybrid)

// ── static_child_host ────────────────────────────────────────────────────

typedef static_child_host__sce_synth_invoke__worker_actions_t worker_table_t;

// A child's host: its table, which names its recorder, and the recorder.
typedef struct {
    worker_table_t table;
    recorder_t calls;
} worker_host_t;

static void on_started(void *user_data) {
    record(&((worker_host_t *)user_data)->calls, "started");
}

static void on_finished(void *user_data, uint32_t steps) {
    char text[24];
    (void)snprintf(text, sizeof(text), "finished(%u)", (unsigned)steps);
    record(&((worker_host_t *)user_data)->calls, text);
}

// The parent's host: it answers a fresh child host for each question — or none —
// and keeps every one, in the order asked.
typedef struct {
    int asked;
    bool answer_none;
    worker_host_t workers[CHILDREN_MAX];
    static_child_host_actions_t table;
} parent_host_t;

static const worker_table_t *on_actions_for_worker(void *user_data) {
    parent_host_t *host = (parent_host_t *)user_data;
    if (host->answer_none || host->asked >= CHILDREN_MAX) {
        host->asked++;
        return NULL;
    }
    worker_host_t *worker = &host->workers[host->asked++];
    memset(worker, 0, sizeof(*worker));
    worker->table.started = on_started;
    worker->table.finished = on_finished;
    worker->table.user_data = worker;
    return &worker->table;
}

static void parent_host_init(parent_host_t *host) {
    memset(host, 0, sizeof(*host));
    host->table.actions_for_worker = on_actions_for_worker;
    host->table.user_data = host;
}

static bool start(parent_host_t *host, static_child_host_t *sm) {
    if (!static_child_host_init_with_actions(sm, &host->table)) {
        (void)fprintf(stderr, "static_child_host: FAIL - a complete table was refused\n");
        return false;
    }
    for (int i = 0; i < 40; ++i) {
        static_child_host_step(sm);
    }
    return true;
}

// The child performs its first act through the table its parent answered: the
// table was there when the child was begun, not installed after.
static int the_child_performs_its_first_act_through_the_table_its_parent_answered(void) {
    static parent_host_t host;
    static static_child_host_t sm;
    parent_host_init(&host);
    if (!start(&host, &sm)) {
        return 1;
    }
    const char *const want[] = {"started"};
    int bad = expect_count((unsigned)host.asked, 1u, "the parent's table was asked once");
    bad |= calls_are("the first act", &host.workers[0].calls, want, 1);
    static_child_host_destroy(&sm);
    return bad;
}

static int the_child_reports_what_it_did_through_the_same_table(void) {
    static parent_host_t host;
    static static_child_host_t sm;
    parent_host_init(&host);
    if (!start(&host, &sm)) {
        return 1;
    }
    static_child_host_send(&sm, "a");
    static_child_host_send(&sm, "b");
    const char *const want[] = {"started", "finished(2)"};
    int bad = expect_count(static_child_host_get_completed(&sm), 1u, "the child ended and the parent counted it");
    bad |= calls_are("the child's run", &host.workers[0].calls, want, 2);
    bad |= expect_count((unsigned)host.asked, 1u, "nothing asked the parent's table again");
    static_child_host_destroy(&sm);
    return bad;
}

static int a_state_invoked_again_is_given_a_table_of_its_own(void) {
    static parent_host_t host;
    static static_child_host_t sm;
    parent_host_init(&host);
    if (!start(&host, &sm)) {
        return 1;
    }
    static_child_host_send(&sm, "a");
    static_child_host_send(&sm, "b");
    static_child_host_send(&sm, "again");
    static_child_host_send(&sm, "back");
    const char *const first[] = {"started", "finished(2)"};
    const char *const second[] = {"started"};
    int bad = expect_count((unsigned)host.asked, 2u, "asked once per start");
    // The first child's run is its own, and the second starts from nothing.
    bad |= calls_are("the first child", &host.workers[0].calls, first, 2);
    bad |= calls_are("the second child", &host.workers[1].calls, second, 1);
    static_child_host_destroy(&sm);
    return bad;
}

// A table the parent's table answers NULL for is a child that is not begun: its
// act is not performed, nothing is spawned, and the run does not end.
static int a_table_that_answers_none_is_an_invocation_that_does_not_happen(void) {
    static parent_host_t host;
    static static_child_host_t sm;
    parent_host_init(&host);
    host.answer_none = true;
    if (!start(&host, &sm)) {
        return 1;
    }
    static_child_host_send(&sm, "a");
    static_child_host_send(&sm, "b");
    int bad = expect_count((unsigned)host.asked, 1u, "the parent's table was asked once");
    bad |= expect_count(static_child_host_get_completed(&sm), 0u, "no child was spawned, so none ended");
    static_child_host_destroy(&sm);
    return bad;
}

// ── static_child_host_hybrid ─────────────────────────────────────────────

typedef struct {
    static_hosted_first_actions_t table;
    recorder_t calls;
} first_host_t;

typedef struct {
    static_hosted_second_actions_t table;
    recorder_t calls;
} second_host_t;

static void on_first_ran(void *user_data) {
    record(&((first_host_t *)user_data)->calls, "first_ran");
}

static void on_second_ran(void *user_data) {
    record(&((second_host_t *)user_data)->calls, "second_ran");
}

typedef struct {
    int asked_first;
    int asked_second;
    first_host_t firsts[CHILDREN_MAX];
    second_host_t seconds[CHILDREN_MAX];
    static_child_host_hybrid_actions_t table;
} hybrid_parent_host_t;

static const static_hosted_first_actions_t *on_actions_for_work_first(void *user_data) {
    hybrid_parent_host_t *host = (hybrid_parent_host_t *)user_data;
    first_host_t *first = &host->firsts[host->asked_first++];
    memset(first, 0, sizeof(*first));
    first->table.first_ran = on_first_ran;
    first->table.user_data = first;
    return &first->table;
}

static const static_hosted_second_actions_t *on_actions_for_work_second(void *user_data) {
    hybrid_parent_host_t *host = (hybrid_parent_host_t *)user_data;
    second_host_t *second = &host->seconds[host->asked_second++];
    memset(second, 0, sizeof(*second));
    second->table.second_ran = on_second_ran;
    second->table.user_data = second;
    return &second->table;
}

// A candidate is given the table answered for it and for no other.
static int a_candidate_is_given_the_table_answered_for_it_and_no_other(void) {
    static hybrid_parent_host_t host;
    static static_child_host_hybrid_t sm;
    memset(&host, 0, sizeof(host));
    host.table.actions_for_work_static_hosted_first = on_actions_for_work_first;
    host.table.actions_for_work_static_hosted_second = on_actions_for_work_second;
    host.table.user_data = &host;
    if (!static_child_host_hybrid_init_with_actions(&sm, &host.table)) {
        (void)fprintf(stderr, "static_child_host: FAIL - a complete table was refused\n");
        return 1;
    }
    for (int i = 0; i < 40; ++i) {
        static_child_host_hybrid_step(&sm);
    }
    const char *const first[] = {"first_ran"};
    const char *const second[] = {"second_ran"};
    int bad = expect_count((unsigned)host.asked_first, 1u, "the first candidate's table was asked once");
    bad |= expect_count((unsigned)host.asked_second, 0u, "the second candidate's table was not asked");
    bad |= calls_are("the first candidate", &host.firsts[0].calls, first, 1);
    bad |= expect_count(static_child_host_hybrid_get_completed(&sm), 1u, "it ran and ended");

    static_child_host_hybrid_send(&sm, "again");
    static_child_host_hybrid_send(&sm, "back");
    bad |= expect_count((unsigned)host.asked_first, 1u, "the first candidate's table was not asked again");
    bad |= expect_count((unsigned)host.asked_second, 1u, "the second candidate's table was asked once");
    bad |= calls_are("the second candidate", &host.seconds[0].calls, second, 1);
    // The first candidate's run is its own and was not repeated.
    bad |= calls_are("the first candidate, after", &host.firsts[0].calls, first, 1);
    bad |= expect_count(static_child_host_hybrid_get_completed(&sm), 2u, "both ran and ended");
    static_child_host_hybrid_destroy(&sm);
    return bad;
}

// A table with a member missing is refused by `_init`, as the machine's own is,
// and that covers the member that answers a child's table.
static int a_table_without_the_member_that_answers_a_childs_table_is_refused(void) {
    static parent_host_t host;
    static static_child_host_t sm;
    parent_host_init(&host);
    host.table.actions_for_worker = NULL;
    return expect(!static_child_host_init_with_actions(&sm, &host.table),
                  "a table with no answer for its child's table was accepted");
}

int main(void) {
    int bad = 0;
    bad |= the_child_performs_its_first_act_through_the_table_its_parent_answered();
    bad |= the_child_reports_what_it_did_through_the_same_table();
    bad |= a_state_invoked_again_is_given_a_table_of_its_own();
    bad |= a_table_that_answers_none_is_an_invocation_that_does_not_happen();
    bad |= a_candidate_is_given_the_table_answered_for_it_and_no_other();
    bad |= a_table_without_the_member_that_answers_a_childs_table_is_refused();
    if (bad == 0) {
        (void)printf("static_child_host: PASS\n");
    }
    return bad;
}
