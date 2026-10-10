// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh EventQueueBridge runtime verification.
//
// EventQueueBridge is the forge runtime's SCQ queue for many producers and one
// consumer. test_mesh_local.cpp only checks that it compiles; this test runs
// it: the capacity it declares, the places it hands out, first-in first-out
// order, and that real threads lose nothing and reorder nothing between them,
// which is the property the Vyukov ring it replaced did not formally have.

#include "mesh/EventQueueBridge.h"

#include <atomic>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <thread>
#include <vector>

namespace {

int g_failures = 0;

void check(bool ok, const char *what) {
    if (!ok) {
        std::fprintf(stderr, "FAIL: %s\n", what);
        ++g_failures;
    }
}

struct Event {
    std::uint32_t producer;
    std::uint32_t sequence;
};

using Bridge = SCE::Mesh::EventQueueBridge<Event, 64>;
using SCE::Forge::Queue::PushStatus;

void exactly_the_capacity_fits_in_order() {
    Bridge bridge;
    auto producer = bridge.producer();
    auto consumer = bridge.consumer();
    check(producer.has_value() && consumer.has_value(), "a fresh bridge hands out a handle a side");
    if (!producer || !consumer) {
        return;
    }
    check(!consumer->try_pop().has_value(), "a new bridge is empty");
    for (std::uint32_t i = 0; i < 64; ++i) {
        check(producer->try_push(Event{0, i}) == PushStatus::Ok, "a push below capacity is taken");
    }
    check(producer->try_push(Event{0, 99}) == PushStatus::Full, "a full bridge refuses the push");
    for (std::uint32_t i = 0; i < 64; ++i) {
        const auto popped = consumer->try_pop();
        check(popped.has_value() && popped->sequence == i, "events come out in the order they went in");
    }
    check(!consumer->try_pop().has_value(), "a drained bridge is empty");
    // The slots are reusable: a second lap through the ring.
    for (std::uint32_t i = 0; i < 64; ++i) {
        check(producer->try_push(Event{1, i}) == PushStatus::Ok, "the ring is reusable after a drain");
    }
    for (std::uint32_t i = 0; i < 64; ++i) {
        const auto popped = consumer->try_pop();
        check(popped.has_value() && popped->producer == 1 && popped->sequence == i, "second lap keeps order");
    }
}

void a_side_has_as_many_places_as_the_ring_has_slots() {
    Bridge bridge;
    std::vector<Bridge::Producer> producers;
    for (std::size_t i = 0; i < 64; ++i) {
        auto handle = bridge.producer();
        check(handle.has_value(), "a producer place is free below the ring size");
        if (!handle) {
            return;
        }
        producers.push_back(std::move(*handle));
    }
    check(!bridge.producer().has_value(), "a producer past the ring size is refused");
    producers.pop_back();
    check(bridge.producer().has_value(), "a released place is free again");
}

void producers_on_threads_lose_nothing_and_reorder_nothing() {
    constexpr std::uint32_t kProducers = 4;
    constexpr std::uint32_t kPerProducer = 20000;
    Bridge bridge;
    std::atomic<std::uint64_t> pushed{0};
    std::vector<std::thread> threads;
    threads.reserve(kProducers);
    for (std::uint32_t p = 0; p < kProducers; ++p) {
        threads.emplace_back([&bridge, &pushed, p] {
            auto producer = bridge.producer();
            if (!producer) {
                std::fprintf(stderr, "FAIL: a producer thread got no place\n");
                std::abort();
            }
            for (std::uint32_t i = 0; i < kPerProducer; ++i) {
                while (producer->try_push(Event{p, i}) != PushStatus::Ok) {
                    std::this_thread::yield();
                }
                pushed.fetch_add(1, std::memory_order_relaxed);
            }
        });
    }

    // One consumer: the engine sees one event at a time. Each producer's events
    // reach it in the order they were pushed, and every event arrives once.
    auto consumer = bridge.consumer();
    check(consumer.has_value(), "the consumer place is free");
    std::vector<std::uint32_t> next(kProducers, 0);
    std::uint64_t received = 0;
    const std::uint64_t total = std::uint64_t{kProducers} * kPerProducer;
    while (consumer && received < total) {
        const auto popped = consumer->try_pop();
        if (!popped) {
            std::this_thread::yield();
            continue;
        }
        check(popped->producer < kProducers, "an event names a producer that exists");
        if (popped->producer < kProducers) {
            check(popped->sequence == next[popped->producer], "one producer's events keep their order");
            next[popped->producer] = popped->sequence + 1;
        }
        ++received;
    }
    for (auto &thread : threads) {
        thread.join();
    }
    check(received == total, "every event arrived");
    check(!consumer || !consumer->try_pop().has_value(), "nothing arrived twice");
    for (std::uint32_t p = 0; p < kProducers; ++p) {
        check(next[p] == kPerProducer, "every event of every producer arrived exactly once");
    }
}

}  // namespace

int main() {
    exactly_the_capacity_fits_in_order();
    a_side_has_as_many_places_as_the_ring_has_slots();
    producers_on_threads_lose_nothing_and_reorder_nothing();
    if (g_failures != 0) {
        std::fprintf(stderr, "EventQueueBridge: %d failure(s)\n", g_failures);
        return 1;
    }
    std::printf("EventQueueBridge: PASS\n");
    return 0;
}
