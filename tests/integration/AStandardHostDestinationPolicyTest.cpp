// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The destination policy of a server's HTTP transport (sce/include/host/DestinationPolicy.h;
// docs/adr/0109, D4): the special-purpose ranges of RFC 6890 range by range, the IPv6 forms that carry
// an IPv4 address judged by the address they carry, the allow-list and the public internet, with names
// answered by a resolver the test scripts; and the reference host answering each failure a transport
// reports with its code. Needs no network library, so it runs wherever standard_host_scenario_test does.

#include "host/DestinationPolicy.h"
#include "host/StandardHost.h"

#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
using host::SpecialPurpose;

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

/// A resolver that answers from a table and records what it was asked.
class TableResolver : public host::Resolver {
public:
    explicit TableResolver(std::map<std::string, std::vector<std::string>> answers) : answers_(std::move(answers)) {}

    std::optional<std::vector<host::IpAddress>> resolve(const std::string &name) override {
        asked.push_back(name);
        const auto found = answers_.find(name);
        if (found == answers_.end()) {
            return std::nullopt;
        }
        std::vector<host::IpAddress> addresses;
        for (const auto &text : found->second) {
            addresses.push_back(address(text));
        }
        return addresses;
    }

    std::vector<std::string> asked;

private:
    std::map<std::string, std::vector<std::string>> answers_;
};

std::vector<std::string> texts(const std::vector<host::IpAddress> &addresses) {
    std::vector<std::string> out;
    for (const auto &each : addresses) {
        out.push_back(each.text());
    }
    return out;
}

}  // namespace

