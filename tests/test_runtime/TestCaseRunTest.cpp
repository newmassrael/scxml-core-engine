// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The test profile's runtime (docs/SCE_TEST_PROFILE.md §12.2), held to the
// rules of §2 to §5 with components written here by hand, before any machine
// is generated: each one reads its snapshot and calls its context as a
// generated machine does (§12.3).

#include <gtest/gtest.h>

#include <functional>
#include <map>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "test/TestCaseRun.h"

namespace {

using SCE::Test::ComponentContext;
using SCE::Test::ComponentMachine;
using SCE::Test::Configuration;
using SCE::Test::Duration;
using SCE::Test::Instant;
using SCE::Test::LogKind;
using SCE::Test::Message;
using SCE::Test::Port;
using SCE::Test::SystemAdapter;
using SCE::Test::TestCaseResult;
using SCE::Test::TestCaseRun;
using SCE::Test::Timer;
using SCE::Test::TimerStatus;
using SCE::Test::Verdict;

constexpr Duration SECOND = 1000000;

/// A component written by hand: what it does when it starts, and what it does
/// on each snapshot. Either may end it, as entering a top-level final does.
class Scripted : public ComponentMachine {
public:
    using Step = std::function<void(ComponentContext &, Scripted &)>;

    Scripted(Step onStart, Step onSnapshot) : onStart_(std::move(onStart)), onSnapshot_(std::move(onSnapshot)) {}

    void start(ComponentContext &context) override {
        context_ = &context;
        onStart_(context, *this);
    }

    void deliverSnapshot() override {
        onSnapshot_(*context_, *this);
    }

    bool terminated() const override {
        return final_;
    }

    void end() {
        final_ = true;
    }

private:
    Step onStart_;
    Step onSnapshot_;
    ComponentContext *context_ = nullptr;
    bool final_ = false;
};

/// The machines of a test, by component type.
struct Machines {
    std::map<std::string, std::pair<Scripted::Step, Scripted::Step>> types;

    SCE::Test::MachineFactory factory() const {
        return [types = types](const std::string &type) -> std::unique_ptr<ComponentMachine> {
            const auto &[onStart, onSnapshot] = types.at(type);
            return std::make_unique<Scripted>(onStart, onSnapshot);
        };
    }
};

/// A system adapter that replays a trace and records what was sent to it.
class TraceAdapter : public SystemAdapter {
public:
    struct Input {
        Instant at;
        std::string systemPort;
        std::string event;
    };

    explicit TraceAdapter(std::vector<Input> trace = {}) : trace_(std::move(trace)) {}

    void send(const std::string &systemPort, const Message &message, Instant now) override {
        sent.push_back({now, systemPort, message.event});
    }

    std::optional<Instant> nextInput() const override {
        if (next_ == trace_.size()) {
            return std::nullopt;
        }
        return trace_[next_].at;
    }

    std::vector<std::pair<std::string, Message>> takeInputsDue(Instant now) override {
        std::vector<std::pair<std::string, Message>> due;
        while (next_ < trace_.size() && trace_[next_].at <= now) {
            due.emplace_back(trace_[next_].systemPort, Message{trace_[next_].event, {}});
            ++next_;
        }
        return due;
    }

