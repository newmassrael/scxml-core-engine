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
 * The contract is the lending type's, `<pool>_over_t`: every scenario makes
 * one over a span of the arena (docs/adr/0111 D4). The lending type is
 * declared in `<pool>_over.h`, which this test includes as a program that
 * lends does; the owner's `<pool>.h` includes none of it, and the shared
 * pool's includes it for the book its own storage is kept in. The pool over
 * the header's own storage runs none of the scenarios; each is held to the
 * calls of a pool before the book instead (`_run_owned`), and the owner's to
 * one state per slot.
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "pool_conformance_owner.h"
/* The header of a pool that is not shared includes nothing of its lending form. */
#ifdef SCE_FORGE_POOL_CONFORMANCE_OWNER_OVER_H
#error "pool_conformance_owner.h includes its lending form's header"
#endif
#include "pool_conformance_owner_over.h"

#include "pool_conformance_shared.h"
/* A shared pool's header includes its lending form's: its own storage is kept
 * in the lending type, whose words are atomic. */
#ifndef SCE_FORGE_POOL_CONFORMANCE_SHARED_OVER_H
#error "pool_conformance_shared.h does not include its lending form's header"
#endif
#include "pool_conformance_shared_over.h"

/* The owner's pool over its own storage keeps one state per slot, no book. */
SCE_STATIC_ASSERT(sizeof(pool_conformance_owner_slot_states) ==
                      POOL_CONFORMANCE_OWNER_SLOT_COUNT * sizeof(sce_slot_state_t),
                  "the pool over its own storage keeps one state per slot");

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
SCE_ALIGNAS(16) static uint8_t arena[272];

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

int main(void) {
    if (g_pool_scenario_count == 0u) {
        fprintf(stderr, "buffer_pool: a contract with no scenarios checks nothing\n");
        return 1;
    }
    pool_conformance_owner_run_owned();
    pool_conformance_shared_run_owned();
    for (size_t s = 0; s < g_pool_scenario_count; ++s) {
        pool_conformance_owner_run(&g_pool_scenarios[s]);
        pool_conformance_shared_run(&g_pool_scenarios[s]);
    }
    if (failures != 0) {
        fprintf(stderr, "buffer_pool: %d check(s) failed\n", failures);
        return 1;
    }
    printf("buffer_pool: %zu scenarios held for both lending types, and both own pools kept their calls\n",
           g_pool_scenario_count);
    return 0;
}
