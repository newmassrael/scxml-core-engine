// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0086, section 8.5 (a): a C11 machine whose data model is a script
// engine holds each event's data in a text its engine's allocator gives it.
// The queue entry owns the text; the dequeue hands it to the engine and frees
// it; a scheduled entry owns its text until it comes due, when it is moved into
// a queue, or until a <cancel> or the machine's destroy frees it. Each case
// reads how many texts the machine holds (`_owned_event_texts`) after each act,
// so a text kept past its last holder reads as one too many and a text freed
// twice as a count that fell past zero.
//
// Fixture:
// integration_resources/a_script_machine_owns_each_event_text/a_script_machine_owns_each_event_text.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_script_machine_owns_each_event_text ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "a_script_machine_owns_each_event_text_sm.h"

typedef a_script_machine_owns_each_event_text_t sm_t;
typedef a_script_machine_owns_each_event_text_event_with_meta_t meta_t;

#define TAKEN_CAP 256

/* Hand the machine the event `name` with `data`, a view it copies, NULL for
   none. */
static void hand(sm_t *sm, const char *name, const char *data) {
    meta_t meta;
    memset(&meta, 0, sizeof(meta));
    (void)a_script_machine_owns_each_event_text_resolve_event_by_name(name, &meta.event);
    meta.data = data;
    a_script_machine_owns_each_event_text_raise_external(sm, &meta);
}

static int expect_owned(const char *what, const sm_t *sm, uint32_t want) {
    const uint32_t got = a_script_machine_owns_each_event_text_owned_event_texts(sm);
    if (got != want) {
        (void)fprintf(stderr, "FAIL: %s: the machine holds %u event texts, want %u\n", what, (unsigned)got,
                      (unsigned)want);
        return 1;
    }
    return 0;
}

static int expect_taken(const char *what, const sm_t *sm, const char *want) {
    char got[TAKEN_CAP];
    size_t len = 0u;
    if (!a_script_machine_owns_each_event_text_taken(sm, got, sizeof(got), &len) || len >= sizeof(got)) {
        (void)fprintf(stderr, "FAIL: %s: `taken` could not be read\n", what);
        return 1;
    }
    if (strcmp(got, want) != 0) {
        (void)fprintf(stderr, "FAIL: %s:\n  taken `%s`\n  want  `%s`\n", what, got, want);
        return 1;
    }
    return 0;
}

/* Each event a host hands the machine owns a text from the moment it is queued
   until it has been processed, and no longer. */
static int a_dequeued_event_frees_its_text(void) {
    sm_t sm;
    int bad = 0;
    a_script_machine_owns_each_event_text_init_with_clock(&sm, sce_clock_manual(0u));
    bad |= expect_owned("before any event", &sm, 0u);
    hand(&sm, "note", "\"a\"");
    hand(&sm, "note", "\"b\"");
    hand(&sm, "drop", NULL);
    bad |= expect_owned("two queued events carry data and one carries none", &sm, 2u);
    a_script_machine_owns_each_event_text_step(&sm);
    bad |= expect_taken("each event was read with its own data", &sm, "a;b;");
    bad |= expect_owned("every processed event's text was freed", &sm, 0u);
    a_script_machine_owns_each_event_text_destroy(&sm);
    bad |= expect_owned("after destroy", &sm, 0u);
    return bad;
}

/* An event the document sends to its internal queue owns its text there, and
   the internal drain frees it once the engine read it. */
static int an_internal_event_frees_its_text_once_read(void) {
    sm_t sm;
    int bad = 0;
    a_script_machine_owns_each_event_text_init_with_clock(&sm, sce_clock_manual(0u));
    hand(&sm, "echo", NULL);
    a_script_machine_owns_each_event_text_step(&sm);
    bad |= expect_taken("the internal event carried its data", &sm, "inner;");
    bad |= expect_owned("the internal event's text was freed", &sm, 0u);
    a_script_machine_owns_each_event_text_destroy(&sm);
    return bad;
}

/* A scheduled entry owns its text until it comes due or is cancelled. The
   cancelled one is freed by the <cancel>; the due one is moved into the
   external queue and freed when it has been processed. */
