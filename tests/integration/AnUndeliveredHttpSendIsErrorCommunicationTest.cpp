// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4: "If it is unable to dispatch the message, the Processor
// MUST place the error error.communication on the internal event queue." A
// BasicHTTP POST that cannot reach its server is such a message — Interpreter
// path, for a send made at once and for one whose delay elapses.
//
// The Interpreter mapped only a target that names no session to the error, and
// left a target that was found and could not be reached (a refused connection)
// in its log: the document never heard that its message did not go, and waited
// for an answer that was never coming. The generated engines' siblings are the
// `a_delayed_http_send_is_posted_when_due` tests of each lane, whose transport
// reports the POST of `now` undelivered.

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"
#include "tests/w3c/W3CHttpTestServer.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <thread>

namespace SCE {
namespace Tests {

class AnUndeliveredHttpSendIsErrorCommunicationTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
        // A port nothing listens on: one the kernel assigned to a listener that
        // has since stopped, so a POST there is refused.
        W3C::W3CHttpTestServer probe(0, "/test");
        ASSERT_TRUE(probe.start()) << "could not bind a probe listener to find a free port";
        closedPort_ = probe.getPort();
        probe.stop();
    }

    void TearDown() override {
        if (sm_) {
            sm_->stop();
        }
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A document whose one BasicHTTP send, made with `delayAttribute`, is
    /// addressed to the closed port: `pass` on that send's error.communication,
    /// `fail` on anything else.
    std::string document(const std::string &delayAttribute) const {
        return R"scxml(<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
  <state id="s">
    <onentry>
      <send id="post" event="ping" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            target="http://127.0.0.1:)scxml" +
               std::to_string(closedPort_) + R"scxml(/test" )scxml" + delayAttribute + R"scxml(/>
    </onentry>
    <transition event="error.communication" cond="_event.sendid == 'post'" target="pass"/>
    <transition event="*" target="fail"/>
  </state>
  <final id="pass"/>
  <final id="fail"/>
</scxml>)scxml";
    }

    /// Run `scxml` until it ends, or for at most ten seconds, and answer where.
    std::string endOf(const std::string &scxml) {
        sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
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

        EXPECT_TRUE(sm_->loadSCXMLFromString(scxml));
        EXPECT_TRUE(sm_->start());
        for (int i = 0; i < 200 && !sm_->terminalState().has_value(); ++i) {
            eventRaiser->processQueuedEvents();
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        }
        return sm_->terminalState().value_or("<still running>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
    int closedPort_ = 0;
};

TEST_F(AnUndeliveredHttpSendIsErrorCommunicationTest, ASendMadeAtOnce) {
    EXPECT_EQ(endOf(document("")), "pass")
        << "a POST refused by a port nothing listens on must put error.communication, with the send's id, "
           "on the internal queue";
}

TEST_F(AnUndeliveredHttpSendIsErrorCommunicationTest, ASendWhoseDelayElapses) {
    EXPECT_EQ(endOf(document(R"(delay="10ms")")), "pass")
        << "a delayed POST refused when its delay elapses must put error.communication, with the send's id, "
           "on the internal queue";
}

}  // namespace Tests
}  // namespace SCE
