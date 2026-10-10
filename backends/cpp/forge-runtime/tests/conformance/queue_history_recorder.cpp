// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The C++ arm's stress runs of the `queue` kind, written as histories (SCE
// Protocol-Synthesis RFC §synth-5-P, verification layer 2).
//
// This program runs the queues of sce/forge/queue.h on real threads, records
// every attempt each thread made, and writes one history per run in the JSON
// form every backend writes (tests/forge/conformance/queue_history.schema.json).
// It judges nothing: `sce-codegen check-queue-history` does, for this
// backend as for the others, so no backend carries a linearizability checker
// of its own. CTest runs it, then the command over what it wrote
// (CMakeLists.txt, fixture `queue_histories`).
//
// The recording follows the Rust arm's: one sequentially consistent counter
// that both sides read immediately before and immediately after each call, so
// a read-modify-write chain on one atomic orders the readings with the calls
// between them and "returned before invoked" in the record means it in the
// run. Every attempt is recorded — the refused pushes and the empty pops too,
// because those are results the checker must account for.
//
// The shapes are the Rust arm's. A shape is recorded many times, each run its
// own file: the schedules that matter (a pop stopped between taking its
// element and handing its slot back) are a small fraction of those a machine
// gives, so one recording of a shape would judge the likely ones.

#include "sce/forge/queue.h"
#include "sce/forge/queue_segmented.h"

#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <new>
#include <optional>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace {

namespace queue = ::SCE::Forge::Queue;

enum class Call { Push, Pop };
enum class Outcome { Pushed, Full, Popped, Empty };

struct Operation {
    Call call;
    Outcome outcome;
    std::uint64_t value;  // the value pushed, or popped; unused for an empty pop
    std::uint64_t invoked;
    std::uint64_t returned;
};

using Participant = std::vector<Operation>;

/// How many runs of each SCQ shape are recorded: short runs, many of them,
/// because the schedules that matter are a small fraction of those a machine
/// gives.
constexpr int kRunsPerScqShape = 25;
/// How many runs of each Lamport capacity are recorded. A Lamport run pushes
/// thousands of values and its history is megabytes, and the ring has no
/// schedule a second run reaches that the first does not: the Rust arm records
/// it once.
constexpr int kRunsPerSpscCapacity = 3;
/// How many values each Lamport run pushes through the queue.
constexpr std::uint64_t kValuesPerSpscRun = 2000;

/// One clock that every participant reads, before and after every call.
class Clock {
public:
    std::uint64_t tick() noexcept {
        return now_.fetch_add(1, std::memory_order_seq_cst);
    }

private:
    std::atomic<std::uint64_t> now_{1};
};

/// A run that does not finish in this long has lost an element, and says so.
class Deadline {
public:
    Deadline() : end_(std::chrono::steady_clock::now() + std::chrono::seconds(120)) {}

    bool passed() const {
        return std::chrono::steady_clock::now() > end_;
    }

private:
    std::chrono::steady_clock::time_point end_;
};

const char *call_word(Call call) {
    return call == Call::Push ? "push" : "pop";
}

const char *outcome_word(Outcome outcome) {
    switch (outcome) {
    case Outcome::Pushed:
        return "pushed";
    case Outcome::Full:
        return "full";
    case Outcome::Popped:
        return "popped";
    case Outcome::Empty:
        return "empty";
    }
    return "?";
}

/// The history in its JSON form. A value is an unsigned integer, written whole.
std::string to_json(std::size_t capacity, const char *refusal, const char *empty_pops,
                    const std::vector<Participant> &participants) {
    std::string out = "{\"version\":1,\"capacity\":" + std::to_string(capacity) + ",\"refusal\":\"" + refusal + "\",";
    // The default reading of an empty pop is the strict one, so it is written
    // only for a queue that excuses one.
    if (empty_pops != nullptr) {
        out += std::string("\"empty_pops\":\"") + empty_pops + "\",";
    }
    out += "\"participants\":[";
    for (std::size_t who = 0; who < participants.size(); ++who) {
        out += who == 0 ? "[" : ",[";
        for (std::size_t at = 0; at < participants[who].size(); ++at) {
            const Operation &op = participants[who][at];
            out += at == 0 ? "{" : ",{";
            out += std::string("\"call\":\"") + call_word(op.call) + "\"";
            // A push names the value it pushed, a pop that popped the value it
            // returned, and a pop that found the queue empty names none.
            if (op.call == Call::Push || op.outcome == Outcome::Popped) {
                out += ",\"value\":" + std::to_string(op.value);
            }
            out += std::string(",\"outcome\":\"") + outcome_word(op.outcome) + "\"";
            out += ",\"invoked\":" + std::to_string(op.invoked) + ",\"returned\":" + std::to_string(op.returned) + "}";
        }
        out += "]";
    }
    out += "]}";
    return out;
}

