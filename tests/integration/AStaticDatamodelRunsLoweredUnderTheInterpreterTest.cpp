// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15), lowered to the
// ecmascript document the Interpreter runs, and run.
//
// The Interpreter refuses a `sce-static` document as written
// (AStaticDatamodelRunsUnderTheInterpreterTest): its script engine holds no
// types and checks no integer operation, so it would run the document with a
// meaning the document did not give it. `sce-codegen lower` is the other
// answer — it replaces the document's expressions with ECMAScript that means
// what the typed document means, and every other byte is the author's.
//
// What this holds it to is the one thing that makes the lowering worth having:
// the Interpreter, run on the lowered document, does what the generated
// backends do. The Kotlin and Rust backends replay
// `sce-build/tests/fixtures/static_datamodel/scenarios/*.json` — each a list
// of steps, an external event and what the machine must hold after it — and
// this replays the same files. One oracle judges three engines, so a
// disagreement is a defect in one of them and not a difference in what each
// was asked.
//
// Nothing here is listed. The scenarios are found by scanning the directory,
// the machine each one drives by the `machine` it names, and whether the
// Interpreter can run it by asking `sce-codegen lower`: a machine whose
// constructs have no lowering yet is refused with a typed diagnostic naming
// the construct, and starts being replayed the day the lowering covers it.

#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <nlohmann/json.hpp>
#include <sstream>
#include <string>
#include <unistd.h>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

const std::filesystem::path kFixtures =
    std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build/tests/fixtures/static_datamodel";

std::string slurp(const std::filesystem::path &path) {
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    return buffer.str();
}

/// What a command wrote to the stream it was asked to keep, and how it ended.
struct Ran {
    int status = -1;
    std::string output;
};

Ran capture(const std::string &command) {
    Ran ran;
    FILE *pipe = popen(command.c_str(), "r");
    if (pipe == nullptr) {
        return ran;
    }
    char buffer[4096];
    while (std::fgets(buffer, sizeof(buffer), pipe) != nullptr) {
        ran.output += buffer;
    }
    ran.status = pclose(pipe);
    return ran;
}

/// `sce-codegen lower <fixture>`: the lowered document on stdout when it
/// succeeds; when it refuses, the diagnostic, from the stream diagnostics
/// travel on.
struct Lowered {
    bool ok = false;
    std::string document;
    nlohmann::json refusal;
};

Lowered lower(const std::filesystem::path &fixture) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    EXPECT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    Lowered lowered;
    if (bin == nullptr) {
        return lowered;
    }
    const std::string base = std::string("\"") + bin + "\" lower \"" + fixture.string() + "\" --error-format=json";
    const Ran ran = capture(base + " 2>/dev/null");
    if (ran.status == 0) {
        lowered.ok = true;
        lowered.document = ran.output;
        return lowered;
    }
    // The diagnostic is one NDJSON record on stderr; stdout is discarded so
    // that what is read here is what the refusal said.
    const Ran refused = capture(base + " 2>&1 >/dev/null");
    const auto eol = refused.output.find('\n');
    lowered.refusal = nlohmann::json::parse(refused.output.substr(0, eol), nullptr, false);
    return lowered;
}

/// Whether a value the script engine holds is the scenario's. Both are read
/// as JSON, so `253` and `253.0`, or `true` and `true`, are the same value.
bool holds(const nlohmann::json &got, const nlohmann::json &want) {
    return got == want;
}

}  // namespace

