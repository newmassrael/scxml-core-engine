// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1 donedata surfacing — Interpreter local-invoke path.
//
// Closes the W3C IRP coverage gap: no public SCXML IRP test exercises
// `<donedata>` on the invoked child's top-level `<final>` combined with
// `done.invoke.<id>._event.data` readback on the parent. The mesh
// `test_mesh_session_f_donedata` covers the same contract for the AOT
// wire-18 path; this fixture mirrors it for the Interpreter code path
// through `SCXMLInvokeHandler`'s completion callback.
//
// The parent sequentially invokes two workers. Each worker reaches a
// top-level `<final>` carrying `<donedata>`:
//
//   inv_param   — `<donedata><param name="result" expr="42"/></donedata>`.
//                 The cond `_event.data.result === 42` proves the param
//                 envelope survived the done.invoke emission and the
//                 typed value re-hydrated after JSON round-trip.
//
//   inv_content — `<donedata><content expr="'hello_content'"/></donedata>`.
//                 The cond `_event.data === 'hello_content'` proves the
//                 content-expression branch serialises through
//                 `DoneDataHelper::evaluateContent` and re-parses into a
//                 string on the parent side.
//
// Any fall-through `done.invoke.<id>` takes the deterministic `fail`
// transition so a regression surfaces as an explicit state mismatch
// rather than a 5s timeout.

#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <thread>

namespace SCE {
namespace Tests {

class DonedataLocalInvokeTest : public ::testing::Test {
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

// Parent + two inline workers. Uses ecmascript data model so that the
// W3C SCXML B.2 JSON round-trip on `_event.data` yields structurally
// identical values on both sides (number stays number, string stays
// string) without the Lua transformer's index-shift concerns.
static const char *kParentScxml = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript" initial="phase_param">
    <datamodel>
        <data id="param_ok" expr="false"/>
    </datamodel>
    <state id="phase_param">
        <invoke type="scxml" id="inv_param">
            <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml"
                       version="1.0" datamodel="ecmascript" initial="done">
                    <final id="done">
                        <donedata>
                            <param name="result" expr="42"/>
                        </donedata>
                    </final>
                </scxml>
            </content>
        </invoke>
        <transition event="done.invoke.inv_param"
                    cond="_event.data &amp;&amp; _event.data.result === 42"
                    target="phase_content">
            <assign location="param_ok" expr="true"/>
        </transition>
        <transition event="done.invoke.inv_param" target="fail"/>
        <transition event="error.execution" target="fail"/>
    </state>
    <state id="phase_content">
        <invoke type="scxml" id="inv_content">
            <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml"
                       version="1.0" datamodel="ecmascript" initial="done">
                    <final id="done">
                        <donedata>
                            <content expr="'hello_content'"/>
                        </donedata>
                    </final>
                </scxml>
            </content>
        </invoke>
        <transition event="done.invoke.inv_content"
                    cond="param_ok &amp;&amp; _event.data === 'hello_content'"
                    target="pass"/>
        <transition event="done.invoke.inv_content" target="fail"/>
        <transition event="error.execution" target="fail"/>
    </state>
    <final id="pass"/>
    <final id="fail"/>
</scxml>)";

TEST_F(DonedataLocalInvokeTest, ParentObservesDonedataOnDoneInvoke) {
    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    ASSERT_TRUE(sm->loadSCXMLFromString(kParentScxml));
    ASSERT_TRUE(sm->start());

    // W3C SCXML 3.13: reaching a top-level `<final>` halts processing —
    // `isRunning()` flips to false and the current state name freezes on
    // `pass` or `fail`. Poll `isRunning()` instead of `isInFinalState()`
    // (the latter short-circuits on the halted flag and returns false
    // after halt, see StateMachine.cpp).
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — done.invoke was not emitted or "
                                  << "the parent is stuck in phase_param/phase_content";

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "parent reached `fail`: `_event.data.result === 42` (param branch) or "
        << "`_event.data === 'hello_content'` (content branch) failed. Empty `_event.data` "
        << "means the Interpreter's InvokeExecutor completion callback dropped the "
        << "child's donedata — mirror AOT's `stashDonedataAtFinal` in `StateMachine` and "
        << "thread it through the `done.invoke` raise in `InvokeExecutor.cpp`.";
}

}  // namespace Tests
}  // namespace SCE
