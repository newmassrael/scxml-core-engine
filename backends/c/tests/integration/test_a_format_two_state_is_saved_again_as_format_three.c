// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0090, section 7): a state saved in
// format 2, before an event's origin was held as one pair, is read by the table of that section and
// written back as format 3. Each reading fixture of `saved/format2/` is restored by the generated
// C11 machine of its document, at the wall-clock moment the shared fixtures were saved at, and
// saved at once; the text it saves is, byte for byte, the one `saved/format2/read_as/` holds under
// the same name.
//
// The fixtures are the shared tree as the other backends wrote it, the same tree with the queued
// event's origin as this backend stored it (its sender's location, unencoded), the tree with one
// queued event for each row of the table, and the shared timers, whose `internal` act stored the
// sender. `saved/format2/README.md` lists what each record is read as.
//
// Fixtures: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml and
// static_timers.scxml; the texts are sce-build/tests/fixtures/static_datamodel/saved/format2/.

// `opendir` is POSIX, and the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include "static_invoke_tree_sm.h"
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

enum { TEXT_CAP = 32768 };

// The restore of a text and the save of what it restored, at once, at the moment it was restored
// at. Each machine is handed a clock the test owns, so that the text it saves is not a reading of
// the wall.
typedef bool (*round_trip_fn)(const char *text, size_t len, int64_t wall, char *out, size_t cap,
                              sce_saved_error_t *err);

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

ROUND_TRIP_CLOCKED(static_invoke_tree)
ROUND_TRIP_CLOCKED(static_timers)

// Each machine a reading fixture is of, by the stem of the document.
typedef struct {
    const char *stem;
    round_trip_fn round_trip;
} machine_t;

static const machine_t machines[] = {
    {"static_invoke_tree", round_trip_static_invoke_tree},
    {"static_timers", round_trip_static_timers},
};

// The machine of the longest stem that begins the file's name, which goes on with `_` or `.`:
// `static_invoke_tree_working_c11.json` is a moment of `static_invoke_tree`.
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

