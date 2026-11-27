// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The trace port (docs/adr/0115, D4 and D8) -- C++ AOT channel.
//
// This target builds a_computed_event_name_is_matched_like_any_other with
// `SCE_TRACE_PORT=1` (tests/CMakeLists.txt), so its engine holds a trace
// recorder and `setTracePort` is a member of it. The document takes six
// transitions, one out of each of s1 to s6, each on an event whose name a
// `<send>` computed: three off its external queue (s1's now, s2's after a delay,
// s6's addressed to this session by its id) and three off its internal queue
// (s3's now, s4's after a delay, s5's to a target it computed). So the run is a
// start, six event steps and a stop, and this holds what the trace port is
// told:
//
//   * the steps in that order, each event step of the kind its queue is and
//     naming the whole name its event was sent under, which is not the name of
//     the event of the document it matched (`request`, or the wildcard);
//   * each event step's transition, the state it exited, the state it entered
//     and the configuration it left;
//   * the transitions the steps took are the ones the transition observer is
//     told, in the order it is told them: the two hooks are told one run;
//   * the start enters s1, and the stop exits `pass` and names it the run's
//     outcome;
//   * a port set and cleared again is told nothing, and the run is the same.
//
// The W3C documents docs/formal has models of take no external event, so this
// is the target that holds the external step; the C11 lane's twin is
// backends/c/tests/integration/test_a_traced_run_tells_each_step_it_takes.c.
// The same machine built with nothing set is
// AComputedEventNameIsMatchedLikeAnyOtherAotTest.cpp, in integration_tests,
// where it holds no port.
//
// Fixture:
// integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml

