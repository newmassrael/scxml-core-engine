// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reference host's network transport on libcurl (sce/include/host/CurlTransport.h) behind its
// destination policy (docs/adr/0109, D4), held against a cpp-httplib server on a loopback port of this
// process. The names are answered by a resolver the test scripts, so nothing reaches another machine: a
// name that resolves to loopback is refused before a byte is sent, one the allow-list lets reach loopback
// is served, a name is resolved once per exchange and the connection is made to the address checked, a
// redirect is answered as it came, and the bounds of time and size hold.
//
// Compiled into standard_host_scenario_test only where libcurl (sce_host_http_curl,
// cmake/SCEHostHttp.cmake) and cpp-httplib are both built.

#include "host/CurlTransport.h"
#include "host/DestinationPolicy.h"
#include "host/StandardHost.h"

#if defined(_WIN32)
#include <winsock2.h>
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#endif

#include <algorithm>
#include <chrono>
#include <cstring>
#include <gtest/gtest.h>
#include <httplib.h>
#include <map>
#include <memory>
#include <mutex>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <thread>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;

host::IpAddress address(const std::string &text) {
    const auto parsed = host::IpAddress::parse(text);
    EXPECT_TRUE(parsed.has_value()) << text;
    return parsed.value_or(host::IpAddress{});
}

host::IpNetwork network(const std::string &text, size_t prefix) {
    const auto made = host::IpNetwork::make(address(text), prefix);
    EXPECT_TRUE(made.has_value()) << text << "/" << prefix;
    return *made;
}

/// A resolver that answers each name with the next of its answers (the last one again once they run out),
/// and records the names it was asked.
class ScriptedResolver : public host::Resolver {
public:
    explicit ScriptedResolver(std::map<std::string, std::vector<std::vector<std::string>>> answers)
        : answers_(std::move(answers)) {}

    std::optional<std::vector<host::IpAddress>> resolve(const std::string &name) override {
        const size_t before = static_cast<size_t>(std::count(asked.begin(), asked.end(), name));
        asked.push_back(name);
        const auto found = answers_.find(name);
        if (found == answers_.end() || found->second.empty()) {
            return std::nullopt;
        }
        const auto &turn = found->second[std::min(before, found->second.size() - 1)];
        std::vector<host::IpAddress> addresses;
        for (const auto &text : turn) {
            addresses.push_back(address(text));
        }
        return addresses;
    }

    std::vector<std::string> asked;

private:
    std::map<std::string, std::vector<std::vector<std::string>>> answers_;
};

/// A server on a loopback port: `/ok` is served, `/moved` redirects to a private address, `/big` sends a
/// body of 65 bytes, `/wide` a head of about 10000 bytes, and `/slow` answers after four seconds. It
/// records the `Host` of every request it was sent.
class LoopbackServer {
public:
    LoopbackServer() {
        server_.Get("/ok", [this](const httplib::Request &request, httplib::Response &response) {
            seen(request);
            response.set_header("X-Reply", "yes");
            response.set_content("served", "text/plain");
        });
        server_.Get("/moved", [this](const httplib::Request &request, httplib::Response &response) {
            seen(request);
            response.status = 302;
            response.set_header("Location", "http://10.0.0.1/inside");
        });
        server_.Get("/big", [this](const httplib::Request &request, httplib::Response &response) {
            seen(request);
            response.set_content(std::string(65, 'x'), "text/plain");
        });
        server_.Get("/wide", [this](const httplib::Request &request, httplib::Response &response) {
            seen(request);
            response.set_header("X-Wide", std::string(10000, 'w'));
        });
        server_.Get("/slow", [this](const httplib::Request &request, httplib::Response &) {
            seen(request);
            std::this_thread::sleep_for(std::chrono::seconds(4));
        });
        port_ = server_.bind_to_any_port("127.0.0.1");
        serving_ = std::thread([this] { server_.listen_after_bind(); });
        server_.wait_until_ready();
    }

    ~LoopbackServer() {
        server_.stop();
        serving_.join();
    }

    LoopbackServer(const LoopbackServer &) = delete;
    LoopbackServer &operator=(const LoopbackServer &) = delete;

