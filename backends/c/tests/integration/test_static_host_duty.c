// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0038 under generated C: a machine that states a duty
// (`<sce:action effect="duty">`) takes a host table whose first member says what
// the host does with duties, and refuses a table that only simulates them — or
// says nothing — unless the caller initialised the machine through the entry that
// names a simulated host. A machine that states none has no such member and takes
// any table, as before.
//
// `static_host_duty` is the document the other backends drive
// (`sce-build/tests/fixtures/static_datamodel/static_host_duty.scxml`): `engaged`
// takes the duty `start_keepalive` on entry beside the notice `note_engaged` and
// gives the duty back through `stop_keepalive` on its <onexit>. `static_host_call`
// states no duty. `static_host_duty_child` (tests/integration) invokes `worker`, a
// child that states a duty; its parent states none, and the opt-in its caller gives
// covers the child it begins.
//
// Linked WITHOUT `sce_c_scripting` and `lua54`: the machines have no script
// engine, so the link is the proof.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_host_call_sm.h"
#include "static_host_duty_child_sm.h"
#include "static_host_duty_sm.h"

#define CALLS_MAX 8

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
        (void)fprintf(stderr, "static_host_duty: FAIL - %s\n", what);
        return 1;
    }
    return 0;
}

static int expect_count(unsigned got, unsigned want, const char *what) {
    if (got != want) {
        (void)fprintf(stderr, "static_host_duty: FAIL - %s: %u, want %u\n", what, got, want);
        return 1;
    }
    return 0;
}

// The calls a recorder holds, against the calls expected, in order.
static int calls_are(const char *what, const recorder_t *r, const char *const *want, int want_count) {
    int bad = expect_count((unsigned)r->count, (unsigned)want_count, what);
    for (int i = 0; i < want_count && i < r->count && i < CALLS_MAX; ++i) {
        if (strcmp(r->calls[i], want[i]) != 0) {
            (void)fprintf(stderr, "static_host_duty: FAIL - %s: call %d is \"%s\", want \"%s\"\n", what, i, r->calls[i],
                          want[i]);
            bad = 1;
        }
    }
    return bad;
}

