// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce/forge/queue.h — the C++ arm of the `queue` kind's verification (SCE
// Protocol-Synthesis RFC §synth-5-P), layers 1 and 5's inputs:
//
//   - Contract scenarios. Every scenario in
//     tests/forge/conformance/queue_contract.json runs against the runtime
//     queue its storage row names, the same scenarios the Rust arm runs. A row
//     this arm has no runtime for is refused by name, never skipped.
//   - The runtime's own properties the scenarios do not reach: the entry map
//     of an SCQ ring is a bijection, a side hands out no more handles than it
//     has places, a refused push leaves its argument as it was, a queue in
//     static storage is constant-initialised and usable, and real threads
//     lose nothing and keep each producer's order.
//
// The thread tests are also the inputs of the sanitizer lanes
// (scripts/build_tsan.sh): ThreadSanitizer finds a data race in them, which
// no assertion here would.

#include "sce/forge/queue.h"

#include <nlohmann/json.hpp>

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <fstream>
#include <map>
#include <optional>
#include <string>
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
using nlohmann::json;
using Destroyed = std::vector<std::uint64_t>;

/// An element that records its own destruction, so a scenario can say which
/// elements a destroyed queue destroyed. A moved-from element records nothing:
/// only the object that holds the value does.
struct Tracked {
    std::uint64_t value = 0;
    Destroyed *destroyed = nullptr;

    Tracked(std::uint64_t v, Destroyed *log) noexcept : value(v), destroyed(log) {}

    Tracked(Tracked &&other) noexcept : value(other.value), destroyed(other.destroyed) {
        other.destroyed = nullptr;
    }

    Tracked(const Tracked &) = delete;
    Tracked &operator=(const Tracked &) = delete;
    Tracked &operator=(Tracked &&) = delete;

