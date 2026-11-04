// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, D4 and D6: what the C++ Interpreter hands the host for a
// host-served <send>, a host-run <invoke> and a Mesh request, held to the table
// the generated engine is held to.
//
// The Interpreter evaluates each element itself, from its own model of the
// document, and the generated engine evaluates it from code the build wrote. A
// host is ported between them only if the two hand it the same request, field
// for field, so neither engine's test states the request: the table does, and
// HostRequestAotTest.cpp reads the same rows. The Mesh rows are the requests the
// build's lowering makes for the backends that route Mesh through the host
// (host_processor_analyzer.rs reads them for that), and the Interpreter, which
// serves a Mesh request through the router the host registers, is held to them
// here.
//
// Fixture: sce-build/tests/fixtures/host_processor/host_request_cases.json and
// the documents it names.

#include "integration/HostRequestCases.h"

#include "core/HostProcessor.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

/// The type the documents name for the host's processor and invoker.
constexpr const char *DECLARED_TYPE = "x-sce-host";

class HostRequestTest : public ::testing::Test {
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

    /// Run @p document with a processor and an invoker for the declared type and
    /// a Mesh router, each recording what it was handed, until the requests made
    /// on entering its initial state are all made; then stop it.
    void run(const std::string &document) {
        auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
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

        sm->registerEventProcessor(DECLARED_TYPE, [this](const HostSendRequest &request) {
            sends.push_back(request);
            return std::vector<HostSendResponse>{};
        });
        const auto record = [this](const HostInvokeEvent &event) {
            if (event.start.has_value()) {
                starts.push_back(*event.start);
            }
            return std::optional<HostInvokeResponse>();
        };
        sm->registerInvoker(DECLARED_TYPE, record);
        sm->registerMeshRpcInvoker(record);

        ASSERT_TRUE(sm->loadSCXML(HostRequestCases::fixtureDirectory() + document)) << document;
        ASSERT_TRUE(sm->start()) << document;
        sm->stop();
    }

    IScriptEngine *engine_ = nullptr;
    std::vector<HostSendRequest> sends;
    std::vector<HostInvokeRequest> starts;
};

}  // namespace

TEST_F(HostRequestTest, TheInterpreterHandsTheHostTheRequestsTheTableGives) {
    const auto rows = HostRequestCases::cases();
    // A floor: an empty table would pass every comparison below.
    ASSERT_GE(rows.size(), 6u) << "the table was not read";
    std::set<std::string> documents;
    for (const auto &row : rows) {
        documents.insert(row.at("document").get<std::string>());
    }
    for (const auto &document : documents) {
        run(document);
    }

    for (const auto &row : rows) {
        const auto &request = row.at("request");
        if (row.at("kind").get<std::string>() == "send") {
            const auto sendId = request.at("sendId").get<std::string>();
            const HostSendRequest *made = nullptr;
            for (const auto &send : sends) {
                if (send.sendId == sendId) {
                    made = &send;
                }
            }
            ASSERT_NE(made, nullptr) << row.at("name").get<std::string>() << ": the processor was never handed it";
            HostRequestCases::expectSendRequest(row, *made);
            continue;
        }
        const auto invokeId = request.at("invokeId").get<std::string>();
        const HostInvokeRequest *started = nullptr;
        for (const auto &start : starts) {
            if (start.invokeId == invokeId) {
                started = &start;
            }
        }
        ASSERT_NE(started, nullptr) << row.at("name").get<std::string>() << ": the invoker was never asked to start it";
        HostRequestCases::expectInvokeRequest(row, *started);
        // docs/adr/0045: the session that started it, which the host ends it by.
        EXPECT_FALSE(started->sessionId.empty()) << row.at("name").get<std::string>();
    }
}

}  // namespace SCE::Tests
