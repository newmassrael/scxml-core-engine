// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045, section 8): the saved
// states the shared fixtures hold are the text every backend writes and reads, so a state saved
// by one backend is one another restores. Each format-2 text is restored by the generated C11
// machine of its document and saved again, which is the same text byte for byte; each format-1
// text, which every backend still reads, is restored and saved as format 2 under the machine's own
// session id.
//
// The cases below it hold the machine's own behaviour: a waiting send survives a restore, a
// queued event is delivered after one, a state that is not the document's is refused and leaves
// the storage as it was.
//
// The Rust, Kotlin, Go, Python and C++ halves are `static_datamodel.rs`,
// `StaticDatamodelTest.kt`, `a_state_another_backend_saved_is_restored_test.go`,
// `test_a_state_another_backend_saved_is_restored.py` and
// `AStateAnotherBackendSavedIsRestoredTest.cpp`; the shared texts are
// `sce-build/tests/fixtures/static_datamodel/saved/`.

// `opendir` is POSIX, and the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include "static_enum_sm.h"
#include "static_history_sm.h"
#include "static_list_sm.h"
#include "static_optional_record_sm.h"
#include "static_optional_sm.h"
#include "static_record_sm.h"
#include "static_schema_pending_sm.h"
#include "static_schema_revision_sm.h"
#include "static_send_idlocation_sm.h"
#include "static_timers_sm.h"

#include <dirent.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_SAVED_FIXTURES_DIR
#error "SCE_SAVED_FIXTURES_DIR names sce-build/tests/fixtures/static_datamodel/saved; the build defines it"
#endif

// 2023-11-14T22:13:20Z, the wall-clock moment the shared fixtures were saved at.
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

enum { TEXT_CAP = 16384 };

// The one round trip every machine makes: the restore of a text, then the save of what it restored
// at the moment it was restored at. A machine whose delayed sends wait on a clock is handed one the
// test owns, so that the text it saves is not a reading of the wall.
typedef bool (*round_trip_fn)(const char *text, size_t len, int64_t wall, char *out, size_t cap,
                              sce_saved_error_t *err);

#define ROUND_TRIP(M)                                                                                                  \
    static bool round_trip_##M(const char *text, size_t len, int64_t wall, char *out, size_t cap,                      \
                               sce_saved_error_t *err) {                                                               \
        static M##_t sm;                                                                                               \
        size_t n = 0u;                                                                                                 \
        return M##_restore(&sm, text, len, wall, err) && M##_save(&sm, wall, out, cap, &n, err);                       \
    }

#define ROUND_TRIP_CLOCKED(M)                                                                                          \
    static bool round_trip_##M(const char *text, size_t len, int64_t wall, char *out, size_t cap,                      \
                               sce_saved_error_t *err) {                                                               \
        static M##_t sm;                                                                                               \
        M##_restore_args_t args;                                                                                       \
        size_t n = 0u;                                                                                                 \
        memset(&args, 0, sizeof(args));                                                                                \
        args.clock = sce_clock_manual(0u);                                                                             \
        return M##_restore(&sm, text, len, wall, &args, err) && M##_save(&sm, wall, out, cap, &n, err);                \
    }

ROUND_TRIP(static_enum)
ROUND_TRIP(static_history)
ROUND_TRIP(static_list)
ROUND_TRIP(static_optional)
ROUND_TRIP(static_optional_record)
ROUND_TRIP(static_record)
ROUND_TRIP(static_schema_revision)
ROUND_TRIP_CLOCKED(static_schema_pending)
ROUND_TRIP_CLOCKED(static_send_idlocation)
ROUND_TRIP_CLOCKED(static_timers)

// Each machine a shared saved state is of, by the stem of the document.
typedef struct {
    const char *stem;
    round_trip_fn round_trip;
} machine_t;

static const machine_t machines[] = {
    {"static_enum", round_trip_static_enum},
    {"static_history", round_trip_static_history},
    {"static_list", round_trip_static_list},
    {"static_optional", round_trip_static_optional},
    {"static_optional_record", round_trip_static_optional_record},
    {"static_record", round_trip_static_record},
    {"static_schema_pending", round_trip_static_schema_pending},
    {"static_schema_revision", round_trip_static_schema_revision},
    {"static_send_idlocation", round_trip_static_send_idlocation},
    {"static_timers", round_trip_static_timers},
};

