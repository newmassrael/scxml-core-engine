// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0057 §6 and §10: the C11 machine generated for every build of a
// product line, compiled under one feature set, is the machine that set's
// variant generates alone (the commuting law `select(gen(D)) = gen(project(D))`).
//
// The generator proves it of the tokens, build by build, before it writes
// anything. This program is compiled under one feature set (`-D` of each
// feature's macro, as a build of the product line selects), so the compiler
// resolves the `#if` itself, and drives both machines through every run of up
// to four of the events either one names: they must agree on every observation
// a host has of a machine — where it is, what it asked its host to do, which
// events it discarded, which names it had no event for, and where it ended.
// Each saves the same text, which records this build's configuration, and
// refuses a state taken in another (docs/adr/0057, section 7).
//
// Fixture: tests/product_line/session.scxml, generated with the symbol prefix
// `lifted` once for every build and with `alone` for this feature set; see
// the block that builds this program in backends/c/tests/CMakeLists.txt.

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "alone/session_sm.h"
#include "lifted/session_sm.h"

/* Every event name either machine knows, and one neither does. */
static const char *const EVENTS[] = {"ready", "inbound.start", "connect", "done", "close", "unknown"};
#define EVENT_COUNT (sizeof EVENTS / sizeof EVENTS[0])
#define LONGEST_RUN 4

/* What a host observes of a machine after a run of events. */
struct observation {
    char active[128];
    char calls[512];
    uint32_t discarded;
    /* Names the machine has no event for: neither machine can be handed one. */
    uint32_t unnamed;
    char terminal[32];
};

static void append(char *to, size_t size, const char *word) {
    size_t used = strlen(to);
    snprintf(to + used, size - used, "%s;", word);
}

static void called(void *user_data, const char *action) {
    struct observation *seen = user_data;
    append(seen->calls, sizeof seen->calls, action);
}

static void on_idle(void *user_data) {
    called(user_data, "on_idle");
}

/* The host serves the actions this build's machine declares, as any host of
   a product line does: an action of a feature it leaves out is no member of
   the machine's table. */
#if defined(SCE_FEATURE_SESSION_UNICAST)
static void announce_unicast(void *user_data) {
    called(user_data, "announce_unicast");
}

#define SERVE_UNICAST(host) (host).announce_unicast = announce_unicast;
#else
#define SERVE_UNICAST(host)
#endif
#if defined(SCE_FEATURE_SESSION_UNICAST_ACCEPT)
static void begin_accept(void *user_data) {
    called(user_data, "begin_accept");
}

static void end_accept(void *user_data) {
    called(user_data, "end_accept");
}

#define SERVE_ACCEPT(host)                                                                                             \
    (host).begin_accept = begin_accept;                                                                                \
    (host).end_accept = end_accept;
#else
#define SERVE_ACCEPT(host)
#endif

