// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The C++ trace port's recorder and the writer of the run trace format
// (docs/adr/0115, D4 and D8; core/TracePort.h), held to the cases of the Rust
// runtime's `trace_port` module and the C11 runtime's `c11_unit_trace_port`, so
// the lanes write one format in one layout: a lane's run of a document differs
// from another lane's, line for line, only where the runs do.

#include "core/TracePort.h"

#include <gtest/gtest.h>
#include <optional>
#include <string>
#include <type_traits>
#include <vector>

namespace SCE::Tests {
namespace {

using ::SCE::Core::RunTrace;
using ::SCE::Core::TraceFired;
using ::SCE::Core::TraceRecorder;
using ::SCE::Core::TraceStep;
using ::SCE::Core::TraceStepKind;

/// One step of each kind with every field filled, so what a kind writes is
/// what it says and not what happens to be empty.
TraceStep filledStep(TraceStepKind kind) {
    TraceStep told;
    told.kind = kind;
    told.event = std::string("go.on");
    told.fired = {TraceFired{"s0", 2}};
    told.exited = {"s01", "s0"};
    told.entered = {"s1"};
    told.configuration = {"s1"};
    told.outcome = "pass";
    return told;
}

std::vector<std::string> names(const std::vector<const char *> &list) {
    return std::vector<std::string>(list.begin(), list.end());
}

/// A policy written by hand declares no `TRACE_PORT`; one that declares it
/// says whether its machine tells a port.
struct NoSwitchPolicy {};

struct TracedPolicy {
    static constexpr bool TRACE_PORT = true;
};

struct UntracedPolicy {
    static constexpr bool TRACE_PORT = false;
};

}  // namespace

// The switch an engine reads, and what it holds for it: an engine whose machine
// tells no port holds an empty base, which costs it no byte.
TEST(TracePortTest, APolicyThatDeclaresNoSwitchTellsNoPortAndTheEngineHoldsNothingForIt) {
    static_assert(!::SCE::Core::TracePortTrait<NoSwitchPolicy>::value, "a hand-written policy tells no port");
    static_assert(!::SCE::Core::TracePortTrait<UntracedPolicy>::value, "a policy that is off tells no port");
    static_assert(::SCE::Core::TracePortTrait<TracedPolicy>::value, "a policy that is on tells its port");
    static_assert(std::is_empty_v<::SCE::Core::TracePortSlot<false>>, "an engine that tells no port holds nothing");
    static_assert(!std::is_empty_v<::SCE::Core::TracePortSlot<true>>, "an engine that tells a port holds a recorder");
    SUCCEED();
}

TEST(TracePortTest, EachKindWritesTheFieldsTheFormatAsksOfIt) {
    EXPECT_EQ(filledStep(TraceStepKind::Start).toJson(),
              R"({"kind": "start", "entered": ["s1"], "configuration": ["s1"]})");
    EXPECT_EQ(
        filledStep(TraceStepKind::Eventless).toJson(),
        R"({"kind": "eventless", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]})");
    EXPECT_EQ(
        filledStep(TraceStepKind::Internal).toJson(),
        R"({"kind": "internal", "event": "go.on", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]})");
    EXPECT_EQ(
        filledStep(TraceStepKind::External).toJson(),
        R"({"kind": "external", "event": "go.on", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]})");
    EXPECT_EQ(filledStep(TraceStepKind::Stop).toJson(),
              R"({"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"})");

    TraceStep unended = filledStep(TraceStepKind::Stop);
    unended.outcome = nullptr;
    EXPECT_EQ(unended.toJson(), R"({"kind": "stop", "exited": ["s01", "s0"], "outcome": null})");
}

TEST(TracePortTest, ATraceIsWrittenWithItsVersionAndLaneOneStepToALine) {
    RunTrace trace;
    EXPECT_EQ(trace.toJson(), "{\n  \"version\": 1,\n  \"lane\": \"cpp\",\n  \"steps\": []\n}\n");
    trace.push(filledStep(TraceStepKind::Start));
    trace.push(filledStep(TraceStepKind::Stop));
    EXPECT_EQ(trace.toJson(), "{\n  \"version\": 1,\n  \"lane\": \"cpp\",\n  \"steps\": [\n    "
                              "{\"kind\": \"start\", \"entered\": [\"s1\"], \"configuration\": [\"s1\"]},\n    "
                              "{\"kind\": \"stop\", \"exited\": [\"s01\", \"s0\"], \"outcome\": \"pass\"}\n  ]\n}\n");
    EXPECT_EQ(trace.steps().size(), 2u);
}

TEST(TracePortTest, ANameIsWrittenAsAJsonString) {
    TraceStep quoted = filledStep(TraceStepKind::Start);
    quoted.entered = {"a\"b\\c"};
    quoted.configuration.clear();
    EXPECT_EQ(quoted.toJson(), R"({"kind": "start", "entered": ["a\"b\\c"], "configuration": []})");
}

