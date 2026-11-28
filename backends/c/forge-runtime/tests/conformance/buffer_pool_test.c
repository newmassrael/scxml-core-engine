/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The C11 arm of the buffer-pool kind's lending contract (docs/adr/0111 D4):
 * every scenario of tests/forge/conformance/buffer_pool/pool_contract.json,
 * written as C by CMakeLists.txt, runs against the owner's pool and the shared
 * one generated from the documents beside it. The Rust arm
 * (backends/rust/forge-conformance/tests/forge_buffer_pool.rs) runs the same
 * scenarios against the same documents.
 *
 * The runner is written once, in buffer_pool_runner.inc, and included once per
 * pool with POOL_PREFIX naming the pool's functions and types: the two pools
 * are distinct singletons with distinct names, and a runner of one cannot be
 * handed the other. The shared pool moves its book through the sce_atomic_*
 * intrinsics, which sce_atomic_host.c supplies on the host.
 *
 * The contract is a lending pool's. The pool the contract names in
 * `fixed_pools` lends every slot whole and runs none of it; it is held here to
 * the layout and the calls of a pool before the book (docs/adr/0111 D4).
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "pool_conformance_fixed.h"
#include "pool_conformance_owner.h"
#include "pool_conformance_shared.h"

/* The contract's pools keep the book of their lendings; the fixed one keeps
 * one state per slot and nothing else, no book and no generation. */
#if POOL_CONFORMANCE_OWNER_LENDS != 1 || POOL_CONFORMANCE_SHARED_LENDS != 1
#error "every pool the contract runs against lends at run time"
#endif
#if POOL_CONFORMANCE_FIXED_LENDS != 0
#error "the fixed pool lends every slot whole"
#endif
SCE_STATIC_ASSERT(sizeof(pool_conformance_fixed_slot_states) ==
                      POOL_CONFORMANCE_FIXED_SLOT_COUNT * sizeof(sce_slot_state_t),
                  "a pool that lends every slot whole keeps one state per slot");

enum {
    OP_COUNT,
    OP_SLOT_LEN,
    OP_FREE,
    OP_VIOLATIONS,
    OP_REFUSED_PREFIXES,
    OP_ACQUIRE,
    OP_RECEIVE,
    OP_CAPACITY,
    OP_WRITE,
    OP_READ,
    OP_ARM_TX,
    OP_RETURN,
    OP_INDEX,
    OP_INTO_INDEX,
    OP_RECLAIM,
    OP_GENERATION,
    OP_TAKE,
    OP_SENT,
    OP_COMPLETE_TX,
    OP_LEND,
    OP_HOLD,
    OP_RELEASE,
    OP_LENT
};

/* A lending's outcome, a take's, and the rest's, as the contract spells them. */
enum { LEND_OK = 0, LEND_EXHAUSTED = 1, LEND_LENGTH = 2, LEND_HEADROOM = 3 };

enum { TAKE_OK = 0, TAKE_STALE = 1, TAKE_PREFIX_TOO_WIDE = 2 };

enum { OUTCOME_OK = 0, OUTCOME_REFUSED = 1 };

/* One step: the registers it reads and writes, its numbers (a length, a
 * frame, holds, a state), the bytes it writes or expects, and its expected
 * outcome in the encoding of its op. */
typedef struct {
    int op;
    int slot;
    int index;
    int out;
    long a;
    long b;
    long expect;
    size_t nbytes;
    uint8_t bytes[8];
} pool_step_t;

/* One scenario: how its pool is made, whether that is expected to succeed,
 * and its steps. */
typedef struct {
    const char *id;
    int over;
    size_t count;
    size_t span;
    size_t offset;
    size_t len;
    int make_ok;
    const pool_step_t *steps;
    size_t step_count;
} pool_scenario_t;

#include "pool_contract_scenarios.inc"

static int failures = 0;

#define CHECK(cond, id, step)                                                                                          \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            fprintf(stderr, "buffer_pool: FAIL %s step %zu: %s (line %d)\n", (id), (size_t)(step), #cond, __LINE__);   \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

/* Storage a caller holds, on the documents' 16-byte alignment, large enough
 * for every span a scenario cuts and the offset it starts at. */
static SCE_ALIGNAS(16) uint8_t arena[272];

#define POOL_CAT2(a, b) a##b
#define POOL_CAT(a, b) POOL_CAT2(a, b)

#define POOL_PREFIX pool_conformance_owner_
#define POOL_UPPER POOL_CONFORMANCE_OWNER_
#include "buffer_pool_runner.inc"
#undef POOL_PREFIX
#undef POOL_UPPER

#define POOL_PREFIX pool_conformance_shared_
#define POOL_UPPER POOL_CONFORMANCE_SHARED_
#include "buffer_pool_runner.inc"
#undef POOL_PREFIX
#undef POOL_UPPER

/* docs/adr/0111 D4: the pool that lends every slot whole keeps the calls it
 * had before the book: a lending names no length, a frame is armed whole, and
 * the seam is keyed by the slot's position. */
static void fixed_pool_keeps_the_calls_before_the_book(void) {
    const char *id = "fixed_pool_keeps_the_calls_before_the_book";
    sce_slot_handle_t h = pool_conformance_fixed_pool_acquire_for_encode();
    uint8_t *w;
    size_t ti = h.idx;
    sce_slot_handle_t r;
    size_t ri;
    sce_slot_handle_t done;

    CHECK(h.state == SCE_SLOT_CPU_MUT, id, 0u);
    w = pool_conformance_fixed_slot_write(&h);
    CHECK(w != NULL, id, 1u);
    if (w != NULL) {
        w[0] = 0xA1;
        w[POOL_CONFORMANCE_FIXED_SLOT_SIZE - 1u] = 0xA2;
    }
    CHECK(pool_conformance_fixed_link_arm_tx(&h), id, 2u);
    CHECK(pool_conformance_fixed_dma_start_tx(ti), id, 3u);
    CHECK(pool_conformance_fixed_tx_complete(ti), id, 4u);

    r = pool_conformance_fixed_link_arm_rx();
    CHECK(r.state == SCE_SLOT_DMA_ARMED_RX, id, 5u);
    ri = r.idx;
    CHECK(pool_conformance_fixed_dma_start_rx(&r), id, 6u);
    done = pool_conformance_fixed_rx_complete(ri);
    CHECK(done.state == SCE_SLOT_CPU_REF, id, 7u);
    CHECK(pool_conformance_fixed_pool_return(&done), id, 8u);
    CHECK(pool_conformance_fixed_free_count() == POOL_CONFORMANCE_FIXED_SLOT_COUNT, id, 9u);
}

int main(void) {
    fixed_pool_keeps_the_calls_before_the_book();
    if (g_pool_scenario_count == 0u) {
        fprintf(stderr, "buffer_pool: a contract with no scenarios checks nothing\n");
        return 1;
    }
    for (size_t s = 0; s < g_pool_scenario_count; ++s) {
        pool_conformance_owner_run(&g_pool_scenarios[s]);
        pool_conformance_shared_run(&g_pool_scenarios[s]);
    }
    if (failures != 0) {
        fprintf(stderr, "buffer_pool: %d check(s) failed\n", failures);
        return 1;
    }
    printf("buffer_pool: %zu scenarios held for both lending pools, and the fixed pool kept its calls\n",
           g_pool_scenario_count);
    return 0;
}
