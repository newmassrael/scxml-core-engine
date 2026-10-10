// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The C11 queue headers, consumed as C++ (SCE Protocol-Synthesis RFC §synth-5-P).
//
// Every generated C header opens `extern "C"` because a C++ consumer includes it,
// and the runtime headers they include (sce/forge/queue.h, queue_irq.h,
// atomics.h) are written to be valid C++ as well. `_Static_assert`, designated
// initialisers, a `void *` converted without a cast: any of them would turn the
// header into a parse error on one of the two languages, and no C compiler would
// say so. This file includes all five lowerings and runs each once.

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <new>

#include "host_irq.h"

#include "queue_c_intrusive.h"
#include "queue_c_intrusive_irq.h"
#include "queue_c_irq_many.h"
#include "queue_c_irq_one.h"
#include "queue_c_scq32.h"
#include "queue_c_scq64.h"
#include "queue_c_segmented.h"
#include "queue_c_segmented_many.h"
#include "queue_c_spsc.h"

namespace {

// The segmented queues take their segments from an allocator the caller injects,
// a struct of two function pointers and a context.
void *systemAllocate(void *, std::size_t size, std::size_t align) {
    return ::operator new(size, std::align_val_t(align), std::nothrow);
}

void systemDeallocate(void *, void *block, std::size_t, std::size_t align) {
    ::operator delete(block, std::align_val_t(align));
}

sce_queue_allocator_t systemAllocator() {
    sce_queue_allocator_t allocator{};
    allocator.context = nullptr;
    allocator.allocate = systemAllocate;
    allocator.deallocate = systemDeallocate;
    allocator.progress = SCE_QUEUE_PROGRESS_LOCK_FREE;
    return allocator;
}

queue_c_segmented_t g_linked;
queue_c_segmented_many_domain_t g_many_domain;
queue_c_segmented_many_t g_many;

// One event through each segmented queue, across a segment boundary.
int segmentedRoundTrip() {
    const sce_queue_allocator_t allocator = systemAllocator();
    int failures = 0;
    queue_conformance_event_t in{};
    queue_conformance_event_t out{};
    std::uint32_t producer = 0;
    std::uint32_t consumer = 0;
    if (!queue_c_segmented_init(&g_linked, &allocator) || !queue_c_segmented_producer_acquire(&g_linked) ||
        !queue_c_segmented_consumer_acquire(&g_linked)) {
        return 1;
    }
    for (std::uint16_t i = 0; i < 5; ++i) {
        in.sensor_id = 7;
        in.value = i;
        failures += queue_c_segmented_try_push(&g_linked, &in) == SCE_QUEUE_PUSH_OK ? 0 : 1;
    }
    for (std::uint16_t i = 0; i < 5; ++i) {
        failures += (queue_c_segmented_try_pop(&g_linked, &out) && out.sensor_id == 7 && out.value == i) ? 0 : 1;
    }
    queue_c_segmented_producer_release(&g_linked);
    queue_c_segmented_consumer_release(&g_linked);
    queue_c_segmented_destroy(&g_linked);

    if (!queue_c_segmented_many_domain_init(&g_many_domain) ||
        !queue_c_segmented_many_init(&g_many, &g_many_domain, &allocator) ||
        !queue_c_segmented_many_producer_acquire(&g_many, &producer) ||
        !queue_c_segmented_many_consumer_acquire(&g_many, &consumer)) {
        return failures + 1;
    }
    for (std::uint16_t i = 0; i < 5; ++i) {
        in.sensor_id = 8;
        in.value = i;
        failures += queue_c_segmented_many_try_push(&g_many, producer, &in) == SCE_QUEUE_PUSH_OK ? 0 : 1;
    }
    for (std::uint16_t i = 0; i < 5; ++i) {
        failures +=
            (queue_c_segmented_many_try_pop(&g_many, consumer, &out) && out.sensor_id == 8 && out.value == i) ? 0 : 1;
    }
    queue_c_segmented_many_producer_release(&g_many, producer);
    queue_c_segmented_many_consumer_release(&g_many, consumer);
    queue_c_segmented_many_destroy(&g_many);
    return failures;
}

template <typename Queue, typename TryPush, typename TryPop, typename Acquire>
int roundTrip(Queue &queue, TryPush tryPush, TryPop tryPop, Acquire acquireProducer, Acquire acquireConsumer) {
    queue_conformance_event_t in{};
    queue_conformance_event_t out{};
    in.sensor_id = 7;
    in.value = 1234;
    if (!acquireProducer(&queue) || !acquireConsumer(&queue)) {
        return 1;
    }
    if (tryPush(&queue, &in) != SCE_QUEUE_PUSH_OK) {
        return 1;
    }
    if (!tryPop(&queue, &out) || out.sensor_id != 7 || out.value != 1234) {
        return 1;
    }
    return tryPop(&queue, &out) ? 1 : 0;
}

// The intrusive queues name nodes by index in an array the caller owns.
template <typename Queue, typename Init, typename Push, typename TryPop>
int nodeRoundTrip(Queue &queue, Init init, Push push, TryPop tryPop) {
    queue_conformance_node_t nodes[3]{};
    std::uint32_t index = 0;
    if (!init(&queue, nodes, 3u, 2u)) {
        return 1;
    }
    push(&queue, 1u);
    push(&queue, 0u);
    if (!tryPop(&queue, &index) || index != 1u || !tryPop(&queue, &index) || index != 0u) {
        return 1;
    }
    return tryPop(&queue, &index) ? 1 : 0;
}

queue_c_intrusive_t g_intrusive;
queue_c_intrusive_irq_t g_intrusive_irq;
queue_c_spsc_t g_spsc;
queue_c_scq64_t g_scq64;
queue_c_scq32_t g_scq32;
queue_c_irq_one_t g_irq_one;
queue_c_irq_many_t g_irq_many;

}  // namespace

