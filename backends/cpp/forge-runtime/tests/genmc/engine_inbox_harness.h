// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the C++ inbox harnesses share (engine_inbox_*.cpp; docs/adr/0147, D5): the
// queue of a hosted engine's inbox, made and driven as sce/include/core/EngineInbox.h
// makes and drives it, over the forge LSCQ (sce/forge/queue_segmented.h). Included
// once, by a harness, before anything of its own.
//
// WHAT IS JUDGED. `EngineInboxState` moves each input into a node of its own and
// pushes the node's pointer through an `Lscq<std::unique_ptr<Input>, Segment, R, H,
// Allocator, Progress::Blocking>` whose ring and domain are one size; a poster takes
// a producer handle for each push and gives it back after, and the owner takes a
// consumer handle for each drain. The harnesses make that queue in that shape and
// drive it that way, so every ordering GenMC reads is the LSCQ's as it ships, and
// as the gate's C++ mutants rewrite it. An input's words are written by the poster
// before the push and read by the owner through the pointer the queue handed over,
// so a hand-over too weak to carry them is a race GenMC reports.
//
// WHAT IS NOT, AND WHY:
//
//   - The sizes. A segment of 64 inputs and 64 participants are constants of
//     EngineInbox.h, and each makes a loop (a ring's 128 entries, a scan of 64
//     hazard slots) past the gate's unroll bound, where GenMC kills the execution.
//     A harness takes the smallest that hold its scenario.
//   - The gate that refuses a post to an inbox whose run ended (the closed bit and
//     the count of posts in flight), and the waker. EngineInbox.h reaches them with
//     std::mutex, std::function, std::shared_ptr and std::thread::id, the parts of
//     libstdc++ genmc_cxx_shim.h declares and does not make run. No GenMC harness
//     judges the C++ gate; the C11 door's refusal of a session gone is
//     backends/c/forge-runtime/tests/genmc/inbox_session_gone.c.
//   - The allocation of a node. `new` is an external function GenMC cannot execute,
//     so a node is made in memory GenMC allocates (hazard_keeps_a_named_node.cpp says
//     why) and given back by a deleter of its own, in place of
//     `std::default_delete`. The slot carries the same pointer.

#ifndef SCE_FORGE_GENMC_ENGINE_INBOX_HARNESS_H
#define SCE_FORGE_GENMC_ENGINE_INBOX_HARNESS_H

#include <atomic>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <new>
#include <optional>
#include <utility>

#include <pthread.h>

// One look of a dequeue at an entry still empty, for the reason
// lscq_segment_handover.cpp gives.
#define SCE_QUEUE_SCQ_SPINS 1u

#include <sce/forge/queue.h>
#include <sce/forge/queue_segmented.h>

#include "genmc_cxx_support.h"

namespace engine_inbox_harness {

using SCE::Forge::Queue::Lscq;
using SCE::Forge::Queue::Progress;
using SCE::Forge::Queue::PushStatus;

// An input: who posted it, its place among that poster's posts (from 1), and a
// word made of both, so an input read before it was whole shows.
struct Input {
    std::uint64_t poster;
    std::uint64_t seq;
    std::uint64_t check;
};

constexpr std::uint64_t check_of(std::uint64_t poster, std::uint64_t seq) noexcept {
    return (poster << 8) | seq;
}

// Gives a node back to GenMC's allocator.
struct NodeRelease {
    void operator()(Input *input) const noexcept {
        input->~Input();
        __VERIFIER_free(input);
    }
};

using Node = std::unique_ptr<Input, NodeRelease>;

// A node holding the input `seq` of `poster`, made on the posting thread as
// `EngineInboxState::post` makes one.
inline Node make_node(std::uint64_t poster, std::uint64_t seq) {
    void *memory = __VERIFIER_malloc_aligned(alignof(Input), sizeof(Input));
    return Node(::new (memory) Input{poster, seq, check_of(poster, seq)});
}

// Where the queue's segments come from: GenMC's allocation primitives, and at most
// `Budget` segments over the run, past which it refuses as a heap that refuses
// does. Any number of threads may call it at once.
template <int Budget> class BudgetAllocator {
public:
    static constexpr Progress kProgress = Progress::Blocking;

    void *allocate(std::size_t size, std::size_t align) noexcept {
        if (left_.fetch_sub(1, std::memory_order_relaxed) <= 0) {
            return nullptr;
        }
        return __VERIFIER_malloc_aligned(align, size);
    }

    void deallocate(void *block, std::size_t, std::size_t) noexcept {
        __VERIFIER_free(block);
    }

private:
    std::atomic<int> left_{Budget};
};

// The inbox's queue, with the ring and the domain one size, as EngineInbox.h makes
// them (`ENGINE_INBOX_PARTICIPANTS` for both).
template <std::size_t Segment, std::size_t Participants, typename Allocator>
using InboxQueue = Lscq<Node, Segment, Participants, Participants, Allocator, Progress::Blocking>;

// Post `node`; whether the queue took it. As `EngineInboxState::push`: a producer
// handle for the push, given back after it. That push waits for a place when every
// one is taken; a harness sizes its queue so that one is always free, since the
// wait is a loop GenMC bounds. A refused node is left with the poster.
template <typename Queue> bool post(Queue &queue, Node &node) {
    auto producer = queue.producer();
    assert(producer.has_value());
    return producer->try_push(std::move(node)) == PushStatus::Ok;
}

// What the owner applied, in order, and in which of its steps. Written by the owner
// alone and read by `main` once every thread is joined.
struct Applied {
    std::uint64_t poster;
    std::uint64_t seq;
    unsigned step;
};

inline constexpr unsigned kLogMax = 8;
inline Applied g_log[kLogMax];
inline unsigned g_log_count = 0;
inline unsigned g_step = 0;

// Apply one input: it must be whole.
inline void apply(const Input &input) {
    assert(input.poster == 1 || input.poster == 2);
    assert(input.check == check_of(input.poster, input.seq));
    assert(g_log_count < kLogMax);
    g_log[g_log_count] = Applied{input.poster, input.seq, g_step};
    g_log_count++;
}

// Apply what the queue holds, oldest first. As `EngineInboxState::drain`: one
// consumer handle for the drain. That drain stops at the count of posts taken when
// it began; this one stops when the queue answers empty, which in a harness is
// bounded by the posts it makes.
template <typename Queue> void drain(Queue &queue) {
    auto consumer = queue.consumer();
    assert(consumer.has_value());
    for (unsigned i = 0; i < kLogMax; i++) {
        std::optional<Node> taken = consumer->try_pop();
        if (!taken) {
            return;
        }
        apply(**taken);
    }
}

// Where in the log the post `seq` of `poster` was applied, or -1 when it was not.
// A post applied twice is an error of its own.
inline int applied_at(std::uint64_t poster, std::uint64_t seq) {
    int at = -1;
    for (unsigned i = 0; i < g_log_count; i++) {
        if (g_log[i].poster == poster && g_log[i].seq == seq) {
            assert(at < 0);
            at = static_cast<int>(i);
        }
    }
    return at;
}

// At most once, and never lost: a post the queue took was applied exactly once, and
// one it refused never was. Answers where it was applied.
inline int applied_once_if_taken(std::uint64_t poster, std::uint64_t seq, bool taken) {
    const int at = applied_at(poster, seq);
    assert(taken == (at >= 0));
    return at;
}

}  // namespace engine_inbox_harness

#endif  // SCE_FORGE_GENMC_ENGINE_INBOX_HARNESS_H
