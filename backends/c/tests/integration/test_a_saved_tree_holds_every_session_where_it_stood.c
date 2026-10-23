// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Format 2: a tree of sessions" (docs/adr/0045, section 8) — the C11
// half of the Rust suite's `a_saved_tree_holds_every_session_where_it_stood.rs`: the tree fixture
// saved byte for byte, restored from that text, and refused for a grandchild of another shape and
// a child of a later revision.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml. The shared text,
// sce-build/tests/fixtures/static_datamodel/saved/static_invoke_tree_working.json, is the tree
// `working` leaves:
//
//   - `worker` was started twice (`rest`, `back`), and has taken `go`: it counted a step, holds a
//     delayed `report` to its parent, asks its host for the time (a host-run invocation that has
//     not answered), and runs a grandchild under the hybrid candidate `static_hybrid_watcher`,
//     whose `pick` was changed to the other candidate after it started (`swap`); it keeps the id
//     the grandchild was started under in `helper_id` (`idlocation`).
//   - The parent holds a delayed `nudge` to the child (`arm`) and the child's `answer` in its
//     queue; the child holds the parent's `swap` in its own.
//
// The root is named by the id the process drew, where the shared text names it `fixture-root`: a
// C11 machine is given no id at creation (it draws `<name>_session_<n>`), and the id carries on
// through a restore. So the text a run saves is held to the shared one once its drawn id is
// replaced by the shared one, in the root's id and in every child's, which are the root's with a
// suffix.

// `strstr` and `snprintf` are C99; the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include "static_invoke_tree_sm.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_SAVED_FIXTURES_DIR
#error "SCE_SAVED_FIXTURES_DIR names sce-build/tests/fixtures/static_datamodel/saved; the build defines it"
#endif

#define TREE static_invoke_tree
#define CLOCK_TYPE "sce-host-clock-now"
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

enum { TEXT_CAP = 32768 };

typedef static_invoke_tree_t tree_t;

// What the host that serves the clock was asked: it never answers, so the invocation is running
// when the tree is saved.
typedef struct {
    int starts;
    char session[96];
} clock_host_t;

