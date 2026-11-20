// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the C++ reference host reports through the metrics port (docs/adr/0109,
// D16; sce/include/host/Metrics.h): how long each start of an operation took to
// answer, by its type, and the code of each failure and refusal, through the
// recording implementation; and the port's closed sets, value for value, as
// the Rust crate `sce-rust-metrics` holds them.

#include "host/Metrics.h"
#include "host/StandardHost.h"

#include <gtest/gtest.h>

#include <map>
#include <string>

namespace {

using SCE::Metrics::Counter;
using SCE::Metrics::Histogram;
using SCE::Metrics::HostFailure;
using SCE::Metrics::HostOperation;
using SCE::Metrics::Recording;
using SCE::StandardHost::Exchange;
using SCE::StandardHost::HttpRequest;
using SCE::StandardHost::Invocation;
using SCE::StandardHost::Outcome;
using SCE::StandardHost::Params;
using SCE::StandardHost::ReferenceHost;
using SCE::StandardHost::ScriptedClock;
using SCE::StandardHost::Transport;

/// A transport whose every exchange gets no response.
class Unreachable : public Transport {
public:
    Exchange exchange(const HttpRequest &) override {
        return Exchange{Exchange::Kind::Failure, {}};
    }
};

/// A transport that holds every exchange: the host answers later, or never.
class Holding : public Transport {
public:
    Exchange exchange(const HttpRequest &) override {
        return Exchange{Exchange::Kind::Hold, {}};
    }
};

std::uint64_t failures(const Recording &recording, HostOperation operation, HostFailure code) {
    return recording.counted(Counter::hostOperationFailures(operation, code));
}

// Every type the reference host serves itself is one operation of the
// registry's closed set, and each operation's type reads back to it. Red if a
// type is added to TYPES and not to the registry.
TEST(AStandardHostReportsToItsMetrics, EveryTypeItServesIsOneOperationOfTheRegistry) {
    for (const char *type : SCE::StandardHost::TYPES) {
        EXPECT_TRUE(SCE::Metrics::hostOperationOf(type).has_value()) << type;
    }
    for (std::size_t i = 0; i < SCE::Metrics::kHostOperationTypes.size(); ++i) {
        const auto operation = static_cast<HostOperation>(i);
        EXPECT_EQ(SCE::Metrics::hostOperationOf(SCE::Metrics::value(operation)), operation);
    }
    EXPECT_FALSE(SCE::Metrics::hostOperationOf("phone").has_value());
    for (std::size_t i = 0; i < SCE::Metrics::kHostFailureCodes.size(); ++i) {
        const auto code = static_cast<HostFailure>(i);
        EXPECT_EQ(SCE::Metrics::hostFailureOf(SCE::Metrics::value(code)), code);
    }
    EXPECT_EQ(SCE::Metrics::hostFailureOf("a client's name"), HostFailure::Other);
    EXPECT_STREQ(SCE::Metrics::kRegistry, "sce-metrics-1");
}

// A series carries the labels its family fixes and no other, each a value of a
// closed set. Red if a factory hands a label of another family.
TEST(AStandardHostReportsToItsMetrics, ASeriesCarriesTheLabelsItsFamilyFixes) {
    const auto failure = Counter::hostOperationFailures(HostOperation::KvGet, HostFailure::Unavailable);
    EXPECT_STREQ(failure.family(), "sce_host_operation_failures");
    const auto labels = failure.labels();
    ASSERT_EQ(labels.size, 2u);
    EXPECT_STREQ(labels.pairs[0].name, "type");
    EXPECT_STREQ(labels.pairs[0].value, "sce-host-kv-get");
    EXPECT_STREQ(labels.pairs[1].name, "code");
    EXPECT_STREQ(labels.pairs[1].value, "unavailable");

    const auto exchanges = Counter::clientExchanges(SCE::Metrics::ExchangeStatus::NoResponse);
    EXPECT_STREQ(exchanges.family(), "sce_client_exchanges");
    ASSERT_EQ(exchanges.labels().size, 1u);
    EXPECT_STREQ(exchanges.labels().pairs[0].value, "none");
    EXPECT_STREQ(Counter::clientExchanges(SCE::Metrics::exchangeStatusOf(502)).labels().pairs[0].value, "other");
    EXPECT_STREQ(Counter::clientExchanges(SCE::Metrics::exchangeStatusOf(429)).labels().pairs[0].value, "429");

    EXPECT_STREQ(Counter::auditAppends().family(), "sce_host_audit_appends");
    EXPECT_EQ(Counter::auditAppends().labels().size, 0u);
    const auto duration = Histogram::hostOperationDuration(HostOperation::AuditAppend);
    EXPECT_STREQ(duration.family(), "sce_host_operation_duration_seconds");
    EXPECT_STREQ(duration.labels().pairs[0].value, "sce-host-audit-append");
}

// A start answered at once is measured once under its type and counts no
// failure; a refusal counts as invalid_argument; an injected failure under its
// own code, and one no operation names as other; a failure the operation met
// under its code. Red if ReferenceHost::report reads the code from anywhere but
// the answer, or measures what it does not answer.
TEST(AStandardHostReportsToItsMetrics, EachStartIsMeasuredAndEachFailureCountedByItsCode) {
    ScriptedClock clock;
    Unreachable transport;
    ReferenceHost host(clock, transport);
    Recording recording;
    host.setMetrics(recording);

    host.start("sce-host-kv-put", Invocation{"s", "put", 1}, Params{{"key", "k"}, {"value", "v"}});
    const auto measured = recording.observed(Histogram::hostOperationDuration(HostOperation::KvPut));
    ASSERT_EQ(measured.size(), 1u);
    EXPECT_GE(measured[0], 0.0);
    EXPECT_EQ(recording.countedIn("sce_host_operation_failures"), 0u) << "a put that completed is no failure";

    const auto refused = host.start("sce-host-kv-get", Invocation{"s", "get", 1}, Params{{"key", ""}});
    ASSERT_TRUE(refused.has_value());
    EXPECT_EQ(refused->kind, Outcome::Kind::Refuse);
    EXPECT_EQ(failures(recording, HostOperation::KvGet, HostFailure::InvalidArgument), 1u);

    host.setFaults({{"sce-host-kv-get", "unavailable"}, {"sce-host-kv-delete", "a code of its own"}});
    host.start("sce-host-kv-get", Invocation{"s", "get", 2}, Params{{"key", "k"}});
    host.start("sce-host-kv-delete", Invocation{"s", "delete", 1}, Params{{"key", "k"}});
    EXPECT_EQ(failures(recording, HostOperation::KvGet, HostFailure::Unavailable), 1u);
    EXPECT_EQ(failures(recording, HostOperation::KvDelete, HostFailure::Other), 1u);
    host.setFaults({});

    const Params fetch{{"method", "GET"}, {"url", "https://example.com/"}, {"headers", ""}, {"body", ""}};
    host.start("sce-host-http-fetch", Invocation{"s", "fetch", 1}, fetch);
    EXPECT_EQ(failures(recording, HostOperation::HttpFetch, HostFailure::Network), 1u);
    EXPECT_EQ(recording.observations(), 5u) << "each start answered at once is measured once";
}

// A start the host holds is answered later or never: it is not measured and
// counts no failure; a type the host does not serve is reported as nothing.
// Red if ReferenceHost::report measures a held start.
TEST(AStandardHostReportsToItsMetrics, AHeldStartIsNotMeasured) {
    ScriptedClock clock;
    Holding transport;
    ReferenceHost host(clock, transport);
    Recording recording;
    host.setMetrics(recording);
    const Params fetch{{"method", "GET"}, {"url", "https://example.com/"}, {"headers", ""}, {"body", ""}};
    const auto held = host.start("sce-host-http-fetch", Invocation{"s", "fetch", 1}, fetch);
    ASSERT_TRUE(held.has_value());
    EXPECT_EQ(held->kind, Outcome::Kind::Hold);
    EXPECT_FALSE(host.start("sce-host-not-a-type", Invocation{"s", "other", 1}, Params{}).has_value());
    EXPECT_EQ(recording.observations(), 0u);
    EXPECT_EQ(recording.countedIn("sce_host_operation_failures"), 0u);
}

// The first member of a record the host writes is what a failure's code is
// read from, and a record that opens with another member names none.
TEST(AStandardHostReportsToItsMetrics, AFailureCodeIsReadFromTheRecordsFirstMember) {
    namespace detail = SCE::StandardHost::detail;
    EXPECT_EQ(detail::leadingText(detail::failure("sce-host-kv-get", "unavailable").data, "error"), "unavailable");
    EXPECT_EQ(detail::leadingText(detail::record({detail::text("state", "granted")}).data, "error"), "");
    EXPECT_EQ(detail::leadingText(R"({"reason":"invalid_argument"})", "reason"), "invalid_argument");
    EXPECT_EQ(detail::leadingText(R"({"state":"x","error":"unavailable"})", "error"), "");
}

}  // namespace
