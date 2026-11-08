// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import SceTestProfile
import StaticDatamodelMachines
import XCTest

// A test suite generated in Swift (docs/SCE_TEST_PROFILE.md §11.6 to §11.8, §12.2, §12.4, §12.6 slice 6),
// run end to end by the profile's runner, `SceTestProfile.runSuite`. The suite, its test cases and their
// configurations are the tables sce-codegen writes from the fixtures under
// `sce-build/tests/fixtures/test_profile/runtime/` into the target `StaticDatamodelMachines` (regen:
// `scripts/regen_static_datamodel_swift.sh`), each the members of an enum named by its document, and the
// components are the machines it writes from the components there. Only the system adapter, which plays
// the system under test, is written here.
//
// Each test is the twin of the Rust case of the same name in
// `backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs`, itself the twin of the C++ gtest
// in `tests/integration/ATestSuiteRunsEndToEndTest.cpp`, of the Kotlin
// `ATestSuiteRunsThroughItsRunnerTest.kt`, of the Go `a_test_suite_runs_through_its_runner_test.go`, of
// the Python `test_a_test_suite_runs_through_its_runner.py` and of the C11
// `test_a_test_suite_runs_through_its_runner.c`, with the same suite, worlds, verdicts and `NOW`.
// `param_asker`, the component that declares a module parameter, is run here: the suite's tables give
// its value. What the generator writes for each table is held by
// `sce-build/tests/a_test_suite_is_generated_in_swift.rs`.

/// A millisecond of `NOW`, which counts microseconds.
private let millisecond: TestDuration = 1000

/// A second of `NOW`.
private let second: TestDuration = 1000 * millisecond

/// How long the system under test takes to answer a request.
private let answerDelay: TestDuration = 500

/// Every job the system under test was asked for, in the order it was asked, across the test cases of
/// a run, which the test keeps while each run owns its adapter.
private final class Requested {
    var jobs: [UInt32] = []
}

/// The system under test on the system port `sut`: it answers each request, `answerDelay` after it,
/// with a reply of `code` for the job the request names, and records each job it was asked for.
private final class Responder: SystemAdapter {
    private let code: UInt32
    private let requested: Requested
    private var pending: [(at: TestInstant, message: Message)] = []

    init(code: UInt32, requested: Requested) {
        self.code = code
        self.requested = requested
    }