class AStaticDatamodelRunsLoweredUnderTheInterpreterTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A variable of the running machine, as the JSON the script engine
    /// writes for it. Undefined (`JSON.stringify` of nothing) reads as null.
    nlohmann::json variable(const StateMachine &machine, const std::string &name) {
        auto result = engine_->evaluateExpression(machine.getSessionId(), "JSON.stringify(" + name + ")").get();
        if (!result.isSuccess()) {
            ADD_FAILURE() << "reading `" << name << "`: " << result.getErrorMessage();
            return nullptr;
        }
        const auto text = result.getValue<std::string>();
        return text.empty() ? nlohmann::json(nullptr) : nlohmann::json::parse(text, nullptr, false);
    }

    /// Replay `scenario` against `document`, the machine the scenario names
    /// once lowered.
    void replay(const std::string &document, const nlohmann::json &scenario) {
        const auto machine = std::make_shared<StateMachine>(*engine_);
        ASSERT_TRUE(machine->loadSCXMLFromString(document)) << "the Interpreter does not load the lowered document";
        ASSERT_TRUE(machine->start());

        const auto &steps = scenario.at("steps");
        ASSERT_FALSE(steps.empty()) << "a scenario with no steps judges nothing";
        for (std::size_t n = 0; n < steps.size(); ++n) {
            const auto &step = steps[n];
            const std::string note = step.value("note", std::string{});
            SCOPED_TRACE("step " + std::to_string(n) + (note.empty() ? "" : " (" + note + ")"));
            if (step.contains("event")) {
                const std::string data = step.contains("data") ? step.at("data").dump() : std::string{};
                machine->processEvent(step.at("event").get<std::string>(), data);
            }
            const auto &expect = step.at("expect");
            // A machine that ended in a top-level <final> has no saved state
            // to read on the generated backends — the save refuses one — so
            // what a scenario says of it is that it ended, and that is the
            // whole of the step.
            if (expect.value("ended", false)) {
                EXPECT_FALSE(expect.contains("state") || expect.contains("variables"))
                    << "an ended machine has no state or variables to read";
                EXPECT_TRUE(machine->isInFinalState()) << "the machine ended in a top-level <final>";
                continue;
            }
            if (expect.contains("state")) {
                EXPECT_EQ(expect.at("state").get<std::string>(), machine->getCurrentState()) << "the current state";
            }
            if (expect.contains("variables")) {
                for (const auto &[name, want] : expect.at("variables").items()) {
                    const auto got = variable(*machine, name);
                    EXPECT_TRUE(holds(got, want))
                        << "variable `" << name << "` is " << got.dump() << ", not " << want.dump();
                }
            }
        }
    }

    IScriptEngine *engine_ = nullptr;
};

// Every scenario the generated backends replay, on the machine it drives,
// lowered and run by the Interpreter.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, TheInterpreterDoesWhatTheGeneratedBackendsDo) {
    std::vector<std::string> replayed;
    std::vector<std::string> notYetLowered;
    for (const auto &entry : std::filesystem::directory_iterator(kFixtures / "scenarios")) {
        if (entry.path().extension() != ".json") {
            continue;
        }
        const std::string name = entry.path().stem().string();
        SCOPED_TRACE(name);
        const auto scenario = nlohmann::json::parse(slurp(entry.path()), nullptr, false);
        ASSERT_FALSE(scenario.is_discarded()) << "a scenario is JSON";
        const std::string machine = scenario.at("machine").get<std::string>();

        const Lowered lowered = lower(kFixtures / (machine + ".scxml"));
        if (!lowered.ok) {
            // A refusal is an answer, and it must be the typed one: a crash
            // or a bare failure would read the same as "not yet lowered".
            ASSERT_TRUE(lowered.refusal.is_object()) << "`sce-codegen lower` failed with no diagnostic";
            EXPECT_EQ("generate/unsupported-feature", lowered.refusal.value("code", std::string{}))
                << lowered.refusal.dump();
            notYetLowered.push_back(name);
            continue;
        }
        replay(lowered.document, scenario);
        replayed.push_back(name);
    }

    // Floor: a scan that found nothing to replay would pass, so what it must
    // have replayed is asserted. Every scenario is of a machine whose every
    // construct the lowering covers today, and the floor rises as the lowering
    // grows.
    EXPECT_GE(replayed.size(), 11u) << "replayed " << replayed.size()
                                    << " scenarios, not yet lowered: " << notYetLowered.size();
    for (const auto &name : notYetLowered) {
        RecordProperty("not_yet_lowered_" + name, "refused by sce-codegen lower");
    }
}

// A document under another data model is not the lowering's to touch: what it
// prints for one is the document.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, ADocumentUnderAnotherDataModelIsPrintedAsItIs) {
    const auto dir = std::filesystem::temp_directory_path() / ("sce-lower-ecmascript-" + std::to_string(::getpid()));
    std::filesystem::create_directories(dir);
    const auto path = dir / "plain.scxml";
    const std::string document =
        R"(<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
  <datamodel><data id="level" expr="250"/></datamodel>
  <state id="s"/>
</scxml>)";
    {
        std::ofstream out(path);
        out << document;
    }
    const Lowered lowered = lower(path);
    std::error_code ec;
    std::filesystem::remove_all(dir, ec);
    ASSERT_TRUE(lowered.ok);
    EXPECT_EQ(document, lowered.document);
}

}  // namespace Tests
}  // namespace SCE