int main() {
    int failures = 0;
    if (!queue_c_spsc_init(&g_spsc) || !queue_c_scq64_init(&g_scq64) || !queue_c_scq32_init(&g_scq32) ||
        !queue_c_irq_one_init(&g_irq_one) || !queue_c_irq_many_init(&g_irq_many)) {
        return 1;
    }
    failures += roundTrip(g_spsc, queue_c_spsc_try_push, queue_c_spsc_try_pop, queue_c_spsc_producer_acquire,
                          queue_c_spsc_consumer_acquire);
    failures += roundTrip(g_scq64, queue_c_scq64_try_push, queue_c_scq64_try_pop, queue_c_scq64_producer_acquire,
                          queue_c_scq64_consumer_acquire);
    failures += roundTrip(g_scq32, queue_c_scq32_try_push, queue_c_scq32_try_pop, queue_c_scq32_producer_acquire,
                          queue_c_scq32_consumer_acquire);
    failures += roundTrip(g_irq_one, queue_c_irq_one_try_push, queue_c_irq_one_try_pop,
                          queue_c_irq_one_producer_acquire, queue_c_irq_one_consumer_acquire);
    failures += roundTrip(g_irq_many, queue_c_irq_many_try_push, queue_c_irq_many_try_pop,
                          queue_c_irq_many_producer_acquire, queue_c_irq_many_consumer_acquire);
    failures += segmentedRoundTrip();
    failures += nodeRoundTrip(g_intrusive, queue_c_intrusive_init, queue_c_intrusive_push, queue_c_intrusive_try_pop);
    failures += nodeRoundTrip(g_intrusive_irq, queue_c_intrusive_irq_init, queue_c_intrusive_irq_push,
                              queue_c_intrusive_irq_try_pop);
    std::printf("generated queues (C11 headers as C++): %d failure(s)\n", failures);
    return failures == 0 ? 0 : 1;
}