static int a_cancelled_entry_frees_its_text_and_a_due_one_moves_it(void) {
    sm_t sm;
    int bad = 0;
    a_script_machine_owns_each_event_text_init_with_clock(&sm, sce_clock_manual(0u));
    hand(&sm, "arm", NULL);
    a_script_machine_owns_each_event_text_step(&sm);
    bad |= expect_owned("two scheduled entries carry data", &sm, 2u);
    hand(&sm, "drop", NULL);
    a_script_machine_owns_each_event_text_step(&sm);
    bad |= expect_owned("the cancelled entry's text was freed", &sm, 1u);
    a_script_machine_owns_each_event_text_advance_time_ms(&sm, 1000u);
    bad |= expect_taken("the due entry was delivered with its data", &sm, "due;");
    bad |= expect_owned("the delivered entry's text was freed once processed", &sm, 0u);
    a_script_machine_owns_each_event_text_advance_time_ms(&sm, 20000u);
    bad |= expect_taken("the cancelled entry was never delivered", &sm, "due;");
    a_script_machine_owns_each_event_text_destroy(&sm);
    return bad;
}

/* Destroy frees what the machine still holds: queued events nothing has
   processed and scheduled entries that have not come due. */
static int destroy_frees_every_text_the_machine_still_holds(void) {
    sm_t sm;
    int bad = 0;
    a_script_machine_owns_each_event_text_init_with_clock(&sm, sce_clock_manual(0u));
    hand(&sm, "arm", NULL);
    a_script_machine_owns_each_event_text_step(&sm);
    hand(&sm, "note", "\"x\"");
    hand(&sm, "note", "\"y\"");
    hand(&sm, "note", "\"z\"");
    bad |= expect_owned("two scheduled entries and three queued events", &sm, 5u);
    a_script_machine_owns_each_event_text_destroy(&sm);
    bad |= expect_owned("destroy freed every text", &sm, 0u);
    /* A second destroy finds nothing left to free. */
    a_script_machine_owns_each_event_text_destroy(&sm);
    bad |= expect_owned("a second destroy", &sm, 0u);
    return bad;
}

/* The engine's allocator, wrapped so a case can make the next allocation
   fail. Every other call goes to the allocator the engine was created with,
   which therefore frees what it gave before the wrap. */
typedef struct {
    lua_Alloc inner;
    void *inner_context;
    bool fail_next;
    uint32_t failed;
} failing_alloc_t;

static void *failing_alloc(void *context, void *ptr, size_t osize, size_t nsize) {
    failing_alloc_t *self = (failing_alloc_t *)context;
    if (ptr == NULL && nsize != 0u && self->fail_next) {
        self->fail_next = false;
        self->failed++;
        return NULL;
    }
    return self->inner(self->inner_context, ptr, osize, nsize);
}

/* An event whose text the allocator cannot give is refused whole: counted,
   raised as error.execution, and never queued with its data cut or lost. */
static int an_event_the_allocator_cannot_hold_is_refused_and_raised(void) {
    sm_t sm;
    failing_alloc_t wrap;
    int bad = 0;
    a_script_machine_owns_each_event_text_init_with_clock(&sm, sce_clock_manual(0u));
    memset(&wrap, 0, sizeof(wrap));
    wrap.inner = lua_getallocf(sm.L, &wrap.inner_context);
    lua_setallocf(sm.L, failing_alloc, &wrap);
    /* The raise drains an empty inbox and allocates the event's text: the
       first allocation it makes is the text's. */
    wrap.fail_next = true;
    hand(&sm, "note", "\"lost\"");
    wrap.fail_next = false;
    if (wrap.failed != 1u) {
        (void)fprintf(stderr, "FAIL: the raise made no allocation for the event's text\n");
        bad = 1;
    }
    if (a_script_machine_owns_each_event_text_refused_event_data(&sm) != 1u) {
        (void)fprintf(stderr, "FAIL: the refused event was counted %u times, want 1\n",
                      (unsigned)a_script_machine_owns_each_event_text_refused_event_data(&sm));
        bad = 1;
    }
    a_script_machine_owns_each_event_text_step(&sm);
    bad |= expect_taken("the refusal was raised as error.execution and the event never ran", &sm, "refused;");
    bad |= expect_owned("nothing is held for the refused event", &sm, 0u);
    a_script_machine_owns_each_event_text_destroy(&sm);
    return bad;
}

int main(void) {
    int bad = 0;
    bad |= a_dequeued_event_frees_its_text();
    bad |= an_internal_event_frees_its_text_once_read();
    bad |= a_cancelled_entry_frees_its_text_and_a_due_one_moves_it();
    bad |= destroy_frees_every_text_the_machine_still_holds();
    bad |= an_event_the_allocator_cannot_hold_is_refused_and_raised();
    if (bad == 0) {
        (void)printf("a_script_machine_owns_each_event_text: all cases passed\n");
    }
    return bad;
}
