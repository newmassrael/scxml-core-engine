// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RFC §synth-5-D — forge bounded-collection C11 output, exercised
// rather than read.
//
// Sibling of `test_forge_buffer_pool.c`, and the reason it exists is
// slightly different. The pool header had no compile target at all; the
// bounded collection was compiled, but only by accident — a keyexpr
// fixture happens to emit one, so `c7_keyexpr_field_match` dragged it
// through a compiler as a side effect. Accidental coverage disappears
// the moment that unrelated fixture changes, and it never covered
// behaviour in the first place.
//
// The property worth asserting here is the generation-tagged handle.
// `insert` returns a handle carrying the slot's current generation;
// `remove` bumps it. That is what makes a stale handle to a *reused*
// slot detectable — the ABA case, where a naive slot-index handle would
// silently address whatever moved in afterwards. A text grep over the
// emitted source cannot see any of it.
//
// Fixtures: tests/forge/resources/{subscription_entry,local_sub_table}.scxml
// (capacity 8, `<sce:on-overflow>reject</sce:on-overflow>`).

#include <stdio.h>

#include "local_sub_table.h"

static int failures = 0;

#define CHECK(cond, msg)                                                                                               \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            fprintf(stderr, "forge_bounded_collection: FAIL — %s\n  (%s, line %d)\n", (msg), #cond, __LINE__);         \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

static subscription_entry_t entry(uint32_t id) {
    subscription_entry_t e;
    e.callback_id = id;
    return e;
}

/// insert → get → remove, with `len` following each step.
static void insert_get_remove_round_trip(void) {
    local_sub_table_t t;
    local_sub_table_init(&t);
    CHECK(local_sub_table_len(&t) == 0u, "a freshly initialised table is empty");

    const subscription_entry_t e = entry(4242u);
    local_sub_table_insert_result_t r = local_sub_table_insert(&t, &e);
    CHECK(r.ok, "inserting into a table with room must succeed");
    CHECK(local_sub_table_len(&t) == 1u, "insert must raise len by one");

    const subscription_entry_t *got = local_sub_table_get(&t, r.handle);
    CHECK(got != NULL, "a live handle must resolve");
    if (got != NULL) {
        CHECK(got->callback_id == 4242u, "the element read back must be the one inserted");
    }

    CHECK(local_sub_table_remove(&t, r.handle), "removing a live handle must succeed");
    CHECK(local_sub_table_len(&t) == 0u, "remove must lower len by one");
    CHECK(local_sub_table_get(&t, r.handle) == NULL, "a removed handle must stop resolving");
}

/// The ABA case: a handle to a slot that has since been reused must not
/// resolve, and must not remove the new occupant.
///
/// This is the assertion the generation counter exists for. With a
/// slot-index-only handle every check here still passes on the *first*
/// insert and then silently addresses the wrong element — which is why
/// the reused slot, not merely a removed one, is what is tested.
static void stale_handle_to_a_reused_slot_is_rejected(void) {
    local_sub_table_t t;
    local_sub_table_init(&t);

    const subscription_entry_t first = entry(1u);
    local_sub_table_insert_result_t a = local_sub_table_insert(&t, &first);
    CHECK(a.ok, "first insert must succeed");
    const local_sub_table_handle_t stale = a.handle;

    CHECK(local_sub_table_remove(&t, a.handle), "removing the first element must succeed");

    const subscription_entry_t second = entry(2u);
    local_sub_table_insert_result_t b = local_sub_table_insert(&t, &second);
    CHECK(b.ok, "second insert must succeed");
    CHECK(local_sub_table_handle_slot(b.handle) == local_sub_table_handle_slot(stale),
          "the second insert must reuse the freed slot — otherwise this test proves nothing");
    CHECK(local_sub_table_handle_generation(b.handle) != local_sub_table_handle_generation(stale),
          "reusing a slot must hand out a different generation");

    CHECK(local_sub_table_get(&t, stale) == NULL, "a stale handle must not resolve to the element that replaced it");
    CHECK(!local_sub_table_remove(&t, stale), "a stale handle must not remove the new occupant");
    CHECK(local_sub_table_len(&t) == 1u, "the rejected remove must leave the new element in place");

    const subscription_entry_t *live = local_sub_table_get(&t, b.handle);
    CHECK(live != NULL && live->callback_id == 2u, "the live handle must still resolve to its own element");
}

/// `<sce:on-overflow>reject</sce:on-overflow>`: a full table refuses
/// rather than evicting.
static void overflow_is_rejected(void) {
    local_sub_table_t t;
    local_sub_table_init(&t);

    for (uint32_t i = 0u; i < LOCAL_SUB_TABLE_CAPACITY; ++i) {
        const subscription_entry_t e = entry(i);
        local_sub_table_insert_result_t r = local_sub_table_insert(&t, &e);
        CHECK(r.ok, "every insert up to capacity must succeed");
    }
    CHECK(local_sub_table_len(&t) == LOCAL_SUB_TABLE_CAPACITY, "the table must fill to capacity");

    const subscription_entry_t overflow = entry(999u);
    local_sub_table_insert_result_t r = local_sub_table_insert(&t, &overflow);
    CHECK(!r.ok, "inserting past capacity must be rejected under `reject` overflow policy");
    CHECK(local_sub_table_len(&t) == LOCAL_SUB_TABLE_CAPACITY, "a rejected insert must not change len");
}

static uint32_t seen_count;
static uint32_t seen_sum;

static void tally(const subscription_entry_t *e, void *ctx) {
    (void)ctx;
    ++seen_count;
    seen_sum += e->callback_id;
}

/// `foreach` visits exactly the live elements, skipping freed slots.
static void foreach_visits_live_elements_only(void) {
    local_sub_table_t t;
    local_sub_table_init(&t);

    local_sub_table_handle_t to_remove;
    for (uint32_t i = 1u; i <= 4u; ++i) {
        const subscription_entry_t e = entry(i);
        local_sub_table_insert_result_t r = local_sub_table_insert(&t, &e);
        CHECK(r.ok, "setup inserts must succeed");
        if (i == 2u) {
            to_remove = r.handle;
        }
    }
    CHECK(local_sub_table_remove(&t, to_remove), "removing the middle element must succeed");

    seen_count = 0u;
    seen_sum = 0u;
    local_sub_table_foreach(&t, tally, NULL);
    CHECK(seen_count == 3u, "foreach must visit exactly the live elements");
    CHECK(seen_sum == 1u + 3u + 4u, "foreach must skip the freed slot rather than visit stale bytes");
}

int main(void) {
    insert_get_remove_round_trip();
    stale_handle_to_a_reused_slot_is_rejected();
    overflow_is_rejected();
    foreach_visits_live_elements_only();

    if (failures != 0) {
        fprintf(stderr,
                "forge_bounded_collection: %d assertion(s) failed — the generated collection "
                "does not match RFC §synth-5-D. An emit-shape grep over this header would "
                "still be green.\n",
                failures);
        return 1;
    }
    return 0;
}
