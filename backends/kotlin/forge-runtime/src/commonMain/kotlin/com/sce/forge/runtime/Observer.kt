// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package com.sce.forge.runtime

/**
 * Observer building blocks: hysteresis state, domain-tagged events, event
 * queue. See SCE_FORGE.md Section 4.11.
 */

/**
 * Marker interface for an event domain. Each generated observer is
 * parameterized over a domain whose `Tag` enum lists the events valid in
 * that domain. Different domains produce incompatible queue/event types —
 * this is the type-safety mechanism for cross-file event composition.
 */
public interface EventDomain<Tag : Enum<Tag>>

/**
 * Models a 1-bit hysteresis state machine. The generated `update()` loop
 * calls [enterIf] and [leaveIf]; both return `true` exactly when a
 * transition actually occurred, so the generated code can push the
 * corresponding event without re-checking state.
 */
public class ThresholdState {
    private var _active = false
    public val active: Boolean get() = _active

    public fun enterIf(condition: Boolean): Boolean {
        if (!_active && condition) {
            _active = true
            return true
        }
        return false
    }

    public fun leaveIf(condition: Boolean): Boolean {
        if (_active && condition) {
            _active = false
            return true
        }
        return false
    }

    public fun reset() {
        _active = false
    }
}

/**
 * FIFO of domain-tagged events. Returned by value from observer `update()`
 * methods. Backed by a list — the JVM has no embedded heap constraint, so a
 * list is the natural data structure here. The cross-language behavioural
 * contract (push, size, iteration order, clear) matches the C++ and Rust
 * implementations exactly.
 */
public class EventQueue<Tag : Enum<Tag>> {
    private val buffer = mutableListOf<Tag>()

    public fun push(tag: Tag): Boolean {
        buffer.add(tag)
        return true
    }

    public val size: Int get() = buffer.size
    public fun isEmpty(): Boolean = buffer.isEmpty()
    public operator fun get(index: Int): Tag = buffer[index]
    public fun asList(): List<Tag> = buffer.toList()
    public fun clear() {
        buffer.clear()
    }
}
