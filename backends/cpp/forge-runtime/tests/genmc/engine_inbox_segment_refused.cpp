// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/* THREADS: 3 */
/* CXX_STRONG_CAS */
/* REQUIRED: acq_rel all */

// What this harness is expected to catch, and what has not been measured: the same
// two mutants as engine_inbox_two_posters.cpp. A segment holds one input and the
// allocator gives no second, so the second post is taken only into the slot the
// owner read the first out of: a reused slot, written by a poster other than the
// first. Chosen by reading and NOT run on GenMC when this was written; the gate
// prints each mutant that survives.

// The GenMC layer of the queue kind's verification (the Protocol-Synthesis RFC's queue
// section), for the inbox of a hosted C++ engine (sce/include/core/EngineInbox.h;
// docs/adr/0084, D2; docs/adr/0147, D5): the refusal of a post when memory for it
// is refused. A hosted inbox is unbounded, so this is the refusal it has where a
// bounded inbox refuses as full: the newest segment is closed and the allocator
// will not give another. engine_inbox_harness.h says what of EngineInbox.h this
// reads and what it does not.
//
// The allocator gives the first segment and no other, and the segment holds one
// input. Two posters post one input each while the owner drains once; `main`
// drains again once every thread is joined. Held to docs/adr/0147, D1, and to
// docs/adr/0084, D2 (a refused post is handed back):
//
//   - a post the queue took is applied exactly once, whole, and a post it refused
//     never is;
//   - a refused post's node is still the poster's, untouched, and one the queue
//     took is the queue's.
//
// Both posts can be refused: a push that took the segment's free slot finds the
// segment closed under it by the other push, which found no slot, and gives its
// element back (sce/forge/queue.h, `Scq::put`), and the allocator refuses both.
//
// WHAT IT DOES NOT JUDGE: a spurious failure of a weak compare-and-swap
// (`CXX_STRONG_CAS` above), for the reason lscq_segment_handover.cpp gives.

#include "engine_inbox_harness.h"

namespace {

namespace inbox = ::engine_inbox_harness;

// One input a segment, one segment in all, and a ring and a domain for the three
// participants.
using Allocator = inbox::BudgetAllocator<1>;
using Queue = inbox::InboxQueue<1, 4, Allocator>;

Allocator g_allocator;
Queue::Domain g_domain;

// The queue is built in `main`, in storage of its own, for the reason
// linked_segment_handover.cpp gives.
alignas(Queue) unsigned char g_storage[sizeof(Queue)];
Queue *g_queue = nullptr;

bool g_taken[2] = {false, false};

void *post(void *arg) {
    const auto poster = static_cast<std::uint64_t>(reinterpret_cast<std::uintptr_t>(arg));
    inbox::Node node = inbox::make_node(poster, 1);
    inbox::Input *const made = node.get();
    const bool taken = inbox::post(*g_queue, node);
    if (taken) {
        assert(node == nullptr);
    } else {
        // Handed back whole.
        assert(node.get() == made);
        assert(node->poster == poster && node->seq == 1);
        assert(node->check == inbox::check_of(poster, 1));
    }
    g_taken[poster - 1] = taken;
    (void)made;
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
    pthread_create(&first, nullptr, post, reinterpret_cast<void *>(std::uintptr_t{1}));
    pthread_create(&second, nullptr, post, reinterpret_cast<void *>(std::uintptr_t{2}));
    pthread_create(&owner, nullptr, own, nullptr);
    pthread_join(first, nullptr);
    pthread_join(second, nullptr);
    pthread_join(owner, nullptr);

    // The end of the run: what the queue still holds is applied.
    inbox::g_step = 2;
    inbox::drain(*g_queue);

    (void)inbox::applied_once_if_taken(1, 1, g_taken[0]);
    (void)inbox::applied_once_if_taken(2, 1, g_taken[1]);
    assert(inbox::g_log_count == (g_taken[0] ? 1u : 0u) + (g_taken[1] ? 1u : 0u));

    // Every segment still held goes back, and the retired ones with them.
    g_queue->~Queue();
    return 0;
}