// A recorder tells its port each step once the step ends, with what was added
// between, and the configuration only of a step that leaves one.
TEST(TracePortTest, ARecorderTellsItsPortEachStepOnceItEnds) {
    std::vector<TraceStep> told;
    TraceRecorder recorder;
    recorder.set([&told](const TraceStep &step) { told.push_back(step); });

    recorder.begin(TraceStepKind::Internal, std::string("e"));
    EXPECT_TRUE(recorder.isOpen());
    recorder.fired("s0", 1);
    recorder.exited("s0");
    recorder.entered("s1");
    EXPECT_TRUE(told.empty()) << "told only once it ends";
    recorder.end({"s1"}, "ignored");
    EXPECT_FALSE(recorder.isOpen());

    recorder.begin(TraceStepKind::Stop);
    recorder.exited("s1");
    recorder.end({"ignored"}, "pass");

    ASSERT_EQ(told.size(), 2u);
    EXPECT_EQ(told[0].kind, TraceStepKind::Internal);
    EXPECT_EQ(told[0].event, std::optional<std::string>("e"));
    ASSERT_EQ(told[0].fired.size(), 1u);
    EXPECT_EQ(told[0].fired[0], (TraceFired{"s0", 1}));
    EXPECT_EQ(names(told[0].exited), std::vector<std::string>{"s0"});
    EXPECT_EQ(names(told[0].entered), std::vector<std::string>{"s1"});
    EXPECT_EQ(names(told[0].configuration), std::vector<std::string>{"s1"});
    EXPECT_EQ(told[0].outcome, nullptr);

    EXPECT_EQ(told[1].kind, TraceStepKind::Stop);
    EXPECT_EQ(told[1].event, std::nullopt);
    EXPECT_TRUE(told[1].fired.empty());
    EXPECT_EQ(names(told[1].exited), std::vector<std::string>{"s1"});
    EXPECT_TRUE(told[1].entered.empty());
    EXPECT_TRUE(told[1].configuration.empty());
    ASSERT_NE(told[1].outcome, nullptr);
    EXPECT_EQ(std::string(told[1].outcome), "pass");
}

// With no port set nothing is opened, and what the engine adds is kept nowhere;
// a cleared port is told nothing more.
TEST(TracePortTest, ARecorderWithNoPortKeepsNothing) {
    TraceRecorder recorder;
    EXPECT_FALSE(recorder.hasPort());
    recorder.begin(TraceStepKind::Start);
    EXPECT_FALSE(recorder.isOpen());
    recorder.entered("s0");
    recorder.end({"s0"}, nullptr);

    int told = 0;
    recorder.set([&told](const TraceStep &) { ++told; });
    EXPECT_TRUE(recorder.hasPort());
    recorder.begin(TraceStepKind::Start);
    recorder.clear();
    EXPECT_FALSE(recorder.hasPort());
    EXPECT_FALSE(recorder.isOpen());
    recorder.end({"s0"}, nullptr);
    EXPECT_EQ(told, 0);
}

// A step opened in place of one left open replaces it: what the first was told
// is no part of the second.
TEST(TracePortTest, AStepOpenedInPlaceOfAnOpenOneStartsEmpty) {
    std::vector<TraceStep> told;
    TraceRecorder recorder;
    recorder.set([&told](const TraceStep &step) { told.push_back(step); });
    recorder.begin(TraceStepKind::Eventless);
    recorder.exited("s0");
    recorder.begin(TraceStepKind::External, std::string("request.new"));
    recorder.entered("s2");
    recorder.end({"s2"}, nullptr);

    ASSERT_EQ(told.size(), 1u);
    EXPECT_EQ(told[0].kind, TraceStepKind::External);
    EXPECT_EQ(told[0].event, std::optional<std::string>("request.new"));
    EXPECT_TRUE(told[0].exited.empty());
    EXPECT_EQ(names(told[0].entered), std::vector<std::string>{"s2"});
}

}  // namespace SCE::Tests
