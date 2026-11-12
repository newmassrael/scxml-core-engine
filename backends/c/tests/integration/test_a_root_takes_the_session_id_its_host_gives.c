// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A root takes its session id from its host, on the C11 backend as on every
// other (docs/adr/0045, "Questions the owner answered" 2), over the generated
// machine of `static_enum`, a document a saved state holds:
//
//   * an id the host gives is the session's id, the location it publishes spells
//     it, and a saved state names the session by it;
//   * an id the machine cannot hold whole — too long, published past a URI's
//     bytes, no Nmtoken, empty or none — is refused with its reason, and the
//     caller's storage is left as it was: no id is cut, and none is drawn in its
//     place;
//   * a restore keeps the id the state was saved with, whatever the storage was
//     started under before;
//   * a machine its host gives no id draws its own, `static_enum_session_<n>`.
//
// The Rust half that compiles the entry and drives it is
// sce-build/tests/c11_session_identity.rs; a test component started under its
// name is held by test_a_test_suite_runs_through_its_runner.c.

#include "../require_started.h"
#include "static_enum_sm.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

// 2023-11-14T22:13:20Z, a wall-clock moment for the save and the restore.
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

static const char GIVEN[] = "fixture-root.given";

// A given id is the session's, its location spells it, and the machine runs on
// under it.
static void a_given_id_is_the_session_id(void) {
    static static_enum_t sm;
    const sce_start_refusal_t refused = static_enum_init_with_session_id(&sm, GIVEN);
    CHECK(refused == SCE_START_REFUSAL_NONE, "the id is taken: %s", sce_start_refusal_text(refused));
    CHECK(strcmp(static_enum_session_id(&sm), GIVEN) == 0, "the session id is `%s`", static_enum_session_id(&sm));
    CHECK(strcmp(static_enum_scxml_location(&sm), "sce://scxml/fixture-root.given") == 0,
          "the published location is `%s`", static_enum_scxml_location(&sm));
    static_enum_step(&sm);
    CHECK(strcmp(static_enum_session_id(&sm), GIVEN) == 0, "the id is kept through the session, not `%s`",
          static_enum_session_id(&sm));
    static_enum_destroy(&sm);
}

// A refused start answers `expected` and leaves every byte of the storage as it
// was.
static void refused_untouched(const char *what, const char *id, sce_start_refusal_t expected) {
    static static_enum_t sm;
    static static_enum_t before;
    memset(&sm, 0xAB, sizeof(sm));
    memcpy(&before, &sm, sizeof(sm));
    const sce_start_refusal_t refused = static_enum_init_with_session_id(&sm, id);
    CHECK(refused == expected, "%s: refused as %d (%s), expected %d (%s)", what, (int)refused,
          sce_start_refusal_text(refused), (int)expected, sce_start_refusal_text(expected));
    CHECK(memcmp(&sm, &before, sizeof(sm)) == 0, "%s: the storage was touched by a refused start", what);
}

static void an_id_the_machine_cannot_hold_is_refused_and_touches_nothing(void) {
    static char too_long[SCE_MAX_SESSION_ID_LEN + 1];
    static char published_past_a_uri[SCE_MAX_SESSION_ID_LEN];
    memset(too_long, 'a', sizeof(too_long) - 1u);
    too_long[sizeof(too_long) - 1u] = '\0';
    refused_untouched("an id of SCE_MAX_SESSION_ID_LEN bytes", too_long, SCE_START_REFUSAL_SESSION_ID_TOO_LONG);
    // ':' is a name character the location percent-encodes in three bytes.
    memset(published_past_a_uri, ':', sizeof(published_past_a_uri) - 1u);
    published_past_a_uri[sizeof(published_past_a_uri) - 1u] = '\0';
    if (sce_scxml_location_len(published_past_a_uri) >= (size_t)SCE_MAX_URI_LEN) {
        refused_untouched("an id published past a URI", published_past_a_uri,
                          SCE_START_REFUSAL_SESSION_LOCATION_TOO_LONG);
    }
    refused_untouched("an id with a space", "fixture root", SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN);
    refused_untouched("the empty id", "", SCE_START_REFUSAL_SESSION_ID_EMPTY);
    refused_untouched("no id", NULL, SCE_START_REFUSAL_SESSION_ID_EMPTY);
}

// A restore binds the id the state was saved with: the storage it restores into
// was started under another id, and drew none of its own that survives.
static void a_restore_keeps_the_id_it_was_saved_with(void) {
    static static_enum_t saved;
    static static_enum_t restored;
    static char text[TEXT_CAP];
    size_t len = 0u;
    sce_saved_error_t err;
    err.message[0] = '\0';
    CHECK(static_enum_init_with_session_id(&saved, GIVEN) == SCE_START_REFUSAL_NONE, "the id is taken");
    CHECK(static_enum_save(&saved, SAVED_AT_MS, text, sizeof(text), &len, &err), "%s", err.message);
    CHECK(strstr(text, GIVEN) != NULL, "the saved state names the session by its id:\n%s", text);
    CHECK(static_enum_init_with_session_id(&restored, "someone-else") == SCE_START_REFUSAL_NONE, "the id is taken");
    CHECK(static_enum_restore(&restored, text, len, SAVED_AT_MS, &err), "%s", err.message);
    CHECK(strcmp(static_enum_session_id(&restored), GIVEN) == 0, "the restored session is `%s`, not the saved `%s`",
          static_enum_session_id(&restored), GIVEN);
    CHECK(strcmp(static_enum_scxml_location(&restored), "sce://scxml/fixture-root.given") == 0,
          "the restored session publishes `%s`", static_enum_scxml_location(&restored));
    static_enum_destroy(&saved);
    static_enum_destroy(&restored);
}

// Given no id, the machine draws its own, as it always has.
static void no_id_gives_the_drawn_default(void) {
    static static_enum_t sm;
    static const char DRAWN[] = "static_enum_session_";
    REQUIRE_STARTED(static_enum_init(&sm), STATIC_ENUM_START_OK);
    CHECK(strncmp(static_enum_session_id(&sm), DRAWN, sizeof(DRAWN) - 1u) == 0, "the drawn id is `%s`",
          static_enum_session_id(&sm));
    CHECK(strcmp(static_enum_session_id(&sm), GIVEN) != 0, "a machine given no id holds none it was not given");
    static_enum_destroy(&sm);
}

int main(void) {
    a_given_id_is_the_session_id();
    an_id_the_machine_cannot_hold_is_refused_and_touches_nothing();
    a_restore_keeps_the_id_it_was_saved_with();
    no_id_gives_the_drawn_default();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)printf("a root takes the session id its host gives: all checks passed\n");
    return 0;
}
