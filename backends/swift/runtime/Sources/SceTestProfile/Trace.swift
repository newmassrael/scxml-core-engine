// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The recorded input trace of a test case and its replay (docs/SCE_TTCN3_LANE.md §10;
// docs/SCE_TEST_PROFILE.md §3.2, §3.3, §12.4): the inputs of one run from the system under test, each
// at its logical instant, `schemas/sce-test-trace.v1.schema.json`, and the system adapter that plays
// them, `TraceAdapter`.
//
// A trace is one text of the JSON Canonicalization Scheme, so that it has one spelling and two traces
// are equal exactly when their bytes are: `{"inputs":[{"at","port","type","value"}, …],"schema",
// "suite","test_case"}`. `at` is `NOW` in microseconds as the text of its digits, `port` a system port
// of the test case's configuration, and `type` and `value` the record type's name and the value in the
// canonical form a message record of the test log writes (§12.5). The inputs stand in order of `at`;
// the inputs of one instant in the order they are appended to their ports' queues (§3.2 step 2).
//
// The value of an input is read by its type: the reader of the test case's configuration
// (`InputReader`, generated as `input` in the configuration's unit) knows which records each system
// port takes and reads each with the reader of its record (`FromCanonical`). A trace that does not fit
// is refused whole before the run starts, which then ends with `error` (P13).
//
// Cross-language siblings: `sce_rust_runtime::test::trace` (Rust), which reads and refuses the same
// texts in the same words, `com.sce.runtime.test.Trace` (Kotlin), `sce/include/test/Trace.h` (C++).

import SceRuntime

/// The name of the trace's shape, its `schema`.
public let testTraceSchema = "sce-test-trace.v1"

/// The stability status of the trace's wire schema, pinned to the schema file's own
/// `x-sce-schema-status` by a test of the Swift runtime and by `schema_file_declares_status` of the
/// Rust one. `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
public let testTraceSchemaStatus = "pre-release"

/// Why a trace, or a value of one, is not read: the caller's input, refused by name before anything of
/// the run happens.
public struct TraceRefusal: Error, Equatable, Sendable, CustomStringConvertible {
    /// Why.
    public let reason: String

    public init(_ reason: String) {
        self.reason = reason
    }

    public var description: String {
        reason
    }

    /// An input of the record type `typeName` on `systemPort`, which the configuration whose system
    /// ports are `systemPorts` does not take: the port is none of them, or its type's `in` list does not
    /// name the type (docs/SCE_TTCN3_LANE.md §10.2).
    public static func notAnInput(_ systemPort: String, _ typeName: String, _ systemPorts: [String])
        -> TraceRefusal
    {
        if systemPorts.contains(where: { FromCanonical.same($0, systemPort) }) {
            return TraceRefusal("'\(typeName)' is not on the `in` list of the type of the system port '\(systemPort)'")
        }
        return TraceRefusal("'\(systemPort)' is not a system port of the test case's configuration")
    }
}

/// The reader of the inputs of a test case's configuration: the message an input of the record type
/// `typeName` on the system port `systemPort` carries, its value read from the canonical form, or why it
/// is not one.
///
/// Generated as `input(systemPort:typeName:value:)` in the unit of a test configuration, which names the
/// records each of its system ports takes.
public typealias InputReader =
    (_ systemPort: String, _ typeName: String, _ value: JSONValue) throws(TraceRefusal) -> Message

/// The reader of a configuration with no system port: it refuses every input, naming its port.
public func noInputs(systemPort: String, typeName: String, value: JSONValue) throws(TraceRefusal) -> Message {
    throw TraceRefusal.notAnInput(systemPort, typeName, [])
}

/// One input of a trace.
public struct TraceInput: Equatable, Sendable {
    /// `NOW` when it is due.
    public var at: TestInstant
    /// The system port it arrives on.
    public var port: String
    /// The name of its record type, as the test log names it.
    public var messageType: String
    /// Its value, in the canonical form of the test log.
    public var value: JSONValue

    public init(at: TestInstant, port: String, messageType: String, value: JSONValue) {
        self.at = at
        self.port = port
        self.messageType = messageType
        self.value = value
    }
}

/// A recorded input trace of one test case of one suite.
public struct Trace: Equatable, Sendable {
    /// The suite's name.
    public var suite: String
    /// The test case's name.
    public var testCase: String
    /// The inputs, in order of `at`, those of one instant in the order they are appended to their
    /// ports.
    public var inputs: [TraceInput]