    ~Tracked() {
        if (destroyed != nullptr) {
            destroyed->push_back(value);
        }
    }
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━ Layer 1 — contract scenarios ━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// What a scenario asks of a queue, so one reading of the steps serves every
/// runtime this arm has. Each runtime reaches its producer and consumer its
/// own way; the scenarios do not know how.
template <typename Q> struct Subject {
    Q queue;

    static constexpr std::size_t capacity_constant() {
        return Q::kCapacity;
    }

    std::size_t capacity() const {
        return queue.capacity();
    }

    queue::PushStatus push(Tracked &&element) {
        auto producer = queue.producer();
        CHECK(producer.has_value(), "a scenario holds one handle at a time");
        return producer->try_push(std::move(element));
    }

    std::optional<Tracked> pop() {
        auto consumer = queue.consumer();
        CHECK(consumer.has_value(), "a scenario holds one handle at a time");
        return consumer->try_pop();
    }
};

template <typename Q> void run_scenario(const std::string &id, const json &steps) {
    Destroyed destroyed;
    std::optional<Subject<Q>> subject;
    subject.emplace();

    std::size_t index = 0;
    for (const json &step : steps) {
        const std::string context = "scenario " + id + " step " + std::to_string(index++);
        const std::string op = step.at("op").get<std::string>();
        const json &expect = step.at("expect");
        if (!subject.has_value()) {
            CHECK(false, (context + ": a step follows \"destroy\"").c_str());
            return;
        }

        if (op == "capacity") {
            CHECK(subject->capacity() == expect.get<std::uint64_t>(), context.c_str());
            CHECK(Subject<Q>::capacity_constant() == subject->capacity(), context.c_str());
        } else if (op == "push") {
            const std::uint64_t value = step.at("value").get<std::uint64_t>();
            Tracked element(value, &destroyed);
            const queue::PushStatus status = subject->push(std::move(element));
            const std::string want = expect.get<std::string>();
            if (status == queue::PushStatus::Ok) {
                CHECK(want == "ok", (context + ": push succeeded, expected " + want).c_str());
            } else if (status == queue::PushStatus::Full) {
                CHECK(want == "full", (context + ": push was refused, expected " + want).c_str());
                // A refused push hands back its own value: the argument was not
                // moved from.
                CHECK(element.value == value && element.destroyed == &destroyed,
                      (context + ": a refused push leaves its element as it was").c_str());
            } else {
                CHECK(false, (context + ": a bounded queue reported OutOfMemory").c_str());
            }
        } else if (op == "pop") {
            const std::optional<Tracked> popped = subject->pop();
            if (expect.is_string()) {
                CHECK(expect.get<std::string>() == "empty" && !popped.has_value(),
                      (context + ": expected the queue to be empty").c_str());
            } else {
                CHECK(popped.has_value() && popped->value == expect.get<std::uint64_t>(), context.c_str());
            }
        } else if (op == "destroy") {
            Destroyed expected = expect.get<Destroyed>();
            destroyed.clear();
            subject.reset();
            Destroyed actual = destroyed;
            std::sort(expected.begin(), expected.end());
            std::sort(actual.begin(), actual.end());
            CHECK(actual == expected, (context + ": the queue destroyed a different set").c_str());
        } else {
            CHECK(false, (context + ": unknown op \"" + op + "\"").c_str());
        }
    }
}

/// The capacities the fixture uses. A template argument needs its value at
/// compile time; a scenario naming another capacity stops here by name.
void dispatch_bounded_spsc(const std::string &id, std::uint64_t capacity, const json &steps) {
    switch (capacity) {
    case 1:
        run_scenario<queue::Spsc<Tracked, 1>>(id, steps);
        break;
    case 2:
        run_scenario<queue::Spsc<Tracked, 2>>(id, steps);
        break;
    case 3:
        run_scenario<queue::Spsc<Tracked, 3>>(id, steps);
        break;
    case 4:
        run_scenario<queue::Spsc<Tracked, 4>>(id, steps);
        break;
    default:
        CHECK(false,
              ("scenario " + id + ": capacity " + std::to_string(capacity) + " is not in this arm's dispatch; add it")
                  .c_str());
    }
}

/// The same for the SCQ row, whose ring is the capacity rounded up to a power
/// of two, so each capacity names both.
void dispatch_bounded_scq(const std::string &id, std::uint64_t capacity, const json &steps) {
    switch (capacity) {
    case 1:
        run_scenario<queue::Scq<Tracked, 1, 1>>(id, steps);
        break;
    case 2:
        run_scenario<queue::Scq<Tracked, 2, 2>>(id, steps);
        break;
    case 3:
        run_scenario<queue::Scq<Tracked, 3, 4>>(id, steps);
        break;
    case 4:
        run_scenario<queue::Scq<Tracked, 4, 4>>(id, steps);
        break;
    case 5:
        run_scenario<queue::Scq<Tracked, 5, 8>>(id, steps);
        break;
    default:
        CHECK(false,
              ("scenario " + id + ": capacity " + std::to_string(capacity) + " is not in this arm's dispatch; add it")
                  .c_str());
    }
}

/// A node of the caller's array: a payload the queue must leave alone, and the
/// link it uses in place.
struct Node {
    std::uint64_t payload;
    std::uint32_t link;
    std::uint32_t tag;
};

constexpr std::uint32_t kNodeTag = 0xA5A5A5A5u;

std::uint64_t node_payload(std::uint32_t index) {
    return static_cast<std::uint64_t>(index) * 40503u + 7u;
}

/// An intrusive scenario: `value` and `expect` are indices among `nodes` nodes
/// the caller owns; the queue is given one more as its stub.
template <bool ManyConsumers>
void run_intrusive_scenario(const std::string &id, std::uint32_t nodes, const json &steps) {
    std::vector<Node> array(nodes + 1u);
    for (std::uint32_t i = 0; i < array.size(); ++i) {
        array[i] = Node{node_payload(i), 0xDEADBEEFu, kNodeTag};
    }
    queue::IntrusiveMpsc<Node, &Node::link, ManyConsumers> list(array.data(), nodes + 1u, nodes);
    CHECK(list.nodes() == nodes + 1u && list.stub() == nodes, "the stub is the node set aside");
    std::size_t index = 0;
    for (const json &step : steps) {
        const std::string context = "scenario " + id + " step " + std::to_string(index++);
        const std::string op = step.at("op").get<std::string>();
        if (op == "push") {
            list.push(step.at("value").get<std::uint32_t>());
        } else if (op == "pop") {
            const json &expect = step.at("expect");
            const std::optional<std::uint32_t> got = list.try_pop();
            if (expect.is_string()) {
                CHECK(!got.has_value(), (context + ": expected the queue to be empty").c_str());
            } else {
                CHECK(got.has_value() && *got == expect.get<std::uint32_t>(),
                      (context + ": popped " + (got ? std::to_string(*got) : "nothing") + ", want node " +
                       std::to_string(expect.get<std::uint32_t>()))
                          .c_str());
            }
        } else {
            CHECK(false, (context + ": an intrusive scenario has no step " + op).c_str());
        }
    }
    for (std::uint32_t i = 0; i < array.size(); ++i) {
        CHECK(array[i].payload == node_payload(i) && array[i].tag == kNodeTag,
              ("scenario " + id + ": the queue wrote outside the link of node " + std::to_string(i)).c_str());
    }
}

void every_contract_scenario_holds() {
    std::ifstream in(QUEUE_CONTRACT_JSON_PATH);
    CHECK(in.good(), "the contract file opens");
    const json contract = json::parse(in);
    CHECK(contract.at("version").get<int>() == 1, "this arm reads version 1 of the contract");
    const json &scenarios = contract.at("scenarios");
    CHECK(!scenarios.empty(), "a contract with no scenarios checks nothing");

    for (const json &scenario : scenarios) {
        const std::string id = scenario.at("id").get<std::string>();
        const std::string storage = scenario.at("storage").get<std::string>();
        const std::string producers = scenario.at("producers").get<std::string>();
        const std::string consumers = scenario.at("consumers").get<std::string>();
        const json &steps = scenario.at("steps");
        if (storage == "intrusive") {
            // An intrusive scenario names the nodes the caller owns, not a capacity.
            const std::uint32_t nodes = scenario.at("nodes").get<std::uint32_t>();
            if (consumers == "one") {
                run_intrusive_scenario<false>(id, nodes, steps);
            } else {
                run_intrusive_scenario<true>(id, nodes, steps);
            }
            continue;
        }
        const std::uint64_t capacity = scenario.at("capacity").get<std::uint64_t>();
        if (storage == "bounded" && producers == "one" && consumers == "one") {
            dispatch_bounded_spsc(id, capacity, steps);
        } else if (storage == "bounded") {
            // Any other cardinality selects the SCQ row (the RFC's selection
            // table), so the three combinations share one runtime.
            dispatch_bounded_scq(id, capacity, steps);
        } else {
            CHECK(false, ("scenario " + id + ": the C++ arm has no runtime for the row " + storage + "/" + producers +
                          "/" + consumers)
                             .c_str());
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━ The runtime's own properties ━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// `map` and `unmap` are inverse bijections on the entries, for rings below
/// and above the size at which the rotation starts.
template <std::size_t R> void map_is_a_bijection() {
    constexpr std::uint64_t entries = 2 * R;
    std::vector<bool> seen(entries, false);
    for (std::uint64_t ticket = 0; ticket < entries; ++ticket) {
        const std::size_t position = queue::detail::Ring<R>::map(ticket);
        CHECK(position < entries, "a ticket maps inside the ring");
        if (position >= entries) {
            continue;
        }
        CHECK(!seen[position], "two tickets share an entry");
        seen[position] = true;
        CHECK(queue::detail::Ring<R>::unmap(position) == ticket, "unmap inverts map");
    }
}

void the_entry_map_is_a_bijection() {
    map_is_a_bijection<1>();
    map_is_a_bijection<2>();
    map_is_a_bijection<4>();
    map_is_a_bijection<8>();
    map_is_a_bijection<64>();
}

/// A single-threaded run of `rounds` fills and drains of a queue of capacity
/// `N`: exact capacity, FIFO order, and the same again after the ring's cycles
/// have moved many times.
template <std::size_t N, std::size_t R> void scq_fills_and_drains(std::size_t rounds) {
    queue::Scq<std::uint64_t, N, R> q;
    auto producer = q.producer();
    auto consumer = q.consumer();
    CHECK(producer.has_value() && consumer.has_value(), "a fresh queue hands out a handle a side");
    std::uint64_t next = 0;
    std::uint64_t expected = 0;
    for (std::size_t round = 0; round < rounds; ++round) {
        for (std::size_t i = 0; i < N; ++i) {
            std::uint64_t value = next++;
            CHECK(producer->try_push(std::move(value)) == queue::PushStatus::Ok, "a queue below capacity takes a push");
        }
        std::uint64_t overflow = next;
        CHECK(producer->try_push(std::move(overflow)) == queue::PushStatus::Full,
              "a queue at capacity refuses the next push");
        CHECK(overflow == next, "a refused push leaves its argument as it was");
        for (std::size_t i = 0; i < N; ++i) {
            const std::optional<std::uint64_t> got = consumer->try_pop();
            CHECK(got.has_value() && *got == expected, "first in, first out");
            ++expected;
        }
        CHECK(!consumer->try_pop().has_value(), "a drained queue is empty");
    }
}

void an_scq_queue_fills_and_drains_across_many_cycles() {
    scq_fills_and_drains<1, 1>(2000);
    scq_fills_and_drains<3, 4>(2000);
    scq_fills_and_drains<4, 4>(2000);
    scq_fills_and_drains<5, 8>(2000);
    scq_fills_and_drains<16, 16>(500);
}

void a_lamport_ring_keeps_order_across_many_laps() {
    queue::Spsc<std::uint64_t, 3> q;
    auto producer = q.producer();
    auto consumer = q.consumer();
    std::uint64_t next = 0;
    std::uint64_t expected = 0;
    for (int i = 0; i < 20000; ++i) {
        std::uint64_t value = next++;
        CHECK(producer->try_push(std::move(value)) == queue::PushStatus::Ok, "room for one");
        if (i % 3 == 2) {
            for (int k = 0; k < 3; ++k) {
                const std::optional<std::uint64_t> got = consumer->try_pop();
                CHECK(got.has_value() && *got == expected, "first in, first out");
                ++expected;
            }
        }
    }
}

/// A side hands out no more handles than it has places, and a place comes back
/// when its handle is destroyed.
void a_side_hands_out_no_more_handles_than_it_has_places() {
    queue::Scq<int, 2, 4> q;
    std::vector<queue::Scq<int, 2, 4>::Producer> producers;
    while (auto p = q.producer()) {
        producers.push_back(std::move(*p));
        CHECK(producers.size() <= 4, "more producer handles than ring slots");
        if (producers.size() > 4) {
            break;
        }
    }
    CHECK(producers.size() == 4, "the ring is correct for as many participants a side as it has slots");
    CHECK(!producers.front().try_clone().has_value(), "a clone past the places is refused");
    producers.pop_back();
    CHECK(q.producer().has_value(), "a destroyed handle gives its place back");

    queue::Spsc<int, 2> lamport;
    {
        auto only = lamport.producer();
        CHECK(only.has_value(), "the one producer place is free");
        CHECK(!lamport.producer().has_value(), "a ring shared by two producers is not the Lamport ring");
        auto consumer = lamport.consumer();
        CHECK(consumer.has_value(), "the consumer place is its own");
        CHECK(!lamport.consumer().has_value(), "a ring shared by two consumers is not the Lamport ring");
    }
    CHECK(lamport.producer().has_value() && lamport.consumer().has_value(),
          "destroying the handles gives both places back");
}

/// A Lamport ring handle taken again sees what the earlier one left: its
/// indices are read from the queue, not assumed.
void a_new_lamport_handle_continues_where_the_last_stopped() {
    queue::Spsc<std::uint64_t, 4> q;
    std::uint64_t value = 7;
    CHECK(q.producer()->try_push(std::move(value)) == queue::PushStatus::Ok, "push through a first handle");
    value = 8;
    CHECK(q.producer()->try_push(std::move(value)) == queue::PushStatus::Ok, "push through a second handle");
    const std::optional<std::uint64_t> a = q.consumer()->try_pop();
    const std::optional<std::uint64_t> b = q.consumer()->try_pop();
    CHECK(a == 7 && b == 8, "the elements leave in order through different handles");
}

/// An element that owns a counted resource: the queue must move it in and out
/// without losing or duplicating the count.
struct Counted {
    static inline std::atomic<int> alive{0};
    int id;

    explicit Counted(int i) noexcept : id(i) {
        alive.fetch_add(1);
    }

    Counted(Counted &&other) noexcept : id(other.id) {
        alive.fetch_add(1);
    }

    Counted(const Counted &) = delete;
    Counted &operator=(const Counted &) = delete;
    Counted &operator=(Counted &&) = delete;

    ~Counted() {
        alive.fetch_sub(1);
    }
};

void a_queue_destroys_exactly_the_elements_it_still_holds() {
    {
        queue::Scq<Counted, 4, 4> q;
        {
            auto producer = q.producer();
            for (int i = 0; i < 3; ++i) {
                Counted c(i);
                CHECK(producer->try_push(std::move(c)) == queue::PushStatus::Ok, "room");
            }
        }
        {
            auto consumer = q.consumer();
            const std::optional<Counted> got = consumer->try_pop();
            CHECK(got.has_value() && got->id == 0, "the oldest leaves first");
        }
        // Two elements are still held.
    }
    CHECK(Counted::alive.load() == 0, "no element outlives its queue and none is destroyed twice");
}

/// The module says a queue "may be a static": it owns its storage, allocates
/// nothing, and is constant-initialised, so it is usable before any dynamic
/// initialisation has run. (`constinit` makes the compiler reject a queue
/// that is not.)
constinit queue::Scq<std::uint64_t, 4, 4> static_queue;
constinit queue::Spsc<std::uint64_t, 4> static_ring;

void a_queue_in_static_storage_is_constant_initialised_and_usable() {
    auto producer = static_queue.producer();
    auto consumer = static_queue.consumer();
    std::uint64_t value = 31;
    CHECK(producer->try_push(std::move(value)) == queue::PushStatus::Ok, "push into a static queue");
    CHECK(consumer->try_pop() == 31, "pop from a static queue");

    auto ring_producer = static_ring.producer();
    auto ring_consumer = static_ring.consumer();
    value = 32;
    CHECK(ring_producer->try_push(std::move(value)) == queue::PushStatus::Ok, "push into a static ring");
    CHECK(ring_consumer->try_pop() == 32, "pop from a static ring");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━ Real threads ━━━━━━━━━━━━━━━━━━━━━━━━━━━━

struct Event {
    std::uint8_t producer;
    std::uint32_t sequence;
};

/// Two producers and two consumers on real threads: nothing is lost, nothing
/// is duplicated, and each consumer sees each producer's elements in the order
/// they went in. The consumers stop when every element sent has been taken,
/// not after a quiet spell, so a slow producer is never mistaken for a lost
/// element; the deadline turns a genuinely lost element into a failure.
void the_scq_queue_loses_nothing_between_threads() {
    constexpr std::uint32_t kPerProducer = 20000;
    constexpr std::size_t kTotal = 2 * kPerProducer;
    queue::Scq<Event, 8, 8> q;
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
    for (std::uint8_t p = 0; p < 2; ++p) {
        threads.emplace_back([&, p] {
            auto producer = q.producer();
            for (std::uint32_t i = 0; i < kPerProducer; ++i) {
                Event e{p, i};
                while (producer->try_push(std::move(e)) != queue::PushStatus::Ok) {
                    std::this_thread::yield();
                }
            }
        });
    }
    for (std::thread &t : threads) {
        t.join();
    }

    std::map<std::uint8_t, std::vector<std::uint32_t>> per_producer;
    for (const std::vector<Event> &log : logs) {
        std::map<std::uint8_t, std::uint32_t> last;
        for (const Event &e : log) {
            const auto it = last.find(e.producer);
            if (it != last.end()) {
                CHECK(e.sequence > it->second, "a producer's elements went backwards");
            }
            last[e.producer] = e.sequence;
            per_producer[e.producer].push_back(e.sequence);
        }
    }
    for (std::uint8_t p = 0; p < 2; ++p) {
        std::vector<std::uint32_t> got = per_producer[p];
        std::sort(got.begin(), got.end());
        CHECK(got.size() == kPerProducer, "every element exactly once");
        bool exact = true;
        for (std::uint32_t i = 0; i < got.size() && exact; ++i) {
            exact = got[i] == i;
        }
        CHECK(exact, "every element exactly once, none twice");
    }
}

/// One producer and one consumer through the Lamport ring: order and count.
void the_lamport_ring_loses_nothing_between_threads() {
    constexpr std::uint32_t kCount = 200000;
    queue::Spsc<std::uint32_t, 16> q;
    std::vector<std::uint32_t> got;
    got.reserve(kCount);
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(60);

    std::thread consumer_thread([&] {
        auto consumer = q.consumer();
        while (got.size() < kCount) {
            if (std::chrono::steady_clock::now() > deadline) {
                CHECK(false, "the elements did not all arrive before the deadline");
                return;
            }
            if (std::optional<std::uint32_t> v = consumer->try_pop()) {
                got.push_back(*v);
            } else {
                std::this_thread::yield();
            }
        }
    });
    std::thread producer_thread([&] {
        auto producer = q.producer();
        for (std::uint32_t i = 0; i < kCount; ++i) {
            std::uint32_t v = i;
            while (producer->try_push(std::move(v)) != queue::PushStatus::Ok) {
                std::this_thread::yield();
            }
        }
    });
    producer_thread.join();
    consumer_thread.join();

    bool in_order = got.size() == kCount;
    for (std::uint32_t i = 0; i < got.size() && in_order; ++i) {
        in_order = got[i] == i;
    }
    CHECK(in_order, "every element exactly once, in the order pushed");
}

}  // namespace

int main() {
    every_contract_scenario_holds();
    the_entry_map_is_a_bijection();
    an_scq_queue_fills_and_drains_across_many_cycles();
    a_lamport_ring_keeps_order_across_many_laps();
    a_side_hands_out_no_more_handles_than_it_has_places();
    a_new_lamport_handle_continues_where_the_last_stopped();
    a_queue_destroys_exactly_the_elements_it_still_holds();
    a_queue_in_static_storage_is_constant_initialised_and_usable();
    the_scq_queue_loses_nothing_between_threads();
    the_lamport_ring_loses_nothing_between_threads();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("queue_runtime_test: all checks passed");
    return 0;
}
