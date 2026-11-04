// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, D4: what the generated C++ engine hands the host for a
// host-served <send> and a host-run <invoke>, held to the table the Interpreter
// is held to (HostRequestTest.cpp). The rows of the table are the requests; this
// engine runs the document they name and compares what its processor and its
// invoker were handed, field by field. The Mesh rows are not this engine's: its
// generated router takes a Mesh request itself (SCE_MESH.md, section 19).
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_request.scxml,
// compiled by tests/CMakeLists.txt with `x-sce-host` declared as processor and
// as invoker.

#include "statechart_host_request_sm.h"

#include "integration/HostRequestCases.h"

#include "core/HostProcessor.h"
#include "scripting/JSEngine.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::statechart_host_request::statechart_host_request;

/// The type the fixture was compiled for, as processor and as invoker.
constexpr const char *DECLARED_TYPE = "x-sce-host";

class HostRequestAotTest : public ::testing::Test {
protected:
    void SetUp() override {
        SCE::JSEngine::instance().initialize();
    }

    void TearDown() override {
        SCE::JSEngine::instance().shutdown();
    }
};

}  // namespace

TEST_F(HostRequestAotTest, TheGeneratedEngineHandsTheHostTheRequestsTheTableGives) {
    std::vector<SCE::HostSendRequest> sends;
    std::vector<SCE::HostInvokeRequest> starts;

    Machine sm;
    sm.registerEventProcessor(DECLARED_TYPE, [&sends](const SCE::HostSendRequest &request) {
        sends.push_back(request);
        return std::vector<SCE::HostSendResponse>{};
    });
    sm.registerInvoker(DECLARED_TYPE, [&starts](const SCE::HostInvokeEvent &event) {
        if (event.start.has_value()) {
            starts.push_back(*event.start);
        }
        return std::optional<SCE::HostInvokeResponse>();
    });
    // Registered before the machine runs: the requests are made as its initial
    // state is entered.
    sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                           [](SCE::IScriptEngine *) {}));
    sm.initialize();
    sm.step();

    const auto rows = HostRequestCases::casesOf("statechart_host_request.scxml");
    // A floor: a table with no row for this document would pass every comparison below.
    ASSERT_GE(rows.size(), 4u) << "the table names no request of statechart_host_request.scxml";
    for (const auto &row : rows) {
        const auto &request = row.at("request");
        if (row.at("kind").get<std::string>() == "send") {
            const auto sendId = request.at("sendId").get<std::string>();
            const SCE::HostSendRequest *made = nullptr;
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
        const SCE::HostInvokeRequest *started = nullptr;
        for (const auto &start : starts) {
            if (start.invokeId == invokeId) {
                started = &start;
            }
        }
        ASSERT_NE(started, nullptr) << row.at("name").get<std::string>() << ": the invoker was never asked to start it";
        HostRequestCases::expectInvokeRequest(row, *started);
    }
}

}  // namespace SCE::Tests
