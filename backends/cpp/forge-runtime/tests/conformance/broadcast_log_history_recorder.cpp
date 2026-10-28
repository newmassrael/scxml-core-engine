// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The C++ arm's stress runs of the `broadcast-log` kind, written as histories
// (ADR 0062 section 6, layer 3).
//
// This program runs the log of sce/forge/broadcast_log.h on real threads, records
// every operation each thread made, and writes one history per run in the JSON
// form every backend writes (tests/forge/conformance/broadcast_history.schema.json).
// It judges nothing: `sce-codegen check-broadcast-history` does, for this backend
// as for the others, so no backend carries a linearizability checker of its own.
// CTest runs it, then the command over what it wrote (CMakeLists.txt, fixture
// `broadcast_histories`).
//
// The recording follows the queue recorder's: one sequentially consistent counter
// that every thread reads immediately before and immediately after each call, so a
// read-modify-write chain on one atomic orders the readings with the calls between
// them and "returned before invoked" in the record means it in the run.
//
// The checker is a general search over the operations of all participants, so a
// history is kept short: a run publishes tens of entries, and the shapes cover the
// retentions 1 to 4 and one to three readers. A reader that finds the log empty
// records that read and then waits, unrecorded, for the head to move: the waiting
// is a read of the head that the history does not claim, and an empty read changes
// nothing, so leaving the wait out leaves a history that is judged exactly as the
// run was. The first reader of a shape opens at sequence number 0, the second at
// the oldest retained entry and the third at the head, so that all three ways of
// opening are judged.

#include "sce/forge/broadcast_log.h"

#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <string>
#include <system_error>
#include <thread>
#include <vector>

namespace {

namespace blog = ::SCE::Forge::Broadcast;

enum class Kind { Publish, Open, Item, Empty, Lagged, Head };
enum class From { Oldest, Head, At };

struct Operation {
    Operation(Kind k, std::uint64_t invoked_at, std::uint64_t returned_at)
        : kind(k), invoked(invoked_at), returned(returned_at) {}

    Kind kind;
    std::uint64_t invoked;
    std::uint64_t returned;
    std::uint64_t seq = 0;      // a publish, an item
    std::uint64_t value = 0;    // a publish, an item
    std::uint64_t skipped = 0;  // a lagged read
    std::uint64_t at = 0;       // an open from a position
    std::uint64_t cursor = 0;   // an open
    std::uint64_t head = 0;     // a head
    From from = From::Oldest;   // an open
};

using Participant = std::vector<Operation>;

/// How many runs of each shape are recorded: short runs, many of them, because
/// the schedules that matter are a small fraction of those a machine gives.
constexpr int kRunsPerShape = 20;
/// The words of an entry the runs publish.
constexpr std::size_t kEntryWords = 2;
/// The readers a run has at most: the first opens at 0, the second at the oldest
/// retained entry, the third at the head.
constexpr std::size_t kMostReaders = 3;

/// One clock that every participant reads, before and after every call.
class Clock {
public:
    std::uint64_t tick() noexcept {
        return now_.fetch_add(1, std::memory_order_seq_cst);
    }

private:
    std::atomic<std::uint64_t> now_{1};
};

/// A run that does not finish in this long has lost something, and says so.
class Deadline {
public:
    Deadline() : end_(std::chrono::steady_clock::now() + std::chrono::seconds(120)) {}

