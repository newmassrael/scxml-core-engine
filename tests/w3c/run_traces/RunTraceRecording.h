// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of the C++ lane, recorded as the trace `sce-codegen formal traces`
// judges a behaviour of the document's model (docs/adr/0115, D4 and D8). The
// twin of the Rust lane's `a_run_is_a_behaviour_of_its_model`
// (backends/rust/tests) and the C11 lane's `run_trace_recording.h`
// (backends/c/tests/integration).
//
// A program builds one W3C document that `docs/formal` has a model of with its
// machine's trace port on (`SCE_TRACE_PORT=1`, tests/w3c/W3CPrograms.cmake),
// keeps each step the port is told, runs the machine to its end as the
// conformance runner does (`SimpleAotTest`), and holds what was told to the
// recorded run `docs/formal/traces.json` declares of that model, byte for byte.
// The judge reads the recorded file, so a run that moves is seen here first, and
// a recorded run that no longer is the lane's is never judged in its place. A
// C++ run of a document is judged against the same generated model as the Rust
// run of it, and these documents leave a run's steps no choice, so the two
// recorded runs differ only in their lane, or the judge refuses one.
//
// Recording again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 ctest -R cpp_a_run_of_test_
// then `sce-codegen formal traces --manifest docs/formal/traces.json`, and read
// the diff of the recorded runs.

#pragma once

#include "core/TracePort.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdlib>
#include <fstream>
#include <gtest/gtest.h>
#include <iterator>
#include <memory>
#include <optional>
#include <string>

#ifndef SCE_FORMAL_TRACES_DIR
#error "SCE_FORMAL_TRACES_DIR names docs/formal/traces; the build defines it"
#endif

namespace SCE::Tests::RunTraceRecording {

/// Run `SM` to its end with its trace port set, and hold the run it told to the
/// recorded run of `model` (`docs/formal/traces/<model>.cpp.json`), or record it
/// again when `UPDATE_EXPECT` is set.
template <typename SM> void holdTheRunToTheRecordedRunOf(const std::string &model) {
    static_assert(SM::HAS_TRACE_PORT, "the program is built with SCE_TRACE_PORT=1 (tests/w3c/W3CPrograms.cmake)");

    ::SCE::Core::RunTrace trace;
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter: the engine's lifetime is the
        // ScriptEngineProvider singleton's, and the shared_ptr is a view of it.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.setTracePort([&trace](const ::SCE::Core::TraceStep &step) { trace.push(step); });
    ASSERT_EQ(sm.initialize(), std::nullopt) << model << ": the traced machine did not start";
    ASSERT_EQ(sm.terminalState(), SM::State::Pass) << model << ": the traced run did not end in `pass`";

    // A start first, and a stop last that exits the state the run ended in and
    // names it: what every recorded run holds.
    const auto &steps = trace.steps();
    ASSERT_FALSE(steps.empty()) << model << ": the port was told no step";
    EXPECT_EQ(steps.front().kind, ::SCE::Core::TraceStepKind::Start)
        << model << ": the run does not begin with a start";
    const ::SCE::Core::TraceStep &stop = steps.back();
    EXPECT_EQ(stop.kind, ::SCE::Core::TraceStepKind::Stop) << model << ": the run does not end with a stop";
    ASSERT_EQ(stop.exited.size(), 1u) << model << ": the stop does not exit one state";
    EXPECT_EQ(std::string(stop.exited.front()), "pass") << model << ": the stop does not exit `pass`";
    ASSERT_NE(stop.outcome, nullptr) << model << ": the stop names no outcome";
    EXPECT_EQ(std::string(stop.outcome), "pass") << model << ": the stop names another outcome than `pass`";

    const std::string text = trace.toJson();
    const std::string path = std::string(SCE_FORMAL_TRACES_DIR) + "/" + model + ".cpp.json";
    if (std::getenv("UPDATE_EXPECT") != nullptr) {
        std::ofstream out(path, std::ios::binary | std::ios::trunc);
        out << text;
        out.close();
        ASSERT_TRUE(out.good()) << path << " was not written whole";
        return;
    }
    std::ifstream in(path, std::ios::binary);
    ASSERT_TRUE(in.is_open()) << path
                              << " cannot be read: record it with 'UPDATE_EXPECT=1 ctest -R cpp_a_run_of_test_'";
    const std::string recorded((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
    EXPECT_EQ(recorded, text) << "the run of " << model
                              << " is not the recorded one that 'sce-codegen formal traces' judges";
}

}  // namespace SCE::Tests::RunTraceRecording
