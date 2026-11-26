/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 3 */
/* REQUIRED: cas rmw all */

/*
 * What this harness is expected to catch, and what has not been measured: the
 * same three mutants as inbox_two_posters.c, for the same reason. An inbox of one
 * slot takes the second post only into the slot the owner read the first out of,
 * which is scq64_slot_reuse.c's scenario with the reused slot written by a poster
 * other than the first. Chosen by reading and NOT run on GenMC when this was
 * written; the gate prints each mutant that survives.
 */

/*
 * The GenMC layer of the queue kind's verification (the Protocol-Synthesis RFC's
 * queue section), for the inbox of a generated C11 machine (sce/inbox.h; docs/adr/0084,
 * D2; docs/adr/0147, D5): the refusal of a full inbox.
 *
 * The inbox holds one input, and its queue hands out one producer place. Two
 * posters post one input each while the owner drains once; `main` drains again
 * once every thread is joined. A post is refused when the other post holds the
 * room, the slot or the place; nothing else can hold it, since the owner holds the
 * slot only for an input a post put there. Held to docs/adr/0147, D1, and to the
 * bounded inbox of docs/adr/0084, D2 (nothing is dropped silently):
 *
 *   - a post the inbox took is applied exactly once, and a post it refused never
 *     is;
 *   - each refusal was answered false and counted, and only refusals were;
 *   - a refusal means the inbox was full: of two posts, at least one is taken.
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

static inbox_session_t g_session;

static bool g_taken[2];

static void *post(void *arg) {
    const uint64_t poster = (uint64_t)(uintptr_t)arg;
    const inbox_input_t input = inbox_event(poster, 1u);
    g_taken[poster - 1u] = inbox_session_post(&g_session, &input);
    return NULL;
}

static void *own(void *arg) {
    (void)arg;
    inbox_step = 1u;
    inbox_drain(&g_session);
    return NULL;
}

int main(void) {
    pthread_t first;
    pthread_t second;
    pthread_t owner;
    unsigned refused;

    g_session.session_id = "root";
    if (!sce_inbox_init(&g_session.inbox)) {
        return 1;
    }
    pthread_create(&first, NULL, post, (void *)(uintptr_t)1u);
    pthread_create(&second, NULL, post, (void *)(uintptr_t)2u);
    pthread_create(&owner, NULL, own, NULL);
    pthread_join(first, NULL);
    pthread_join(second, NULL);
    pthread_join(owner, NULL);

    /* The end of the run: what the inbox still holds is applied. */
    inbox_step = 2u;
    inbox_drain(&g_session);

    (void)inbox_applied_once_if_taken(1u, 1u, g_taken[0]);
    (void)inbox_applied_once_if_taken(2u, 1u, g_taken[1]);
    refused = (g_taken[0] ? 0u : 1u) + (g_taken[1] ? 0u : 1u);
    assert(sce_inbox_refused(&g_session.inbox) == refused);
    assert(refused < 2u);
    (void)refused;
    return 0;
}