/// RFC 6890 and the registries it set up, range by range, with the addresses at and just past the edge of
/// each, and every IPv6 form that carries an IPv4 address judged by the address it carries. The same
/// table as the Rust policy's (backends/rust/host-features/src/destination.rs).
TEST(AStandardHostDestinationPolicy, ClassifiesEverySpecialPurposeRange) {
    const std::vector<std::pair<std::string, std::optional<SpecialPurpose>>> table = {
        {"0.0.0.0", SpecialPurpose::Unspecified},
        {"0.1.2.3", SpecialPurpose::ThisNetwork},
        {"0.255.255.255", SpecialPurpose::ThisNetwork},
        {"1.0.0.0", std::nullopt},
        {"10.0.0.1", SpecialPurpose::Private},
        {"10.255.255.255", SpecialPurpose::Private},
        {"11.0.0.0", std::nullopt},
        {"100.63.255.255", std::nullopt},
        {"100.64.0.0", SpecialPurpose::Shared},
        {"100.127.255.255", SpecialPurpose::Shared},
        {"100.128.0.0", std::nullopt},
        {"126.255.255.255", std::nullopt},
        {"127.0.0.1", SpecialPurpose::Loopback},
        {"127.255.255.254", SpecialPurpose::Loopback},
        {"128.0.0.0", std::nullopt},
        {"169.254.169.254", SpecialPurpose::LinkLocal},
        {"169.255.0.0", std::nullopt},
        {"172.15.255.255", std::nullopt},
        {"172.16.0.1", SpecialPurpose::Private},
        {"172.31.255.255", SpecialPurpose::Private},
        {"172.32.0.0", std::nullopt},
        {"192.0.0.8", SpecialPurpose::ProtocolAssignments},
        {"192.0.1.0", std::nullopt},
        {"192.0.2.1", SpecialPurpose::Documentation},
        {"192.88.99.1", SpecialPurpose::Reserved},
        {"192.167.255.255", std::nullopt},
        {"192.168.1.1", SpecialPurpose::Private},
        {"192.169.0.0", std::nullopt},
        {"198.17.255.255", std::nullopt},
        {"198.18.0.1", SpecialPurpose::Benchmarking},
        {"198.19.255.255", SpecialPurpose::Benchmarking},
        {"198.20.0.0", std::nullopt},
        {"198.51.100.7", SpecialPurpose::Documentation},
        {"203.0.113.9", SpecialPurpose::Documentation},
        {"223.255.255.255", std::nullopt},
        {"224.0.0.1", SpecialPurpose::Multicast},
        {"239.255.255.255", SpecialPurpose::Multicast},
        {"240.0.0.1", SpecialPurpose::Reserved},
        {"255.255.255.254", SpecialPurpose::Reserved},
        {"255.255.255.255", SpecialPurpose::Broadcast},
        {"8.8.8.8", std::nullopt},
        {"93.184.215.14", std::nullopt},
        {"::", SpecialPurpose::Unspecified},
        {"::1", SpecialPurpose::Loopback},
        {"::2", SpecialPurpose::ThisNetwork},
        {"::ffff:127.0.0.1", SpecialPurpose::Loopback},
        {"::ffff:10.1.2.3", SpecialPurpose::Private},
        {"::ffff:169.254.169.254", SpecialPurpose::LinkLocal},
        {"::ffff:8.8.8.8", std::nullopt},
        {"::ffff:0:127.0.0.1", SpecialPurpose::Loopback},
        {"::ffff:0:8.8.8.8", std::nullopt},
        {"::127.0.0.1", SpecialPurpose::Loopback},
        {"::8.8.8.8", std::nullopt},
        {"64:ff9b::7f00:1", SpecialPurpose::Loopback},
        {"64:ff9b::a9fe:a9fe", SpecialPurpose::LinkLocal},
        {"64:ff9b::c0a8:101", SpecialPurpose::Private},
        {"64:ff9b::808:808", std::nullopt},
        {"64:ff9b:1::1", SpecialPurpose::LocalTranslation},
        {"64:ff9b:1:ffff::1", SpecialPurpose::LocalTranslation},
        {"2002:7f00:1::1", SpecialPurpose::Loopback},
        {"2002:c0a8:101::", SpecialPurpose::Private},
        {"2002:a00:1::", SpecialPurpose::Private},
        {"2002:808:808::1", std::nullopt},
        {"100::1", SpecialPurpose::Discard},
        {"100::ffff:ffff:ffff:ffff", SpecialPurpose::Discard},
        {"100:0:0:1::", SpecialPurpose::Reserved},
        {"2001::1", SpecialPurpose::ProtocolAssignments},
        {"2001:1ff:ffff::1", SpecialPurpose::ProtocolAssignments},
        {"2001:200::1", std::nullopt},
        {"2001:db8::1", SpecialPurpose::Documentation},
        {"2001:db9::1", std::nullopt},
        {"3fff::1", SpecialPurpose::Documentation},
        {"3fff:fff::1", SpecialPurpose::Documentation},
        {"3fff:1000::1", std::nullopt},
        {"fc00::1", SpecialPurpose::UniqueLocal},
        {"fdff:ffff::1", SpecialPurpose::UniqueLocal},
        {"fe80::1", SpecialPurpose::LinkLocal},
        {"febf::1", SpecialPurpose::LinkLocal},
        {"fec0::1", SpecialPurpose::Reserved},
        {"ff02::1", SpecialPurpose::Multicast},
        {"5f00::1", SpecialPurpose::Reserved},
        {"4000::1", SpecialPurpose::Reserved},
        {"1fff:ffff::1", SpecialPurpose::Reserved},
        {"2606:4700:4700::1111", std::nullopt},
        {"2001:4860:4860::8888", std::nullopt},
    };
    for (const auto &[text, expected] : table) {
        EXPECT_EQ(host::specialPurpose(address(text)), expected) << text;
    }
}