bool write_history(const std::filesystem::path &dir, const std::string &name, std::size_t capacity, const char *refusal,
                   const std::vector<Participant> &participants, const char *empty_pops = nullptr) {
    std::ofstream file(dir / (name + ".json"), std::ios::binary | std::ios::trunc);
    file << to_json(capacity, refusal, empty_pops, participants) << '\n';
    return file.good();
}

/// One producer and one consumer on two threads through the Lamport ring.
bool record_spsc_run(const std::filesystem::path &dir, const std::string &name, auto &queue_instance,
                     std::size_t capacity) {
    Clock clock;
    Deadline deadline;
    auto producer = queue_instance.producer();
    auto consumer = queue_instance.consumer();
    if (!producer || !consumer) {
        std::fprintf(stderr, "%s: a fresh queue hands out a handle a side\n", name.c_str());
        return false;
    }

    std::vector<Participant> participants(2);
    std::atomic<bool> timed_out{false};
    std::thread producing([&] {
        Participant &ops = participants[0];
        for (std::uint64_t value = 1; value <= kValuesPerSpscRun; ++value) {
            for (;;) {
                if (deadline.passed()) {
                    timed_out = true;
                    return;
                }
                std::uint64_t pushed = value;
                const std::uint64_t invoked = clock.tick();
                const queue::PushStatus status = producer->try_push(std::move(pushed));
                const std::uint64_t returned = clock.tick();
                if (status == queue::PushStatus::Ok) {
                    ops.push_back({Call::Push, Outcome::Pushed, value, invoked, returned});
                    break;
                }
                ops.push_back({Call::Push, Outcome::Full, value, invoked, returned});
                std::this_thread::yield();
            }
        }
    });
    std::thread consuming([&] {
        Participant &ops = participants[1];
        std::uint64_t delivered = 0;
        while (delivered < kValuesPerSpscRun) {
            if (deadline.passed()) {
                timed_out = true;
                return;
            }
            const std::uint64_t invoked = clock.tick();
            const std::optional<std::uint64_t> got = consumer->try_pop();
            const std::uint64_t returned = clock.tick();
            if (got) {
                ops.push_back({Call::Pop, Outcome::Popped, *got, invoked, returned});
                ++delivered;
            } else {
                ops.push_back({Call::Pop, Outcome::Empty, 0, invoked, returned});
                std::this_thread::yield();
            }
        }
    });
    producing.join();
    consuming.join();
    if (timed_out) {
        std::fprintf(stderr, "%s: the elements did not all arrive before the deadline\n", name.c_str());
        return false;
    }
    return write_history(dir, name, capacity, "at-capacity", participants);
}

/// `producers` producers and `consumers` consumers on real threads through the
/// SCQ queue, each producer pushing `per_producer` distinct values.
template <std::size_t N, std::size_t R>
bool record_scq_run(const std::filesystem::path &dir, const std::string &name, std::size_t producers,
                    std::size_t consumers, std::uint64_t per_producer) {
    queue::Scq<std::uint64_t, N, R> queue_instance;
    Clock clock;
    Deadline deadline;
    const std::uint64_t total = producers * per_producer;
    std::atomic<std::uint64_t> delivered{0};
    std::atomic<bool> timed_out{false};
    std::vector<Participant> participants(producers + consumers);

    std::vector<std::thread> threads;
    for (std::size_t p = 0; p < producers; ++p) {
        threads.emplace_back([&, p] {
            auto producer = queue_instance.producer();
            Participant &ops = participants[p];
            for (std::uint64_t i = 0; i < per_producer; ++i) {
                const std::uint64_t value = static_cast<std::uint64_t>(p) * per_producer + i + 1;
                for (;;) {
                    if (deadline.passed()) {
                        timed_out = true;
                        return;
                    }
                    std::uint64_t pushed = value;
                    const std::uint64_t invoked = clock.tick();
                    const queue::PushStatus status = producer->try_push(std::move(pushed));
                    const std::uint64_t returned = clock.tick();
                    if (status == queue::PushStatus::Ok) {
                        ops.push_back({Call::Push, Outcome::Pushed, value, invoked, returned});
                        break;
                    }
                    ops.push_back({Call::Push, Outcome::Full, value, invoked, returned});
                    std::this_thread::yield();
                }
            }
        });
    }
    for (std::size_t c = 0; c < consumers; ++c) {
        threads.emplace_back([&, c] {
            auto consumer = queue_instance.consumer();
            Participant &ops = participants[producers + c];
            while (delivered.load(std::memory_order_acquire) < total) {
                if (deadline.passed()) {
                    timed_out = true;
                    return;
                }
                const std::uint64_t invoked = clock.tick();
                const std::optional<std::uint64_t> got = consumer->try_pop();
                const std::uint64_t returned = clock.tick();
                if (got) {
                    ops.push_back({Call::Pop, Outcome::Popped, *got, invoked, returned});
                    delivered.fetch_add(1, std::memory_order_acq_rel);
                } else {
                    ops.push_back({Call::Pop, Outcome::Empty, 0, invoked, returned});
                    std::this_thread::yield();
                }
            }
        });
    }
    for (std::thread &t : threads) {
        t.join();
    }
    if (timed_out) {
        std::fprintf(stderr, "%s: the elements did not all arrive before the deadline\n", name.c_str());
        return false;
    }
    return write_history(dir, name, N, "while-slots-are-held", participants);
}

