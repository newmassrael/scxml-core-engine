// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of received messages and
// the snapshot of its head (ES 201 873-4 §8.3.3), and the logical clock's instants and lengths.
//
// The other runtimes call the port `Port` and the length `Duration`. Swift has a `Duration` of its
// own, which a module that declares another would hide from every module that imports both, and
// Foundation declares a `Port`; so the profile's are `TestPort`, `TestInstant` and `TestDuration`,
// named as `TestCase` and `TestCaseRun` are.
//
// Cross-language siblings: `sce_rust_runtime::test::Port` (Rust), `SCE::Test::Port` (C++),
// `com.sce.runtime.test.Port` (Kotlin), `testprofile.Port` (Go), `sce_runtime.testprofile.Port`
// (Python), `sce_test_port_t` (C11).

/// An instant of the logical clock `NOW`: microseconds since the test case started
/// (docs/SCE_TEST_PROFILE.md §3.3).
public typealias TestInstant = Int64

/// A length of logical time, in microseconds.
public typealias TestDuration = Int64

/// A message on a port: the event name of its record and the record's value.
///
/// The value is the record type the statechart generator writes for the `event-schema` that declares
/// it; a template's `matches` reads it as that type (`receives`). Matching is on the value and never
/// on an encoding (ADR 0091 C2). The value is a Swift value, never changed once sent, so a message is
/// shared freely: every holder reads the same record.
public struct Message: Sendable {
    /// The event name of the message's record.
    public let event: String
    /// The record's value, read back by its type (`valueAs`).
    public let value: any Sendable

    public init(event: String, value: any Sendable) {
        self.event = event
        self.value = value
    }

    /// The value, where it is a `V`; nil where it is of another type, which a template of `V` does
    /// not match.
    public func valueAs<V>(_ type: V.Type) -> V? {
        value as? V
    }
}

/// One component port: its queue and its snapshot (-4 §8.3.3).
///
/// Only the component that owns the port removes from it, so between the snapshot and the removal of
/// a branch the head is the message the snapshot saw; what others append goes to the back. The
/// snapshot records only whether the queue held a message: a message that arrives in an empty queue
/// after the snapshot is not in it. A new port is empty, and its snapshot saw nothing.
public final class TestPort {
    private var queue: [Message] = []
    private var snapshotHolds = false

    public init() {}

    /// A message a connected port sent, or one from the SUT on a mapped port, appended in arrival order
    /// (-4 §8.3.3.1).
    public func enqueue(_ message: Message) {
        queue.append(message)
    }

    /// `SNAP-VALUE`: the head of the queue, taken in a round's snapshot step
    /// (docs/SCE_TEST_PROFILE.md §3.2 step 3).
    public func takeSnapshot() {
        snapshotHolds = !queue.isEmpty
    }

    /// The message the last snapshot saw at the head, or nil when it saw none.
    public func snapshot() -> Message? {
        snapshotHolds ? queue.first : nil
    }

    /// `remove-from-port` of a receive branch that was taken on the snapshot (-4 §9.37.1, P4 step 1).
    ///
    /// The caller holds that the snapshot saw a message, which a branch's guard established, so an empty
    /// queue here is a machine that took a receive branch its guard did not allow, and it stops the
    /// program, as the Rust runtime panics.
    public func removeHead() -> Message {
        guard !queue.isEmpty else {
            fatalError("a receive branch is taken only on a snapshot that saw a message")
        }
        snapshotHolds = false
        return queue.removeFirst()
    }

    /// How many messages the queue holds.
    public var count: Int {
        queue.count
    }

    /// Whether the queue holds no message.
    public var isEmpty: Bool {
        queue.isEmpty
    }
}
