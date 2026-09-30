// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) and the
// Interpreter: generated code runs it, and this engine refuses it.
//
// Generated code holds the document's variables to their types and checks its
// integer operations, so an overflowing `uint8` keeps its value and raises
// `error.execution`. The Interpreter hands every expression to its script
// engine, which evaluates it as ECMAScript. Measured 2026-09-30 by replaying
// the fixtures the AOT backends replay, before the parser refused the value:
//
//   * `static_overflow`: `level + 3` from 250 wrote 256 (the AOT machines keep
//     253), `refusals` stayed 0 (they read 1), and a guard over the
//     overflowing sum was true (they are false) — so `probe` left `waiting`.
//   * `sync_client`: `DeleteOutcome`, `Failure`, `RetryAt` and `UploadOutcome`,
//     the algorithms it imports, were undefined names (`ReferenceError`), so
//     the machine left its scenario at the first response.
//
// The Interpreter accepted the document and ran it with a meaning the
// document did not give it, with no diagnostic. That is what this file holds
// closed: a `sce-static` document is refused with a typed diagnostic, never
// run. The refusal is the Interpreter's counterpart of the generated side's
// `generate/unsupported-feature` for a backend that cannot lower the model.

#include "factory/NodeFactory.h"
#include "parsing/Diagnostic.h"
#include "parsing/SCXMLParser.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <nlohmann/json.hpp>
#include <sstream>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

const std::string kStaticFixtures = std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/static_datamodel";

std::string slurp(const std::filesystem::path &path) {
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    return buffer.str();
}

/// Every statechart under the shared fixture directory that declares
/// `datamodel="sce-static"`, derived from what the files say rather than from a
/// list a new fixture would have to be added to.
std::vector<std::filesystem::path> staticDatamodelStatecharts() {
    std::vector<std::filesystem::path> found;
    for (const auto &entry : std::filesystem::directory_iterator(kStaticFixtures)) {
        if (entry.path().extension() != ".scxml") {
            continue;
        }
        if (slurp(entry.path()).find("datamodel=\"sce-static\"") != std::string::npos) {
            found.push_back(entry.path());
        }
    }
    return found;
}

}  // namespace

class AStaticDatamodelRunsUnderTheInterpreterTest : public ::testing::Test {
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

    IScriptEngine *engine_ = nullptr;
};

// Refused, and refused with the diagnostic a repair consumer reads — for every
// such document in the shared fixture set, through the entry point the
// Interpreter itself uses.
TEST_F(AStaticDatamodelRunsUnderTheInterpreterTest, TheInterpreterRefusesWhatGeneratedCodeRuns) {
    const auto fixtures = staticDatamodelStatecharts();
    // Floor: a scan that stopped finding the fixtures would refuse nothing and
    // pass, so the count is asserted, not implied.
    ASSERT_GE(fixtures.size(), 5u) << "found " << fixtures.size() << " sce-static statecharts under " << kStaticFixtures
                                   << " — the scan is broken, not the refusal";

    for (const auto &fixture : fixtures) {
        SCOPED_TRACE(fixture.filename().string());
        const auto document = slurp(fixture);

        SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
        EXPECT_EQ(parser.parseContent(document), nullptr) << "the parser must not build a model it cannot run";
        ASSERT_FALSE(parser.getDiagnostics().empty()) << "a refusal with no typed diagnostic is a silent one";
        const auto record = parser.getDiagnostics().front()->to_json();
        EXPECT_EQ(record.value("code", std::string{}), "scxml/unsupported-datamodel");
        EXPECT_EQ(record.value("actual", std::string{}), "sce-static");

        const auto machine = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
        EXPECT_FALSE(machine->loadSCXMLFromString(document)) << "and the runtime entry point must not start it";
    }
}

// The refusal is about this value, not a blanket: a document that declares the
// data model this engine runs still loads and starts.
TEST_F(AStaticDatamodelRunsUnderTheInterpreterTest, AnEcmascriptDocumentIsStillRun) {
    const std::string document =
        R"(<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
             <datamodel><data id="level" expr="250"/></datamodel>
             <state id="s"/>
           </scxml>)";
    const auto machine = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    ASSERT_TRUE(machine->loadSCXMLFromString(document));
    EXPECT_TRUE(machine->start());
}

}  // namespace Tests
}  // namespace SCE
