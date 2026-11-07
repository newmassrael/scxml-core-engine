// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Test components generated in C++ (docs/SCE_TEST_PROFILE.md §12.3, §12.6
// slice 4), run end to end by the profile's runtime, SCE::Test::TestCaseRun.
// The machines are the statecharts sce-codegen writes from the fixtures under
// sce-build/tests/fixtures/test_profile/runtime/, driven only through the doors
// of §12.3; the configuration, the factory of machines and the system adapter
// are written here, since the tables and the runner are the next slice's.

#include <gtest/gtest.h>

#include <any>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "answerer_sm.h"
#include "asker_sm.h"
#include "coordinator_sm.h"
#include "idler_sm.h"
#include "job_reply.h"
#include "job_request.h"
#include "keeper_sm.h"
#include "test/TestCaseRun.h"

namespace {

using SCE::Test::ComponentMachine;
using SCE::Test::Configuration;
using SCE::Test::Duration;
using SCE::Test::Instant;
using SCE::Test::LogKind;
using SCE::Test::Message;
using SCE::Test::SystemAdapter;
using SCE::Test::TestCaseResult;
using SCE::Test::TestCaseRun;
using SCE::Test::Verdict;

using Request = SCE::Generated::JobRequest::JobRequestPayload;
using Reply = SCE::Generated::JobReply::JobReplyPayload;

constexpr Duration MILLISECOND = 1000;
constexpr Duration SECOND = 1000 * MILLISECOND;

/// How long the system under test takes to answer a request.
constexpr Duration ANSWER_DELAY = 500;

/// The generated machine of each component type a configuration here names.
std::unique_ptr<ComponentMachine> machineOf(const std::string &type) {
    if (type == "asker") {
        return std::make_unique<SCE::Generated::asker::askerComponent>();
    }
    if (type == "coordinator") {
        return std::make_unique<SCE::Generated::coordinator::coordinatorComponent>();
    }
    if (type == "answerer") {
        return std::make_unique<SCE::Generated::answerer::answererComponent>();
    }
    if (type == "keeper") {
        return std::make_unique<SCE::Generated::keeper::keeperComponent>();
    }
    if (type == "idler") {
        return std::make_unique<SCE::Generated::idler::idlerComponent>();
    }
    throw std::out_of_range("no generated component of type " + type);
}

/// The system under test on the mapped port `sut`: it answers each request,
/// ANSWER_DELAY after it, with a reply of `code` for the requested job, or
/// for `job` when one is given. It records what it was sent.
class Responder : public SystemAdapter {
public:
    Responder(std::optional<std::uint32_t> job, std::uint32_t code) : job_(job), code_(code) {}

    void send(const std::string &systemPort, const Message &message, Instant now) override {
        sent.emplace_back(systemPort, message.event);
        const auto *request = std::any_cast<Request>(&message.value);
        if (request == nullptr) {
            return;
        }
        requested.push_back(request->job);
        Reply reply{};
        reply.job = job_.value_or(request->job);
        reply.code = code_;
        pending_.emplace_back(now + ANSWER_DELAY, Message{SCE::Generated::JobReply::EVENT_NAME, reply});
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

    /// Each send the adapter was handed: the system port and the event.
    std::vector<std::pair<std::string, std::string>> sent;
    /// The job each request named.
    std::vector<std::uint32_t> requested;

private:
    std::optional<std::uint32_t> job_;
    std::uint32_t code_;
    std::vector<std::pair<Instant, Message>> pending_;
};

/// A system under test nothing is mapped to.
class Silent : public SystemAdapter {
public:
    void send(const std::string &, const Message &, Instant) override {
        ADD_FAILURE() << "no port is mapped, so nothing reaches the system adapter";
    }

    std::optional<Instant> nextInput() const override {
        return std::nullopt;
    }

