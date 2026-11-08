// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceTestProfile
import StaticDatamodelMachines
import XCTest

// Test components generated in Swift (docs/SCE_TEST_PROFILE.md §12.3, §12.6 slice 6), run end to end by
// the profile's runtime, `SceTestProfile.TestCaseRun`. The machines are the statecharts sce-codegen
// writes from the fixtures under `sce-build/tests/fixtures/test_profile/runtime/` into the target
// `StaticDatamodelMachines`, beside the records and templates they name (regen:
// `scripts/regen_static_datamodel_swift.sh`), driven only through the doors of §12.3; the
// configuration, the factory of machines and the system adapter are written here.
//
// Each test is the twin of the Rust case of the same name in
// `backends/rust/tests/tests/a_test_component_runs_through_its_doors.rs`, itself the twin of the C++
// gtest in `tests/integration/ATestComponentRunsThroughItsDoorsTest.cpp`, of the Kotlin
// `ATestComponentRunsThroughItsDoorsTest.kt`, of the Go `a_test_component_runs_through_its_doors_test.go`,
// of the Python `test_a_test_component_runs_through_its_doors.py` and of the C11
// `test_a_test_component_runs_through_its_doors.c`, with the same scenario, verdict, `NOW` and log.
// `param_asker`, which declares a module parameter, is run where the other backends run it, by the
// suite whose tables give its value (`ATestSuiteRunsThroughItsRunnerTests.swift`). What the generator
// writes for each component is held by `sce-build/tests/a_test_component_is_generated_in_swift.rs`.

/// A millisecond of `NOW`, which counts microseconds.
private let millisecond: TestInstant = 1000

/// A second of `NOW`.
private let second: TestInstant = 1000 * millisecond

/// How long the system under test takes to answer a request.
private let answerDelay: TestInstant = 500

/// The generated component of each component type a configuration here names, and none for another
/// type, which ends the run with `error`.
private func machine(of componentType: String) -> (any ComponentMachine)? {
    switch componentType {
    case "asker": return AskerComponent()
    case "coordinator": return CoordinatorComponent()
    case "answerer": return AnswererComponent()
    case "ptc_keeper": return PtcKeeperComponent()
    case "idler": return IdlerComponent()
    default: return nil
    }
}

/// One message the system under test was handed: its system port and its event.
private struct Sent: Equatable {
    let systemPort: String
    let event: String
}

/// What the system under test was handed: each send, and the job each request named.
private final class Seen {
    var sent: [Sent] = []
    var requested: [UInt32] = []
}

/// The system under test on the mapped port `sut`: it answers each request, `answerDelay` after it,
/// with a reply of `code` for the requested job, or for `job` when one is given. It records what it
/// was sent in `seen`, which the test keeps while the run holds the adapter.
private final class Responder: SystemAdapter {
    private let seen: Seen
    private let code: UInt32
    private let job: UInt32?
    private var pending: [(at: TestInstant, message: Message)] = []

    init(_ seen: Seen, code: UInt32, job: UInt32? = nil) {
        self.seen = seen
        self.code = code
        self.job = job
    }

    func send(systemPort: String, message: Message, now: TestInstant) {
        seen.sent.append(Sent(systemPort: systemPort, event: message.event))
        guard let request = message.valueAs(JobRequestPayload.self) else {
            return
        }
        seen.requested.append(request.job)
        let reply = JobReplyPayload(job: job ?? request.job, code: code)
        pending.append((at: now + answerDelay, message: Message(event: JobReply.eventName, value: reply)))
    }

    func nextInput() -> TestInstant? {
        pending.first?.at
    }

    func takeInputsDue(now: TestInstant) -> [SystemInput] {
        var due: [SystemInput] = []
        while let first = pending.first, first.at <= now {
            pending.removeFirst()
            due.append(SystemInput(systemPort: "sut", message: first.message))
        }
        return due
    }
}

/// A system under test nothing is mapped to.
private final class Silent: SystemAdapter {
    func send(systemPort: String, message: Message, now: TestInstant) {
        XCTFail("no port is mapped, so nothing reaches the system adapter, yet \(message.event) was sent on \(systemPort)")
    }

    func nextInput() -> TestInstant? {
        nil
    }

    func takeInputsDue(now: TestInstant) -> [SystemInput] {
        []
    }
}

/// The index of the first record of `kind` by `component`, or nil.
private func firstRecord(_ result: TestCaseResult, _ component: String, _ kind: LogKind) -> Int? {
    result.log.firstIndex { $0.component == component && $0.kind == kind }
}

/// The index of the first record of `kind` by `component`, which the log holds.
private func recordOf(_ result: TestCaseResult, _ component: String, _ kind: LogKind) throws -> Int {
    try XCTUnwrap(firstRecord(result, component, kind), "\(component) logs \(kind): \(result.log)")
}

/// One test case run of `configuration` against `adapter`. Not named `run`: inside a test
/// method that name is `XCTestCase.run()`, which hides a function of the module.
private func runTestCase(_ configuration: Configuration, _ adapter: any SystemAdapter) -> TestCaseResult {
    TestCaseRun(configuration, machine(of:), adapter).run()
}

/// The asker as the MTC, its port `sut` mapped to the system port `sut`.
private func askerConfiguration() -> Configuration {
    Configuration(
        mtcType: "asker",
        portsOfType: ["asker": ["sut"]],
        timersOfType: ["asker": ["reply"]],
        mappings: [Mapping(Endpoint(component: mtcName, port: "sut"), systemPort: "sut")])
}

/// The asker run against a responder that answers with `code`, for the requested job or for `job`
/// when one is given.
private func runAsker(code: UInt32, job: UInt32? = nil) -> (TestCaseResult, Seen) {
    let seen = Seen()
    return (runTestCase(askerConfiguration(), Responder(seen, code: code, job: job)), seen)
}

