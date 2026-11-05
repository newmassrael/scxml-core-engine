// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: the reading of inline `<content>` text is the data model's —
// Interpreter path.
//
// Sibling of `InlineContentIsReadByItsDataModelAotTest.cpp`. The Interpreter
// reads a `<send>`'s text by the document's data model
// (`SendAction::isContentUnderNullDatamodel`) and carries a `<donedata>`'s XML as the
// document text, so this pins the verdict every generated engine is held to.
//
// Fixture:
// integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml
// (canonical, shared with every channel).

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
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

class InlineContentIsReadByItsDataModelTest : public ::testing::Test {
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

TEST_F(InlineContentIsReadByItsDataModelTest, InlineContentIsTheValueItsDataModelReads) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/inline_content_is_read_by_its_data_model/"
                                "inline_content_is_read_by_its_data_model.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the child's `<send target="#_parent">` needs the dispatcher
    // chain (scheduler, target factory, dispatcher).
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
    sm->setEventRaiser(eventRaiser);
    sm->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm->start());

    // A top-level `<final>` halts processing, so the state freezes on the verdict.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "the parent did not halt within 5s: the child's events never reached it";

    const std::string reached = sm->terminalState().value_or("");
    EXPECT_NE(reached, "failNumber") << "`number` arrived with `_event.data` unequal to 21: under the ECMAScript "
                                     << "data model `<content>21</content>` is the number 21.";
    EXPECT_NE(reached, "failText") << "`text` arrived with `_event.data` other than 'two words': text that is no "
                                   << "expression is the space-normalized string it spells.";
    EXPECT_NE(reached, "failObject")
        << "`object` arrived without `_event.data.k === 7`: `<content>{\"k\": 7}</content>` "
        << "is an object literal, the object it denotes.";
    EXPECT_NE(reached, "failDocument") << "`done.invoke.reporter` arrived without a document whose `book` carries "
                                       << "title=\"x\": a `<donedata>`'s XML `<content>` is a document, not an "
                                       << "expression to evaluate.";
    EXPECT_EQ(reached, "pass");
}

}  // namespace Tests
}  // namespace SCE
