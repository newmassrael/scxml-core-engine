// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Each scenario of the properties judge's turnstile, replayed by the C++ lane
// (docs/adr/0115, D15 and the converse of D8). A scenario is the run TLC gave of
// a violation or a witness, with the events its environment put on the external
// queue; this program gives the machine those events, on a manual clock, with its
// trace port on, and holds the run the port is told to the recorded run
// `<name>.cpp.json` beside the scenario, byte for byte.
// `sce_build::formal::replay` judges each recorded run to be the scenario's, step
// for step (sce-build/tests/a_scenario_is_the_run_of_every_lane.rs), as it judges
// the Rust and the C11 lanes'. The twin of the Rust lane's
// `a_scenario_of_the_turnstile_is_its_run` and of the C11 lane's
// `c11_a_scenario_of_the_turnstile_is_its_run`.
//
// The harness gives an input once the machine has taken every step it can, which
// is after the step the input names unless the model's environment put it on the
// queue in the middle of a macrostep. A macrostep that also sends to its own
// external queue after that point would take the two in another order here; the
// judge then finds the runs part, which is a defect of this harness to settle,
// and not a verdict about the document. A delayed send comes due by the clock
// moving to the next deadline, which is the earliest: a scenario in which the
// model's time makes a later one due first is one no clock replays, and the judge
// finds that too.
//
// The scenarios are written by `sce-codegen formal properties --scenarios
// sce-build/tests/fixtures/formal_properties/scenarios`. Recording the lane's
// runs again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 ctest -R cpp_a_scenario_of_the_turnstile_is_its_run
//
// Fixture: sce-build/tests/fixtures/formal_properties/turnstile.scxml

#include "common/SceClock.h"
#include "core/TracePort.h"
#include "scripting/ScriptEngineProvider.h"
#include "turnstile_sm.h"

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <iterator>
#include <memory>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#ifndef SCE_FORMAL_SCENARIOS_DIR
#error "SCE_FORMAL_SCENARIOS_DIR names sce-build/tests/fixtures/formal_properties/scenarios; the build defines it"
#endif

