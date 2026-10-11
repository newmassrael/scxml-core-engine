// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/* THREADS: 2 */
/* REQUIRED: release seq_cst acquire_release all */

// What this harness catches, measured 2026-10-11 on GenMC v0.19.0 (RC11): the
// baseline explores 940 complete executions with no error, and every release
// weakened to relaxed (`release`, which includes the store that gives a slot
// back), every sequentially consistent ordering (`seq_cst`: the publication, the
// re-check and the compare-and-swap that unlinks), the acquire/release pair
// (`acquire_release`) and every ordering (`all`) are each reported as an attempt
// to access freed memory. The acquire loads weakened alone (`acquire`) and the
// acquire-release orderings of a read-modify-write (`acq_rel`) survived this
// harness; they carry no hand-over in the one scenario here, so none is listed as
// required, and the gate prints each one that survives.
//
// The baseline explores more executions than the C harness of the same name (9)
// because the C++ domain's retired list is pushed with `compare_exchange_weak`,
// which GenMC could not strengthen to a strong one (it says so, once per call
// site, in its warnings): each weak CAS may fail spuriously, and each failure is
// an execution.

// Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
// §synth-5-P): the C++ hazard-pointer domain of the segmented queues under GenMC.
// It is the C harness of the same name
// (backends/c/forge-runtime/tests/genmc/hazard_keeps_a_named_node.c) over the C++
// runtime as it ships.
//
// The domain's contract is that a node a participant has protected is not freed
// while it is named, whatever the other participants do. It rests on one
// handshake: a participant publishes the node it read with a sequentially
// consistent store and re-reads the location with a sequentially consistent load;
// whoever unlinks a node does so with a sequentially consistent operation before
// the retirement, and the scan reads the hazards with a sequentially consistent
// load after that. Either the scan sees the hazard and keeps the node, or the
// participant's re-read sees the location already moved and does not use the
// node. The C++ runtime uses no fence (the C one does): every operation in the
// argument is itself sequentially consistent (queue_segmented.h, *Ordering*).
//
// The harness is the smallest program that has both sides. A location holds a
// node. One thread protects it, reads the node and gives the slot back. The other
// installs a second node with the sequentially consistent compare-and-swap the
// contract gives the unlinking side, retires the first and runs the scan. A node
// read after it was freed is a failure GenMC reports, and the contract is what
// keeps it from happening.

#include <atomic>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <new>
#include <optional>

#include <pthread.h>

#include <sce/forge/queue.h>
#include <sce/forge/queue_segmented.h>

#include "genmc_cxx_support.h"

namespace {

using SCE::Forge::Queue::HazardDomain;
using SCE::Forge::Queue::Retired;

// A node the domain can retire: the header first, as the domain requires. Built
// by hand in memory GenMC allocates, and not with `new`.
//
// `__VERIFIER_malloc` and `__VERIFIER_free`, which GenMC's <pthread.h> declares
// through genmc_internal.h, and neither <cstdlib> nor `malloc`. <cstdlib> reaches
// the system <stdlib.h> and so glibc's <sys/types.h>, whose `pthread_t` and the
// rest collide with the ones GenMC's own <pthread.h> defines ("typedef
// redefinition with different types"); and a bare `malloc` call, which
// `__builtin_malloc` makes, is an unknown external function to the interpreter
// ("Tried to execute an unknown external function: malloc"): GenMC's own
// <stdlib.h> is what turns `malloc` into the primitive, and a C++ source never
// reaches it (both measured 2026-10-11). Calling the primitive is what that
// header does.
struct Node : Retired {
    std::uint64_t data;
};

HazardDomain<2> g_domain;

// The location the nodes are installed in.
std::atomic<Node *> g_source{nullptr};

Node *g_first = nullptr;
Node *g_second = nullptr;

void reclaim(Retired *node, void *) noexcept {
    __VERIFIER_free(node);
}

Node *make(std::uint64_t data) {
    void *memory = __VERIFIER_malloc(sizeof(Node));
    Node *node = ::new (memory) Node();
    node->data = data;
    return node;
}

void *read_the_node(void *) {
    auto hazard = g_domain.acquire();
    if (!hazard) {
        return nullptr;
    }
    Node *node = hazard->protect(g_source);
    // Whichever node was read, it is whole and not yet freed.
    assert(node->data == 1u || node->data == 2u);
    return nullptr;
}

void *replace_the_node(void *) {
    Node *expected = g_first;
    // The sequentially consistent operation the contract gives the unlinking
    // side, before the retirement.
    if (g_source.compare_exchange_strong(expected, g_second, std::memory_order_seq_cst, std::memory_order_seq_cst)) {
        g_domain.retire(g_first, reclaim, nullptr);
        g_domain.collect();
    }
    return nullptr;
}

}  // namespace

int main() {
    pthread_t reader;
    pthread_t replacer;

    g_first = make(1);
    g_second = make(2);
    g_source.store(g_first, std::memory_order_relaxed);

    pthread_create(&reader, nullptr, read_the_node, nullptr);
    pthread_create(&replacer, nullptr, replace_the_node, nullptr);
    pthread_join(reader, nullptr);
    pthread_join(replacer, nullptr);

    // What the scan kept and what is still installed go back.
    g_domain.flush();
    __VERIFIER_free(g_second);
    return 0;
}
