// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The test profile's runtime (docs/SCE_TEST_PROFILE.md §12.2), held to the
// rules of §2 to §5 with components written here by hand, before any machine
// is generated: each one reads its snapshot and calls its context as a
// generated machine does (§12.3). The runner of a suite (test/Suite.h) is
// held to P11 over test cases made of the same components.

#include <gtest/gtest.h>

#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"
#include "saved/SavedValues.h"
#include "test/Canonical.h"
#include "test/Suite.h"
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

/// A record that holds no field.
struct Empty {};

/// The canonical form of a message whose record holds no field.
SCE::Saved::JsonValue noFields(const Empty &) {
    return SCE::Saved::JsonValue(SCE::Saved::JsonObject{});
}

/// A message of the record `event` that holds no field: `{}` in the test log,
/// of the record type `empty`.
Message message(const std::string &event) {
    return Message{event, SCE::Test::messageValue(Empty{}, "empty", noFields)};
}

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
            due.emplace_back(trace_[next_].systemPort, message(trace_[next_].event));
            ++next_;
        }
        return due;
    }

    std::vector<Input> sent;

private:
    std::vector<Input> trace_;
    std::size_t next_ = 0;
};

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

/// P2, §12.3: a receive branch with a template matches only a message of its
/// record, under its record's event, whose value its `match` holds; the guard
/// reads the snapshot and removes nothing.
TEST(TestCaseRunTest, AReceiveWithATemplateMatchesOnlyItsRecordsValue) {
    Configuration configuration;
    configuration.mtcType = "Reader";
    configuration.portsOfType = {{"Reader", {"p"}}};
    configuration.mappings = {{{"mtc", "p"}, "sut"}};
    std::vector<std::string> seen;
    Machines machines;
    machines.types["Reader"] = {[](ComponentContext &, Scripted &) {},
                                [&seen](ComponentContext &context, Scripted &self) {
                                    const auto isSeven = [](const int &value) { return value == 7; };
                                    if (SCE::Test::receives<int>(context, "p", "number", isSeven)) {
                                        seen.push_back("seven");
                                        EXPECT_TRUE(SCE::Test::receives<int>(context, "p", "number", isSeven))
                                            << "a guard that matched removed the message";
                                        context.receive("p");
                                        self.end();
                                    } else if (context.snapshotOf("p") != nullptr) {
                                        seen.push_back(context.snapshotOf("p")->event);
                                        context.receive("p");
                                    }
                                }};

    class Inputs : public TraceAdapter {
    public:
        std::vector<std::pair<std::string, Message>> takeInputsDue(Instant now) override {
            std::vector<std::pair<std::string, Message>> due;
            if (now == 0 && !given_) {
                given_ = true;
                const auto written = [](const std::string &text) { return SCE::Test::Canonical::text(text); };
                const auto number = [](const int &n) { return SCE::Test::Canonical::integer(n); };
                due.emplace_back("sut", Message{"number", SCE::Test::messageValue(std::string("7"), "text", written)});
                due.emplace_back("sut", Message{"other", SCE::Test::messageValue(7, "number", number)});
                due.emplace_back("sut", Message{"number", SCE::Test::messageValue(6, "number", number)});
                due.emplace_back("sut", Message{"number", SCE::Test::messageValue(7, "number", number)});
            }
            return due;
        }

    private:
        bool given_ = false;
    };

    Inputs adapter;
    TestCaseResult result = TestCaseRun(configuration, machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(seen, (std::vector<std::string>{"number", "other", "number", "seven"}))
        << "a value of another type, another event and another value do not match";
    EXPECT_EQ(result.verdict, Verdict::None);
}

/// §11.5, §11.8: a component reads the module parameters of its run into its
/// variables before it starts, as a generated machine reads each through the
/// reader of the type it declares.
Scripted::Step readsJob(std::optional<std::uint32_t> &job) {
    return [&job](ComponentContext &context, Scripted &self) {
        const bool read = context.readModuleParameters([&job](const SCE::Test::ModuleParameters &given) {
            job = SCE::Saved::savedReadUint32(given.of("job"), "job");
        });
        if (!read) {
            return;
        }
        context.setverdict(Verdict::Pass, "");
        self.end();
    };
}

Configuration lone(const std::string &type) {
    Configuration configuration;
    configuration.mtcType = type;
    return configuration;
}

/// §11.8: the value the run is given is the one the component reads, and the
/// component starts on it.
TEST(TestCaseRunTest, AComponentReadsTheModuleParameterItsRunIsGiven) {
    std::optional<std::uint32_t> job;
    Machines machines;
    machines.types["Reader"] = {readsJob(job), [](ComponentContext &, Scripted &) {}};
    SCE::Test::ModuleParameters parameters;
    parameters.set("job", SCE::Saved::savedWriteUint32(7));
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(lone("Reader"), machines.factory(), adapter, std::nullopt, parameters).run();
    EXPECT_EQ(job, std::optional<std::uint32_t>(7));
    EXPECT_EQ(result.verdict, Verdict::Pass);
}

/// P13: a module parameter the run has no value for is a dynamic error, named
/// in the log, and the component goes no further than the read.
TEST(TestCaseRunTest, AModuleParameterTheRunIsNotGivenIsAnError) {
    std::optional<std::uint32_t> job;
    Machines machines;
    machines.types["Reader"] = {readsJob(job), [](ComponentContext &, Scripted &) {}};
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(lone("Reader"), machines.factory(), adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Error);
    EXPECT_FALSE(job.has_value());
    ASSERT_FALSE(result.log.empty());
    EXPECT_EQ(result.log.back().kind, LogKind::Error);
    EXPECT_NE(result.log.back().detail.find("'job'"), std::string::npos) << result.log.back().detail;
}

/// P13: a value that is not of the type the component declares is refused as a
/// restored value of another type is, and not converted.
TEST(TestCaseRunTest, AModuleParameterOfAnotherTypeIsAnError) {
    std::optional<std::uint32_t> job;
    Machines machines;
    machines.types["Reader"] = {readsJob(job), [](ComponentContext &, Scripted &) {}};
    SCE::Test::ModuleParameters parameters;
    parameters.set("job", SCE::Saved::savedWriteString("seven"));
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(lone("Reader"), machines.factory(), adapter, std::nullopt, parameters).run();
    EXPECT_EQ(result.verdict, Verdict::Error);
    EXPECT_FALSE(job.has_value());
    EXPECT_TRUE(logged(result, "mtc", LogKind::Error));
}

/// P13: a component type the factory of machines makes no machine for ends the
/// run with error where a component of it is created.
TEST(TestCaseRunTest, AComponentTypeWithNoMachineIsAnError) {
    SCE::Test::MachineFactory none = [](const std::string &) -> std::unique_ptr<ComponentMachine> { return nullptr; };
    TraceAdapter adapter;
    TestCaseResult result = TestCaseRun(lone("Ghost"), none, adapter, std::nullopt).run();
    EXPECT_EQ(result.verdict, Verdict::Error);
    EXPECT_TRUE(logged(result, "mtc", LogKind::Error));
    EXPECT_FALSE(logged(result, "mtc", LogKind::Start)) << "no component of the type was created";
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

// ── The runner of a suite (test/Suite.h, P11) ────────────────────────

/// A test case whose MTC, of type @p type, sets @p verdict as it starts and
/// ends there.
SCE::Test::TestCase endsWith(const std::string &name, const std::string &type, Verdict verdict) {
    Machines machines;
    machines.types[type] = {[verdict](ComponentContext &context, Scripted &self) {
                                context.setverdict(verdict, "");
                                self.end();
                            },
                            [](ComponentContext &, Scripted &) {}};
    return SCE::Test::TestCase{name, lone(type), machines.factory(), {}};
}

/// The adapters of a suite's run, each a fresh TraceAdapter, recording the test
/// case each was built for.
SCE::Test::AdapterFactory tracing(std::vector<std::string> &built) {
    return [&built](const std::string &testCase) -> std::unique_ptr<SystemAdapter> {
        built.push_back(testCase);
        return std::make_unique<TraceAdapter>();
    };
}

/// P11: the runner runs the test cases a world selects, in the suite's order,
/// and skips the others; a test case with no selection runs in every world.
TEST(TestSuiteRun, RunsTheTestCasesAWorldSelectsInTheSuitesOrder) {
    SCE::Test::Suite suite;
    suite.name = "ordered";
    suite.executes.push_back(SCE::Test::Execute{endsWith("first", "First", Verdict::Pass), std::nullopt, nullptr});
    suite.executes.push_back(
        SCE::Test::Execute{endsWith("second", "Second", Verdict::Fail), std::nullopt,
                           [](const SCE::Test::Features &world) { return SCE::Test::enabled(world, "x"); }});
    suite.executes.push_back(
        SCE::Test::Execute{endsWith("third", "Third", Verdict::Inconclusive), std::nullopt,
                           [](const SCE::Test::Features &world) { return SCE::Test::enabled(world, "y"); }});
    std::vector<std::string> built;
    const auto results = SCE::Test::runSuite(suite, {"y"}, tracing(built));
    ASSERT_EQ(results.size(), 2u);
    EXPECT_EQ(results[0].testCase, "first");
    EXPECT_EQ(results[0].result.verdict, Verdict::Pass);
    EXPECT_EQ(results[1].testCase, "third");
    EXPECT_EQ(results[1].result.verdict, Verdict::Inconclusive);
    EXPECT_EQ(built, (std::vector<std::string>{"first", "third"})) << "an adapter is built for each case run";
}

/// P11: each test case is a fresh run from NOW = 0 under its own guard.
TEST(TestSuiteRun, EachTestCaseIsAFreshRunUnderItsOwnGuard) {
    Machines waiting;
    waiting.types["Waiter"] = {[](ComponentContext &, Scripted &) {}, [](ComponentContext &, Scripted &) {}};
    Configuration waiter = lone("Waiter");
    SCE::Test::Suite suite;
    suite.name = "guarded";
    suite.executes.push_back(
        SCE::Test::Execute{SCE::Test::TestCase{"stuck", waiter, waiting.factory(), {}}, 5 * SECOND, nullptr});
    suite.executes.push_back(SCE::Test::Execute{endsWith("quick", "Quick", Verdict::Pass), std::nullopt, nullptr});
    std::vector<std::string> built;
    const auto results = SCE::Test::runSuite(suite, {}, tracing(built));
    ASSERT_EQ(results.size(), 2u);
    EXPECT_EQ(results[0].result.verdict, Verdict::Error);
    EXPECT_EQ(results[0].result.endedAt, 5 * SECOND) << "the guard ends the first";
    EXPECT_EQ(results[1].result.verdict, Verdict::Pass);
    EXPECT_EQ(results[1].result.endedAt, 0) << "the second starts again at NOW = 0";
}

/// -1 §8.2.1: a value the caller gives a module parameter replaces the suite's
/// default, and a parameter the suite does not declare cannot be given one.
TEST(TestSuiteRun, AnOverrideReplacesTheDefaultOfAModuleParameterTheSuiteDeclares) {
    std::optional<std::uint32_t> job;
    Machines machines;
    machines.types["Reader"] = {readsJob(job), [](ComponentContext &, Scripted &) {}};
    SCE::Test::Suite suite;
    suite.name = "parameters";
    suite.moduleParameters.set("job", SCE::Saved::savedWriteUint32(7));
    suite.executes.push_back(SCE::Test::Execute{
        SCE::Test::TestCase{"reads", lone("Reader"), machines.factory(), {"job"}}, std::nullopt, nullptr});
    std::vector<std::string> built;
    auto results = SCE::Test::runSuite(suite, {}, tracing(built));
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(results[0].result.verdict, Verdict::Pass);
    EXPECT_EQ(job, std::optional<std::uint32_t>(7)) << "the default";

    SCE::Test::ModuleParameters overrides;
    overrides.set("job", SCE::Saved::savedWriteUint32(9));
    results = SCE::Test::runSuite(suite, {}, tracing(built), overrides);
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(job, std::optional<std::uint32_t>(9)) << "the override";

    SCE::Test::ModuleParameters stray;
    stray.set("limit", SCE::Saved::savedWriteUint32(1));
    EXPECT_THROW(SCE::Test::runSuite(suite, {}, tracing(built), stray), std::invalid_argument);
}

/// P13: a test case whose components read a module parameter the run has no
/// value for ends with error before anything of it runs, its adapter included.
TEST(TestSuiteRun, ATestCaseThatReadsAModuleParameterWithNoValueIsAnError) {
    SCE::Test::TestCase reads = endsWith("reads", "Reader", Verdict::Pass);
    reads.moduleParameters = {"job"};
    SCE::Test::Suite suite;
    suite.name = "undeclared";
    suite.executes.push_back(SCE::Test::Execute{reads, std::nullopt, nullptr});
    std::vector<std::string> built;
    const auto results = SCE::Test::runSuite(suite, {}, tracing(built));
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(results[0].result.verdict, Verdict::Error);
    ASSERT_EQ(results[0].result.log.size(), 1u);
    EXPECT_NE(results[0].result.log[0].detail.find("'job'"), std::string::npos);
    EXPECT_TRUE(built.empty()) << "no adapter is built for a case that cannot start";
}

/// P13: a test case the caller gives no system adapter ends with error.
TEST(TestSuiteRun, ATestCaseWithNoSystemAdapterIsAnError) {
    SCE::Test::Suite suite;
    suite.name = "unadapted";
    suite.executes.push_back(SCE::Test::Execute{endsWith("only", "Only", Verdict::Pass), std::nullopt, nullptr});
    const auto results =
        SCE::Test::runSuite(suite, {}, [](const std::string &) -> std::unique_ptr<SystemAdapter> { return nullptr; });
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(results[0].result.verdict, Verdict::Error);
}

// ── The test log (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3) ─
//
// Each record written where the semantics places its event, in the order the
// rounds produce them, and the whole log in its one canonical text. The twins
// are the Rust runtime's, module `the_test_log` of
// backends/rust/runtime/tests/a_test_case_runs_as_the_test_profile_says.rs,
// whose logs these are byte for byte.

/// An MTC and its echo, joined `p` to `q`; the MTC has two timers.
Configuration pinging() {
    Configuration configuration;
    configuration.mtcType = "Tester";
    configuration.ptcTypes = {{"echo", "Echo"}};
    configuration.portsOfType = {{"Tester", {"p"}}, {"Echo", {"q"}}};
    configuration.timersOfType = {{"Tester", {"t", "s"}}};
    configuration.connections = {{{"mtc", "p"}, {"echo", "q"}}};
    return configuration;
}

/// The ping's value, a record of one number of the record type `ping`, as the
/// unit of a template over it writes it.
Message ping(std::uint32_t n) {
    return Message{"ping", SCE::Test::messageValue(n, "ping", [](const std::uint32_t &held) {
                       return SCE::Test::Canonical::record({{"n", SCE::Test::Canonical::integer(held)}});
                   })};
}

/// The MTC creates the echo, pings it, starts two timers and logs; at the first
/// timer's timeout it stops the second, and the first again, which no longer
/// runs, and passes. The echo takes the ping, passes with a reason, and
/// terminates.
Machines pingingMachines() {
    Machines machines;
    machines.types["Tester"] = {[](ComponentContext &context, Scripted &) {
                                    context.create("echo");
                                    context.send("p", ping(3));
                                    context.startTimer("t", 10);
                                    context.startTimer("s", 20);
                                    context.log("asked \"echo\"");
                                },
                                [](ComponentContext &context, Scripted &self) {
                                    if (context.timedOut("t")) {
                                        context.timeout("t");
                                        context.stopTimer("s");
                                        context.stopTimer("t");
                                        context.setverdict(Verdict::Pass, "");
                                        self.end();
                                    }
                                }};
    machines.types["Echo"] = {[](ComponentContext &, Scripted &) {},
                              [](ComponentContext &context, Scripted &self) {
                                  if (context.snapshotOf("q") != nullptr) {
                                      context.receive("q");
                                      context.setverdict(Verdict::Pass, "pinged");
                                      self.end();
                                  }
                              }};
    return machines;
}

/// A suite of the one test case @p testCase, named `logged_suite`.
SCE::Test::Suite suiteOf(SCE::Test::TestCase testCase) {
    SCE::Test::Suite suite;
    suite.name = "logged_suite";
    suite.executes.push_back(SCE::Test::Execute{std::move(testCase), std::nullopt, nullptr});
    return suite;
}

/// P4, P6, P7, P8, P9, P10, P12, P14: the whole log of a run through the
/// suite's runner. The echo's work comes in the round after the MTC's start, at
/// NOW = 0, the timeout at 10; the stop of a timer that no longer runs, and the
/// removal of the terminated echo when the MTC ends, write no record.
TEST(TheTestLog, ARunWritesEachRecordWhereItsEventHappens) {
    const SCE::Test::Suite suite =
        suiteOf(SCE::Test::TestCase{"pings_the_echo", pinging(), pingingMachines().factory(), {}});
    std::vector<std::string> built;
    const auto results = SCE::Test::runSuite(suite, {}, tracing(built));
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(results[0].canonicalLog(suite.name),
              "{\"records\":["
              "{\"at\":\"0\",\"kind\":\"test-case-started\"},"
              "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"component-created\",\"type\":\"Tester\"},"
              "{\"at\":\"0\",\"component\":\"echo\",\"kind\":\"component-created\",\"type\":\"Echo\"},"
              "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"message-sent\",\"port\":\"p\","
              "\"type\":\"ping\",\"value\":{\"n\":\"3\"}},"
              "{\"at\":\"0\",\"component\":\"mtc\",\"duration_us\":\"10\",\"kind\":\"timer-started\","
              "\"timer\":\"t\"},"
              "{\"at\":\"0\",\"component\":\"mtc\",\"duration_us\":\"20\",\"kind\":\"timer-started\","
              "\"timer\":\"s\"},"
              "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"log\",\"text\":\"asked \\\"echo\\\"\"},"
              "{\"at\":\"0\",\"component\":\"echo\",\"kind\":\"message-received\",\"port\":\"q\","
              "\"type\":\"ping\",\"value\":{\"n\":\"3\"}},"
              "{\"at\":\"0\",\"component\":\"echo\",\"kind\":\"verdict-set\",\"reason\":\"pinged\","
              "\"verdict\":\"pass\"},"
              "{\"at\":\"0\",\"component\":\"echo\",\"kind\":\"component-done\",\"verdict\":\"pass\"},"
              "{\"at\":\"10\",\"component\":\"mtc\",\"kind\":\"timer-timed-out\",\"timer\":\"t\"},"
              "{\"at\":\"10\",\"component\":\"mtc\",\"kind\":\"timer-stopped\",\"timer\":\"s\"},"
              "{\"at\":\"10\",\"component\":\"mtc\",\"kind\":\"verdict-set\",\"verdict\":\"pass\"},"
              "{\"at\":\"10\",\"component\":\"mtc\",\"kind\":\"component-done\",\"verdict\":\"pass\"},"
              "{\"at\":\"10\",\"kind\":\"test-case-ended\",\"verdict\":\"pass\"}"
              "],\"schema\":\"sce-test-log.v1\",\"suite\":\"logged_suite\","
              "\"test_case\":\"pings_the_echo\",\"verdict\":\"pass\"}");
}

/// P13: an error the runtime sets is a verdict without the runtime's own
/// reason, which another runtime would not write alike.
TEST(TheTestLog, AnErrorTheRuntimeSetsIsLoggedWithoutItsCause) {
    Machines machines;
    machines.types["Waiter"] = {[](ComponentContext &, Scripted &) {}, [](ComponentContext &, Scripted &) {}};
    TraceAdapter adapter;
    const TestCaseResult result = TestCaseRun(lone("Waiter"), machines.factory(), adapter, 7).run();
    EXPECT_EQ(result.verdict, Verdict::Error);
    EXPECT_EQ(result.canonicalLog("logged_suite", "waits"),
              "{\"records\":["
              "{\"at\":\"0\",\"kind\":\"test-case-started\"},"
              "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"component-created\",\"type\":\"Waiter\"},"
              "{\"at\":\"7\",\"component\":\"mtc\",\"kind\":\"verdict-set\",\"verdict\":\"error\"},"
              "{\"at\":\"7\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}"
              "],\"schema\":\"sce-test-log.v1\",\"suite\":\"logged_suite\","
              "\"test_case\":\"waits\",\"verdict\":\"error\"}");
}

/// P13: a test case that cannot start, for want of an adapter, logs its start
/// and its end alone, since no component was created.
TEST(TheTestLog, ATestCaseThatCannotStartLogsItsStartAndEnd) {
    const SCE::Test::Suite suite = suiteOf(SCE::Test::TestCase{"unplayed", pinging(), pingingMachines().factory(), {}});
    const auto results =
        SCE::Test::runSuite(suite, {}, [](const std::string &) -> std::unique_ptr<SystemAdapter> { return nullptr; });
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(results[0].canonicalLog(suite.name), "{\"records\":["
                                                   "{\"at\":\"0\",\"kind\":\"test-case-started\"},"
                                                   "{\"at\":\"0\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}"
                                                   "],\"schema\":\"sce-test-log.v1\",\"suite\":\"logged_suite\","
                                                   "\"test_case\":\"unplayed\",\"verdict\":\"error\"}");
}

}  // namespace
