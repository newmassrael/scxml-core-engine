/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 3 */
/* REQUIRED: cas rmw all */

/*
 * What this harness is expected to catch, and what has not been measured: the
 * same three mutants as inbox_two_posters.c. The slot's inbox holds one input, so
 * the first poster's second ending is written into the slot the owner read its
 * first out of: scq64_slot_reuse.c's scenario. Chosen by reading and NOT run on
 * GenMC when this was written; the gate prints each mutant that survives.
 */

/*
 * The GenMC layer of the queue kind's verification (the Protocol-Synthesis RFC's
 * queue section), for the directory and the inbox of a generated C11 tree
 * (sce/inbox.h; docs/adr/0084, D2 as amended at step 10; docs/adr/0147, D5): an
 * ending that lands in a slot restarted since it was posted.
 *
 * A child session runs in the root's one slot when the threads start, with one
 * host-run invocation started. The owner applies the slot's inbox once, as at a
 * macrostep boundary of that child; ends the child (its invocation is cancelled,
 * it leaves the directory and what its inbox holds is applied); and starts it
 * again in the same slot under the same session, as an `<invoke>` with a fixed
 * id does, and the new run starts the invocation again. The first poster posts
 * the first run's ending twice, as a host that reports twice; its post finds the
 * slot's session while the first run holds it, and may land in the inbox after the
 * second run stands in the slot. The second poster posts the second run's ending
 * once the restart is done. `main` drains the slot's inbox once every thread is
 * joined. Held to docs/adr/0084, D2 ("an ending that lands in a slot restarted
 * since is refused by the token check") and docs/adr/0147, D1:
 *
 *   - a post the slot's inbox took is applied exactly once;
 *   - the first run's ending is accepted at most once, and only by the first run:
 *     applied after that run ended, in its own end or in the second run, it is
 *     refused by the token;
 *   - the second run's ending, when the inbox took it, is accepted.
 *
 * The flag the second poster waits for is the harness's own atomic, which no
 * mutant rewrites, and the wait is an assumption: an execution in which the
 * second poster runs before the restart is cut off, never read as a pass.
 */

#include <assert.h>
#include <pthread.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "../conformance/sce_atomic_host.c"

#define SCE_MAX_INBOX 1
#include "inbox_tree.h"

#define FIRST_POSTER 1u
#define SECOND_POSTER 2u

/* The owner's steps, recorded with each input it applies. */
#define STEP_FIRST_RUN 1u
#define STEP_FIRST_RUN_ENDS 2u
#define STEP_END_OF_LOOP 3u

static inbox_tree_t g_tree;

/* The tokens the two runs' invocations were started with. The second is written
   by the owner before it sets `g_restarted`. */
static uint64_t g_first_token;
static uint64_t g_second_token;
static uint32_t g_restarted;

static inbox_route_t g_first_routes[2];
static inbox_route_t g_second_route;

static void *post_the_first_runs_ending_twice(void *arg) {
    uint64_t seq;
    (void)arg;
    for (seq = 1u; seq <= 2u; seq++) {
        const inbox_input_t ending = inbox_ending(FIRST_POSTER, seq, INBOX_SESSION_FIRST, g_first_token);
        g_first_routes[seq - 1u] = inbox_tree_post_ending(&g_tree, &ending);
    }
    return NULL;
}

static void *post_the_second_runs_ending(void *arg) {
    inbox_input_t ending;
    (void)arg;
    __VERIFIER_assume(__atomic_load_n(&g_restarted, __ATOMIC_ACQUIRE) != 0u);
    ending = inbox_ending(SECOND_POSTER, 1u, INBOX_SESSION_FIRST, g_second_token);
    g_second_route = inbox_tree_post_ending(&g_tree, &ending);
    return NULL;
}

static void *own(void *arg) {
    (void)arg;
    inbox_step = STEP_FIRST_RUN;
    inbox_drain(&g_tree.child);
    inbox_step = STEP_FIRST_RUN_ENDS;
    inbox_tree_end_child(&g_tree);
    g_second_token = inbox_tree_start_child(&g_tree, INBOX_SESSION_FIRST);
    __atomic_store_n(&g_restarted, 1u, __ATOMIC_RELEASE);
    return NULL;
}

int main(void) {
    pthread_t first;
    pthread_t second;
    pthread_t owner;
    unsigned accepted_first = 0u;
    int at;
    uint64_t seq;

    inbox_tree_build(&g_tree);
    g_first_token = inbox_tree_start_child(&g_tree, INBOX_SESSION_FIRST);

    pthread_create(&first, NULL, post_the_first_runs_ending_twice, NULL);
    pthread_create(&second, NULL, post_the_second_runs_ending, NULL);
    pthread_create(&owner, NULL, own, NULL);
    pthread_join(first, NULL);
    pthread_join(second, NULL);
    pthread_join(owner, NULL);

    /* The end of the owner's loop: what the slot's inbox still holds is applied
       by the second run, which stands in the slot now. */
    inbox_step = STEP_END_OF_LOOP;
    inbox_drain(&g_tree.child);

    for (seq = 1u; seq <= 2u; seq++) {
        at = inbox_applied_once_if_taken(FIRST_POSTER, seq, g_first_routes[seq - 1u] == INBOX_ROUTE_TAKEN);
        if (at >= 0 && inbox_log[at].outcome == INBOX_ACCEPTED) {
            /* Only the first run accepts its ending. */
            assert(inbox_log[at].step == STEP_FIRST_RUN);
            accepted_first++;
        }
        if (at >= 0) {
            assert(inbox_log[at].outcome == INBOX_ACCEPTED || inbox_log[at].outcome == INBOX_REFUSED_BY_TOKEN);
        }
    }
    assert(accepted_first <= 1u);

    /* The second run's ending finds the session the second run holds, and the
       second run accepts it. */
    assert(g_second_route != INBOX_ROUTE_NOT_HELD);
    at = inbox_applied_once_if_taken(SECOND_POSTER, 1u, g_second_route == INBOX_ROUTE_TAKEN);
    if (at >= 0) {
        assert(inbox_log[at].outcome == INBOX_ACCEPTED);
    }
    (void)at;
    (void)accepted_first;
    return 0;
}
