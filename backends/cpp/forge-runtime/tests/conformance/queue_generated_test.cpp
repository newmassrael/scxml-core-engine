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
#include "queue_conformance_intrusive.h"
#include "queue_conformance_intrusive_many.h"
#include "queue_conformance_node.h"
#include "queue_conformance_scq.h"
#include "queue_conformance_segmented.h"
#include "queue_conformance_segmented_many.h"
#include "queue_conformance_spsc.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <map>
#include <new>
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
namespace node_ns = ::SCE::Generated::QueueConformanceNode;
namespace intrusive = ::SCE::Generated::QueueConformanceIntrusive;
namespace intrusive_many = ::SCE::Generated::QueueConformanceIntrusiveMany;
namespace segmented = ::SCE::Generated::QueueConformanceSegmented;
namespace segmented_many = ::SCE::Generated::QueueConformanceSegmentedMany;

using Node = node_ns::QueueConformanceNode;

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

// ─── The intrusive list ───

void the_intrusive_modules_state_what_the_document_required_and_what_it_gives() {
    CHECK(!intrusive::MANY_CONSUMERS, "one consumer needs no flag");
    CHECK(intrusive_many::MANY_CONSUMERS, "many consumers are serialised by a flag");
    CHECK(intrusive::DECLARED_PROGRESS == "blocking", "declared progress");
    CHECK(intrusive::PUSH_PROGRESS == "wait-free", "a push is one exchange and one store");
    CHECK(intrusive::POP_PROGRESS == "blocking", "a pop can wait behind a producer");
    CHECK(contains(intrusive::ALGORITHM, "Vyukov"), "many producers select Vyukov's list");
    CHECK(intrusive_many::POP_PROGRESS == "blocking", "a pop can also wait for the flag");
}

/// One queue of either shape over the caller's array of generated nodes: the
/// nodes come back in the order pushed, the last one included and at once, and
/// the queue leaves every field but the link alone.
template <typename Queue> void an_intrusive_queue_hands_nodes_back_in_order() {
    Node nodes[4];
    for (std::uint8_t i = 0; i < 4; ++i) {
        nodes[i] = Node{.sensor_id = static_cast<std::uint8_t>(i + 1),
                        .value = static_cast<std::uint16_t>(i * 9),
                        .next = 0xDEADBEEFu};
    }
    Queue q(nodes, 4, 3);
    CHECK(!q.try_pop().has_value(), "a new queue is empty");
    q.push(2);
    q.push(0);
    q.push(1);
    CHECK(q.try_pop() == std::optional<std::uint32_t>(2), "first in, first out");
    CHECK(q.try_pop() == std::optional<std::uint32_t>(0), "first in, first out");
    q.push(2);
    CHECK(q.try_pop() == std::optional<std::uint32_t>(1), "first in, first out");
    CHECK(q.try_pop() == std::optional<std::uint32_t>(2), "the last node comes back at once, and can be pushed again");
    CHECK(!q.try_pop().has_value(), "drained");
    for (std::uint8_t i = 0; i < 4; ++i) {
        CHECK(nodes[i].sensor_id == i + 1 && nodes[i].value == i * 9, "the queue touched more than the link");
    }
}

// ─── The segmented queues ───

/// A bump arena over one block: the allocator a segmented queue is injected with
/// on a target that has no heap. A grant is a compare-and-swap on the offset, so
/// an allocation is lock-free, which is what `kProgress` says and what the
/// documents declare for it. Nothing is given back one block at a time: the arena
/// is released whole, which is how such a target frees what a queue held.
class Arena {
public:
    static constexpr queue::Progress kProgress = queue::Progress::LockFree;

    explicit Arena(std::size_t len)
        : base_(static_cast<unsigned char *>(::operator new(len, std::align_val_t(64)))), len_(len) {}

    ~Arena() {
        ::operator delete(base_, std::align_val_t(64));
    }

    Arena(const Arena &) = delete;
    Arena &operator=(const Arena &) = delete;

