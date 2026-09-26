// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitStates: a state leaves the configuration AFTER its
// own `<onexit>` has run — Interpreter path.
//
// The procedure is onexit, then cancelInvoke, then configuration.delete(s),
// for each state in exitOrder. So `In(s)` inside s's own `<onexit>` is true,
// the parent is still active inside the child's `<onexit>`, and the child is
// already gone inside the parent's. The configuration after the microstep is
// the same whatever order an engine used, so the handlers record what they saw
// and those records are the verdict.
//
// Sibling of `OnexitRunsBeforeTheStateLeavesAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <fstream>
#include <gtest/gtest.h>
#include <sstream>
#include <string>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class OnexitRunsBeforeTheStateLeavesTest : public ::testing::Test {
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

TEST_F(OnexitRunsBeforeTheStateLeavesTest, AStateIsStillActiveWhileItsOwnOnexitRuns) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/onexit_runs_before_the_state_leaves/"
                                "onexit_runs_before_the_state_leaves.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
            (void)event;
            try {
                return target->send(event).get().isSuccess;
            } catch (...) {
                return false;
            }
        });
    auto eventRaiser = std::make_shared<EventRaiserImpl>();
    eventRaiser->setScheduler(scheduler);
    eventRaiser->setImmediateMode(false);
    sm->setEventRaiser(eventRaiser);
    sm->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm->start());

    // What the handlers recorded, next to the configuration, so a failure says
    // which observation was wrong rather than only which final it reached.
    const auto describe = [&sm]() {
        std::string out;
        for (const auto &s : sm->getActiveStates()) {
            out += " " + s;
        }
        auto &engine = ScriptEngineProvider::getScriptEngine();
        const auto read = [&](const char *name) {
            auto result = engine.evaluateExpression(sm->getSessionId(), name).get();
            return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
        };
        return out + "  (selfInInner=" + read("selfInInner") + " parentInInner=" + read("parentInInner") +
               " selfInOuter=" + read("selfInOuter") + " childInOuter=" + read("childInOuter") +
               " exits=" + read("exits") + "; wanted 1 / 1 / 1 / 0 / 2)";
    };

    ASSERT_TRUE(sm->isStateActive("inner")) << "the run has to start inside `inner`. active:" << describe();

    // The raiser is in queued mode (see setImmediateMode(false) above), so the
    // event sits on the external queue until the drain runs.
    ASSERT_TRUE(sm->raiseExternalEvent("leave", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("settled"))
        << "`leave` did not carry the machine to `settled`. The document checks its clauses in "
           "document order and lands each in a `<final>` of its own: `failExits` (a handler did "
           "not run), `failSelfInInner` / `failSelfInOuter` (a state was already out of the "
           "configuration during its own `<onexit>`), `failParentInInner` (the parent left "
           "before its child's `<onexit>`), `failChildInOuter` (the child was still active "
           "during its parent's `<onexit>`). active:"
        << describe();
}

}  // namespace Tests
}  // namespace SCE