    bool passed() const {
        return std::chrono::steady_clock::now() > end_;
    }

private:
    std::chrono::steady_clock::time_point end_;
};

/// The history in its JSON form. A value is an unsigned integer, written whole.
std::string operation_json(const Operation &op) {
    std::string out;
    switch (op.kind) {
    case Kind::Publish:
        out = "{\"call\":\"publish\",\"value\":" + std::to_string(op.value) + ",\"seq\":" + std::to_string(op.seq);
        break;
    case Kind::Open:
        if (op.from == From::At) {
            out = "{\"call\":\"open\",\"from\":\"at\",\"at\":" + std::to_string(op.at) +
                  ",\"cursor\":" + std::to_string(op.cursor);
        } else {
            out = std::string("{\"call\":\"open\",\"from\":\"") + (op.from == From::Oldest ? "oldest" : "head") +
                  "\",\"cursor\":" + std::to_string(op.cursor);
        }
        break;
    case Kind::Item:
        out = "{\"call\":\"read\",\"outcome\":\"item\",\"seq\":" + std::to_string(op.seq) +
              ",\"value\":" + std::to_string(op.value);
        break;
    case Kind::Empty:
        out = "{\"call\":\"read\",\"outcome\":\"empty\"";
        break;
    case Kind::Lagged:
        out = "{\"call\":\"read\",\"outcome\":\"lagged\",\"skipped\":" + std::to_string(op.skipped);
        break;
    case Kind::Head:
        out = "{\"call\":\"head\",\"head\":" + std::to_string(op.head);
        break;
    }
    out += ",\"invoked\":" + std::to_string(op.invoked) + ",\"returned\":" + std::to_string(op.returned) + "}";
    return out;
}

std::string participant_json(const Participant &ops) {
    std::string out = "[";
    for (std::size_t at = 0; at < ops.size(); ++at) {
        out += (at == 0 ? "" : ",") + operation_json(ops[at]);
    }
    return out + "]";
}

std::string to_json(std::uint64_t retention, const Participant &publisher, const std::vector<Participant> &readers) {
    std::string out =
        "{\"version\":1,\"retention\":" + std::to_string(retention) + ",\"publisher\":" + participant_json(publisher);
    out += ",\"readers\":[";
    for (std::size_t who = 0; who < readers.size(); ++who) {
        out += (who == 0 ? "" : ",") + participant_json(readers[who]);
    }
    return out + "]}";
}

bool write_history(const std::filesystem::path &dir, const std::string &name, std::uint64_t retention,
                   const Participant &publisher, const std::vector<Participant> &readers) {
    std::ofstream file(dir / (name + ".json"), std::ios::binary | std::ios::trunc);
    file << to_json(retention, publisher, readers) << '\n';
    return file.good();
}

/// One run of a publisher and `readers` readers on a log of `Slots` slots (which
/// retains `Slots - 1` entries), the publisher publishing `publishes` entries.
template <std::size_t Slots>
bool record_run(const std::filesystem::path &dir, const std::string &name, std::size_t readers,
                std::uint64_t publishes) {
    using Log = blog::BroadcastLog<kEntryWords, Slots>;
    Log log;
    Clock clock;
    Deadline deadline;
    std::atomic<bool> failed{false};
    Participant publisher_ops;
    std::vector<Participant> reader_ops(readers);

    std::vector<std::thread> threads;
    for (std::size_t r = 0; r < readers; ++r) {
        threads.emplace_back([&, r] {
            Participant &ops = reader_ops[r];
            const From from = r == 0 ? From::At : (r == 1 ? From::Oldest : From::Head);
            std::uint64_t invoked = clock.tick();
            blog::Reader reader =
                from == From::At ? log.reader_at(0) : (from == From::Oldest ? log.reader_oldest() : log.reader_head());
            std::uint64_t returned = clock.tick();
            Operation open{Kind::Open, invoked, returned};
            open.from = from;
            open.cursor = reader.cursor();
            ops.push_back(open);

            typename Log::Entry out{};
            std::uint64_t reads = 0;
            while (reader.cursor() < publishes) {
                invoked = clock.tick();
                const blog::Read read = log.read(reader, out);
                returned = clock.tick();
                switch (read.kind) {
                case blog::ReadKind::Item: {
                    Operation op{Kind::Item, invoked, returned};
                    op.seq = read.seq;
                    op.value = out[0];
                    ops.push_back(op);
                    for (std::size_t k = 1; k < kEntryWords; ++k) {
                        if (out[k] != out[0]) {
                            failed = true;
                        }
                    }
                    break;
                }
                case blog::ReadKind::Lagged: {
                    Operation op{Kind::Lagged, invoked, returned};
                    op.skipped = read.skipped;
                    ops.push_back(op);
                    break;
                }
                case blog::ReadKind::Empty:
                    ops.push_back(Operation{Kind::Empty, invoked, returned});
                    // Wait, unrecorded, for the head to move past the cursor.
                    while (log.head() <= reader.cursor()) {
                        if (deadline.passed()) {
                            failed = true;
                            return;
                        }
                        std::this_thread::yield();
                    }
                    break;
                }
                if (++reads % 8 == 0) {
                    invoked = clock.tick();
                    const std::uint64_t head = log.head();
                    returned = clock.tick();
                    Operation op{Kind::Head, invoked, returned};
                    op.head = head;
                    ops.push_back(op);
                }
            }
        });
    }
    threads.emplace_back([&] {
        auto publisher = log.publisher();
        if (!publisher) {
            failed = true;
            return;
        }
        for (std::uint64_t n = 0; n < publishes; ++n) {
            typename Log::Entry entry{};
            entry.fill(n + 1);
            const std::uint64_t invoked = clock.tick();
            const std::uint64_t seq = publisher->publish(entry);
            const std::uint64_t returned = clock.tick();
            if (seq != n) {
                failed = true;
                return;
            }
            Operation op{Kind::Publish, invoked, returned};
            op.seq = seq;
            op.value = n + 1;
            publisher_ops.push_back(op);
            if (n % 4 == 3) {
                const std::uint64_t head_invoked = clock.tick();
                const std::uint64_t head = log.head();
                const std::uint64_t head_returned = clock.tick();
                Operation head_op{Kind::Head, head_invoked, head_returned};
                head_op.head = head;
                publisher_ops.push_back(head_op);
            }
            // Give the readers the processor, so that they are sometimes ahead of
            // the publisher and sometimes far behind it.
            if (n % 8 == 7) {
                std::this_thread::yield();
            }
        }
    });
    for (std::thread &t : threads) {
        t.join();
    }
    if (failed) {
        std::fprintf(stderr, "%s: an entry was torn, a sequence number was wrong or a reader missed the deadline\n",
                     name.c_str());
        return false;
    }
    return write_history(dir, name, Slots - 1, publisher_ops, reader_ops);
}

struct Shape {
    std::size_t retention;
    std::size_t readers;
    std::uint64_t publishes;
};

/// The retentions 1 to 4, one to three readers: the smallest logs lap their slots
/// the most, and the larger ones leave a reader room to be ahead of the publisher.
constexpr Shape kShapes[] = {{1, 2, 80}, {2, 3, 80}, {3, 2, 80}, {1, 1, 120}, {4, 3, 60}};

bool record_shape(const std::filesystem::path &dir, const std::string &name, const Shape &shape) {
    switch (shape.retention) {
    case 1:
        return record_run<2>(dir, name, shape.readers, shape.publishes);
    case 2:
        return record_run<3>(dir, name, shape.readers, shape.publishes);
    case 3:
        return record_run<4>(dir, name, shape.readers, shape.publishes);
    case 4:
        return record_run<5>(dir, name, shape.readers, shape.publishes);
    default:
        std::fprintf(stderr, "%s: no log of retention %zu is recorded\n", name.c_str(), shape.retention);
        return false;
    }
}

}  // namespace

int main(int argc, char **argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: %s <directory the histories are written to>\n", argv[0]);
        return 2;
    }
    const std::filesystem::path dir = argv[1];
    std::error_code error;
    std::filesystem::create_directories(dir, error);
    if (error) {
        std::fprintf(stderr, "cannot create %s\n", dir.string().c_str());
        return 2;
    }
    for (const Shape &shape : kShapes) {
        if (shape.readers > kMostReaders) {
            std::fprintf(stderr, "a shape has more readers than the recorder opens ways\n");
            return 2;
        }
        for (int run = 0; run < kRunsPerShape; ++run) {
            const std::string name = "cpp_broadcast_r" + std::to_string(shape.retention) + "_c" +
                                     std::to_string(shape.readers) + "_n" + std::to_string(shape.publishes) + "_" +
                                     std::to_string(run);
            if (!record_shape(dir, name, shape)) {
                return 1;
            }
        }
    }
    return 0;
}
