// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, D4: what the generated C++ engine hands the host for a
// host-served <send> and a host-run <invoke>, held to the table the Interpreter
// is held to (HostRequestTest.cpp). The rows of the table are the requests; this
// engine runs the documents they name and compares what its processor and its
// invoker were handed, field by field. The Mesh rows are not this engine's: its
// generated router takes a Mesh request itself (SCE_MESH.md, section 19).
//
// Fixtures: sce-build/tests/fixtures/host_processor/statechart_host_request.scxml,
// compiled by tests/CMakeLists.txt with `x-sce-host` declared as processor and
// as invoker, and statechart_host_request_null.scxml, the null data model's
// reading, compiled with `x-sce-host` declared as processor.

#include "statechart_host_request_null_sm.h"
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
using NullMachine = SCE::Generated::statechart_host_request_null::statechart_host_request_null;

/// The type the fixtures were compiled for, as processor and as invoker.
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

/// Hold what @p sends and @p starts recorded to the table's rows for @p document,
/// of which there are at least @p floor.
void expectTheRowsOf(const std::string &document, size_t floor, const std::vector<SCE::HostSendRequest> &sends,
                     const std::vector<SCE::HostInvokeRequest> &starts) {
    const auto rows = HostRequestCases::casesOf(document);
    // A floor: a table with no row for this document would pass every comparison below.
    ASSERT_GE(rows.size(), floor) << "the table names too few requests of " << document;
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
    EXPECT_EQ(sm.initialize(), std::nullopt);
    sm.step();

    expectTheRowsOf("statechart_host_request.scxml", 5u, sends, starts);
}

// The null data model has no expressions: an inline <content> is handed to the
// host as it is written, and its event data is the space-normalized string, on
// this engine as on the Interpreter.
TEST_F(HostRequestAotTest, UnderTheNullDataModelTheGeneratedEngineHandsTheHostTheContentAsWritten) {
    std::vector<SCE::HostSendRequest> sends;

    NullMachine sm;
    sm.registerEventProcessor(DECLARED_TYPE, [&sends](const SCE::HostSendRequest &request) {
        sends.push_back(request);
        return std::vector<SCE::HostSendResponse>{};
    });
    sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                           [](SCE::IScriptEngine *) {}));
    EXPECT_EQ(sm.initialize(), std::nullopt);
    sm.step();

    expectTheRowsOf("statechart_host_request_null.scxml", 1u, sends, {});
}

}  // namespace SCE::Tests
