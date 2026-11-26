/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * What the C11 inbox harnesses share (inbox_*.c; docs/adr/0147, D5): the session
 * of a generated C11 machine as far as its inbox reaches, a tree of a root and one
 * child slot, and the two halves of the door, the posters' and the owner's.
 * Included by a harness after the hosted atomics, with `SCE_MAX_INBOX` defined.
 * It is found through the gate's include path, not beside the harness, because a
 * mutant's copy of the harness is compiled from the gate's work directory.
 *
 * WHAT IS THE RUNTIME'S AND WHAT IS WRITTEN HERE. Every atomic the C11 door uses is
 * in sce/inbox.h: the queue of a session's inbox, the count of refused posts, and
 * the directory slot a child session is found through, read under its sequence
 * stamp. The generated machine adds none: no template under
 * tools/codegen/templates/c/ calls an `sce_atomic_*` function. So the harnesses
 * call sce/inbox.h as the generated code calls it, and judge the token check by
 * sce/host_processor.h's own functions; what is written out below is only the
 * generated code's sequential composition of them (which inbox a post goes to,
 * what a drain applies, what a restart zeroes), each function naming the template
 * text it stands for. A change of that composition in the templates is not followed
 * here by itself, so it is the change that must carry this file along.
 *
 * WHAT IS CUT, AND WHY. The bounds are the smallest that hold the scenarios, since
 * GenMC kills an execution at a loop of more than the gate's unroll bound (20): a
 * session id of at most 11 bytes (`SCE_MAX_SESSION_ID_LEN` 12, three directory
 * words, where the default 128 makes the directory's write a loop of 32), and one
 * look of a dequeue at an entry still empty (`SCE_QUEUE_SCQ_SPINS`, as
 * lscq_segment_handover.c gives it, for the reason given there). An input is five
 * 64-bit words, not a generated `_inbox_input_t`, because the queue copies a slot
 * whole and a copy of hundreds of bytes is a loop past the bound; the words are
 * what the generated input carries that the door reads (the session an ending
 * names and its token) and what the harness needs to tell posts apart. The owner
 * applies an input to a log rather than to an external queue, and its drain is not
 * held back by a full external queue: neither changes what the door does.
 */

#ifndef SCE_GENMC_INBOX_TREE_H
#define SCE_GENMC_INBOX_TREE_H

#include <assert.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#ifndef SCE_MAX_INBOX
#error "a harness defines SCE_MAX_INBOX, the capacity of each session's inbox, before it includes inbox_tree.h"
#endif

#define SCE_MAX_SESSION_ID_LEN 12
#define SCE_INBOX_ATOMIC_RMW_WIDTH 64
#define SCE_QUEUE_SCQ_SPINS 1u

/*
 * The runtime copies with `memcpy` at a length it holds at run time, which GenMC
 * does not turn into memory events (scq64_two_threads.c says what that hid). So
 * the copy is written out: by 64-bit words while both ends are aligned for them,
 * which is every copy of an input, and then by bytes, which is every copy of the
 * short ids the token check keeps. Either way each byte is read and written at the
 * width it was written at, so no access GenMC sees mixes sizes.
 */
static void *genmc_copy(void *destination, const void *source, size_t bytes) {
    unsigned char *to = (unsigned char *)destination;
    const unsigned char *from = (const unsigned char *)source;
    size_t done = 0u;
    if ((((uintptr_t)destination | (uintptr_t)source) % sizeof(uint64_t)) == 0u) {
        uint64_t *to_words = (uint64_t *)destination;
        const uint64_t *from_words = (const uint64_t *)source;
        for (; done + sizeof(uint64_t) <= bytes; done += sizeof(uint64_t)) {
            to_words[done / sizeof(uint64_t)] = from_words[done / sizeof(uint64_t)];
        }
    }
    for (; done < bytes; done++) {
        to[done] = from[done];
    }
    return destination;
}

#define memcpy genmc_copy

#include <sce/host_processor.h>
#include <sce/inbox.h>

#include <genmc.h>

/* ── An input ─────────────────────────────────────────────────────── */

/* An external event, applied by being counted, or a host-invoke ending, applied
   by the session check and the token check. */
#define INBOX_EVENT 1u
#define INBOX_ENDING 2u

/* The sessions an ending can name, by index: none, the two a slot holds in turn,
   and one no slot ever holds whose first word is the first's and whose second is
   the third's. */
#define INBOX_SESSION_FIRST 1u
#define INBOX_SESSION_MIXED 2u
#define INBOX_SESSION_SECOND 3u

static const char *const inbox_session_ids[] = {"", "AAAAXXXX", "AAAABBBB", "YYYYBBBB"};

/* The one host-run invocation of the child, whose start the token names. */
static const char inbox_processor_type[] = "t";
static const char inbox_invoke_id[] = "i";

typedef struct {
    uint64_t kind;
    /* The session an ending names, an index into `inbox_session_ids`. */
    uint64_t session;
    /* The token of the start the ending is for. */
    uint64_t token;
    /* Who posted it, and its place among that poster's posts, from 1. */
    uint64_t poster;
    uint64_t seq;
} inbox_input_t;

static inline inbox_input_t inbox_event(uint64_t poster, uint64_t seq) {
    inbox_input_t input;
    input.kind = INBOX_EVENT;
    input.session = 0u;
    input.token = 0u;
    input.poster = poster;
    input.seq = seq;
    return input;
}

static inline inbox_input_t inbox_ending(uint64_t poster, uint64_t seq, uint64_t session, uint64_t token) {
    inbox_input_t input;
    input.kind = INBOX_ENDING;
    input.session = session;
    input.token = token;
    input.poster = poster;
    input.seq = seq;
    return input;
}

/* Written into the slot the inbox holds for it, field by field, as the generated
   `_inbox_write_ending` writes an ending (tools/codegen/templates/c/
   state_machine.c.jinja2): an input is never assembled on the poster's stack and
   copied in. */
static inline void inbox_write(void *slot, const void *context) {
    inbox_input_t *input = (inbox_input_t *)slot;
    const inbox_input_t *from = (const inbox_input_t *)context;
    input->kind = from->kind;
    input->session = from->session;
    input->token = from->token;
    input->poster = from->poster;
    input->seq = from->seq;
}

/* ── A session ────────────────────────────────────────────────────── */

/* A generated machine as far as the door reaches it, in its order: what a child
   started again in its slot loses (`_init_construct` zeroes the machine up to
   `inbox`), then what it keeps, its inbox and the slots of it. */
typedef struct {
    const char *session_id;
    sce_host_invocation_set_t started;
    sce_inbox_t inbox;
    inbox_input_t slots[SCE_MAX_INBOX];
} inbox_session_t;

/* A post to this session's own inbox: the generated `_inbox_post_ending`, and
   `_inbox_post` for an input that names no other session. */
static inline bool inbox_session_post(inbox_session_t *session, const inbox_input_t *input) {
    return sce_inbox_post_with(&session->inbox, (unsigned char *)session->slots, sizeof(session->slots[0]), inbox_write,
                               input);
}

/* ── What the owner applied ───────────────────────────────────────── */

typedef enum {
    INBOX_COUNTED,
    INBOX_ACCEPTED,
    INBOX_REFUSED_BY_TOKEN,
    INBOX_DROPPED_BY_SESSION,
} inbox_outcome_t;

typedef struct {
    uint64_t poster;
    uint64_t seq;
    inbox_outcome_t outcome;
    /* The owner's step it was applied in, which the harness names. */
    unsigned step;
} inbox_applied_t;

#define INBOX_LOG_MAX 8u

/* Written by the owner alone, and read by `main` once every thread is joined. */
static inbox_applied_t inbox_log[INBOX_LOG_MAX];
static unsigned inbox_log_count;
static unsigned inbox_step;

/* Apply one input by the operation its kind names: the generated
   `apply_inbox_input`, and for an ending its `apply_host_invoke_ending`, which
   drops one naming another session and otherwise asks the token check. */
static inline void inbox_apply(inbox_session_t *session, const inbox_input_t *input) {
    inbox_outcome_t outcome = INBOX_COUNTED;
    assert(input->kind == INBOX_EVENT || input->kind == INBOX_ENDING);
    assert(input->poster == 1u || input->poster == 2u);
    if (input->kind == INBOX_ENDING) {
        if (sce_text_compare(inbox_session_ids[input->session], session->session_id) != 0) {
            outcome = INBOX_DROPPED_BY_SESSION;
        } else if (sce_host_invocation_take_token(&session->started, inbox_processor_type, inbox_invoke_id,
                                                  input->token)) {
            outcome = INBOX_ACCEPTED;
        } else {
            outcome = INBOX_REFUSED_BY_TOKEN;
        }
    }
    assert(inbox_log_count < INBOX_LOG_MAX);
    inbox_log[inbox_log_count].poster = input->poster;
    inbox_log[inbox_log_count].seq = input->seq;
    inbox_log[inbox_log_count].outcome = outcome;
    inbox_log[inbox_log_count].step = inbox_step;
    inbox_log_count++;
}

/* Apply what the session's inbox holds, oldest first and at most as many as it
   holds: the generated `drain_inbox`. The owner's. */
static inline void inbox_drain(inbox_session_t *session) {
    inbox_input_t input;
    uint32_t applied = 0u;
    while (applied < (uint32_t)SCE_MAX_INBOX &&
           sce_inbox_take(&session->inbox, (unsigned char *)session->slots, sizeof(session->slots[0]), &input)) {
        inbox_apply(session, &input);
        applied++;
    }
}

/* Where in the log the post `seq` of `poster` was applied, or -1 when it was not.
   A post applied twice is an error of its own. */
static inline int inbox_applied_at(uint64_t poster, uint64_t seq) {
    int at = -1;
    unsigned i;
    for (i = 0u; i < inbox_log_count; i++) {
        if (inbox_log[i].poster == poster && inbox_log[i].seq == seq) {
            assert(at < 0);
            at = (int)i;
        }
    }
    return at;
}

/* At most once, and never lost: a post the inbox took was applied exactly once,
   and one it refused never was. Answers where it was applied. */
static inline int inbox_applied_once_if_taken(uint64_t poster, uint64_t seq, bool taken) {
    const int at = inbox_applied_at(poster, seq);
    assert(taken == (at >= 0));
    return at;
}

/* ── A tree of a root and one child slot ──────────────────────────── */

/* The root, the directory entry of its one child slot, and the session that
   stands in the slot, in the generated layout's order: the directory beside the
   root's inbox, the child last. */
typedef struct {
    inbox_session_t root;
    sce_inbox_directory_slot_t child_directory;
    inbox_session_t child;
} inbox_tree_t;

/* Build every inbox of the tree: the generated `_build_inboxes`, run by the
   root's initialiser before any other context can post. The directory starts
   zeroed, which holds no session. */
static inline void inbox_tree_build(inbox_tree_t *tree) {
    const bool root_built = sce_inbox_init(&tree->root.inbox);
    const bool child_built = sce_inbox_init(&tree->child.inbox);
    assert(root_built && child_built);
    (void)root_built;
    (void)child_built;
    tree->root.session_id = "root";
}

/* A child session starts in the slot, under the session `session` names; the
   token its one host-run invocation was started with. As the generated start of a
   `sce-static` child does it (tools/codegen/templates/c/invoke_methods.jinja2):
   `_invoked_begin` zeroes the machine up to its inbox, the session id is bound,
   the session enters the directory (_macros/c_tree_directory.jinja2, `enter`),
   and its entry walk starts the invocation, which marks it in its own table.

   Of what the zeroing clears, the token check reads only the table's count and
   the token the next start receives, so those two are what is cleared here: a
   `memset` of the whole table is one GenMC cannot follow. */
static inline uint64_t inbox_tree_start_child(inbox_tree_t *tree, uint64_t session) {
    inbox_session_t *child = &tree->child;
    child->started.count = 0;
    child->started.next_token = 0u;
    child->session_id = inbox_session_ids[session];
    sce_inbox_directory_enter(&tree->child_directory, child->session_id);
    return sce_host_invocation_mark(&child->started, inbox_processor_type, inbox_invoke_id);
}

/* The child session ends: its state's exit cancels the invocation it started,
   the session leaves the directory, and what its inbox still holds is applied
   (tools/codegen/templates/c/state_machine.c.jinja2, the exit of an invoking
   state; _macros/c_tree_directory.jinja2, `leave` and `apply_held`). */
static inline void inbox_tree_end_child(inbox_tree_t *tree) {
    inbox_session_t *child = &tree->child;
    (void)sce_host_invocation_take(&child->started, inbox_processor_type, inbox_invoke_id, NULL);
    sce_inbox_directory_leave(&tree->child_directory);
    inbox_drain(child);
}

typedef enum {
    INBOX_ROUTE_TAKEN,
    INBOX_ROUTE_FULL,
    INBOX_ROUTE_NOT_HELD,
} inbox_route_t;

/* Post an ending that names a session below the root: the generated `_inbox_post`
   and `_inbox_route_ending` for a tree of one slot. The address is resolved
   here, when it is posted: an ending whose session the slot holds goes to that
   session's own inbox, which may refuse it as full; one naming a session the tree
   does not hold is refused and counted on the root. Any context's. */
static inline inbox_route_t inbox_tree_post_ending(inbox_tree_t *tree, const inbox_input_t *ending) {
    if (sce_inbox_directory_holds(&tree->child_directory, inbox_session_ids[ending->session])) {
        return inbox_session_post(&tree->child, ending) ? INBOX_ROUTE_TAKEN : INBOX_ROUTE_FULL;
    }
    sce_inbox_count_refusal(&tree->root.inbox);
    return INBOX_ROUTE_NOT_HELD;
}

#endif /* SCE_GENMC_INBOX_TREE_H */