// The stem of the document a shared text is of: the file's own name up to the part that says which
// moment of the run it is. `static_schema_pending_first` and `static_schema_revision_second` are
// moments of one document, and `static_timers_midway` of another. The longest stem that begins the
// name is the document's: `static_optional_record` is a document of its own, not a moment of
// `static_optional`.
static const machine_t *machine_of(const char *name) {
    const machine_t *found = NULL;
    size_t found_len = 0u;
    for (size_t i = 0u; i < sizeof(machines) / sizeof(machines[0]); ++i) {
        const size_t n = strlen(machines[i].stem);
        if (strncmp(name, machines[i].stem, n) == 0 && (name[n] == '_' || name[n] == '.') && n > found_len) {
            found = &machines[i];
            found_len = n;
        }
    }
    return found;
}

// The text of `path` without its closing newline; false when it cannot be read.
static bool read_trimmed(const char *path, char *out, size_t cap, size_t *len) {
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return false;
    }
    const size_t n = fread(out, 1u, cap - 1u, file);
    (void)fclose(file);
    size_t end = n;
    while (end > 0u && (out[end - 1u] == '\n' || out[end - 1u] == '\r')) {
        --end;
    }
    out[end] = '\0';
    *len = end;
    return true;
}

// Every text of `directory` that is of a machine here, restored and saved again; `format2` says the
// text is saved as it was. How many were held to a machine.
static int round_trips(const char *directory, bool format2) {
    DIR *dir = opendir(directory);
    int checked = 0;
    if (dir == NULL) {
        CHECK(false, "cannot open %s", directory);
        return 0;
    }
    for (struct dirent *entry = readdir(dir); entry != NULL; entry = readdir(dir)) {
        const size_t n = strlen(entry->d_name);
        if (n < 6u || strcmp(entry->d_name + n - 5u, ".json") != 0) {
            continue;
        }
        const machine_t *machine = machine_of(entry->d_name);
        if (machine == NULL) {
            // A tree's text is the restore of a parent and its children, which the machines it
            // names hold.
            continue;
        }
        char path[1024];
        static char text[TEXT_CAP];
        static char again[TEXT_CAP];
        size_t len = 0u;
        sce_saved_error_t err;
        (void)snprintf(path, sizeof(path), "%s/%s", directory, entry->d_name);
        if (!read_trimmed(path, text, sizeof(text), &len)) {
            CHECK(false, "cannot read %s", path);
            continue;
        }
        const bool ok = machine->round_trip(text, len, SAVED_AT_MS, again, sizeof(again), &err);
        CHECK(ok, "%s: restoring and saving again was refused: %s", entry->d_name, ok ? "" : err.message);
        if (ok && format2) {
            CHECK(strcmp(again, text) == 0, "%s: saved again as\n%s\nthe shared text is\n%s", entry->d_name, again,
                  text);
        } else if (ok) {
            // What a format-1 text holds is what a restore reads; saving it again writes format 2,
            // with the machine's own session id.
            CHECK(strncmp(again, "{\"format\":2,", 12u) == 0, "%s: saved again as %s", entry->d_name, again);
        }
        ++checked;
    }
    (void)closedir(dir);
    return checked;
}

static void a_format_two_state_another_backend_saved_is_restored_and_saved_as_it_was(void) {
    const int checked = round_trips(SCE_SAVED_FIXTURES_DIR, true);
    CHECK(checked >= 8, "only %d of the shared format-2 texts were held to a machine", checked);
}

// A text is held to the machine of the longest stem that begins its name: the optional record's
// state is the document `static_optional_record`'s, whose name begins with `static_optional`'s.
static void a_document_whose_name_begins_with_anothers_is_held_to_its_own_machine(void) {
    const machine_t *record = machine_of("static_optional_record.json");
    const machine_t *plain = machine_of("static_optional.json");
    CHECK(record != NULL && strcmp(record->stem, "static_optional_record") == 0,
          "static_optional_record.json is held to %s", record == NULL ? "no machine" : record->stem);
    CHECK(plain != NULL && strcmp(plain->stem, "static_optional") == 0, "static_optional.json is held to %s",
          plain == NULL ? "no machine" : plain->stem);
}

