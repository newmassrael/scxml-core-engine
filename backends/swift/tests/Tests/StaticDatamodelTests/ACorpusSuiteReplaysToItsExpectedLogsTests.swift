// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import SceTestProfile
import StaticDatamodelMachines
import XCTest

// docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every trace of a suite of the corpus replays, through the suite's
// runner (`runRunner`, the program of §4 run in place), to its expected log, byte for byte. The suites are
// the ones sce-codegen writes into the target `StaticDatamodelMachines` (regen:
// `scripts/regen_static_datamodel_swift.sh`); runtime_suite's own traces are replayed by
// `ATestSuiteRunsThroughItsRunnerTests.swift`, beside the rest of what that suite is held to.
//
// The expected logs are the files every backend is held to, the canonical logs a reviewed run of the Rust
// runner wrote. Each test is the twin of the Rust case of the same name in
// `backends/rust/tests/tests/the_corpus_replays_to_its_expected_logs.rs`.

/// The repository, above this file: the file, then StaticDatamodelTests, Tests, tests, swift and
/// backends.
private func corpusRepoRoot() -> URL {
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url
}

/// The directory of the runs of `suite`: its `traces/` and `expected_logs/`.
private func corpusRunsDir(_ suite: String) -> URL {
    corpusRepoRoot().appendingPathComponent("sce-build/tests/fixtures/test_profile/runs/\(suite)")
}

/// The names of the files in `dir`, each as its bytes, sorted by them, never by Swift's comparison of
/// texts.
private func corpusSortedNames(_ dir: URL) throws -> [[UInt8]] {
    try FileManager.default.contentsOfDirectory(atPath: dir.path).map { Array($0.utf8) }
        .sorted { $0.lexicographicallyPrecedes($1) }
}

/// A suite of the corpus and how its runner is run over its traces.
private struct CorpusReplayed {
    /// The suite's unit name, which names the directory of its runs.
    let name: String
    /// The suite's table, which the runner runs.
    let suite: SceTestProfile.Suite
    /// The reader of a module parameter's value from the command line.
    let parameter: SceTestProfile.ParameterReader
    /// The world that selects every test case, as `--world` takes it.
    let world: String
    /// What the runner prints: each test case its world selects, in the suite's order, with its verdict.
    let verdicts: String
}