    std::vector<std::pair<std::string, Message>> takeInputsDue(Instant) override {
        return {};
    }
};

/// The index of the first record of `kind` by `component`, or none.
std::optional<std::size_t> firstRecord(const TestCaseResult &result, const std::string &component, LogKind kind) {
    for (std::size_t i = 0; i < result.log.size(); ++i) {
        if (result.log[i].component == component && result.log[i].kind == kind) {
            return i;
        }
    }
    return std::nullopt;
}

/// The asker as the MTC, its port `sut` mapped to the system port `sut`.
Configuration askerConfiguration() {
    Configuration configuration;
    configuration.mtcType = "asker";
    configuration.portsOfType = {{"asker", {"sut"}}};
    configuration.timersOfType = {{"asker", {"reply"}}};
    configuration.mappings = {{{"mtc", "sut"}, "sut"}};
    return configuration;
}

TestCaseResult runAsker(Responder &responder) {
    return TestCaseRun(askerConfiguration(), machineOf, responder, std::nullopt).run();
}

/// P6, P2, P4: the send carries the template's value bound to the component's
/// variable, a reply its receive branch's template matches is taken when it
/// arrives, and the content reads the reply as `_event.data`.
TEST(ATestComponent, ThatIsAnsweredWithAMatchingReplyPasses) {
    Responder responder(std::nullopt, 0);
    const TestCaseResult result = runAsker(responder);
    EXPECT_EQ(result.verdict, Verdict::Pass);
    EXPECT_EQ(result.endedAt, ANSWER_DELAY) << "the reply is taken in the round it arrives in";
    ASSERT_EQ(responder.sent.size(), 1u);
    EXPECT_EQ(responder.sent[0].first, "sut");
    EXPECT_EQ(responder.sent[0].second, SCE::Generated::JobRequest::EVENT_NAME)
        << "a message travels under its template's record's event";
    EXPECT_EQ(responder.requested, std::vector<std::uint32_t>{7}) << "the template's value, given `job`";
    EXPECT_TRUE(firstRecord(result, "mtc", LogKind::Receive).has_value());
    // P8: the `<log>` of the branch's content is in the test log, with the
    // instant and the component, its label and the job it read.
    const auto logged = firstRecord(result, "mtc", LogKind::Log);
    ASSERT_TRUE(logged.has_value()) << "the component's <log> is recorded in the test log";
    EXPECT_EQ(result.log[*logged].detail, "answered: 7");
    EXPECT_EQ(result.log[*logged].at, ANSWER_DELAY);
}

/// P2, P7, §7: a reply the template does not match enables no branch and is
/// not removed, and the timer's timeout branch fails the test case.
TEST(ATestComponent, ThatIsAnsweredWithAReplyItsTemplateDoesNotMatchFailsWhenItsTimerFires) {
    Responder responder(std::nullopt, 3);
    const TestCaseResult result = runAsker(responder);
    EXPECT_EQ(result.verdict, Verdict::Fail);
    EXPECT_EQ(result.endedAt, SECOND) << "time advances to the timer's instant";
    EXPECT_FALSE(firstRecord(result, "mtc", LogKind::Receive).has_value())
        << "a reply that does not match is never taken";
    EXPECT_TRUE(firstRecord(result, "mtc", LogKind::Timeout).has_value());
}

/// P4: the content reads the message the branch took: a reply of success for
/// another job matches the template, which takes any job, and the content's
/// comparison of the job it read fails the test case.
TEST(ATestComponent, ReadsTheReplyItTookAsTheEventsData) {
    Responder responder(8, 0);
    const TestCaseResult result = runAsker(responder);
    EXPECT_EQ(result.verdict, Verdict::Fail);
    EXPECT_EQ(result.endedAt, ANSWER_DELAY);
    EXPECT_TRUE(firstRecord(result, "mtc", LogKind::Receive).has_value());
}

/// P9, P6, P2: the MTC creates a PTC connected to it, which asks it for job 42
/// as it starts; the MTC answers with the job it read, the PTC passes on the
/// reply and terminates, and the MTC's done branch passes on the next
/// snapshot. Leaving the invoking state kills the PTC, before the done
/// branch's content runs.
TEST(ATestComponent, CreatesAPtcExchangesAMessageWithItAndReadsItsDone) {
    Configuration configuration;
    configuration.mtcType = "coordinator";
    configuration.ptcTypes = {{"answerer", "answerer"}};
    configuration.portsOfType = {{"coordinator", {"peer"}}, {"answerer", {"peer"}}};
    configuration.connections = {{{"mtc", "peer"}, {"answerer", "peer"}}};
    Silent adapter;
    const TestCaseResult result = TestCaseRun(configuration, machineOf, adapter, std::nullopt).run();

    EXPECT_EQ(result.verdict, Verdict::Pass);
    EXPECT_EQ(result.endedAt, 0) << "a branch fired in every round, so time never moved";
    const auto created = firstRecord(result, "answerer", LogKind::Create);
    const auto asked = firstRecord(result, "answerer", LogKind::Send);
    const auto answered = firstRecord(result, "mtc", LogKind::Send);
    const auto done = firstRecord(result, "answerer", LogKind::Done);
    const auto killed = firstRecord(result, "answerer", LogKind::Kill);
    const auto passed = firstRecord(result, "mtc", LogKind::Verdict);
    ASSERT_TRUE(created && asked && answered && done && killed && passed);
    EXPECT_LT(*created, *asked) << "the PTC is created when the macrostep that entered its state ends";
    // P8: a `<log>` that writes only a label is recorded, by the PTC.
    const auto noted = firstRecord(result, "answerer", LogKind::Log);
    ASSERT_TRUE(noted.has_value());
    EXPECT_EQ(result.log[*noted].detail, "asked");
    EXPECT_LT(*asked, *noted) << "the log runs in its place, after the send before it";
    EXPECT_EQ(result.log[*asked].detail, "peer job.request");
    EXPECT_EQ(result.log[*answered].detail, "peer job.reply");
    EXPECT_LT(*answered, *done);
    EXPECT_LT(*done, *killed) << "a done branch fires on a snapshot taken after the PTC terminated";
    EXPECT_LT(*killed, *passed) << "the invoking state is exited before the done branch's content runs";
}

/// P9, P14: leaving the state that invoked a PTC removes it while it still
/// runs, at that instant, and its local verdict still counts.
TEST(ATestComponent, KillsAPtcOnLeavingTheStateThatInvokedIt) {
    Configuration configuration;
    configuration.mtcType = "keeper";
    configuration.ptcTypes = {{"idler", "idler"}};
    configuration.portsOfType = {{"idler", {"peer"}}};
    configuration.timersOfType = {{"keeper", {"patience", "linger"}}};
    Silent adapter;
    const TestCaseResult result = TestCaseRun(configuration, machineOf, adapter, std::nullopt).run();

    const auto killed = firstRecord(result, "idler", LogKind::Kill);
    ASSERT_TRUE(killed.has_value());
    EXPECT_EQ(result.log[*killed].at, 10 * MILLISECOND) << "the PTC is removed when `patience` runs out";
    EXPECT_FALSE(firstRecord(result, "idler", LogKind::Done).has_value());
    EXPECT_EQ(result.endedAt, 15 * MILLISECOND);
    EXPECT_EQ(result.verdict, Verdict::Inconclusive) << "the removed PTC's inconclusive outweighs the MTC's pass";
}

}  // namespace
