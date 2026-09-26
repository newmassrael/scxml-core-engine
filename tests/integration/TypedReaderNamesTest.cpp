// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3: the typed_reader_names document, run by the C++ Interpreter.
//
// The Interpreter emits no typed readers — those are what the AOT channels
// name, and what the fixture was written about. What it shares with them is
// the data model the readers read: `box`, `object`, `pass`, `screen-rules` and
// `a_b` are declared and initialised, and `bump` adds 10 to the four an
// expression can name. So this driver reads each variable by name, the key the
// engine holds it under — `screen-rules` included, a name no expression can
// spell and the one the C11 channel once failed to create at all — and pins
// the same values the AOT drivers pin through their readers.
//
// Fixture: integration_resources/typed_reader_names/typed_reader_names.scxml
// (canonical, shared with the C++ AOT / C11 / Go / Kotlin / Python / Rust
// channels).

#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <fstream>
#include <gtest/gtest.h>
#include <sstream>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class TypedReaderNamesTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture =
            std::string(SCE_PROJECT_ROOT) + "/integration_resources/typed_reader_names/typed_reader_names.scxml";
        std::ifstream in(fixture);
        ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
        std::ostringstream buffer;
        buffer << in.rdbuf();

        sm_ = std::make_shared<StateMachine>(*engine_);
        raiser_ = std::make_shared<EventRaiserImpl>();
        sm_->setEventRaiser(raiser_);
        ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
        ASSERT_TRUE(sm_->start());
        ASSERT_EQ(sm_->getCurrentState(), "idle");
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// The variable `name` holds, read by the key the engine stores it under.
    std::string value(const std::string &name) {
        auto result = engine_->getVariable(sm_->getSessionId(), name).get();
        EXPECT_TRUE(result.isSuccess()) << "the fixture declares `" << name << "` in its datamodel";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<EventRaiserImpl> raiser_;
    std::shared_ptr<StateMachine> sm_;
};

// Each variable holds the value it was declared with.
TEST_F(TypedReaderNamesTest, EachVariableHoldsItsDeclaredValue) {
    EXPECT_EQ(value("box"), "1");
    EXPECT_EQ(value("object"), "2");
    EXPECT_EQ(value("pass"), "3");
    EXPECT_EQ(value("screen-rules"), "4");
    EXPECT_EQ(value("a_b"), "10");
}

// And the value it holds now: `bump` adds 10 to four of them, and leaves
// `screen-rules` — which no ECMAScript expression can name — alone.
TEST_F(TypedReaderNamesTest, BumpChangesTheFourAnExpressionCanName) {
    sm_->processEvent("bump");
    EXPECT_EQ(value("box"), "11");
    EXPECT_EQ(value("object"), "12");
    EXPECT_EQ(value("pass"), "13");
    EXPECT_EQ(value("screen-rules"), "4");
    EXPECT_EQ(value("a_b"), "20");
}

}  // namespace Tests
}  // namespace SCE
