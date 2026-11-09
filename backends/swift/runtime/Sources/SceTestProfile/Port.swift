// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of received messages and
// the snapshot of its head (ES 201 873-4 §8.3.3), a message and the value it carries with its type's
// name and canonical form (§12.5), and the logical clock's instants and lengths.
//
// The other runtimes call the port `Port` and the length `Duration`. Swift has a `Duration` of its
// own, which a module that declares another would hide from every module that imports both, and
// Foundation declares a `Port`; so the profile's are `TestPort`, `TestInstant` and `TestDuration`,
// named as `TestCase` and `TestCaseRun` are.
//
// Cross-language siblings: `sce_rust_runtime::test::Port` (Rust), `SCE::Test::Port` (C++),
// `com.sce.runtime.test.Port` (Kotlin), `testprofile.Port` (Go), `sce_runtime.testprofile.Port`
// (Python), `sce_test_port_t` (C11).

import SceRuntime

/// An instant of the logical clock `NOW`: microseconds since the test case started
/// (docs/SCE_TEST_PROFILE.md §3.3).
public typealias TestInstant = Int64

/// A length of logical time, in microseconds.
public typealias TestDuration = Int64

/// A value a message carries, its type's name and its canonical form in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5).
///
/// The value is read back by its type (`Message.valueAs`); its canonical form is written by the types
/// its document declares (`Canonical`), which its Swift type alone does not say. `messageValue` makes
/// one of a value, its type's name and the writer of its form, which is what a generated send and a
/// system adapter give; a host may conform a type of its own.
///
/// Cross-language siblings: `sce_rust_runtime::test::TestValue` (Rust), `SCE::Test::MessageValue`
/// (C++), `com.sce.runtime.test.TestValue` (Kotlin).
public protocol TestValue: Sendable {
    /// The value, to be read back by its type.
    func asAny() -> any Sendable

    /// The name of the value's record as the types module of the suite's TTCN-3 export names it
    /// (docs/SCE_TEST_PROFILE.md §9.5): the unit name of its `event-schema` document. A value in TCI
    /// always carries its type, and so does a message of the test log.
    var typeName: String { get }

    /// The value in the canonical form of the test log: built from texts, truth values, arrays and
    /// objects alone (`Canonical`), so that the JSON Canonicalization Scheme writes it as the same bytes
    /// on every runtime. It runs while the run's state is held, so it reads the value and nothing else.
    func canonical() -> JSONValue
}

/// A value, its type's name and the writer of its canonical form: what `messageValue` makes.
///
/// It is `Equatable` and `Sendable`, so a send's request carries it as its value (`HostSendValue`), and
/// two are equal where they hold equal values of one type under one type name: the writer is the
/// type's, so it is not compared.
public struct MessageValue<V: Equatable & Sendable>: TestValue, Equatable, CustomStringConvertible {
    /// The value.
    public let value: V
    public let typeName: String
    private let write: @Sendable (V) -> JSONValue

    public init(_ value: V, typeName: String, canonical: @escaping @Sendable (V) -> JSONValue) {
        self.value = value
        self.typeName = typeName
        self.write = canonical
    }

    public func asAny() -> any Sendable {
        value
    }

    public func canonical() -> JSONValue {
        write(value)
    }

    public static func == (lhs: MessageValue, rhs: MessageValue) -> Bool {
        lhs.typeName == rhs.typeName && lhs.value == rhs.value
    }

    // The value's type is not known to every reader, so only its type's name is shown.
    public var description: String {
        "TestValue(typeName=\(typeName), ..)"
    }
}

/// `value` as a message's value, of the record `typeName`, whose canonical form `canonical` writes: the
/// `TYPE` and the `canonical` of the unit of a template over its record, as a generated send gives them
/// (`<Template>.TYPE`, `<Template>.canonical`).
public func messageValue<V: Equatable & Sendable>(
    _ value: V, _ typeName: String, _ canonical: @escaping @Sendable (V) -> JSONValue
) -> MessageValue<V> {
    MessageValue(value, typeName: typeName, canonical: canonical)
}

/// A message on a port: the event name of its record and the record's value.
///
/// The value is of the record type the unit of its `event-schema` declares; a template's `matches`
/// reads it as that type (`receives`). Matching is on the value and never on an encoding (ADR 0091 C2),
/// and so is the test log (`canonicalValue`). The value is a Swift value, never changed once sent, so a
/// message is shared freely: every holder reads the same record.
public struct Message: Sendable {
    /// The event name of the message's record.
    public let event: String
    /// The record's value, with its type's name and the writer of its canonical form.
    public let value: any TestValue

    public init(event: String, value: any TestValue) {
        self.event = event
        self.value = value
    }

    /// A message of the record `event` whose value is `value`, of the record type `typeName`, whose
    /// canonical form `canonical` writes (`messageValue`).
    public static func of<V: Equatable & Sendable>(
        event: String, value: V, typeName: String, canonical: @escaping @Sendable (V) -> JSONValue
    ) -> Message {
        Message(event: event, value: messageValue(value, typeName, canonical))
    }

    /// The value, where it is a `V`; nil where it is of another type, which a template of `V` does
    /// not match.
    public func valueAs<V>(_ type: V.Type) -> V? {
        value.asAny() as? V
    }

    /// The value in the canonical form of the test log.
    public func canonicalValue() -> JSONValue {
        value.canonical()
    }

    /// The name of the value's record type in the test log (`TestValue.typeName`).
    public var typeName: String {
        value.typeName
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
