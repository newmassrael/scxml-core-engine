// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The test log of a test case run (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3 and answer 8): the
// records of TCI's test logging interface (ETSI ES 201 873-6), each written where the profile's semantics
// places its event, in the order the round scheduler produces them, and the one canonical text of a run's
// log, `schemas/sce-test-log.v1.schema.json`.
//
// The text is the JSON Canonicalization Scheme of the log (`writeCanonicalJSON`), and it holds no number:
// an instant, a duration and every integer of a value are texts of their digits (`Canonical`). So two
// runs agree, under C3, exactly when their logs are the same bytes, whichever runtime wrote each.
//
// Cross-language siblings: `sce_rust_runtime::test::TestLogRecord` and `TestCaseResult::canonical_log`
// (Rust), whose text this one writes byte for byte, `sce/include/test/TestLog.h` (C++),
// `com.sce.runtime.test.TestLogRecord` (Kotlin).

import SceRuntime

/// The name of the test log's shape, its `schema`.
public let testLogSchema = "sce-test-log.v1"

/// An event of TCI's test logging interface (ES 201 873-6), as the profile's semantics places it
/// (docs/SCE_TEST_PROFILE.md §12.5).
///
/// A component is named `mtc` or by the PTC's name in its configuration.
public enum TestLogEvent: Equatable, Sendable {
    /// The test case started, at `NOW = 0` (P11).
    case testCaseStarted
    /// The test case ended with its verdict (P14).
    case testCaseEnded(verdict: Verdict)
    /// A component of the type `componentType` was created: the MTC as the test case starts, a PTC by
    /// the invocation that creates it (P9).
    case componentCreated(component: String, componentType: String)
    /// A component terminated, with its local verdict then (P10).
    case componentDone(component: String, verdict: Verdict)
    /// A component sent a message on a port (P6): its record type's name (`TestValue.typeName`) and its
    /// value in canonical form.
    case messageSent(component: String, port: String, messageType: String, value: JSONValue)
    /// A receive branch took the head of a port's queue (P4 step 1): the message was received and
    /// matched.
    case messageReceived(component: String, port: String, messageType: String, value: JSONValue)
    /// A component started a timer, for a duration in microseconds (P7).
    case timerStarted(component: String, timer: String, duration: TestDuration)
    /// A component stopped a running timer (P7). A stop of a timer that is not running changes nothing,
    /// and is not recorded.
    case timerStopped(component: String, timer: String)
    /// A timeout branch took a timer that had timed out (P4 step 1).
    case timerTimedOut(component: String, timer: String)
    /// A component's local verdict was set: by `<sce:setverdict>` (P12), or to `error` by the runtime
    /// (P13). The reason is the document's, and none is recorded for an empty one or for the runtime's
    /// `error`, whose cause is the runtime's own prose, which another runtime would not write alike.
    case verdictSet(component: String, verdict: Verdict, reason: String?)
    /// A `<log>` (P8).
    case log(component: String, text: String)

    /// The record's `kind` in the test log: the TCI event in kebab case.
    public var kind: String {
        switch self {
        case .testCaseStarted: return "test-case-started"
        case .testCaseEnded: return "test-case-ended"
        case .componentCreated: return "component-created"
        case .componentDone: return "component-done"
        case .messageSent: return "message-sent"
        case .messageReceived: return "message-received"
        case .timerStarted: return "timer-started"
        case .timerStopped: return "timer-stopped"
        case .timerTimedOut: return "timer-timed-out"
        case .verdictSet: return "verdict-set"
        case .log: return "log"
        }
    }
}

/// One member of a record or of the log, named `name`.
private func member(_ name: String, _ value: JSONValue) -> (key: String, value: JSONValue) {
    (key: name, value: value)
}

/// One member of a record or of the log whose value is the text `text`.
private func textMember(_ name: String, _ text: String) -> (key: String, value: JSONValue) {
    member(name, Canonical.text(text))
}

/// One record of the test log: an event and `NOW` when it happened.
public struct TestLogRecord: Equatable, Sendable {
    /// `NOW` when it was recorded.
    public var at: TestInstant
    /// What happened.
    public var event: TestLogEvent

    public init(at: TestInstant, event: TestLogEvent) {
        self.at = at
        self.event = event
    }

    /// The record as the test log writes it: `at`, `kind`, and the members of its kind.
    public func toValue() -> JSONValue {
        var members = [member("at", Canonical.integer(at)), textMember("kind", event.kind)]
        switch event {
        case .testCaseStarted:
            break
        case .testCaseEnded(let verdict):
            members.append(textMember("verdict", verdict.spelling))
        case .componentCreated(let component, let componentType):
            members.append(textMember("component", component))
            members.append(textMember("type", componentType))
        case .componentDone(let component, let verdict):
            members.append(textMember("component", component))
            members.append(textMember("verdict", verdict.spelling))
        case .messageSent(let component, let port, let messageType, let value),
            .messageReceived(let component, let port, let messageType, let value):
            members.append(textMember("component", component))
            members.append(textMember("port", port))
            members.append(textMember("type", messageType))
            members.append(member("value", value))
        case .timerStarted(let component, let timer, let duration):
            members.append(textMember("component", component))
            members.append(textMember("timer", timer))
            members.append(member("duration_us", Canonical.integer(duration)))
        case .timerStopped(let component, let timer), .timerTimedOut(let component, let timer):
            members.append(textMember("component", component))
            members.append(textMember("timer", timer))
        case .verdictSet(let component, let verdict, let reason):
            members.append(textMember("component", component))
            members.append(textMember("verdict", verdict.spelling))
            if let reason {
                members.append(textMember("reason", reason))
            }
        case .log(let component, let text):
            members.append(textMember("component", component))
            members.append(textMember("text", text))
        }
        return .object(members)
    }
}

/// The test log of the run of `testCase` of the suite `suite`, which ended with `verdict` and wrote
/// `records`, in its one canonical text: the JSON Canonicalization Scheme of `schema`, `suite`,
/// `test_case`, `verdict` and `records`, each record in the order the run produced it.
///
/// A message's value that is not in the canonical form (`TestValue.canonical`) but holds a number past
/// the range of a double, or an object that names a member twice, which no writer of `Canonical`
/// builds, stops the program, as the Rust runtime panics.
func writeTestLog(_ suite: String, _ testCase: String, _ verdict: Verdict, _ records: [TestLogRecord])
    -> String
{
    let log = JSONValue.object([
        textMember("schema", testLogSchema),
        textMember("suite", suite),
        textMember("test_case", testCase),
        textMember("verdict", verdict.spelling),
        member("records", .array(records.map { $0.toValue() })),
    ])
    do {
        return try writeCanonicalJSON(log)
    } catch {
        // The refusal describes itself by its reason (`NotCanonical.description`).
        fatalError("the test log of \(suite) \(testCase) is not canonical: \(error)")
    }
}
