// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A test suite generated in C++ (docs/SCE_TEST_PROFILE.md §11.6 to §11.8,
// §12.2, §12.6 slice 5), run end to end by the profile's runner,
// SCE::Test::runSuite. The suite, its test cases and their configurations are
// the tables sce-codegen writes from the fixtures under
// sce-build/tests/fixtures/test_profile/runtime/, and the components are the
// machines it writes from the components there. Only the system adapter, which
// plays the system under test, is written here.

#include <gtest/gtest.h>

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <map>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include "answers_the_job.h"
#include "asking_configuration.h"
#include "job_reply.h"
#include "job_request.h"
#include "keeping_configuration.h"
#include "reply_ok.h"
#include "runtime_suite.h"
#include "saved/SavedValues.h"
#include "serving_configuration.h"
#include "test/Runner.h"
#include "test/Suite.h"
#include "test/TestCaseRun.h"
#include "test/Trace.h"

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace {

using SCE::Test::AdapterFactory;
using SCE::Test::CaseResult;
using SCE::Test::Duration;
using SCE::Test::Instant;
using SCE::Test::LogKind;
using SCE::Test::Message;
using SCE::Test::SystemAdapter;
using SCE::Test::Trace;
using SCE::Test::Verdict;

using Request = SCE::Generated::JobRequest::JobRequestPayload;
using Reply = SCE::Generated::JobReply::JobReplyPayload;

constexpr Duration MILLISECOND = 1000;
constexpr Duration SECOND = 1000 * MILLISECOND;

/// How long the system under test takes to answer a request.
constexpr Duration ANSWER_DELAY = 500;

/// The system under test on the system port `sut`: it answers each request,
/// ANSWER_DELAY after it, with a reply of `code` for the job the request
/// names, and records each job it was asked for.
class Responder : public SystemAdapter {
public:
    Responder(std::uint32_t code, std::vector<std::uint32_t> &requested) : code_(code), requested_(requested) {}

    void send(const std::string &systemPort, const Message &message, Instant now) override {
        const auto *request = message.value.as<Request>();
        ASSERT_EQ(systemPort, "sut");
        ASSERT_NE(request, nullptr) << "a request travels as its record's value";
        requested_.push_back(request->job);
        Reply reply{};
        reply.job = request->job;
        reply.code = code_;
        // The reply's canonical form in the test log is the one the unit of a
        // template over its record writes.
        pending_.emplace_back(now + ANSWER_DELAY, Message{SCE::Generated::JobReply::EVENT_NAME,
                                                          SCE::Test::messageValue(reply, SCE::Generated::ReplyOk::TYPE,
                                                                                  SCE::Generated::ReplyOk::canonical)});
    }

    std::optional<Instant> nextInput() const override {
        if (pending_.empty()) {
            return std::nullopt;
        }
        return pending_.front().first;
    }

    std::vector<std::pair<std::string, Message>> takeInputsDue(Instant now) override {
        std::vector<std::pair<std::string, Message>> due;
        while (!pending_.empty() && pending_.front().first <= now) {
            due.emplace_back("sut", std::move(pending_.front().second));
            pending_.erase(pending_.begin());
        }
        return due;
    }

private:
    std::uint32_t code_;
    std::vector<std::uint32_t> &requested_;
    std::vector<std::pair<Instant, Message>> pending_;
};

/// A fresh Responder for each test case, answering with `code`.
AdapterFactory responders(std::uint32_t code, std::vector<std::uint32_t> &requested) {
    return [code, &requested](const std::string &) -> std::unique_ptr<SystemAdapter> {
        return std::make_unique<Responder>(code, requested);
    };
}

std::vector<std::string> names(const std::vector<CaseResult> &results) {
    std::vector<std::string> named;
    for (const auto &result : results) {
        named.push_back(result.testCase);
    }
    return named;
}

bool logged(const CaseResult &result, LogKind kind) {
    for (const auto &record : result.result.log) {
        if (record.kind == kind) {
            return true;
        }
    }
    return false;
}

/// P11, P12, P13, P14: in a world with every feature the suite runs every test
/// case in its order, and each ends with the verdict it expects: a job answered
/// in time passes, a PTC served passes, a removed PTC's inconclusive outweighs
/// its MTC's pass, and a guard shorter than the answer ends in error.
TEST(ATestSuite, RunsEveryTestCaseOfAWorldWithTheVerdictItExpects) {
    std::vector<std::uint32_t> requested;
    const auto results = SCE::Test::runSuite(SCE::Generated::RuntimeSuite::suite(), {"ptcs", "removal", "guard"},
                                             responders(0, requested));
    ASSERT_EQ(names(results),
              (std::vector<std::string>{"answers_the_job", "serves_a_ptc", "keeps_a_ptc", "answers_before_the_guard"}));
    EXPECT_EQ(results[0].result.verdict, Verdict::Pass);
    EXPECT_EQ(results[0].result.endedAt, ANSWER_DELAY) << "the reply is taken in the round it arrives in";
    EXPECT_EQ(results[1].result.verdict, Verdict::Pass);
    EXPECT_EQ(results[1].result.endedAt, 0) << "a branch fired in every round, so time never moved";
    EXPECT_EQ(results[2].result.verdict, Verdict::Inconclusive);
    EXPECT_EQ(results[2].result.endedAt, 15 * MILLISECOND);
    EXPECT_EQ(results[3].result.verdict, Verdict::Error);
    EXPECT_EQ(results[3].result.endedAt, 100) << "the guard, 0.1ms, ends the case before the answer";
    EXPECT_TRUE(logged(results[3], LogKind::Error));
    EXPECT_EQ(requested, (std::vector<std::uint32_t>{7, 7}))
        << "each asking case asks for the job the suite's module parameter gives";
}

/// P2, P7: a system under test that answers with a failure code fails the
/// test case when its timer runs out. A world with no feature runs only the
/// test case no selection guards.
TEST(ATestSuite, FailsAJobTheSystemUnderTestAnswersWithAFailure) {
    std::vector<std::uint32_t> requested;
    const auto results = SCE::Test::runSuite(SCE::Generated::RuntimeSuite::suite(), {}, responders(3, requested));
    ASSERT_EQ(names(results), (std::vector<std::string>{"answers_the_job"}));
    EXPECT_EQ(results[0].result.verdict, Verdict::Fail);
    EXPECT_EQ(results[0].result.endedAt, SECOND) << "time advances to the timer's instant";
    EXPECT_TRUE(logged(results[0], LogKind::Timeout));
}

/// The test log every runtime writes of `answers_the_job` in a world with no
/// feature, answered with success: one file the C++ and the Rust runs are both
/// held to (`a_generated_test_case_writes_its_test_log` in
/// backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs), so the
/// agreement of the backends is one set of bytes. The file ends in a newline,
/// which no canonical text holds, so it is read without it.
std::string sharedLog(const std::string &testCase) {
    const std::string path = std::string(SCE_PROJECT_ROOT) +
                             "/sce-build/tests/fixtures/test_profile/runtime/expected_logs/" + testCase + ".json";
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "the shared test log is not readable: " << path;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    std::string text = buffer.str();
    const bool endsInNewline = !text.empty() && text.back() == '\n';
    EXPECT_TRUE(endsInNewline) << "the shared log ends in one newline: " << path;
    if (endsInNewline) {
        text.pop_back();
    }
    return text;
}

/// docs/SCE_TEST_PROFILE.md §12.5: the test log of a generated test case, in
/// its one canonical text, byte for byte the one the Rust runtime writes of
/// the same run. The request the component sends carries the value and the
/// canonical writer its template's unit gives, the job of the suite's module
/// parameter; the reply arrives at 500 and is taken in that round, which stops
/// the timer, passes and ends the MTC.
TEST(ATestSuite, AGeneratedTestCaseWritesItsTestLog) {
    std::vector<std::uint32_t> requested;
    const auto results = SCE::Test::runSuite(SCE::Generated::RuntimeSuite::suite(), {}, responders(0, requested));
    ASSERT_EQ(names(results), (std::vector<std::string>{"answers_the_job"}));
    EXPECT_EQ(results[0].canonicalLog("runtime_suite"), sharedLog("answers_the_job"));
}

/// The directory of the suite's committed traces (docs/SCE_TTCN3_LANE.md
/// §10.2), `<test case>.json` each.
std::string tracesDir() {
    return std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/test_profile/runtime/traces";
}

/// docs/SCE_TTCN3_LANE.md §10.5: the generated suite replayed from the
/// committed trace of `answers_the_job` writes the test log every runtime
/// writes of it, byte for byte: the configuration's header reads the reply
/// from its canonical form and writes it back the same, and the clock
/// advances to the trace's instant as it did to the Responder's. The twin of
/// the Rust `a_recorded_trace_replays_to_the_test_log_every_runtime_writes`.
TEST(ATestSuite, ARecordedTraceReplaysToTheTestLogEveryRuntimeWrites) {
    const auto results =
        SCE::Test::runSuiteWithTraces(SCE::Generated::RuntimeSuite::suite(), {}, [](const std::string &testCase) {
            return SCE::Test::Runner::readTrace(tracesDir(), testCase);
        });
    ASSERT_EQ(names(results), (std::vector<std::string>{"answers_the_job"}));
    EXPECT_EQ(results[0].result.verdict, Verdict::Pass);
    EXPECT_EQ(results[0].canonicalLog("runtime_suite"), sharedLog("answers_the_job"));
}

/// §10.2, P13: a trace of another test case, or one the configuration does not
/// take, ends the test case in error before any component is created, naming
/// why; the test case reads its trace through its configuration's `input`.
TEST(ATestSuite, EndsInErrorWhenItsTraceDoesNotFit) {
    EXPECT_TRUE(SCE::Generated::AnswersTheJob::testCase().inputs == &SCE::Generated::AskingConfiguration::input);
    const std::string reply = "{\"inputs\":[{\"at\":\"500\",\"port\":\"oracle\",\"type\":\"job_reply\","
                              "\"value\":{\"code\":\"0\",\"job\":\"7\"}}],\"schema\":\"sce-test-trace.v1\","
                              "\"suite\":\"runtime_suite\",\"test_case\":\"answers_the_job\"}";
    const auto unmapped = SCE::Test::runSuiteWithTraces(SCE::Generated::RuntimeSuite::suite(), {},
                                                        [&reply](const std::string &) { return Trace::parse(reply); });
    ASSERT_EQ(unmapped.size(), 1u);
    EXPECT_EQ(unmapped[0].result.verdict, Verdict::Error);
    ASSERT_FALSE(unmapped[0].result.log.empty());
    EXPECT_EQ(unmapped[0].result.log[0].detail,
              "the trace is refused: input 0, at 500: 'oracle' is not a system port of the test case's configuration");
    const auto other =
        SCE::Test::runSuiteWithTraces(SCE::Generated::RuntimeSuite::suite(), {}, [](const std::string &) {
            Trace trace;
            trace.suite = "runtime_suite";
            trace.testCase = "serves_a_ptc";
            return trace;
        });
    ASSERT_EQ(other.size(), 1u);
    EXPECT_EQ(other[0].result.verdict, Verdict::Error);
    EXPECT_EQ(other[0].result.log[0].detail,
              "the trace is of the test case 'serves_a_ptc' of the suite 'runtime_suite'");
}

/// A directory for the runner's logs, made empty, which @p name keeps apart
/// from every other test's.
std::filesystem::path logDir(const std::string &name) {
    const std::filesystem::path dir = std::filesystem::temp_directory_path() / ("sce-cpp-runtime-suite-runner-" + name);
    std::filesystem::remove_all(dir);
    return dir;
}

/// The text of the file at @p path.
std::string fileText(const std::filesystem::path &path) {
    std::ifstream in(path, std::ios::binary);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path.string();
    std::ostringstream buffer;
    buffer << in.rdbuf();
    return buffer.str();
}

/// docs/SCE_TTCN3_LANE.md §4: the suite's runner, given the directories of the
/// traces and of the logs, runs each test case its world selects against its
/// trace, writes `<case>.log.json`, the canonical log with nothing after it,
/// and prints each case's verdict; a module parameter's value on its command
/// line replaces its default, and one the suite does not declare is refused
/// with exit status 2 before any case runs.
TEST(ATestSuite, TheSuitesRunnerWritesTheLogOfEachTrace) {
    const auto runner = [](const std::filesystem::path &logs, std::vector<std::string> more, std::string &out,
                           std::string &err) {
        std::vector<std::string> args{"--trace-dir", tracesDir(), "--log-dir", logs.string()};
        args.insert(args.end(), more.begin(), more.end());
        std::ostringstream printed;
        std::ostringstream refused;
        const int status =
            SCE::Test::Runner::main(SCE::Generated::RuntimeSuite::suite(),
                                    &SCE::Generated::RuntimeSuite::moduleParameter, args, printed, refused);
        out = printed.str();
        err = refused.str();
        return status;
    };
    std::string out;
    std::string err;
    const std::filesystem::path logs = logDir("default");
    ASSERT_EQ(runner(logs, {}, out, err), 0) << err;
    EXPECT_EQ(out, "answers_the_job pass\n");
    EXPECT_EQ(fileText(logs / "answers_the_job.log.json"), sharedLog("answers_the_job"));
    std::filesystem::remove_all(logs);

    const std::filesystem::path overridden = logDir("overridden");
    ASSERT_EQ(runner(overridden, {"--world", "", "--module-parameter", "job=9"}, out, err), 0) << err;
    const std::string log = fileText(overridden / "answers_the_job.log.json");
    EXPECT_NE(log.find("\"kind\":\"message-sent\",\"port\":\"sut\",\"type\":\"job_request\",\"value\":{\"job\":\"9\"}"),
              std::string::npos)
        << "the request names the job of the value given: " << log;
    std::filesystem::remove_all(overridden);

    const std::filesystem::path undeclared = logDir("undeclared");
    EXPECT_EQ(runner(undeclared, {"--module-parameter", "limit=1"}, out, err), 2);
    EXPECT_NE(err.find("the suite declares no module parameter 'limit'"), std::string::npos) << err;
    EXPECT_FALSE(std::filesystem::exists(undeclared)) << "no case ran, so no log was written";
}

/// §11.8: a world runs the test cases whose selection it satisfies, in the
/// suite's order: `ptcs` alone selects the served PTC and not the removed one,
/// and `guard` selects the guarded case.
TEST(ATestSuite, RunsTheTestCasesAWorldSelects) {
    std::vector<std::uint32_t> requested;
    EXPECT_EQ(names(SCE::Test::runSuite(SCE::Generated::RuntimeSuite::suite(), {"ptcs"}, responders(0, requested))),
              (std::vector<std::string>{"answers_the_job", "serves_a_ptc"}));
    EXPECT_EQ(names(SCE::Test::runSuite(SCE::Generated::RuntimeSuite::suite(), {"guard"}, responders(0, requested))),
              (std::vector<std::string>{"answers_the_job", "answers_before_the_guard"}));
}

/// -1 §8.2.1, §11.5: a value the caller gives the module parameter replaces the
/// suite's default, and the component asks for the job it names.
TEST(ATestSuite, AsksForTheJobAnOverrideOfItsModuleParameterNames) {
    std::vector<std::uint32_t> requested;
    SCE::Test::ModuleParameters overrides;
    overrides.set("job", SCE::Saved::savedWriteUint32(9));
    const auto results =
        SCE::Test::runSuite(SCE::Generated::RuntimeSuite::suite(), {}, responders(0, requested), overrides);
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(results[0].result.verdict, Verdict::Pass);
    EXPECT_EQ(requested, (std::vector<std::uint32_t>{9}));
}

/// P13: a value of the module parameter that is not of the type the component
/// declares ends the test case in error before the component starts.
TEST(ATestSuite, EndsInErrorWhenAModuleParameterIsNotOfItsType) {
    std::vector<std::uint32_t> requested;
    SCE::Test::ModuleParameters overrides;
    overrides.set("job", SCE::Saved::savedWriteString("seven"));
    const auto results =
        SCE::Test::runSuite(SCE::Generated::RuntimeSuite::suite(), {}, responders(0, requested), overrides);
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(results[0].result.verdict, Verdict::Error);
    EXPECT_TRUE(requested.empty()) << "the component did not start, so it sent nothing";
}

/// The session id `machine` runs as, when it is a component of the type `Component`.
template <typename Component> std::optional<std::string> sessionOf(const SCE::Test::ComponentMachine &machine) {
    if (const auto *typed = dynamic_cast<const Component *>(&machine)) {
        return typed->sessionId();
    }
    return std::nullopt;
}

/// A component the configuration makes, run as it is, which records the session
/// id its machine started under by the name of the component.
class Recorded : public SCE::Test::ComponentMachine {
public:
    Recorded(std::unique_ptr<SCE::Test::ComponentMachine> inner, std::map<std::string, std::string> &sessions)
        : inner_(std::move(inner)), sessions_(sessions) {}

    void start(SCE::Test::ComponentContext &context) override {
        inner_->start(context);
        std::optional<std::string> session = sessionOf<SCE::Generated::coordinator::coordinatorComponent>(*inner_);
        if (!session) {
            session = sessionOf<SCE::Generated::answerer::answererComponent>(*inner_);
        }
        if (!session) {
            session = sessionOf<SCE::Generated::ptc_keeper::ptc_keeperComponent>(*inner_);
        }
        if (!session) {
            session = sessionOf<SCE::Generated::idler::idlerComponent>(*inner_);
        }
        sessions_[context.name()] = session.value_or("(a component of no type this test reads)");
    }

    void deliverSnapshot() override {
        inner_->deliverSnapshot();
    }

    bool terminated() const override {
        return inner_->terminated();
    }

private:
    std::unique_ptr<SCE::Test::ComponentMachine> inner_;
    std::map<std::string, std::string> &sessions_;
};

/// The session each component of a run of `configuration` started under, by the
/// component's name, its machines made by `machine`.
std::map<std::string, std::string>
sessionsOfARun(SCE::Test::Configuration configuration,
               std::unique_ptr<SCE::Test::ComponentMachine> (*machine)(const std::string &)) {
    std::map<std::string, std::string> sessions;
    std::vector<std::uint32_t> requested;
    Responder responder(0, requested);
    SCE::Test::TestCaseRun run(
        std::move(configuration),
        [machine, &sessions](const std::string &componentType) -> std::unique_ptr<SCE::Test::ComponentMachine> {
            auto made = machine(componentType);
            if (!made) {
                return nullptr;
            }
            return std::make_unique<Recorded>(std::move(made), sessions);
        },
        responder, std::nullopt);
    (void)run.run();
    return sessions;
}

/// docs/adr/0045: each component's session is named by the component, as on
/// every other backend: the MTC's is `mtc` and a PTC's the name its
/// configuration gives it, not an id the machine drew. The twin of the C11
/// suite test's `each_component_runs_under_its_configuration_name`.
TEST(ATestSuite, EachComponentRunsUnderItsConfigurationName) {
    EXPECT_EQ(sessionsOfARun(SCE::Generated::ServingConfiguration::configuration(),
                             &SCE::Generated::ServingConfiguration::machine),
              (std::map<std::string, std::string>{{"answerer", "answerer"}, {"mtc", "mtc"}}));
    EXPECT_EQ(sessionsOfARun(SCE::Generated::KeepingConfiguration::configuration(),
                             &SCE::Generated::KeepingConfiguration::machine),
              (std::map<std::string, std::string>{{"idler", "idler"}, {"mtc", "mtc"}}));
}

/// §11.6: the configuration's tables name the module parameters its
/// components read, which a suite that runs it must give.
TEST(ATestSuite, ReadsTheModuleParametersItsConfigurationNames) {
    EXPECT_EQ(SCE::Generated::AskingConfiguration::moduleParameters(), (std::vector<std::string>{"job"}));
    EXPECT_EQ(SCE::Generated::AskingConfiguration::configuration().mtcType, "param_asker");
    EXPECT_EQ(SCE::Generated::AskingConfiguration::machine("nobody"), nullptr);
}

}  // namespace
