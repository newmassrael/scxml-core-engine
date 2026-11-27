/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 3 */
/* REQUIRED: all */

/*
 * What this harness is expected to catch, and what has not been measured: every
 * ordering weakened (`all`), under which the owner may read the queue as it stood
 * before the second post, which returned before the owner heard of it. Chosen by
 * reading and NOT run on GenMC when this was written; the gate prints each mutant
 * that survives, and one listed here that survives is taken off the list by
 * whoever reads that output, with the reason.
 */

/*
 * The GenMC layer of the queue kind's verification (the Protocol-Synthesis RFC's
 * queue section), for the inbox of a generated C11 machine (sce/inbox.h; docs/adr/0084,
 * D2; docs/adr/0147, D1 and D5): a take past a post still under way.
 *
 * The first poster posts one input; the second posts one and then says it has
 * returned. The owner, having heard that the second post returned, drains the
 * inbox once, as the drain before an event of its own does, and `main` drains it
 * again once every thread is joined, as at the end of a run. Every interleaving
 * of the first post with the rest is explored, the one where it holds a ticket of
 * the queue and has not yet filled its entry among them: Vyukov's bounded queue
 * would answer empty there, in front of the second input, and the event of the
 * owner's own would enter ahead of a post that returned before it (the Rust
 * runtime's bounded inbox did, until its take learnt to wait). Held to
 * docs/adr/0147, D1:
 *
 *   - at most once, and never lost: every post the inbox took is applied exactly
 *     once, and every post it refused never is;
 *   - arrival order: a post that returned before the owner's drain began is
 *     applied by that drain, whatever another post still under way holds.
 *
 * The flag the owner reads is the harness's own atomic, which no mutant rewrites:
 * it orders the second post before the drain in real time and gives the owner
 * nothing of the queue.
 */

#include <assert.h>
#include <pthread.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "../conformance/sce_atomic_host.c"

#define SCE_MAX_INBOX 2
#include "inbox_tree.h"

#define FIRST_POSTER 1u
#define SECOND_POSTER 2u

static inbox_session_t g_session;

static bool g_first_taken;
static bool g_second_taken;
static uint32_t g_second_returned;
static bool g_owner_heard;

static void *post_first(void *arg) {
    const inbox_input_t input = inbox_event(FIRST_POSTER, 1u);
    (void)arg;
    g_first_taken = inbox_session_post(&g_session, &input);
    return NULL;
}

static void *post_second(void *arg) {
    const inbox_input_t input = inbox_event(SECOND_POSTER, 1u);
    (void)arg;
    g_second_taken = inbox_session_post(&g_session, &input);
    __atomic_store_n(&g_second_returned, 1u, __ATOMIC_SEQ_CST);
    return NULL;
}

static void *own(void *arg) {
    (void)arg;
    g_owner_heard = __atomic_load_n(&g_second_returned, __ATOMIC_SEQ_CST) != 0u;
    inbox_step = 1u;
    if (g_owner_heard) {
        inbox_drain(&g_session);
    }
    return NULL;
}

int main(void) {
    pthread_t first;
    pthread_t second;
    pthread_t owner;
    int first_at;
    int second_at;

    g_session.session_id = "root";
    if (!sce_inbox_init(&g_session.inbox)) {
        return 1;
    }
    pthread_create(&first, NULL, post_first, NULL);
    pthread_create(&second, NULL, post_second, NULL);
    pthread_create(&owner, NULL, own, NULL);
    pthread_join(first, NULL);
    pthread_join(second, NULL);
    pthread_join(owner, NULL);

    /* The end of the run: what the inbox still holds is applied. */
    inbox_step = 2u;
    inbox_drain(&g_session);

    first_at = inbox_applied_once_if_taken(FIRST_POSTER, 1u, g_first_taken);
    second_at = inbox_applied_once_if_taken(SECOND_POSTER, 1u, g_second_taken);
    /* The second post returned before the owner's drain began, so that drain
       applied it, past whatever the first post still held. */
    if (g_owner_heard && second_at >= 0) {
        assert(inbox_log[second_at].step == 1u);
    }
    (void)first_at;
    (void)second_at;
    return 0;
}
