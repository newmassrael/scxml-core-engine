// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045): a child session an
// `<invoke type="scxml">` started is part of what a machine is doing, so a saved state holds each
// running child inside its parent, with its whole state, and a restore resumes it where it stood.
// Nothing a child did at its start happens again, and it raises its `done.invoke` once, when it
// ends.
//
// `static_invoke.scxml` invokes `worker`, a child that takes `a` and then `b` and ends; the parent
// forwards what it is sent (`autoforward`), so the host drives the child through the parent and
// every case is read from the parent: `done.invoke.worker` counts the run in `completed` and
// leaves the parent in `working`. A child that had taken `a` is resumed at `second`, so `b` alone
// ends it.
//
// The shared instance `saved/static_invoke_working.json` is the format-2 text of the machine just
// started, its root given the id the fixtures name it by. A format-1 text holds the child's id
// alone, and the build that wrote it promised to start the child again from its beginning:
// `saved/format1/static_invoke_working.json` is the one case in which a restore does (docs/adr/0045,
// section 3). A C11 root draws its own id, so a text a run saves is held to the shared one once the
// id it drew is replaced by the shared one, in the root's id and in the child's, which is the
// root's with a suffix.
//
// The Rust, Kotlin, Go and Python halves are `a_saved_child_session_is_resumed_after_a_restore.rs`,
// `ASavedChildSessionIsResumedAfterARestoreTest.kt`,
// `a_saved_child_session_is_resumed_after_a_restore_test.go` and
// `test_a_saved_child_session_is_resumed_after_a_restore.py`.

// `snprintf` and `strstr` are C99; the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include "static_invoke_sm.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_SAVED_FIXTURES_DIR
#error "SCE_SAVED_FIXTURES_DIR names sce-build/tests/fixtures/static_datamodel/saved; the build defines it"
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

enum { TEXT_CAP = 8192 };

typedef static_invoke_t machine_t;

