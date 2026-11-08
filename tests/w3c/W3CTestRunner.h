// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "common/TestUtils.h"
#include "core/LogMacros.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "interfaces/ITestConverter.h"
#include "interfaces/ITestExecutor.h"
#include "interfaces/ITestMetadataParser.h"
#include "interfaces/ITestReporter.h"
#include "interfaces/ITestResultValidator.h"
#include "interfaces/ITestSuite.h"
#include "runtime/EventRaiserImpl.h"
#include "scripting/JSEngine.h"
#include <chrono>
#include <exception>
#include <memory>
#include <optional>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace SCE::W3C {

// W3C Test Runner Configuration Constants
constexpr auto EXECUTOR_DEFAULT_TIMEOUT_MS = std::chrono::milliseconds(5000);  // Executor timeout for test execution
constexpr auto POLL_INTERVAL_MS = std::chrono::milliseconds(10);               // Polling interval for state checks
constexpr auto VALIDATOR_TIMEOUT_MS = std::chrono::milliseconds(10000);        // Validator timeout threshold
constexpr auto CLEANUP_DELAY_MS = std::chrono::milliseconds(10);               // Graceful thread termination delay

// Forward declarations
class W3CHttpTestServer;

/**
 * @brief Factory for creating W3C test components
 *
 * Dependency Inversion: Creates concrete implementations through interfaces
 * Single Responsibility: Only responsible for component creation
 */
/**
 * @brief RAII wrapper for shared test resources (EventRaiser, EventScheduler, EventDispatcher)
 *
 * This struct owns the lifecycle of shared resources that can be reused across multiple
 * StateMachine instances (e.g., in invoke scenarios). The destructor ensures proper
 * cleanup order with EventScheduler's deadlock protection.
 *
 * Separation of concerns
 * - TestResources: Owns EventRaiser/EventScheduler/EventDispatcher (can be shared)
 * - StateMachineContext: Owns only StateMachine (always exclusive)
 */
struct TestResources {
    const std::shared_ptr<SCE::EventRaiserImpl> eventRaiser;
    const std::shared_ptr<SCE::EventSchedulerImpl> scheduler;
    const std::shared_ptr<SCE::EventDispatcherImpl> eventDispatcher;

    TestResources(std::shared_ptr<SCE::EventRaiserImpl> er, std::shared_ptr<SCE::EventSchedulerImpl> sch,
                  std::shared_ptr<SCE::EventDispatcherImpl> ed)
        : eventRaiser(std::move(er)), scheduler(std::move(sch)), eventDispatcher(std::move(ed)) {}

    ~TestResources() {
        // Cleanup order: eventDispatcher -> scheduler -> eventRaiser -> JSEngine registry
        // W3C SCXML compliance: Clean resource release for test isolation

        // 1. Stop dispatching new events (pthread cleanup)
        if (eventDispatcher) {
            eventDispatcher->shutdown();
        }

        // 2. Cancel pending scheduled events
        if (scheduler) {
            scheduler->shutdown(true);
        }

        // 3. Stop event raiser
        if (eventRaiser) {
            eventRaiser->shutdown();
        }

        // 4. Clear JSEngine event raiser registry (prevents cross-test interference)
        // Note: JSEngine is a singleton, so we only clear the registry, not shutdown
        // The engine will be shutdown once at program termination
        SCE::JSEngine::clearEventRaiserRegistry();

        // Small delay for graceful thread termination
        std::this_thread::sleep_for(CLEANUP_DELAY_MS);
    }
};

class TestComponentFactory {
public:
    static std::unique_ptr<ITestConverter> createConverter();
    static std::unique_ptr<ITestMetadataParser> createMetadataParser();
    static std::unique_ptr<ITestExecutor> createExecutor();
    static std::unique_ptr<ITestResultValidator> createValidator();
    static std::unique_ptr<ITestSuite> createTestSuite(const std::string &resourcePath = "resources");
    static std::unique_ptr<ITestReporter> createConsoleReporter();
    static std::unique_ptr<ITestReporter> createXMLReporter(const std::string &outputPath);
    static std::unique_ptr<ITestReporter> createCompositeReporter(std::unique_ptr<ITestReporter> consoleReporter,
                                                                  std::unique_ptr<ITestReporter> xmlReporter);

    /**
     * @brief Create shared test resources with RAII lifecycle management
     *
     * Creates EventRaiser, EventScheduler, and EventDispatcher that can be shared
     * across multiple StateMachine instances. Resources are automatically cleaned up
     * when the returned unique_ptr goes out of scope.
     *
     * Enables resource sharing for W3C invoke scenarios while
     * maintaining clear ownership semantics through RAII.
     *
     * @return Unique pointer to TestResources with automatic cleanup
     */
    static std::unique_ptr<TestResources> createResources();
};

/**
 * @brief Main W3C test runner orchestrator
 *
 * Single Responsibility: Only orchestrates the testing process
 * Dependency Inversion: Depends on interfaces, not concrete implementations
 */