    uint16_t port() const {
        return static_cast<uint16_t>(port_);
    }

    std::vector<std::string> hosts() {
        const std::lock_guard<std::mutex> lock(mutex_);
        return hosts_;
    }

private:
    void seen(const httplib::Request &request) {
        const std::lock_guard<std::mutex> lock(mutex_);
        hosts_.push_back(request.get_header_value("Host"));
    }

    httplib::Server server_;
    int port_ = 0;
    std::thread serving_;
    std::mutex mutex_;
    std::vector<std::string> hosts_;
};

host::HttpRequest get(const std::string &url) {
    return {"GET", url, {}, ""};
}

/// A policy whose one entry is `http://127.0.0.1:<port>`, which may reach that address.
host::DestinationPolicy loopbackLiteral(uint16_t port) {
    return host::DestinationPolicy::allowList(
        {host::AllowedDestination(host::Scheme::Http, "127.0.0.1", port).reaching(network("127.0.0.1", 32))},
        std::make_shared<ScriptedResolver>(std::map<std::string, std::vector<std::vector<std::string>>>{}));
}

}  // namespace

TEST(AStandardHostCurlTransport, RefusesANameThatResolvesToLoopbackAndServesAnAllowListedOne) {
    LoopbackServer server;
    const uint16_t port = server.port();
    const std::string at = ":" + std::to_string(port);
    auto resolver = std::make_shared<ScriptedResolver>(std::map<std::string, std::vector<std::vector<std::string>>>{
        {"loopback.test", {{"127.0.0.1"}}},
        {"served.test", {{"127.0.0.1"}}},
    });
    host::CurlTransport transport(host::DestinationPolicy::allowList(
        {host::AllowedDestination(host::Scheme::Http, "loopback.test", port),
         host::AllowedDestination(host::Scheme::Http, "served.test", port).reaching(network("127.0.0.0", 8))},
        resolver));

    const host::Exchange refused = transport.exchange(get("http://loopback.test" + at + "/ok"));
    EXPECT_EQ(refused.kind, host::Exchange::Kind::DestinationRefused);
    EXPECT_TRUE(server.hosts().empty());

    const host::Exchange served = transport.exchange(get("http://served.test" + at + "/ok"));
    ASSERT_EQ(served.kind, host::Exchange::Kind::Response);
    EXPECT_EQ(served.response.status, 200);
    EXPECT_EQ(served.response.body, "served");
    bool replied = false;
    for (const auto &header : served.response.headers) {
        replied = replied || (header.name == "X-Reply" && header.value == "yes");
    }
    EXPECT_TRUE(replied);
    EXPECT_EQ(server.hosts(), std::vector<std::string>{"served.test" + at});
}

TEST(AStandardHostCurlTransport, RefusesATargetOffTheAllowListBeforeItsNameIsResolved) {
    LoopbackServer server;
    const uint16_t port = server.port();
    const std::string at = ":" + std::to_string(port);
    auto resolver = std::make_shared<ScriptedResolver>(std::map<std::string, std::vector<std::vector<std::string>>>{
        {"other.test", {{"127.0.0.1"}}},
    });
    host::CurlTransport transport(host::DestinationPolicy::allowList(
        {host::AllowedDestination(host::Scheme::Http, "served.test", port).reaching(network("127.0.0.0", 8))},
        resolver));
    EXPECT_EQ(transport.exchange(get("http://other.test" + at + "/ok")).kind, host::Exchange::Kind::DestinationRefused);
    EXPECT_EQ(transport.exchange(get("https://served.test" + at + "/ok")).kind,
              host::Exchange::Kind::DestinationRefused);
    EXPECT_TRUE(resolver->asked.empty());
    EXPECT_TRUE(server.hosts().empty());
}

