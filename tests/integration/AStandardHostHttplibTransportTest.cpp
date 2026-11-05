// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reference host's transport over cpp-httplib (sce/include/host/HttplibTransport.h),
// held against a local server.
//
// Apart from AStandardHostScenarioTest.cpp because cpp-httplib is a native library:
// sce/CMakeLists.txt does not build it under Emscripten, and this file is compiled
// into standard_host_scenario_test only where it is, as the HTTP tests of
// integration_tests are. The scenarios beside it reach the host through a scripted
// transport and run under Node too.

#include "host/HttplibTransport.h"
#include "host/StandardHost.h"

#include <gtest/gtest.h>
#include <httplib.h>
#include <nlohmann/json.hpp>
#include <string>
#include <thread>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;

}  // namespace

/// The transport over cpp-httplib: a local server answers, the host carries the request's headers and
/// body to it, follows no redirect, and gives the response's fields in a fixed order.
TEST(AStandardHostScenario, TheHttplibTransportCarriesAnExchangeToALocalServer) {
    httplib::Server server;
    std::string seenBody;
    std::string seenTrace;
    server.Post("/items", [&](const httplib::Request &request, httplib::Response &response) {
        seenBody = request.body;
        seenTrace = request.get_header_value("X-Trace");
        response.status = 201;
        response.set_header("X-Reply", "yes");
        response.set_content("made", "text/plain");
    });
    server.Get("/moved", [](const httplib::Request &, httplib::Response &response) {
        response.status = 301;
        response.set_header("Location", "/elsewhere");
    });
    server.Get("/missing", [](const httplib::Request &, httplib::Response &response) { response.status = 404; });
    const int port = server.bind_to_any_port("127.0.0.1");
    ASSERT_GT(port, 0);
    std::thread serving([&server] { server.listen_after_bind(); });
    server.wait_until_ready();
    const std::string base = "http://127.0.0.1:" + std::to_string(port);

    host::ScriptedClock clock;
    host::HttplibTransport transport;
    host::ReferenceHost reference(clock, transport);
    const auto fetch = [&reference](const std::string &method, const std::string &url, const std::string &headers = "",
                                    const std::string &body = "") {
        const auto outcome = reference.start("sce-host-http-fetch", "fetch",
                                             {{"method", method}, {"url", url}, {"headers", headers}, {"body", body}});
        EXPECT_TRUE(outcome && outcome->kind == host::Outcome::Kind::Done);
        return json::parse(outcome->data);
    };

    const json made = fetch("POST", base + "/items", "X-Trace: 7\nContent-Type: text/plain", "payload");
    EXPECT_EQ(made.at("error"), "");
    EXPECT_EQ(made.at("status"), 201);
    EXPECT_EQ(made.at("body"), "made");
    EXPECT_NE(made.at("headers").get<std::string>().find("x-reply: yes"), std::string::npos);
    EXPECT_EQ(seenBody, "payload");
    EXPECT_EQ(seenTrace, "7");
    const json moved = fetch("GET", base + "/moved");
    EXPECT_EQ(moved.at("status"), 301);
    EXPECT_NE(moved.at("headers").get<std::string>().find("location: /elsewhere"), std::string::npos);
    EXPECT_EQ(fetch("GET", base + "/missing").at("status"), 404);

    server.stop();
    serving.join();
    EXPECT_EQ(fetch("GET", base + "/items").at("error"), "network");
}

}  // namespace SCE::Tests
