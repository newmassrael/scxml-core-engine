// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The queue kind's generated header (SCE Protocol-Synthesis RFC §synth-5-P),
// compiled and run.
//
// sce-build's tests read what the generator writes for a queue and look for
// text in it, which shows the text is what the template says and nothing
// about whether the names in it are the names sce/forge/queue.h has. Here the
// header the generator writes for each bounded algorithm row is compiled into
// this test over the element codec it names, and the queue it defines is
// used: a rename in the runtime, or a constant the template computes wrongly,
// stops the build or fails a check. The runtime's own properties are
// queue_runtime_test's; this file holds what the generated module adds.
//
// The headers come from tests/forge/resources/queue_conformance_*.scxml,
// generated at build time into the same directory as the numerical
// conformance fixtures.

#include "queue_conformance_event.h"
#include "queue_conformance_scq.h"
#include "queue_conformance_spsc.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <map>
#include <optional>
#include <string_view>
#include <thread>
#include <utility>
#include <vector>

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                         \
        }                                                                                                              \
    } while (0)

namespace queue = ::SCE::Forge::Queue;
namespace element_ns = ::SCE::Generated::QueueConformanceEvent;
namespace spsc = ::SCE::Generated::QueueConformanceSpsc;
namespace scq = ::SCE::Generated::QueueConformanceScq;

using Event = element_ns::QueueConformanceEvent;

Event event(std::uint8_t sensor_id, std::uint16_t value) {
    return Event{.sensor_id = sensor_id, .value = value};
}

bool contains(std::string_view haystack, std::string_view needle) {
    return haystack.find(needle) != std::string_view::npos;
}

// What the generated headers state about their storage is checked when this
// file is compiled: a constant the template computes wrongly stops the build.
static_assert(spsc::STORAGE_BYTES >= spsc::CAPACITY * sizeof(Event), "the Lamport ring holds its capacity of elements");
static_assert((scq::RING_SLOTS & (scq::RING_SLOTS - 1)) == 0, "an SCQ ring is a power of two of slots");
static_assert(scq::RING_SLOTS >= scq::CAPACITY && scq::RING_SLOTS >= scq::PARTICIPANTS,
              "the ring is at least as large as the capacity and the participants");

// ─── The contract the module states ───

void the_lamport_ring_module_states_what_the_document_required_and_what_it_gives() {
    CHECK(spsc::CAPACITY == 4, "capacity is the document's");
    CHECK(spsc::DECLARED_PROGRESS == "wait-free", "declared progress");
    CHECK(spsc::PUSH_PROGRESS == "wait-free", "push progress");
    CHECK(spsc::POP_PROGRESS == "wait-free", "pop progress");
    CHECK(contains(spsc::ALGORITHM, "Lamport"), "one producer and one consumer select the Lamport ring");
    spsc::QueueConformanceSpsc q;
    CHECK(q.capacity() == spsc::CAPACITY, "the queue reports the module's capacity");
}

void the_scq_module_sizes_its_rings_from_capacity_and_participants() {
    CHECK(scq::CAPACITY == 6, "capacity is the document's");
    CHECK(scq::PARTICIPANTS == 3, "participants are the document's");
    CHECK(scq::RING_SLOTS == 8, "the next power of two at or above max(capacity, participants)");
    CHECK(scq::DECLARED_PROGRESS == "lock-free", "declared progress");
    CHECK(scq::PUSH_PROGRESS == "lock-free", "push progress");
    CHECK(scq::POP_PROGRESS == "lock-free", "pop progress");
    CHECK(contains(scq::ALGORITHM, "SCQ"), "many producers and consumers select SCQ");
    CHECK(scq::WRAP_BOUND_OPS == (std::uint64_t{1} << 62), "the wrap bound the template states is the runtime's");
    scq::QueueConformanceScq q;
    CHECK(q.capacity() == scq::CAPACITY, "the queue reports the module's capacity");
}

// ─── The queue it defines is the queue the table named ───

void the_lamport_ring_hands_elements_over_in_order_and_refuses_when_full() {
    spsc::QueueConformanceSpsc q;
    auto producer = q.producer();
    auto consumer = q.consumer();
    CHECK(producer.has_value() && consumer.has_value(), "a fresh queue hands out a handle a side");

    CHECK(!consumer->try_pop().has_value(), "a new queue is empty");
    for (std::uint16_t i = 0; i < spsc::CAPACITY; ++i) {
        Event pushed = event(1, i);
        CHECK(producer->try_push(std::move(pushed)) == queue::PushStatus::Ok, "a push below capacity fits");
    }
    Event refused = event(9, 99);
    CHECK(producer->try_push(std::move(refused)) == queue::PushStatus::Full, "a full queue refuses the push");
    CHECK(refused.sensor_id == 9 && refused.value == 99, "the refused element comes back as it was");

    for (std::uint16_t i = 0; i < spsc::CAPACITY; ++i) {
        const std::optional<Event> got = consumer->try_pop();
        CHECK(got.has_value() && got->sensor_id == 1 && got->value == i, "first in, first out");
    }
    CHECK(!consumer->try_pop().has_value(), "drained");
}