final class ACorpusSuiteReplaysToItsExpectedLogsTests: XCTestCase {
    /// Runs the suite's runner over every trace of its directory, in the world that selects every test
    /// case, and holds what it prints to the verdicts it expects, every log it writes to its expected log,
    /// and the set of logs to the set of expected logs.
    private func replays(_ replayed: CorpusReplayed) throws {
        let runs = corpusRunsDir(replayed.name)
        let scratch = FileManager.default.temporaryDirectory
            .appendingPathComponent("sce-corpus-runner-\(replayed.name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
        addTeardownBlock {
            try? FileManager.default.removeItem(at: scratch)
        }
        let logs = scratch.appendingPathComponent("logs")
        var out: [String] = []
        var err: [String] = []
        let status = runRunner(
            suite: replayed.suite, parameter: replayed.parameter,
            args: [
                "--trace-dir", runs.appendingPathComponent("traces").path, "--log-dir", logs.path, "--world",
                replayed.world,
            ],
            files: FoundationRunnerFiles(), out: { out.append($0) }, err: { err.append($0) })
        XCTAssertEqual(status, 0, "\(err)")
        let output = out.map { $0 + "\n" }.joined()
        XCTAssertEqual(Array(output.utf8), Array(replayed.verdicts.utf8), "\(err)")
        let expectedDir = runs.appendingPathComponent("expected_logs")
        let expected = try corpusSortedNames(expectedDir).map { name -> [UInt8] in
            XCTAssertTrue(
                name.count > 5 && name.suffix(5).elementsEqual(".json".utf8),
                "the expected log \(String(decoding: name, as: UTF8.self)) is a .json file")
            return Array(name.dropLast(5))
        }
        var written: [[UInt8]] = []
        for name in try corpusSortedNames(logs) {
            let text = String(decoding: name, as: UTF8.self)
            guard name.count > 9 && name.suffix(9).elementsEqual(".log.json".utf8) else {
                XCTFail("the runner wrote \(text), which is no log of a case")
                written.append(name)
                continue
            }
            let testCase = String(decoding: name.dropLast(9), as: UTF8.self)
            written.append(Array(name.dropLast(9)))
            let path = expectedDir.appendingPathComponent("\(testCase).json")
            guard FileManager.default.fileExists(atPath: path.path) else {
                XCTFail("the case \(testCase) has no expected log \(path.path)")
                continue
            }
            let want = Array(try Data(contentsOf: path))
            XCTAssertEqual(want.last, UInt8(ascii: "\n"), "the expected log of \(testCase) ends in one newline")
            XCTAssertEqual(
                Array(try Data(contentsOf: logs.appendingPathComponent(text))), Array(want.dropLast()),
                "the log of \(testCase) of \(replayed.name) is not its expected log")
        }
        XCTAssertEqual(
            written, expected, "the runner wrote a log of each case that has an expected log, and of no other")
    }

    /// docs/SCE_TTCN3_LANE.md §12.2: signed and 64-bit integers in a message, the verdicts `fail` and
    /// `none`, the two ways to `error` a document leads a run to, a test case that can never end and a
    /// dynamic error, and a branch taken before the states it leaves log their exits
    /// (docs/SCE_TEST_PROFILE.md §4.4).
    func testTheBoundarySuiteReplaysToItsExpectedLogs() throws {
        try replays(
            CorpusReplayed(
                name: "boundary_suite", suite: BoundarySuite.suite(),
                parameter: BoundarySuite.moduleParameter(id:literal:), world: "",
                verdicts: "exchanges_signed_numbers pass\nanswers_another_job fail\nsets_no_verdict none\n"
                    + "never_ends error\nsends_to_a_removed_ptc error\ntakes_before_its_exits pass\n"))
    }

    /// docs/SCE_TEST_PROFILE.md §11.8: a status every matcher of `every_matcher` accepts, at 1 ms,
    /// which the tester's branch reads as a composed message; the helper is told and passes, and the
    /// tester settles for the duration its start gives and passes at 501 ms.
    func testTheJobSuiteReplaysToItsExpectedLogs() throws {
        try replays(
            CorpusReplayed(
                name: "job_suite", suite: JobSuite.suite(), parameter: JobSuite.moduleParameter(id:literal:),
                world: "job-timeout,job-retry", verdicts: "completes_in_time pass\n"))
    }

    /// docs/SCE_TEST_PROFILE.md §9.5: an order of two lines, a customer and a card, which
    /// `order_placed_match` accepts over its union, its list of records and its template references, at
    /// 1 ms.
    func testTheOrdersSuiteReplaysToItsExpectedLogs() throws {
        try replays(
            CorpusReplayed(
                name: "orders_suite", suite: OrdersSuite.suite(),
                parameter: OrdersSuite.moduleParameter(id:literal:), world: "", verdicts: "orders_case pass\n"))
    }

    /// docs/SCE_TEST_PROFILE.md §4.5 (P5), §4.7 (P7): an order at 100 ms, which the atomic state's own
    /// targetless branch takes before its ancestor's, and one at 500 ms, which the ancestor's internal
    /// branch takes, restarting the running timer; the inner timer then runs out twice and the tester
    /// passes at 1250 ms.
    func testTheNestedSuiteReplaysToItsExpectedLogs() throws {
        try replays(
            CorpusReplayed(
                name: "nested_suite", suite: NestedSuite.suite(),
                parameter: NestedSuite.moduleParameter(id:literal:), world: "", verdicts: "nested_case pass\n"))
    }
}