#include "a_computed_event_name_is_matched_like_any_other_sm.h"
#include "core/TracePort.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstddef>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {
namespace {

using TracedSM =
    SCE::Generated::a_computed_event_name_is_matched_like_any_other::a_computed_event_name_is_matched_like_any_other;
using ::SCE::Core::TraceStep;
using ::SCE::Core::TraceStepKind;

static_assert(TracedSM::HAS_TRACE_PORT, "this target is built with SCE_TRACE_PORT=1 (tests/CMakeLists.txt)");

/// What one run told its two hooks, and where it ended.
struct TracedRun {
    std::vector<TraceStep> told;
    std::vector<TracedSM::TransitionTaken> observed;
    bool reachedPass = false;
};

/// Runs the machine to its end with a trace port and a transition observer set
/// before the start, the port cleared again before the start when `clear`.
TracedRun runTraced(bool clear) {
    TracedRun run;
    TracedSM sm;
    if constexpr (TracedSM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter: the engine's lifetime is the
        // ScriptEngineProvider singleton's, and the shared_ptr is a view of it.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.setTracePort([&run](const TraceStep &step) { run.told.push_back(step); });
    sm.setTransitionObserver([&run](const TracedSM::TransitionTaken &taken) { run.observed.push_back(taken); });
    if (clear) {
        sm.clearTracePort();
    }
    EXPECT_EQ(sm.initialize(), std::nullopt);
    run.reachedPass = sm.runUntilCompletion(std::chrono::seconds(3)) && sm.terminalState() == TracedSM::State::Pass;
    return run;
}

/// The one name a list of a step holds, or a word saying it holds another
/// number of them.
std::string only(const std::vector<const char *> &names) {
    if (names.size() != 1) {
        return "<" + std::to_string(names.size()) + " names>";
    }
    return names.front();
}

/// An event step the run must take: the queue its event comes off, the name it
/// was sent under, and the state its transition leaves for the next.
struct ExpectedEventStep {
    TraceStepKind kind;
    const char *event;
    const char *source;
    const char *target;
};

constexpr ExpectedEventStep EXPECTED[] = {
    {TraceStepKind::External, "request.new", "s1", "s2"},   {TraceStepKind::External, "other.thing", "s2", "s3"},
    {TraceStepKind::Internal, "request.again", "s3", "s4"}, {TraceStepKind::Internal, "last.one", "s4", "s5"},
    {TraceStepKind::Internal, "request.third", "s5", "s6"}, {TraceStepKind::External, "last.two", "s6", "pass"},
};

constexpr std::size_t EXPECTED_EVENTS = sizeof(EXPECTED) / sizeof(EXPECTED[0]);

}  // namespace

// A start, the six event steps in the order the run takes them, and a stop.
TEST(ATracedRunTellsEachStepItTakesAotTest, EachStepIsToldInTheOrderTheRunTakesIt) {
    const TracedRun run = runTraced(false);
    ASSERT_TRUE(run.reachedPass) << "the traced run did not end in `pass`";
    ASSERT_EQ(run.told.size(), EXPECTED_EVENTS + 2)
        << "the port was told another number of steps than a start, " << EXPECTED_EVENTS << " event steps and a stop";

    const TraceStep &start = run.told.front();
    EXPECT_EQ(start.kind, TraceStepKind::Start);
    EXPECT_FALSE(start.event.has_value());
    EXPECT_TRUE(start.fired.empty());
    EXPECT_TRUE(start.exited.empty());
    EXPECT_EQ(only(start.entered), "s1") << "the start enters s1 alone";
    EXPECT_EQ(only(start.configuration), "s1");

    for (std::size_t i = 0; i < EXPECTED_EVENTS; ++i) {
        const TraceStep &step = run.told[i + 1];
        const ExpectedEventStep &expected = EXPECTED[i];
        EXPECT_EQ(step.kind, expected.kind) << "step " << i + 1 << " is " << SCE::Core::traceStepKindName(step.kind)
                                            << ", not " << SCE::Core::traceStepKindName(expected.kind);
        EXPECT_EQ(step.event, std::optional<std::string>(expected.event))
            << "step " << i + 1 << " does not name the event by the name it was sent under";
        ASSERT_EQ(step.fired.size(), 1u) << "step " << i + 1 << " takes one transition";
        EXPECT_EQ(std::string(step.fired.front().source), expected.source);
        EXPECT_EQ(step.fired.front().index, 0u);
        EXPECT_EQ(only(step.exited), expected.source) << "step " << i + 1;
        EXPECT_EQ(only(step.entered), expected.target) << "step " << i + 1;
        EXPECT_EQ(only(step.configuration), expected.target) << "step " << i + 1;
        EXPECT_EQ(step.outcome, nullptr);
    }

    const TraceStep &stop = run.told.back();
    EXPECT_EQ(stop.kind, TraceStepKind::Stop);
    EXPECT_FALSE(stop.event.has_value());
    EXPECT_TRUE(stop.fired.empty());
    EXPECT_EQ(only(stop.exited), "pass") << "the stop exits `pass` alone";
    EXPECT_TRUE(stop.entered.empty());
    EXPECT_TRUE(stop.configuration.empty());
    ASSERT_NE(stop.outcome, nullptr);
    EXPECT_EQ(std::string(stop.outcome), "pass");
}

// The transitions the steps took, in the order they took them, are the ones the
// transition observer is told.
TEST(ATracedRunTellsEachStepItTakesAotTest, TheStepsTakeTheTransitionsTheObserverIsTold) {
    const TracedRun run = runTraced(false);
    ASSERT_TRUE(run.reachedPass) << "the traced run did not end in `pass`";
    std::vector<std::string> taken;
    for (const TraceStep &step : run.told) {
        for (const auto &fired : step.fired) {
            taken.push_back(std::string(fired.source) + "#" + std::to_string(fired.index));
        }
    }
    std::vector<std::string> observed;
    for (const TracedSM::TransitionTaken &row : run.observed) {
        observed.push_back(std::string(TracedSM::PolicyType::getStateName(row.source)) + "#" +
                           std::to_string(row.index));
    }
    EXPECT_EQ(taken, observed);
    EXPECT_EQ(taken.size(), EXPECTED_EVENTS);
}

// Every step the port is told is one the run trace format's writer writes as
// its kind says, and the whole run one step to a line.
TEST(ATracedRunTellsEachStepItTakesAotTest, EveryStepToldIsWrittenInTheRunTraceFormat) {
    const TracedRun run = runTraced(false);
    ASSERT_TRUE(run.reachedPass) << "the traced run did not end in `pass`";
    SCE::Core::RunTrace trace;
    for (const TraceStep &step : run.told) {
        trace.push(step);
    }
    const std::string text = trace.toJson();
    EXPECT_NE(text.find(R"({"kind": "external", "event": "request.new", "fired": [{"source": "s1", "index": 0}], )"
                        R"("exited": ["s1"], "entered": ["s2"], "configuration": ["s2"]})"),
              std::string::npos)
        << text;
    EXPECT_NE(text.find(R"({"kind": "stop", "exited": ["pass"], "outcome": "pass"})"), std::string::npos) << text;
}

// A port set and then cleared is told nothing, and the run still reaches `pass`,
// as one that never had a port does.
TEST(ATracedRunTellsEachStepItTakesAotTest, AClearedPortIsToldNothingAndTheRunIsTheSame) {
    const TracedRun run = runTraced(true);
    EXPECT_TRUE(run.reachedPass) << "the run did not end in `pass`";
    EXPECT_TRUE(run.told.empty()) << "a cleared port was told " << run.told.size() << " steps";
    EXPECT_EQ(run.observed.size(), EXPECTED_EVENTS) << "the observer, still set, was told the run";
}

}  // namespace SCE::Tests
