/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 3 */
/* REQUIRED: cas rmw all */

/*
 * What this harness is expected to catch, and what has not been measured: the
 * compare-and-swap weakened to relaxed (`cas`), every read-modify-write weakened
 * (`rmw`) and every ordering weakened (`all`). An inbox of two slots takes three
 * posts only when the owner has applied one of them in between, so the third is
 * written into a slot the owner read an input out of: the slot reuse of
 * scq64_slot_reuse.c, where those three were measured caught (2026-10-10), over
 * the same queue row. They were chosen by that reading and were NOT run on GenMC
 * when this was written; the gate prints each mutant that survives, and one listed
 * here that survives is taken off the list by whoever reads that output, with the
 * reason.
 */

/*
 * The GenMC layer of the queue kind's verification (the Protocol-Synthesis RFC's
 * queue section), for the inbox of a generated C11 machine (sce/inbox.h; docs/adr/0084;
 * docs/adr/0147, D5): two posters and the owner, on threads of their own.
 *
 * The first poster posts two inputs and then says so; the second posts one,
 * noting whether it had heard that the first was done. The owner drains the inbox
 * once while they post, as at a macrostep boundary, and `main` drains it again
 * once every thread is joined, as at the end of a run. Held to docs/adr/0147, D1:
 *
 *   - at most once, and never lost: every post the inbox took is applied exactly
 *     once, every post it refused never is, and the inbox counted each refusal;
 *   - arrival order: the first poster's two posts are applied in the order they
 *     were made, and a post made after another had returned (the second poster's,
 *     when it heard the first was done) is applied after it.
 *
 * The flag the second poster reads is the harness's own atomic, which no mutant
 * rewrites: it orders the posts against each other in real time and gives the
 * owner nothing.
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

static bool g_first_taken[2];
static bool g_second_taken;
static uint32_t g_first_done;
static bool g_second_after_first;

static void *post_two(void *arg) {
    uint64_t seq;
    (void)arg;
    for (seq = 1u; seq <= 2u; seq++) {
        const inbox_input_t input = inbox_event(FIRST_POSTER, seq);
        g_first_taken[seq - 1u] = inbox_session_post(&g_session, &input);
    }
    __atomic_store_n(&g_first_done, 1u, __ATOMIC_SEQ_CST);
    return NULL;
}

static void *post_one(void *arg) {
    const inbox_input_t input = inbox_event(SECOND_POSTER, 1u);
    (void)arg;
    g_second_after_first = __atomic_load_n(&g_first_done, __ATOMIC_SEQ_CST) != 0u;
    g_second_taken = inbox_session_post(&g_session, &input);
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
    unsigned refused = 0u;
    int first_at[2];
    int second_at;
    unsigned i;

    g_session.session_id = "root";
    if (!sce_inbox_init(&g_session.inbox)) {
        return 1;
    }
    pthread_create(&first, NULL, post_two, NULL);
    pthread_create(&second, NULL, post_one, NULL);
    pthread_create(&owner, NULL, own, NULL);
    pthread_join(first, NULL);
    pthread_join(second, NULL);
    pthread_join(owner, NULL);

    /* The end of the run: what the inbox still holds is applied. */
    inbox_step = 2u;
    inbox_drain(&g_session);

    first_at[0] = inbox_applied_once_if_taken(FIRST_POSTER, 1u, g_first_taken[0]);
    first_at[1] = inbox_applied_once_if_taken(FIRST_POSTER, 2u, g_first_taken[1]);
    second_at = inbox_applied_once_if_taken(SECOND_POSTER, 1u, g_second_taken);
    refused += g_first_taken[0] ? 0u : 1u;
    refused += g_first_taken[1] ? 0u : 1u;
    refused += g_second_taken ? 0u : 1u;
    assert(sce_inbox_refused(&g_session.inbox) == refused);
    assert(inbox_log_count == 3u - refused);
    for (i = 0u; i < inbox_log_count; i++) {
        assert(inbox_log[i].outcome == INBOX_COUNTED);
    }

    /* The first poster's posts in the order it made them. */
    if (first_at[0] >= 0 && first_at[1] >= 0) {
        assert(first_at[0] < first_at[1]);
    }
    /* A post made after the first poster's had returned comes after them. */
    if (g_second_after_first && second_at >= 0) {
        assert(first_at[0] < second_at);
        assert(first_at[1] < second_at);
    }
    (void)first_at;
    (void)second_at;
    return 0;
}