    void *allocate(std::size_t size, std::size_t align) noexcept {
        std::size_t used = next_.load(std::memory_order_relaxed);
        for (;;) {
            const std::uintptr_t address = reinterpret_cast<std::uintptr_t>(base_) + used;
            const std::size_t padding = (align - address % align) % align;
            const std::size_t end = used + padding + size;
            if (end > len_) {
                return nullptr;
            }
            if (next_.compare_exchange_weak(used, end, std::memory_order_acq_rel, std::memory_order_relaxed)) {
                return base_ + used + padding;
            }
        }
    }

    void deallocate(void *, std::size_t, std::size_t) noexcept {}

    /// How much of the block has been given out.
    std::size_t used() const noexcept {
        return next_.load(std::memory_order_acquire);
    }

private:
    unsigned char *base_;
    std::size_t len_;
    std::atomic<std::size_t> next_{0};
};

void the_segmented_modules_state_what_the_document_required_and_what_it_gives() {
    CHECK(segmented::SEGMENT == 3, "segment is the document's");
    CHECK(segmented::DECLARED_PROGRESS == "lock-free", "declared progress");
    CHECK(segmented::PUSH_PROGRESS == "lock-free", "a push is no stronger than its allocator");
    CHECK(segmented::POP_PROGRESS == "wait-free", "a pop of linked Lamport rings is wait-free");
    CHECK(contains(segmented::ALGORITHM, "Lamport"), "one producer and one consumer select linked Lamport rings");
    CHECK(segmented::ALLOCATOR_PROGRESS == queue::Progress::LockFree, "the allocator progress is the document's");
    CHECK(segmented::ALLOCATOR_PROGRESS_WORD == "lock-free", "and its word");

    CHECK(segmented_many::SEGMENT == 3 && segmented_many::PARTICIPANTS == 2, "segment and participants");
    CHECK(segmented_many::RING_SLOTS == 4, "the next power of two at or above max(segment, participants)");
    CHECK(segmented_many::HAZARD_SLOTS == 4, "a slot for every handle on either side");
    CHECK(segmented_many::PUSH_PROGRESS == "lock-free" && segmented_many::POP_PROGRESS == "lock-free",
          "LSCQ is lock-free on both sides");
    CHECK(contains(segmented_many::ALGORITHM, "LSCQ"), "many producers and consumers select LSCQ");
    CHECK(segmented_many::WRAP_BOUND_OPS == (std::uint64_t{1} << 62), "the bound the template states is the runtime's");
}

void the_linked_lamport_queue_hands_events_over_in_order_across_segments() {
    Arena arena(16 * 1024);
    segmented::QueueConformanceSegmented<Arena> q(arena);
    CHECK(q.valid(), "the arena gives the first segment");
    auto producer = q.producer();
    auto consumer = q.consumer();
    CHECK(producer.has_value() && consumer.has_value(), "one handle a side");
    CHECK(!consumer->try_pop().has_value(), "a new queue is empty");
    // Three segments' worth and a part: order holds across the links.
    for (std::uint16_t i = 0; i < 8; ++i) {
        Event pushed = event(1, i);
        CHECK(producer->try_push(std::move(pushed)) == queue::PushStatus::Ok, "a push fits the arena");
    }
    for (std::uint16_t i = 0; i < 8; ++i) {
        const std::optional<Event> got = consumer->try_pop();
        CHECK(got.has_value() && got->sensor_id == 1 && got->value == i, "first in, first out");
    }
    CHECK(!consumer->try_pop().has_value(), "drained");
    CHECK(!q.producer().has_value(), "there is one producer, and it is taken");
}

