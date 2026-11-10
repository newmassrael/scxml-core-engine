// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The recorded input trace of a test case and its replay
// (docs/SCE_TTCN3_LANE.md §10; docs/SCE_TEST_PROFILE.md §3.2, §3.3, §12.4):
// the inputs of one run from the system under test, each at its logical
// instant, `schemas/sce-test-trace.v1.schema.json`, and the system adapter
// that plays them, TraceAdapter.
//
// A trace is one text of the JSON Canonicalization Scheme, so that it has one
// spelling and two traces are equal exactly when their bytes are:
// `{"inputs":[{"at","port","type","value"}, ...],"schema","suite","test_case"}`.
// `at` is `NOW` in microseconds as the text of its digits, `port` a system
// port of the test case's configuration, and `type` and `value` the record
// type's name and the value in the canonical form a message record of the
// test log writes (§12.5). The inputs stand in order of `at`; the inputs of
// one instant in the order they are appended to their ports' queues (§3.2
// step 2).
//
// The value of an input is read by its type: the reader of the test case's
// configuration (InputReader, generated as `input` in the configuration's
// unit) knows which records each system port takes and reads each with the
// reader of its record (test/FromCanonical.h). A trace that does not fit is
// refused whole before the run starts, which then ends with `error` (P13).
// The Rust sibling, which is the authoritative reader, is
// `sce_rust_runtime::test::trace`; the refusals here are its, word for word.

#pragma once

#include <algorithm>
#include <array>
#include <cstddef>
#include <deque>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <tuple>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"
#include "test/FromCanonical.h"
#include "test/Port.h"
#include "test/TestCaseRun.h"
#include "test/TraceRefusal.h"

namespace SCE::Test {

/// The name of the trace's shape, its `schema`.
inline constexpr const char *TEST_TRACE_SCHEMA = "sce-test-trace.v1";

/// The stability status of the trace's wire schema, pinned to the schema
/// file's own `x-sce-schema-status` by the test of this header.
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both, and the Rust
/// runtime's `TEST_TRACE_SCHEMA_STATUS` with them.
inline constexpr const char *TEST_TRACE_SCHEMA_STATUS = "pre-release";

/// The reader of the inputs of a test case's configuration: the message an
/// input of the record type @p typeName on the system port @p systemPort
/// carries, its value read from the canonical form; TraceRefusal, naming why,
/// where it is not one.
///
/// Generated as `input` in the unit of a test configuration, which names the
/// records each of its system ports takes.
using InputReader = Message (*)(const std::string &systemPort, const std::string &typeName,
                                const SCE::Saved::JsonValue &value);

/// The reader of a configuration with no system port: it refuses every input,
/// naming its port.
[[noreturn]] inline Message noInputs(const std::string &systemPort, const std::string &typeName,
                                     const SCE::Saved::JsonValue &) {
    throw TraceRefusal::notAnInput(systemPort, typeName, {});
}

/// One input of a trace.
struct TraceInput {
    /// `NOW` when it is due.
    Instant at = 0;
    /// The system port it arrives on.
    std::string port;
    /// The name of its record type, as the test log names it.
    std::string messageType;
    /// Its value, in the canonical form of the test log.
    SCE::Saved::JsonValue value;
};

namespace detail {

/// The members of @p value, an object whose members are @p names exactly, each
/// in the order of @p names; refused, naming @p what, otherwise.
template <std::size_t N>
std::array<const SCE::Saved::JsonValue *, N>
members(const SCE::Saved::JsonValue &value, const std::array<const char *, N> &names, const std::string &what) {
    if (!value.isObject()) {
        throw TraceRefusal(what + " is not an object");
    }
    const SCE::Saved::JsonObject &object = value.asObject();
    for (const std::string &member : object.keys()) {
        if (std::none_of(names.begin(), names.end(), [&member](const char *name) { return member == name; })) {
            throw TraceRefusal(what + " has no member '" + member + "'");
        }
    }
    std::array<const SCE::Saved::JsonValue *, N> found{};
    for (std::size_t at = 0; at < N; ++at) {
        found[at] = object.get(names[at]);
        if (found[at] == nullptr) {
            throw TraceRefusal(what + " has no '" + names[at] + "'");
        }
    }
    return found;
}

/// The text @p value, which is not empty, at @p what.
inline std::string nameOf(const SCE::Saved::JsonValue &value, const std::string &what) {
    if (!value.isString() || value.asString().empty()) {
        throw TraceRefusal(what + " is not a name");
    }
    return value.asString();
}

/// An instant: the text of the decimal digits of a count of microseconds from
/// the start of the test case, with no leading zero, within `NOW`'s range.
inline Instant instantOf(const SCE::Saved::JsonValue &value, const std::string &what) {
    const TraceRefusal refused(what + " is not an instant, the text of the digits of a count of microseconds");
    if (!value.isString()) {
        throw refused;
    }
    const std::string &digits = value.asString();
    const auto count = FromCanonical::decimal(digits);
    if (!count || count->negative) {
        throw refused;
    }
    const auto instant = FromCanonical::held<Instant>(*count);
    if (!instant) {
        throw TraceRefusal(what + " (" + digits + ") is past the microseconds the clock counts to");
    }
    return *instant;
}

}  // namespace detail

/// A recorded input trace of one test case of one suite.
struct Trace {
    /// The suite's name.
    std::string suite;
    /// The test case's name.
    std::string testCase;
    /// The inputs, in order of `at`, those of one instant in the order they
    /// are appended to their ports.
    std::vector<TraceInput> inputs;

