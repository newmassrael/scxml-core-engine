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

    printf("feature_conformance: PASS - %lu runs observed alike\n", runs);
    return 0;
}