// Whether a path `snprintf` wrote in `n` characters fits a buffer of `cap`. A path that does not
// fit fails the test: reading the truncated name would read some other file, or none.
static bool path_fits(int n, size_t cap) {
    return n >= 0 && (size_t)n < cap;
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

static void every_format_two_text_is_saved_again_as_the_format_three_text_it_is_read_as(void) {
    char directory[1024];
    DIR *dir;
    int checked = 0;
    if (!path_fits(snprintf(directory, sizeof(directory), "%s/format2", SCE_SAVED_FIXTURES_DIR), sizeof(directory))) {
        CHECK(false, "the path of %s/format2 does not fit", SCE_SAVED_FIXTURES_DIR);
        return;
    }
    dir = opendir(directory);
    if (dir == NULL) {
        CHECK(false, "cannot open %s", directory);
        return;
    }
    for (struct dirent *entry = readdir(dir); entry != NULL; entry = readdir(dir)) {
        const size_t n = strlen(entry->d_name);
        if (n < 6u || strcmp(entry->d_name + n - 5u, ".json") != 0) {
            continue;
        }
        const machine_t *machine = machine_of(entry->d_name);
        char path[1024];
        static char text[TEXT_CAP];
        static char want[TEXT_CAP];
        static char again[TEXT_CAP];
        size_t len = 0u;
        size_t want_len = 0u;
        sce_saved_error_t err;
        // Every reading fixture is held to a machine: one this test cannot restore is a gap, not a skip.
        CHECK(machine != NULL, "%s is of no machine this test builds", entry->d_name);
        if (machine == NULL) {
            continue;
        }
        if (!path_fits(snprintf(path, sizeof(path), "%s/%s", directory, entry->d_name), sizeof(path))) {
            CHECK(false, "the path of %s does not fit", entry->d_name);
            continue;
        }
        if (!read_trimmed(path, text, sizeof(text), &len)) {
            CHECK(false, "cannot read %s", path);
            continue;
        }
        if (!path_fits(snprintf(path, sizeof(path), "%s/read_as/%s", directory, entry->d_name), sizeof(path))) {
            CHECK(false, "the path of read_as/%s does not fit", entry->d_name);
            continue;
        }
        if (!read_trimmed(path, want, sizeof(want), &want_len)) {
            CHECK(false, "cannot read %s", path);
            continue;
        }
        CHECK(strncmp(text, "{\"format\":2,", 12u) == 0, "%s is a format-2 text", entry->d_name);
        const bool ok = machine->round_trip(text, len, SAVED_AT_MS, again, sizeof(again), &err);
        CHECK(ok, "%s: restoring and saving again was refused: %s", entry->d_name, ok ? "" : err.message);
        if (ok) {
            CHECK(strcmp(again, want) == 0, "%s: saved again as\n%s\nit is read as\n%s", entry->d_name, again, want);
        }
        ++checked;
    }
    (void)closedir(dir);
    CHECK(checked >= 4, "only %d of the format-2 reading texts were held to a machine", checked);
}

// `text` with every `from` made `to`, into `out` of `cap` bytes.
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

// The reading fixture `name` with each `from` made `to`, restored by `machine` and saved at once:
// the text `read_as/<name>` holds.
static void a_changed_text_is_saved_as(const machine_t *machine, const char *name, const char *const *from,
                                       const char *const *to, size_t count) {
    char path[1024];
    static char text[TEXT_CAP];
    static char changed[TEXT_CAP];
    static char want[TEXT_CAP];
    static char again[TEXT_CAP];
    size_t len = 0u;
    size_t want_len = 0u;
    sce_saved_error_t err;
    if (machine == NULL) {
        CHECK(false, "%s is of no machine this test builds", name);
        return;
    }
    if (!path_fits(snprintf(path, sizeof(path), "%s/format2/%s", SCE_SAVED_FIXTURES_DIR, name), sizeof(path))) {
        CHECK(false, "the path of %s does not fit", name);
        return;
    }
    if (!read_trimmed(path, text, sizeof(text), &len)) {
        CHECK(false, "cannot read %s", path);
        return;
    }
    if (!path_fits(snprintf(path, sizeof(path), "%s/format2/read_as/%s", SCE_SAVED_FIXTURES_DIR, name), sizeof(path))) {
        CHECK(false, "the path of read_as/%s does not fit", name);
        return;
    }
    if (!read_trimmed(path, want, sizeof(want), &want_len)) {
        CHECK(false, "cannot read %s", path);
        return;
    }
    for (size_t i = 0u; i < count; ++i) {
        CHECK(strstr(text, from[i]) != NULL, "%s holds %s", name, from[i]);
        replace_all(text, from[i], to[i], changed, sizeof(changed));
        memcpy(text, changed, sizeof(text));
    }
    const bool ok = machine->round_trip(text, strlen(text), SAVED_AT_MS, again, sizeof(again), &err);
    CHECK(ok, "%s, changed: restoring and saving again was refused: %s", name, ok ? "" : err.message);
    if (ok) {
        CHECK(strcmp(again, want) == 0, "%s, changed: saved again as\n%s\nit is read as\n%s", name, again, want);
    }
}

// docs/adr/0090, section 7: a `raise`, `parent` or `child` act of a format-2 text whose stored
// origin is empty is sent by the session that holds it, so it is saved again with that session's
// pair and id, as the act that stored the id is.
static void a_format_two_act_stored_with_no_origin_is_the_sessions_own_send(void) {
    static const char *const timers_from[] = {"\"origin\":\"fixture-root\""};
    static const char *const timers_to[] = {"\"origin\":\"\""};
    static const char *const tree_from[] = {"\"sendid\":\"poke\",\"origin\":\"fixture-root\"",
                                            "\"sendid\":\"report\",\"origin\":\"fixture-root.working.2.worker\""};
    static const char *const tree_to[] = {"\"sendid\":\"poke\",\"origin\":\"\"",
                                          "\"sendid\":\"report\",\"origin\":\"\""};
    a_changed_text_is_saved_as(machine_of("static_timers.json"), "static_timers.json", timers_from, timers_to, 1u);
    a_changed_text_is_saved_as(machine_of("static_invoke_tree_working.json"), "static_invoke_tree_working.json",
                               tree_from, tree_to, 2u);
}

int main(void) {
    every_format_two_text_is_saved_again_as_the_format_three_text_it_is_read_as();
    a_format_two_act_stored_with_no_origin_is_the_sessions_own_send();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("a format-2 state saved again: ok");
    return 0;
}
