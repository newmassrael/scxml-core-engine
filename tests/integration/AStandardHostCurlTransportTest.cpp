// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reference host's network transport on libcurl (sce/include/host/CurlTransport.h) behind its
// destination policy (docs/adr/0109, D4), held against a cpp-httplib server on a loopback port of this
// process. The names are answered by a resolver the test scripts, so nothing reaches another machine: a
// name that resolves to loopback is refused before a byte is sent, one the allow-list lets reach loopback
// is served, a name is resolved once per exchange and the connection is made to the address checked, a
// redirect is answered as it came, and the bounds of time and size hold. Through a proxy of this process
// (POSIX sockets, so not on Windows), the target is still judged, the tunnel is opened to the address
// checked, the socket guard admits the proxy's address, and the proxy's credentials reach the proxy alone.
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
#include <netdb.h>
#include <netinet/in.h>
#include <poll.h>
#include <sys/socket.h>
#include <unistd.h>
#endif

#include <algorithm>
#include <atomic>
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

    /// How many requests carried a `Proxy-Authorization`.
    size_t proxyCredentialsSeen() {
        const std::lock_guard<std::mutex> lock(mutex_);
        return proxyCredentials_;
    }

private:
    void seen(const httplib::Request &request) {
        const std::lock_guard<std::mutex> lock(mutex_);
        hosts_.push_back(request.get_header_value("Host"));
        if (request.has_header("Proxy-Authorization")) {
            ++proxyCredentials_;
        }
    }

    httplib::Server server_;
    int port_ = 0;
    std::thread serving_;
    std::mutex mutex_;
    std::vector<std::string> hosts_;
    size_t proxyCredentials_ = 0;
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

#if !defined(_WIN32)

/// Send all of `size` bytes of `data` on `socket`; false when the peer is gone.
bool sendAll(int socket, const char *data, size_t size) {
#if defined(MSG_NOSIGNAL)
    const int flags = MSG_NOSIGNAL;
#else
    const int flags = 0;
#endif
    while (size > 0) {
        const ssize_t sent = ::send(socket, data, size, flags);
        if (sent <= 0) {
            return false;
        }
        data += sent;
        size -= static_cast<size_t>(sent);
    }
    return true;
}

bool sendAll(int socket, const std::string &text) {
    return sendAll(socket, text.data(), text.size());
}

/// Copy between `a` and `b` both ways until either ends or both are quiet for ten seconds.
void relay(int a, int b) {
    pollfd ends[2] = {{a, POLLIN, 0}, {b, POLLIN, 0}};
    char buffer[16384];
    while (::poll(ends, 2, 10000) > 0) {
        for (int i = 0; i < 2; ++i) {
            if ((ends[i].revents & (POLLIN | POLLHUP | POLLERR)) == 0) {
                continue;
            }
            const ssize_t got = ::recv(ends[i].fd, buffer, sizeof buffer, 0);
            if (got <= 0 || !sendAll(ends[1 - i].fd, buffer, static_cast<size_t>(got))) {
                return;
            }
        }
    }
}

/// An HTTP proxy on a loopback address that opens a `CONNECT` tunnel to the `address:port` it is asked for
/// and relays both ways, or, when `refusing`, answers `407`; and the head of every `CONNECT` it read.
class TunnellingProxy {
public:
    TunnellingProxy(const char *address, bool refusing) : refusing_(refusing), address_(address) {
        listener_ = ::socket(AF_INET, SOCK_STREAM, 0);
        const int yes = 1;
        ::setsockopt(listener_, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof yes);
        struct sockaddr_in at;
        std::memset(&at, 0, sizeof at);
        at.sin_family = AF_INET;
        ::inet_pton(AF_INET, address, &at.sin_addr);
        const bool listening =
            ::bind(listener_, reinterpret_cast<struct sockaddr *>(&at), sizeof at) == 0 && ::listen(listener_, 8) == 0;
        EXPECT_TRUE(listening) << address;
        socklen_t length = sizeof at;
        ::getsockname(listener_, reinterpret_cast<struct sockaddr *>(&at), &length);
        port_ = ntohs(at.sin_port);
        accepting_ = std::thread([this] { accept(); });
    }

    ~TunnellingProxy() {
        // A connection of its own wakes the accept, which then sees the stop; closing a listening socket
        // does not wake one on every platform.
        stopping_ = true;
        struct sockaddr_in at;
        std::memset(&at, 0, sizeof at);
        at.sin_family = AF_INET;
        at.sin_port = htons(port_);
        ::inet_pton(AF_INET, address_.c_str(), &at.sin_addr);
        const int waking = ::socket(AF_INET, SOCK_STREAM, 0);
        if (waking >= 0) {
            ::connect(waking, reinterpret_cast<struct sockaddr *>(&at), sizeof at);
            ::close(waking);
        }
        accepting_.join();
        ::close(listener_);
        for (auto &handler : handlers_) {
            handler.join();
        }
    }

    TunnellingProxy(const TunnellingProxy &) = delete;
    TunnellingProxy &operator=(const TunnellingProxy &) = delete;

    uint16_t port() const {
        return port_;
    }

    std::vector<std::string> heads() {
        const std::lock_guard<std::mutex> lock(mutex_);
        return heads_;
    }

private:
    void accept() {
        for (;;) {
            const int client = ::accept(listener_, nullptr, nullptr);
            if (client < 0) {
                return;
            }
            if (stopping_) {
                ::close(client);
                return;
            }
            handlers_.emplace_back([this, client] { handle(client); });
        }
    }

