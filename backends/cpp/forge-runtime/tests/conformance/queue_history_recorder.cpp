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

#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <fstream>
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
std::string to_json(std::size_t capacity, const char *refusal, const std::vector<Participant> &participants) {
    std::string out = "{\"version\":1,\"capacity\":" + std::to_string(capacity) + ",\"refusal\":\"" + refusal +
                      "\",\"participants\":[";
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
                   const std::vector<Participant> &participants) {
    std::ofstream file(dir / (name + ".json"), std::ios::binary | std::ios::trunc);
    file << to_json(capacity, refusal, participants) << '\n';
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

    std::size_t written = 0;
    for (const auto &entry : std::filesystem::directory_iterator(dir)) {
        written += entry.path().extension() == ".json" ? 1 : 0;
    }
    std::printf("queue_history_recorder: %zu histories written to %s\n", written, dir.c_str());
    return ok ? 0 : 1;
}
