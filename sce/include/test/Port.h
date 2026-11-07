// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of
// received messages and the snapshot of its head (ES 201 873-4 §8.3.3).

#pragma once

#include <any>
#include <cstdint>
#include <deque>
#include <optional>
#include <string>
#include <utility>

namespace SCE::Test {

/// An instant of the logical clock `NOW`: microseconds since the test case
/// started (docs/SCE_TEST_PROFILE.md §3.3).
using Instant = std::int64_t;

/// A length of logical time, in microseconds.
using Duration = std::int64_t;

/**
 * @brief A message on a port: the event name of its record and the record's value
 *
 * The value is the record type the statechart generator writes for the
 * `event-schema` that declares it; a template's matcher reads it as that type.
 * Matching is on the value and never on an encoding (ADR 0091 C2).
 */
struct Message {
    std::string event;
    std::any value;
};

/**
 * @brief One component port: its queue and its snapshot (-4 §8.3.3)
 *
 * Only the component that owns the port removes from it, so between the
 * snapshot and the removal of a branch the head is the message the snapshot
 * saw; what others append goes to the back. The snapshot records only whether
 * the queue held a message: a message that arrives in an empty queue after the
 * snapshot is not in it.
 */
class Port {
public:
    /// A message a connected port sent, or one from the SUT on a mapped port,
    /// appended in arrival order (-4 §8.3.3.1).
    void enqueue(Message message) {
        queue_.push_back(std::move(message));
    }

    /// `SNAP-VALUE`: the head of the queue, taken in a round's snapshot step
    /// (docs/SCE_TEST_PROFILE.md §3.2 step 3).
    void takeSnapshot() {
        snapshotHolds_ = !queue_.empty();
    }

    /// The message the last snapshot saw at the head, or none.
    const Message *snapshot() const {
        return snapshotHolds_ ? &queue_.front() : nullptr;
    }

    /// `remove-from-port` of a receive branch that was taken on the snapshot
    /// (-4 §9.37.1, P4 step 1). The caller holds that the snapshot saw a
    /// message, which a branch's guard established.
    Message removeHead() {
        Message head = std::move(queue_.front());
        queue_.pop_front();
        snapshotHolds_ = false;
        return head;
    }

    std::size_t size() const {
        return queue_.size();
    }

private:
    std::deque<Message> queue_;
    bool snapshotHolds_ = false;
};

}  // namespace SCE::Test
