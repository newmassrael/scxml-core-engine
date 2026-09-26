// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A wildcard keeps its own guard and its own type — Interpreter path.
//
// W3C SCXML 3.12.1 lets a `*` descriptor match every event, and that is all
// it changes: W3C SCXML 3.13 still asks the transition's `cond` whether it is
// enabled, and a `type="internal"` transition whose target is a proper
// descendant of its compound source still does not exit that source.
//
// No W3C document writes a wildcard with a `cond` or with `type="internal"`, so
// a generator that moved the wildcard into a hand-written fallback dropped both
// without a suite noticing — the Kotlin one did until 2026-09-13.
//
// Sibling of `WildcardInDocumentOrderAotTest.cpp` (C++ AOT channel).
//
// Fixture: integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml

#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <fstream>
#include <gtest/gtest.h>
#include <sstream>
#include <thread>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class WildcardInDocumentOrderTest : public ::testing::Test {
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

    IScriptEngine *engine_;
};

TEST_F(WildcardInDocumentOrderTest, AWildcardKeepsItsGuardAndItsType) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) + "/integration_resources/wildcard_in_document_order/"
                                                                "wildcard_in_document_order.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    ASSERT_TRUE(sm->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm->start());

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(3);
    while (std::chrono::steady_clock::now() < deadline && sm->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    // Each failure final names the case, so this one assertion says which
    // property of the wildcard was lost:
    //   failGuardIgnored              a wildcard fired with its guard false
    //   failGuardNeverFired           a wildcard did not fire with its guard true
    //   failGuardedInternalReentered  a guarded internal wildcard exited its source
    //   failSealedInternalReentered   an unguarded internal wildcard exited its source
    // A machine resting in `guardedFrom` or `sealedFrom` took no internal
    // wildcard at all.
    EXPECT_EQ(sm->terminalState().value_or(""), "pass")
        << "the machine did not reach pass: a wildcard is enabled only when its guard is true, and an "
           "internal wildcard targeting a descendant of its compound source must not exit and re-enter "
           "that source";
}

}  // namespace Tests
}  // namespace SCE
