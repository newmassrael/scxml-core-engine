// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + B.2: a payload a HOST injects reaches the datamodel as a
// value — Interpreter path.
//
// `raiseExternalEvent(name, data)` takes the payload as its second argument,
// and until this fixture every caller in the repository passed `""` — measured
// 2026-08-16, on every channel. So the boundary an embedder actually calls was
// covered by no test, while the payload paths the W3C suite does cover
// (`<send><content>`, `<param>`, `<donedata>`) all originate INSIDE the
// document and are lowered separately.
//
// Sibling of `EventDataArrivesAsSentAotTest.cpp` (C++ AOT). Each engine
// decodes the payload through its own code, so an engine that is right today
// is not thereby asked less.
//
// Fixture:
// integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml

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

class EventDataArrivesAsSentTest : public ::testing::Test {
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

TEST_F(EventDataArrivesAsSentTest, AHostsJsonPayloadIsAddressableAndItsTextStaysText) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) + "/integration_resources/event_data_arrives_as_sent/"
                                                                "event_data_arrives_as_sent.scxml";
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

    const auto describe = [&sm]() {
        std::string out;
        for (const auto &s : sm->getActiveStates()) {
            out += " " + s;
        }
        return out;
    };

    ASSERT_TRUE(sm->isStateActive("waiting"))
        << "the fixture is supposed to start in `waiting`, so nothing below is testing what it "
           "claims. active:"
        << describe();

    // A JSON object, the shape an embedder has when it holds structured data
    // and a state machine to give it to. The raiser is in queued mode, so the
    // event sits on the external queue until the drain runs — the same
    // macrostep boundary the engine uses for its own sends.
    ASSERT_TRUE(sm->raiseExternalEvent("payload", R"({"milestone":"refined","turns":2})"));
    eventRaiser->processQueuedEvents();

    EXPECT_FALSE(sm->isStateActive("mangled"))
        << "the host sent a JSON object and the guard `_event.data.milestone === 'refined' && "
           "_event.data.turns === 2` did not hold, so the payload did not arrive as an object "
           "with those properties. active:"
        << describe();
    ASSERT_TRUE(sm->isStateActive("heard"))
        << "the payload guard neither matched nor mismatched — the machine is not in `heard`. "
           "active:"
        << describe();

    // Text that is not JSON. The same call, and it must NOT be parsed into
    // something else: `hold the line` is the value the document compares
    // against, character for character.
    ASSERT_TRUE(sm->raiseExternalEvent("note", "hold the line"));
    eventRaiser->processQueuedEvents();

    EXPECT_FALSE(sm->isStateActive("garbled"))
        << "the host sent the text `hold the line` and `_event.data === 'hold the line'` did not "
           "hold, so a payload that is not JSON did not arrive as the string it was sent as. "
           "active:"
        << describe();

    // Text that happens to be a valid expression. §scxml-B-2-8-1 gives the
    // payload three readings and none of them is "evaluate it": a payload is
    // what a host, a peer session or an HTTP sender put there, and running it
    // makes `_event.data` mean whatever the receiver's engine is written in.
    ASSERT_TRUE(sm->raiseExternalEvent("arith", "2 + 3"));
    eventRaiser->processQueuedEvents();

    EXPECT_FALSE(sm->isStateActive("evaluated"))
        << "the host sent the text `2 + 3` and it arrived as 5 — the payload was run rather than "
           "read. active:"
        << describe();
    EXPECT_TRUE(sm->isStateActive("documented"))
        << "the arithmetic-shaped payload neither matched nor mismatched. active:" << describe();

    // §scxml-B-2-8-1's XML rung, reached through the EVENT path. The `<data>`
    // path is `xml_data_is_a_dom_tree`'s and the two are lowered on separate
    // code in every backend.
    // Leading whitespace on purpose: the reading is chosen by the first
    // NON-blank character, and a pretty-printed document is the ordinary shape
    // of one. The scan past it is small enough to look redundant.
    ASSERT_TRUE(sm->raiseExternalEvent("doc", "\n  "
                                              R"(<books xmlns=""><book title="t1"/></books>)"));
    eventRaiser->processQueuedEvents();

    EXPECT_FALSE(sm->isStateActive("flattened"))
        << "the host sent a well-formed XML document and "
           "`_event.data.documentElement.nodeName === 'books'` did not hold, so the payload did "
           "not become the DOM structure the clause requires. active:"
        << describe();

    // The sentence that closes the clause. Every `error.*` message this
    // repository raises names the SCXML construct that failed, so every one of
    // them has exactly this shape: it opens like a document and is not one.
    // Until 2026-08-19 this engine built a DOM out of the fragment that
    // happened to parse, because `XMLDocument::isValid` asked whether the tree
    // was non-empty rather than whether the parse succeeded.
    ASSERT_TRUE(sm->raiseExternalEvent("broken", "<assign>  to  detail failed"));
    eventRaiser->processQueuedEvents();

    EXPECT_FALSE(sm->isStateActive("swallowed"))
        << "the host sent `<assign>  to  detail failed`, which opens with `<` and is not a valid "
           "XML document, so §scxml-B-2-8-1's closing MUST applies and the reading is the "
           "space-normalized string. active:"
        << describe();
    EXPECT_TRUE(sm->isStateActive("settled"))
        << "the malformed-XML payload neither matched nor mismatched. active:" << describe();
}

}  // namespace Tests
}  // namespace SCE
