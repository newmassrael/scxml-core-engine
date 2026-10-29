// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The building blocks of the `observer` kind (SCE_FORGE.md Section 4.11): a hysteresis state and a
// queue of the events an observer raises. Siblings: `com.sce.forge.runtime.ThresholdState` and
// `EventQueue` (Kotlin), `sce_forge_runtime::observer` (Rust) and `scealgorithm` (Go).

/// A 1-bit hysteresis state machine.
///
/// The generated `update` calls `enterIf` and `leaveIf`, each of which answers `true` exactly when
/// a transition happened, so the code that raises the event need not look at the state again.
public final class ThresholdState {
    private var isActive = false

    public init() {}

    /// Whether the state is entered.
    public var active: Bool { isActive }

    /// Enters the state where `condition` holds and it is not entered: answers `true` for that
    /// transition and `false` for every other call.
    @discardableResult
    public func enterIf(_ condition: Bool) -> Bool {
        if !isActive && condition {
            isActive = true
            return true
        }
        return false
    }

    /// Leaves the state where `condition` holds and it is entered: answers `true` for that
    /// transition and `false` for every other call.
    @discardableResult
    public func leaveIf(_ condition: Bool) -> Bool {
        if isActive && condition {
            isActive = false
            return true
        }
        return false
    }

    /// Back to not entered.
    public func reset() {
        isActive = false
    }
}

/// The events one `update` raised, in the order it raised them.
///
/// A value, returned by value: the observer keeps none of it, and a caller that holds one holds
/// the events of one `update` and no other. `Tag` is the observer's own enumeration of the events
/// it can raise.
public struct EventQueue<Tag> {
    private var buffer: [Tag] = []

    public init() {}

    /// Appends `tag`. Answers `true`: a queue of this kind has no bound to refuse at.
    @discardableResult
    public mutating func push(_ tag: Tag) -> Bool {
        buffer.append(tag)
        return true
    }

    /// How many events are queued.
    public var size: Int { buffer.count }

    public var isEmpty: Bool { buffer.isEmpty }

    /// The event at `index`, in the order they were pushed.
    public subscript(index: Int) -> Tag { buffer[index] }

    /// The queued events, in order.
    public func asArray() -> [Tag] { buffer }

    /// Drops every queued event.
    public mutating func clear() {
        buffer.removeAll()
    }
}