class W3CTestRunner {
private:
    std::unique_ptr<ITestConverter> converter_;
    std::unique_ptr<ITestMetadataParser> metadataParser_;
    std::unique_ptr<ITestExecutor> executor_;
    std::unique_ptr<ITestResultValidator> validator_;
    std::unique_ptr<ITestSuite> testSuite_;
    std::unique_ptr<ITestReporter> reporter_;

    // Failed test log directory (empty = disabled)
    std::string failedLogDir_;

    // The repetition of the run the reports made now belong to (setIteration)
    int iteration_{1};

    /**
     * @brief Run a single test
     */
    TestReport runSingleTest(const std::string &testDirectory);

    /**
     * @brief Cleanup shared state between test executions for proper isolation
     *
     * Ensures no cross-test interference from EventRaiserRegistry entries,
     * stale script engine sessions, or SessionRegistry invoke mappings.
     * Called after each test completes (success or failure).
     */
    void cleanupBetweenTests();

    /**
     * @brief Run a single test with HTTP server for bidirectional communication
     *
     * @param httpServer The in-process listener whose events the run raises,
     *        or null where the host owns the listener (WASM: the host's answers
     *        each POST with the event it made, which HttpEventTarget raises).
     */
    TestReport runSingleTestWithHttpServer(const std::string &testDirectory, W3CHttpTestServer *httpServer);

    /**
     * @brief Run one fixture on the Interpreter, with the BasicHTTP listener
     *        when the registry says it needs one
     *
     * The one place every run mode starts a fixture's listener: natively an
     * in-process W3CHttpTestServer, stopped after the fixture; under WASM the
     * host's, started before main().
     */
    TestReport runInterpreterFixture(const std::string &testDir);

    /**
     * @brief Whether a fixture needs the BasicHTTP listener: the conformance
     *        registry runs it under its `http` harness (W3CTestRegistry)
     *
     * @param testDirectory Path to the test directory
     * @return true if HTTP server is required, false otherwise
     */
    bool requiresHttpServer(const std::string &testDirectory) const;

    /**
     * @brief The report of a BasicHTTP fixture this build does not run
     *
     * Under ThreadSanitizer the fixtures the registry runs under its `http`
     * harness are run on neither engine: the listener's and the client's
     * threads, and getaddrinfo("localhost"), are not TSan-clean. The one
     * decision for both engines, keyed by the fixture's full ID (variant
     * suffix included, e.g. "403a"), the same ID every other report of the
     * fixture carries, so a skip names the fixture it skipped.
     *
     * @param fixtureId The fixture's full ID, as reports carry it
     * @param engineType "interpreter" or "aot"
     * @return The skip report, or std::nullopt when the fixture runs
     */
    static std::optional<TestReport> tsanSkipReport(const std::string &fixtureId, const std::string &engineType);

    /**
     * @brief Run one fixture on the Interpreter and report it
     *
     * The fixture's report under its full ID: the skip report under
     * ThreadSanitizer (tsanSkipReport), an ERROR when the run throws
     * (reportingErrors), or what the run returned.
     */
    TestReport runOnInterpreter(const std::string &testDir);

    /**
     * @brief The document a fixture runs, and the path its relative
     *        references resolve against
     */
    struct FixtureDocument {
        std::string scxml;
        std::string sourcePath;
    };

    /**
     * @brief Read the document a fixture runs
     *
     * The fixture's committed SCXML when it has one, its TXML converted
     * otherwise; the other TXML documents of its directory, which it invokes,
     * are converted beside it.
     *
     * @param testDirectory Path to the test directory ("path/403:a" for a variant)
     * @param testId The fixture's full ID ("403a")
     */
    FixtureDocument loadFixtureDocument(const std::string &testDirectory, const std::string &testId);

    /**
     * @brief Run a manual fixture on the Interpreter and judge it by its verifier
     *
     * A fixture whose metadata says `manual: True` names an observable its
     * final state cannot carry, so the validator cannot judge it: its verifier
     * does (ManualVerification.h), from what the run showed (ManualRun). The
     * report's testType is "manual". A manual fixture with no verifier is not
     * run, and is reported as a skip under its own ID.
     *
     * @param testDirectory Path to the test directory
     * @param report The fixture's report, its ID and metadata read
     * @return The report with the verifier's verdict
     */
    TestReport runManualFixture(const std::string &testDirectory, TestReport report);

    /**
     * @brief Run a single test with AOT engine; throws what the run throws
     *
     * The body of runAotTest, which turns a throw into the test's ERROR report.
     */
    TestReport runAotTestOrThrow(const std::string &testId);

public:
    /**
     * @brief Constructor with dependency injection
     */
    W3CTestRunner(std::unique_ptr<ITestConverter> converter, std::unique_ptr<ITestMetadataParser> metadataParser,
                  std::unique_ptr<ITestExecutor> executor, std::unique_ptr<ITestResultValidator> validator,
                  std::unique_ptr<ITestSuite> testSuite, std::unique_ptr<ITestReporter> reporter);

