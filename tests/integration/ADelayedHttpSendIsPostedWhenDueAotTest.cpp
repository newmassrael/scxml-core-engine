// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.2: a delay is a property of the send, not of the
// processor it names — a delayed BasicHTTP send is POSTed when due, and
// <cancel> reaches it while it waits — C++ AOT.
//
// Fixture: integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_delayed_http_send_is_posted_when_due ...)`.

#include "a_delayed_http_send_is_posted_when_due_sm.h"
#include "common/SceClock.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_delayed_http_send_is_posted_when_due::a_delayed_http_send_is_posted_when_due;

std::vector<std::string> events(const std::vector<SCE::Static::HttpSendRequest> &posted) {
    std::vector<std::string> names;
    for (const auto &request : posted) {
        names.push_back(request.eventName);
    }
    return names;
}

}  // namespace

TEST(ADelayedHttpSendIsPostedWhenDueAotTest, ADelayedHttpSendIsPostedWhenDue) {
    SM sm;
    sm.setClock(std::make_shared<SCE::ManualClock>(0));
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    // The host stands in for the HTTP transport: it keeps what it was handed.
    std::vector<SCE::Static::HttpSendRequest> posted;
    sm.setHttpSendCallback([&posted](const SCE::Static::HttpSendRequest &request) { posted.push_back(request); });
    sm.initialize();

    // A zero wait, written or evaluated, is no deferral: the POST is made
    // before initialize() returns, with no tick to bring it out.
    EXPECT_EQ(events(posted), (std::vector<std::string>{"now", "zero", "zeroexpr"}))
        << "the undelayed send and the two zero-delay sends are POSTed at once";

    sm.advanceTimeMs(99);
    EXPECT_EQ(events(posted), (std::vector<std::string>{"now", "zero", "zeroexpr"}))
        << "a send delayed 100ms is not POSTed at 99ms";

    sm.advanceTimeMs(1);
    ASSERT_EQ(events(posted), (std::vector<std::string>{"now", "zero", "zeroexpr", "later"}))
        << "it is POSTed when the delay has elapsed, and the cancelled one never is";
    EXPECT_EQ(posted[3].target, "http://127.0.0.1:18081/later");
    EXPECT_EQ(posted[3].sendId, "later");

    sm.advanceTimeMs(100);
    ASSERT_EQ(events(posted), (std::vector<std::string>{"now", "zero", "zeroexpr", "later", "dynamic"}));
    EXPECT_EQ(posted[4].target, "http://127.0.0.1:18081/dynamic") << "a targetexpr is read when the send is made";
    ASSERT_EQ(posted[4].params.count("k"), 1u);
    EXPECT_EQ(posted[4].params.at("k"), (std::vector<std::string>{"v"}));

    sm.advanceTimeMs(100);
    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "the run must end in `done`";
    EXPECT_EQ(events(posted), (std::vector<std::string>{"now", "zero", "zeroexpr", "later", "dynamic"}));
}

}  // namespace SCE::Tests
