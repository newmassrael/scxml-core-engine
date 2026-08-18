// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: a `<data>` element's XML content is a DOM structure a
// document can walk — Interpreter path.
//
// The appendix obliges the Processor to create "the corresponding DOM
// structure". Measured 2026-08-18, what every backend created was an object
// carrying three methods — `getElementsByTagName`, `getAttribute` and a
// non-standard `getTagName`, which are the two names the W3C IRP suite reads
// plus one — so `doc.tagName`, `doc.firstChild` and `doc.childNodes.length`
// answered nil on all seven channels with 204/204 W3C fixtures green.
//
// What this asks that `tests/engine/DomReadSurfaceTest.cpp` does not: that a
// DOCUMENT reaches the binding. That test measures both C++ engines directly
// against `tests/ecmascript/dom_read_surface.json`; this one goes through the
// `<data>` initializer and the guards, which is the path an author's document
// takes.
//
// Sibling of `XmlDataIsADomTreeAotTest.cpp` (C++ AOT). The Interpreter reads
// the fixture at run time and the AOT engine compiles it, so neither is asked
// less because the other passes.
//
// Fixture:
// integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml

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

class XmlDataIsADomTreeTest : public ::testing::Test {
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

TEST_F(XmlDataIsADomTreeTest, ADataElementsXmlIsADomTreeTheDocumentCanWalk) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) + "/integration_resources/xml_data_is_a_dom_tree/"
                                                                "xml_data_is_a_dom_tree.scxml";
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
    // Every transition is eventless, so starting the machine runs it to its
    // verdict; no event is needed to ask the question.
    ASSERT_TRUE(sm->start());

    const auto describe = [&sm]() {
        std::string out;
        for (const auto &s : sm->getActiveStates()) {
            out += " " + s;
        }
        return out;
    };

    EXPECT_FALSE(sm->isStateActive("notADocument"))
        << "the variable did not hold a document: `doc.nodeType === 9`, `doc.nodeName === "
           "'#document'`, `doc.documentElement.tagName === 'books'` or `doc.hasAttribute('count')` "
           "did not hold. active:"
        << describe();
    EXPECT_FALSE(sm->isStateActive("wrongTree"))
        << "the document element's children are not the two `<book>` elements in document order — "
           "the whitespace between them may have become nodes, or a sibling/parent link is "
           "missing. active:"
        << describe();
    EXPECT_FALSE(sm->isStateActive("noText"))
        << "character data did not report itself as a text node, or `textContent` did not read the "
           "text below the element. active:"
        << describe();
    EXPECT_TRUE(sm->isStateActive("settled"))
        << "the machine reached none of its four verdicts, so the guards did not evaluate at all. "
           "active:"
        << describe();
}

}  // namespace Tests
}  // namespace SCE
