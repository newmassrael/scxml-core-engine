// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
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
// the system under test, is written here; the same suite is also replayed from the committed trace of
// `answers_the_job` (docs/SCE_TTCN3_LANE.md §10.5) through `SceTestProfile.runSuiteWithTraces`, and from
// every committed trace, each to its expected log, through the suite's runner program (§4), run in
// place and as a process: the executable target `RuntimeSuiteRunner`, whose `main.swift` calls the
// suite's entry.
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
        // The reply's canonical form in the test log is the one the unit of a template over its record
        // writes.
        let message = Message.of(
            event: JobReply.eventName, value: reply, typeName: ReplyOk.TYPE,
            canonical: { ReplyOk.canonical($0) })
        pending.append((at: now + answerDelay, message: message))
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

/// The test log every runtime writes of `answers_the_job` in a world with no feature, answered with
/// success: one file the Rust, C++ and Kotlin runs are held to (`a_generated_test_case_writes_its_test_log`
/// in backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs), read where it is, so the
/// agreement of the backends is one set of bytes. The file ends in one newline, which no canonical text
/// holds.
private func answersTheJobLog() throws -> String {
    let path = repoRoot().appendingPathComponent(
        "sce-build/tests/fixtures/test_profile/runtime/expected_logs/answers_the_job.json")
    let text = try String(contentsOf: path, encoding: .utf8)
    XCTAssertTrue(text.hasSuffix("\n") && !text.hasSuffix("\n\n"), "the shared log ends in one newline")
    return String(text.dropLast())
}

/// The repository, above this file: the file, then StaticDatamodelTests, Tests, tests, swift and
/// backends.
private func repoRoot() -> URL {
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url
}

/// The directory of the suite's committed traces (docs/SCE_TTCN3_LANE.md §10.2).
private func tracesDir() -> URL {
    repoRoot().appendingPathComponent("sce-build/tests/fixtures/test_profile/runtime/traces")
}

/// The bytes of the text of the file at `url`.
private func contents(_ url: URL) throws -> [UInt8] {
    Array(try Data(contentsOf: url))
}

/// The world that selects every test case of the suite, so that the runner replays every trace of the
/// directory.
private let everyFeature = "ptcs,removal,guard"

/// The lines the runner prints in the world of every feature: each case of the suite, in its order, with
/// the verdict it ends with.
private let everyVerdict =
    "answers_the_job pass\nserves_a_ptc pass\nkeeps_a_ptc inconc\nanswers_before_the_guard error\n"

/// The directory of the expected log of each committed trace, `<test case>.json` each: the canonical log
/// a reviewed run wrote of it, with one newline after it.
private func expectedLogsDir() -> URL {
    repoRoot().appendingPathComponent("sce-build/tests/fixtures/test_profile/runtime/expected_logs")
}

/// docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every log the runner wrote into `logs`, `<case>.log.json`, is
/// the expected log of its case byte for byte, and the runner wrote a log for every expected log and
/// nothing else. The twin of `assert_every_log_is_its_expected_log` (Rust). Names are compared by their
/// bytes, never by Swift's comparison of texts.
private func assertEveryLogIsItsExpectedLog(_ logs: URL) throws {
    let files = FileManager.default
    func sortedNames(_ dir: URL) throws -> [[UInt8]] {
        try files.contentsOfDirectory(atPath: dir.path).map { Array($0.utf8) }
            .sorted { $0.lexicographicallyPrecedes($1) }
    }
    let expected = try sortedNames(expectedLogsDir()).map { name -> [UInt8] in
        XCTAssertTrue(
            name.count > 5 && name.suffix(5).elementsEqual(".json".utf8),
            "the expected log \(String(decoding: name, as: UTF8.self)) is a .json file")
        return Array(name.dropLast(5))
    }
    var written: [[UInt8]] = []
    for name in try sortedNames(logs) {
        let text = String(decoding: name, as: UTF8.self)
        guard name.count > 9 && name.suffix(9).elementsEqual(".log.json".utf8) else {
            XCTFail("the runner wrote \(text), which is no log of a case")
            written.append(name)
            continue
        }
        let testCase = String(decoding: name.dropLast(9), as: UTF8.self)
        written.append(Array(name.dropLast(9)))
        let path = expectedLogsDir().appendingPathComponent("\(testCase).json")
        guard files.fileExists(atPath: path.path) else {
            XCTFail("the case \(testCase) has no expected log \(path.path)")
            continue
        }
        let want = try contents(path)
        XCTAssertEqual(want.last, UInt8(ascii: "\n"), "the expected log of \(testCase) ends in one newline")
        XCTAssertEqual(
            try contents(logs.appendingPathComponent(text)), Array(want.dropLast()),
            "the log of \(testCase) is not its expected log")
    }
    XCTAssertEqual(
        written, expected, "the runner wrote a log of each case that has an expected log, and of no other")
}