namespace SCE::Tests {
namespace {

using Turnstile = SCE::Generated::turnstile::turnstile;

static_assert(Turnstile::HAS_TRACE_PORT, "this program is built with SCE_TRACE_PORT=1 (tests/w3c/W3CPrograms.cmake)");

/// What a scenario's file is named by after its name (`sce_build::formal::replay::SCENARIO_SUFFIX`).
constexpr const char *SCENARIO_SUFFIX = ".scenario.json";

/// Where an event the environment put on the external queue came from.
enum class InputKind {
    /// It came from outside.
    Arrive,
    /// A delayed send the machine made came due.
    Due,
};

/// One event the environment put on the external queue during a scenario.
struct Input {
    InputKind kind = InputKind::Arrive;
    /// How many steps of the run were over when it came, from 1.
    std::size_t after = 0;
    std::string event;
};

/// Each scenario in `dir`, by its name, in the order of the names.
std::vector<std::pair<std::string, std::filesystem::path>> scenarios(const std::filesystem::path &dir) {
    std::vector<std::pair<std::string, std::filesystem::path>> found;
    const std::string suffix = SCENARIO_SUFFIX;
    for (const auto &entry : std::filesystem::directory_iterator(dir)) {
        const std::string file = entry.path().filename().string();
        if (file.size() > suffix.size() && file.compare(file.size() - suffix.size(), suffix.size(), suffix) == 0) {
            found.emplace_back(file.substr(0, file.size() - suffix.size()), entry.path());
        }
    }
    std::sort(found.begin(), found.end());
    return found;
}

/// The whole of the file at `path`, or nothing when it cannot be read.
std::optional<std::string> readFile(const std::filesystem::path &path) {
    std::ifstream in(path, std::ios::binary);
    if (!in.is_open()) {
        return std::nullopt;
    }
    return std::string((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
}

/// The inputs of the scenario `text` is, which `sce_build::run_trace` reads of reference: every
/// other member of the document is read there and stepped over here. An input with a member the
/// form does not have, or without one it has, is refused rather than replayed in part: nothing,
/// and `why` says what is wrong.
std::optional<std::vector<Input>> readInputs(const std::string &text, std::string &why) {
    const nlohmann::json document = nlohmann::json::parse(text, nullptr, false);
    if (document.is_discarded() || !document.is_object()) {
        why = "it is no JSON object";
        return std::nullopt;
    }
    std::vector<Input> inputs;
    const auto listed = document.find("inputs");
    if (listed == document.end()) {
        return inputs;
    }
    if (!listed->is_array()) {
        why = "`inputs` is no array";
        return std::nullopt;
    }
    for (const nlohmann::json &input : *listed) {
        const std::string at = "inputs[" + std::to_string(inputs.size()) + "]";
        if (!input.is_object() || input.size() != 3 || !input.contains("kind") || !input.contains("after") ||
            !input.contains("event")) {
            why = at + " is not an object of a `kind`, an `after` and an `event`";
            return std::nullopt;
        }
        const nlohmann::json &kind = input.at("kind");
        const nlohmann::json &after = input.at("after");
        const nlohmann::json &event = input.at("event");
        if (!kind.is_string() || !after.is_number_unsigned() || !event.is_string()) {
            why = at + " has a member of another type than the form's";
            return std::nullopt;
        }
        Input read;
        const std::string named = kind.get<std::string>();
        if (named == "arrive") {
            read.kind = InputKind::Arrive;
        } else if (named == "due") {
            read.kind = InputKind::Due;
        } else {
            why = at + " is of the kind '" + named + "', which the form does not have";
            return std::nullopt;
        }
        read.after = after.get<std::size_t>();
        read.event = event.get<std::string>();
        inputs.push_back(std::move(read));
    }
    return inputs;
}

/// Give the turnstile `inputs`, on a manual clock, and write the run its trace port is told into
/// `text`, in the run trace format.
void replay(const std::string &name, const std::vector<Input> &inputs, std::string &text) {
    // Before the machine, so a port the machine tells as it goes away still has its trace.
    ::SCE::Core::RunTrace trace;
    Turnstile sm;
    if constexpr (Turnstile::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter: the engine's lifetime is the
        // ScriptEngineProvider singleton's, and the shared_ptr is a view of it.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    // Before the start: the entry configuration's `<onentry>` may arm a delayed send.
    sm.setClock(std::make_shared<::SCE::ManualClock>(0));
    sm.setTracePort([&trace](const ::SCE::Core::TraceStep &step) { trace.push(step); });
    ASSERT_EQ(sm.initialize(), std::nullopt) << name << ": the machine did not start";
    for (const Input &input : inputs) {
        const std::size_t told = trace.steps().size();
        ASSERT_GE(told, input.after) << name << ": the machine took " << told << " steps, and the model " << input.after
                                     << " before `" << input.event << "` came";
        if (input.kind == InputKind::Arrive) {
            sm.raiseExternal(input.event);
            sm.tick();
        } else {
            const auto wait = sm.timeUntilNextScheduled();
            ASSERT_TRUE(wait.has_value())
                << name << ": `" << input.event << "` comes due, and the machine holds no delayed send";
            sm.advanceTimeMs(static_cast<std::uint64_t>(wait->count()));
        }
    }
    text = trace.toJson();
}

/// Hold `text` to the recorded run at `path`, or record it again when `UPDATE_EXPECT` is set.
void holdsToTheRecordedRun(const std::filesystem::path &path, const std::string &text) {
    if (std::getenv("UPDATE_EXPECT") != nullptr) {
        std::ofstream out(path, std::ios::binary | std::ios::trunc);
        out << text;
        out.close();
        ASSERT_TRUE(out.good()) << path.string() << " was not written whole";
        return;
    }
    const std::optional<std::string> recorded = readFile(path);
    ASSERT_TRUE(recorded.has_value()) << path.string()
                                      << " cannot be read: record it with 'UPDATE_EXPECT=1 ctest -R "
                                         "cpp_a_scenario_of_the_turnstile_is_its_run'";
    EXPECT_EQ(*recorded, text) << "the run of " << path.string() << " is not the recorded one the replay judge reads";
}

}  // namespace

TEST(AScenarioOfTheTurnstileIsItsRun, EachScenarioOfTheTurnstileIsItsRunOnTheCppLane) {
    const std::filesystem::path dir = SCE_FORMAL_SCENARIOS_DIR;
    ASSERT_TRUE(std::filesystem::is_directory(dir))
        << dir.string() << " is no directory: write the scenarios with 'sce-codegen formal properties --scenarios'";
    const auto found = scenarios(dir);
    ASSERT_FALSE(found.empty()) << dir.string()
                                << " holds no scenario: write them with 'sce-codegen formal properties --scenarios'";
    for (const auto &[name, path] : found) {
        SCOPED_TRACE(name);
        const std::optional<std::string> text = readFile(path);
        ASSERT_TRUE(text.has_value()) << path.string() << " cannot be read";
        std::string why;
        const std::optional<std::vector<Input>> inputs = readInputs(*text, why);
        ASSERT_TRUE(inputs.has_value()) << "the inputs of " << path.string() << " are not read: " << why;
        std::string run;
        replay(name, *inputs, run);
        if (HasFatalFailure()) {
            return;
        }
        holdsToTheRecordedRun(dir / (name + ".cpp.json"), run);
        if (HasFatalFailure()) {
            return;
        }
    }
}

}  // namespace SCE::Tests