static void clock_invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out) {
    clock_host_t *host = (clock_host_t *)user_data;
    (void)out;
    if (event->phase == SCE_HOST_INVOKE_START) {
        host->starts++;
        (void)snprintf(host->session, sizeof(host->session), "%s", event->session_id);
    }
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

static void shared_text(char *out, size_t cap, size_t *len) {
    char path[1024];
    (void)snprintf(path, sizeof(path), "%s/static_invoke_tree_working.json", SCE_SAVED_FIXTURES_DIR);
    CHECK(read_trimmed(path, out, cap, len), "cannot read %s", path);
}

// `text` with every `from` made `to`: the text of a run, named by the id it drew, as the shared
// text names it.
static void replace_all(const char *text, const char *from, const char *to, char *out, size_t cap) {
    const size_t from_len = strlen(from);
    size_t at = 0u;
    while (*text != '\0' && at + 1u < cap) {
        if (strncmp(text, from, from_len) == 0) {
            const size_t to_len = strlen(to);
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

static void raise_named(tree_t *sm, const char *name) {
    static_invoke_tree_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    CHECK(static_invoke_tree_resolve_event_by_name(name, &meta.event), "the document names %s", name);
    static_invoke_tree_raise_external(sm, &meta);
}

static void tick(tree_t *sm) {
    static_invoke_tree_tick(sm);
}

// The tree, started: the host serves the clock and never answers.
static void started(tree_t *sm, clock_host_t *host) {
    sce_host_invoker_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    CHECK(sce_host_invoker_register(&wiring, CLOCK_TYPE, clock_invoker, host), "the invoker registers");
    static_invoke_tree_init_with_clock_and_host_invokers(sm, sce_clock_manual(0u), &wiring);
}

// The run the shared text is the save of.
static void tree_working(tree_t *sm, clock_host_t *host) {
    static const char *const moves[] = {"rest", "back"};
    started(sm, host);
    // Started twice: left and entered again.
    for (size_t i = 0u; i < sizeof(moves) / sizeof(moves[0]); ++i) {
        raise_named(sm, moves[i]);
        tick(sm);
    }
    // `go`, forwarded in one tick and taken by the child in the next.
    raise_named(sm, "go");
    tick(sm);
    tick(sm);
    CHECK(host->starts == 1, "the child asked its host once: %d starts", host->starts);
    // `swap`, forwarded and taken: `pick` names the other candidate now.
    raise_named(sm, "swap");
    tick(sm);
    tick(sm);
    // A delayed send to the child.
    raise_named(sm, "arm");
    tick(sm);
    // `ask`, then `swap`, taken two to a call: the child answers `question` at once, behind the
    // host's `swap`, and the call ends after the parent takes `swap`, which it forwards to the
    // child, so the child's answer waits in the parent's queue and the parent's `swap` in the
    // child's.
    CHECK(static_invoke_tree_set_max_external_events_per_call(sm, 2u), "the budget is set");
    raise_named(sm, "ask");
    raise_named(sm, "swap");
    tick(sm);
}

// The tree saved, once the id it drew is the shared one.
static bool saved_as_shared(const tree_t *sm, char *out, size_t cap) {
    static char text[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    if (!static_invoke_tree_save(sm, SAVED_AT_MS, text, sizeof(text), &len, &err)) {
        CHECK(false, "the tree saves: %s", err.message);
        return false;
    }
    replace_all(text, static_invoke_tree_session_id(sm), SHARED_ROOT, out, cap);
    return true;
}

static bool restored_from(tree_t *sm, const char *text, sce_saved_error_t *err) {
    static_invoke_tree_restore_args_t args;
    memset(&args, 0, sizeof(args));
    args.clock = sce_clock_manual(0u);
    return static_invoke_tree_restore(sm, text, strlen(text), SAVED_AT_MS, &args, err);
}

// ── What the tree holds ─────────────────────────────────────────────────────

static void the_tree_holds_every_session_where_it_stood(void) {
    static tree_t sm;
    static char text[TEXT_CAP];
    clock_host_t host;
    sce_saved_session_t root;
    sce_saved_error_t err;
    sce_jiter_t it;
    sce_jv_t item;
    sce_saved_child_t worker;
    sce_saved_session_t child;
    memset(&host, 0, sizeof(host));
    tree_working(&sm, &host);
    if (!saved_as_shared(&sm, text, sizeof(text))) {
        return;
    }
    CHECK(sce_saved_read(text, strlen(text), &root, &err), "%s", err.message);
    CHECK(sce_jv_string_is(root.session_id, SHARED_ROOT) && sce_jv_string_is(root.current, "working"),
          "the root stands in `working`");
    CHECK(sce_saved_invoke_start(&root, "worker") == 2, "the worker was started twice");
    CHECK(sce_jv_count(root.external) == 1u, "the child's answer waits in the parent's queue");
    CHECK(sce_jv_count(root.invokes) == 1u, "one running child");
    CHECK(sce_jv_iter(root.invokes, &it) && sce_jiter_element(&it, &item), "the child is there");
    CHECK(sce_saved_child_read(item, 0u, &worker, &err), "%s", err.message);
    CHECK(sce_jv_string_is(worker.id, "worker") && worker.start == 2 &&
              sce_jv_string_is(worker.invoke_id, "working.2.worker"),
          "the worker is start 2 under `working.2.worker`");
    CHECK(sce_saved_session_read(worker.session, SCE_SAVED_FORMAT_2, &child, &err), "%s", err.message);
    CHECK(sce_jv_string_is(child.session_id, SHARED_ROOT ".working.2.worker"), "the child's session id");
    CHECK(sce_jv_string_is(child.current, "second"), "the child took `go`");
    CHECK(sce_jv_count(child.invokes) == 1u, "one grandchild");
    CHECK(sce_jv_count(child.host_invokes) == 1u, "the child's host-run invocation runs");
    CHECK(strstr(host.session, ".working.2.worker") != NULL, "the request names the child's session: %s", host.session);
}

static void the_shared_text_restores_and_saves_again_as_it_was(void) {
    static tree_t sm;
    static char want[TEXT_CAP];
    static char again[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    shared_text(want, sizeof(want), &len);
    CHECK(restored_from(&sm, want, &err), "restores: %s", err.message);
    {
        size_t written = 0u;
        CHECK(static_invoke_tree_save(&sm, SAVED_AT_MS, again, sizeof(again), &written, &err), "%s", err.message);
        CHECK(strcmp(again, want) == 0, "a restored tree stands where the saved one stood:\n%s\nwant\n%s", again, want);
    }
}

// ── A tree that is not the document's ───────────────────────────────────────

static void a_grandchild_of_another_shape_refuses_the_tree_by_its_path(void) {
    static tree_t sm;
    static char shared[TEXT_CAP];
    static char changed[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    const char *shape = "a85ea43d8de628b004074e59c0ca90c198a4f7736073bd953750bca35c174c7a";
    shared_text(shared, sizeof(shared), &len);
    replace_all(shared, shape, "0000000000000000000000000000000000000000000000000000000000000000", changed,
                sizeof(changed));
    CHECK(strcmp(shared, changed) != 0, "the change applied");
    CHECK(!restored_from(&sm, changed, &err), "restored");
    CHECK(strstr(err.message, "invokes[0].session.invokes[0].session") != NULL && strstr(err.message, "shape") != NULL,
          "the refusal names the grandchild's path: %s", err.message);
}

static void a_child_of_a_later_revision_refuses_the_tree_by_its_path(void) {
    // The child holds a record of `schema_revised.scxml`, saved at its third revision; a fourth this
    // build does not know is refused, naming the child.
    static tree_t sm;
    static char shared[TEXT_CAP];
    static char changed[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    shared_text(shared, sizeof(shared), &len);
    CHECK(strstr(shared, "\"revision\":3") != NULL, "the child names the revision of its record");
    replace_all(shared, "\"revision\":3", "\"revision\":4", changed, sizeof(changed));
    CHECK(!restored_from(&sm, changed, &err), "restored");
    CHECK(strstr(err.message, "invokes[0].session") != NULL && strstr(err.message, "later than") != NULL,
          "the refusal names the child's path: %s", err.message);
}

// ── Every boundary a host calls at ──────────────────────────────────────────

static void a_save_after_any_host_call_finds_nothing_in_transit(void) {
    // After each call of a run that drives the child every way there is (a send made at once, a
    // delayed one armed, a forwarded event, a tick, a start again), the tree saves, the text it saves
    // restores, and the restored tree saves that text again.
    static const char *const events[] = {"ask", "go",   "ask",  "swap", "ask", "arm",
                                         "ask", "rest", "back", "ask",  "go",  "ask"};
    static tree_t sm;
    static tree_t copied;
    static char text[TEXT_CAP];
    static char again[TEXT_CAP];
    clock_host_t host;
    memset(&host, 0, sizeof(host));
    started(&sm, &host);
    for (size_t i = 0u; i < sizeof(events) / sizeof(events[0]); ++i) {
        for (int after = 0; after < 2; ++after) {
            size_t len = 0u;
            size_t written = 0u;
            sce_saved_error_t err;
            if (after == 0) {
                raise_named(&sm, events[i]);
            } else {
                tick(&sm);
            }
            if (!static_invoke_tree_save(&sm, SAVED_AT_MS, text, sizeof(text), &len, &err)) {
                CHECK(false, "the tree saves at every boundary, but after %s %s: %s",
                      after == 0 ? "raising" : "ticking", events[i], err.message);
                return;
            }
            if (!restored_from(&copied, text, &err)) {
                CHECK(false, "the text saved after %s restores: %s", events[i], err.message);
                return;
            }
            if (!static_invoke_tree_save(&copied, SAVED_AT_MS, again, sizeof(again), &written, &err)) {
                CHECK(false, "the restored tree saves after %s: %s", events[i], err.message);
                return;
            }
            CHECK(strcmp(again, text) == 0, "the tree restored after %s stands where it was saved:\n%s\nwant\n%s",
                  events[i], again, text);
        }
    }
}

// ── A child's invocation is served by the tree's host ───────────────────────

static void a_child_sessions_host_invocation_is_completed_by_the_session_that_started_it(void) {
    static tree_t sm;
    clock_host_t host;
    memset(&host, 0, sizeof(host));
    tree_working(&sm, &host);
    CHECK(host.starts == 1, "the child's invoker is the tree's: %d starts", host.starts);
    // The request names the child's session, and the completion reaches it wherever it stands.
    CHECK(!static_invoke_tree_complete_host_invoke_in(&sm, static_invoke_tree_session_id(&sm), CLOCK_TYPE, "clock", 0u,
                                                      "{}"),
          "the root has no such invocation");
    CHECK(static_invoke_tree_complete_host_invoke_in(&sm, host.session, CLOCK_TYPE, "clock", 0u, "{}"),
          "the child's session takes the completion of the invocation it started: %s", host.session);
}

static void a_child_sessions_host_invocation_is_failed_by_the_session_that_started_it(void) {
    static tree_t sm;
    clock_host_t host;
    memset(&host, 0, sizeof(host));
    tree_working(&sm, &host);
    CHECK(!static_invoke_tree_fail_host_invoke_in(&sm, static_invoke_tree_session_id(&sm), CLOCK_TYPE, "clock", 0u,
                                                  "{}", NULL, NULL),
          "the root has no such invocation");
    CHECK(!static_invoke_tree_fail_host_invoke_in(&sm, "no.such.session", CLOCK_TYPE, "clock", 0u, "{}", NULL, NULL),
          "the tree holds no such session");
    CHECK(static_invoke_tree_fail_host_invoke_in(&sm, host.session, CLOCK_TYPE, "clock", 0u, "{}", NULL, NULL),
          "the child's session takes the failure of the invocation it started: %s", host.session);
    // A run ends once, by success or failure, whichever comes first.
    CHECK(!static_invoke_tree_fail_host_invoke_in(&sm, host.session, CLOCK_TYPE, "clock", 0u, "{}", NULL, NULL),
          "a failure is accepted once");
    CHECK(!static_invoke_tree_complete_host_invoke_in(&sm, host.session, CLOCK_TYPE, "clock", 0u, "{}"),
          "a run that failed is not completed after");
}

int main(void) {
    the_tree_holds_every_session_where_it_stood();
    the_shared_text_restores_and_saves_again_as_it_was();
    a_grandchild_of_another_shape_refuses_the_tree_by_its_path();
    a_child_of_a_later_revision_refuses_the_tree_by_its_path();
    a_save_after_any_host_call_finds_nothing_in_transit();
    a_child_sessions_host_invocation_is_completed_by_the_session_that_started_it();
    a_child_sessions_host_invocation_is_failed_by_the_session_that_started_it();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("a saved tree: ok");
    return 0;
}