    /// The trace @p text holds: the JSON Canonicalization Scheme of one trace,
    /// which a file may follow with one line feed, as a text file ends.
    ///
    /// Refused with TraceRefusal, naming why, where the text is not JSON, the
    /// shape is not `sce-test-trace.v1`, the inputs are out of the order of
    /// their instants, or the text is not the trace's one canonical spelling.
    /// Whether each input fits the suite is the configuration's to read
    /// (TraceAdapter).
    static Trace parse(std::string_view text) {
        std::string_view body = text;
        if (!body.empty() && body.back() == '\n') {
            body.remove_suffix(1);
        }
        SCE::Saved::JsonValue value;
        try {
            value = SCE::Saved::parseJson(body);
        } catch (const SCE::Saved::JsonError &refused) {
            throw TraceRefusal(std::string("the trace is ") + refused.what());
        }
        const auto [schema, suiteName, caseName, records] =
            detail::members<4>(value, {"schema", "suite", "test_case", "inputs"}, "the trace");
        if (!schema->isString() || schema->asString() != TEST_TRACE_SCHEMA) {
            throw TraceRefusal(std::string("the trace's schema is not ") + TEST_TRACE_SCHEMA);
        }
        if (!records->isArray()) {
            throw TraceRefusal("the trace's inputs are not an array");
        }
        Trace trace;
        const SCE::Saved::JsonArray &items = records->asArray();
        trace.inputs.reserve(items.size());
        for (std::size_t index = 0; index < items.size(); ++index) {
            const std::string what = "input " + std::to_string(index);
            const auto [at, port, messageType, held] =
                detail::members<4>(items[index], {"at", "port", "type", "value"}, what);
            TraceInput input;
            input.at = detail::instantOf(*at, "the instant of " + what);
            if (!trace.inputs.empty() && trace.inputs.back().at > input.at) {
                throw TraceRefusal(what + ", at " + std::to_string(input.at) + ", comes after an input at " +
                                   std::to_string(trace.inputs.back().at) +
                                   ": the inputs stand in order of their instants");
            }
            input.port = detail::nameOf(*port, "the port of " + what);
            input.messageType = detail::nameOf(*messageType, "the type of " + what);
            input.value = *held;
            trace.inputs.push_back(std::move(input));
        }
        trace.suite = detail::nameOf(*suiteName, "the trace's suite");
        trace.testCase = detail::nameOf(*caseName, "the trace's test case");
        const TraceRefusal notCanonical(
            "the trace is not written in its one canonical text, the JSON Canonicalization Scheme");
        std::string canonical;
        try {
            canonical = trace.canonicalText();
        } catch (const SCE::Saved::JsonError &) {
            throw notCanonical;
        }
        if (canonical != body) {
            throw notCanonical;
        }
        return trace;
    }

