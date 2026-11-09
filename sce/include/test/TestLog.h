// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The test log of a test case run (docs/SCE_TEST_PROFILE.md §12.5;
// docs/adr/0091, C3 and answer 8): the records of TCI's test logging
// interface (ETSI ES 201 873-6), each written where the profile's semantics
// places its event, in the order the round scheduler produces them, and the
// one canonical text of a run's log, `schemas/sce-test-log.v1.schema.json`.
//
// The text is the JSON Canonicalization Scheme of the log
// (SCE::Saved::writeCanonical), and it holds no number: an instant, a duration
// and every integer of a value are texts of their digits (test/Canonical.h).
// So two runs agree, under C3, exactly when their logs are the same bytes,
// whichever runtime wrote each. The Rust sibling is
// `backends/rust/runtime/src/test/test_log.rs`, record for record.

#pragma once

#include <optional>
#include <string>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

#include "saved/SavedJson.h"
#include "test/Canonical.h"
#include "test/Port.h"
#include "test/Verdict.h"

namespace SCE::Test {

/// The name of the test log's shape, its `schema`.
inline constexpr const char *TEST_LOG_SCHEMA = "sce-test-log.v1";

/// The stability status of the test log's wire schema, pinned to the schema
/// file's own `x-sce-schema-status` by
/// `TheTestLog.TheSchemaFileDeclaresTheStatusThisRuntimeDoes`
/// (tests/test_runtime/TestLogTest.cpp).
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
inline constexpr const char *TEST_LOG_SCHEMA_STATUS = "pre-release";

/// The events of TCI's test logging interface (ES 201 873-6), as the profile's
/// semantics places them (docs/SCE_TEST_PROFILE.md §12.5), each with its
/// record's `kind`, the TCI event in kebab case. A component is named `mtc` or
/// by the PTC's name in its configuration.
namespace TestLog {

/// The test case started, at `NOW = 0` (P11).
struct TestCaseStarted {
    static constexpr const char *KIND = "test-case-started";
};

/// The test case ended with its verdict (P14).
struct TestCaseEnded {
    static constexpr const char *KIND = "test-case-ended";
    Verdict verdict;
};

/// A component was created: the MTC as the test case starts, a PTC by the
/// invocation that creates it (P9).
struct ComponentCreated {
    static constexpr const char *KIND = "component-created";
    std::string component;
    std::string componentType;
};

/// A component terminated, with its local verdict (P10).
struct ComponentDone {
    static constexpr const char *KIND = "component-done";
    std::string component;
    Verdict verdict;
};

/// A component sent a message on a port (P6).
struct MessageSent {
    static constexpr const char *KIND = "message-sent";
    std::string component;
    std::string port;
    /// The name of the message's record type (MessageValue::typeName()).
    std::string messageType;
    /// The message's value in canonical form.
    SCE::Saved::JsonValue value;
};

/// A receive branch took the head of a port's queue (P4 step 1): the message
/// was received and matched.
struct MessageReceived {
    static constexpr const char *KIND = "message-received";
    std::string component;
    std::string port;
    /// The name of the message's record type (MessageValue::typeName()).
    std::string messageType;
    /// The message's value in canonical form.
    SCE::Saved::JsonValue value;
};

/// A component started a timer, for a duration in microseconds (P7).
struct TimerStarted {
    static constexpr const char *KIND = "timer-started";
    std::string component;
    std::string timer;
    Duration duration;
};

/// A component stopped a running timer (P7). A stop of a timer that is not
/// running changes nothing, and is not recorded.
struct TimerStopped {
    static constexpr const char *KIND = "timer-stopped";
    std::string component;
    std::string timer;
};

/// A timeout branch took a timer that had timed out (P4 step 1).
struct TimerTimedOut {
    static constexpr const char *KIND = "timer-timed-out";
    std::string component;
    std::string timer;
};

/// A component's local verdict was set: by `<sce:setverdict>` (P12), or to
/// `error` by the runtime (P13). The reason is the document's, and none is
/// recorded for an empty one or for the runtime's `error`, whose cause is the
/// runtime's own prose, which another runtime would not write alike.
struct VerdictSet {
    static constexpr const char *KIND = "verdict-set";
    std::string component;
    Verdict verdict;
    std::optional<std::string> reason;
};

/// A `<log>` (P8).
struct Log {
    static constexpr const char *KIND = "log";
    std::string component;
    std::string text;
};

}  // namespace TestLog

/// An event of the test log: one of the events of namespace TestLog.
using TestLogEvent =
    std::variant<TestLog::TestCaseStarted, TestLog::TestCaseEnded, TestLog::ComponentCreated, TestLog::ComponentDone,
                 TestLog::MessageSent, TestLog::MessageReceived, TestLog::TimerStarted, TestLog::TimerStopped,
                 TestLog::TimerTimedOut, TestLog::VerdictSet, TestLog::Log>;

/// The record's `kind` in the test log: the TCI event in kebab case.
inline const char *testLogKind(const TestLogEvent &event) {
    return std::visit([](const auto &held) { return std::decay_t<decltype(held)>::KIND; }, event);
}

namespace detail {

/// The members of each kind of record beside `at` and `kind`.
inline void addMembers(SCE::Saved::JsonObject &, const TestLog::TestCaseStarted &) {}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::TestCaseEnded &event) {
    members.set("verdict", Canonical::text(verdictName(event.verdict)));
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::ComponentCreated &event) {
    members.set("component", Canonical::text(event.component));
    members.set("type", Canonical::text(event.componentType));
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::ComponentDone &event) {
    members.set("component", Canonical::text(event.component));
    members.set("verdict", Canonical::text(verdictName(event.verdict)));
}

/// A message sent and a message received hold the same members.
template <typename Carried> inline void addMessage(SCE::Saved::JsonObject &members, const Carried &event) {
    members.set("component", Canonical::text(event.component));
    members.set("port", Canonical::text(event.port));
    members.set("type", Canonical::text(event.messageType));
    members.set("value", event.value);
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::MessageSent &event) {
    addMessage(members, event);
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::MessageReceived &event) {
    addMessage(members, event);
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::TimerStarted &event) {
    members.set("component", Canonical::text(event.component));
    members.set("timer", Canonical::text(event.timer));
    members.set("duration_us", Canonical::integer(event.duration));
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::TimerStopped &event) {
    members.set("component", Canonical::text(event.component));
    members.set("timer", Canonical::text(event.timer));
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::TimerTimedOut &event) {
    members.set("component", Canonical::text(event.component));
    members.set("timer", Canonical::text(event.timer));
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::VerdictSet &event) {
    members.set("component", Canonical::text(event.component));
    members.set("verdict", Canonical::text(verdictName(event.verdict)));
    if (event.reason.has_value()) {
        members.set("reason", Canonical::text(*event.reason));
    }
}

inline void addMembers(SCE::Saved::JsonObject &members, const TestLog::Log &event) {
    members.set("component", Canonical::text(event.component));
    members.set("text", Canonical::text(event.text));
}

}  // namespace detail