/// A node of the caller's array: a payload, and the link the queue uses.
struct Node {
    std::uint64_t payload;
    std::uint32_t link;
};

/// `producers` producers and `consumers` consumers on real threads through the
/// intrusive list, over `producers * per_producer` nodes and the stub. A push
/// cannot fail. A pop may answer empty while a push is in flight, so the
/// history says so (`empty_pops`).
template <bool ManyConsumers>
bool record_intrusive_run(const std::filesystem::path &dir, const std::string &name, std::size_t producers,
                          std::size_t consumers, std::uint64_t per_producer) {
    const std::uint64_t total = producers * per_producer;
    std::vector<Node> array(total + 1);
    queue::IntrusiveMpsc<Node, &Node::link, ManyConsumers> list(array.data(), static_cast<std::uint32_t>(total + 1),
                                                                static_cast<std::uint32_t>(total));
    Clock clock;
    Deadline deadline;
    std::atomic<std::uint64_t> delivered{0};
    std::atomic<bool> timed_out{false};
    std::vector<Participant> participants(producers + consumers);

    std::vector<std::thread> threads;
    for (std::size_t p = 0; p < producers; ++p) {
        threads.emplace_back([&, p] {
            Participant &ops = participants[p];
            for (std::uint64_t i = 0; i < per_producer; ++i) {
                const std::uint64_t index = static_cast<std::uint64_t>(p) * per_producer + i;
                const std::uint64_t value = index + 1;
                array[index].payload = value;
                const std::uint64_t invoked = clock.tick();
                list.push(static_cast<std::uint32_t>(index));
                const std::uint64_t returned = clock.tick();
                ops.push_back({Call::Push, Outcome::Pushed, value, invoked, returned});
            }
        });
    }
    for (std::size_t c = 0; c < consumers; ++c) {
        threads.emplace_back([&, c] {
            Participant &ops = participants[producers + c];
            while (delivered.load(std::memory_order_acquire) < total) {
                if (deadline.passed()) {
                    timed_out = true;
                    return;
                }
                const std::uint64_t invoked = clock.tick();
                const std::optional<std::uint32_t> got = list.try_pop();
                const std::uint64_t returned = clock.tick();
                if (got) {
                    ops.push_back({Call::Pop, Outcome::Popped, array[*got].payload, invoked, returned});
                    delivered.fetch_add(1, std::memory_order_acq_rel);
                } else {
                    ops.push_back({Call::Pop, Outcome::Empty, 0, invoked, returned});
                    std::this_thread::yield();
                }
            }
        });
    }
    for (std::thread &t : threads) {
        t.join();
    }
    if (timed_out) {
        std::fprintf(stderr, "%s: the nodes did not all arrive before the deadline\n", name.c_str());
        return false;
    }
    return write_history(dir, name, total, "at-capacity", participants, "while-a-push-is-in-flight");
}

template <bool ManyConsumers>
bool record_intrusive_runs(const std::filesystem::path &dir, std::size_t producers, std::size_t consumers,
                           std::uint64_t per_producer) {
    bool ok = true;
    for (int run = 0; run < kRunsPerScqShape; ++run) {
        ok &= record_intrusive_run<ManyConsumers>(dir,
                                                  "cpp_intrusive_p" + std::to_string(producers) + "_c" +
                                                      std::to_string(consumers) + "_" + std::to_string(run),
                                                  producers, consumers, per_producer);
    }
    return ok;
}

/// The system allocator behind the contract a `segmented` queue is injected
/// with, counting the blocks it has out so a run can say that the queue gave every
/// segment back. The system allocator can block, which is what `kProgress` says.
class SystemAllocator {
public:
    static constexpr queue::Progress kProgress = queue::Progress::Blocking;

