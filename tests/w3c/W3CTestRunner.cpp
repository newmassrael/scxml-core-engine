// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "W3CTestRunner.h"
#include "BasicHttpTestEndpoint.h"
#include "ManualVerification.h"
#ifndef __EMSCRIPTEN__
#include "W3CHttpTestServer.h"
#endif
#include "common/Logger.h"
#include "core/LogMacros.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventRaiserService.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "impl/TXMLConverter.h"
#include "impl/TestMetadataParser.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "runtime/StateMachineBuilder.h"
#include "runtime/StateMachineContext.h"
#include "scripting/ScriptEngineProvider.h"
#include "scripting/SessionRegistry.h"
#include <algorithm>
#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <thread>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#include <emscripten/heap.h>
#endif

// AOT Test Registry (for registry-based test execution)
#include "aot_tests/AotTestRegistry.h"

// W3C Test Registry (Single Source of Truth for AOT vs Interpreter classification)
#include "W3CTestRegistry.h"

// Test Summary Helper (Single Source of Truth for summary calculation)
#include "TestSummaryHelper.h"

namespace SCE::W3C {

// Forward declaration for implementations
class TestResultValidator;
class W3CTestSuite;

// Factory implementations
std::unique_ptr<ITestConverter> TestComponentFactory::createConverter() {
    return std::make_unique<TXMLConverter>();
}

std::unique_ptr<ITestMetadataParser> TestComponentFactory::createMetadataParser() {
    return std::make_unique<TestMetadataParser>();
}

std::unique_ptr<ITestExecutor> TestComponentFactory::createExecutor() {
    // W3C SCXML compliance: Use real StateMachine with full invoke support
    class StateMachineTestExecutor : public ITestExecutor {
    private:
        std::chrono::milliseconds timeout_{EXECUTOR_DEFAULT_TIMEOUT_MS};

    public:
        void setTimeout(std::chrono::milliseconds timeoutMs) override {
            timeout_ = timeoutMs;
        }

        TestExecutionContext executeTest(const std::string &scxmlContent, const TestMetadata &metadata) override {
            auto startTime = std::chrono::steady_clock::now();

            TestExecutionContext testContext;
            testContext.scxmlContent = scxmlContent;
            testContext.metadata = metadata;
            // W3C tests should pass when correctly implemented, regardless of conformance level
            // conformance indicates whether implementation is required, not expected outcome
            testContext.expectedTarget = "pass";

            // Create shared resources using RAII factory pattern
            auto resources = TestComponentFactory::createResources();

            try {
                SCE_LOG_DEBUG("StateMachineTestExecutor: Starting test execution for test {}", metadata.id);

                // Build StateMachine with resource injection, then wrap in RAII context
                auto stateMachineUnique = SCE::StateMachineBuilder()
                                              .withScriptEngine(SCE::ScriptEngineProvider::getScriptEngine())
                                              .withEventDispatcher(resources->eventDispatcher)
                                              .withEventRaiser(resources->eventRaiser)
                                              .build();

                // Wrap in StateMachineContext for RAII cleanup
                auto smContext = std::make_unique<SCE::StateMachineContext>(std::move(stateMachineUnique));
                auto *stateMachine = smContext->get();

                // W3C SCXML compliance: EventRaiser callback should pass eventData to StateMachine
                // StateMachine will automatically set its own callback, so we don't override it here
                // eventRaiser->setEventCallback(...); // Removed - let StateMachine set its own callback

                // Load SCXML content
                if (!stateMachine->loadSCXMLFromString(scxmlContent)) {
                    SCE_LOG_ERROR("StateMachineTestExecutor: Failed to load SCXML content");
                    testContext.finalState = "error";
                    testContext.errorMessage = "Failed to load SCXML content";
                    return testContext;
                }

                // W3C SCXML compliance: Ensure EventRaiser callback is properly set after SCXML loading
                // This guarantees that any initialization during SCXML loading doesn't override our callback
                stateMachine->setEventRaiser(resources->eventRaiser);

                // Start the state machine
                if (!stateMachine->start()) {
                    SCE_LOG_ERROR("StateMachineTestExecutor: Failed to start StateMachine");
                    testContext.finalState = "error";
                    testContext.errorMessage = "Failed to start StateMachine";
                    return testContext;
                }

                // Wait for StateMachine to reach final state or timeout
                auto waitStart = std::chrono::steady_clock::now();
                std::string currentState;

                while (std::chrono::steady_clock::now() - waitStart < timeout_) {
                    // W3C SCXML compliance: Process queued events before checking state
                    // This ensures events from child invokes (event1, done.invoke) are processed
                    resources->eventRaiser->processQueuedEvents();

                    currentState = stateMachine->terminalState().value_or(stateMachine->getCurrentState());

                    // Check if we reached a final state (pass or fail)
                    if (currentState == "pass" || currentState == "fail") {
                        SCE_LOG_DEBUG("StateMachineTestExecutor: Reached final state: {}", currentState);
                        break;
                    }

                    // Small sleep to avoid busy waiting
#ifdef __EMSCRIPTEN__
                    // W3C SCXML C.2: CRITICAL - Yield to Node.js event loop
                    // emscripten_sleep() allows HTTP callbacks to execute during polling
                    emscripten_sleep(static_cast<int>(POLL_INTERVAL_MS.count()));
#else
                    std::this_thread::sleep_for(POLL_INTERVAL_MS);
#endif
                }

                // Get final state - always read fresh state after loop exit
                testContext.finalState = stateMachine->terminalState().value_or(stateMachine->getCurrentState());
                SCE_LOG_DEBUG("StateMachineTestExecutor: Test completed with final state: {}", testContext.finalState);

                auto endTime = std::chrono::steady_clock::now();
                testContext.executionTime = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

                // RAII cleanup: StateMachineContext destructor handles all cleanup automatically
                SCE_LOG_DEBUG("StateMachineTestExecutor: Automatic cleanup will occur on scope exit");
                return testContext;

            } catch (const std::exception &e) {
                SCE_LOG_ERROR("StateMachineTestExecutor: Exception during test execution: {}", e.what());
                testContext.finalState = "error";
                testContext.errorMessage = "Exception: " + std::string(e.what());

                auto endTime = std::chrono::steady_clock::now();
                testContext.executionTime = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

                // RAII cleanup: StateMachineContext destructor handles cleanup on exception
                SCE_LOG_DEBUG("StateMachineTestExecutor: Automatic cleanup will occur on scope exit after exception");
                return testContext;
            }
        }

        TestExecutionContext executeTest(const std::string &scxmlContent, const TestMetadata &metadata,
                                         const std::string &sourceFilePath) override {
            auto startTime = std::chrono::steady_clock::now();

            TestExecutionContext testContext;
            testContext.scxmlContent = scxmlContent;
            testContext.metadata = metadata;
            testContext.expectedTarget = "pass";

            // Create shared resources using RAII factory pattern
            auto resources = TestComponentFactory::createResources();

            try {
                SCE_LOG_DEBUG("StateMachineTestExecutor: Starting test execution for test {} with source path: {}",
                              metadata.id, sourceFilePath);

                // Build StateMachine with resource injection, then wrap in RAII context
                auto stateMachineUnique = SCE::StateMachineBuilder()
                                              .withScriptEngine(SCE::ScriptEngineProvider::getScriptEngine())
                                              .withEventDispatcher(resources->eventDispatcher)
                                              .withEventRaiser(resources->eventRaiser)
                                              .build();

                // Wrap in StateMachineContext for RAII cleanup
                auto smContext = std::make_unique<SCE::StateMachineContext>(std::move(stateMachineUnique));
                auto *stateMachine = smContext->get();

                // Register source file path for relative path resolution before loading SCXML
                SCE::SessionRegistry::instance().registerSessionFilePath(stateMachine->getSessionId(), sourceFilePath);
                SCE_LOG_DEBUG("StateMachineTestExecutor: Registered source file path '{}' for session '{}'",
                              sourceFilePath, stateMachine->getSessionId());

                // Load SCXML content
                if (!stateMachine->loadSCXMLFromString(scxmlContent)) {
                    SCE_LOG_ERROR("StateMachineTestExecutor: Failed to load SCXML content");
                    testContext.finalState = "error";
                    testContext.errorMessage = "Failed to load SCXML content";
                    return testContext;
                }

                // Set EventRaiser and start the state machine
                stateMachine->setEventRaiser(resources->eventRaiser);
                if (!stateMachine->start()) {
                    SCE_LOG_ERROR("StateMachineTestExecutor: Failed to start StateMachine");
                    testContext.finalState = "error";
                    testContext.errorMessage = "Failed to start StateMachine";
                    return testContext;
                }

                // Wait for StateMachine to reach final state or timeout
                auto waitStart = std::chrono::steady_clock::now();
                std::string currentState;

                while (std::chrono::steady_clock::now() - waitStart < timeout_) {
                    // W3C SCXML compliance: Process queued events before checking state
                    // This ensures events from child invokes (event1, done.invoke) are processed
                    resources->eventRaiser->processQueuedEvents();

                    currentState = stateMachine->terminalState().value_or(stateMachine->getCurrentState());
                    if (currentState == "pass" || currentState == "fail") {
                        SCE_LOG_DEBUG("StateMachineTestExecutor: Reached final state: {}", currentState);
                        break;
                    }
                    std::this_thread::sleep_for(POLL_INTERVAL_MS);
                }

                // Get final state - always read fresh state after loop exit
                testContext.finalState = stateMachine->terminalState().value_or(stateMachine->getCurrentState());
                SCE_LOG_DEBUG("StateMachineTestExecutor: Test completed with final state: {}", testContext.finalState);

                auto endTime = std::chrono::steady_clock::now();
                testContext.executionTime = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

                // RAII cleanup: StateMachineContext destructor handles all cleanup automatically
                SCE_LOG_DEBUG("StateMachineTestExecutor: Automatic cleanup will occur on scope exit");
                return testContext;

            } catch (const std::exception &e) {
                SCE_LOG_ERROR("StateMachineTestExecutor: Exception during test execution: {}", e.what());
                testContext.finalState = "error";
                testContext.errorMessage = "Exception: " + std::string(e.what());

                auto endTime = std::chrono::steady_clock::now();
                testContext.executionTime = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

                // RAII cleanup: StateMachineContext destructor handles cleanup on exception
                SCE_LOG_DEBUG("StateMachineTestExecutor: Automatic cleanup will occur on scope exit after exception");
                return testContext;
            }
        }
    };

    return std::make_unique<StateMachineTestExecutor>();
}

std::unique_ptr<ITestResultValidator> TestComponentFactory::createValidator() {
    class TestResultValidator : public ITestResultValidator {
    public:
        ValidationResult validateResult(const TestExecutionContext &context) override {
            // A manual fixture's final state is not its verdict: its verifier
            // judges it (ManualVerification.h), never this
            if (context.metadata.manual) {
                return ValidationResult::skip("Skipped: manual test");
            }

            if (!context.errorMessage.empty()) {
                return ValidationResult(false, TestResult::ERROR, "Execution error: " + context.errorMessage);
            }

            if (context.executionTime > VALIDATOR_TIMEOUT_MS) {
                return ValidationResult(false, TestResult::TIMEOUT, "Test execution timed out");
            }

            return validateFinalState(context);
        }

        bool shouldSkipTest(const TestMetadata &metadata) override {
            return metadata.manual;
        }

    private:
        ValidationResult validateFinalState(const TestExecutionContext &context) {
            const std::string &finalState = context.finalState;
            const std::string &expectedTarget = context.expectedTarget;

            if (expectedTarget == "unknown") {
                return ValidationResult(false, TestResult::ERROR, "Cannot determine expected test outcome");
            }

            if (finalState == expectedTarget) {
                if (expectedTarget == "pass") {
                    return ValidationResult(true, TestResult::PASS, "Test reached expected pass state");
                } else {
                    return ValidationResult(true, TestResult::FAIL, "Test reached expected fail state");
                }
            }

            if (expectedTarget == "pass" && finalState == "fail") {
                return ValidationResult(true, TestResult::FAIL, "Test should pass but reached fail state");
            }

            if (expectedTarget == "fail" && finalState == "pass") {
                return ValidationResult(true, TestResult::FAIL, "Test should fail but reached pass state");
            }

            return ValidationResult(false, TestResult::ERROR, "Test ended in unknown state: " + finalState);
        }
    };

    return std::make_unique<TestResultValidator>();
}

std::unique_ptr<ITestSuite> TestComponentFactory::createTestSuite(const std::string &resourcePath) {
    class W3CTestSuite : public ITestSuite {
    private:
        std::string resourcePath_;

        bool isNumericTestDir(const std::string &dirName) {
            return !dirName.empty() && std::all_of(dirName.begin(), dirName.end(), ::isdigit);
        }

        static int extractTestId(const std::string &testPath) {
            std::filesystem::path path(testPath);
            std::string dirName = path.filename().string();
            try {
                return std::stoi(dirName);
            } catch (...) {
                return 0;
            }
        }

    public:
        explicit W3CTestSuite(const std::string &resourcePath) : resourcePath_(resourcePath) {}

        TestSuiteInfo getInfo() const override {
            TestSuiteInfo info;
            info.name = "W3C SCXML Test Suite";
            info.description = "Official W3C SCXML 1.0 Conformance Tests";
            info.resourcePath = resourcePath_;

            try {
                auto tests = const_cast<W3CTestSuite *>(this)->discoverTests();
                info.totalTests = tests.size();
            } catch (...) {
                info.totalTests = 0;
            }

            return info;
        }

        std::vector<std::string> discoverTests() override {
            std::vector<std::string> testDirs;

            try {
                for (const auto &entry : std::filesystem::directory_iterator(resourcePath_)) {
                    if (entry.is_directory()) {
                        std::string dirName = entry.path().filename().string();

                        if (isNumericTestDir(dirName)) {
                            std::string txmlPath = getTXMLPath(entry.path().string());
                            std::string metadataPath = getMetadataPath(entry.path().string());

                            if (std::filesystem::exists(txmlPath)) {
                                // Main test file exists - add it
                                testDirs.push_back(entry.path().string());
                            } else {
                                // Check for variant test files (test403a.txml, test403b.txml, etc.)
                                int testId = extractTestId(entry.path().string());
                                bool hasVariants = false;

                                // Common variant suffixes: a, b, c, d, e
                                for (char suffix = 'a'; suffix <= 'z'; ++suffix) {
                                    std::string variantPath =
                                        entry.path().string() + "/test" + std::to_string(testId) + suffix + ".txml";
                                    if (std::filesystem::exists(variantPath)) {
                                        // Add variant as separate test (with suffix in path for identification)
                                        testDirs.push_back(entry.path().string() + ":" + std::string(1, suffix));
                                        hasVariants = true;
                                    } else {
                                        // Stop checking once we hit a missing variant
                                        break;
                                    }
                                }

                                if (!hasVariants) {
                                    // No main file and no variants - skip this test
                                    SCE_LOG_DEBUG("W3CTestSuite: No TXML file found for test {}", testId);
                                }
                            }
                        }
                    }
                }
            } catch (const std::exception &e) {
                throw std::runtime_error("Failed to discover W3C tests: " + std::string(e.what()));
            }

            std::sort(testDirs.begin(), testDirs.end(), [](const std::string &a, const std::string &b) {
                int idA = extractTestId(a);
                int idB = extractTestId(b);
                if (idA != idB) {
                    return idA < idB;
                }
                // Same test ID, compare by variant suffix (e.g., ":a" < ":b" < ":c")
                return a < b;
            });

            return testDirs;
        }

        std::string getTXMLPath(const std::string &testDirectory) override {
            // Check if path contains variant suffix (format: "path/403:a")
            size_t colonPos = testDirectory.find(':');
            if (colonPos != std::string::npos) {
                // Extract base path and variant suffix
                std::string basePath = testDirectory.substr(0, colonPos);
                std::string variant = testDirectory.substr(colonPos + 1);
                int testId = extractTestId(basePath);
                return basePath + "/test" + std::to_string(testId) + variant + ".txml";
            }

            // Normal path without variant
            int testId = extractTestId(testDirectory);
            return testDirectory + "/test" + std::to_string(testId) + ".txml";
        }

        std::string getMetadataPath(const std::string &testDirectory) override {
            // Remove variant suffix if present (format: "path/403:a")
            size_t colonPos = testDirectory.find(':');
            if (colonPos != std::string::npos) {
                std::string basePath = testDirectory.substr(0, colonPos);
                return basePath + "/metadata.txt";
            }
            return testDirectory + "/metadata.txt";
        }
    };

    return std::make_unique<W3CTestSuite>(resourcePath);
}

std::unique_ptr<ITestReporter> TestComponentFactory::createConsoleReporter() {
    class ConsoleTestReporter : public ITestReporter {
    private:
        size_t testCount_ = 0;

    public:
        void beginTestRun(const std::string &testSuiteName) override {
            SCE_LOG_INFO("=== {} ===", testSuiteName);
            testCount_ = 0;
        }

        void reportTestResult(const TestReport &report) override {
            ++testCount_;

            SCE_LOG_INFO("[{}] Test {} ({}): {}", testCount_, report.testId, report.metadata.specnum,
                         testResultToString(report.validationResult.finalResult));

            if (report.validationResult.finalResult != TestResult::PASS) {
                SCE_LOG_INFO(" - {}", report.validationResult.reason);
            }

            SCE_LOG_INFO(" ({}ms)", report.executionContext.executionTime.count());
        }

        void generateSummary(const TestRunSummary &summary) override {
            SCE_LOG_INFO("\n=== Test Results Summary ===");
            SCE_LOG_INFO("Total tests: {}", summary.totalTests);
            SCE_LOG_INFO("Passed: {}", summary.passedTests);
            SCE_LOG_INFO("Failed: {}", summary.failedTests);
            SCE_LOG_INFO("Errors: {}", summary.errorTests);
            SCE_LOG_INFO("Skipped: {}", summary.skippedTests);
            SCE_LOG_INFO("Pass rate: {}%", summary.passRate);
            SCE_LOG_INFO("Total time: {}ms", summary.totalExecutionTime.count());
        }

        void endTestRun() override {
            // Do not output end message - let main runner handle it
        }

        std::string getOutputDestination() const override {
            return "Console";
        }
    };

    return std::make_unique<ConsoleTestReporter>();
}

std::unique_ptr<ITestReporter> TestComponentFactory::createXMLReporter(const std::string &outputPath) {
    // Single Responsibility: XML test result reporting
    class XMLTestReporter : public ITestReporter {
    private:
        std::string outputPath_;
        size_t testCount_ = 0;
        std::vector<TestReport> allReports_;

    public:
        explicit XMLTestReporter(const std::string &outputPath) : outputPath_(outputPath) {}

        void beginTestRun(const std::string &testSuiteName) override {
            testCount_ = 0;
            allReports_.clear();
            SCE_LOG_INFO("=== {} (Writing to XML: {}) ===", testSuiteName, outputPath_);
        }

        void reportTestResult(const TestReport &report) override {
            ++testCount_;
            allReports_.push_back(report);
            // XMLReporter only stores data - no console output
        }

        // The counts written are tallied from the reports this reporter heard,
        // not taken from the caller's summary, so the file's header always
        // agrees with the testcases it lists.
        void generateSummary([[maybe_unused]] const TestRunSummary &summary) override {
            // Write XML report file with separate testsuites for each engine
            std::ofstream xmlFile(outputPath_);
            if (xmlFile) {
                // Separate reports by engine type
                std::vector<TestReport> interpreterReports;
                std::vector<TestReport> aotReports;

                for (const auto &report : allReports_) {
                    if (report.engineType == "interpreter") {
                        interpreterReports.push_back(report);
                    } else if (report.engineType == "aot") {
                        aotReports.push_back(report);
                    }
                }

                // Helper function to escape XML special characters (W3C XML compliance)
                auto escapeXml = [](const std::string &str) -> std::string {
                    std::string escaped;
                    escaped.reserve(str.size());
                    for (char c : str) {
                        switch (c) {
                        case '&':
                            escaped += "&amp;";
                            break;
                        case '<':
                            escaped += "&lt;";
                            break;
                        case '>':
                            escaped += "&gt;";
                            break;
                        case '"':
                            escaped += "&quot;";
                            break;
                        case '\'':
                            escaped += "&apos;";
                            break;
                        default:
                            escaped += c;
                            break;
                        }
                    }
                    return escaped;
                };

                // A run repeated with --repeat reports each test once per
                // iteration. Its testcases are named "Test_<id>@<iteration>"
                // (1-based) so every one stays distinct, and a run of one
                // iteration keeps the plain "Test_<id>". The test's own ID
                // and the iteration are attributes as well, for a reader that
                // wants them apart.
                const bool repeated = std::any_of(allReports_.begin(), allReports_.end(),
                                                  [](const TestReport &report) { return report.iteration > 1; });

                // Helper function to write XML testcase element (Zero Duplication Principle)
                auto writeTestCase = [&escapeXml, repeated](std::ofstream &xmlFile, const TestReport &report,
                                                            const std::string &classname) {
                    const TestResult result = report.validationResult.finalResult;
                    std::string name = "Test_" + report.testId;
                    if (repeated) {
                        name += "@" + std::to_string(report.iteration);
                    }
                    xmlFile << "    <testcase classname=\"" << classname << "\" "
                            << "name=\"" << escapeXml(name) << "\" "
                            << "id=\"" << escapeXml(report.testId) << "\" "
                            << "iteration=\"" << report.iteration << "\" "
                            << "time=\"" << (report.executionContext.executionTime.count() / 1000.0) << "\" "
                            << "type=\"" << report.testType << "\" "
                            << "result=\"" << testResultToString(result) << "\" "
                            << "description=\"" << escapeXml(report.validationResult.reason) << "\"";

                    if (result == TestResult::SKIPPED) {
                        // JUnit's own mark for a test that was not run, so
                        // no reader counts it as a pass or a failure
                        xmlFile << ">" << std::endl;
                        xmlFile << "      <skipped message=\"" << escapeXml(report.validationResult.reason) << "\"/>"
                                << std::endl;
                        xmlFile << "    </testcase>" << std::endl;
                    } else if (failsTheRun(result)) {
                        xmlFile << ">" << std::endl;
                        xmlFile << "      <failure message=\"" << escapeXml(report.validationResult.reason) << "\"/>"
                                << std::endl;
                        xmlFile << "    </testcase>" << std::endl;
                    } else {
                        xmlFile << "/>" << std::endl;
                    }
                };

                // The counts are the tally every other reader uses
                // (TestSummaryHelper), over the reports this file lists, so
                // the header and the testcases cannot disagree.
                auto writeTotals = [](std::ofstream &xmlFile, const TestRunSummary &tally) {
                    xmlFile << "tests=\"" << tally.totalTests << "\" "
                            << "failures=\"" << tally.failedTests << "\" "
                            << "errors=\"" << tally.errorTests << "\" "
                            << "skipped=\"" << tally.skippedTests << "\" "
                            << "time=\"" << (tally.totalExecutionTime.count() / 1000.0) << "\"";
                };

                auto writeSuite = [&](const std::vector<TestReport> &reports, const char *suiteName,
                                      const std::string &classname) {
                    if (reports.empty()) {
                        return;
                    }
                    xmlFile << "  <testsuite name=\"" << suiteName << "\" ";
                    writeTotals(xmlFile, Common::TestSummaryHelper::calculateSummary(reports));
                    xmlFile << ">" << std::endl;
                    for (const auto &report : reports) {
                        writeTestCase(xmlFile, report, classname);
                    }
                    xmlFile << "  </testsuite>" << std::endl;
                };

                // Write XML with separate testsuites
                xmlFile << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>" << std::endl;
                xmlFile << "<testsuites ";
                writeTotals(xmlFile, Common::TestSummaryHelper::calculateSummary(allReports_));
                xmlFile << ">" << std::endl;

                writeSuite(interpreterReports, "W3C_SCXML_Interpreter", "W3C_Interpreter");
                writeSuite(aotReports, "W3C_SCXML_AOT", "W3C_AOT");

                xmlFile << "</testsuites>" << std::endl;
                xmlFile.close();

                // Generate HTML report using Python script
                // Script location: tests/w3c/scripts/xml_to_html.py (version-controlled)
                std::filesystem::path scriptPath =
                    std::filesystem::path(__FILE__).parent_path() / "scripts" / "xml_to_html.py";
                std::filesystem::path scriptDir = scriptPath.parent_path();
                std::filesystem::path xmlPathObj(outputPath_);

                // Make XML path absolute if relative
                if (xmlPathObj.is_relative()) {
                    xmlPathObj = std::filesystem::current_path() / xmlPathObj;
                }

                // Check if Python script exists
                if (std::filesystem::exists(scriptPath)) {
                    // Redirect stderr to temporary file for detailed error capture
                    std::string errorLogPath = (xmlPathObj.parent_path() / "html_generation_error.log").string();
                    std::string command =
                        "python3 " + scriptPath.string() + " " + xmlPathObj.string() + " 2> " + errorLogPath;
                    SCE_LOG_DEBUG("Executing HTML generation: {}", command);

                    int result = std::system(command.c_str());

                    if (result == 0) {
                        std::string htmlPath = outputPath_;
                        htmlPath.replace(htmlPath.find(".xml"), 4, ".html");
                        SCE_LOG_INFO("HTML report generated: {}", htmlPath);

                        // Clean up error log if successful
                        std::filesystem::remove(errorLogPath);
                    } else {
                        SCE_LOG_WARN("Failed to generate HTML report (exit code: {})", result);

                        // Read and log error details if available
                        std::ifstream errorLog(errorLogPath);
                        if (errorLog.is_open()) {
                            std::string errorLine;
                            SCE_LOG_DEBUG("Python script error details:");
                            while (std::getline(errorLog, errorLine)) {
                                SCE_LOG_DEBUG("  {}", errorLine);
                            }
                            errorLog.close();
                        }
                    }
                } else {
                    SCE_LOG_DEBUG("HTML generation script not found: {}", scriptPath.string());
                }
            }

            // Do not show console summary - let main runner handle it
        }

        void endTestRun() override {
            // XMLReporter only writes files - no console output
        }

        std::string getOutputDestination() const override {
            return outputPath_;
        }

        std::vector<TestReport> getAllReports() const override {
            return allReports_;
        }
    };

    return std::make_unique<XMLTestReporter>(outputPath);
}

std::unique_ptr<ITestReporter>
TestComponentFactory::createCompositeReporter(std::unique_ptr<ITestReporter> consoleReporter,
                                              std::unique_ptr<ITestReporter> xmlReporter) {
    // Composite Pattern: Combines multiple reporters
    class CompositeTestReporter : public ITestReporter {
    private:
        std::unique_ptr<ITestReporter> consoleReporter_;
        std::unique_ptr<ITestReporter> xmlReporter_;

    public:
        CompositeTestReporter(std::unique_ptr<ITestReporter> consoleReporter,
                              std::unique_ptr<ITestReporter> xmlReporter)
            : consoleReporter_(std::move(consoleReporter)), xmlReporter_(std::move(xmlReporter)) {}

        void beginTestRun(const std::string &suiteName) override {
            consoleReporter_->beginTestRun(suiteName);
            xmlReporter_->beginTestRun(suiteName);
        }

        void reportTestResult(const TestReport &report) override {
            consoleReporter_->reportTestResult(report);
            xmlReporter_->reportTestResult(report);
        }

        void generateSummary(const TestRunSummary &summary) override {
            consoleReporter_->generateSummary(summary);
            xmlReporter_->generateSummary(summary);
        }

        void endTestRun() override {
            consoleReporter_->endTestRun();
            xmlReporter_->endTestRun();
        }

        std::string getOutputDestination() const override {
            // Return XML reporter's destination as the primary output
            return xmlReporter_->getOutputDestination();
        }

        std::vector<TestReport> getAllReports() const override {
            // Get reports from XML reporter which stores all reports
            return xmlReporter_->getAllReports();
        }
    };

    return std::make_unique<CompositeTestReporter>(std::move(consoleReporter), std::move(xmlReporter));
}

std::unique_ptr<TestResources> TestComponentFactory::createResources() {
    // Create EventRaiser
    auto eventRaiser = std::make_shared<SCE::EventRaiserImpl>();

    // Create EventScheduler with event execution callback
    auto scheduler = std::make_shared<SCE::EventSchedulerImpl>([](const SCE::EventDescriptor &event,
                                                                  std::shared_ptr<SCE::IEventTarget> target,
                                                                  const std::string &sendId) -> bool {
        // Event execution callback: send event to target and return success status
        SCE_LOG_DEBUG("EventScheduler: Executing event '{}' with sendId '{}' on target '{}'", event.eventName, sendId,
                      target->getDebugInfo());

        auto future = target->send(event);
        try {
            auto sendResult = future.get();
            if (sendResult.isSuccess) {
                SCE_LOG_DEBUG("EventScheduler: Event '{}' (sendId: '{}') executed successfully", event.eventName,
                              sendId);
            } else {
                SCE_LOG_WARN("EventScheduler: Event '{}' (sendId: '{}') execution failed", event.eventName, sendId);
            }
            return sendResult.isSuccess;
        } catch (const std::exception &e) {
            SCE_LOG_ERROR("EventScheduler: Failed to send event '{}' (sendId: '{}'): {}", event.eventName, sendId,
                          e.what());
            return false;
        }
    });

    // W3C SCXML 6.2: Connect EventRaiser to EventScheduler for delayed event polling (WASM support)
    eventRaiser->setScheduler(scheduler);

    // Create EventTargetFactory and EventDispatcher
    auto targetFactory = std::make_shared<SCE::EventTargetFactoryImpl>(eventRaiser);
    auto eventDispatcher = std::make_shared<SCE::EventDispatcherImpl>(scheduler, targetFactory);

    // Create TestResources with const fields initialized via constructor
    return std::make_unique<TestResources>(std::move(eventRaiser), std::move(scheduler), std::move(eventDispatcher));
}

// W3CTestRunner implementation
/**
 * @brief Save captured debug logs to a file for a failed test
 *
 * @param logDir Directory to save the log file
 * @param testId Test ID (e.g., "446", "403a")
 * @param engineType Engine type ("interpreter" or "aot")
 * @param report The test report with result details
 * @param capturedLogs The captured log buffer
 */
static void saveFailedTestLog(const std::string &logDir, const std::string &testId, const std::string &engineType,
                              const TestReport &report, const std::string &capturedLogs) {
    if (logDir.empty() || capturedLogs.empty()) {
        return;
    }

    try {
        std::filesystem::create_directories(logDir);

        std::string filename = "Test_" + testId + "_" + engineType + ".log";
        std::filesystem::path logPath = std::filesystem::path(logDir) / filename;

        std::ofstream file(logPath);
        if (!file.is_open()) {
            SCE_LOG_WARN("Failed to open log file for writing: {}", logPath.string());
            return;
        }

        // Write header with test metadata
        file << "=== Failed Test Log ===" << "\n";
        file << "Test ID:     " << testId << "\n";
        file << "Engine:      " << engineType << "\n";
        file << "Test Type:   " << report.testType << "\n";
        file << "Result:      " << testResultToString(report.validationResult.finalResult) << "\n";
        file << "Reason:      " << report.validationResult.reason << "\n";
        file << "Final State: " << report.executionContext.finalState << "\n";
        file << "Duration:    " << report.executionContext.executionTime.count() << "ms" << "\n";
        if (!report.metadata.description.empty()) {
            file << "Description: " << report.metadata.description << "\n";
        }
        if (!report.metadata.specnum.empty()) {
            file << "Spec:        " << report.metadata.specnum << "\n";
        }
        file << "\n=== Debug Logs ===" << "\n";
        file << capturedLogs;

        SCE_LOG_INFO("Saved failed test log: {}", logPath.string());
    } catch (const std::exception &e) {
        SCE_LOG_WARN("Failed to save test log for test {}: {}", testId, e.what());
    }
}

/**
 * @brief The fixture ID a test directory names: `.../201` is "201", and
 *        `.../403:a` (a variant) is "403a"
 */
static std::string fixtureIdOf(const std::string &testDirectory) {
    std::string fixtureId = std::filesystem::path(testDirectory).filename().string();
    fixtureId.erase(std::remove(fixtureId.begin(), fixtureId.end(), ':'), fixtureId.end());
    return fixtureId;
}

/**
 * @brief Wait one poll interval, yielding to the host's event loop under WASM
 *        as the executor's wait does
 */
static void waitOnePollInterval() {
#ifdef __EMSCRIPTEN__
    emscripten_sleep(static_cast<int>(POLL_INTERVAL_MS.count()));
#else
    std::this_thread::sleep_for(POLL_INTERVAL_MS);
#endif
}

/**
 * @brief Run a manual fixture's document on the Interpreter and record what
 *        the run showed
 *
 * The machine runs until it halts or EXECUTOR_DEFAULT_TIMEOUT_MS ends the run,
 * and is watched on until `watchAtLeast` from its start has passed, since what
 * a verifier judges can come after the halt. The queued events of every
 * session the run started, the delayed ones the scheduler polls for included,
 * are delivered on each turn; once the machine has halted they are offered to
 * a machine that must refuse them.
 *
 * @return Why the run could not be started; empty when it started, or when
 *         the document was rejected at load (which some verdicts ask for)
 */
static std::string watchManualRun(const std::string &scxml, const std::string &sourcePath,
                                  std::chrono::milliseconds watchAtLeast, ManualRun &run) {
    auto resources = TestComponentFactory::createResources();

    auto stateMachineUnique = SCE::StateMachineBuilder()
                                  .withScriptEngine(SCE::ScriptEngineProvider::getScriptEngine())
                                  .withEventDispatcher(resources->eventDispatcher)
                                  .withEventRaiser(resources->eventRaiser)
                                  .build();
    auto smContext = std::make_unique<SCE::StateMachineContext>(std::move(stateMachineUnique));
    auto *stateMachine = smContext->get();

    // Relative references in the document resolve against its own file
    SCE::SessionRegistry::instance().registerSessionFilePath(stateMachine->getSessionId(), sourcePath);

    if (!stateMachine->loadSCXMLFromString(scxml)) {
        run.loaded = false;
        run.loadError = stateMachine->getLastLoadError();
        return {};
    }
    run.loaded = true;

    stateMachine->setEventRaiser(resources->eventRaiser);
    if (!stateMachine->start()) {
        return "Failed to start StateMachine";
    }

    const auto runStart = std::chrono::steady_clock::now();
    while (true) {
        const auto elapsed = std::chrono::steady_clock::now() - runStart;
        if (elapsed >= EXECUTOR_DEFAULT_TIMEOUT_MS) {
            break;
        }
        resources->eventRaiser->processQueuedEvents();
        if (!stateMachine->isRunning() && elapsed >= watchAtLeast) {
            break;
        }
        waitOnePollInterval();
    }

    run.halted = !stateMachine->isRunning();
    run.terminalState = stateMachine->terminalState();
    run.endState = run.terminalState.value_or(stateMachine->getCurrentState());
    run.eventsProcessed = stateMachine->getStatistics().totalEvents;
    return {};
}

TestReport W3CTestRunner::errorReport(const std::string &testId, const std::string &engineType,
                                      const std::string &reason) {
    TestReport report;
    report.timestamp = std::chrono::system_clock::now();
    report.testId = testId;
    report.engineType = engineType;
    report.executionContext.finalState = "error";
    report.executionContext.errorMessage = reason;
    report.executionContext.executionTime = std::chrono::milliseconds(0);
    // The reporters print the ID and the reason but not the engine, so the
    // reason names it.
    report.validationResult =
        ValidationResult(false, TestResult::ERROR, "the " + engineType + " run did not complete: " + reason);
    return report;
}

W3CTestRunner::W3CTestRunner(std::unique_ptr<ITestConverter> converter,
                             std::unique_ptr<ITestMetadataParser> metadataParser,
                             std::unique_ptr<ITestExecutor> executor, std::unique_ptr<ITestResultValidator> validator,
                             std::unique_ptr<ITestSuite> testSuite, std::unique_ptr<ITestReporter> reporter)
    : converter_(std::move(converter)), metadataParser_(std::move(metadataParser)), executor_(std::move(executor)),
      validator_(std::move(validator)), testSuite_(std::move(testSuite)), reporter_(std::move(reporter)) {}

std::vector<TestReport> W3CTestRunner::runAllTests() {
    // The test run (beginTestRun ... endTestRun) is the caller's: one CLI
    // invocation is one run for the reporter however many times it repeats
    // this, so starting one here would clear the reports of the iterations
    // before it.
    std::vector<TestReport> reports;
    auto testDirectories = testSuite_->discoverTests();

    SCE_LOG_INFO("W3C Test Execution: Starting {} discovered tests", testDirectories.size());

    // Memory profiling: Track test progress and heap size (WASM only)
    size_t testIndex = 0;

#ifdef __EMSCRIPTEN__
    // WASM memory investigation: Track initial heap size for cumulative delta calculation
    size_t heapInitial = emscripten_get_heap_size();
    SCE_LOG_DEBUG("WASM heap tracking: INITIAL heap = {} MB", heapInitial / (1024 * 1024));
#endif

    for (const auto &testDir : testDirectories) {
        SCE_LOG_DEBUG("W3C Test Execution: Running test {}", testDir);

#ifdef __EMSCRIPTEN__
        // WASM memory leak investigation: Log heap size BEFORE each test
        size_t heapBefore = emscripten_get_heap_size();
        SCE_LOG_DEBUG("WASM heap tracking: test {}/{} BEFORE - {} MB", testIndex, testDirectories.size(),
                      heapBefore / (1024 * 1024));
#endif

        // A test that throws is reported as an ERROR and counted, never left
        // out of the run (errorReport says why); each report names the
        // fixture by its full ID, a skip under ThreadSanitizer included.
        reports.push_back(runOnInterpreter(testDir));

#ifdef __EMSCRIPTEN__
        // WASM memory leak investigation: Log heap size AFTER test but BEFORE cleanup
        size_t heapAfterTest = emscripten_get_heap_size();
        size_t testDelta = heapAfterTest - heapBefore;
        SCE_LOG_DEBUG("WASM heap tracking: test {}/{} AFTER test - {} MB (delta: +{} MB)", testIndex,
                      testDirectories.size(), heapAfterTest / (1024 * 1024), testDelta / (1024 * 1024));
#endif

        // W3C SCXML: Cleanup shared state for test isolation (Zero Duplication)
        cleanupBetweenTests();

#ifdef __EMSCRIPTEN__
        // WASM memory leak investigation: Log heap size AFTER cleanup with cumulative tracking
        size_t heapAfterCleanup = emscripten_get_heap_size();
        size_t cleanupDelta = heapAfterTest - heapAfterCleanup;
        size_t cumulativeDelta = heapAfterCleanup - heapInitial;
        SCE_LOG_DEBUG(
            "WASM heap tracking: test {}/{} AFTER cleanup - {} MB (freed: {} MB, net: +{} MB, cumulative: +{} MB)",
            testIndex, testDirectories.size(), heapAfterCleanup / (1024 * 1024), cleanupDelta / (1024 * 1024),
            (heapAfterCleanup - heapBefore) / (1024 * 1024), cumulativeDelta / (1024 * 1024));
#endif

        SCE_LOG_DEBUG("W3C Test Execution: Test {} completed (Registry cleared → Engine reset)", testDir);

        // Memory profiling: Increment test counter
        testIndex++;
    }

    SCE_LOG_INFO("W3C Test Execution: Completed {} tests total", reports.size());

#ifdef __EMSCRIPTEN__
    // WASM memory investigation: Final heap size after all tests
    size_t heapFinal = emscripten_get_heap_size();
    size_t totalAccumulation = heapFinal - heapInitial;
    SCE_LOG_DEBUG("WASM heap tracking: FINAL heap = {} MB (total: +{} MB from {} tests)", heapFinal / (1024 * 1024),
                  totalAccumulation / (1024 * 1024), reports.size());
#endif

    return reports;
}

TestReport W3CTestRunner::runSingleTest(const std::string &testDirectory) {
    TestReport report;
    report.timestamp = std::chrono::system_clock::now();
    report.engineType = "interpreter";  // Interpreter engine execution
    report.testType = "interpreter";    // Interpreter runtime execution

    // Start log capture for failure diagnostics
    if (!failedLogDir_.empty()) {
        SCE::Logger::startCapture();
    }

    try {
        // Parse metadata
        std::string metadataPath = testSuite_->getMetadataPath(testDirectory);
        SCE_LOG_DEBUG("W3C Single Test: Parsing metadata from {}", metadataPath);
        report.metadata = metadataParser_->parseMetadata(metadataPath);

        // The fixture's full ID, variant suffix included ("path/403:a" is "403a")
        report.testId = fixtureIdOf(testDirectory);

        // A manual fixture names an observable its final state cannot carry,
        // so the validator cannot judge it: its verifier does, from a run
        // watched under a log capture of its own (runManualFixture).
        if (report.metadata.manual) {
            SCE::Logger::stopCapture();
            SCE::Logger::getCapturedLogs();  // Discard
            return runManualFixture(testDirectory, report);
        }

        const FixtureDocument document = loadFixtureDocument(testDirectory, report.testId);

        // Execute test
        SCE_LOG_DEBUG("W3C Single Test: Executing test {}", report.testId);
        report.executionContext = executor_->executeTest(document.scxml, report.metadata, document.sourcePath);

        // Validate result
        SCE_LOG_DEBUG("W3C Single Test: Validating result for test {}", report.testId);
        report.validationResult = validator_->validateResult(report.executionContext);

        // Use verified description if available, otherwise use metadata
        if (report.validationResult.finalResult == TestResult::PASS) {
            // Single Source of Truth: Always use metadata.txt description (not verification JSON notes)
            // This ensures Interpreter and AOT descriptions are always consistent
            std::string specDescription = "W3C SCXML " + report.metadata.specnum + ": " + report.metadata.description;

            report.validationResult = ValidationResult(true, TestResult::PASS, specDescription);
        }

        SCE_LOG_DEBUG("W3C Single Test: Test {} completed with result: {}", report.testId,
                      static_cast<int>(report.validationResult.finalResult));

        // Save captured logs on failure
        if (!failedLogDir_.empty()) {
            SCE::Logger::stopCapture();
            if (report.validationResult.finalResult != TestResult::PASS) {
                saveFailedTestLog(failedLogDir_, report.testId, report.engineType, report,
                                  SCE::Logger::getCapturedLogs());
            } else {
                SCE::Logger::getCapturedLogs();  // Discard
            }
        }

        return report;
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("W3C Single Test: Exception in test {}: {}", testDirectory, e.what());

        // Save captured logs on exception
        if (!failedLogDir_.empty()) {
            SCE::Logger::stopCapture();
            saveFailedTestLog(failedLogDir_, report.testId, "interpreter", report, SCE::Logger::getCapturedLogs());
        }

        throw;  // The caller reports it as an ERROR (reportingErrors)
    }
}

W3CTestRunner::FixtureDocument W3CTestRunner::loadFixtureDocument(const std::string &testDirectory,
                                                                  const std::string &testId) {
    FixtureDocument document;

    // Check if SCXML file exists directly (for tests like 513 with direct SCXML)
    std::string scxmlPath = testDirectory + "/test" + testId + ".scxml";

    std::ifstream scxmlFile(scxmlPath);
    if (scxmlFile.good()) {
        // Use existing SCXML file directly (skip TXML conversion)
        SCE_LOG_DEBUG("W3C Single Test: Using existing SCXML from {}", scxmlPath);
        document.scxml = std::string((std::istreambuf_iterator<char>(scxmlFile)), std::istreambuf_iterator<char>());
        document.sourcePath = scxmlPath;  // Use SCXML path as reference
    } else {
        // Read and convert TXML
        document.sourcePath = testSuite_->getTXMLPath(testDirectory);
        SCE_LOG_DEBUG("W3C Single Test: Reading TXML from {}", document.sourcePath);
        std::ifstream txmlFile(document.sourcePath);
        std::string txml((std::istreambuf_iterator<char>(txmlFile)), std::istreambuf_iterator<char>());

        SCE_LOG_DEBUG("W3C Single Test: Converting TXML to SCXML for test {}", testId);

        // Log original TXML before conversion
        SCE_LOG_DEBUG("W3C Test {}: Original TXML content:\n{}", testId, txml);

        document.scxml = converter_->convertTXMLToSCXML(txml);

        // Log converted SCXML after conversion
        SCE_LOG_DEBUG("W3C Test {}: Converted SCXML content:\n{}", testId, document.scxml);
    }

    // Convert all sub-TXML files in the test directory for invoke elements
    // Extract actual directory path (remove variant suffix if present)
    std::string actualTestDir = testDirectory;
    size_t colonPos2 = testDirectory.find(':');
    if (colonPos2 != std::string::npos) {
        actualTestDir = testDirectory.substr(0, colonPos2);
    }
    std::filesystem::path testDir(actualTestDir);
    for (const auto &entry : std::filesystem::directory_iterator(testDir)) {
        if (entry.is_regular_file() && entry.path().extension() == ".txml") {
            std::string filename = entry.path().filename().string();
            // Skip main test file
            if (filename == "test" + testId + ".txml") {
                continue;
            }

            // Convert sub-TXML file to SCXML (without W3C validation for sub-files)
            std::ifstream subTxmlFile(entry.path());
            std::string subTxml((std::istreambuf_iterator<char>(subTxmlFile)), std::istreambuf_iterator<char>());
            std::string subScxml =
                static_cast<SCE::W3C::TXMLConverter *>(converter_.get())->convertTXMLToSCXMLWithoutValidation(subTxml);

            // Write the converted SCXML only when it differs from what is
            // already there.
            //
            // This writes into `resources/<N>/`, which is the TRACKED
            // source tree, and the conversion is deterministic — the
            // result equals the committed file every time. Writing it
            // unconditionally therefore changed nothing but the mtime,
            // and that was not free: the generated `.d` depfiles list
            // these documents as inputs, so one test run left eight
            // generation rules permanently dirty. Every subsequent build
            // regenerated and relinked, and the relinked binary differed
            // (measured 2026-08-13: 40 bytes, in `.debug_info` and the
            // build-id derived from it) even though every object file was
            // byte-identical.
            //
            // The cost of that was a check nobody could run: the second
            // question `scripts/mutate` asks is "did restoring the source
            // restore the binaries", and for any ctest target reached
            // through this runner the answer was permanently no — so
            // mutation evidence there was unavailable rather than merely
            // unpinned.
            std::filesystem::path subScxmlPath = entry.path();
            subScxmlPath.replace_extension(".scxml");

            bool identical = false;
            {
                std::ifstream existing(subScxmlPath);
                if (existing) {
                    const std::string current((std::istreambuf_iterator<char>(existing)),
                                              std::istreambuf_iterator<char>());
                    identical = (current == subScxml);
                }
            }

            if (identical) {
                SCE_LOG_DEBUG("W3C Test {}: Sub-file {} already matches its conversion, leaving it alone", testId,
                              filename);
            } else {
                std::ofstream subScxmlFile(subScxmlPath);
                subScxmlFile << subScxml;
                SCE_LOG_DEBUG("W3C Test {}: Converted sub-file {} to {}", testId, filename,
                              subScxmlPath.filename().string());
            }
        }
    }

    return document;
}

TestReport W3CTestRunner::runManualFixture(const std::string &testDirectory, TestReport report) {
    const ManualVerifier *verifier = manualVerifierFor(report.testId);
    if (verifier == nullptr) {
        // Not run, and reported as a skip under its own ID, so a manual
        // fixture registered without a verifier shows in every tally instead
        // of passing (ManualVerification.h)
        SCE_LOG_WARN("W3C Test {}: manual fixture with no verifier in ManualVerification.cpp, skipped", report.testId);
        report.validationResult = ValidationResult::skip("Skipped: manual fixture with no verifier");
        return report;
    }
    report.testType = "manual";

    const FixtureDocument document = loadFixtureDocument(testDirectory, report.testId);
    const auto startTime = std::chrono::steady_clock::now();

    // The verifier reads the <log> output of the run, the invoked sessions'
    // included, so the run is watched under a capture of its own, which also
    // serves as the failed-test log. A run that cannot start, or that throws,
    // is the fixture's ERROR, not a verdict.
    ManualRun run;
    std::string startFailure;
    SCE::Logger::startCapture();
    try {
        startFailure = watchManualRun(document.scxml, document.sourcePath, verifier->watchAtLeast, run);
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("W3C Test {}: Exception during manual verification: {}", report.testId, e.what());
        startFailure = "Exception: " + std::string(e.what());
    }
    SCE::Logger::stopCapture();
    const std::string capturedLogs = SCE::Logger::getCapturedLogs();

    report.executionContext.executionTime =
        std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - startTime);