/// One record of the test log: an event and `NOW` when it happened.
struct TestLogRecord {
    /// `NOW` when it was recorded.
    Instant at;
    /// What happened.
    TestLogEvent event;

    /// The record as the test log writes it: `at`, `kind`, and the members of
    /// its kind.
    SCE::Saved::JsonValue toValue() const {
        SCE::Saved::JsonObject members;
        members.set("at", Canonical::integer(at));
        members.set("kind", Canonical::text(testLogKind(event)));
        std::visit([&members](const auto &held) { detail::addMembers(members, held); }, event);
        return SCE::Saved::JsonValue(std::move(members));
    }
};

/**
 * @brief The test log of one run of the test case @p testCase of the suite
 *        @p suite, in its one canonical text
 *
 * The JSON Canonicalization Scheme of `schema`, `suite`, `test_case`,
 * `verdict` and `records`, each record in the order the run produced it.
 * Throws SCE::Saved::JsonError when a message's value is not in the canonical
 * form (MessageValue::canonical()) but holds a number past the range of a
 * double, which no writer of test/Canonical.h builds.
 */
inline std::string writeTestLog(const std::string &suite, const std::string &testCase, Verdict verdict,
                                const std::vector<TestLogRecord> &records) {
    SCE::Saved::JsonArray written;
    written.reserve(records.size());
    for (const TestLogRecord &record : records) {
        written.push_back(record.toValue());
    }
    SCE::Saved::JsonObject log;
    log.set("schema", Canonical::text(TEST_LOG_SCHEMA));
    log.set("suite", Canonical::text(suite));
    log.set("test_case", Canonical::text(testCase));
    log.set("verdict", Canonical::text(verdictName(verdict)));
    log.set("records", SCE::Saved::JsonValue(std::move(written)));
    return SCE::Saved::writeCanonical(SCE::Saved::JsonValue(std::move(log)));
}

}  // namespace SCE::Test