    void *allocate(std::size_t size, std::size_t align) noexcept {
        void *block = ::operator new(size, std::align_val_t(align), std::nothrow);
        if (block != nullptr) {
            live_.fetch_add(1, std::memory_order_seq_cst);
        }
        return block;
    }

    void deallocate(void *block, std::size_t, std::size_t align) noexcept {
        ::operator delete(block, std::align_val_t(align));
        live_.fetch_sub(1, std::memory_order_seq_cst);
    }

    std::size_t live() const noexcept {
        return live_.load(std::memory_order_seq_cst);
    }

private:
    std::atomic<std::size_t> live_{0};
};

/// `producers` producers and `consumers` consumers on real threads through a
/// segmented queue, each producer pushing `per_producer` distinct values. A
/// segmented queue has no capacity and the recording gives none that could
/// matter: the history is judged against a queue that holds every value, so a push
/// is never refused, and an empty pop is judged exactly as for any other queue.
/// `make_push` and `make_pop` are called on the thread that will use them and give
/// the function that pushes or pops through the handle that thread owns.
template <typename MakePush, typename MakePop>
bool record_segmented_run(const std::filesystem::path &dir, const std::string &name, std::size_t producers,
                          std::size_t consumers, std::uint64_t per_producer, MakePush make_push, MakePop make_pop) {
    Clock clock;
    Deadline deadline;
    const std::uint64_t total = producers * per_producer;
    std::atomic<std::uint64_t> delivered{0};
    std::atomic<bool> timed_out{false};
    std::vector<Participant> participants(producers + consumers);

    std::vector<std::thread> threads;
    for (std::size_t p = 0; p < producers; ++p) {
        threads.emplace_back([&, p] {
            auto push_one = make_push();
            Participant &ops = participants[p];
            for (std::uint64_t i = 0; i < per_producer; ++i) {
                const std::uint64_t value = static_cast<std::uint64_t>(p) * per_producer + i + 1;
                std::uint64_t pushed = value;
                const std::uint64_t invoked = clock.tick();
                const queue::PushStatus status = push_one(std::move(pushed));
                const std::uint64_t returned = clock.tick();
                if (status != queue::PushStatus::Ok) {
                    timed_out = true;
                    return;
                }
                ops.push_back({Call::Push, Outcome::Pushed, value, invoked, returned});
            }
        });
    }
    for (std::size_t c = 0; c < consumers; ++c) {
        threads.emplace_back([&, c] {
            auto pop_one = make_pop();
            Participant &ops = participants[producers + c];
            while (delivered.load(std::memory_order_acquire) < total) {
                if (deadline.passed()) {
                    timed_out = true;
                    return;
                }
                const std::uint64_t invoked = clock.tick();
                const std::optional<std::uint64_t> got = pop_one();
                const std::uint64_t returned = clock.tick();
                if (got) {
                    ops.push_back({Call::Pop, Outcome::Popped, *got, invoked, returned});
                    delivered.fetch_add(1, std::memory_order_acq_rel);
                } else {
                    ops.push_back({Call::Pop, Outcome::Empty, 0, invoked, returned});
                    std::this_thread::yield();
                }
            }
        });
    }
    for (std::thread &t : threads) {
        t.join();
    }
    if (timed_out) {
        std::fprintf(stderr, "%s: the elements did not all arrive before the deadline\n", name.c_str());
        return false;
    }
    return write_history(dir, name, total, "at-capacity", participants);
}

/// Linked Lamport rings of four-element segments: one producer, one consumer.
bool record_linked_runs(const std::filesystem::path &dir, std::uint64_t per_producer) {
    bool ok = true;
    for (int run = 0; run < kRunsPerScqShape; ++run) {
        SystemAllocator allocator;
        bool recorded = false;
        {
            queue::LinkedLamport<std::uint64_t, 4, SystemAllocator, queue::Progress::Blocking> q(allocator);
            recorded = record_segmented_run(
                dir, "cpp_linked_lamport_n4_p1_c1_" + std::to_string(run), 1, 1, per_producer,
                [&] {
                    return [producer = std::move(*q.producer())](std::uint64_t &&v) mutable {
                        return producer.try_push(std::move(v));
                    };
                },
                [&] { return [consumer = std::move(*q.consumer())]() mutable { return consumer.try_pop(); }; });
        }
        // The queue is gone: every segment must be back, the unconsumed ones too.
        ok &= recorded && allocator.live() == 0;
    }
    return ok;
}