TEST(AStandardHostDestinationPolicy, ReadsAndWritesAddressLiterals) {
    const std::vector<std::pair<std::string, std::string>> canonical = {
        {"127.0.0.1", "127.0.0.1"},
        {"::", "::"},
        {"::1", "::1"},
        {"1::", "1::"},
        {"2606:4700:4700:0:0:0:0:1111", "2606:4700:4700::1111"},
        {"2001:DB8::1", "2001:db8::1"},
        {"2001:db8:0:0:1:0:0:1", "2001:db8::1:0:0:1"},
        {"::ffff:127.0.0.1", "::ffff:7f00:1"},
        {"1:2:3:4:5:6:7:8", "1:2:3:4:5:6:7:8"},
        {"1:0:3:4:5:6:7:8", "1:0:3:4:5:6:7:8"},
    };
    for (const auto &[text, written] : canonical) {
        EXPECT_EQ(address(text).text(), written) << text;
    }
    for (const char *notOne :
         {"", "1.2.3", "1.2.3.4.5", "256.0.0.1", "01.2.3.4", "1.2.3.4 ", "example.com", ":::", "1::2::3",
          "1:2:3:4:5:6:7:8:9", "1:2:3:4:5:6:7", "12345::", "fe80::1%eth0", "1.2.3.4::", "::g", "[::1]"}) {
        EXPECT_FALSE(host::IpAddress::parse(notOne).has_value()) << notOne;
    }
}

TEST(AStandardHostDestinationPolicy, ANetworkHoldsWhatItsPrefixCoversAndNothingOfTheOtherFamily) {
    const host::IpNetwork loopback = network("127.0.0.0", 8);
    EXPECT_TRUE(loopback.contains(address("127.1.2.3")));
    EXPECT_FALSE(loopback.contains(address("128.0.0.1")));
    EXPECT_FALSE(loopback.contains(address("::ffff:127.0.0.1")));
    const host::IpNetwork one = network("10.0.0.7", 32);
    EXPECT_TRUE(one.contains(address("10.0.0.7")));
    EXPECT_FALSE(one.contains(address("10.0.0.8")));
    const host::IpNetwork everything = network("::", 0);
    EXPECT_TRUE(everything.contains(address("2606:4700::1")));
    EXPECT_FALSE(everything.contains(address("1.1.1.1")));
    EXPECT_FALSE(host::IpNetwork::make(address("10.0.0.0"), 33).has_value());
    EXPECT_FALSE(host::IpNetwork::make(address("::"), 129).has_value());
}

TEST(AStandardHostDestinationPolicy, RefusesATargetOffTheAllowListBeforeItsNameIsResolved) {
    const auto resolver = std::make_shared<TableResolver>(std::map<std::string, std::vector<std::string>>{
        {"api.example.com", {"93.184.215.14"}},
    });
    const auto policy = host::DestinationPolicy::allowList(
        {host::AllowedDestination(host::Scheme::Https, "api.example.com", 443)}, resolver);
    using Verdict = host::Admission::Verdict;
    EXPECT_EQ(policy.admit("https", "other.example.com", 443).verdict, Verdict::NotAllowed);
    EXPECT_EQ(policy.admit("http", "api.example.com", 443).verdict, Verdict::NotAllowed);
    EXPECT_EQ(policy.admit("https", "api.example.com", 8443).verdict, Verdict::NotAllowed);
    EXPECT_EQ(policy.admit("ftp", "api.example.com", 443).verdict, Verdict::NotAllowed);
    EXPECT_TRUE(resolver->asked.empty());
    const host::Admission admitted = policy.admit("https", "API.Example.com.", 443);
    ASSERT_EQ(admitted.verdict, Verdict::Admitted);
    EXPECT_EQ(admitted.destination.host, "api.example.com");
    EXPECT_EQ(admitted.destination.port, 443);
    EXPECT_EQ(texts(admitted.destination.addresses), std::vector<std::string>{"93.184.215.14"});
    EXPECT_EQ(resolver->asked, std::vector<std::string>{"api.example.com"});
}