    std::vector<Input> sent;

private:
    std::vector<Input> trace_;
    std::size_t next_ = 0;
};

Message message(const std::string &event) {
    return Message{event, {}};
}

bool logged(const TestCaseResult &result, const std::string &component, LogKind kind) {
    for (const auto &record : result.log) {
        if (record.component == component && record.kind == kind) {
            return true;
        }
    }
    return false;
}

/// An MTC and one PTC joined by one connection, the MTC's `p` to the PTC's `q`.
Configuration mtcAndEcho() {
    Configuration configuration;
    configuration.mtcType = "Tester";
    configuration.ptcTypes = {{"echo", "Echo"}};
    configuration.portsOfType = {{"Tester", {"p"}}, {"Echo", {"q"}}};
    configuration.timersOfType = {{"Tester", {"reply"}}};
    configuration.connections = {{{"mtc", "p"}, {"echo", "q"}}};
    return configuration;
}

/// The MTC creates the echo, sends to it, and waits for the answer within a
/// second: a receive branch passes, a timeout branch fails.
std::pair<Scripted::Step, Scripted::Step> tester() {
    return {[](ComponentContext &context, Scripted &) {
                context.create("echo");
                context.send("p", message("ping"));
                context.startTimer("reply", SECOND);
            },
            [](ComponentContext &context, Scripted &self) {
                if (context.snapshotOf("p") != nullptr) {
                    EXPECT_EQ(context.receive("p").event, "pong");
                    context.stopTimer("reply");
                    context.setverdict(Verdict::Pass, "");
                    self.end();
                } else if (context.timedOut("reply")) {
                    context.timeout("reply");
                    context.setverdict(Verdict::Fail, "no reply in time");
                    self.end();
                }
            }};
}

TEST(TestVerdict, OnlyEverBecomesWorse) {
    EXPECT_EQ(SCE::Test::worse(Verdict::Pass, Verdict::None), Verdict::Pass);
    EXPECT_EQ(SCE::Test::worse(Verdict::Pass, Verdict::Inconclusive), Verdict::Inconclusive);
    EXPECT_EQ(SCE::Test::worse(Verdict::Fail, Verdict::Pass), Verdict::Fail);
    EXPECT_EQ(SCE::Test::worse(Verdict::Fail, Verdict::Error), Verdict::Error);
}

TEST(TestPort, ASnapshotSeesOnlyWhatWasQueuedWhenItWasTaken) {
    Port port;
    port.takeSnapshot();
    port.enqueue(message("late"));
    EXPECT_EQ(port.snapshot(), nullptr) << "a message that arrives after the snapshot is not in it";
    port.takeSnapshot();
    ASSERT_NE(port.snapshot(), nullptr);
    port.enqueue(message("behind"));
    EXPECT_EQ(port.snapshot()->event, "late") << "a later message goes behind the head";
    EXPECT_EQ(port.removeHead().event, "late");
    EXPECT_EQ(port.size(), 1u);
}

TEST(TestTimer, TimesOutAtItsInstantAndRestartsWhenStartedAgain) {
    Timer timer;
    timer.start(0, 10);
    timer.update(9);
    EXPECT_EQ(timer.status(), TimerStatus::Running);
    timer.start(5, 10);
    EXPECT_EQ(timer.expiry(), std::optional<Instant>(15)) << "a running timer that is started again restarts";
    timer.update(15);
    EXPECT_EQ(timer.status(), TimerStatus::Timeout);
    timer.takeSnapshot();
    EXPECT_TRUE(timer.snapshotTimedOut());
    timer.timeoutTaken();
    EXPECT_EQ(timer.status(), TimerStatus::Idle);
    timer.stop();
    EXPECT_EQ(timer.status(), TimerStatus::Idle) << "stopping a timer that is not running does nothing";
}

/// P6, P9, §3.2: a message sent in a round is taken in a later round, and the
/// whole exchange happens at one instant, because a branch fired in each round.
TEST(TestCaseRunTest, AnAnsweredRequestPassesWithoutTimeMoving) {
    Machines machines;
    machines.types["Tester"] = tester();
    machines.types["Echo"] = {[](ComponentContext &, Scripted &) {},
                              [](ComponentContext &context, Scripted &) {
                                  if (context.snapshotOf("q") != nullptr) {
                                      EXPECT_EQ(context.receive("q").event, "ping");
                                      context.send("q", message("pong"));
                                  }
                              }};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(mtcAndEcho(), machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Pass);
    EXPECT_EQ(result.endedAt, 0);
    EXPECT_TRUE(logged(result, "echo", LogKind::Kill)) << "the MTC's end removes the PTC (P10)";
}

/// P7, §3.3: with nothing to receive, time advances to the timer's instant.
TEST(TestCaseRunTest, AnUnansweredRequestFailsWhenItsTimerExpires) {
    Machines machines;
    machines.types["Tester"] = tester();
    machines.types["Echo"] = {[](ComponentContext &, Scripted &) {}, [](ComponentContext &, Scripted &) {}};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(mtcAndEcho(), machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Fail);
    EXPECT_EQ(result.endedAt, SECOND);
}

/// P13: no branch can fire and no instant is pending, so the test case can
/// never end; the runtime ends it with error.
TEST(TestCaseRunTest, ATestCaseThatCanNeverEndIsAnError) {
    Configuration configuration;
    configuration.mtcType = "Waiter";
    configuration.portsOfType = {{"Waiter", {"p"}}};
    configuration.mappings = {{{"mtc", "p"}, "sut"}};
    Machines machines;
    machines.types["Waiter"] = {[](ComponentContext &, Scripted &) {}, [](ComponentContext &, Scripted &) {}};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(configuration, machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Error);
    EXPECT_TRUE(logged(result, "mtc", LogKind::Error));
}

/// P11: the guard instant ends a test case that has not ended, with error.
TEST(TestCaseRunTest, TheGuardEndsATestCaseWithError) {
    Configuration configuration;
    configuration.mtcType = "Waiter";
    configuration.portsOfType = {{"Waiter", {"p"}}};
    configuration.mappings = {{{"mtc", "p"}, "sut"}};
    Machines machines;
    machines.types["Waiter"] = {[](ComponentContext &context, Scripted &) { context.setverdict(Verdict::Pass, ""); },
                                [](ComponentContext &, Scripted &) {}};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(configuration, machines.factory(), adapter, 5 * SECOND).run();
    EXPECT_EQ(result.verdict, Verdict::Error);
    EXPECT_EQ(result.endedAt, 5 * SECOND);
}

/// §2.2, §12.2: a mapped port's sends reach the adapter, and the trace's
/// inputs reach the port at their instants.
TEST(TestCaseRunTest, AMappedPortTalksToTheSystemAdapterOnTheLogicalClock) {
    Configuration configuration;
    configuration.mtcType = "Tester";
    configuration.portsOfType = {{"Tester", {"p"}}};
    configuration.timersOfType = {{"Tester", {"reply"}}};
    configuration.mappings = {{{"mtc", "p"}, "sut"}};
    Machines machines;
    machines.types["Tester"] = {[](ComponentContext &context, Scripted &) {
                                    context.send("p", message("ping"));
                                    context.startTimer("reply", SECOND);
                                },
                                tester().second};
    TraceAdapter adapter({{500, "sut", "pong"}});
    TestCaseResult result = TestCaseRun(configuration, machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Pass);
    EXPECT_EQ(result.endedAt, 500);
    ASSERT_EQ(adapter.sent.size(), 1u);
    EXPECT_EQ(adapter.sent[0].systemPort, "sut");
    EXPECT_EQ(adapter.sent[0].event, "ping");
}

/// P9, P14: a removed PTC's verdict still counts in the test case verdict.
TEST(TestCaseRunTest, ARemovedPtcsVerdictStillCounts) {
    Configuration configuration = mtcAndEcho();
    Machines machines;
    machines.types["Tester"] = {[](ComponentContext &context, Scripted &) {
                                    context.create("echo");
                                    context.startTimer("reply", 10);
                                },
                                [](ComponentContext &context, Scripted &self) {
                                    if (context.timedOut("reply")) {
                                        context.timeout("reply");
                                        context.kill("echo");
                                        context.setverdict(Verdict::Pass, "");
                                        self.end();
                                    }
                                }};
    machines.types["Echo"] = {[](ComponentContext &context, Scripted &) { context.setverdict(Verdict::Fail, ""); },
                              [](ComponentContext &, Scripted &) {}};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(configuration, machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Fail);
}

/// §3.2 step 4: components select one at a time in creation order, so when
/// the MTC terminates in a round, a PTC that would have selected after it in
/// that round does not.
TEST(TestCaseRunTest, NothingSelectsAfterTheMtcTerminatesInTheRound) {
    Configuration configuration = mtcAndEcho();
    configuration.timersOfType["Echo"] = {"late"};
    Machines machines;
    machines.types["Tester"] = {[](ComponentContext &context, Scripted &) {
                                    context.create("echo");
                                    context.startTimer("reply", 10);
                                },
                                [](ComponentContext &context, Scripted &self) {
                                    if (context.timedOut("reply")) {
                                        context.timeout("reply");
                                        context.setverdict(Verdict::Pass, "");
                                        self.end();
                                    }
                                }};
    machines.types["Echo"] = {[](ComponentContext &context, Scripted &) { context.startTimer("late", 10); },
                              [](ComponentContext &context, Scripted &) {
                                  if (context.timedOut("late")) {
                                      context.timeout("late");
                                      context.setverdict(Verdict::Fail, "");
                                  }
                              }};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(configuration, machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Pass);
    EXPECT_FALSE(logged(result, "echo", LogKind::Timeout));
}

/// P2, P10: a PTC that terminates is in DONE from the next snapshot on, and a
/// done branch removes nothing.
TEST(TestCaseRunTest, ADoneBranchSeesAPtcThatTerminated) {
    Configuration configuration = mtcAndEcho();
    Machines machines;
    machines.types["Tester"] = {[](ComponentContext &context, Scripted &) { context.create("echo"); },
                                [](ComponentContext &context, Scripted &self) {
                                    if (context.done("echo")) {
                                        context.doneTaken("echo");
                                        context.kill("echo");
                                        self.end();
                                    }
                                }};
    machines.types["Echo"] = {[](ComponentContext &context, Scripted &self) {
                                  context.setverdict(Verdict::Pass, "");
                                  self.end();
                              },
                              [](ComponentContext &, Scripted &) {}};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(configuration, machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Pass);
    EXPECT_TRUE(logged(result, "echo", LogKind::Done));
}

/// P6, P13: a send on a port whose peer is not present is a dynamic error.
TEST(TestCaseRunTest, ASendToAnAbsentPeerIsAnError) {
    Configuration configuration = mtcAndEcho();
    Machines machines;
    machines.types["Tester"] = {[](ComponentContext &context, Scripted &) { context.send("p", message("ping")); },
                                [](ComponentContext &, Scripted &) {}};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(configuration, machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Error);
    EXPECT_EQ(result.endedAt, 0);
}

}  // namespace
