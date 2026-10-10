// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh EventQueueBridge — bounded queue with many producers and one
// consumer, for cross-thread event injection.
//
// It is the forge runtime's SCQ data queue (sce/forge/queue.h; the queue kind's
// migration rule in the SCE Protocol-Synthesis RFC, section 5.P), not a queue of
// the mesh's own.
// The queue it replaces was Dmitry Vyukov's bounded MPSC ring, which the header
// called lock-free and which is not, formally: a producer that has claimed a
// cell and then stalls hides every later cell from the consumer until it
// resumes. SCQ is lock-free on both sides: a stalled participant never hides
// what others have pushed.
//
// Consumer: `ShmChannel` places this queue in shared memory for the shm
// transport's control ring. SCQ's rings carry indices, never pointers, and its
// elements are stored inline, so the queue means the same in every process
// that maps it.

#pragma once

#include "sce/forge/queue.h"

#include <cstddef>

namespace SCE::Mesh {

/// Bounded lock-free queue for cross-thread event delivery, for many
/// producers and one consumer.
///
/// @tparam T        Event type (nothrow-move-constructible and
///                  nothrow-destructible)
/// @tparam Capacity Queue depth, a power of 2 (default: 256). It is also the
///                  ring size, so up to `Capacity` producer handles and as many
///                  consumer handles may be alive at once.
///
/// Operations go through handles, which keep the number of concurrent
/// participants within what the ring's empty test is justified for:
///
///   producer()        — a producing handle, or `std::nullopt` when `Capacity`
///                       are alive; give it back by destroying it.
///   Producer::try_push(T&&) -> PushStatus
///                     — `Ok` moves the event in; `Full` leaves it untouched.
///   consumer()        — the consuming handle. Exactly one thread pops, so the
///                       engine still sees one event at a time and the W3C
///                       run-to-completion guarantee holds without a lock on
///                       the processing path (SCE_MESH.md section 10.3: this
///                       is where concurrent transport deliveries are
///                       serialised into one instance's queue).
///   Consumer::try_pop() -> std::optional<T>
///
/// A handle is process-local state and must not be stored in the queue's own
/// storage: a queue in shared memory is used through handles each process
/// holds for itself.
template <typename T, std::size_t Capacity = 256>
using EventQueueBridge = ::SCE::Forge::Queue::Scq<T, Capacity, Capacity>;

}  // namespace SCE::Mesh