static void a_format_one_state_is_restored_and_takes_the_machines_own_session(void) {
    char directory[1024];
    (void)snprintf(directory, sizeof(directory), "%s/format1", SCE_SAVED_FIXTURES_DIR);
    const int checked = round_trips(directory, false);
    CHECK(checked >= 10, "only %d of the shared format-1 texts were held to a machine", checked);
}

// ── A waiting send survives a restore ────────────────────────────────────────

static int64_t trace_of(const static_timers_t *sm) {
    return (int64_t)static_timers_get_trace(sm);
}

// An event the document names, delivered and run to the end of its macrostep.
static void raise_timers(static_timers_t *sm, const char *name) {
    static_timers_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    CHECK(static_timers_resolve_event_by_name(name, &meta.event), "the document names %s", name);
    static_timers_raise_external(sm, &meta);
    static_timers_step(sm);
}

static void start_timers(static_timers_t *sm) {
    static_timers_init_with_clock(sm, sce_clock_manual(0u));
}

static void a_waiting_send_survives_a_restore(void) {
    static static_timers_t saved;
    static static_timers_t restored;
    static char text[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    static_timers_restore_args_t args;

    start_timers(&saved);
    static_timers_advance_time_ms(&saved, 1500u);
    static_timers_run(&saved);
    CHECK(trace_of(&saved) == 1, "`inner` was delivered at 1s");

    // The state is saved with three sends waiting: the two due at 2s and the one at 5s.
    CHECK(static_timers_save(&saved, SAVED_AT_MS, text, sizeof(text), &len, &err), "%s", err.message);
    CHECK(strstr(text, "\"due\":\"1700000000500\"") != NULL, "`beat` is due 500 ms from the moment of the save: %s",
          text);
    CHECK(strstr(text, "\"due\":\"1700000003500\"") != NULL, "`timeout` is due 3500 ms from it: %s", text);

    // A machine in another process comes up where it stood, on a clock of its own.
    memset(&args, 0, sizeof(args));
    args.clock = sce_clock_manual(0u);
    CHECK(static_timers_restore(&restored, text, len, SAVED_AT_MS, &args, &err), "%s", err.message);
    CHECK(trace_of(&restored) == 1, "the restored machine holds `trace` as it was");

    // 500 ms on, `beat` then `echo`, in the order they were sent.
    static_timers_advance_time_ms(&restored, 500u);
    static_timers_run(&restored);
    CHECK(trace_of(&restored) == 124, "the sends due together are delivered in the order they were sent: %ld",
          (long)trace_of(&restored));

    // `timeout` is due 3 s after that and takes the machine to `done`, not a moment before.
    {
        static_timers_state_t done;
        static_timers_advance_time_ms(&restored, 2900u);
        static_timers_run(&restored);
        CHECK(!static_timers_terminal_state(&restored, &done), "`timeout` is not due yet");
        static_timers_advance_time_ms(&restored, 200u);
        static_timers_run(&restored);
        CHECK(static_timers_terminal_state(&restored, &done) && done == STATIC_TIMERS_STATE_DONE,
              "the restored machine ends at `done` when `timeout` comes due");
    }
}

static void the_id_a_cancel_names_is_part_of_what_is_saved(void) {
    static static_timers_t saved;
    static static_timers_t restored;
    static char text[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    static_timers_restore_args_t args;
    static_timers_state_t done;

    start_timers(&saved);
    static_timers_advance_time_ms(&saved, 100u);
    static_timers_run(&saved);
    CHECK(static_timers_save(&saved, SAVED_AT_MS, text, sizeof(text), &len, &err), "%s", err.message);
    memset(&args, 0, sizeof(args));
    args.clock = sce_clock_manual(0u);
    CHECK(static_timers_restore(&restored, text, len, SAVED_AT_MS, &args, &err), "%s", err.message);

    // `stop` cancels the send named `timer`, which a restore has to keep possible.
    raise_timers(&restored, "stop");
    static_timers_run(&restored);
    static_timers_advance_time_ms(&restored, 10000u);
    static_timers_run(&restored);
    CHECK(!static_timers_terminal_state(&restored, &done), "the cancelled `timeout` was never delivered");
    CHECK(trace_of(&restored) == 124, "the sends that were not cancelled were delivered: %ld",
          (long)trace_of(&restored));
}

static void a_queued_event_is_delivered_after_a_restore(void) {
    static static_timers_t restored;
    static char text[TEXT_CAP];
    static char with_event[TEXT_CAP];
    char path[1024];
    size_t len = 0u;
    sce_saved_error_t err;
    static_timers_restore_args_t args;
    static_timers_state_t done;
    const char *none = "\"external\":[]";
    const char *queued = "\"external\":[{\"name\":\"stop\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\","
                         "\"origin\":\"\",\"origintype\":\"http://www.w3.org/TR/scxml/#SCXMLEventProcessor\","
                         "\"invokeid\":\"\",\"hostinvoketoken\":null}]";
    (void)snprintf(path, sizeof(path), "%s/static_timers_midway.json", SCE_SAVED_FIXTURES_DIR);
    CHECK(read_trimmed(path, text, sizeof(text), &len), "the shared text");
    const char *at = strstr(text, none);
    CHECK(at != NULL, "the shared text queues nothing");
    if (at == NULL) {
        return;
    }
    (void)snprintf(with_event, sizeof(with_event), "%.*s%s%s", (int)(at - text), text, queued, at + strlen(none));
    memset(&args, 0, sizeof(args));
    args.clock = sce_clock_manual(0u);
    CHECK(static_timers_restore(&restored, with_event, strlen(with_event), SAVED_AT_MS, &args, &err), "%s",
          err.message);
    // The event waits for the first drive, and when it is taken it cancels `timeout`.
    static_timers_run(&restored);
    static_timers_advance_time_ms(&restored, 60000u);
    static_timers_run(&restored);
    CHECK(!static_timers_terminal_state(&restored, &done), "`stop` was delivered, so `timeout` never was");
}

// ── A state that is not the document's ──────────────────────────────────────────

static void a_refused_restore_leaves_the_storage_as_it_was(void) {
    static static_enum_t sm;
    static static_enum_t before;
    static char text[TEXT_CAP];
    char path[1024];
    size_t len = 0u;
    sce_saved_error_t err;
    memset(&sm, 0xAB, sizeof(sm));
    memcpy(&before, &sm, sizeof(sm));
    // A timers text is not a state of the enum machine: its shape is another document's.
    (void)snprintf(path, sizeof(path), "%s/static_timers_midway.json", SCE_SAVED_FIXTURES_DIR);
    CHECK(read_trimmed(path, text, sizeof(text), &len), "the shared text");
    CHECK(!static_enum_restore(&sm, text, len, SAVED_AT_MS, &err), "a text of another document is refused");
    CHECK(strstr(err.message, "the saved state is of a document of shape ") == err.message, "%s", err.message);
    CHECK(memcmp(&sm, &before, sizeof(sm)) == 0, "the storage was touched by a refused restore");
    CHECK(!static_enum_judge(text, len, &err), "judging it says the same");
}

static void a_text_that_is_not_a_state_is_refused_in_the_words_every_backend_uses(void) {
    static static_timers_t sm;
    static_timers_restore_args_t args;
    sce_saved_error_t err;
    memset(&args, 0, sizeof(args));
    CHECK(!static_timers_restore(&sm, "[]", 2u, SAVED_AT_MS, &args, &err) &&
              strcmp(err.message, "the saved state is not an object") == 0,
          "%s", err.message);
    CHECK(!static_timers_restore(&sm, "{\"format\":9}", 12u, SAVED_AT_MS, &args, &err) &&
              strcmp(err.message, "the saved state is format 9, and this runtime reads formats 1 and 2") == 0,
          "%s", err.message);
}

// A shared text with `from` made `to`: the refusal a machine gives for what a document does not hold.
static bool restore_changed(const char *name, const char *from, const char *to, sce_saved_error_t *err) {
    static static_timers_t sm;
    static char text[TEXT_CAP];
    static char changed[TEXT_CAP];
    char path[1024];
    size_t len = 0u;
    static_timers_restore_args_t args;
    (void)snprintf(path, sizeof(path), "%s/%s", SCE_SAVED_FIXTURES_DIR, name);
    if (!read_trimmed(path, text, sizeof(text), &len)) {
        CHECK(false, "cannot read %s", path);
        return true;
    }
    const char *at = strstr(text, from);
    if (at == NULL) {
        CHECK(false, "%s does not hold %s", name, from);
        return true;
    }
    (void)snprintf(changed, sizeof(changed), "%.*s%s%s", (int)(at - text), text, to, at + strlen(from));
    memset(&args, 0, sizeof(args));
    args.clock = sce_clock_manual(0u);
    return static_timers_restore(&sm, changed, strlen(changed), SAVED_AT_MS, &args, err);
}

static void a_state_the_document_could_not_have_been_in_is_refused_by_name(void) {
    sce_saved_error_t err;
    CHECK(!restore_changed("static_timers_midway.json", "\"configuration\":[\"waiting\"]",
                           "\"configuration\":[\"nowhere\"]", &err) &&
              strcmp(err.message, "the document has no state 'nowhere'") == 0,
          "%s", err.message);
    CHECK(!restore_changed("static_timers_midway.json", "\"configuration\":[\"waiting\"]", "\"configuration\":[]",
                           &err) &&
              strstr(err.message, "the saved configuration is refused: ") == err.message,
          "%s", err.message);
    CHECK(!restore_changed("static_timers_midway.json", "\"event\":\"beat\"", "\"event\":\"never\"", &err) &&
              strcmp(err.message, "pending[0] is 'never', which the document does not name") == 0,
          "%s", err.message);
    CHECK(!restore_changed("static_timers_midway.json", "\"variables\":{\"trace\":1}", "\"variables\":{}", &err) &&
              strcmp(err.message, "the saved state has no variable 'trace'") == 0,
          "%s", err.message);
    CHECK(!restore_changed("static_timers_midway.json", "\"variables\":{\"trace\":1}", "\"variables\":{\"trace\":-1}",
                           &err) &&
              strstr(err.message, "'trace'") == err.message,
          "%s", err.message);
    CHECK(!restore_changed("static_timers_midway.json", "\"history\":{}", "\"history\":{\"nowhere\":[\"waiting\"]}",
                           &err) &&
              strcmp(err.message,
                     "the saved state records the history 'nowhere', which the document does not declare") == 0,
          "%s", err.message);
}

// ── Saving ──────────────────────────────────────────────────────────────────────

static void a_machine_that_is_not_running_is_not_saved(void) {
    static static_timers_t sm;
    static char text[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    start_timers(&sm);
    raise_timers(&sm, "timeout");
    static_timers_run(&sm);
    CHECK(!static_timers_save(&sm, SAVED_AT_MS, text, sizeof(text), &len, &err) &&
              strcmp(err.message, "the machine is not running: it was never started, or it has ended") == 0,
          "%s", err.message);
}

static void a_save_into_no_buffer_measures_the_text(void) {
    static static_timers_t sm;
    static char text[TEXT_CAP];
    static char small[8];
    size_t measured = 0u;
    size_t written = 0u;
    sce_saved_error_t err;
    start_timers(&sm);
    CHECK(static_timers_save(&sm, SAVED_AT_MS, NULL, 0u, &measured, &err), "%s", err.message);
    CHECK(static_timers_save(&sm, SAVED_AT_MS, text, sizeof(text), &written, &err), "%s", err.message);
    CHECK(measured == written && strlen(text) == written, "measured %lu, wrote %lu", (unsigned long)measured,
          (unsigned long)written);
    CHECK(!static_timers_save(&sm, SAVED_AT_MS, small, sizeof(small), &written, &err) && written == measured &&
              strstr(err.message, "the buffer holds 8 bytes") == err.message,
          "%s", err.message);
}

int main(void) {
    a_document_whose_name_begins_with_anothers_is_held_to_its_own_machine();
    a_format_two_state_another_backend_saved_is_restored_and_saved_as_it_was();
    a_format_one_state_is_restored_and_takes_the_machines_own_session();
    a_waiting_send_survives_a_restore();
    the_id_a_cancel_names_is_part_of_what_is_saved();
    a_queued_event_is_delivered_after_a_restore();
    a_refused_restore_leaves_the_storage_as_it_was();
    a_text_that_is_not_a_state_is_refused_in_the_words_every_backend_uses();
    a_state_the_document_could_not_have_been_in_is_refused_by_name();
    a_machine_that_is_not_running_is_not_saved();
    a_save_into_no_buffer_measures_the_text();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("a state another backend saved: ok");
    return 0;
}