void the_lscq_queue_hands_events_over_in_order_across_segments() {
    Arena arena(64 * 1024);
    segmented_many::QueueConformanceSegmentedManyDomain domain;
    segmented_many::QueueConformanceSegmentedMany<Arena> q(arena, domain);
    CHECK(q.valid(), "the arena gives the first segment");
    auto producer = q.producer();
    auto consumer = q.consumer();
    CHECK(producer.has_value() && consumer.has_value(), "a place a side");
    CHECK(!consumer->try_pop().has_value(), "a new queue is empty");
    for (std::uint16_t i = 0; i < 10; ++i) {
        Event pushed = event(2, i);
        CHECK(producer->try_push(std::move(pushed)) == queue::PushStatus::Ok, "a push fits the arena");
    }
    for (std::uint16_t i = 0; i < 10; ++i) {
        const std::optional<Event> got = consumer->try_pop();
        CHECK(got.has_value() && got->sensor_id == 2 && got->value == i, "first in, first out");
    }
    CHECK(!consumer->try_pop().has_value(), "drained");
    // A second producer and a second consumer take the other slots of the domain,
    // which has one for every handle either side can hold.
    auto second_producer = producer->try_clone();
    auto second_consumer = consumer->try_clone();
    CHECK(second_producer.has_value() && second_consumer.has_value(), "the second handle of each side");
    CHECK(!producer->try_clone().has_value() && !consumer->try_clone().has_value(),
          "the domain's four slots are taken");
    Event pushed = event(3, 9);
    CHECK(second_producer->try_push(std::move(pushed)) == queue::PushStatus::Ok, "the second producer pushes");
    const std::optional<Event> got = second_consumer->try_pop();
    CHECK(got.has_value() && got->sensor_id == 3 && got->value == 9, "and the second consumer pops it");
}

void a_spent_arena_refuses_the_segment_and_the_event_stays_with_its_owner() {
    // An arena that holds the first segment and not a second: the push that needs
    // one reports OutOfMemory and leaves its element as it was.
    Arena probe(1 << 20);
    {
        segmented::QueueConformanceSegmented<Arena> probed(probe);
    }
    Arena arena(probe.used());
    segmented::QueueConformanceSegmented<Arena> q(arena);
    CHECK(q.valid(), "the arena holds exactly the first segment");
    auto producer = q.producer();
    auto consumer = q.consumer();
    for (std::uint16_t i = 0; i < segmented::SEGMENT; ++i) {
        Event pushed = event(1, i);
        CHECK(producer->try_push(std::move(pushed)) == queue::PushStatus::Ok, "the first segment holds a segment");
    }
    Event refused = event(9, 99);
    CHECK(producer->try_push(std::move(refused)) == queue::PushStatus::OutOfMemory, "the next needs a second segment");
    CHECK(refused.sensor_id == 9 && refused.value == 99, "the refused event comes back whole");
    for (std::uint16_t i = 0; i < segmented::SEGMENT; ++i) {
        const std::optional<Event> got = consumer->try_pop();
        CHECK(got.has_value() && got->sensor_id == 1 && got->value == i, "what fit comes out in order");
    }

    // The same for the list of rings.
    Arena probe_many(1 << 20);
    segmented_many::QueueConformanceSegmentedManyDomain probe_domain;
    {
        segmented_many::QueueConformanceSegmentedMany<Arena> probed(probe_many, probe_domain);
    }
    Arena arena_many(probe_many.used());
    segmented_many::QueueConformanceSegmentedManyDomain domain;
    segmented_many::QueueConformanceSegmentedMany<Arena> many(arena_many, domain);
    CHECK(many.valid(), "the arena holds exactly the first segment");
    auto many_producer = many.producer();
    for (std::uint16_t i = 0; i < segmented_many::SEGMENT; ++i) {
        Event pushed = event(1, i);
        CHECK(many_producer->try_push(std::move(pushed)) == queue::PushStatus::Ok, "the first segment holds a segment");
    }
    Event refused_many = event(9, 99);
    CHECK(many_producer->try_push(std::move(refused_many)) == queue::PushStatus::OutOfMemory,
          "the next needs a second segment");
    CHECK(refused_many.sensor_id == 9 && refused_many.value == 99, "the refused event comes back whole");
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
    the_intrusive_modules_state_what_the_document_required_and_what_it_gives();
    an_intrusive_queue_hands_nodes_back_in_order<intrusive::QueueConformanceIntrusive>();
    an_intrusive_queue_hands_nodes_back_in_order<intrusive_many::QueueConformanceIntrusiveMany>();
    the_segmented_modules_state_what_the_document_required_and_what_it_gives();
    the_linked_lamport_queue_hands_events_over_in_order_across_segments();
    the_lscq_queue_hands_events_over_in_order_across_segments();
    a_spent_arena_refuses_the_segment_and_the_event_stays_with_its_owner();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("queue_generated_test: all checks passed");
    return 0;
}