TEST(AStandardHostDestinationPolicy, APatternBelowANameAdmitsItsSubdomainsAndNotTheName) {
    const auto resolver = std::make_shared<TableResolver>(std::map<std::string, std::vector<std::string>>{
        {"a.example.com", {"93.184.215.14"}},
        {"a.b.example.com", {"93.184.215.14"}},
        {"example.com", {"93.184.215.14"}},
        {"badexample.com", {"93.184.215.14"}},
    });
    const auto policy = host::DestinationPolicy::allowList(
        {host::AllowedDestination(host::Scheme::Https, "*.example.com", 443)}, resolver);
    using Verdict = host::Admission::Verdict;
    EXPECT_EQ(policy.admit("https", "a.example.com", 443).verdict, Verdict::Admitted);
    EXPECT_EQ(policy.admit("https", "a.b.example.com", 443).verdict, Verdict::Admitted);
    EXPECT_EQ(policy.admit("https", "example.com", 443).verdict, Verdict::NotAllowed);
    EXPECT_EQ(policy.admit("https", "badexample.com", 443).verdict, Verdict::NotAllowed);
}

TEST(AStandardHostDestinationPolicy, RefusesAnAllowListedNameThatResolvesToASpecialPurposeAddress) {
    const auto resolver = std::make_shared<TableResolver>(std::map<std::string, std::vector<std::string>>{
        {"internal.example.com", {"127.0.0.1", "::ffff:10.0.0.1"}},
        {"reached.example.com", {"10.0.3.7"}},
    });
    const auto policy = host::DestinationPolicy::allowList(
        {host::AllowedDestination(host::Scheme::Http, "internal.example.com", 80),
         host::AllowedDestination(host::Scheme::Http, "reached.example.com", 8080).reaching(network("10.0.0.0", 8))},
        resolver);
    const host::Admission refused = policy.admit("http", "internal.example.com", 80);
    ASSERT_EQ(refused.verdict, host::Admission::Verdict::AddressesRefused);
    ASSERT_EQ(refused.refused.size(), 2u);
    EXPECT_EQ(refused.refused[0].first.text(), "127.0.0.1");
    EXPECT_EQ(refused.refused[0].second, SpecialPurpose::Loopback);
    EXPECT_EQ(refused.refused[1].second, SpecialPurpose::Private);
    EXPECT_EQ(refused.failure(), host::Exchange::Kind::DestinationRefused);
    const host::Admission reached = policy.admit("http", "reached.example.com", 8080);
    ASSERT_EQ(reached.verdict, host::Admission::Verdict::Admitted);
    EXPECT_EQ(texts(reached.destination.addresses), std::vector<std::string>{"10.0.3.7"});
}

TEST(AStandardHostDestinationPolicy, AdmitsOnlyThePublicAddressesOfANameThatResolvesToBoth) {
    const auto resolver = std::make_shared<TableResolver>(std::map<std::string, std::vector<std::string>>{
        {"mixed.example.com", {"10.0.0.1", "93.184.215.14", "::1"}},
    });
    const auto policy = host::DestinationPolicy::publicInternet(resolver);
    const host::Admission admitted = policy.admit("https", "mixed.example.com", 443);
    ASSERT_EQ(admitted.verdict, host::Admission::Verdict::Admitted);
    EXPECT_EQ(texts(admitted.destination.addresses), std::vector<std::string>{"93.184.215.14"});
}

TEST(AStandardHostDestinationPolicy, ThePublicInternetIsHttpsAtAPublicAddressOnly) {
    const auto resolver = std::make_shared<TableResolver>(std::map<std::string, std::vector<std::string>>{
        {"public.example.com", {"2606:4700:4700::1111"}},
        {"rebound.example.com", {"64:ff9b::7f00:1"}},
    });
    const auto policy = host::DestinationPolicy::publicInternet(resolver);
    EXPECT_EQ(policy.admit("http", "public.example.com", 80).verdict, host::Admission::Verdict::NotAllowed);
    EXPECT_TRUE(resolver->asked.empty());
    EXPECT_EQ(policy.admit("https", "public.example.com", 8443).verdict, host::Admission::Verdict::Admitted);
    const host::Admission rebound = policy.admit("https", "rebound.example.com", 443);
    ASSERT_EQ(rebound.verdict, host::Admission::Verdict::AddressesRefused);
    ASSERT_EQ(rebound.refused.size(), 1u);
    EXPECT_EQ(rebound.refused[0].second, SpecialPurpose::Loopback);
}