    if (!startFailure.empty()) {
        report.executionContext.finalState = "error";
        report.executionContext.errorMessage = startFailure;
        report.validationResult = ValidationResult(false, TestResult::ERROR, startFailure);
    } else {
        run.logCaptured = !capturedLogs.empty();
        run.logMessages = logMessagesIn(capturedLogs);
        report.executionContext.finalState = run.loaded ? run.endState : "rejected at load";
        report.validationResult = verifier->judge(run);
    }

    SCE_LOG_INFO("W3C Test {}: manual verification {}: {}", report.testId,
                 testResultToString(report.validationResult.finalResult), report.validationResult.reason);

    if (!failedLogDir_.empty() && report.validationResult.finalResult != TestResult::PASS) {
        saveFailedTestLog(failedLogDir_, report.testId, report.engineType, report, capturedLogs);
    }

    return report;
}

void W3CTestRunner::cleanupBetweenTests() {
    // Step 1: Clear EventRaiserRegistry (breaks shared_ptr cycles with HttpEventTarget)
    if (SCE::EventRaiserService::isInitialized()) {
        SCE::EventRaiserService::getInstance().clearAll();
    }

    // Step 2: Reset active script engine via ScriptEngineProvider (engine-agnostic)
    SCE::ScriptEngineProvider::getScriptEngine().reset();

    SCE_LOG_DEBUG("W3C Test Execution: Inter-test cleanup completed");
}