// The text of the shared instance without its closing newline.
static void shared_text(bool format1, char *out, size_t cap) {
    char path[1024];
    size_t n;
    FILE *file;
    (void)snprintf(path, sizeof(path), "%s/%sstatic_invoke_working.json", SCE_SAVED_FIXTURES_DIR,
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

// `shared` with its first `from` made `to`; false when it holds no `from`.
static bool changed(const char *shared, const char *from, const char *to, char *out, size_t cap) {
    const char *at = strstr(shared, from);
    if (at == NULL) {
        return false;
    }
    (void)snprintf(out, cap, "%.*s%s%s", (int)(at - shared), shared, to, at + strlen(from));
    return true;
}

// An event the document names, raised on the machine's external queue and run to the end of its
// macrostep: the parent forwards it to its child.
static void send_to(machine_t *sm, const char *name) {
    static_invoke_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    CHECK(static_invoke_resolve_event_by_name(name, &meta.event), "the document names %s", name);
    static_invoke_raise_external(sm, &meta);
    static_invoke_step(sm);
}

static bool working(const machine_t *sm) {
    return static_invoke_in_state(sm, STATIC_INVOKE_STATE_WORKING);
}

static uint32_t completed(const machine_t *sm) {
    return static_invoke_get_completed(sm);
}

// The machine just started.
static void started(machine_t *sm) {
    static_invoke_init(sm);
}

static bool saved(const machine_t *sm, char *out, size_t cap) {
    size_t len = 0u;
    sce_saved_error_t err;
    if (!static_invoke_save(sm, SAVED_AT_MS, out, cap, &len, &err)) {
        CHECK(false, "the machine saves: %s", err.message);
        return false;
    }
    return true;
}

// The machine saved, once the id it drew is the shared one.
static bool saved_as_shared(const machine_t *sm, char *out, size_t cap) {
    static char text[TEXT_CAP];
    if (!saved(sm, text, sizeof(text))) {
        return false;
    }
    replace_all(text, static_invoke_session_id(sm), SHARED_ROOT, out, cap);
    return true;
}

static bool restored_from(machine_t *sm, const char *text, sce_saved_error_t *err) {
    return static_invoke_restore(sm, text, strlen(text), SAVED_AT_MS, err);
}

static void restores(machine_t *sm, const char *text) {
    sce_saved_error_t err;
    CHECK(restored_from(sm, text, &err), "restores: %s", err.message);
}

// ── What a machine is saved with ────────────────────────────────────────────

static void a_machine_is_saved_with_the_whole_state_of_the_child_it_is_running(void) {
    static machine_t sm;
    static char text[TEXT_CAP];
    static char want[TEXT_CAP];
    started(&sm);
    if (!saved_as_shared(&sm, text, sizeof(text))) {
        return;
    }
    // Its id is this session's and the id it was started under: the tree's alone.
    CHECK(strstr(text, "\"id\":\"worker\",\"start\":\"1\",\"invokeid\":\"working.1.worker\"") != NULL,
          "the child is start 1 of `worker`, started under `working.1.worker`: %s", text);
    CHECK(strstr(text, "\"sessionid\":\"" SHARED_ROOT ".working.1.worker\"") != NULL, "the child's session id: %s",
          text);
    CHECK(strstr(text, "\"current\":\"first\"") != NULL, "the child stands in `first`: %s", text);
    CHECK(strstr(text, "\"invokestarts\":{\"worker\":\"1\"}") != NULL, "the starts: %s", text);
    // The text every backend writes for the same machine, byte for byte.
    shared_text(false, want, sizeof(want));
    CHECK(strcmp(text, want) == 0, "saved as\n%s\nthe shared text is\n%s", text, want);
    static_invoke_destroy(&sm);
}

static void a_child_that_had_taken_an_event_is_resumed_with_it_taken(void) {
    static machine_t before;
    static machine_t after;
    static char text[TEXT_CAP];
    started(&before);
    send_to(&before, "a");
    if (!saved(&before, text, sizeof(text))) {
        return;
    }
    // Resumed at `second`: `b` alone ends it, and its `done.invoke` arrives once.
    restores(&after, text);
    CHECK(working(&after), "the restored machine stands in `working`");
    send_to(&after, "b");
    CHECK(completed(&after) == 1u, "the child resumed where it stood and ended on `b`: completed is %u",
          (unsigned)completed(&after));
    send_to(&after, "b");
    CHECK(completed(&after) == 1u, "it ended once: completed is %u", (unsigned)completed(&after));
    static_invoke_destroy(&before);
    static_invoke_destroy(&after);
}

static void a_restored_tree_saves_the_text_it_was_restored_from(void) {
    static machine_t sm;
    static char want[TEXT_CAP];
    static char again[TEXT_CAP];
    shared_text(false, want, sizeof(want));
    restores(&sm, want);
    if (saved(&sm, again, sizeof(again))) {
        CHECK(strcmp(again, want) == 0, "the child kept its id, its start and where it stood: saved as\n%s\nwant\n%s",
              again, want);
    }
    static_invoke_destroy(&sm);
}

static void the_count_of_starts_is_carried_through_a_restore(void) {
    // The resumed child was not started again, and when it ends the count the restore carried on
    // is still its one start.
    static machine_t before;
    static machine_t after;
    static char text[TEXT_CAP];
    started(&before);
    send_to(&before, "a");
    if (!saved(&before, text, sizeof(text))) {
        return;
    }
    restores(&after, text);
    send_to(&after, "b");
    CHECK(completed(&after) == 1u, "completed is %u", (unsigned)completed(&after));
    if (saved(&after, text, sizeof(text))) {
        CHECK(strstr(text, "\"invokestarts\":{\"worker\":\"1\"}") != NULL, "the starts: %s", text);
        CHECK(strstr(text, "\"invokes\":[]") != NULL, "the child ended, and is listed: %s", text);
    }
    static_invoke_destroy(&before);
    static_invoke_destroy(&after);
}

static void a_child_that_has_ended_is_not_resumed_or_started_again(void) {
    // `done.invoke.worker` leaves the parent in `working`, so a machine saved after its child ended
    // stands in the state that invokes it with nothing running. The invocation is complete: it is
    // not listed, and a restore does not run it a second time.
    static machine_t before;
    static machine_t after;
    static char text[TEXT_CAP];
    started(&before);
    send_to(&before, "a");
    send_to(&before, "b");
    CHECK(completed(&before) == 1u && working(&before), "completed %u, working %d", (unsigned)completed(&before),
          working(&before) ? 1 : 0);
    if (!saved(&before, text, sizeof(text))) {
        return;
    }
    CHECK(strstr(text, "\"invokes\":[]") != NULL, "the child ended, and is listed: %s", text);
    restores(&after, text);
    CHECK(completed(&after) == 1u, "the run is kept: completed is %u", (unsigned)completed(&after));
    send_to(&after, "a");
    send_to(&after, "b");
    CHECK(completed(&after) == 1u, "a child started again would have ended again: completed is %u",
          (unsigned)completed(&after));
    static_invoke_destroy(&before);
    static_invoke_destroy(&after);
}

static void a_machine_that_left_the_invoking_state_saves_no_invocation(void) {
    static machine_t sm;
    static machine_t again;
    static char text[TEXT_CAP];
    started(&sm);
    send_to(&sm, "abort");
    CHECK(!working(&sm), "`abort` leaves `working`");
    if (!saved(&sm, text, sizeof(text))) {
        return;
    }
    CHECK(strstr(text, "\"invokes\":[]") != NULL, "cancelled with its state: %s", text);
    // And nothing is started when it comes back: no child is there to take `a` and `b`, and `idle`
    // invokes nothing.
    restores(&again, text);
    send_to(&again, "a");
    send_to(&again, "b");
    CHECK(!static_invoke_is_in_final_state(&again), "the restored machine is in a final state");
    static_invoke_destroy(&sm);
    static_invoke_destroy(&again);
}

static void an_event_saved_in_the_childs_queue_is_taken_by_the_resumed_child(void) {
    // `a` reached the child's own queue and the child had not taken it when the tree was saved: it
    // is in the child's `external`, and the resumed child takes it, then ends on `b`.
    static machine_t sm;
    static char shared[TEXT_CAP];
    static char text[TEXT_CAP];
    shared_text(false, shared, sizeof(shared));
    CHECK(changed(shared,
                  "\"history\":{},\"pending\":[],\"invokes\":[],\"invokestarts\":{},\"hostinvokes\":[],"
                  "\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":[]",
                  "\"history\":{},\"pending\":[],\"invokes\":[],\"invokestarts\":{},\"hostinvokes\":[],"
                  "\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":[{\"name\":\"a\",\"data\":\"\","
                  "\"type\":\"external\",\"sendid\":\"\",\"origin\":\"\",\"origintype\":\"\",\"invokeid\":\"\","
                  "\"hostinvoketoken\":null}]",
                  text, sizeof(text)),
          "the child's queue was changed");
    restores(&sm, text);
    static_invoke_step(&sm);
    send_to(&sm, "b");
    CHECK(completed(&sm) == 1u, "the queued `a` reached the resumed child: completed is %u", (unsigned)completed(&sm));
    static_invoke_destroy(&sm);
}

// ── A tree that is not the document's ───────────────────────────────────────

typedef struct {
    const char *what;
    const char *from;
    const char *to;
    const char *expected;
} refusal_t;

// Each change to `shared` is refused, in the words every backend uses, and leaves the storage as it was.
static void each_change_is_refused(const char *shared, const refusal_t *cases, size_t count) {
    static machine_t sm;
    static machine_t before;
    static char text[TEXT_CAP];
    for (size_t i = 0u; i < count; ++i) {
        sce_saved_error_t err;
        memset(&sm, 0xAB, sizeof(sm));
        memcpy(&before, &sm, sizeof(sm));
        if (!changed(shared, cases[i].from, cases[i].to, text, sizeof(text))) {
            CHECK(false, "%s: the change did not apply", cases[i].what);
            continue;
        }
        if (restored_from(&sm, text, &err)) {
            CHECK(false, "%s: restored", cases[i].what);
            continue;
        }
        CHECK(strstr(err.message, cases[i].expected) != NULL, "%s: refused as \"%s\", want it to contain \"%s\"",
              cases[i].what, err.message, cases[i].expected);
        CHECK(memcmp(&sm, &before, sizeof(sm)) == 0, "%s: a refused restore touched the storage", cases[i].what);
    }
}

static void a_tree_whose_child_is_not_one_the_document_could_run_is_refused_whole(void) {
    static char shared[TEXT_CAP];
    static const refusal_t cases[] = {
        {"a child the document does not invoke", "\"id\":\"worker\"", "\"id\":\"stranger\"",
         "which the document does not invoke"},
        {"a start that is not the count its invoke holds", "\"start\":\"1\"", "\"start\":\"2\"",
         "the session counts 1 starts of it"},
        {"a child of a document its invoke does not start", "\"document\":\"static_invoke__sce_synth_invoke__worker\"",
         "\"document\":\"static_hybrid_holder\"", "does not start"},
        {"a child whose id is not its parent's and the id it was started under",
         "\"sessionid\":\"fixture-root.working.1.worker\"", "\"sessionid\":\"elsewhere\"",
         "a child of 'fixture-root' started under 'working.1.worker'"},
        {"a child of another shape", "688ad00d954ad9f35bdaa5eae06768a788173e0f8a8236e670efa68038781a6d",
         "0000000000000000000000000000000000000000000000000000000000000000",
         "the session at invokes[0].session: the saved state is of a document of shape"},
        {"a child that stands in no state of its document", "\"configuration\":[\"first\"],\"current\":\"first\"",
         "\"configuration\":[\"nowhere\"],\"current\":\"nowhere\"",
         "the session at invokes[0].session: the document has no state 'nowhere'"},
    };
    shared_text(false, shared, sizeof(shared));
    each_change_is_refused(shared, cases, sizeof(cases) / sizeof(cases[0]));
}

// ── Format 1: a child named by its id is started again ──────────────────────

static void a_format_one_state_starts_its_child_again_from_the_beginning(void) {
    // The only case in which a child restarts: the text holds nothing of it.
    static machine_t sm;
    static char text[TEXT_CAP];
    shared_text(true, text, sizeof(text));
    restores(&sm, text);
    CHECK(working(&sm), "the restored machine stands in `working`");
    // The child is at `first`: `b` is not what it waits for.
    send_to(&sm, "b");
    CHECK(working(&sm) && completed(&sm) == 0u,
          "`b` alone does not end a child that starts over: working %d, completed %u", working(&sm) ? 1 : 0,
          (unsigned)completed(&sm));
    send_to(&sm, "a");
    send_to(&sm, "b");
    CHECK(completed(&sm) == 1u, "`a` then `b` ends it, and `done.invoke.worker` arrived once: completed is %u",
          (unsigned)completed(&sm));
    static_invoke_destroy(&sm);
}

static void a_format_one_state_naming_an_invocation_the_document_cannot_have_run_is_refused(void) {
    static char shared[TEXT_CAP];
    static const refusal_t cases[] = {
        {"an id the document does not invoke", "\"invokes\":[\"worker\"]", "\"invokes\":[\"stranger\"]",
         "which the document does not invoke"},
        {"an id named twice", "\"invokes\":[\"worker\"]", "\"invokes\":[\"worker\",\"worker\"]",
         "which an earlier entry already names"},
        {"an invocation whose state the configuration is not in",
         "\"configuration\":[\"working\"],\"current\":\"working\"", "\"configuration\":[\"idle\"],\"current\":\"idle\"",
         "whose state the saved configuration does not stand in"},
    };
    shared_text(true, shared, sizeof(shared));
    each_change_is_refused(shared, cases, sizeof(cases) / sizeof(cases[0]));
}

int main(void) {
    a_machine_is_saved_with_the_whole_state_of_the_child_it_is_running();
    a_child_that_had_taken_an_event_is_resumed_with_it_taken();
    a_restored_tree_saves_the_text_it_was_restored_from();
    the_count_of_starts_is_carried_through_a_restore();
    a_child_that_has_ended_is_not_resumed_or_started_again();
    a_machine_that_left_the_invoking_state_saves_no_invocation();
    an_event_saved_in_the_childs_queue_is_taken_by_the_resumed_child();
    a_tree_whose_child_is_not_one_the_document_could_run_is_refused_whole();
    a_format_one_state_starts_its_child_again_from_the_beginning();
    a_format_one_state_naming_an_invocation_the_document_cannot_have_run_is_refused();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("a saved child session: ok");
    return 0;
}