/// A name that answers the address checked and then another is resolved once for the exchange, and the
/// connection is made to the first: were it resolved again to connect, it would reach `127.0.0.2`, where
/// nothing listens.
TEST(AStandardHostCurlTransport, ConnectsToTheAddressCheckedAndResolvesAnewForEachExchange) {
    LoopbackServer server;
    const uint16_t port = server.port();
    const std::string url = "http://rebinding.test:" + std::to_string(port) + "/ok";
    auto resolver = std::make_shared<ScriptedResolver>(std::map<std::string, std::vector<std::vector<std::string>>>{
        {"rebinding.test", {{"127.0.0.1"}, {"127.0.0.2"}}},
    });
    host::CurlTransport transport(host::DestinationPolicy::allowList(
        {host::AllowedDestination(host::Scheme::Http, "rebinding.test", port).reaching(network("127.0.0.1", 32))},
        resolver));

    const host::Exchange served = transport.exchange(get(url));
    ASSERT_EQ(served.kind, host::Exchange::Kind::Response);
    EXPECT_EQ(served.response.status, 200);
    EXPECT_EQ(resolver->asked, std::vector<std::string>{"rebinding.test"});

    // The next exchange asks again, and `127.0.0.2` is no address it may reach.
    EXPECT_EQ(transport.exchange(get(url)).kind, host::Exchange::Kind::DestinationRefused);
    EXPECT_EQ(resolver->asked, (std::vector<std::string>{"rebinding.test", "rebinding.test"}));
    EXPECT_EQ(server.hosts().size(), 1u);
}

/// The guard libcurl opens each socket through: an address the policy did not admit is refused, of either
/// family, whatever libcurl was told.
TEST(AStandardHostCurlTransport, OpensASocketOnlyToAnAdmittedAddress) {
    const std::vector<host::IpAddress> admitted{address("127.0.0.1"), address("::1")};
    struct sockaddr_in in4;
    std::memset(&in4, 0, sizeof in4);
    in4.sin_family = AF_INET;
    in4.sin_port = htons(80);
    in4.sin_addr.s_addr = htonl(0x7f000001u);
    EXPECT_TRUE(host::detail::socketAdmitted(reinterpret_cast<const struct sockaddr *>(&in4), admitted));
    in4.sin_addr.s_addr = htonl(0x7f000002u);
    EXPECT_FALSE(host::detail::socketAdmitted(reinterpret_cast<const struct sockaddr *>(&in4), admitted));
    struct sockaddr_in6 in6;
    std::memset(&in6, 0, sizeof in6);
    in6.sin6_family = AF_INET6;
    in6.sin6_port = htons(80);
    in6.sin6_addr.s6_addr[15] = 1;
    EXPECT_TRUE(host::detail::socketAdmitted(reinterpret_cast<const struct sockaddr *>(&in6), admitted));
    in6.sin6_addr.s6_addr[15] = 2;
    EXPECT_FALSE(host::detail::socketAdmitted(reinterpret_cast<const struct sockaddr *>(&in6), admitted));
}

TEST(AStandardHostCurlTransport, AnswersARedirectToAPrivateAddressAsThe3xxItIs) {
    LoopbackServer server;
    const uint16_t port = server.port();
    host::CurlTransport transport(loopbackLiteral(port));
    const host::Exchange moved = transport.exchange(get("http://127.0.0.1:" + std::to_string(port) + "/moved"));
    ASSERT_EQ(moved.kind, host::Exchange::Kind::Response);
    EXPECT_EQ(moved.response.status, 302);
    bool located = false;
    for (const auto &header : moved.response.headers) {
        located = located || (header.name == "Location" && header.value == "http://10.0.0.1/inside");
    }
    EXPECT_TRUE(located);
    EXPECT_EQ(server.hosts().size(), 1u);
}

TEST(AStandardHostCurlTransport, FailsABodyOrAHeadPastItsBoundAsTooLarge) {
    LoopbackServer server;
    const uint16_t port = server.port();
    const std::string base = "http://127.0.0.1:" + std::to_string(port);
    host::CurlTransportSettings settings;
    settings.maxResponseBytes = 64;
    settings.maxHeaderBytes = 8192;
    host::CurlTransport transport(loopbackLiteral(port), settings);
    EXPECT_EQ(transport.exchange(get(base + "/big")).kind, host::Exchange::Kind::ResponseTooLarge);
    EXPECT_EQ(transport.exchange(get(base + "/wide")).kind, host::Exchange::Kind::ResponseTooLarge);
    const host::Exchange fits = transport.exchange(get(base + "/ok"));
    ASSERT_EQ(fits.kind, host::Exchange::Kind::Response);
    EXPECT_EQ(fits.response.body, "served");
}