#define STATIC_MACHINE_SEND(M)                                                                                         \
    static void M##_send(M##_t *sm, const char *name) {                                                                \
        M##_event_t event;                                                                                             \
        if (!M##_resolve_event_by_name(name, &event)) {                                                                \
            (void)fprintf(stderr, "static_host_duty: FAIL - no event %s\n", name);                                     \
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

STATIC_MACHINE_SEND(static_host_duty)
STATIC_MACHINE_SEND(static_host_duty_child)

// ── static_host_duty ─────────────────────────────────────────────────────

static void on_start_keepalive(void *user_data) {
    record((recorder_t *)user_data, "start_keepalive");
}

static void on_note_engaged(void *user_data) {
    record((recorder_t *)user_data, "note_engaged");
}

static void on_stop_keepalive(void *user_data) {
    record((recorder_t *)user_data, "stop_keepalive");
}

// A complete table that says `realization`. Every case below but the unstated one
// writes the member, as the interface asks of a host.
static void duty_table(static_host_duty_actions_t *host, sce_action_realization_t realization, recorder_t *r) {
    memset(host, 0, sizeof(*host));
    host->realization = realization;
    host->start_keepalive = on_start_keepalive;
    host->note_engaged = on_note_engaged;
    host->stop_keepalive = on_stop_keepalive;
    host->user_data = r;
}

// The member that says what the host does is the first of the table, so a host
// reads it before the operations it owes, and the table is the same shape in every
// C compiler that lays a struct out in order.
static int the_realization_leads_the_table(void) {
    return expect_count((unsigned)offsetof(static_host_duty_actions_t, realization), 0u,
                        "the realization is the first member of the table");
}

// A host that performs the duties is accepted by the entry every host uses, and
// the machine takes the duty on entry and gives it back on exit.
static int a_host_that_performs_the_duties_is_accepted(void) {
    static recorder_t r;
    static static_host_duty_t sm;
    static_host_duty_actions_t host;
    memset(&r, 0, sizeof(r));
    duty_table(&host, SCE_ACTION_PERFORMS, &r);
    if (!static_host_duty_init_with_actions(&sm, &host)) {
        (void)fprintf(stderr, "static_host_duty: FAIL - a table that performs was refused\n");
        return 1;
    }
    int bad = expect_count((unsigned)r.count, 0u, "`idle` calls nothing on the host");
    static_host_duty_send(&sm, "engage");
    const char *const engaged[] = {"start_keepalive", "note_engaged"};
    bad |= calls_are("entering `engaged`", &r, engaged, 2);
    static_host_duty_send(&sm, "release");
    const char *const released[] = {"start_keepalive", "note_engaged", "stop_keepalive"};
    bad |= calls_are("leaving `engaged`", &r, released, 3);
    static_host_duty_destroy(&sm);
    return bad;
}

// A table that says it only simulates the duties is refused by the entry every
// host uses: false, nothing called, and the caller's storage as it was — the refusal
// is made before the machine is touched, as an incomplete table's is.
static int a_host_that_only_simulates_the_duties_is_refused(void) {
    static recorder_t r;
    static static_host_duty_t sm;
    static unsigned char before[sizeof(static_host_duty_t)];
    static_host_duty_actions_t host;
    memset(&r, 0, sizeof(r));
    duty_table(&host, SCE_ACTION_SIMULATES, &r);
    memset(&sm, 0xA5, sizeof(sm));
    memcpy(before, &sm, sizeof(sm));
    int bad = expect(!static_host_duty_init_with_actions(&sm, &host), "a table that only simulates was accepted");
    bad |= expect_count((unsigned)r.count, 0u, "a refused host was not called");
    bad |= expect(memcmp(before, &sm, sizeof(sm)) == 0, "a refused call left the machine's storage as it was");
    return bad;
}

// A table that never writes the member reads as the value zero, which is no claim:
// a duty machine refuses it as it refuses a NULL operation, so that leaving the line
// out cannot say `performs`.
static int a_table_that_says_nothing_is_refused(void) {
    static recorder_t r;
    static static_host_duty_t sm;
    static_host_duty_actions_t host;
    memset(&r, 0, sizeof(r));
    duty_table(&host, SCE_ACTION_UNSTATED, &r);
    int bad = expect(!static_host_duty_init_with_actions(&sm, &host), "an unstated table was accepted");
    bad |= expect(!static_host_duty_init_with_simulated_actions(&sm, &host),
                  "an unstated table was accepted through the entry for a simulated host");
    bad |= expect_count((unsigned)r.count, 0u, "a refused host was not called");
    return bad;
}

// The caller names a simulated host by the entry it calls, and the machine runs
// against it: a recorder is the case the entry exists for. A table that performs is
// accepted through it too.
static int a_simulating_host_is_accepted_through_the_entry_that_names_it(void) {
    static recorder_t r;
    static static_host_duty_t sm;
    static_host_duty_actions_t host;
    memset(&r, 0, sizeof(r));
    duty_table(&host, SCE_ACTION_SIMULATES, &r);
    if (!static_host_duty_init_with_simulated_actions(&sm, &host)) {
        (void)fprintf(stderr, "static_host_duty: FAIL - a simulating table was refused by the entry that names it\n");
        return 1;
    }
    static_host_duty_send(&sm, "engage");
    const char *const engaged[] = {"start_keepalive", "note_engaged"};
    int bad = calls_are("entering `engaged`", &r, engaged, 2);
    static_host_duty_destroy(&sm);

    memset(&r, 0, sizeof(r));
    duty_table(&host, SCE_ACTION_PERFORMS, &r);
    bad |= expect(static_host_duty_init_with_simulated_actions(&sm, &host),
                  "a table that performs was refused by the entry for a simulated host");
    static_host_duty_destroy(&sm);
    return bad;
}

// The opt-in lifts the refusal of a simulating table and nothing else: an operation
// the table leaves NULL is still an act nobody performs. `realization` is an enum, so
// the loop that checks the operations is not what reads it.
static int the_opt_in_does_not_excuse_a_missing_operation(void) {
    static recorder_t r;
    static static_host_duty_t sm;
    static_host_duty_actions_t host;
    memset(&r, 0, sizeof(r));
    duty_table(&host, SCE_ACTION_SIMULATES, &r);
    host.stop_keepalive = NULL;
    int bad = expect(!static_host_duty_init_with_simulated_actions(&sm, &host),
                     "a table with a NULL operation was accepted for a simulated host");
    duty_table(&host, SCE_ACTION_PERFORMS, &r);
    host.note_engaged = NULL;
    bad |= expect(!static_host_duty_init_with_actions(&sm, &host), "a table with a NULL operation was accepted");
    return bad;
}

// ── a machine that states no duty ────────────────────────────────────────

static void on_show_attempts(void *user_data, uint32_t count, bool exhausted) {
    (void)count;
    (void)exhausted;
    record((recorder_t *)user_data, "show_attempts");
}

// `show_due` takes arguments that may be absent (docs/adr/0055, step 10), and is called only
// from events this test does not send; the table is complete only with a member for it.
static void on_show_due(void *user_data, sce_optional_int32_t day, sce_optional_bool_t urgent) {
    (void)day;
    (void)urgent;
    record((recorder_t *)user_data, "show_due");
}

// A machine of notices has no realization member: a table written without ever
// naming one is complete, and is accepted.
static int a_machine_that_states_no_duty_takes_a_table_with_no_realization(void) {
    static recorder_t r;
    static static_host_call_t sm;
    memset(&r, 0, sizeof(r));
    static_host_call_actions_t host = {
        .show_attempts = on_show_attempts,
        .show_due = on_show_due,
        .user_data = &r,
    };
    if (!static_host_call_init_with_actions(&sm, &host)) {
        (void)fprintf(stderr, "static_host_duty: FAIL - a table of a machine with no duty was refused\n");
        return 1;
    }
    int bad = expect(r.count > 0, "the machine's first entry called the host");
    static_host_call_destroy(&sm);
    return bad;
}

// ── static_host_duty_child ───────────────────────────────────────────────

typedef static_host_duty_child__sce_synth_invoke__worker_actions_t worker_table_t;

// The child's host: its table, which names its recorder, and the recorder.
typedef struct {
    worker_table_t table;
    recorder_t calls;
} worker_host_t;

static void on_begin_work(void *user_data) {
    record(&((worker_host_t *)user_data)->calls, "begin_work");
}

static void on_end_work(void *user_data) {
    record(&((worker_host_t *)user_data)->calls, "end_work");
}

// The parent's host: it answers the child's table, saying `realization` for it, and
// counts how often it was asked. The parent states no duty, so its own table has no
// realization member to write.
typedef struct {
    int asked;
    sce_action_realization_t worker_realization;
    worker_host_t worker;
    static_host_duty_child_actions_t table;
} parent_host_t;

static const worker_table_t *on_actions_for_worker(void *user_data) {
    parent_host_t *host = (parent_host_t *)user_data;
    host->asked++;
    memset(&host->worker, 0, sizeof(host->worker));
    host->worker.table.realization = host->worker_realization;
    host->worker.table.begin_work = on_begin_work;
    host->worker.table.end_work = on_end_work;
    host->worker.table.user_data = &host->worker;
    return &host->worker.table;
}

static void parent_host_init(parent_host_t *host, sce_action_realization_t worker_realization) {
    memset(host, 0, sizeof(*host));
    host->worker_realization = worker_realization;
    host->table.actions_for_worker = on_actions_for_worker;
    host->table.user_data = host;
}

static void settle(static_host_duty_child_t *sm) {
    for (int i = 0; i < 40; ++i) {
        static_host_duty_child_step(sm);
    }
}

// A child whose host performs its duty is begun by a parent that gave no opt-in,
// and the run goes as a child's does: the duty on entry, given back on exit.
static int a_child_whose_host_performs_is_begun_without_an_opt_in(void) {
    static parent_host_t host;
    static static_host_duty_child_t sm;
    parent_host_init(&host, SCE_ACTION_PERFORMS);
    if (!static_host_duty_child_init_with_actions(&sm, &host.table)) {
        (void)fprintf(stderr, "static_host_duty: FAIL - a complete parent table was refused\n");
        return 1;
    }
    settle(&sm);
    int bad = expect_count((unsigned)host.asked, 1u, "the parent's table was asked once");
    const char *const begun[] = {"begin_work"};
    bad |= calls_are("the child's entry", &host.worker.calls, begun, 1);
    static_host_duty_child_send(&sm, "stop");
    const char *const ended[] = {"begin_work", "end_work"};
    bad |= calls_are("the child's run", &host.worker.calls, ended, 2);
    bad |= expect_count(static_host_duty_child_get_completed(&sm), 1u, "the child ended and the parent counted it");
    static_host_duty_child_destroy(&sm);
    return bad;
}

// A child whose host only simulates its duty is not begun by a parent that gave no
// opt-in: the child meets its host, refuses it, and the invocation does not happen.
static int a_child_whose_host_only_simulates_is_not_begun_without_an_opt_in(void) {
    static parent_host_t host;
    static static_host_duty_child_t sm;
    parent_host_init(&host, SCE_ACTION_SIMULATES);
    if (!static_host_duty_child_init_with_actions(&sm, &host.table)) {
        (void)fprintf(stderr, "static_host_duty: FAIL - a complete parent table was refused\n");
        return 1;
    }
    settle(&sm);
    int bad = expect_count((unsigned)host.asked, 1u, "the parent's table was asked once");
    bad |= expect_count((unsigned)host.worker.calls.count, 0u, "a refused child host was not called");
    static_host_duty_child_send(&sm, "stop");
    bad |= expect_count(static_host_duty_child_get_completed(&sm), 0u, "no child was begun, so none ended");
    static_host_duty_child_destroy(&sm);
    return bad;
}

// The opt-in the caller gave the parent covers the child the parent begins: the
// parent is initialised through the entry that names a simulated host, and the child
// whose host only simulates is begun with it.
static int the_opt_in_of_the_parent_covers_the_child_it_begins(void) {
    static parent_host_t host;
    static static_host_duty_child_t sm;
    parent_host_init(&host, SCE_ACTION_SIMULATES);
    if (!static_host_duty_child_init_with_simulated_actions(&sm, &host.table)) {
        (void)fprintf(stderr, "static_host_duty: FAIL - the entry for a simulated host refused a complete table\n");
        return 1;
    }
    settle(&sm);
    int bad = expect_count((unsigned)host.asked, 1u, "the parent's table was asked once");
    const char *const begun[] = {"begin_work"};
    bad |= calls_are("the child's entry", &host.worker.calls, begun, 1);
    static_host_duty_child_send(&sm, "stop");
    const char *const ended[] = {"begin_work", "end_work"};
    bad |= calls_are("the child's run", &host.worker.calls, ended, 2);
    bad |= expect_count(static_host_duty_child_get_completed(&sm), 1u, "the child ended and the parent counted it");
    static_host_duty_child_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= the_realization_leads_the_table();
    bad |= a_host_that_performs_the_duties_is_accepted();
    bad |= a_host_that_only_simulates_the_duties_is_refused();
    bad |= a_table_that_says_nothing_is_refused();
    bad |= a_simulating_host_is_accepted_through_the_entry_that_names_it();
    bad |= the_opt_in_does_not_excuse_a_missing_operation();
    bad |= a_machine_that_states_no_duty_takes_a_table_with_no_realization();
    bad |= a_child_whose_host_performs_is_begun_without_an_opt_in();
    bad |= a_child_whose_host_only_simulates_is_not_begun_without_an_opt_in();
    bad |= the_opt_in_of_the_parent_covers_the_child_it_begins();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static host duty: ok\n");
    return 0;
}