    public init(suite: String, testCase: String, inputs: [TraceInput]) {
        self.suite = suite
        self.testCase = testCase
        self.inputs = inputs
    }

    /// The trace as the one JSON value its canonical text writes.
    private func canonicalValue() -> JSONValue {
        let text = { (text: String) in JSONValue.text(text) }
        let inputs = self.inputs.map { input in
            JSONValue.object([
                (key: "at", value: text(String(input.at))),
                (key: "port", value: text(input.port)),
                (key: "type", value: text(input.messageType)),
                (key: "value", value: input.value),
            ])
        }
        return .object([
            (key: "schema", value: text(testTraceSchema)),
            (key: "suite", value: text(suite)),
            (key: "test_case", value: text(testCase)),
            (key: "inputs", value: .array(inputs)),
        ])
    }

    /// The trace in its one canonical text, the JSON Canonicalization Scheme, with no line feed after
    /// it.
    ///
    /// A value that holds a number past the range of a double, or an object that names a member twice,
    /// which no canonical value holds and `parse` refuses, stops the program, as the Rust runtime
    /// panics.
    public func canonicalText() -> String {
        do {
            return try writeCanonicalJSON(canonicalValue())
        } catch {
            // The refusal describes itself by its reason (`NotCanonical.description`).
            fatalError("the trace of \(suite) \(testCase) is not canonical: \(error)")
        }
    }

    /// The trace `text` holds: the JSON Canonicalization Scheme of one trace, which a file may follow
    /// with one line feed, as a text file ends.
    ///
    /// Whether each input fits the suite is the configuration's to read (`TraceAdapter.replaying`).
    ///
    /// - Throws: `TraceRefusal` naming why, where the text is not JSON, the shape is not
    ///   `sce-test-trace.v1`, the inputs are out of the order of their instants, or the text is not the
    ///   trace's one canonical spelling.
    public static func parse(_ text: String) throws(TraceRefusal) -> Trace {
        var bytes = Array(text.utf8)
        if bytes.last == UInt8(ascii: "\n") {
            bytes.removeLast()
        }
        let body = String(decoding: bytes, as: UTF8.self)
        let value: JSONValue
        do {
            value = try parseJSON(body)
        } catch {
            // The reader's refusal describes itself (`JSONError.description`).
            throw TraceRefusal("the trace is \(error)")
        }
        let envelope = try members(value, ["schema", "suite", "test_case", "inputs"], "the trace")
        guard case .text(let schema) = envelope[0], FromCanonical.same(schema, testTraceSchema) else {
            throw TraceRefusal("the trace's schema is not \(testTraceSchema)")
        }
        guard case .array(let items) = envelope[3] else {
            throw TraceRefusal("the trace's inputs are not an array")
        }
        var read: [TraceInput] = []
        read.reserveCapacity(items.count)
        for (index, item) in items.enumerated() {
            let what = "input \(index)"
            let input = try members(item, ["at", "port", "type", "value"], what)
            let at = try instantOf(input[0], "the instant of \(what)")
            if let before = read.last, before.at > at {
                throw TraceRefusal(
                    "\(what), at \(at), comes after an input at \(before.at): "
                        + "the inputs stand in order of their instants")
            }
            read.append(
                TraceInput(
                    at: at,
                    port: try nameOf(input[1], "the port of \(what)"),
                    messageType: try nameOf(input[2], "the type of \(what)"),
                    value: input[3]))
        }
        let trace = Trace(
            suite: try nameOf(envelope[1], "the trace's suite"),
            testCase: try nameOf(envelope[2], "the trace's test case"),
            inputs: read)
        // A value with no canonical text, a number past the range of a double among them, is a trace
        // not written in it either: it is refused here as one, never handed to `canonicalText`, which
        // holds that a trace has that text.
        let canonical = try? writeCanonicalJSON(trace.canonicalValue())
        guard let canonical, FromCanonical.same(canonical, body) else {
            throw TraceRefusal(
                "the trace is not written in its one canonical text, the JSON Canonicalization Scheme")
        }
        return trace
    }

    /// The members of `value`, an object whose members are `names` exactly, each in the order of
    /// `names`.
    ///
    /// - Throws: `TraceRefusal` naming `what` otherwise.
    private static func members(_ value: JSONValue, _ names: [String], _ what: String) throws(TraceRefusal)
        -> [JSONValue]
    {
        guard case .object(let members) = value else {
            throw TraceRefusal("\(what) is not an object")
        }
        if let stray = members.first(where: { member in !names.contains { FromCanonical.same($0, member.key) } }) {
            throw TraceRefusal("\(what) has no member '\(stray.key)'")
        }
        var found: [JSONValue] = []
        for name in names {
            guard let held = value.member(name) else {
                throw TraceRefusal("\(what) has no '\(name)'")
            }
            found.append(held)
        }
        return found
    }