    /// The trace in its one canonical text, the JSON Canonicalization Scheme,
    /// with no line feed after it.
    ///
    /// Throws SCE::Saved::JsonError where a value holds a number past the
    /// range of a double, which no canonical value holds and parse() refuses.
    std::string canonicalText() const {
        SCE::Saved::JsonArray written;
        written.reserve(inputs.size());
        for (const TraceInput &input : inputs) {
            SCE::Saved::JsonObject record;
            record.set("at", SCE::Saved::JsonValue(std::to_string(input.at)));
            record.set("port", SCE::Saved::JsonValue(input.port));
            record.set("type", SCE::Saved::JsonValue(input.messageType));
            record.set("value", input.value);
            written.emplace_back(std::move(record));
        }
        SCE::Saved::JsonObject members;
        members.set("schema", SCE::Saved::JsonValue(TEST_TRACE_SCHEMA));
        members.set("suite", SCE::Saved::JsonValue(suite));
        members.set("test_case", SCE::Saved::JsonValue(testCase));
        members.set("inputs", SCE::Saved::JsonValue(std::move(written)));
        return SCE::Saved::writeCanonical(SCE::Saved::JsonValue(std::move(members)));
    }
};

/// What a component sent to a system port, as a TraceAdapter records it.
struct SystemOutput {
    /// `NOW` when it was sent.
    Instant at = 0;
    /// The system port it was sent to.
    std::string systemPort;
    /// The message.
    Message message;
};

/// The record of what the components of a run sent to the system ports,
/// shared with the caller while the run holds the adapter.
using SystemOutputs = std::shared_ptr<std::vector<SystemOutput>>;

/**
 * @brief A system adapter that plays a trace (docs/SCE_TTCN3_LANE.md §10.5)
 *
 * At each instant it delivers the inputs due then, in the trace's order, to
 * their system ports (docs/SCE_TEST_PROFILE.md §3.2 step 2), and it records
 * what the components send to the system ports.
 *
 * It announces the instant of its next input (nextInput()), so when no branch
 * can fire the run's clock advances to the earlier of that instant and the
 * next timer expiry or the guard (§3.3, §12.4 step 4), and the run is the same
 * whenever it is played.
 */
class TraceAdapter : public SystemAdapter {
public:
    /// The player of @p trace, each input's value read by @p read, the reader
    /// of the test case's configuration. Refused whole with TraceRefusal,
    /// naming the input, where one does not fit the configuration: its port is
    /// no system port of it, its type is not on the `in` list of that port's
    /// type, or its value does not belong to its type (docs/SCE_TTCN3_LANE.md
    /// §10.2).
    TraceAdapter(const Trace &trace, InputReader read) : sent_(std::make_shared<std::vector<SystemOutput>>()) {
        for (std::size_t index = 0; index < trace.inputs.size(); ++index) {
            const TraceInput &input = trace.inputs[index];
            try {
                inputs_.emplace_back(input.at, input.port, read(input.port, input.messageType, input.value));
            } catch (const TraceRefusal &refused) {
                throw TraceRefusal("input " + std::to_string(index) + ", at " + std::to_string(input.at) + ": " +
                                   refused.reason());
            }
        }
    }

    /// What the components send to the system ports, in the order they send
    /// it: a handle that outlives the run, which holds the adapter.
    SystemOutputs sent() const {
        return sent_;
    }

    void send(const std::string &systemPort, const Message &message, Instant now) override {
        sent_->push_back(SystemOutput{now, systemPort, message});
    }

    std::optional<Instant> nextInput() const override {
        if (inputs_.empty()) {
            return std::nullopt;
        }
        return std::get<0>(inputs_.front());
    }

    std::vector<std::pair<std::string, Message>> takeInputsDue(Instant now) override {
        std::vector<std::pair<std::string, Message>> due;
        while (!inputs_.empty() && std::get<0>(inputs_.front()) <= now) {
            due.emplace_back(std::move(std::get<1>(inputs_.front())), std::move(std::get<2>(inputs_.front())));
            inputs_.pop_front();
        }
        return due;
    }

private:
    std::deque<std::tuple<Instant, std::string, Message>> inputs_;
    SystemOutputs sent_;
};

}  // namespace SCE::Test
