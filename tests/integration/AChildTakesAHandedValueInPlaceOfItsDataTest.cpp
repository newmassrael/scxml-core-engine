// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4 (docs/adr/0098): a value an invoke hands its child is the
// initial value of the child's top-level <data> of that name, declared in
// place of its own — C++ Interpreter.
//
// The parent evaluates every <param> and namelist location and hands the
// child every value (`SCXMLInvokeHandler`); the child's start decides what it
// takes (`DataModelInitializer::initializeAllDataItems`): a top-level <data>
// whose id was handed is declared with that value and its own expr is never
// evaluated, a state's <data> keeps its own, and a name no top-level <data>
// declares is not added. The parent keeps what each child reported in its
// own datamodel and judges it, landing a failure in a <final> named for the
// sentence that broke; the test reads the verdict and the kept values.
//
// Sibling of `AChildTakesAHandedValueInPlaceOfItsDataAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "integration/AHandedValueRefusingEngine.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>
#include <thread>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class AChildTakesAHandedValueInPlaceOfItsDataTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// Runs the fixture over `engine` until the parent halts.
    void run(IScriptEngine &engine) {
        const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                    "/integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/"
                                    "a_child_takes_a_handed_value_in_place_of_its_data.scxml";
        std::ifstream in(fixture);
        ASSERT_TRUE(in.is_open()) << "fixture not readable: " << fixture;
        std::ostringstream buffer;
        buffer << in.rdbuf();

        sm_ = std::make_shared<StateMachine>(engine);
        // §scxml-6.2: each child reports with `<send target="#_parent">`, which
        // needs the dispatcher chain wired on the Interpreter, as
        // `InvokeParamSeedsDeclaredChildDataTest` wires it.
        auto scheduler = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                try {
                    return target->send(event).get().isSuccess;
                } catch (...) {
                    return false;
                }
            });
        auto eventRaiser = std::make_shared<EventRaiserImpl>();
        eventRaiser->setScheduler(scheduler);
        eventRaiser->setImmediateMode(false);
        sm_->setEventRaiser(eventRaiser);
        sm_->setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

        ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
        ASSERT_TRUE(sm_->start());

        // §scxml-3.13: a top-level `<final>` halts the parent once the last
        // child has ended, or once a judgement failed.
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
        while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        ASSERT_FALSE(sm_->isRunning())
            << "the parent did not halt within 5s: a child never reported, or the child of `firstMacrostep` "
               "read its own value of `handed` in its first macrostep and stands in `missed`. "
            << describe();
    }

    /// A value the parent kept, for the failure message.
    std::string read(const char *expression) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), expression).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    /// Whether an expression over what the parent kept holds, compared in the
    /// datamodel so a number is not compared as text.
    bool holds(const char *expression) const {
        return read(expression) == "true";
    }

    std::string describe() const {
        std::string text = "terminal=" + sm_->terminalState().value_or("<none>") + " ";
        for (const char *name : {"raising_seen", "effectful_seen", "effects_seen", "refusable_seen", "kept_seen",
                                 "stranger_seen", "payload_seen", "script_seen", "entry_seen", "errors_seen",
                                 "listed_seen", "stray_seen", "record_seen", "list_errors_seen"}) {
            text += std::string(name) + "=" + read(name) + " ";
        }
        return text;
    }

    IScriptEngine *engine_ = nullptr;
    /// The double a refusing run is driven over. Declared before `sm_`, so it
    /// outlives the machine that holds a reference to it.
    std::unique_ptr<AHandedValueRefusingEngine> refusing_;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(AChildTakesAHandedValueInPlaceOfItsDataTest, AHandedValueIsDeclaredInPlaceOfTheDatasOwn) {
    run(ScriptEngineProvider::getScriptEngine());
    if (HasFatalFailure()) {
        return;
    }
    EXPECT_EQ(sm_->terminalState().value_or(""), "pass")
        << "failOwnValueRan / failChildRaised: the own expr of a <data> whose value was handed was "
           "evaluated, and §scxml-6.4 says any value specified in the <data> element is ignored. "
           "failStateDataTookTheValue / failLateStateDataTookTheValue: a state's <data> took a handed "
           "value; only the top-level declarations take one. failUnmatchedNameAdded / "
           "failNamelistNameAdded: a handed name that matches no top-level <data> became a variable. "
           "failObjectNotWhole: an object value lost its nesting or its array. "
           "failChildSharedTheParentsObject: the child was handed the parent's object, not a copy. "
           "failScriptReadItsOwnValue / failEntryReadItsOwnValue: the top-level <script> or the initial "
           "<onentry> ran before the values were declared. failHandedValueLost / failNamelistValueLost / "
           "failNamelistOwnValueRaised: a handed value did not take its <data>'s place. "
        << describe();
}

// §scxml-5.3: a handed value the child's session cannot hold is the error of
// that <data>'s initialization, as a failing expr would be: error.execution,
// and the variable left unbound. The parent judges every other value before
// the child's error count, so `failChildRaised` says they all took their
// places.
TEST_F(AChildTakesAHandedValueInPlaceOfItsDataTest, AHandedValueTheSessionCannotHoldFailsItsData) {
    refusing_ = std::make_unique<AHandedValueRefusingEngine>("refusable");
    run(*refusing_);
    if (HasFatalFailure()) {
        return;
    }
    EXPECT_EQ(refusing_->refusals(), 1u) << "the child declares the handed value once, and nothing else. "
                                         << describe();
    EXPECT_EQ(sm_->terminalState().value_or(""), "failChildRaised")
        << "the child that could not hold `refusable` raised error.execution with every other value in "
           "its place. "
        << describe();
    EXPECT_TRUE(holds("refusable_seen === 'unbound'"))
        << "the <data> whose handed value could not be held took a value: it must be left as one whose "
           "initialization failed. "
        << describe();
    EXPECT_TRUE(holds("errors_seen === 1"))
        << "the failed declaration of a handed value raised no error.execution in the child, or another "
           "<data> raised too. "
        << describe();
}

}  // namespace Tests
}  // namespace SCE