    void handle(int client) {
        std::string head;
        char byte = 0;
        while (head.size() < 8192 && (head.size() < 4 || head.compare(head.size() - 4, 4, "\r\n\r\n") != 0) &&
               ::recv(client, &byte, 1, 0) == 1) {
            head += byte;
        }
        {
            const std::lock_guard<std::mutex> lock(mutex_);
            heads_.push_back(head);
        }
        if (refusing_) {
            sendAll(client, "HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n");
            ::close(client);
            return;
        }
        // `CONNECT <address>:<port> HTTP/1.1`, an IPv6 address in brackets.
        const size_t begin = head.find(' ') + 1;
        const std::string target = head.substr(begin, head.find(' ', begin) - begin);
        const size_t colon = target.rfind(':');
        std::string name = target.substr(0, colon);
        if (name.size() >= 2 && name.front() == '[') {
            name = name.substr(1, name.size() - 2);
        }
        const std::string service = colon == std::string::npos ? "" : target.substr(colon + 1);
        struct addrinfo hints;
        std::memset(&hints, 0, sizeof hints);
        hints.ai_family = AF_UNSPEC;
        hints.ai_socktype = SOCK_STREAM;
        hints.ai_flags = AI_NUMERICHOST | AI_NUMERICSERV;
        struct addrinfo *found = nullptr;
        int upstream = -1;
        if (::getaddrinfo(name.c_str(), service.c_str(), &hints, &found) == 0) {
            upstream = ::socket(found->ai_family, found->ai_socktype, found->ai_protocol);
            if (upstream >= 0 && ::connect(upstream, found->ai_addr, found->ai_addrlen) != 0) {
                ::close(upstream);
                upstream = -1;
            }
            ::freeaddrinfo(found);
        }
        if (upstream < 0) {
            sendAll(client, "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n");
            ::close(client);
            return;
        }
        if (sendAll(client, "HTTP/1.1 200 Connection established\r\n\r\n")) {
            relay(client, upstream);
        }
        ::close(upstream);
        ::close(client);
    }

    bool refusing_;
    std::string address_;
    std::atomic<bool> stopping_{false};
    int listener_ = -1;
    uint16_t port_ = 0;
    std::thread accepting_;
    std::vector<std::thread> handlers_;
    std::mutex mutex_;
    std::vector<std::string> heads_;
};

/// The proxy's own address: on Linux one of loopback the target server is not on, so that a socket guard
/// admitting the target's address instead of the proxy's would refuse the connection.
#if defined(__linux__)
constexpr const char *PROXY_ADDRESS = "127.0.0.2";
#else
constexpr const char *PROXY_ADDRESS = "127.0.0.1";
#endif

host::CurlTransportSettings through(const TunnellingProxy &proxy) {
    host::CurlTransportSettings settings;
    settings.proxy = host::CurlProxy{PROXY_ADDRESS, proxy.port(), "Basic dTpw"};
    return settings;
}

#endif

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

#if !defined(_WIN32)

/// Through a proxy the target is judged as it is without one, the proxy's own address is not (no entry
/// names it), the tunnel is opened to the address the policy checked and not to the name, the socket guard
/// admits the proxy's address, and the proxy's credentials reach the proxy alone.
TEST(AStandardHostCurlTransport, ThroughAProxyJudgesTheTargetAndTunnelsToTheAddressChecked) {
    LoopbackServer server;
    const uint16_t port = server.port();
    const std::string at = ":" + std::to_string(port);
    TunnellingProxy proxy(PROXY_ADDRESS, false);
    auto resolver = std::make_shared<ScriptedResolver>(std::map<std::string, std::vector<std::vector<std::string>>>{
        {"loopback.test", {{"127.0.0.1"}}},
        {"served.test", {{"127.0.0.1"}}},
    });
    host::CurlTransport transport(
        host::DestinationPolicy::allowList(
            {host::AllowedDestination(host::Scheme::Http, "loopback.test", port),
             host::AllowedDestination(host::Scheme::Http, "served.test", port).reaching(network("127.0.0.0", 8))},
            resolver),
        through(proxy));

    EXPECT_EQ(transport.exchange(get("http://loopback.test" + at + "/ok")).kind,
              host::Exchange::Kind::DestinationRefused);
    EXPECT_EQ(transport.exchange(get("http://other.test" + at + "/ok")).kind, host::Exchange::Kind::DestinationRefused);
    EXPECT_TRUE(proxy.heads().empty());

    const host::Exchange served = transport.exchange(get("http://served.test" + at + "/ok"));
    ASSERT_EQ(served.kind, host::Exchange::Kind::Response);
    EXPECT_EQ(served.response.status, 200);
    EXPECT_EQ(served.response.body, "served");
    const std::vector<std::string> tunnels = proxy.heads();
    ASSERT_EQ(tunnels.size(), 1u);
    EXPECT_EQ(tunnels[0].rfind("CONNECT 127.0.0.1" + at + " HTTP/1.1\r\n", 0), 0u) << tunnels[0];
    EXPECT_NE(tunnels[0].find("Proxy-Authorization: Basic dTpw\r\n"), std::string::npos) << tunnels[0];
    EXPECT_EQ(server.hosts(), std::vector<std::string>{"served.test" + at});
    EXPECT_EQ(server.proxyCredentialsSeen(), 0u);
}

TEST(AStandardHostCurlTransport, FailsAsNetworkWhenTheProxyRefusesTheTunnel) {
    LoopbackServer server;
    const uint16_t port = server.port();
    TunnellingProxy proxy(PROXY_ADDRESS, true);
    host::CurlTransport transport(loopbackLiteral(port), through(proxy));
    EXPECT_EQ(transport.exchange(get("http://127.0.0.1:" + std::to_string(port) + "/ok")).kind,
              host::Exchange::Kind::Failure);
    EXPECT_EQ(proxy.heads().size(), 1u);
    EXPECT_TRUE(server.hosts().empty());
}

#endif

}  // namespace SCE::Tests
