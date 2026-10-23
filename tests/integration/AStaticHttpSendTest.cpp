// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated C++: the
// `<param>`s of a BasicHTTP `<send>` are read from the machine's own fields when the send
// runs, and cross as the text a form carries (docs/adr/0005, decision 4, §scxml-C-2). The
// request is observed where the engine hands it to its transport, so no listener is
// involved. A scenario states what a machine's fields hold and not what it sent over a
// wire, so `static_send_http` has no scenario: each engine holds it with a test of its own
// that records the request.

#include <map>
#include <string>
#include <vector>

#include <gtest/gtest.h>

#include "static_send_http_sm.h"

namespace SCE::Tests {
namespace {

using HttpMachine = ::SCE::Generated::static_send_http::static_send_http;
using HttpParams = std::map<std::string, std::vector<std::string>>;

/// A machine whose transport keeps what it was handed.
struct HttpHarness {
    HttpMachine sm;
    std::vector<SCE::Static::HttpSendRequest> posted;

    HttpHarness() {
        sm.setHttpSendCallback([this](const SCE::Static::HttpSendRequest &request) { posted.push_back(request); });
        sm.initialize();
        sm.step();
    }

    void raise(const std::string &event) {
        sm.raiseExternal(event);
        sm.step();
    }
};

TEST(AStaticHttpSendTest, ASendOverHttpCarriesTheTextTheFieldsHoldWhenItRuns) {
    HttpHarness h;
    h.raise("bump");
    h.raise("go");

    ASSERT_EQ(h.posted.size(), 1u) << "one request is handed to the transport";
    const auto &request = h.posted[0];
    EXPECT_EQ(request.target, "http://example.invalid/hook");
    EXPECT_EQ(request.eventName, "note");
    EXPECT_EQ(request.content, "") << "no <content>, so the body is the pairs";
    const HttpParams wanted = {{"count", {"4"}}, {"ready", {"true"}}, {"label", {"busy"}},
                               {"twice", {"8"}}, {"delta", {"-5"}},   {"ratio", {"1.5"}}};
    EXPECT_EQ(request.params, wanted) << "each value is the text it spells: an integer's digits, `true`, the "
                                         "string, a negative number, a real's String()";
}

TEST(AStaticHttpSendTest, TheHttpPairsAreTheFieldsAsTheyStandAndNotACopyFromStartUp) {
    HttpHarness h;
    h.raise("go");

    ASSERT_EQ(h.posted.size(), 1u);
    const HttpParams wanted = {{"count", {"3"}}, {"ready", {"false"}}, {"label", {"idle"}},
                               {"twice", {"6"}}, {"delta", {"-5"}},    {"ratio", {"1.5"}}};
    EXPECT_EQ(h.posted[0].params, wanted) << "without `bump` the fields hold their initial values, and the request "
                                             "carries those: it is read when the send runs";
}

TEST(AStaticHttpSendTest, AHttpParamThatCannotBeReadIsLeftOutAndTheRequestStillGoes) {
    HttpHarness h;
    h.raise("bump");
    h.raise("boom");

    ASSERT_EQ(h.posted.size(), 1u) << "the request goes with the pair that could be read";
    const HttpParams wanted = {{"count", {"4"}}};
    EXPECT_EQ(h.posted[0].params, wanted) << "`big` is `count * 2000000000`, which a 32-bit field cannot hold: its "
                                             "pair is left out, not carried as a zero";
    EXPECT_EQ(h.sm.errors(), 1u) << "§scxml-5.7.1: the failed pair is reported as error.execution, once";
}

}  // namespace
}  // namespace SCE::Tests
