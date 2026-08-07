// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RFC §synth-5-E — forge buffer-pool C11 output, exercised rather than
// read.
//
// Until this test the pool header's only verification was a text grep
// over the emitted source (`c6_bounded_collection_go_python_c11_emit.rs`
// says so in its own header: "emit-shape grep ... is cheaper than
// standing up gcc"). A byte golden proves the generator is
// deterministic; it does not prove the result compiles, and compiling
// does not prove the slot FSM works. Measured before this existed: the
// pool header had no compile target anywhere in the tree, and
// `bounded_collection` was compiled only incidentally, via a keyexpr
// fixture that happens to emit one.
//
// So this asserts the semantics the RFC specifies, not the presence of
// symbols:
//
//   free_count tracks acquisition and return
//   the slot FSM transitions free → cpu-mut → free
//   bytes written through slot_write are the bytes slot_read returns
//   exhaustion returns INVALID_HANDLE rather than aliasing a live slot
//   a tag mismatch returns false / NULL and leaves state untouched
//
// The last one runs with `SCE_DEBUG_OWNERSHIP` off, which is the
// default: the trap macro is inert there, so the *return-value*
// contract is what is under test. That contract is the release
// behaviour every consumer sees. Asserting the trap instead would need
// a build that aborts on purpose, and a test that overrides the trap
// could never catch a defect in the trap's own default.
//
// Fixture: tests/forge/resources/buffer_pool_ast_export_min.scxml
// (8 slots x 256 bytes, section `sram1`, alignment 32).

#include <stdio.h>
#include <string.h>

#include "buffer_pool_ast_export_min.h"

#define POOL_SLOTS BUFFER_POOL_AST_EXPORT_MIN_SLOT_COUNT
#define POOL_SLOT_BYTES BUFFER_POOL_AST_EXPORT_MIN_SLOT_SIZE

static int failures = 0;

#define CHECK(cond, msg)                                                                                               \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            fprintf(stderr, "forge_buffer_pool: FAIL — %s\n  (%s, line %d)\n", (msg), #cond, __LINE__);                \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

/// free → cpu-mut → free, with `free_count` following each step.
static void acquire_return_round_trip(void) {
    CHECK(buffer_pool_ast_export_min_free_count() == POOL_SLOTS, "a fresh pool must report every slot free");

    sce_slot_handle_t h = buffer_pool_ast_export_min_pool_acquire_for_encode();
    CHECK(h.state == SCE_SLOT_CPU_MUT, "acquire_for_encode must hand back a CPU-mutable handle");
    CHECK(h.idx < POOL_SLOTS, "the handle must index a real slot");
    CHECK(buffer_pool_ast_export_min_free_count() == POOL_SLOTS - 1u,
          "acquiring one slot must drop free_count by exactly one");
    CHECK(buffer_pool_ast_export_min_slot_state(h.idx) == SCE_SLOT_CPU_MUT,
          "the pool's own state table must agree with the handle tag");

    CHECK(buffer_pool_ast_export_min_pool_return(&h), "returning a CPU-owned slot must succeed");
    CHECK(h.state == SCE_SLOT_INVALID, "a returned handle must be invalidated in place");
    CHECK(buffer_pool_ast_export_min_free_count() == POOL_SLOTS, "returning the slot must restore free_count");
}

/// Bytes written through the pool come back byte-identical, and land in
/// the slot the handle names rather than a neighbour.
static void payload_survives_the_round_trip(void) {
    sce_slot_handle_t a = buffer_pool_ast_export_min_pool_acquire_for_encode();
    sce_slot_handle_t b = buffer_pool_ast_export_min_pool_acquire_for_encode();
    CHECK(a.idx != b.idx, "two live handles must not name the same slot");

    uint8_t *wa = buffer_pool_ast_export_min_slot_write(&a);
    uint8_t *wb = buffer_pool_ast_export_min_slot_write(&b);
    CHECK(wa != NULL && wb != NULL, "slot_write must expose CPU-owned slots");
    if (wa != NULL && wb != NULL) {
        memset(wa, 0xA5, POOL_SLOT_BYTES);
        memset(wb, 0x5A, POOL_SLOT_BYTES);

        const uint8_t *ra = buffer_pool_ast_export_min_slot_read(&a);
        const uint8_t *rb = buffer_pool_ast_export_min_slot_read(&b);
        CHECK(ra != NULL && rb != NULL, "slot_read must expose CPU-owned slots");
        if (ra != NULL && rb != NULL) {
            CHECK(ra[0] == 0xA5 && ra[POOL_SLOT_BYTES - 1u] == 0xA5,
                  "the first slot must read back what was written to it, edge to edge");
            CHECK(rb[0] == 0x5A && rb[POOL_SLOT_BYTES - 1u] == 0x5A,
                  "the second slot must read back its own bytes, not its neighbour's");
        }
    }

    CHECK(buffer_pool_ast_export_min_pool_return(&a), "returning the first slot must succeed");
    CHECK(buffer_pool_ast_export_min_pool_return(&b), "returning the second slot must succeed");
    CHECK(buffer_pool_ast_export_min_free_count() == POOL_SLOTS, "both slots must come back");
}

/// A full pool hands out INVALID_HANDLE rather than aliasing a slot
/// that is already owned.
static void exhaustion_yields_invalid_handle(void) {
    sce_slot_handle_t held[POOL_SLOTS];
    for (size_t i = 0; i < POOL_SLOTS; ++i) {
        held[i] = buffer_pool_ast_export_min_pool_acquire_for_encode();
        CHECK(held[i].state == SCE_SLOT_CPU_MUT, "every slot up to capacity must be acquirable");
    }
    CHECK(buffer_pool_ast_export_min_free_count() == 0u, "a fully acquired pool has no free slots");

    sce_slot_handle_t overflow = buffer_pool_ast_export_min_pool_acquire_for_encode();
    CHECK(overflow.state == SCE_SLOT_INVALID, "acquiring past capacity must fail rather than alias a live slot");

    for (size_t i = 0; i < POOL_SLOTS; ++i) {
        CHECK(buffer_pool_ast_export_min_pool_return(&held[i]), "each held slot must return cleanly");
    }
    CHECK(buffer_pool_ast_export_min_free_count() == POOL_SLOTS, "the pool must drain back to empty");
}

