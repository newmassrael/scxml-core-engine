// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A test suite of the test profile as its runner reads it
// (docs/SCE_TEST_PROFILE.md §4.11, §11.7, §11.8, §12.2): the test cases in run
// order, each with its guard and the selection that runs it in a world of the
// suite's features, and the suite's module parameters; and the runner, which
// runs the cases a world selects one at a time, in that order.
//
// The tables are generated from the `test-case` and `test-suite` documents
// (a case's configuration from its `test-configuration`). They are data: what
// a case does is its components' machines, which the run drives
// (sce/include/test/TestCaseRun.h). The runner is a library function, with no
// `main`: a generated or hand-written test calls it with the system adapters
// that play the SUT (runSuite()), or with the recorded input trace of each
// test case (runSuiteWithTraces()), which the program of a generated suite
// does (sce/include/test/Runner.h).

#pragma once

#include <algorithm>
#include <functional>
#include <memory>
#include <optional>
#include <set>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "test/ModuleParameters.h"
#include "test/TestCaseRun.h"
#include "test/Trace.h"

namespace SCE::Test {

/// The features a world of a suite's feature model enables (ADR 0057 §2): a
/// configuration of the model, closed under its `requires` edges.
using Features = std::set<std::string>;

/// Whether @p world enables @p feature: the one test a selection is made of.
inline bool enabled(const Features &world, const std::string &feature) {
    return world.count(feature) != 0;
}

/**
 * @brief A `test-case` (§11.7): its name and what it runs in
 *
 * The configuration and the machines of its components are its
 * configuration's; the module parameters are the ids every component of the
 * configuration reads, which the suite must give.
 */
struct TestCase {
    /// The test case's name in the suite and in the test log.
    std::string name;
    Configuration configuration;
    MachineFactory machines;
    std::vector<std::string> moduleParameters;
    /// The reader of its configuration's inputs, which a trace replayed for it
    /// is read with (runSuiteWithTraces()): which records each system port
    /// takes, and each record's value from its canonical form.
    InputReader inputs = noInputs;
};

/// A selection (§11.8): whether a world of the suite's features runs a test
/// case. A condition of ADR 0057's grammar, lowered to the features it names.
using Selection = bool (*)(const Features &world);

/// One `<sce:execute>` (§11.8, P11): a test case, its guard, and its selection.
struct Execute {
    TestCase testCase;
    /// The guard: the test case's time limit, if it has one.
    std::optional<Duration> guard;
    /// The selection, or none for a test case every world runs.
    Selection selection = nullptr;
};

/// A `test-suite` (§11.8): its test cases in run order and its module
/// parameters at their defaults.
struct Suite {
    std::string name;
    ModuleParameters moduleParameters;
    std::vector<Execute> executes;
};

/// Builds the system adapter a test case runs against, given the test case's
/// name: a fresh adapter for each case, so that no case sees another's trace.
using AdapterFactory = std::function<std::unique_ptr<SystemAdapter>(const std::string &testCase)>;

/// Gives the recorded input trace of a test case, given its name; throws
/// TraceRefusal, naming why, where there is none or it is not read.
using TraceSource = std::function<Trace(const std::string &testCase)>;

/// What the runner reports of one test case it ran.
struct CaseResult {
    std::string testCase;
    TestCaseResult result;