bool W3CTestRunner::requiresHttpServer(const std::string &testDirectory) const {
    // §scxml-C-2: the fixtures that need the BasicHTTP listener are the ones
    // the conformance registry runs under its `http` harness
    // (tests/w3c/conformance/fixtures.json, generated into W3CTestRegistry).
    // This read each fixture's metadata and TXML text for a sign of the
    // processor instead, a second answer that disagreed with the registry on
    // 577: a `<send>` with no target, which raises error.communication and
    // posts nothing.
    //
    // A directory names its fixture: `.../201`, or `.../403:a` for a variant.
    return W3CTestRegistry::needsHttpEndpoint(fixtureIdOf(testDirectory));
}

TestReport W3CTestRunner::runInterpreterFixture(const std::string &testDir) {
    if (!requiresHttpServer(testDir)) {
        return runSingleTest(testDir);
    }
#ifndef __EMSCRIPTEN__
    // The listener, in process: started for the fixture and stopped after it,
    // whether the run returns or throws. A listener that cannot start throws,
    // and the caller reports the fixture as an ERROR (errorReport). It binds
    // the port SCE_W3C_HTTP_PORT names, or one the kernel assigns, and the
    // machine is handed what it bound (basic_http_test_endpoint.h).
    W3CHttpTestServer httpServer(basicHttpInProcessPort(), basicHttpTestPath());
    httpServer.startOrThrow();
    SCE_LOG_INFO("W3C Test {}: HTTP server started successfully on {}", testDir, httpServer.accessUri());

    struct StopOnExit {
        W3CHttpTestServer &server;

        ~StopOnExit() {
            server.stop();
        }
    } stopOnExit{httpServer};

    return runSingleTestWithHttpServer(testDir, &httpServer);
#else
    // The listener, the host's: w3c/wasm_polyfill.js starts it before main()
    // at the same port and path, and it answers each POST with the event it
    // made, which the engine's HttpEventTarget raises.
    return runSingleTestWithHttpServer(testDir, nullptr);
#endif
}

