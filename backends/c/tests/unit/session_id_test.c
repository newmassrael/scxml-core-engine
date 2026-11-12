// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The id a host may give the root session of a machine it starts, on the C11
// backend (`sce/session_id.h`, docs/adr/0045, "Questions the owner answered" 2):
//
//   * an Nmtoken that fits is taken, a tree path and a UUID among them;
//   * no id, the empty text and a text with a character that is not a name
//     character are refused, a malformed byte among them;
//   * an id of `SCE_MAX_SESSION_ID_LEN` bytes or more is refused as too long,
//     whatever it holds, and one a byte shorter is taken;
//   * an id that fits but whose location, percent-encoded, does not fit
//     `SCE_MAX_URI_LEN` is refused;
//   * every refusal has its own sentence.
//
// Header-only code; nothing here allocates.

#include <sce/session_id.h>
#include <sce/types.h>

#include <stdio.h>
#include <string.h>

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

// `count` bytes of `c`, terminated, in `out`, which holds `count + 1`.
static const char *repeated(char *out, char c, size_t count) {
    memset(out, c, count);
    out[count] = '\0';
    return out;
}

static void an_nmtoken_that_fits_is_taken(void) {
    static const char *const TAKEN[] = {
        "mtc",
        "fixture-root",
        "fixture-root.working.1.worker",
        "9f1c7a52-3e0b-4c51-9a2f-6b8d0e4f1a37",
        "a:b_c-d.e",
        // U+00E9, two bytes of UTF-8, is a name character.
        "caf\xC3\xA9",
    };
    for (size_t i = 0u; i < sizeof(TAKEN) / sizeof(TAKEN[0]); ++i) {
        CHECK(sce_session_id_judge(TAKEN[i]) == SCE_START_REFUSAL_NONE, "'%s' is taken", TAKEN[i]);
    }
}

static void what_is_no_nmtoken_is_refused(void) {
    uint32_t offending = 1u;
    CHECK(sce_session_id_judge(NULL) == SCE_START_REFUSAL_SESSION_ID_EMPTY, "no id is refused as empty");
    CHECK(sce_session_id_judge("") == SCE_START_REFUSAL_SESSION_ID_EMPTY, "the empty text is refused");
    CHECK(sce_session_id_judge("a b") == SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN, "a space is no name character");
    CHECK(sce_session_id_judge("a/b") == SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN, "a slash is no name character");
    CHECK(sce_session_id_judge("a\xFF") == SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN,
          "a malformed byte is no name character");
    CHECK(!sce_session_id_is_nmtoken("a b", &offending) && offending == 0x20u, "the offending character is U+%04X",
          (unsigned)offending);
    CHECK(!sce_session_id_is_nmtoken("", &offending) && offending == 0u, "the empty text names no character");
    CHECK(!sce_session_id_is_nmtoken("a\xFF", &offending) && offending == 0xFFFDu,
          "a malformed byte reads as U+FFFD, not U+%04X", (unsigned)offending);
}

static void an_id_past_its_bound_is_refused_not_cut(void) {
    static char id[SCE_MAX_SESSION_ID_LEN + 8];
    CHECK(sce_session_id_judge(repeated(id, 'a', (size_t)SCE_MAX_SESSION_ID_LEN - 1u)) == SCE_START_REFUSAL_NONE,
          "an id of %d bytes fits %d with its terminator", SCE_MAX_SESSION_ID_LEN - 1, SCE_MAX_SESSION_ID_LEN);
    CHECK(sce_session_id_judge(repeated(id, 'a', (size_t)SCE_MAX_SESSION_ID_LEN)) ==
              SCE_START_REFUSAL_SESSION_ID_TOO_LONG,
          "an id of %d bytes does not fit with its terminator", SCE_MAX_SESSION_ID_LEN);
    CHECK(sce_session_id_judge(repeated(id, ' ', (size_t)SCE_MAX_SESSION_ID_LEN + 4u)) ==
              SCE_START_REFUSAL_SESSION_ID_TOO_LONG,
          "the length is judged before the characters");
}

static void an_id_published_past_a_uri_is_refused(void) {
    // ':' is a name character the location percent-encodes, three bytes each, so
    // an id of them fits its own bound long before its location fits a URI's.
    static char id[SCE_MAX_SESSION_ID_LEN];
    const size_t colons = (size_t)SCE_MAX_SESSION_ID_LEN - 1u;
    repeated(id, ':', colons);
    if (sce_scxml_location_len(id) >= (size_t)SCE_MAX_URI_LEN) {
        CHECK(sce_session_id_judge(id) == SCE_START_REFUSAL_SESSION_LOCATION_TOO_LONG,
              "an id published at %lu bytes is refused", (unsigned long)sce_scxml_location_len(id));
    } else {
        CHECK(sce_session_id_judge(id) == SCE_START_REFUSAL_NONE, "an id published at %lu bytes is taken",
              (unsigned long)sce_scxml_location_len(id));
    }
#if SCE_MAX_URI_LEN < 3 * (SCE_MAX_SESSION_ID_LEN - 1) + 12
    CHECK(sce_scxml_location_len(id) >= (size_t)SCE_MAX_URI_LEN,
          "this build's bounds leave room for the case: %lu bytes against %d",
          (unsigned long)sce_scxml_location_len(id), SCE_MAX_URI_LEN);
#endif
}

static void every_refusal_has_its_own_sentence(void) {
    static const sce_start_refusal_t ALL[] = {
        SCE_START_REFUSAL_NONE,
        SCE_START_REFUSAL_SESSION_ID_EMPTY,
        SCE_START_REFUSAL_SESSION_ID_TOO_LONG,
        SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN,
        SCE_START_REFUSAL_SESSION_LOCATION_TOO_LONG,
        SCE_START_REFUSAL_HOST_ACTIONS,
        SCE_START_REFUSAL_SCRIPT_SESSION_UNAVAILABLE,
    };
    const size_t count = sizeof(ALL) / sizeof(ALL[0]);
    for (size_t i = 0u; i < count; ++i) {
        const char *text = sce_start_refusal_text(ALL[i]);
        CHECK(text != NULL && text[0] != '\0', "refusal %d has a sentence", (int)ALL[i]);
        for (size_t k = 0u; k < i; ++k) {
            CHECK(strcmp(text, sce_start_refusal_text(ALL[k])) != 0, "refusals %d and %d say the same", (int)ALL[k],
                  (int)ALL[i]);
        }
    }
    CHECK(strstr(sce_start_refusal_text(SCE_START_REFUSAL_SESSION_ID_TOO_LONG), "SCE_MAX_SESSION_ID_LEN") != NULL,
          "the sentence names the bound");
    CHECK(strstr(sce_start_refusal_text(SCE_START_REFUSAL_SESSION_LOCATION_TOO_LONG), "SCE_MAX_URI_LEN") != NULL,
          "the sentence names the bound");
}

int main(void) {
    an_nmtoken_that_fits_is_taken();
    what_is_no_nmtoken_is_refused();
    an_id_past_its_bound_is_refused_not_cut();
    an_id_published_past_a_uri_is_refused();
    every_refusal_has_its_own_sentence();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("session id: ok");
    return 0;
}