    /// The text `value`, which is not empty, at `what`.
    ///
    /// - Throws: `TraceRefusal` otherwise.
    private static func nameOf(_ value: JSONValue, _ what: String) throws(TraceRefusal) -> String {
        guard case .text(let name) = value, !name.utf8.isEmpty else {
            throw TraceRefusal("\(what) is not a name")
        }
        return name
    }

    /// An instant: the text of the decimal digits of a count of microseconds from the start of the test
    /// case, with no leading zero, within `NOW`'s range.
    ///
    /// - Throws: `TraceRefusal` otherwise.
    private static func instantOf(_ value: JSONValue, _ what: String) throws(TraceRefusal) -> TestInstant {
        guard case .text(let text) = value, let digits = FromCanonical.decimal(text), !digits.hasPrefix("-")
        else {
            throw TraceRefusal("\(what) is not an instant, the text of the digits of a count of microseconds")
        }
        guard let count = TestInstant(digits) else {
            throw TraceRefusal("\(what) (\(digits)) is past the microseconds the clock counts to")
        }
        return count
    }
}

/// What a component sent to a system port, as a `TraceAdapter` records it.
public struct SystemOutput: Sendable {
    /// `NOW` when it was sent.
    public var at: TestInstant
    /// The system port it was sent to.
    public var systemPort: String
    /// The message.
    public var message: Message

    public init(at: TestInstant, systemPort: String, message: Message) {
        self.at = at
        self.systemPort = systemPort
        self.message = message
    }
}

/// A system adapter that plays a trace (docs/SCE_TTCN3_LANE.md §10.5): at each instant it delivers the
/// inputs due then, in the trace's order, to their system ports (docs/SCE_TEST_PROFILE.md §3.2 step 2),
/// and it records what the components send to the system ports.
///
/// It announces the instant of its next input (`nextInput`), so when no branch can fire the run's clock
/// advances to the earlier of that instant and the next timer expiry or the guard (§3.3, §12.4 step 4),
/// and the run is the same whenever it is played.
///
/// A class, as every system adapter is: the run holds it while the caller keeps it, and reads what was
/// sent (`sent`) after the run.
public final class TraceAdapter: SystemAdapter {
    /// The inputs read and waiting for their instants, in the trace's order.
    private let due: [SystemInput]
    /// The instant of each input of `due`, at the same index.
    private let instants: [TestInstant]
    /// How many inputs of `due` are taken.
    private var taken = 0

    /// What the components have sent to the system ports, in the order they sent it.
    public private(set) var sent: [SystemOutput] = []

    private init(due: [SystemInput], instants: [TestInstant]) {
        self.due = due
        self.instants = instants
    }

    /// The player of `trace`, each input's value read by `read`, the reader of the test case's
    /// configuration.
    ///
    /// - Throws: `TraceRefusal` naming the input, where one does not fit the configuration: its port is
    ///   no system port of it, its type is not on the `in` list of that port's type, or its value does
    ///   not belong to its type (docs/SCE_TTCN3_LANE.md §10.2). The whole trace is refused.
    public static func replaying(_ trace: Trace, _ read: InputReader) throws(TraceRefusal) -> TraceAdapter {
        var due: [SystemInput] = []
        var instants: [TestInstant] = []
        for (index, input) in trace.inputs.enumerated() {
            let message: Message
            do throws(TraceRefusal) {
                message = try read(input.port, input.messageType, input.value)
            } catch {
                throw TraceRefusal("input \(index), at \(input.at): \(error.reason)")
            }
            due.append(SystemInput(systemPort: input.port, message: message))
            instants.append(input.at)
        }
        return TraceAdapter(due: due, instants: instants)
    }

    public func send(systemPort: String, message: Message, now: TestInstant) {
        sent.append(SystemOutput(at: now, systemPort: systemPort, message: message))
    }

    public func nextInput() -> TestInstant? {
        taken < instants.count ? instants[taken] : nil
    }

    public func takeInputsDue(now: TestInstant) -> [SystemInput] {
        var inputs: [SystemInput] = []
        while taken < instants.count && instants[taken] <= now {
            inputs.append(due[taken])
            taken += 1
        }
        return inputs
    }
}
