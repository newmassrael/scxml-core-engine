/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 3 */
/* REQUIRED: load_acquire store_release fence_acquire fence_release acquire_release all */

/*
 * What this harness is expected to catch, and what has not been measured. The
 * second poster asks the slot for a session it never holds, made of the first
 * word of the session that leaves it and the second word of the one that enters
 * it, so a read that mixes the two writes answers yes. Read by the protocol of
 * the slot's sequence stamp (sce/inbox.h, `sce_inbox_directory_write_` and
 * `sce_inbox_directory_holds`):
 *
 *   - `fence_release` and `fence_acquire`: a reader that sees a word of the new
 *     write no longer sees the odd stamp before it, so its second load may find
 *     the stamp it first read;
 *   - `store_release` and `load_acquire`: a reader whose first load finds the
 *     even stamp the new write ended with no longer sees the words written before
 *     it, and may read the first word as it was;
 *   - `acquire_release` and `all`, which weaken those together.
 *
 * Each of the six lets the mixed read through and the harness's assertion fails.
 * They were chosen by that reading and were NOT run on GenMC when this was
 * written; the gate prints each mutant that survives, and one listed here that
 * survives is taken off the list by whoever reads that output, with the reason.
 */

/*
 * The GenMC layer of the queue kind's verification (the Protocol-Synthesis RFC's
 * queue section), for the directory of a generated C11 tree (sce/inbox.h; docs/adr/0084,
 * D2 as amended at step 10; docs/adr/0147, D5): a post naming a session that is
 * gone.
 *
 * A child session runs in the root's one slot when the threads start. The owner
 * ends it (its invocation is cancelled, it leaves the directory and what its inbox
 * holds is applied) and starts another child, under another session, in the same
 * slot, as an `<invoke>` whose id is drawn at each start does. Meanwhile the first
 * poster posts an ending for the session that is ending, and the second an ending
 * for the mixed session above; `main` drains the slot's inbox once every thread
 * is joined, as at the end of the owner's loop, since a C11 inbox is never closed.
 * Held to docs/adr/0147, D1:
 *
 *   - a post naming a session the tree does not hold is refused when it is
 *     posted, and counted: the second poster's always, and the first poster's
 *     whenever it began after the session had ended;
 *   - a post the slot's inbox took is applied exactly once, and an ending for the
 *     session that ended is never accepted: the session it names is either over
 *     (its invocation cancelled) or another session stands in the slot.
 *
 * The flag the first poster reads is the harness's own atomic, which no mutant
 * rewrites: it says that the session had ended, and so left the directory, before
 * the post began.
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

static inbox_tree_t g_tree;

/* The token the first child's invocation was started with. */
static uint64_t g_first_token;
/* Set once the first child has ended. */
static uint32_t g_ended;

static inbox_route_t g_first_route;
static bool g_first_after_end;
static inbox_route_t g_second_route;

static void *post_for_the_ending_session(void *arg) {
    const inbox_input_t ending = inbox_ending(FIRST_POSTER, 1u, INBOX_SESSION_FIRST, g_first_token);
    (void)arg;
    g_first_after_end = __atomic_load_n(&g_ended, __ATOMIC_SEQ_CST) != 0u;
    g_first_route = inbox_tree_post_ending(&g_tree, &ending);
    return NULL;
}

static void *post_for_a_session_never_held(void *arg) {
    const inbox_input_t ending = inbox_ending(SECOND_POSTER, 1u, INBOX_SESSION_MIXED, g_first_token);
    (void)arg;
    g_second_route = inbox_tree_post_ending(&g_tree, &ending);
    return NULL;
}

static void *own(void *arg) {
    (void)arg;
    inbox_step = 1u;
    inbox_tree_end_child(&g_tree);
    __atomic_store_n(&g_ended, 1u, __ATOMIC_SEQ_CST);
    inbox_step = 2u;
    (void)inbox_tree_start_child(&g_tree, INBOX_SESSION_SECOND);
    return NULL;
}

int main(void) {
    pthread_t first;
    pthread_t second;
    pthread_t owner;
    unsigned not_held;
    unsigned full;
    int first_at;

    inbox_tree_build(&g_tree);
    g_first_token = inbox_tree_start_child(&g_tree, INBOX_SESSION_FIRST);

    pthread_create(&first, NULL, post_for_the_ending_session, NULL);
    pthread_create(&second, NULL, post_for_a_session_never_held, NULL);
    pthread_create(&owner, NULL, own, NULL);
    pthread_join(first, NULL);
    pthread_join(second, NULL);
    pthread_join(owner, NULL);

    /* The end of the owner's loop: what the slot's inbox still holds is applied
       by the session that stands in it now. */
    inbox_step = 3u;
    inbox_drain(&g_tree.child);

    /* The tree never held the mixed session. */
    assert(g_second_route == INBOX_ROUTE_NOT_HELD);
    /* A post that began after the session ended found no session. */
    if (g_first_after_end) {
        assert(g_first_route == INBOX_ROUTE_NOT_HELD);
    }
    (void)inbox_applied_once_if_taken(SECOND_POSTER, 1u, false);
    first_at = inbox_applied_once_if_taken(FIRST_POSTER, 1u, g_first_route == INBOX_ROUTE_TAKEN);
    if (first_at >= 0) {
        assert(inbox_log[first_at].outcome == INBOX_REFUSED_BY_TOKEN ||
               inbox_log[first_at].outcome == INBOX_DROPPED_BY_SESSION);
    }

    not_held = (g_first_route == INBOX_ROUTE_NOT_HELD ? 1u : 0u) + (g_second_route == INBOX_ROUTE_NOT_HELD ? 1u : 0u);
    full = g_first_route == INBOX_ROUTE_FULL ? 1u : 0u;
    assert(sce_inbox_refused(&g_tree.root.inbox) == not_held);
    assert(sce_inbox_refused(&g_tree.child.inbox) == full);
    (void)first_at;
    (void)not_held;
    (void)full;
    return 0;
}