TEST(AStandardHostDestinationPolicy, JudgesAnAddressLiteralWithoutResolvingIt) {
    const auto resolver = std::make_shared<TableResolver>(std::map<std::string, std::vector<std::string>>{});
    const auto policy = host::DestinationPolicy::publicInternet(resolver);
    const host::Admission linkLocal = policy.admit("https", "[::ffff:169.254.169.254]", 443);
    ASSERT_EQ(linkLocal.verdict, host::Admission::Verdict::AddressesRefused);
    EXPECT_EQ(linkLocal.refused[0].second, SpecialPurpose::LinkLocal);
    const host::Admission literal = policy.admit("https", "[2606:4700:4700::1111]", 443);
    ASSERT_EQ(literal.verdict, host::Admission::Verdict::Admitted);
    EXPECT_EQ(literal.destination.host, "2606:4700:4700::1111");
    EXPECT_EQ(policy.admit("https", "127.0.0.1", 443).verdict, host::Admission::Verdict::AddressesRefused);
    EXPECT_TRUE(resolver->asked.empty());
}

TEST(AStandardHostDestinationPolicy, ANameThatDoesNotResolveIsANetworkFailure) {
    const auto resolver = std::make_shared<TableResolver>(std::map<std::string, std::vector<std::string>>{
        {"empty.example.com", {}},
    });
    const auto policy = host::DestinationPolicy::publicInternet(resolver);
    const host::Admission missing = policy.admit("https", "missing.example.com", 443);
    EXPECT_EQ(missing.verdict, host::Admission::Verdict::Unresolved);
    EXPECT_EQ(missing.failure(), host::Exchange::Kind::Failure);
    EXPECT_EQ(policy.admit("https", "empty.example.com", 443).verdict, host::Admission::Verdict::Unresolved);
    host::Admission notAllowed;
    notAllowed.verdict = host::Admission::Verdict::NotAllowed;
    EXPECT_EQ(notAllowed.failure(), host::Exchange::Kind::DestinationRefused);
}

/// docs/SCE_HOST_FEATURES.md, section 5: each failure a transport reports is the code of the operation's
/// closed set it names, with every other field at its zero (docs/adr/0109, D4, for `destination_refused`).
TEST(AStandardHostDestinationPolicy, TheReferenceHostAnswersEachTransportFailureWithItsCode) {
    struct Failing : host::Transport {
        host::Exchange::Kind kind = host::Exchange::Kind::Failure;

        host::Exchange exchange(const host::HttpRequest &) override {
            return {kind, {}};
        }
    };

    const std::vector<std::pair<host::Exchange::Kind, std::string>> cases = {
        {host::Exchange::Kind::Failure, "network"},
        {host::Exchange::Kind::DestinationRefused, "destination_refused"},
        {host::Exchange::Kind::ResponseTooLarge, "response_too_large"},
    };
    for (const auto &[kind, code] : cases) {
        host::ScriptedClock clock;
        Failing transport;
        transport.kind = kind;
        host::ReferenceHost reference(clock, transport);
        const auto outcome = reference.start(
            "sce-host-http-fetch", host::Invocation{"session", "fetch", 1},
            {{"method", "GET"}, {"url", "https://api.example.com/items"}, {"headers", ""}, {"body", ""}});
        ASSERT_TRUE(outcome && outcome->kind == host::Outcome::Kind::Done) << code;
        const json answered = json::parse(outcome->data);
        EXPECT_EQ(answered.at("error"), code);
        EXPECT_EQ(answered.at("status"), 0) << code;
        EXPECT_EQ(answered.at("body"), "") << code;
    }
}

}  // namespace SCE::Tests