TEST(AStandardHostCurlTransport, FailsAResponsePastTheHeaderBoundAsNetwork) {
    LoopbackServer server;
    const uint16_t port = server.port();
    host::CurlTransportSettings settings;
    settings.timeouts.header = std::chrono::milliseconds(200);
    host::CurlTransport transport(loopbackLiteral(port), settings);
    const auto began = std::chrono::steady_clock::now();
    EXPECT_EQ(transport.exchange(get("http://127.0.0.1:" + std::to_string(port) + "/slow")).kind,
              host::Exchange::Kind::Failure);
    EXPECT_LT(std::chrono::steady_clock::now() - began, std::chrono::seconds(3));
}

TEST(AStandardHostCurlTransport, SendsNoRequestThatNamesAFieldTheTransportWrites) {
    LoopbackServer server;
    const uint16_t port = server.port();
    host::CurlTransport transport(loopbackLiteral(port));
    for (const char *field : {"Host", "Content-Length", "Transfer-Encoding"}) {
        host::HttpRequest request = get("http://127.0.0.1:" + std::to_string(port) + "/ok");
        request.headers = {{field, "1"}};
        EXPECT_EQ(transport.exchange(request).kind, host::Exchange::Kind::Failure) << field;
    }
    EXPECT_TRUE(server.hosts().empty());
}

/// The public internet is `https` at a public address: a name that resolves to loopback is refused before
/// a connection, and a plain `http` URL before its name is resolved.
TEST(AStandardHostCurlTransport, ThePublicInternetReachesNoLoopbackName) {
    auto resolver = std::make_shared<ScriptedResolver>(std::map<std::string, std::vector<std::vector<std::string>>>{
        {"loopback.test", {{"::ffff:127.0.0.1"}}},
    });
    host::CurlTransport transport(host::DestinationPolicy::publicInternet(resolver));
    EXPECT_EQ(transport.exchange(get("http://loopback.test/ok")).kind, host::Exchange::Kind::DestinationRefused);
    EXPECT_TRUE(resolver->asked.empty());
    EXPECT_EQ(transport.exchange(get("https://loopback.test/ok")).kind, host::Exchange::Kind::DestinationRefused);
    EXPECT_EQ(resolver->asked, std::vector<std::string>{"loopback.test"});
}

/// The reference host over this transport: a refused destination is answered with the operation's own
/// failure code, and an admitted one with the response.
TEST(AStandardHostCurlTransport, TheReferenceHostAnswersARefusedDestinationWithItsCode) {
    LoopbackServer server;
    const uint16_t port = server.port();
    auto resolver = std::make_shared<ScriptedResolver>(std::map<std::string, std::vector<std::vector<std::string>>>{
        {"loopback.test", {{"127.0.0.1"}}},
    });
    host::CurlTransport transport(host::DestinationPolicy::allowList(
        {host::AllowedDestination(host::Scheme::Http, "loopback.test", port),
         host::AllowedDestination(host::Scheme::Http, "127.0.0.1", port).reaching(network("127.0.0.1", 32))},
        resolver));
    host::ScriptedClock clock;
    host::ReferenceHost reference(clock, transport);
    const auto fetch = [&reference](const std::string &url) {
        const auto outcome = reference.start("sce-host-http-fetch", host::Invocation{"session", "fetch", 1},
                                             {{"method", "GET"}, {"url", url}, {"headers", ""}, {"body", ""}});
        EXPECT_TRUE(outcome && outcome->kind == host::Outcome::Kind::Done);
        return json::parse(outcome ? outcome->data : std::string("{}"));
    };
    const std::string at = ":" + std::to_string(port);
    EXPECT_EQ(fetch("http://loopback.test" + at + "/ok").at("error"), "destination_refused");
    const json served = fetch("http://127.0.0.1" + at + "/ok");
    EXPECT_EQ(served.at("error"), "");
    EXPECT_EQ(served.at("status"), 200);
    EXPECT_EQ(served.at("body"), "served");
}

}  // namespace SCE::Tests