    ~W3CTestRunner() = default;

    /**
     * @brief The report of a test that was started and did not finish
     *
     * Every test the runner starts, on each engine, leaves exactly one report.
     * A test that throws while its listener starts, while it runs, or while
     * its AOT counterpart runs is reported with TestResult::ERROR rather than
     * left out: a run that drops it reports a smaller total with nothing
     * wrong in it, and exits 0 (TestSummaryHelper::exitStatus counts errors,
     * not absences).
     *
     * @param testId The test's full ID, variant suffix included (e.g., "403a")
     * @param engineType "interpreter" or "aot"
     * @param reason What stopped it (the exception text)
     */
    static TestReport errorReport(const std::string &testId, const std::string &engineType, const std::string &reason);

    /**
     * @brief Run one test on one engine; a throw becomes its ERROR report
     *
     * The one place a started test's exception is turned into a report
     * (errorReport), so no caller keeps its own copy of the conversion.
     */
    template <typename Run>
    static TestReport reportingErrors(const std::string &testId, const std::string &engineType, Run &&run) {
        try {
            return std::forward<Run>(run)();
        } catch (const std::exception &e) {
            SCE_LOG_ERROR("W3C Test {} ({}): the run threw: {}", testId, engineType, e.what());
            return errorReport(testId, engineType, e.what());
        }
    }

    /**
     * @brief Run every W3C fixture on the Interpreter, reporting each
     *
     * Does not begin or end a test run: the caller does, once per
     * invocation, so a run repeated with --repeat keeps the reports of every
     * iteration (setIteration tells them apart).
     *
     * @return The report of every fixture, in discovery order
     */
    std::vector<TestReport> runAllTests();

    /**
     * @brief Hand a report to the run's reporter and return it
     *
     * Every report a run method returns, on either engine, passes here once,
     * stamped with the current iteration, so the console and the XML reporter
     * see the same reports the caller gets back. A caller that makes a report
     * of its own (a test it could not start) hands it here too.
     */
    TestReport reported(TestReport report);

    /**
     * @brief Name the repetition of the run the next reports belong to
     *
     * @param iteration From 1; the CLI's `--repeat N` runs iterations 1..N
     */
    void setIteration(int iteration) {
        iteration_ = iteration;
    }

    /**
     * @brief The full ID of every fixture the suite holds, in discovery order
     *
     * The ID every report of a fixture carries, variant suffix included
     * ("403a"), so a caller that runs fixtures by ID names them as the
     * reports will.
     */
    std::vector<std::string> discoverFixtureIds();

    /**
     * @brief Run a specific test by exact test ID string (e.g., "403a" runs only test403a.scxml)
     *
     * The report is handed to the run's reporter before it is returned.
     *
     * @param testId Exact test ID string (e.g., "403a", "192", "215")
     * @return The test's report, or std::nullopt when no test has this ID
     */
    std::optional<TestReport> runTest(const std::string &testId);

    /**
     * @brief Run all tests matching the given test ID (includes variants)
     *
     * Each report is handed to the run's reporter before it is returned.
     *
     * @param testId W3C test ID (e.g., 403 will run 403a, 403b, 403c if they exist)
     * @param onInterpreter Run each matching test on the Interpreter
     * @param onAot Run each matching test on the AOT engine
     * @return The reports of every matching test on the engines asked for;
     *         empty when no test has this ID (the W3C numbering is sparse)
     */
    std::vector<TestReport> runAllMatchingTests(int testId, bool onInterpreter = true, bool onAot = true);

    /**
     * @brief Run a single test with AOT engine (static generated code)
     *
     * Supports variant test IDs like "403a", "403b", "403c". The report is
     * handed to the run's reporter before it is returned, as an Interpreter
     * report is: both engines report through the one reporter.
     *
     * @param testId The test's full ID (e.g., "144", "403a", "403b")
     * @return TestReport with engineType="aot"; ERROR when the run threw
     */
    TestReport runAotTest(const std::string &testId);

    /**
     * @brief Get test suite for accessing information
     * @return Pointer to test suite interface
     */
    ITestSuite *getTestSuite() const {
        return testSuite_.get();
    }

    /**
     * @brief Get reporter for accessing reporter interface
     * @return Pointer to test reporter interface
     */
    ITestReporter *getReporter() const {
        return reporter_.get();
    }

    /**
     * @brief Enable saving debug logs for failed tests
     *
     * When set, debug-level logs are captured during each test execution.
     * If a test fails, the captured logs are written to a file in the specified directory.
     *
     * @param dir Directory path for failed test log files
     */
    void setFailedLogDir(const std::string &dir) {
        failedLogDir_ = dir;
    }
};

}  // namespace SCE::W3C