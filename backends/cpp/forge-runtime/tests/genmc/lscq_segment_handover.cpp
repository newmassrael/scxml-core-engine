// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/* THREADS: 2 */
/* CXX_STRONG_CAS */
/* REQUIRED: acq_rel all */

// What this harness catches, measured 2026-10-11 on GenMC v0.19.0 (RC11): the
// baseline explores 927 complete executions with no error, and the
// acquire-release orderings of the compare-and-swaps (`acq_rel`) and every
// ordering (`all`) are each reported as an error. It does NOT catch the acquires
// alone, the releases alone, the pair, or the sequentially consistent orderings:
// in this scenario the compare-and-swaps provide the hand-overs those would, and
// nothing here depends on the hazard handshake, exactly as the C harness of the
// same name finds (it catches `cas`, `rmw` and `all`, and nothing else). They are
// listed as surviving rather than required, because requiring them would be a
// claim this harness cannot back.

// Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
// §synth-5-P): the C++ LSCQ (the `segmented` row for any cardinality but one
// producer and one consumer) under GenMC, across a segment boundary with a
// retirement. It is the C harness of the same name
// (backends/c/forge-runtime/tests/genmc/lscq_segment_handover.c) over the C++
// runtime, with the one difference the next paragraph states.
//
// WHAT IT DOES NOT JUDGE. It reads a copy of the queue headers whose
// `compare_exchange_weak` is rewritten to `compare_exchange_strong`
// (`CXX_STRONG_CAS` above; scripts/gates/genmc-queue.sh says why and how). The
// shipped runtime retries a weak compare-and-swap in the SCQ's `catch_up`, and
// GenMC makes every spurious failure of it an execution: the single-threaded start
// of this scenario, with no concurrency at all, is KILLed at the unroll bound and
// GenMC stops with an internal failure (measured 2026-10-11). With the copy the
// whole scenario is explored, and what is not judged is a spurious failure, which
// a strong compare-and-swap never has and every retry loop here tolerates. The
// orderings checked are the ones the runtime writes.
//
// The main thread pushes two elements before the threads start. The segment holds
// one element, so the second push closes the first segment and links a second
// behind it holding the element. Then one thread pushes a third element, into the
// second segment, while the other pops: it takes the first element, finds the
// first segment closed and empty, moves `head` past it, retires it and runs the
// scan that frees what no hazard names.
//
// What it does not exercise is the hazard handshake, for the reason the C harness
// gives: the producer is never inside the segment the consumer retires, and a
// producer inside one another participant retires needs a second producer, which
// did not finish under GenMC. The handshake is checked on the domain by itself,
// in hazard_keeps_a_named_node.cpp.
//
// Every element the consumer is given must be exactly one of the three written,
// whole, and in the order written.
//
// What this does and does not establish about the C++ LSCQ is also what the
// header of genmc_cxx_support.h records: the part of it that the first attempt
// could not judge is now judged, up to the compare-and-swap strengthening above.

#include <cassert>
#include <cstddef>
#include <cstdint>
#include <new>
#include <optional>
#include <utility>

#include <pthread.h>

// A dequeue that finds an entry empty looks again this many times before it makes
// the entry unusable (10000 by default). GenMC explores every iteration of a loop
// up to its unroll bound and then kills the execution, so the retirement of a
// segment, which walks the closed ring's empty entries, never finished with the
// default: the main thread was killed before it created a thread, and the run that
// reported no error described a program with no concurrency in it (measured
// 2026-10-11 on the C runtime, which has the same loop, and again on this one).
// One look models both outcomes of the loop, an entry that is filled meanwhile and
// one that is not.
#define SCE_QUEUE_SCQ_SPINS 1u

#include <sce/forge/queue.h>
#include <sce/forge/queue_segmented.h>

#include "genmc_cxx_support.h"

namespace {

using SCE::Forge::Queue::HazardDomain;
using SCE::Forge::Queue::Lscq;
using SCE::Forge::Queue::Progress;

struct Element {
    std::uint64_t a;
    std::uint64_t b;
};

// The allocator the queue is injected with: GenMC's allocation primitives, for
// the reason hazard_keeps_a_named_node.cpp gives.
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

constexpr std::size_t kHazardSlots = 2;

using Queue = Lscq<Element, 1, 2, kHazardSlots, HarnessAllocator, Progress::Blocking>;

HarnessAllocator g_allocator;
HazardDomain<kHazardSlots> g_domain;

// The queue is built in `main`, in storage of its own, for the reason
// linked_segment_handover.cpp gives.
alignas(Queue) unsigned char g_storage[sizeof(Queue)];
Queue *g_queue = nullptr;

Element g_got[3];
unsigned g_got_count = 0;

void *produce(void *) {
    auto producer = g_queue->producer();
    if (!producer) {
        return nullptr;
    }
    Element e{3, 3};
    (void)producer->try_push(std::move(e));
    return nullptr;
}

void *consume(void *) {
    auto consumer = g_queue->consumer();
    if (!consumer) {
        return nullptr;
    }
    for (unsigned i = 0; i < 2; i++) {
        if (auto element = consumer->try_pop()) {
            g_got[g_got_count++] = *element;
        }
    }
    g_domain.collect();
    return nullptr;
}

}  // namespace

int main() {
    pthread_t producer;
    pthread_t consumer;

    g_queue = ::new (static_cast<void *>(g_storage)) Queue(g_allocator, g_domain);
    if (!g_queue->valid()) {
        return 1;
    }

    // The first two elements, pushed before the threads exist: the second closes
    // the first segment and links the second.
    {
        auto pusher = g_queue->producer();
        if (!pusher) {
            return 1;
        }
        for (std::uint64_t i = 1; i <= 2; i++) {
            Element e{i, i};
            (void)pusher->try_push(std::move(e));
        }
    }

    pthread_create(&producer, nullptr, produce, nullptr);
    pthread_create(&consumer, nullptr, consume, nullptr);
    pthread_join(producer, nullptr);
    pthread_join(consumer, nullptr);

    // Whole elements, in the order written.
    for (unsigned i = 0; i < g_got_count; i++) {
        assert(g_got[i].a == g_got[i].b);
        assert(g_got[i].a >= 1u && g_got[i].a <= 3u);
        if (i > 0) {
            assert(g_got[i - 1u].a < g_got[i].a);
        }
    }
    // Every segment still held goes back, and the retired ones with them: the
    // destructor flushes the domain.
    g_queue->~Queue();
    return 0;
}
