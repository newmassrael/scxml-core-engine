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

#include <cstdint>
#include <cstdio>

#include "host_irq.h"

#include "queue_c_irq_many.h"
#include "queue_c_irq_one.h"
#include "queue_c_scq32.h"
#include "queue_c_scq64.h"
#include "queue_c_spsc.h"

namespace {

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
    std::printf("generated queues (C11 headers as C++): %d failure(s)\n", failures);
    return failures == 0 ? 0 : 1;
}
