// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/* THREADS: 2 */
/* REQUIRED: acquire release acquire_release all */

// What this harness catches, measured 2026-10-11 on GenMC v0.19.0 (RC11): the
// baseline explores 11 complete executions with no error (the C harness of the
// same name explores 11 too), and every acquire weakened to relaxed (`acquire`),
// every release (`release`), the pair (`acquire_release`) and every ordering
// (`all`) are each reported as an error: the consumer reads a segment or a slot
// whose write the producer's hand-over no longer orders before it. The
// acquire-release orderings of a read-modify-write (`acq_rel`) and the
// sequentially consistent ones (`seq_cst`) survive: the ring has none on a
// hand-over, only on claiming its one producer and one consumer place. They are
// not listed as required, and the gate prints each one that survives.

// Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
// §synth-5-P): the C++ linked Lamport rings (the `segmented` row for one producer
// and one consumer) under GenMC, across a segment boundary. It is the C harness of
// the same name (backends/c/forge-runtime/tests/genmc/linked_segment_handover.c)
// over the C++ runtime as it ships.
//
// The segment holds one element, so the producer's second push takes a segment
// from the allocator, links it behind the first and puts the element in it. Two
// kinds of hand-over carry the elements, each one release store read by one
// acquire load: the count of filled slots, which publishes an element to the
// consumer, and the link, which publishes the successor segment. The consumer
// also gives the first segment back to the allocator once it has read the link.
//
// The allocator is GenMC's own, which it models: a block is uninitialized until
// the runtime writes it, so a read the producer's write is not ordered before is
// an error GenMC reports.
//
// Every element the consumer is given must be exactly one of the two written,
// whole, and in the order written. A pop that finds nothing yet is not a failure;
// the harness tries three times and judges only what was delivered.

#include <cassert>
#include <cstddef>
#include <cstdint>
#include <new>
#include <optional>
#include <utility>

#include <pthread.h>

#include <sce/forge/queue.h>
#include <sce/forge/queue_segmented.h>

#include "genmc_cxx_support.h"

namespace {

using SCE::Forge::Queue::LinkedLamport;
using SCE::Forge::Queue::Progress;

struct Element {
    std::uint64_t a;
    std::uint64_t b;
};

// The allocator the queue is injected with: GenMC's allocation primitives, for
// the reason hazard_keeps_a_named_node.cpp gives (neither <cstdlib> nor a bare
// `malloc` reaches them from a C++ source). `__VERIFIER_malloc_aligned` takes the
// alignment first, as GenMC's own `aligned_alloc` passes it.
class HarnessAllocator {
public:
    static constexpr Progress kProgress = Progress::Blocking;

    void *allocate(std::size_t size, std::size_t align) noexcept {
        return __VERIFIER_malloc_aligned(align, size);
    }

    void deallocate(void *block, std::size_t, std::size_t) noexcept {
        __VERIFIER_free(block);
    }
};

using Queue = LinkedLamport<Element, 1, HarnessAllocator, Progress::Blocking>;

HarnessAllocator g_allocator;

// The queue is built in `main`, in storage of its own, and not as a global with a
// constructor: its first segment comes from the allocator, and a global's dynamic
// initialization is one more thing the interpreter has to run before `main`.
alignas(Queue) unsigned char g_storage[sizeof(Queue)];
Queue *g_queue = nullptr;

Element g_got[3];
unsigned g_got_count = 0;

void *produce(void *) {
    auto producer = g_queue->producer();
    if (!producer) {
        return nullptr;
    }
    for (std::uint64_t i = 1; i <= 2; i++) {
        Element e{i, i};
        (void)producer->try_push(std::move(e));
    }
    return nullptr;
}

void *consume(void *) {
    auto consumer = g_queue->consumer();
    if (!consumer) {
        return nullptr;
    }
    for (unsigned i = 0; i < 3; i++) {
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

    g_queue = ::new (static_cast<void *>(g_storage)) Queue(g_allocator);
    if (!g_queue->valid()) {
        return 1;
    }
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
    // Every segment still held goes back; GenMC reports a segment freed twice or
    // one the consumer already gave back.
    g_queue->~Queue();
    return 0;
}
