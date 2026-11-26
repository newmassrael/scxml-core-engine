// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/* THREADS: 3 */
/* CXX_STRONG_CAS */
/* REQUIRED: acq_rel all */

// What this harness is expected to catch, and what has not been measured: the
// acquire-release orderings of the compare-and-swaps weakened to relaxed
// (`acq_rel`) and every ordering weakened (`all`). A segment holds two inputs and
// three are posted, so the third goes either into a slot the owner read an input
// out of or into a segment linked behind the first: a reused slot or a segment
// handed over, the scenario in which lscq_segment_handover.cpp measured those two
// caught (2026-10-11). They were chosen by that reading and were NOT run on GenMC
// when this was written; the gate prints each mutant that survives, and one listed
// here that survives is taken off the list by whoever reads that output, with the
// reason.

// The GenMC layer of the queue kind's verification (the Protocol-Synthesis RFC's queue
// section), for the inbox of a hosted C++ engine (sce/include/core/EngineInbox.h;
// docs/adr/0084; docs/adr/0147, D5): two posters and the owner, on threads of their
// own. engine_inbox_harness.h says what of EngineInbox.h this reads and what it does
// not.
//
// The first poster posts two inputs and then says so; the second posts one, noting
// whether it had heard that the first was done. The owner drains once while they
// post, as at a macrostep boundary, and `main` drains again once every thread is
// joined, as at the end of a run. Held to docs/adr/0147, D1:
//
//   - at most once, and never lost: every post the queue took is applied exactly
//     once, whole, and a hosted inbox, unbounded, refuses none here (its allocator
//     gives every segment asked of it);
//   - arrival order: the first poster's two posts are applied in the order they
//     were made, and a post made after another had returned (the second poster's,
//     when it heard the first was done) is applied after it.
//
// WHAT IT DOES NOT JUDGE: a spurious failure of a weak compare-and-swap
// (`CXX_STRONG_CAS` above), for the reason lscq_segment_handover.cpp gives.
//
// The flag the second poster reads is the harness's own atomic, which no mutant
// rewrites: it orders the posts against each other in real time and gives the owner
// nothing.

#include "engine_inbox_harness.h"

namespace {

namespace inbox = ::engine_inbox_harness;

constexpr std::uint64_t kFirstPoster = 1;
constexpr std::uint64_t kSecondPoster = 2;

// Two inputs a segment, and a ring and a domain for the three participants.
using Allocator = inbox::BudgetAllocator<8>;
using Queue = inbox::InboxQueue<2, 4, Allocator>;

Allocator g_allocator;
Queue::Domain g_domain;

// The queue is built in `main`, in storage of its own, for the reason
// linked_segment_handover.cpp gives.
alignas(Queue) unsigned char g_storage[sizeof(Queue)];
Queue *g_queue = nullptr;

bool g_first_taken[2] = {false, false};
bool g_second_taken = false;
std::atomic<bool> g_first_done{false};
bool g_second_after_first = false;

void *post_two(void *) {
    for (std::uint64_t seq = 1; seq <= 2; seq++) {
        inbox::Node node = inbox::make_node(kFirstPoster, seq);
        g_first_taken[seq - 1] = inbox::post(*g_queue, node);
    }
    g_first_done.store(true, std::memory_order_seq_cst);
    return nullptr;
}

void *post_one(void *) {
    g_second_after_first = g_first_done.load(std::memory_order_seq_cst);
    inbox::Node node = inbox::make_node(kSecondPoster, 1);
    g_second_taken = inbox::post(*g_queue, node);
    return nullptr;
}

void *own(void *) {
    inbox::g_step = 1;
    inbox::drain(*g_queue);
    return nullptr;
}

}  // namespace

int main() {
    pthread_t first;
    pthread_t second;
    pthread_t owner;

    g_queue = ::new (static_cast<void *>(g_storage)) Queue(g_allocator, g_domain);
    if (!g_queue->valid()) {
        return 1;
    }
    pthread_create(&first, nullptr, post_two, nullptr);
    pthread_create(&second, nullptr, post_one, nullptr);
    pthread_create(&owner, nullptr, own, nullptr);
    pthread_join(first, nullptr);
    pthread_join(second, nullptr);
    pthread_join(owner, nullptr);

    // The end of the run: what the queue still holds is applied.
    inbox::g_step = 2;
    inbox::drain(*g_queue);

    assert(g_first_taken[0] && g_first_taken[1] && g_second_taken);
    const int first_at = inbox::applied_once_if_taken(kFirstPoster, 1, g_first_taken[0]);
    const int then_at = inbox::applied_once_if_taken(kFirstPoster, 2, g_first_taken[1]);
    const int second_at = inbox::applied_once_if_taken(kSecondPoster, 1, g_second_taken);
    assert(inbox::g_log_count == 3);

    // The first poster's posts in the order it made them.
    assert(first_at < then_at);
    // A post made after the first poster's had returned comes after them.
    if (g_second_after_first) {
        assert(then_at < second_at);
    }
    (void)first_at;
    (void)then_at;
    (void)second_at;

    // Every segment still held goes back, and the retired ones with them.
    g_queue->~Queue();
    return 0;
}
