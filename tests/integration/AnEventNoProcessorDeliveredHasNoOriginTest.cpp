// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 2): an event no Event I/O Processor
// delivered has no origin — a host-posted event and a host processor's reply
// that name none read both `_event.origin` and `_event.origintype` as the empty
// string — C++ Interpreter.
//
// Fixture:
// integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml

#include "core/HostProcessor.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <thread>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class AnEventNoProcessorDeliveredHasNoOriginTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
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

    /// A value the handlers recorded (W3C SCXML 5.3), for the failure message.
    std::string read(const std::string &name) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), name).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(AnEventNoProcessorDeliveredHasNoOriginTest, AnEventNoProcessorDeliveredHasNoOrigin) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/an_event_no_processor_delivered_has_no_origin/"
                                "an_event_no_processor_delivered_has_no_origin.scxml";

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

    // The host's processor answers the document's send with a reply that names
    // no party, so the reply carries no origin.
    int served = 0;
    sm_->registerEventProcessor("x-sce-host", [&served](const HostSendRequest &) {
        ++served;
        return std::vector<HostSendResponse>{{"host.reply", "", std::nullopt}};
    });

    ASSERT_TRUE(sm_->loadSCXML(fixture)) << "canonical fixture not loadable: " << fixture;
    ASSERT_TRUE(sm_->start());

    // The host raises an event of its own, naming no origin.
    sm_->processEvent("posted", "");
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(served, 1) << "the host's processor was not handed the document's send once";
    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "the run must end in `done`";
    const std::vector<std::pair<std::string, int>> expected = {{"postedBlank", 1}, {"replyBlank", 1}};
    for (const auto &[name, value] : expected) {
        EXPECT_EQ(read(name + " === " + std::to_string(value)), "true")
            << name << " = " << read(name) << ", want " << value
            << " (an engine invented an origin for an event no processor delivered)";
    }
}

}  // namespace Tests
}  // namespace SCE