/* The observation of the machine of prefix P after `run`. */
#define OBSERVER(P)                                                                                                    \
    static void P##observe(const char *const *run, size_t length, struct observation *seen) {                          \
        memset(seen, 0, sizeof *seen);                                                                                 \
        P##session_actions_t host;                                                                                     \
        memset(&host, 0, sizeof host);                                                                                 \
        host.on_idle = on_idle;                                                                                        \
        SERVE_UNICAST(host)                                                                                            \
        SERVE_ACCEPT(host)                                                                                             \
        host.user_data = seen;                                                                                         \
        P##session_t sm;                                                                                               \
        if (!P##session_init_with_actions(&sm, &host)) {                                                               \
            fprintf(stderr, "feature_conformance: FAIL - `" #P "session` refused a host "                              \
                            "that serves every action of this build\n");                                               \
            exit(1);                                                                                                   \
        }                                                                                                              \
        for (size_t i = 0; i < length; ++i) {                                                                          \
            P##session_event_with_meta_t carrier;                                                                      \
            memset(&carrier, 0, sizeof carrier);                                                                       \
            if (!P##session_resolve_event_by_name(run[i], &carrier.event)) {                                           \
                seen->unnamed++;                                                                                       \
                continue;                                                                                              \
            }                                                                                                          \
            P##session_raise_external(&sm, &carrier);                                                                  \
            P##session_step(&sm);                                                                                      \
        }                                                                                                              \
        uint32_t active = P##session_active_states(&sm);                                                               \
        for (uint32_t state = 0; state < 32u; ++state) {                                                               \
            if (active & (UINT32_C(1) << state)) {                                                                     \
                append(seen->active, sizeof seen->active, P##session_state_name((P##session_state_t)state));           \
            }                                                                                                          \
        }                                                                                                              \
        seen->discarded = P##session_discarded_external_events(&sm);                                                   \
        P##session_state_t end;                                                                                        \
        if (P##session_terminal_state(&sm, &end)) {                                                                    \
            snprintf(seen->terminal, sizeof seen->terminal, "%s", P##session_state_name(end));                         \
        }                                                                                                              \
        P##session_destroy(&sm);                                                                                       \
    }

OBSERVER(lifted_)
OBSERVER(alone_)

/* The wall-clock moment the states below are saved at, and the room a saved state has. */
#define SAVED_AT_MS INT64_C(1700000000000)
#define SAVED_CAP 4096u

/* The features this build turns on, sorted, as a saved state writes them and as a refusal names
   them (docs/adr/0057, section 7), and a configuration that is not this build's. */
#if defined(SCE_FEATURE_SESSION_UNICAST) && defined(SCE_FEATURE_SESSION_UNICAST_ACCEPT)
#define THIS_BUILD "[\"session-unicast\",\"session-unicast-accept\"]"
#define THIS_BUILD_NAMED "[session-unicast, session-unicast-accept]"
#elif defined(SCE_FEATURE_SESSION_UNICAST)
#define THIS_BUILD "[\"session-unicast\"]"
#define THIS_BUILD_NAMED "[session-unicast]"
#else
#define THIS_BUILD "[]"
#define THIS_BUILD_NAMED "[]"
#endif
#if defined(SCE_FEATURE_SESSION_UNICAST)
#define OTHER_BUILD "[]"
#define OTHER_BUILD_NAMED "[]"
#else
#define OTHER_BUILD "[\"session-unicast\"]"
#define OTHER_BUILD_NAMED "[session-unicast]"
#endif

/* The state of the machine of prefix P as its start leaves it, saved into `out`, under the session
   id `fixture-root`, which its host gives it (docs/adr/0045, "Questions the owner answered" 2), so
   the two machines save one text. */
#define SAVER(P)                                                                                                       \
    static void P##saved(char *out, size_t cap) {                                                                      \
        struct observation seen;                                                                                       \
        memset(&seen, 0, sizeof seen);                                                                                 \
        P##session_actions_t host;                                                                                     \
        memset(&host, 0, sizeof host);                                                                                 \
        host.on_idle = on_idle;                                                                                        \
        SERVE_UNICAST(host)                                                                                            \
        SERVE_ACCEPT(host)                                                                                             \
        host.user_data = &seen;                                                                                        \
        P##session_t sm;                                                                                               \
        const sce_start_refusal_t refused = P##session_init_with_session_id(&sm, "fixture-root", &host);               \
        if (refused != SCE_START_REFUSAL_NONE) {                                                                       \
            fprintf(stderr, "feature_conformance: FAIL - `" #P "session` refused to start: %s\n",                      \
                    sce_start_refusal_text(refused));                                                                  \
            exit(1);                                                                                                   \
        }                                                                                                              \
        size_t len = 0;                                                                                                \
        sce_saved_error_t err;                                                                                         \
        if (!P##session_save(&sm, SAVED_AT_MS, out, cap, &len, &err)) {                                                \
            fprintf(stderr, "feature_conformance: FAIL - `" #P "session` does not save: %s\n", err.message);           \
            exit(1);                                                                                                   \
        }                                                                                                              \
        P##session_destroy(&sm);                                                                                       \
    }

SAVER(lifted_)
SAVER(alone_)

/* A saved state records the configuration it was taken in, and the machine for every build saves
   exactly what this build's own machine saves: the same text, its features this build's, last. A
   state of another configuration is refused by the features that differ, before the shape is
   judged (docs/adr/0057, section 7). */
static int a_saved_state_records_this_builds_configuration(void) {
    static const char last[] = ",\"features\":" THIS_BUILD "}";
    static const char refusal[] = "the saved state was taken with the features " OTHER_BUILD_NAMED
                                  " on, and this machine is built with " THIS_BUILD_NAMED;
    char lifted_state[SAVED_CAP];
    char alone_state[SAVED_CAP];
    char elsewhere[SAVED_CAP];
    sce_saved_error_t err;
    lifted_saved(lifted_state, sizeof lifted_state);
    alone_saved(alone_state, sizeof alone_state);
    if (strcmp(lifted_state, alone_state) != 0) {
        fprintf(stderr,
                "feature_conformance: FAIL - the machine for every build saves\n  %s\nwhere this build's own "
                "saves\n  %s\n",
                lifted_state, alone_state);
        return 1;
    }
    const size_t len = strlen(lifted_state);
    const size_t tail = strlen(last);
    if (len < tail || strcmp(lifted_state + len - tail, last) != 0) {
        fprintf(stderr, "feature_conformance: FAIL - the saved state does not end with `%s`: %s\n", last, lifted_state);
        return 1;
    }
    if (!lifted_session_judge(alone_state, strlen(alone_state), &err) ||
        !alone_session_judge(lifted_state, len, &err)) {
        fprintf(stderr, "feature_conformance: FAIL - a state of this build is refused: %s\n", err.message);
        return 1;
    }
    snprintf(elsewhere, sizeof elsewhere, "%.*s,\"features\":" OTHER_BUILD "}", (int)(len - tail), lifted_state);
    if (lifted_session_judge(elsewhere, strlen(elsewhere), &err)) {
        fprintf(stderr, "feature_conformance: FAIL - a state taken with the features " OTHER_BUILD_NAMED
                        " is restored by the build of " THIS_BUILD_NAMED "\n");
        return 1;
    }
    if (strncmp(err.message, refusal, strlen(refusal)) != 0) {
        fprintf(stderr, "feature_conformance: FAIL - a state of another configuration is refused with \"%s\"\n",
                err.message);
        return 1;
    }
    return 0;
}

static void print(const char *who, const struct observation *seen) {
    fprintf(stderr, "  %s: active=%s calls=%s discarded=%u unnamed=%u terminal=%s\n", who, seen->active, seen->calls,
            (unsigned)seen->discarded, (unsigned)seen->unnamed, seen->terminal);
}

static int alike(const struct observation *a, const struct observation *b) {
    return strcmp(a->active, b->active) == 0 && strcmp(a->calls, b->calls) == 0 && a->discarded == b->discarded &&
           a->unnamed == b->unnamed && strcmp(a->terminal, b->terminal) == 0;
}

int main(void) {
    const char *run[LONGEST_RUN];
    size_t index[LONGEST_RUN];
    unsigned long runs = 0;
    int ended = 0;
    int called_host = 0;
    for (size_t length = 0; length <= LONGEST_RUN; ++length) {
        memset(index, 0, sizeof index);
        for (;;) {
            for (size_t i = 0; i < length; ++i) {
                run[i] = EVENTS[index[i]];
            }
            struct observation lifted;
            struct observation alone;
            lifted_observe(run, length, &lifted);
            alone_observe(run, length, &alone);
            ++runs;
            if (!alike(&lifted, &alone)) {
                fprintf(stderr, "feature_conformance: FAIL - the machine for every build and "
                                "this build's own part on the run [");
                for (size_t i = 0; i < length; ++i) {
                    fprintf(stderr, "%s%s", i ? ", " : "", run[i]);
                }
                fprintf(stderr, "]\n");
                print("for every build", &lifted);
                print("alone", &alone);
                return 1;
            }
            ended |= alone.terminal[0] != '\0';
            called_host |= alone.calls[0] != '\0';
            /* The next run of this length, the last event turning fastest. */
            size_t digit = length;
            while (digit > 0 && ++index[digit - 1] == EVENT_COUNT) {
                index[--digit] = 0;
            }
            if (digit == 0) {
                break;
            }
        }
    }

    /* Held to something, not only to itself: a run that ended the machine and
       one that called the host were compared. */
    if (!ended || !called_host) {
        fprintf(stderr,
                "feature_conformance: FAIL - no run %s, so the comparison held "
                "the machines to less than it says\n",
                ended ? "called the host" : "ended the machine");
        return 1;
    }

    /* The default state is a value the document computes, and differs between
       builds: `probe` is the first child exactly where `session-unicast` is on. */
    struct observation start;
    lifted_observe(run, 0, &start);
#if defined(SCE_FEATURE_SESSION_UNICAST)
    const char *expected = "probe;";
#else
    const char *expected = "idle;";
#endif
    if (strcmp(start.active, expected) != 0) {
        fprintf(stderr,
                "feature_conformance: FAIL - the machine for every build starts in %s "
                "where this build's default state is %s\n",
                start.active, expected);
        return 1;
    }

    if (a_saved_state_records_this_builds_configuration() != 0) {
        return 1;
    }

    printf("feature_conformance: PASS - %lu runs observed alike\n", runs);
    return 0;
}