void the_scq_queue_holds_exactly_its_capacity_at_rest_and_keeps_order() {
    scq::QueueConformanceScq q;
    auto producer = q.producer();
    auto consumer = q.consumer();
    CHECK(producer.has_value() && consumer.has_value(), "a fresh queue hands out a handle a side");

    for (std::uint16_t i = 0; i < scq::CAPACITY; ++i) {
        Event pushed = event(2, i);
        CHECK(producer->try_push(std::move(pushed)) == queue::PushStatus::Ok, "a push below capacity fits");
    }
    Event refused = event(9, 99);
    CHECK(producer->try_push(std::move(refused)) == queue::PushStatus::Full,
          "a seventh element does not fit a queue of six");
    for (std::uint16_t i = 0; i < scq::CAPACITY; ++i) {
        const std::optional<Event> got = consumer->try_pop();
        CHECK(got.has_value() && got->sensor_id == 2 && got->value == i, "first in, first out");
    }
    CHECK(!consumer->try_pop().has_value(), "drained");
}

void the_scq_queue_hands_out_no_more_handles_than_its_ring_has_slots() {
    scq::QueueConformanceScq q;
    std::vector<scq::QueueConformanceScq::Producer> producers;
    while (auto p = q.producer()) {
        producers.push_back(std::move(*p));
        CHECK(producers.size() <= scq::RING_SLOTS, "more producer handles than ring slots");
        if (producers.size() > scq::RING_SLOTS) {
            break;
        }
    }
    CHECK(producers.size() == scq::RING_SLOTS,
          "the ring is correct for as many participants a side as it has slots, and no more");
}

/// The module says a queue "may be a static": it owns its storage, allocates
/// nothing and is constant-initialised. (`constinit` makes the compiler reject
/// a queue that is not.)
constinit scq::QueueConformanceScq static_queue;

void the_scq_queue_can_be_a_static() {
    auto producer = static_queue.producer();
    auto consumer = static_queue.consumer();
    Event pushed = event(3, 7);
    CHECK(producer->try_push(std::move(pushed)) == queue::PushStatus::Ok, "push into a static queue");
    const std::optional<Event> got = consumer->try_pop();
    CHECK(got.has_value() && got->sensor_id == 3 && got->value == 7, "pop from a static queue");
}

/// Two producers and two consumers on real threads: nothing is lost, nothing
/// is duplicated, and each consumer sees each producer's elements in order.
void the_scq_queue_loses_nothing_between_threads() {
    constexpr std::uint16_t kPerProducer = 5000;
    constexpr std::size_t kTotal = 2 * kPerProducer;
    scq::QueueConformanceScq q;
    std::atomic<std::size_t> taken{0};
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(60);
    std::vector<std::vector<Event>> logs(2);

    std::vector<std::thread> threads;
    for (std::size_t c = 0; c < 2; ++c) {
        threads.emplace_back([&, c] {
            auto consumer = q.consumer();
            while (taken.load(std::memory_order_acquire) < kTotal) {
                if (std::chrono::steady_clock::now() > deadline) {
                    CHECK(false, "the elements did not all arrive before the deadline");
                    return;
                }
                if (std::optional<Event> e = consumer->try_pop()) {
                    taken.fetch_add(1, std::memory_order_acq_rel);
                    logs[c].push_back(*e);
                } else {
                    std::this_thread::yield();
                }
            }
        });
    }
    for (std::uint8_t sensor = 1; sensor <= 2; ++sensor) {
        threads.emplace_back([&, sensor] {
            auto producer = q.producer();
            for (std::uint16_t i = 0; i < kPerProducer; ++i) {
                Event e = event(sensor, i);
                while (producer->try_push(std::move(e)) != queue::PushStatus::Ok) {
                    std::this_thread::yield();
                }
            }
        });
    }
    for (std::thread &t : threads) {
        t.join();
    }

    std::map<std::uint8_t, std::vector<std::uint16_t>> per_sensor;
    for (const std::vector<Event> &log : logs) {
        std::map<std::uint8_t, std::uint16_t> last;
        for (const Event &e : log) {
            const auto it = last.find(e.sensor_id);
            if (it != last.end()) {
                CHECK(e.value > it->second, "a sensor's elements went backwards");
            }
            last[e.sensor_id] = e.value;
            per_sensor[e.sensor_id].push_back(e.value);
        }
    }
    for (std::uint8_t sensor = 1; sensor <= 2; ++sensor) {
        std::vector<std::uint16_t> values = per_sensor[sensor];
        std::sort(values.begin(), values.end());
        bool exact = values.size() == kPerProducer;
        for (std::uint16_t i = 0; i < values.size() && exact; ++i) {
            exact = values[i] == i;
        }
        CHECK(exact, "every element exactly once");
    }
}

}  // namespace

int main() {
    the_lamport_ring_module_states_what_the_document_required_and_what_it_gives();
    the_scq_module_sizes_its_rings_from_capacity_and_participants();
    the_lamport_ring_hands_elements_over_in_order_and_refuses_when_full();
    the_scq_queue_holds_exactly_its_capacity_at_rest_and_keeps_order();
    the_scq_queue_hands_out_no_more_handles_than_its_ring_has_slots();
    the_scq_queue_can_be_a_static();
    the_scq_queue_loses_nothing_between_threads();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("queue_generated_test: all checks passed");
    return 0;
}