/// LSCQ over segments of `N` elements, rings of `R`, a domain of `H` slots.
template <std::size_t N, std::size_t R, std::size_t H>
bool record_lscq_runs(const std::filesystem::path &dir, std::size_t producers, std::size_t consumers,
                      std::uint64_t per_producer) {
    bool ok = true;
    for (int run = 0; run < kRunsPerScqShape; ++run) {
        SystemAllocator allocator;
        queue::HazardDomain<H> domain;
        bool recorded = false;
        {
            queue::Lscq<std::uint64_t, N, R, H, SystemAllocator, queue::Progress::Blocking> q(allocator, domain);
            recorded = record_segmented_run(
                dir,
                "cpp_lscq_n" + std::to_string(N) + "_r" + std::to_string(R) + "_p" + std::to_string(producers) + "_c" +
                    std::to_string(consumers) + "_" + std::to_string(run),
                producers, consumers, per_producer,
                [&] {
                    return [producer = std::move(*q.producer())](std::uint64_t &&v) mutable {
                        return producer.try_push(std::move(v));
                    };
                },
                [&] { return [consumer = std::move(*q.consumer())]() mutable { return consumer.try_pop(); }; });
        }
        ok &= recorded && allocator.live() == 0 && domain.waiting() == 0;
    }
    return ok;
}

template <std::size_t N> bool record_spsc_runs(const std::filesystem::path &dir) {
    bool ok = true;
    for (int run = 0; run < kRunsPerSpscCapacity; ++run) {
        queue::Spsc<std::uint64_t, N> queue_instance;
        ok &= record_spsc_run(dir, "cpp_spsc_n" + std::to_string(N) + "_" + std::to_string(run), queue_instance, N);
    }
    return ok;
}

template <std::size_t N, std::size_t R>
bool record_scq_runs(const std::filesystem::path &dir, std::size_t producers, std::size_t consumers,
                     std::uint64_t per_producer) {
    bool ok = true;
    for (int run = 0; run < kRunsPerScqShape; ++run) {
        ok &= record_scq_run<N, R>(dir,
                                   "cpp_scq_n" + std::to_string(N) + "_r" + std::to_string(R) + "_p" +
                                       std::to_string(producers) + "_c" + std::to_string(consumers) + "_" +
                                       std::to_string(run),
                                   producers, consumers, per_producer);
    }
    return ok;
}

}  // namespace

int main(int argc, char **argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: queue_history_recorder <directory to write the histories to>\n");
        return 2;
    }
    const std::filesystem::path dir = argv[1];
    // Emptied first: a file left from an earlier run would be judged in place
    // of one this run did not write.
    std::filesystem::remove_all(dir);
    std::filesystem::create_directories(dir);

    bool ok = true;
    ok &= record_spsc_runs<1>(dir);
    ok &= record_spsc_runs<2>(dir);
    ok &= record_spsc_runs<3>(dir);
    ok &= record_spsc_runs<8>(dir);
    // A ring is at least as large as the number of participants working it, so
    // the small capacities ride on rings sized for their threads.
    ok &= record_scq_runs<1, 2>(dir, 2, 2, 150);
    ok &= record_scq_runs<3, 4>(dir, 2, 2, 150);
    ok &= record_scq_runs<8, 8>(dir, 2, 2, 150);
    ok &= record_scq_runs<2, 4>(dir, 3, 1, 120);
    ok &= record_scq_runs<4, 4>(dir, 1, 3, 120);
    ok &= record_scq_runs<5, 8>(dir, 2, 2, 150);
    ok &= record_intrusive_runs<false>(dir, 1, 1, 150);
    ok &= record_intrusive_runs<false>(dir, 2, 1, 120);
    ok &= record_intrusive_runs<false>(dir, 3, 1, 100);
    ok &= record_intrusive_runs<true>(dir, 2, 2, 100);
    ok &= record_intrusive_runs<true>(dir, 3, 2, 80);
    ok &= record_linked_runs(dir, 600);
    ok &= record_lscq_runs<2, 4, 8>(dir, 2, 1, 60);
    ok &= record_lscq_runs<2, 4, 8>(dir, 1, 2, 60);
    ok &= record_lscq_runs<2, 4, 8>(dir, 2, 2, 50);
    ok &= record_lscq_runs<4, 4, 8>(dir, 3, 3, 30);

    std::size_t written = 0;
    for (const auto &entry : std::filesystem::directory_iterator(dir)) {
        written += entry.path().extension() == ".json" ? 1 : 0;
    }
    std::printf("queue_history_recorder: %zu histories written to %s\n", written, dir.c_str());
    return ok ? 0 : 1;
}