/// A returned handle is rejected, and the rejection leaves pool state
/// alone.
///
/// This exercises the *index* guard: `pool_return` sets `idx` to
/// `SIZE_MAX` on the way out, so the out-of-range check is what fires
/// here. The state-tag guard is a separate line and needs a separate
/// shape — see `stale_and_forged_handles_are_rejected`. Measured:
/// deleting the tag check alone leaves this function green, because the
/// index check catches the same call.
static void returned_handle_is_rejected_without_side_effects(void) {
    sce_slot_handle_t h = buffer_pool_ast_export_min_pool_acquire_for_encode();
    CHECK(buffer_pool_ast_export_min_pool_return(&h), "first return must succeed");

    const size_t free_after_return = buffer_pool_ast_export_min_free_count();

    CHECK(!buffer_pool_ast_export_min_pool_return(&h), "returning an already-returned handle must fail");
    CHECK(buffer_pool_ast_export_min_free_count() == free_after_return, "a rejected return must not change free_count");

    CHECK(buffer_pool_ast_export_min_slot_read(&h) == NULL, "reading a returned slot must yield NULL");
    CHECK(buffer_pool_ast_export_min_slot_write(&h) == NULL, "writing a returned slot must yield NULL");
}

/// The state-tag guards, reached with handles whose index is perfectly
/// valid so the range check cannot stand in for them.
///
/// Two shapes the RFC's FSM has to reject:
///
///   a *stale* copy — taken while the slot was owned, still naming a
///     real slot with a plausible tag, but the pool has since freed it.
///     `pool_return` compares its own state table against the handle
///     and must refuse.
///
///   a *forged* handle in a non-CPU state. `SCE_SLOT_DMA_BUSY_RX` names
///     a real slot the peripheral owns; exposing its bytes to author
///     code is the contract violation `slot_read` / `slot_write` exist
///     to prevent.
static void stale_and_forged_handles_are_rejected(void) {
    sce_slot_handle_t live = buffer_pool_ast_export_min_pool_acquire_for_encode();
    const sce_slot_handle_t stale = live; /* copy taken while owned */
    CHECK(buffer_pool_ast_export_min_pool_return(&live), "the live handle must return cleanly");

    const size_t free_before = buffer_pool_ast_export_min_free_count();
    sce_slot_handle_t stale_copy = stale;
    CHECK(!buffer_pool_ast_export_min_pool_return(&stale_copy),
          "a stale handle naming a now-free slot must be refused — its index is valid, so "
          "only the state-tag comparison can catch it");
    CHECK(buffer_pool_ast_export_min_free_count() == free_before, "refusing a stale handle must not change free_count");

    sce_slot_handle_t forged;
    forged.state = SCE_SLOT_DMA_BUSY_RX;
    forged.idx = 0u;
    CHECK(buffer_pool_ast_export_min_slot_read(&forged) == NULL,
          "a DMA-owned slot must not be readable by author code");
    CHECK(buffer_pool_ast_export_min_slot_write(&forged) == NULL,
          "a DMA-owned slot must not be writable by author code");
}

/// `pool_return` refuses a slot the peripheral owns.
///
/// The one shape that reaches `pool_return`'s state-tag check and
/// nothing else: a genuinely DMA-armed slot, so the handle's index is
/// valid *and* the pool's state table agrees with the handle. The range
/// check and the table comparison both pass; only "is this CPU-owned?"
/// can reject it. Measured — without this case, deleting that check
/// leaves every other assertion in this file green.
///
/// Runs last: the armed slot has no CPU-side path back to free, so it
/// stays out of the pool for the rest of the process.
static void dma_armed_slot_cannot_be_returned(void) {
    const size_t free_before = buffer_pool_ast_export_min_free_count();

    sce_slot_handle_t armed = buffer_pool_ast_export_min_link_arm_rx();
    CHECK(armed.state == SCE_SLOT_DMA_ARMED_RX, "link_arm_rx must arm a slot for peripheral RX");
    CHECK(buffer_pool_ast_export_min_slot_state(armed.idx) == SCE_SLOT_DMA_ARMED_RX,
          "the pool's state table must agree, so the table comparison cannot be what rejects");

    sce_slot_handle_t attempt = armed;
    CHECK(!buffer_pool_ast_export_min_pool_return(&attempt),
          "returning a DMA-armed slot must fail — the CPU does not own it");
    CHECK(buffer_pool_ast_export_min_slot_state(armed.idx) == SCE_SLOT_DMA_ARMED_RX,
          "a refused return must leave the slot armed");
    CHECK(buffer_pool_ast_export_min_free_count() == free_before - 1u,
          "a refused return must not hand the slot back to the free pool");
}

int main(void) {
    acquire_return_round_trip();
    payload_survives_the_round_trip();
    exhaustion_yields_invalid_handle();
    returned_handle_is_rejected_without_side_effects();
    stale_and_forged_handles_are_rejected();
    dma_armed_slot_cannot_be_returned();

    if (failures != 0) {
        fprintf(stderr,
                "forge_buffer_pool: %d assertion(s) failed — the generated pool's slot FSM "
                "does not match RFC §synth-5-E. A byte golden over this header would still "
                "be green.\n",
                failures);
        return 1;
    }
    return 0;
}
