// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.1 and B.2: the null data model has no expressions, and it is not
// a second reading of inline `<content>` either. Its children take the one
// reading every data model's do, with the expression step read as none: the
// XML as written when the text opens with `<`, and otherwise the
// space-normalized string — Interpreter path.
//
// Measured 2026-11-06 by reading both engines: under `datamodel="null"` the
// Interpreter's `<send>` quoted the children as written, so text laid out over
// lines arrived with its line breaks and runs of spaces, where the same
// document under ECMAScript, and every generated engine under either data
// model (sce-build's `inline_content_reading`), carries the normalized string.
//
// The null data model has no `_event.data` to test in a guard, so the data the
// session's own event arrived with is read from the machine once it has halted:
// the event stays bound until the next one is taken (`getCurrentEventData`),
// and the machine halts on taking it.

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <thread>

namespace SCE {
namespace Tests {

namespace {

// A session that sends itself `carried` with `content` as the children of its
// `<content>`, and halts in `received` on taking it. An error raised on the
// way ends the run in `failed` instead, and would have rebound the event.
std::string documentSending(const std::string &content) {
    return std::string(R"scxml(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="sending">
  <state id="sending">
    <onentry>
      <send event="carried"><content>)scxml") +
           content + R"scxml(</content></send>
    </onentry>
    <transition event="carried" target="received"/>
    <transition event="error" target="failed"/>
  </state>
  <final id="received"/>
  <final id="failed"/>
</scxml>)scxml";
}

// A session whose compound state `outer` ends at once in a `<final>` whose
// `<donedata>` holds `content`, and halts in `received` on taking the
// `done.state.outer` that carries it.
std::string documentEndingWith(const std::string &content) {
    return std::string(R"scxml(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="outer">
  <state id="outer" initial="ended">
    <final id="ended">
      <donedata><content>)scxml") +
           content + R"scxml(</content></donedata>
    </final>
    <transition event="done.state.outer" target="received"/>
    <transition event="error" target="failed"/>
  </state>
  <final id="received"/>
  <final id="failed"/>
</scxml>)scxml";
}

}  // namespace

class NullDataModelContentTakesTheInlineReadingTest : public ::testing::Test {
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

    /// Runs the document that sends `content` until it halts, and returns the
    /// data its `carried` event arrived with, as the JSON text it is bound as.
    std::string deliveredData(const std::string &content) {
        return dataOfTheLastEvent(documentSending(content));
    }

    /// Runs the document whose `<donedata>` holds `content` until it halts,
    /// and returns the data its `done.state.outer` arrived with.
    std::string doneData(const std::string &content) {
        return dataOfTheLastEvent(documentEndingWith(content));
    }

    /// Runs `document` until it halts in `received`, and returns the data of
    /// the event it halted on taking, as the JSON text it is bound as.
    std::string dataOfTheLastEvent(const std::string &document) {
        sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

        // §scxml-6.2: the `<send>` needs the dispatcher chain
        // (scheduler, target factory, dispatcher).
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

        EXPECT_TRUE(sm_->loadSCXMLFromString(document));
        EXPECT_TRUE(sm_->start());

        // A top-level `<final>` halts processing, so nothing is taken after
        // the event that ended the run.
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
        while (std::chrono::steady_clock::now() < deadline) {
            if (!sm_->isRunning()) {
                break;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }

        EXPECT_FALSE(sm_->isRunning()) << "the session did not halt within 5s: the event never reached it";
        EXPECT_EQ(sm_->terminalState().value_or(""), "received")
            << "the run did not end on the event: an error was raised while its data was made";
        return sm_->getCurrentEventData();
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

// Text laid out over lines, with runs of spaces and a tab, is the string it
// spells with each run of whitespace one space, and none at either end.
TEST_F(NullDataModelContentTakesTheInlineReadingTest, TextLaidOutOverLinesIsItsSpaceNormalizedString) {
    EXPECT_EQ(deliveredData("\n        alpha    beta\n        gamma \t delta\n      "), "\"alpha beta gamma delta\"");
}

// The null data model reads no expression in the text, so text that would be
// one under ECMAScript is a string: `21` is not the number.
TEST_F(NullDataModelContentTakesTheInlineReadingTest, ANumberElsewhereIsAString) {
    EXPECT_EQ(deliveredData("21"), "\"21\"");
}

// Nor is `1  +   1` the 2 it evaluates to elsewhere: it is its normalized string.
TEST_F(NullDataModelContentTakesTheInlineReadingTest, AnExpressionElsewhereIsItsNormalizedString) {
    EXPECT_EQ(deliveredData("1  +   1"), "\"1 + 1\"");
}

// Markup is the XML document it is, as written and not quoted: the whitespace
// inside it is the document's and is not normalized away.
TEST_F(NullDataModelContentTakesTheInlineReadingTest, MarkupTravelsAsWritten) {
    const std::string data = deliveredData("<a>b  c</a>");
    EXPECT_EQ(data.rfind("<a", 0), 0u) << data;
    EXPECT_NE(data.find("b  c"), std::string::npos) << data;
}

// A `<donedata>`'s children are the same construct and take the same reading:
// the space-normalized string, where the null model's done data used to be
// the text verbatim, line breaks and runs of spaces included.
TEST_F(NullDataModelContentTakesTheInlineReadingTest, DoneDataTextIsItsSpaceNormalizedString) {
    EXPECT_EQ(doneData("\n        alpha    beta\n        gamma \t delta\n      "), "\"alpha beta gamma delta\"");
}

// Nor does a `<donedata>` read an expression the null model does not have.
TEST_F(NullDataModelContentTakesTheInlineReadingTest, DoneDataExpressionElsewhereIsItsNormalizedString) {
    EXPECT_EQ(doneData("1  +   1"), "\"1 + 1\"");
}

// And its markup is the XML document as written, not a quoted string.
TEST_F(NullDataModelContentTakesTheInlineReadingTest, DoneDataMarkupTravelsAsWritten) {
    const std::string data = doneData("<a>b  c</a>");
    EXPECT_EQ(data.rfind("<a", 0), 0u) << data;
    EXPECT_NE(data.find("b  c"), std::string::npos) << data;
}

}  // namespace Tests
}  // namespace SCE