std::optional<TestReport> W3CTestRunner::tsanSkipReport(const std::string &fixtureId, const std::string &engineType) {
    if (!SCE::Test::Utils::isThreadSanitizerBuild() || !W3CTestRegistry::needsHttpEndpoint(fixtureId)) {
        return std::nullopt;
    }

    SCE_LOG_WARN("W3C Test {} ({}): Skipping HTTP test under ThreadSanitizer (the listener's threads and "
                 "getaddrinfo are incompatible with TSAN)",
                 fixtureId, engineType);

    TestReport report;
    report.timestamp = std::chrono::system_clock::now();
    report.testId = fixtureId;
    report.engineType = engineType;
    report.validationResult = ValidationResult::skip("Skipped: HTTP test under ThreadSanitizer");

    return report;
}

TestReport W3CTestRunner::reported(TestReport report) {
    report.iteration = iteration_;
    reporter_->reportTestResult(report);
    return report;
}

TestReport W3CTestRunner::runOnInterpreter(const std::string &testDir) {
    const std::string fixtureId = fixtureIdOf(testDir);
    if (auto skipReport = tsanSkipReport(fixtureId, "interpreter")) {
        return reported(*skipReport);
    }
    return reported(
        reportingErrors(fixtureId, "interpreter", [this, &testDir] { return runInterpreterFixture(testDir); }));
}

