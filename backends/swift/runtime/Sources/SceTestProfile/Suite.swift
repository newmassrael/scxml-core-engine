// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A test suite of the test profile as its runner reads it (docs/SCE_TEST_PROFILE.md §4.11, §11.7,
// §11.8, §12.2): the test cases in run order, each with its guard and the selection that runs it in a
// world of the suite's features, and the suite's module parameters; and the runner, which runs the
// cases a world selects one at a time, in that order.
//
// The tables are generated from the `test-case` and `test-suite` documents (a case's configuration from
// its `test-configuration`). They are data: what a case does is its components' machines, which the run
// drives (`TestCaseRun`). The runner is a library function, with no `main`: a generated or hand-written
// test calls it with the system adapters that play the SUT.
//
// Cross-language siblings: `sce_rust_runtime::test::run_suite` (Rust), `SCE::Test::runSuite` (C++),
// `com.sce.runtime.test.runSuite` (Kotlin), `testprofile.RunSuite` (Go),
// `sce_runtime.testprofile.run_suite` (Python), `sce_test_run_suite` (C11).

/// The features a world of a suite's feature model enables (ADR 0057 §2): a configuration of the
/// model, closed under its `requires` edges.
public typealias Features = Set<String>

/// Whether `world` enables `feature`: the one test a selection is made of.
public func enabled(_ world: Features, _ feature: String) -> Bool {
    world.contains(feature)
}

/// A `test-case` (§11.7): its name and what it runs in.
///
/// The configuration and the machines of its components are its configuration's; the module parameters
/// are the ids every component of the configuration reads, which the suite must give.
public struct TestCase {
    /// The test case's name in the suite and in the test log.
    public var name: String
    /// The configuration it runs.
    public var configuration: Configuration
    /// The machines of the configuration's component types.
    public var machines: MachineFactory
    /// The ids of the module parameters its components read.
    public var moduleParameters: [String]

    public init(
        name: String, configuration: Configuration, machines: @escaping MachineFactory,
        moduleParameters: [String] = []
    ) {
        self.name = name
        self.configuration = configuration
        self.machines = machines
        self.moduleParameters = moduleParameters
    }
}

/// A selection (§11.8): whether a world of the suite's features runs a test case. A condition of ADR
/// 0057's grammar, lowered to the features it names.
public typealias Selection = (Features) -> Bool

/// One `<sce:execute>` (§11.8, P11): a test case, its guard, and its selection.
public struct Execute {
    /// The test case.
    public var testCase: TestCase
    /// The guard: the test case's time limit, if it has one.
    public var guardAt: TestDuration?
    /// The selection, or nil for a test case every world runs.
    public var selection: Selection?

    public init(_ testCase: TestCase, guardAt: TestDuration? = nil, selection: Selection? = nil) {
        self.testCase = testCase
        self.guardAt = guardAt
        self.selection = selection
    }
}

/// A `test-suite` (§11.8): its test cases in run order and its module parameters at their defaults.
public struct Suite {
    /// The suite's name.
    public var name: String
    /// The module parameters at their defaults.
    public var moduleParameters: ModuleParameters
    /// The test cases, in run order.
    public var executes: [Execute]

    public init(name: String, moduleParameters: ModuleParameters = ModuleParameters(), executes: [Execute] = []) {
        self.name = name
        self.moduleParameters = moduleParameters
        self.executes = executes
    }
}

/// What the runner reports of one test case it ran.
public struct CaseResult: Equatable, Sendable {
    /// The test case's name.
    public var testCase: String
    /// Its verdict and logs.
    public var result: TestCaseResult

    public init(testCase: String, result: TestCaseResult) {
        self.testCase = testCase
        self.result = result
    }

    /// The test log of this run of the test case in the suite `suite`, in its one canonical text
    /// (`TestCaseResult.canonicalLog`).
    public func canonicalLog(suite: String) -> String {
        result.canonicalLog(suite: suite, testCase: testCase)
    }
}

/// An override of a module parameter the suite does not declare: the caller's mistake, not a verdict of
/// any test case, so no case is run.
public struct OverrideRefusal: Error, Equatable, Sendable {
    /// Why.
    public let reason: String

    public init(_ reason: String) {
        self.reason = reason
    }
}

/// P13: the test case cannot start, so it ends at once with `error`. No component was created, so the
/// test log holds its start and its end alone.
private func refused(_ testCase: String, _ why: String) -> CaseResult {
    CaseResult(
        testCase: testCase,
        result: TestCaseResult(
            verdict: .error, endedAt: 0,
            log: [LogRecord(at: 0, component: mtcName, kind: .error, detail: why)],
            records: [
                TestLogRecord(at: 0, event: .testCaseStarted),
                TestLogRecord(at: 0, event: .testCaseEnded(verdict: .error)),
            ]))
}

/// Runs the test cases of `suite` that `world` selects (P11).
///
/// One at a time, on the calling thread, in the suite's order, each a fresh run from `NOW = 0` with
/// fresh entities against the adapter `adapters` builds for it, given the test case's name, so that no
/// case sees another's trace. Each component reads the suite's module parameters, with the values
/// `overrides` gives in place of their defaults.
///
/// A test case whose components read a module parameter the run has no value for ends with `error`
/// (P13) before any component is created, as does one for which `adapters` builds no adapter.
///
/// - Throws: `OverrideRefusal` when `overrides` gives a module parameter the suite does not declare,
///   before any case runs.
public func runSuite(
    _ suite: Suite, world: Features, adapters: (_ testCase: String) -> (any SystemAdapter)?,
    overrides: ModuleParameters = ModuleParameters()
) throws(OverrideRefusal) -> [CaseResult] {
    var parameters = suite.moduleParameters
    for (id, value) in overrides.values {
        guard parameters.has(id) else {
            throw OverrideRefusal(
                "overrides the module parameter '\(id)', which the suite '\(suite.name)' does not declare")
        }
        parameters.set(id, value)
    }
    var results: [CaseResult] = []
    for execute in suite.executes {
        if let selects = execute.selection, !selects(world) {
            continue
        }
        let testCase = execute.testCase
        if let missing = testCase.moduleParameters.first(where: { !parameters.has($0) }) {
            results.append(
                refused(
                    testCase.name,
                    "the test case reads the module parameter '\(missing)', which is given no value"))
            continue
        }
        guard let adapter = adapters(testCase.name) else {
            results.append(refused(testCase.name, "no system adapter is given for the test case"))
            continue
        }
        let run = TestCaseRun(
            testCase.configuration, testCase.machines, adapter, guardAt: execute.guardAt,
            moduleParameters: parameters)
        results.append(CaseResult(testCase: testCase.name, result: run.run()))
    }
    return results
}