/// The reason `read` is refused for, or nil where it reads.
private func refusal<T>(_ read: () throws(TraceRefusal) -> T) -> String? {
    do throws(TraceRefusal) {
        _ = try read()
        return nil
    } catch {
        return error.reason
    }
}

/// What one run of the runner program printed and the status it exited with: the standard output's
/// bytes, and the lines of each stream.
private struct ProgramRun {
    let status: Int32
    let output: [UInt8]
    let out: [String]
    let err: [String]

    init(status: Int32, output: [UInt8], err: [String]) {
        self.status = status
        self.output = output
        self.out = String(decoding: output, as: UTF8.self).split(separator: "\n").map(String.init)
        self.err = err
    }
}

/// The directory SwiftPM builds this package's products in, where the runner program's executable is:
/// the directory of the test bundle on Apple platforms, and of the test executable elsewhere.
private func productsDirectory() -> URL {
    #if os(macOS)
        if let bundle = Bundle.allBundles.first(where: { $0.bundlePath.hasSuffix(".xctest") }) {
            return bundle.bundleURL.deletingLastPathComponent()
        }
    #endif
    return Bundle.main.bundleURL
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

    /// docs/SCE_TEST_PROFILE.md §12.5: the test log of a generated test case, in its one canonical text,
    /// the bytes of the shared file the Rust, C++ and Kotlin runs are held to. The request the component
    /// sends carries the value its template's unit writes, the job of the suite's module parameter; the
    /// reply arrives at 500 and is taken in that round, which stops the timer, passes and ends the MTC.
    func testAGeneratedTestCaseWritesItsTestLog() throws {
        let results = try runTheSuite([], code: 0, requested: Requested())
        XCTAssertEqual(names(results), ["answers_the_job"])
        let only = try XCTUnwrap(results.first)
        let log = only.canonicalLog(suite: "runtime_suite")
        // Compared by bytes: Swift's `==` on a `String` holds two canonically equivalent texts equal,
        // which two logs that agree under C3 are not.
        XCTAssertEqual(Array(log.utf8), Array(try answersTheJobLog().utf8), log)
    }

    /// docs/SCE_TTCN3_LANE.md §10.5: the generated suite replayed from the committed trace of
    /// `answers_the_job` writes the test log every runtime writes of it, byte for byte: the
    /// configuration's unit reads the reply from its canonical form and writes it back the same, and the
    /// clock advances to the trace's instant as it did to the Responder's. The twin of
    /// `a_recorded_trace_replays_to_the_test_log_every_runtime_writes` (Rust).
    func testARecordedTraceReplaysToTheTestLogEveryRuntimeWrites() throws {
        let text = try String(contentsOf: tracesDir().appendingPathComponent("answers_the_job.json"), encoding: .utf8)
        let results = try runSuiteWithTraces(
            RuntimeSuite.suite(), world: [],
            traces: { (_: String) throws(TraceRefusal) -> Trace in try Trace.parse(text) })
        XCTAssertEqual(names(results), ["answers_the_job"])
        let only = try XCTUnwrap(results.first)
        XCTAssertEqual(only.result.verdict, .pass, "\(only.result.log)")
        XCTAssertEqual(Array(only.canonicalLog(suite: "runtime_suite").utf8), Array(try answersTheJobLog().utf8))
    }

    /// §10.2, §10.5: the configuration's unit reads the reply from its canonical form into the record its
    /// components take, and writes it back the same; an input on a port it does not have, of a type the
    /// port does not take, or of a value that does not belong to the type is refused by name.
    func testTheConfigurationReadsTheInputsOfARecordedTrace() throws {
        let reply = try AskingConfiguration.read_job_reply(parseJSON("{\"code\":\"0\",\"job\":\"7\"}"))
        XCTAssertEqual(reply, JobReplyPayload(job: 7, code: 0))
        let written = try writeCanonicalJSON(AskingConfiguration.canonical_job_reply(reply))
        XCTAssertEqual(Array(written.utf8), Array("{\"code\":\"0\",\"job\":\"7\"}".utf8))
        let message = try AskingConfiguration.input(
            systemPort: "sut", typeName: "job_reply", value: AskingConfiguration.canonical_job_reply(reply))
        XCTAssertEqual(message.event, JobReply.eventName)
        XCTAssertEqual(message.typeName, "job_reply")
        XCTAssertEqual(message.valueAs(JobReplyPayload.self), reply)
        XCTAssertEqual(
            refusal { () throws(TraceRefusal) -> Message in
                try AskingConfiguration.input(systemPort: "sut", typeName: "job_request", value: .null)
            },
            "'job_request' is not on the `in` list of the type of the system port 'sut'")
        XCTAssertEqual(
            refusal { () throws(TraceRefusal) -> Message in
                try AskingConfiguration.input(systemPort: "oracle", typeName: "job_reply", value: .null)
            },
            "'oracle' is not a system port of the test case's configuration")
        let leadingZero = try parseJSON("{\"code\":\"0\",\"job\":\"07\"}")
        XCTAssertEqual(
            refusal { () throws(TraceRefusal) -> Message in
                try AskingConfiguration.input(systemPort: "sut", typeName: "job_reply", value: leadingZero)
            },
            "job_reply.job: '07' is not the text of an integer's decimal digits")
    }

    /// A fresh directory, apart from every other test's, removed when the test ends.
    private func scratchDir() throws -> URL {
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("sce-runtime-suite-runner-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        addTeardownBlock {
            try? FileManager.default.removeItem(at: dir)
        }
        return dir
    }

    /// The suite's runner program run in place (`runRunner`, over Foundation's files), given `args`
    /// after the directories of the traces, the committed ones unless `traces` names others, and of the
    /// logs.
    private func runInPlace(_ logs: URL, _ args: [String], traces: URL = tracesDir()) -> ProgramRun {
        var out: [String] = []
        var err: [String] = []
        let status = runRunner(
            suite: RuntimeSuite.suite(), parameter: RuntimeSuite.moduleParameter(id:literal:),
            args: ["--trace-dir", traces.path, "--log-dir", logs.path] + args,
            files: FoundationRunnerFiles(), out: { out.append($0) }, err: { err.append($0) })
        let output = out.map { $0 + "\n" }.joined()
        return ProgramRun(status: status, output: Array(output.utf8), err: err)
    }

    /// docs/SCE_TTCN3_LANE.md §4: the suite's runner program runs each test case its world selects
    /// against its trace and writes `<case>.log.json`, the canonical log with nothing after it, and
    /// prints each case's verdict. In the world of every feature it replays every trace of the directory,
    /// and each log it writes is its case's expected log (§12, §13.1 S2). A module parameter's value on
    /// its command line replaces its default, and one the suite does not declare, or that is no value of
    /// its type, is refused with exit status 2 before any case runs. The twin of
    /// `the_suites_runner_program_writes_the_log_of_each_trace` (Rust).
    func testTheSuitesRunnerProgramWritesTheLogOfEachTrace() throws {
        let scratch = try scratchDir()
        let logs = scratch.appendingPathComponent("every-logs")
        let run = runInPlace(logs, ["--world", everyFeature])
        XCTAssertEqual(run.status, 0, "\(run.err)")
        XCTAssertEqual(run.output, Array(everyVerdict.utf8))
        try assertEveryLogIsItsExpectedLog(logs)

        let overridden = scratch.appendingPathComponent("overridden-logs")
        let given = runInPlace(overridden, ["--world", "", "--module-parameter", "job=9"])
        XCTAssertEqual(given.status, 0, "\(given.err)")
        let bytes = try contents(overridden.appendingPathComponent("answers_the_job.log.json"))
        let log = String(decoding: bytes, as: UTF8.self)
        let request = "\"kind\":\"message-sent\",\"port\":\"sut\",\"type\":\"job_request\",\"value\":{\"job\":\"9\"}"
        XCTAssertTrue(log.contains(request), "the request names the job of the value given: \(log)")

        let refusals = [
            ("limit=1", "the suite declares no module parameter 'limit'"),
            ("job=seven", "--module-parameter job=seven: job: 'seven' is not the text of an integer's decimal digits"),
        ]
        for (index, (literal, why)) in refusals.enumerated() {
            let refused = scratch.appendingPathComponent("refused-\(index)-logs")
            let attempt = runInPlace(refused, ["--module-parameter", literal])
            XCTAssertEqual(attempt.status, 2, literal)
            XCTAssertTrue(attempt.err.contains { $0.contains(why) }, "\(attempt.err)")
            XCTAssertEqual(attempt.err.last, runnerUsage)
            XCTAssertFalse(FileManager.default.fileExists(atPath: refused.path), "no case ran, so no log was written")
        }
    }

    /// P13, §4: a test case whose trace is missing ends with `error` and its log is written, the reason on
    /// the standard error, and the program exits 0, since the verdict is the log's; a command line that
    /// is not the runner's, and a log directory that cannot be made, exit 2. The directory of the traces
    /// holds the committed trace of `answers_the_job` alone, so `serves_a_ptc`'s is missing.
    func testATestCaseWhoseTraceIsMissingEndsInErrorAndItsLogIsWritten() throws {
        let scratch = try scratchDir()
        let traces = scratch.appendingPathComponent("traces")
        try FileManager.default.createDirectory(at: traces, withIntermediateDirectories: true)
        try FileManager.default.copyItem(
            at: tracesDir().appendingPathComponent("answers_the_job.json"),
            to: traces.appendingPathComponent("answers_the_job.json"))
        let logs = scratch.appendingPathComponent("logs")
        let run = runInPlace(logs, ["--world", "ptcs"], traces: traces)
        XCTAssertEqual(run.status, 0, "\(run.err)")
        XCTAssertEqual(run.out, ["answers_the_job pass", "serves_a_ptc error"])
        let refusedLog =
            "{\"records\":[{\"at\":\"0\",\"kind\":\"test-case-started\"},{\"at\":\"0\",\"kind\":\"test-case-ended\","
            + "\"verdict\":\"error\"}],\"schema\":\"sce-test-log.v1\",\"suite\":\"runtime_suite\","
            + "\"test_case\":\"serves_a_ptc\",\"verdict\":\"error\"}"
        XCTAssertEqual(try contents(logs.appendingPathComponent("serves_a_ptc.log.json")), Array(refusedLog.utf8))
        let missing = run.err.contains { line in
            line.hasPrefix("serves_a_ptc: mtc: the trace is refused: ") && line.contains("is not read")
        }
        XCTAssertTrue(missing, "\(run.err)")

        var ran: [String] = []
        let usage = runRunner(
            suite: RuntimeSuite.suite(), parameter: RuntimeSuite.moduleParameter(id:literal:),
            args: ["--log-dir", "logs"], files: FoundationRunnerFiles(), out: { ran.append($0) }, err: { _ in })
        XCTAssertEqual(usage, 2)
        XCTAssertTrue(ran.isEmpty, "a refused command line runs nothing: \(ran)")

        let blocked = scratch.appendingPathComponent("blocked.log")
        try Data().write(to: blocked)
        let unmade = runInPlace(blocked, [])
        XCTAssertEqual(unmade.status, 2)
        XCTAssertTrue(unmade.err.contains { $0.contains("is not made") }, "\(unmade.err)")
    }

    /// §4: the suite's runner is a program: the executable target `RuntimeSuiteRunner`, whose `main.swift`
    /// calls the suite's entry, run as a process of its own, prints the verdict of each case and nothing
    /// else on its standard output, writes its log and exits with the status the command line asks for.
    func testTheSuitesEntryIsTheRunnerProgram() throws {
        let scratch = try scratchDir()
        let executable = productsDirectory().appendingPathComponent("RuntimeSuiteRunner")
        func program(_ name: String, _ args: [String]) throws -> ProgramRun {
            let out = scratch.appendingPathComponent("\(name).out")
            let err = scratch.appendingPathComponent("\(name).err")
            try Data().write(to: out)
            try Data().write(to: err)
            let process = Process()
            process.executableURL = executable
            process.arguments =
                ["--trace-dir", tracesDir().path, "--log-dir", scratch.appendingPathComponent("\(name)-logs").path]
                + args
            process.standardOutput = try FileHandle(forWritingTo: out)
            process.standardError = try FileHandle(forWritingTo: err)
            try process.run()
            process.waitUntilExit()
            let errors = String(decoding: try contents(err), as: UTF8.self).split(separator: "\n").map(String.init)
            return ProgramRun(status: process.terminationStatus, output: try contents(out), err: errors)
        }
        let run = try program("every", ["--world", everyFeature])
        XCTAssertEqual(run.status, 0, "\(run.err)")
        XCTAssertEqual(run.output, Array(everyVerdict.utf8), "the standard output is the contract's lines alone")
        // The standard error may hold diagnostics, whatever they say: none of its lines is one of the
        // standard output's.
        XCTAssertTrue(run.err.allSatisfy { !run.out.contains($0) }, "\(run.err)")
        try assertEveryLogIsItsExpectedLog(scratch.appendingPathComponent("every-logs"))
        let refused = try program("undeclared", ["--module-parameter", "limit=1"])
        XCTAssertEqual(refused.status, 2)
        XCTAssertEqual(refused.output, [], "a refused command line prints nothing on the standard output")
        XCTAssertTrue(
            refused.err.contains { $0.contains("the suite declares no module parameter 'limit'") }, "\(refused.err)")
        XCTAssertFalse(
            FileManager.default.fileExists(atPath: scratch.appendingPathComponent("undeclared-logs").path),
            "no case ran, so no log was written")
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