std::vector<std::string> W3CTestRunner::discoverFixtureIds() {
    std::vector<std::string> fixtureIds;
    for (const auto &testDir : testSuite_->discoverTests()) {
        fixtureIds.push_back(fixtureIdOf(testDir));
    }
    return fixtureIds;
}

std::optional<TestReport> W3CTestRunner::runTest(const std::string &testId) {
    auto testDirectories = testSuite_->discoverTests();

    SCE_LOG_DEBUG("W3CTestRunner: Looking for exact test ID: {}", testId);
    SCE_LOG_DEBUG("W3CTestRunner: Total discovered test directories: {}", testDirectories.size());

    for (const auto &testDir : testDirectories) {
        // Exact match on the fixture's full ID: "../../resources/403:a" is
        // "403a", "../../resources/192" is "192"
        if (fixtureIdOf(testDir) == testId) {
            SCE_LOG_INFO("W3CTestRunner: Found exact match for test ID '{}': {}", testId, testDir);
            return runOnInterpreter(testDir);
        }
    }

    // No test has this ID: the W3C numbering is sparse.
    return std::nullopt;
}

std::vector<TestReport> W3CTestRunner::runAllMatchingTests(int testId, bool onInterpreter, bool onAot) {
    std::vector<TestReport> matchingReports;
    auto testDirectories = testSuite_->discoverTests();

    // Debug: Log discovered test directories for this test ID
    SCE_LOG_DEBUG("W3CTestRunner: Discovered test directories for ID {}: {}", testId, testDirectories.size());
    for (const auto &testDir : testDirectories) {
        SCE_LOG_DEBUG("W3CTestRunner:   - {}", testDir);
    }

    for (const auto &testDir : testDirectories) {
        // The fixture's full ID ("path/403:a" is "403a") and the test number
        // it begins with (403), which is what this call matches on
        const std::string fixtureId = fixtureIdOf(testDir);
        int fixtureNumber = 0;
        try {
            fixtureNumber = std::stoi(fixtureId);
        } catch (...) {
            continue;
        }

        if (fixtureNumber != testId) {
            continue;
        }

        // Each engine asked for leaves one report for the variant, under its
        // full ID, an ERROR when its run throws (errorReport): an Interpreter
        // throw neither drops the Interpreter report nor skips the AOT run.
        // Both report through the run's reporter (reported).
        if (onInterpreter) {
            matchingReports.push_back(runOnInterpreter(testDir));
        }

        if (onAot) {
            SCE_LOG_INFO("W3C Test {}: Running AOT engine test for variant", fixtureId);
            matchingReports.push_back(runAotTest(fixtureId));
            SCE_LOG_INFO("W3C Test {}: AOT engine test completed for variant", fixtureId);
        }

        // W3C SCXML: Cleanup shared state for test isolation (Zero Duplication)
        cleanupBetweenTests();
    }

    // Empty when no test has this ID: the W3C numbering is sparse, and the
    // caller tells that apart from a test that ran by the empty result, not
    // by an exception's text.
    return matchingReports;
}

