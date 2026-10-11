// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/* THREADS: 2 */
/* REQUIRED: acquire release acquire_release all */

// What this harness catches, measured 2026-10-11 on GenMC v0.19.0 (RC11): the
// baseline explores 9 complete executions with no error, and every acquire
// weakened to relaxed (`acquire`), every release (`release`), the pair
// (`acquire_release`) and every ordering (`all`) are each reported as a
// non-atomic race on the slot: the ring's only hand-overs are a release store
// read by an acquire load, so weakening either end leaves the consumer's read and
// the producer's write of the slot unordered. The acquire-release orderings of a
// read-modify-write (`acq_rel`) and the sequentially consistent ones (`seq_cst`)
// survive: the ring has none on a hand-over, only on claiming its one producer
// and one consumer place. They are not listed as required, and the gate prints
// each one that survives.

// Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
// §synth-5-P): the C++ Lamport ring (the `bounded` row for one producer and one
// consumer) under GenMC, with a slot that is reused. It is the C harness of the
// same name (backends/c/forge-runtime/tests/genmc/spsc_slot_reuse.c) over the C++
// runtime as it ships.
//
// A ring of one slot holds one element at a time. The producer pushes two
// elements, so the second can only go into the slot the first was popped from.
// Two hand-overs carry the elements and each is one release store read by one
// acquire load: the producer's store of `tail` publishes a filled slot to the
// consumer, and the consumer's store of `head` hands the slot back, so that the
// consumer's read of the first element happens-before the producer's write of
// the second. A hand-over that is too weak lets the consumer read a slot the
// producer is already overwriting, which GenMC reports as a race.
//
// Every element the consumer is given must be exactly one of the two written,
// whole, and in the order written. A full ring refuses the second push and an
// empty one refuses a pop; the harness tries each once and judges only what was
// delivered.

#include <cassert>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <utility>

#include <pthread.h>

#include <sce/forge/queue.h>

#include "genmc_cxx_support.h"

namespace {

struct Element {
    std::uint64_t a;
    std::uint64_t b;
};

SCE::Forge::Queue::Spsc<Element, 1> g_queue;

Element g_got[2];
unsigned g_got_count = 0;

void *produce(void *) {
    auto producer = g_queue.producer();
    if (!producer) {
        return nullptr;
    }
    for (std::uint64_t i = 1; i <= 2; i++) {
        Element e{i, i};
        // A full ring refuses the push; the producer tries the next element, so
        // the consumer may be given either, or both.
        (void)producer->try_push(std::move(e));
    }
    return nullptr;
}

void *consume(void *) {
    auto consumer = g_queue.consumer();
    if (!consumer) {
        return nullptr;
    }
    for (unsigned i = 0; i < 2; i++) {
        if (auto element = consumer->try_pop()) {
            g_got[g_got_count++] = *element;
        }
    }
    return nullptr;
}

}  // namespace

int main() {
    pthread_t producer;
    pthread_t consumer;
    pthread_create(&producer, nullptr, produce, nullptr);
    pthread_create(&consumer, nullptr, consume, nullptr);
    pthread_join(producer, nullptr);
    pthread_join(consumer, nullptr);

    // Whole elements, in the order written.
    for (unsigned i = 0; i < g_got_count; i++) {
        assert(g_got[i].a == g_got[i].b);
        assert(g_got[i].a == 1u || g_got[i].a == 2u);
        if (i > 0) {
            assert(g_got[i - 1u].a < g_got[i].a);
        }
    }
    return 0;
}
