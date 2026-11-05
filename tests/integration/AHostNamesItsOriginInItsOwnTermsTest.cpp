// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 3): a host names the origin of an event
// it delivers in its own terms — an ending of a host-run invocation whose
// origin is {x-sce-host, motor} reads back `motor` and `x-sce-host` — C++
// Interpreter. The address has no scheme, so it is no SCXML location, and the
// type is the host's: the engine neither wraps the one nor defaults the other.
//
// Fixture:
// integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml

#include "core/EventOrigin.h"
#include "core/HostProcessor.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

constexpr const char *HOST_TYPE = "x-sce-host";

}  // namespace

class AHostNamesItsOriginInItsOwnTermsTest : public ::testing::Test {
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
    // The starts the invoker was handed. Members, so they outlive the machine a
    // test leaves running, which TearDown stops.
    std::mutex tokensMutex_;
    std::vector<uint64_t> tokens_;
};

TEST_F(AHostNamesItsOriginInItsOwnTermsTest, AHostNamesItsOriginInItsOwnTerms) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/a_host_names_its_origin_in_its_own_terms/"
                                "a_host_names_its_origin_in_its_own_terms.scxml";

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

    // An invoker whose work outlives the start: the test ends it, naming the
    // party it stands for.
    sm_->registerInvoker(HOST_TYPE, [this](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (event.start.has_value()) {
            std::lock_guard<std::mutex> lock(tokensMutex_);
            tokens_.push_back(event.start->token);
        }
        return std::nullopt;
    });

    ASSERT_TRUE(sm_->loadSCXML(fixture)) << "canonical fixture not loadable: " << fixture;
    ASSERT_TRUE(sm_->start());

    uint64_t token = 0;
    {
        std::lock_guard<std::mutex> lock(tokensMutex_);
        ASSERT_EQ(tokens_.size(), 1u) << "the host was not asked to run `job` once";
        token = tokens_[0];
    }
    EXPECT_TRUE(sm_->completeHostInvoke(HOST_TYPE, "job", token, "{}", Core::EventOrigin(HOST_TYPE, "motor")))
        << "the running invocation's completion was refused";

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "the run must end in `done`";
    EXPECT_EQ(read("hostNamed === 1"), "true")
        << "hostNamed = " << read("hostNamed")
        << ": the completion did not read back the address and the type the host named";
}

}  // namespace Tests
}  // namespace SCE