    /// The test log of this run of the test case in the suite @p suite, in its
    /// one canonical text (TestCaseResult::canonicalLog()).
    std::string canonicalLog(const std::string &suite) const {
        return result.canonicalLog(suite, testCase);
    }
};

namespace detail {

/// The adapter a test case runs against, or, where it has none, why.
struct Adapted {
    std::unique_ptr<SystemAdapter> adapter;
    std::string refusal;
};

/// P13: the test case @p testCase cannot start, so it ends at once with
/// `error`. No component was created, so the test log holds its start and its
/// end alone.
inline CaseResult refusedCase(const std::string &testCase, std::string why) {
    TestCaseResult result;
    result.verdict = Verdict::Error;
    result.log.push_back(LogRecord{0, MTC_NAME, LogKind::Error, std::move(why)});
    result.records.push_back(TestLogRecord{0, TestLog::TestCaseStarted{}});
    result.records.push_back(TestLogRecord{0, TestLog::TestCaseEnded{Verdict::Error}});
    return CaseResult{testCase, std::move(result)};
}

/// The runner of runSuite() and runSuiteWithTraces(): the test cases @p world
/// selects, each against the adapter @p adapterOf builds for it, or ended with
/// `error` for the reason it gives where it builds none.
inline std::vector<CaseResult> runSelected(const Suite &suite, const Features &world, const ModuleParameters &overrides,
                                           const std::function<Adapted(const TestCase &)> &adapterOf) {
    ModuleParameters parameters = suite.moduleParameters;
    for (const auto &[id, value] : overrides.values()) {
        if (!parameters.has(id)) {
            throw std::invalid_argument("overrides the module parameter '" + id + "', which the suite '" + suite.name +
                                        "' does not declare");
        }
        parameters.set(id, value);
    }
    std::vector<CaseResult> results;
    for (const Execute &execute : suite.executes) {
        if (execute.selection != nullptr && !execute.selection(world)) {
            continue;
        }
        const TestCase &testCase = execute.testCase;
        const auto missing = std::find_if(testCase.moduleParameters.begin(), testCase.moduleParameters.end(),
                                          [&](const std::string &id) { return !parameters.has(id); });
        if (missing != testCase.moduleParameters.end()) {
            results.push_back(refusedCase(testCase.name, "the test case reads the module parameter '" + *missing +
                                                             "', which is given no value"));
            continue;
        }
        Adapted adapted = adapterOf(testCase);
        if (!adapted.adapter) {
            results.push_back(refusedCase(testCase.name, std::move(adapted.refusal)));
            continue;
        }
        TestCaseRun run(testCase.configuration, testCase.machines, *adapted.adapter, execute.guard, parameters);
        results.push_back(CaseResult{testCase.name, run.run()});
    }
    return results;
}

}  // namespace detail

/**
 * @brief Runs the test cases of @p suite that @p world selects (P11)
 *
 * One at a time, on the calling thread, in the suite's order, each a fresh run
 * from `NOW = 0` with fresh entities against the adapter @p adapters builds for
 * it. Each component reads the suite's module parameters, with the values
 * @p overrides gives in place of their defaults.
 *
 * A test case whose components read a module parameter the run has no value
 * for ends with `error` (P13) before any component is created, as does one for
 * which @p adapters builds no adapter. An override of a module parameter the
 * suite does not declare is the caller's mistake, not a verdict of any test
 * case: it is refused with std::invalid_argument before any case runs.
 */
inline std::vector<CaseResult> runSuite(const Suite &suite, const Features &world, const AdapterFactory &adapters,
                                        const ModuleParameters &overrides = ModuleParameters()) {
    return detail::runSelected(suite, world, overrides, [&](const TestCase &testCase) {
        std::unique_ptr<SystemAdapter> adapter = adapters(testCase.name);
        if (!adapter) {
            return detail::Adapted{nullptr, "no system adapter is given for the test case"};
        }
        return detail::Adapted{std::move(adapter), {}};
    });
}

/**
 * @brief Runs the test cases of @p suite that @p world selects, each against
 *        its recorded input trace (docs/SCE_TTCN3_LANE.md §10.5)
 *
 * runSuite(), with the adapter of each test case a TraceAdapter that plays the
 * trace @p traces gives for its name, read by the test case's configuration
 * (TestCase::inputs).
 *
 * A test case whose trace is not given, is of another suite or test case, or
 * does not fit its configuration (a port that is no system port of it, a type
 * its port does not take, a value that does not belong to its type) ends with
 * `error` (P13) before any component is created, its log line naming why. So
 * does one whose components read a module parameter the run has no value for,
 * as under runSuite(), before its trace is asked for.
 */
inline std::vector<CaseResult> runSuiteWithTraces(const Suite &suite, const Features &world, const TraceSource &traces,
                                                  const ModuleParameters &overrides = ModuleParameters()) {
    return detail::runSelected(suite, world, overrides, [&](const TestCase &testCase) {
        try {
            const Trace trace = traces(testCase.name);
            if (trace.suite != suite.name || trace.testCase != testCase.name) {
                return detail::Adapted{nullptr, "the trace is of the test case '" + trace.testCase +
                                                    "' of the suite '" + trace.suite + "'"};
            }
            return detail::Adapted{std::make_unique<TraceAdapter>(trace, testCase.inputs), {}};
        } catch (const TraceRefusal &refusal) {
            return detail::Adapted{nullptr, "the trace is refused: " + refusal.reason()};
        }
    });
}

}  // namespace SCE::Test