TestReport W3CTestRunner::runSingleTestWithHttpServer(const std::string &testDirectory, W3CHttpTestServer *httpServer) {
    TestReport report;
    report.timestamp = std::chrono::system_clock::now();
    report.engineType = "interpreter";  // Interpreter engine execution
    report.testType = "interpreter";    // Interpreter runtime execution

    try {
        // Parse metadata
        std::string metadataPath = testSuite_->getMetadataPath(testDirectory);
        SCE_LOG_DEBUG("W3C Single Test (HTTP): Parsing metadata from {}", metadataPath);
        report.metadata = metadataParser_->parseMetadata(metadataPath);

        // The fixture's full ID, variant suffix included ("path/403:a" is "403a")
        report.testId = fixtureIdOf(testDirectory);

        // A manual fixture is judged by its verifier, whose run has no
        // BasicHTTP listener (runManualFixture); one the registry runs under
        // its `http` harness is not run, and is reported as a skip by name.
        if (validator_->shouldSkipTest(report.metadata)) {
            SCE_LOG_WARN("W3C Test {}: manual fixture under the BasicHTTP harness, which no verifier runs, skipped",
                         report.testId);
            report.validationResult = ValidationResult::skip("Skipped: manual fixture under the BasicHTTP harness");
            return report;
        }

        // Check if SCXML file exists directly (for tests like 513 with direct SCXML)
        std::string scxmlPath = testDirectory + "/test" + report.testId + ".scxml";
        std::string scxml;

        std::ifstream scxmlFile(scxmlPath);
        if (scxmlFile.good()) {
            // Use existing SCXML file directly
            SCE_LOG_DEBUG("W3C Single Test (HTTP): Using existing SCXML from {}", scxmlPath);
            scxml = std::string((std::istreambuf_iterator<char>(scxmlFile)), std::istreambuf_iterator<char>());
        } else {
            // Read and convert TXML
            std::string txmlPath = testSuite_->getTXMLPath(testDirectory);
            SCE_LOG_DEBUG("W3C Single Test (HTTP): Reading TXML from {}", txmlPath);
            std::ifstream txmlFile(txmlPath);
            std::string txml((std::istreambuf_iterator<char>(txmlFile)), std::istreambuf_iterator<char>());

            SCE_LOG_DEBUG("W3C Single Test (HTTP): Converting TXML to SCXML for test {}", report.testId);
            scxml = converter_->convertTXMLToSCXML(txml);
        }

        // Create custom executor with HTTP server integration
        auto startTime = std::chrono::steady_clock::now();

        TestExecutionContext context;
        context.scxmlContent = scxml;
        context.metadata = report.metadata;
        context.expectedTarget = "pass";

        // Create shared resources using RAII factory pattern
        auto resources = TestComponentFactory::createResources();

        try {
            SCE_LOG_DEBUG("StateMachineTestExecutor (HTTP): Starting test execution for test {}", report.metadata.id);

#ifndef __EMSCRIPTEN__
            // §scxml-C-2-3: the location the machine publishes is the
            // in-process listener's own bound address; with no listener of
            // its own the run posts to the agreed out-of-process one.
            const std::string accessUri = (httpServer != nullptr) ? httpServer->accessUri() : basicHttpTestAccessUri();

            // Set up HTTP server eventCallback to use the EventRaiser
            // W3C SCXML compliance: HTTP events must use EXTERNAL priority (test 510)
            if (httpServer != nullptr) {
                httpServer->setEventCallback(
                    [eventRaiser = resources->eventRaiser](const std::string &eventName, const std::string &eventData) {
                        SCE_LOG_INFO(
                            "W3CHttpTestServer: Receiving HTTP event '{}' - raising to SCXML with EXTERNAL priority",
                            eventName);
                        // W3C SCXML 5.10: HTTP events must use external queue (test 510 compliance)
                        eventRaiser->raiseExternalEvent(eventName, eventData);
                    });
            }
#else
            // The host's listener answers each POST with the event it made,
            // and the engine's HttpEventTarget raises it. It is out of
            // process, at the agreed SCE_W3C_HTTP_PORT.
            (void)httpServer;
            const std::string accessUri = basicHttpTestAccessUri();
#endif

            // Build StateMachine with resource injection, then wrap in RAII context
            // §scxml-C-2-3: the harness owns the inbound BasicHTTP listener, so
            // it is the harness that declares the access URI the engine
            // publishes. The converted documents address their sends through
            // that published entry.
            auto stateMachineUnique = SCE::StateMachineBuilder()
                                          .withScriptEngine(SCE::ScriptEngineProvider::getScriptEngine())
                                          .withEventDispatcher(resources->eventDispatcher)
                                          .withEventRaiser(resources->eventRaiser)
                                          .withBasicHttpAccessUri(accessUri)
                                          .build();

            // Wrap in StateMachineContext for RAII cleanup
            auto smContext = std::make_unique<SCE::StateMachineContext>(std::move(stateMachineUnique));
            auto *stateMachine = smContext->get();

            // Load SCXML content
            if (!stateMachine->loadSCXMLFromString(scxml)) {
                SCE_LOG_ERROR("StateMachineTestExecutor (HTTP): Failed to load SCXML content");
                context.finalState = "error";
                context.errorMessage = "Failed to load SCXML content";
                report.executionContext = context;
                return report;
            }

            // Ensure EventRaiser callback is properly set after SCXML loading
            stateMachine->setEventRaiser(resources->eventRaiser);

            // Start the state machine
            if (!stateMachine->start()) {
                SCE_LOG_ERROR("StateMachineTestExecutor (HTTP): Failed to start StateMachine");
                context.finalState = "error";
                context.errorMessage = "Failed to start StateMachine";
                report.executionContext = context;
                return report;
            }

            // Wait for StateMachine to reach final state or timeout
            auto waitStart = std::chrono::steady_clock::now();
            std::string currentState;
            const std::chrono::milliseconds timeout = EXECUTOR_DEFAULT_TIMEOUT_MS;

            // W3C SCXML C.2: Use while loop with ASYNCIFY for both Native and WASM
            // ASYNCIFY allows emscripten_sleep() to yield to event loop for EM_ASYNC_JS HTTP callbacks
            while (std::chrono::steady_clock::now() - waitStart < timeout) {
                currentState = stateMachine->terminalState().value_or(stateMachine->getCurrentState());

                // Check if we reached a final state (pass or fail)
                if (currentState == "pass" || currentState == "fail") {
                    SCE_LOG_DEBUG("StateMachineTestExecutor: Reached final state: {}", currentState);
                    break;
                }

                // Poll at 20 FPS (50ms) - emscripten_sleep() yields to event loop via ASYNCIFY
#ifdef __EMSCRIPTEN__
                emscripten_sleep(POLL_INTERVAL_MS.count());
#else
                std::this_thread::sleep_for(POLL_INTERVAL_MS);
#endif
            }

            // Get final state after loop exit
            context.finalState = currentState;
            SCE_LOG_DEBUG("StateMachineTestExecutor (HTTP): Test completed with final state: {}", context.finalState);

            auto endTime = std::chrono::steady_clock::now();
            context.executionTime = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

            // RAII cleanup: StateMachineContext destructor handles all cleanup automatically
            SCE_LOG_DEBUG("StateMachineTestExecutor (HTTP): Automatic cleanup will occur on scope exit");

            report.executionContext = context;

        } catch (const std::exception &e) {
            SCE_LOG_ERROR("StateMachineTestExecutor (HTTP): Exception during test execution: {}", e.what());
            context.finalState = "error";
            context.errorMessage = "Exception: " + std::string(e.what());

            auto endTime = std::chrono::steady_clock::now();
            context.executionTime = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

            // RAII cleanup: StateMachineContext destructor handles cleanup on exception (HTTP version)
            SCE_LOG_DEBUG(
                "StateMachineTestExecutor (HTTP): Automatic cleanup will occur on scope exit after exception");

            report.executionContext = context;
        }

        // Validate result
        SCE_LOG_DEBUG("W3C Single Test (HTTP): Validating result for test {}", report.testId);
        report.validationResult = validator_->validateResult(report.executionContext);

        // Use verified description if available, otherwise use metadata
        if (report.validationResult.finalResult == TestResult::PASS) {
            // Single Source of Truth: Always use metadata.txt description (not verification JSON notes)
            // This ensures Interpreter and AOT descriptions are always consistent
            std::string specDescription = "W3C SCXML " + report.metadata.specnum + ": " + report.metadata.description;

            report.validationResult = ValidationResult(true, TestResult::PASS, specDescription);
        }

        SCE_LOG_DEBUG("W3C Single Test (HTTP): Test {} completed with result: {}", report.testId,
                      static_cast<int>(report.validationResult.finalResult));

        return report;
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("W3C Single Test (HTTP): Exception in test {}: {}", testDirectory, e.what());
        throw;  // The caller reports it as an ERROR (reportingErrors)
    }
}

