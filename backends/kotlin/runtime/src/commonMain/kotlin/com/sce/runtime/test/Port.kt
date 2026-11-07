// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of
// received messages and the snapshot of its head (ES 201 873-4 §8.3.3).
//
// Cross-language siblings: `sce_rust_runtime::test::Port` (Rust),
// `SCE::Test::Port` (C++).

package com.sce.runtime.test

/**
 * An instant of the logical clock `NOW`: microseconds since the test case
 * started (docs/SCE_TEST_PROFILE.md §3.3).
 */
typealias Instant = Long

/** A length of logical time, in microseconds. */
typealias Duration = Long

/**
 * A message on a port: the event name of its record and the record's value.
 *
 * The value is the record type the statechart generator writes for the
 * `event-schema` that declares it; a template's matcher reads it as that type
 * ([valueAs]). Matching is on the value and never on an encoding (ADR 0091 C2).
 *
 * The value is held as it is: the Rust runtime shares it through an `Arc` so
 * that the request that carries it stays cloneable, and every Kotlin value is
 * already a shared reference, which is never changed once sent.
 */
class Message(
    /** The event name of the message's record. */
    val event: String,
    /** The record's value, read back by its type. */
    val value: Any?,
) {
    /**
     * The value, when it is a [V]; null when it is of another type, which a
     * template of [V] does not match.
     */
    inline fun <reified V : Any> valueAs(): V? = value as? V

    // The value's type is not known here, so only the event is shown.
    override fun toString(): String = "Message(event=$event, ..)"
}

/**
 * One component port: its queue and its snapshot (-4 §8.3.3).
 *
 * Only the component that owns the port removes from it, so between the
 * snapshot and the removal of a branch the head is the message the snapshot
 * saw; what others append goes to the back. The snapshot records only whether
 * the queue held a message: a message that arrives in an empty queue after the
 * snapshot is not in it.
 */
class Port {
    private val queue = ArrayDeque<Message>()
    private var snapshotHolds = false

    /**
     * A message a connected port sent, or one from the SUT on a mapped port,
     * appended in arrival order (-4 §8.3.3.1).
     */
    fun enqueue(message: Message) {
        queue.addLast(message)
    }

    /**
     * `SNAP-VALUE`: the head of the queue, taken in a round's snapshot step
     * (docs/SCE_TEST_PROFILE.md §3.2 step 3).
     */
    fun takeSnapshot() {
        snapshotHolds = queue.isNotEmpty()
    }

    /** The message the last snapshot saw at the head, or null. */
    fun snapshot(): Message? = if (snapshotHolds) queue.firstOrNull() else null

    /**
     * `remove-from-port` of a receive branch that was taken on the snapshot
     * (-4 §9.37.1, P4 step 1).
     *
     * @throws IllegalStateException when the queue is empty. The caller holds
     *   that the snapshot saw a message, which a branch's guard established, so
     *   an empty queue here is a machine that took a receive branch its guard
     *   did not allow.
     */
    fun removeHead(): Message {
        val head = checkNotNull(queue.removeFirstOrNull()) {
            "a receive branch is taken only on a snapshot that saw a message"
        }
        snapshotHolds = false
        return head
    }

    /** How many messages the queue holds. */
    val size: Int
        get() = queue.size

    /** Whether the queue holds no message. */
    fun isEmpty(): Boolean = queue.isEmpty()
}