final class ATestComponentRunsThroughItsDoorsTests: XCTestCase {
    /// P6, P2, P4: the send carries the template's value bound to the component's variable, a reply its
    /// receive branch's template matches is taken when it arrives, and the content reads the reply as
    /// `_event.data`.
    func testATestComponentThatIsAnsweredWithAMatchingReplyPasses() throws {
        let (result, seen) = runAsker(code: 0)
        XCTAssertEqual(result.verdict, .pass, "\(result.log)")
        XCTAssertEqual(result.endedAt, answerDelay, "the reply is taken in the round it arrives in")
        XCTAssertEqual(
            seen.sent, [Sent(systemPort: "sut", event: JobRequest.eventName)],
            "a message travels under its template's record's event")
        XCTAssertEqual(seen.requested, [7], "the template's value, given `job`")
        _ = try recordOf(result, mtcName, .receive)
        // P8: the `<log>` of the branch's content is in the test log, with the instant and the
        // component, its label and the job it read.
        let logged = try recordOf(result, mtcName, .log)
        XCTAssertEqual(result.log[logged].detail, "answered: 7")
        XCTAssertEqual(result.log[logged].at, answerDelay)
    }

    /// P2, P7, §7: a reply the template does not match enables no branch and is not removed, and the
    /// timer's timeout branch fails the test case.
    func testATestComponentThatIsAnsweredWithAReplyItsTemplateDoesNotMatchFailsWhenItsTimerFires() throws {
        let (result, _) = runAsker(code: 3)
        XCTAssertEqual(result.verdict, .fail, "\(result.log)")
        XCTAssertEqual(result.endedAt, second, "time advances to the timer's instant")
        XCTAssertNil(
            firstRecord(result, mtcName, .receive), "a reply that does not match is never taken: \(result.log)")
        _ = try recordOf(result, mtcName, .timeout)
    }

    /// P4: the content reads the message the branch took: a reply of success for another job matches
    /// the template, which takes any job, and the content's comparison of the job it read fails the
    /// test case.
    func testATestComponentReadsTheReplyItTookAsTheEventsData() throws {
        let (result, _) = runAsker(code: 0, job: 8)
        XCTAssertEqual(result.verdict, .fail, "\(result.log)")
        XCTAssertEqual(result.endedAt, answerDelay)
        _ = try recordOf(result, mtcName, .receive)
    }

    /// P9, P6, P2: the MTC creates a PTC connected to it, which asks it for job 42 as it starts; the MTC
    /// answers with the job it read, the PTC passes on the reply and terminates, and the MTC's done
    /// branch passes on the next snapshot. Leaving the invoking state kills the PTC, before the done
    /// branch's content runs.
    func testATestComponentCreatesAPtcExchangesAMessageWithItAndReadsItsDone() throws {
        let configuration = Configuration(
            mtcType: "coordinator",
            ptcTypes: ["answerer": "answerer"],
            portsOfType: ["coordinator": ["peer"], "answerer": ["peer"]],
            connections: [
                Connection(Endpoint(component: mtcName, port: "peer"), Endpoint(component: "answerer", port: "peer"))
            ])
        let result = runTestCase(configuration, Silent())

        XCTAssertEqual(result.verdict, .pass, "\(result.log)")
        XCTAssertEqual(result.endedAt, 0, "a branch fired in every round, so time never moved")
        let created = try recordOf(result, "answerer", .create)
        let asked = try recordOf(result, "answerer", .send)
        let answered = try recordOf(result, mtcName, .send)
        let done = try recordOf(result, "answerer", .done)
        let killed = try recordOf(result, "answerer", .kill)
        let passed = try recordOf(result, mtcName, .verdict)
        XCTAssertLessThan(
            created, asked, "the PTC is created when the macrostep that entered its state ends: \(result.log)")
        // P8: a `<log>` that writes only a label is recorded, by the PTC.
        let noted = try recordOf(result, "answerer", .log)
        XCTAssertEqual(result.log[noted].detail, "asked")
        XCTAssertLessThan(asked, noted, "the log runs in its place, after the send before it: \(result.log)")
        XCTAssertEqual(result.log[asked].detail, "peer job.request")
        XCTAssertEqual(result.log[answered].detail, "peer job.reply")
        XCTAssertLessThan(answered, done, "the MTC answers before the PTC is done: \(result.log)")
        XCTAssertLessThan(
            done, killed, "a done branch fires on a snapshot taken after the PTC terminated: \(result.log)")
        XCTAssertLessThan(
            killed, passed, "the invoking state is exited before the done branch's content runs: \(result.log)")
    }

    /// P9, P14: leaving the state that invoked a PTC removes it while it still runs, at that instant,
    /// and its local verdict still counts.
    func testATestComponentKillsAPtcOnLeavingTheStateThatInvokedIt() throws {
        let configuration = Configuration(
            mtcType: "ptc_keeper",
            ptcTypes: ["idler": "idler"],
            portsOfType: ["idler": ["peer"]],
            timersOfType: ["ptc_keeper": ["patience", "linger"]])
        let result = runTestCase(configuration, Silent())

        let killed = try recordOf(result, "idler", .kill)
        XCTAssertEqual(result.log[killed].at, 10 * millisecond, "the PTC is removed when `patience` runs out")
        XCTAssertNil(firstRecord(result, "idler", .done), "the idler never terminates: \(result.log)")
        XCTAssertEqual(result.endedAt, 15 * millisecond)
        XCTAssertEqual(
            result.verdict, .inconclusive, "the removed PTC's inconclusive outweighs the MTC's pass")
    }
}