    func send(systemPort: String, message: Message, now: TestInstant) {
        XCTAssertEqual(systemPort, "sut")
        guard let request = message.valueAs(JobRequestPayload.self) else {
            XCTFail("a request travels as its record's value, yet \(message.event) was sent")
            return
        }
        requested.jobs.append(request.job)
        let reply = JobReplyPayload(job: request.job, code: code)
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

/// The suite's run in a world of `features`, answered with `code` by a fresh responder for each test
/// case, with `overrides` in place of the defaults of the module parameters they give. Not named `run`:
/// inside a test method that name is `XCTestCase.run()`, which hides a function of the module.
private func runTheSuite(
    _ features: Features, code: UInt32, requested: Requested, overrides: ModuleParameters = ModuleParameters()
) throws(OverrideRefusal) -> [CaseResult] {
    try runSuite(
        RuntimeSuite.suite(), world: features, adapters: { _ in Responder(code: code, requested: requested) },
        overrides: overrides)
}

/// The names of the test cases that ran, in the order they ran.
private func names(_ results: [CaseResult]) -> [String] {
    results.map(\.testCase)
}

/// Whether the log of `result` records a line of `kind`.
private func logged(_ result: CaseResult, _ kind: LogKind) -> Bool {
    result.result.log.contains { $0.kind == kind }
}

final class ATestSuiteRunsThroughItsRunnerTests: XCTestCase {
    /// P11, P12, P13, P14: in a world with every feature the suite runs every test case in its order, and
    /// each ends with the verdict it expects: a job answered in time passes, a PTC served passes, a
    /// removed PTC's inconclusive outweighs its MTC's pass, and a guard shorter than the answer ends in
    /// error.
    func testRunsEveryTestCaseOfAWorldWithTheVerdictItExpects() throws {
        let requested = Requested()
        let results = try runTheSuite(["ptcs", "removal", "guard"], code: 0, requested: requested)
        XCTAssertEqual(
            names(results), ["answers_the_job", "serves_a_ptc", "keeps_a_ptc", "answers_before_the_guard"])
        guard results.count == 4 else {
            return
        }
        let outcome = results.map(\.result)
        XCTAssertEqual(outcome[0].verdict, .pass, "\(outcome[0].log)")
        XCTAssertEqual(outcome[0].endedAt, answerDelay, "the reply is taken in the round it arrives in")
        XCTAssertEqual(outcome[1].verdict, .pass, "\(outcome[1].log)")
        XCTAssertEqual(outcome[1].endedAt, 0, "a branch fired in every round, so time never moved")
        XCTAssertEqual(outcome[2].verdict, .inconclusive, "\(outcome[2].log)")
        XCTAssertEqual(outcome[2].endedAt, 15 * millisecond)
        XCTAssertEqual(outcome[3].verdict, .error, "\(outcome[3].log)")
        XCTAssertEqual(outcome[3].endedAt, 100, "the guard, 0.1ms, ends the case before the answer")
        XCTAssertTrue(logged(results[3], .error))
        XCTAssertEqual(
            requested.jobs, [7, 7], "each asking case asks for the job the suite's module parameter gives")
    }

    /// P2, P7: a system under test that answers with a failure code fails the test case when its timer
    /// runs out. A world with no feature runs only the test case no selection guards.
    func testFailsAJobTheSystemUnderTestAnswersWithAFailure() throws {
        let results = try runTheSuite([], code: 3, requested: Requested())
        XCTAssertEqual(names(results), ["answers_the_job"])
        guard let only = results.first else {
            return
        }
        XCTAssertEqual(only.result.verdict, .fail, "\(only.result.log)")
        XCTAssertEqual(only.result.endedAt, second, "time advances to the timer's instant")
        XCTAssertTrue(logged(only, .timeout))
    }

    /// §11.8: a world runs the test cases whose selection it satisfies, in the suite's order: `ptcs` alone
    /// selects the served PTC and not the removed one, and `guard` selects the guarded case.
    func testRunsTheTestCasesAWorldSelects() throws {
        let requested = Requested()
        let withPtcs = try runTheSuite(["ptcs"], code: 0, requested: requested)
        XCTAssertEqual(names(withPtcs), ["answers_the_job", "serves_a_ptc"])
        let withGuard = try runTheSuite(["guard"], code: 0, requested: requested)
        XCTAssertEqual(names(withGuard), ["answers_the_job", "answers_before_the_guard"])
    }

    /// -1 §8.2.1, §11.5: a value the caller gives the module parameter replaces the suite's default, and
    /// the component asks for the job it names.
    func testAsksForTheJobAnOverrideOfItsModuleParameterNames() throws {
        let requested = Requested()
        var overrides = ModuleParameters()
        overrides.set("job", SavedValues.of(9 as UInt32))
        let results = try runTheSuite([], code: 0, requested: requested, overrides: overrides)
        XCTAssertEqual(results.count, 1)
        XCTAssertEqual(results.first?.result.verdict, .pass, "\(results.map(\.result.log))")
        XCTAssertEqual(requested.jobs, [9])
    }

    /// P13: a value of the module parameter that is not of the type the component declares ends the test
    /// case in error before the component starts.
    func testEndsInErrorWhenAModuleParameterIsNotOfItsType() throws {
        let requested = Requested()
        var overrides = ModuleParameters()
        overrides.set("job", SavedValues.of("seven"))
        let results = try runTheSuite([], code: 0, requested: requested, overrides: overrides)
        XCTAssertEqual(results.count, 1)
        guard let only = results.first else {
            return
        }
        XCTAssertEqual(only.result.verdict, .error, "\(only.result.log)")
        XCTAssertTrue(logged(only, .error), "the refused read is logged as an error")
        XCTAssertTrue(requested.jobs.isEmpty, "the component did not start, so it sent nothing")
    }

    /// -1 §8.2.1: an override of a module parameter the suite does not declare is the caller's mistake,
    /// refused before any test case runs.
    func testRefusesAnOverrideOfAModuleParameterTheSuiteDoesNotDeclare() {
        let requested = Requested()
        var overrides = ModuleParameters()
        overrides.set("limit", SavedValues.of(1 as UInt32))
        XCTAssertThrowsError(
            try runTheSuite(["ptcs", "removal", "guard"], code: 0, requested: requested, overrides: overrides)
        ) { error in
            XCTAssertTrue(
                (error as? OverrideRefusal)?.reason.contains("'limit'") ?? false,
                "the refusal names the module parameter: \(error)")
        }
        XCTAssertTrue(requested.jobs.isEmpty, "no test case runs")
    }

    /// §11.6, §11.7: the configuration's tables name the module parameters its components read, which a
    /// suite that runs it must give, and the test case names itself and its configuration.
    func testReadsTheModuleParametersItsConfigurationNames() {
        XCTAssertEqual(AskingConfiguration.moduleParameters(), ["job"])
        XCTAssertEqual(AskingConfiguration.configuration().mtcType, "param_asker")
        XCTAssertNil(AskingConfiguration.machine("nobody"))
        XCTAssertNotNil(AskingConfiguration.machine("param_asker"))
        let entry = AnswersTheJob.testCase()
        XCTAssertEqual(entry.name, AnswersTheJob.name)
        XCTAssertEqual(entry.name, "answers_the_job")
        XCTAssertEqual(entry.configuration, AskingConfiguration.configuration())
        XCTAssertEqual(entry.moduleParameters, ["job"])
    }
}