TestReport W3CTestRunner::runAotTest(const std::string &testId) {
    // The skip under ThreadSanitizer is the Interpreter's decision, on the
    // same full ID: one rule for both engines (tsanSkipReport).
    if (auto skipReport = tsanSkipReport(testId, "aot")) {
        return reported(*skipReport);
    }
    return reported(reportingErrors(testId, "aot", [this, &testId] { return runAotTestOrThrow(testId); }));
}

TestReport W3CTestRunner::runAotTestOrThrow(const std::string &testId) {
    // Check W3CTestRegistry to determine if test should use Interpreter fallback
    if (W3CTestRegistry::isInterpreterTest(testId)) {
        SCE_LOG_INFO("AOT Test {}: Using Interpreter fallback (Interpreter-only per W3CTestRegistry)", testId);

        // Run with Interpreter engine as fallback
        auto testDirectories = testSuite_->discoverTests();
        for (const auto &testDir : testDirectories) {
            if (fixtureIdOf(testDir) == testId) {
                TestReport report = runSingleTest(testDir);
                // Mark as AOT engine but using Interpreter fallback
                report.engineType = "aot";
                report.testType = "interpreter_fallback";

                return report;
            }
        }

        // Test not found in test suite
        TestReport report;
        report.timestamp = std::chrono::system_clock::now();
        report.testId = testId;
        report.engineType = "aot";
        report.testType = "interpreter_fallback";
        report.validationResult = ValidationResult(false, TestResult::ERROR, "Test not found in test suite");
        report.executionContext.finalState = "error";
        report.executionContext.executionTime = std::chrono::milliseconds(0);
        return report;
    }

    // Try registry-based test first (new modular system)
    auto registryTest = SCE::W3C::AotTests::AotTestRegistry::instance().createTest(testId);
    if (registryTest) {
        TestReport report;
        report.timestamp = std::chrono::system_clock::now();
        report.testId = testId;
        report.engineType = "aot";
        report.testType =
            registryTest->getTestType();  // pure_static or static_hybrid based on Policy::NEEDS_SCRIPT_ENGINE

        // Start log capture for failure diagnostics
        if (!failedLogDir_.empty()) {
            SCE::Logger::startCapture();
        }

        auto startTime = std::chrono::steady_clock::now();

        try {
            bool testPassed = registryTest->run();
            std::string testDescription = registryTest->getDescription();

            auto endTime = std::chrono::steady_clock::now();
            auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

            if (testPassed) {
                report.validationResult = ValidationResult(true, TestResult::PASS, testDescription);
                report.executionContext.finalState = "pass";
            } else {
                report.validationResult = ValidationResult(false, TestResult::FAIL, testDescription);
                report.executionContext.finalState = "fail";
            }

            report.executionContext.executionTime = duration;

            SCE_LOG_INFO("AOT Test {} ({}): {} in {}ms", testId, testDescription, testPassed ? "PASS" : "FAIL",
                         duration.count());

            // Save captured logs on failure
            if (!failedLogDir_.empty()) {
                SCE::Logger::stopCapture();
                if (report.validationResult.finalResult != TestResult::PASS) {
                    saveFailedTestLog(failedLogDir_, report.testId, report.engineType, report,
                                      SCE::Logger::getCapturedLogs());
                } else {
                    SCE::Logger::getCapturedLogs();  // Discard
                }
            }

            return report;

        } catch (const std::exception &e) {
            auto endTime = std::chrono::steady_clock::now();
            auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

            SCE_LOG_ERROR("AOT Test {} failed with exception: {}", testId, e.what());
            const std::string testType = report.testType;
            report = errorReport(report.testId, report.engineType, e.what());
            report.testType = testType;
            report.executionContext.executionTime = duration;

            // Save captured logs on exception
            if (!failedLogDir_.empty()) {
                SCE::Logger::stopCapture();
                saveFailedTestLog(failedLogDir_, report.testId, report.engineType, report,
                                  SCE::Logger::getCapturedLogs());
            }

            return report;
        }
    }

    // Fallback: Test not registered in AOT system
    // This should not happen if W3CTestRegistry is properly maintained
    TestReport report;
    report.timestamp = std::chrono::system_clock::now();
    report.testId = testId;
    report.engineType = "aot";
    report.testType = "not_registered";

    SCE_LOG_WARN("W3C AOT Test: Test {} not registered in AotTestRegistry - check CMakeLists.txt", testId);
    report.validationResult = ValidationResult(false, TestResult::FAIL, "Test not registered in AOT system");
    report.executionContext.finalState = "fail";
    report.executionContext.executionTime = std::chrono::milliseconds(0);

    return report;
}

}  // namespace SCE::W3C
