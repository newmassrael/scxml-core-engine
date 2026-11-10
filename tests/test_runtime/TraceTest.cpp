// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The recorded input trace of the test profile's C++ runtime
// (docs/SCE_TTCN3_LANE.md §10): the trace's reader and its one canonical text
// (test/Trace.h), and the readers of the canonical form of a value it holds
// (test/FromCanonical.h), each the inverse of a writer of test/Canonical.h.
// Each case is the twin of the Rust runtime's of the same subject, in
// backends/rust/runtime/src/test/trace.rs and src/test/from_canonical.rs, with
// the same texts and the same refusals.

#include <gtest/gtest.h>

#include <cstdint>
#include <fstream>
#include <limits>
#include <optional>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"
#include "sce/forge/bounded.h"
#include "test/Canonical.h"
#include "test/FromCanonical.h"
#include "test/Trace.h"
#include "test/TraceRefusal.h"

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace {

using SCE::Saved::JsonValue;
using SCE::Saved::parseJson;
using SCE::Saved::writeCanonical;
using SCE::Test::Trace;
using SCE::Test::TraceAdapter;
using SCE::Test::TraceInput;
using SCE::Test::TraceRefusal;

namespace Canonical = SCE::Test::Canonical;
namespace FromCanonical = SCE::Test::FromCanonical;

std::string readFile(const std::string &path) {
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    return buffer.str();
}

/// The reason @p read refuses with, which it must.
template <typename Read> std::string reason(Read read) {
    try {
        read();
    } catch (const TraceRefusal &refusal) {
        return refusal.reason();
    }
    ADD_FAILURE() << "the value is read where it should be refused";
    return "";
}

// ── The trace (test/Trace.h) ────────────────────────────────────────

/// The status this runtime declares is the schema file's own, and the schema
/// pins the name of the shape it reads. SCE_WIRE_CONTRACTS.md requires one
/// commit to move both.
TEST(TheTrace, TheSchemaFileDeclaresTheStatusThisRuntimeDoes) {
    const JsonValue schema =
        parseJson(readFile(std::string(SCE_PROJECT_ROOT) + "/schemas/sce-test-trace.v1.schema.json"));
    const JsonValue *status = schema.asObject().get("x-sce-schema-status");
    ASSERT_NE(status, nullptr);
    EXPECT_EQ(status->asString(), SCE::Test::TEST_TRACE_SCHEMA_STATUS)
        << "schemas/sce-test-trace.v1.schema.json x-sce-schema-status disagrees with TEST_TRACE_SCHEMA_STATUS";
    const JsonValue *properties = schema.asObject().get("properties");
    ASSERT_NE(properties, nullptr);
    const JsonValue *shape = properties->asObject().get("schema");
    ASSERT_NE(shape, nullptr);
    const JsonValue *named = shape->asObject().get("const");
    ASSERT_NE(named, nullptr);
    EXPECT_EQ(named->asString(), SCE::Test::TEST_TRACE_SCHEMA);
}

const std::string ONE_INPUT = "{\"inputs\":[{\"at\":\"500\",\"port\":\"sut\",\"type\":\"job_reply\","
                              "\"value\":{\"code\":\"0\",\"job\":\"7\"}}],"
                              "\"schema\":\"sce-test-trace.v1\",\"suite\":\"runtime_suite\","
                              "\"test_case\":\"answers_the_job\"}";

/// A trace is read from its canonical text, with or without the line feed a
/// file ends in, and written back as the same bytes.
TEST(TheTrace, IsReadFromItsCanonicalTextAndWrittenBack) {
    const Trace trace = Trace::parse(ONE_INPUT);
    EXPECT_EQ(trace.suite, "runtime_suite");
    EXPECT_EQ(trace.testCase, "answers_the_job");
    ASSERT_EQ(trace.inputs.size(), 1u);
    EXPECT_EQ(trace.inputs[0].at, 500);
    EXPECT_EQ(trace.inputs[0].port, "sut");
    EXPECT_EQ(trace.inputs[0].messageType, "job_reply");
    EXPECT_EQ(writeCanonical(trace.inputs[0].value),
              writeCanonical(Canonical::record(
                  {{"code", Canonical::integer(std::uint32_t{0})}, {"job", Canonical::integer(std::uint32_t{7})}})));
    EXPECT_EQ(trace.canonicalText(), ONE_INPUT);
    const Trace ended = Trace::parse(ONE_INPUT + "\n");
    EXPECT_EQ(ended.canonicalText(), ONE_INPUT);
}

/// @p from replaced once by @p to in ONE_INPUT, which holds it.
std::string changed(const std::string &from, const std::string &to) {
    const std::size_t at = ONE_INPUT.find(from);
    EXPECT_NE(at, std::string::npos) << from;
    std::string text = ONE_INPUT;
    return text.replace(at, from.size(), to);
}

/// Each change of one member of a trace it first reads is refused, by name.
TEST(TheTrace, ThatIsNotOfItsShapeIsRefusedByName) {
    const std::vector<std::pair<std::string, std::string>> cases = {
        {changed("sce-test-trace.v1", "sce-test-trace.v2"), "schema is not sce-test-trace.v1"},
        {changed("\"500\"", "500"), "the instant of input 0"},
        {changed("\"500\"", "\"0500\""), "the instant of input 0"},
        {changed("\"500\"", "\"-5\""), "the instant of input 0"},
        {changed("\"500\"", "\"9223372036854775808\""), "past the microseconds"},
        {changed("\"port\":\"sut\"", "\"port\":\"\""), "the port of input 0"},
        {changed("\"type\"", "\"kind\""), "input 0 has no member 'kind'"},
        {changed(",\"suite\":\"runtime_suite\"", ""), "the trace has no 'suite'"},
        {changed("{\"inputs\"", "{ \"inputs\""), "one canonical text"},
        {changed("[{", "{"), "not JSON"},
    };
    for (const auto &[text, expected] : cases) {
        const std::string refused = reason([&text = text] { (void)Trace::parse(text); });
        EXPECT_NE(refused.find(expected), std::string::npos) << expected << ": " << refused;
    }
    const std::string outOfOrder = "{\"inputs\":[{\"at\":\"5\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}},"
                                   "{\"at\":\"4\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}}],"
                                   "\"schema\":\"sce-test-trace.v1\",\"suite\":\"s\",\"test_case\":\"c\"}";
    EXPECT_NE(reason([&] { (void)Trace::parse(outOfOrder); }).find("input 1, at 4, comes after an input at 5"),
              std::string::npos);
}

/// A number past the range of a double has no canonical text, so a trace whose
/// value holds one is refused as not written in it. The twin of the Rust and
/// Kotlin cases of the same subject.
TEST(TheTrace, WhoseValueHoldsANumberPastADoubleIsRefused) {
    const std::string text = changed("{\"code\":\"0\",\"job\":\"7\"}", "1e400");
    EXPECT_EQ(reason([&] { (void)Trace::parse(text); }),
              "the trace is not written in its one canonical text, the JSON Canonicalization Scheme");
}

/// An input on a port the configuration does not have, or of a type its port
/// does not take, is refused, naming which.
TEST(TheTrace, AnInputTheConfigurationDoesNotTakeIsRefusedByName) {
    EXPECT_EQ(TraceRefusal::notAnInput("sut", "job_request", {"sut"}).reason(),
              "'job_request' is not on the `in` list of the type of the system port 'sut'");
    EXPECT_EQ(TraceRefusal::notAnInput("oracle", "job_reply", {"sut"}).reason(),
              "'oracle' is not a system port of the test case's configuration");
    const Trace trace = Trace::parse(ONE_INPUT);
    EXPECT_EQ(reason([&] { TraceAdapter adapter(trace, SCE::Test::noInputs); }),
              "input 0, at 500: 'sut' is not a system port of the test case's configuration");
}

/// The reader of the one input ONE_INPUT holds: a reply on `sut`, which it
/// carries as its canonical value, under the event `job.reply`.
SCE::Test::Message reply(const std::string &systemPort, const std::string &typeName, const JsonValue &value) {
    if (systemPort != "sut" || typeName != "job_reply") {
        throw TraceRefusal::notAnInput(systemPort, typeName, {"sut"});
    }
    return SCE::Test::Message{"job.reply",
                              SCE::Test::messageValue(value, typeName, [](const JsonValue &held) { return held; })};
}

/// docs/SCE_TEST_PROFILE.md §3.2 step 2: the player announces the instant of
/// its next input, delivers the inputs due at NOW and none before, and records
/// what the components send.
TEST(TheTrace, IsPlayedAtTheInstantsItHolds) {
    TraceAdapter adapter(Trace::parse(ONE_INPUT), &reply);
    EXPECT_EQ(adapter.nextInput(), std::optional<SCE::Test::Instant>{500});
    EXPECT_TRUE(adapter.takeInputsDue(499).empty()) << "an input is not delivered before its instant";
    const auto due = adapter.takeInputsDue(500);
    ASSERT_EQ(due.size(), 1u);
    EXPECT_EQ(due[0].first, "sut");
    EXPECT_EQ(due[0].second.event, "job.reply");
    EXPECT_EQ(due[0].second.value.typeName(), "job_reply");
    EXPECT_EQ(writeCanonical(due[0].second.value.canonical()), "{\"code\":\"0\",\"job\":\"7\"}");
    EXPECT_EQ(adapter.nextInput(), std::nullopt);
    const SCE::Test::SystemOutputs sent = adapter.sent();
    adapter.send("sut", due[0].second, 600);
    ASSERT_EQ(sent->size(), 1u);
    EXPECT_EQ((*sent)[0].at, 600);
    EXPECT_EQ((*sent)[0].systemPort, "sut");
}

// ── The readers of each form (test/FromCanonical.h) ─────────────────

JsonValue textOf(const std::string &text) {
    return JsonValue(text);
}

/// An integer is read from the one spelling the writer gives it, at every
/// width, and any other spelling, or a value past its type, is refused,
/// naming where it stands.
TEST(TheCanonicalReaders, ReadAnIntegerFromTheTextOfItsDigitsAlone) {
    EXPECT_EQ(FromCanonical::integer<std::uint8_t>(Canonical::integer(std::uint8_t{0}), "n"), 0);
    EXPECT_EQ(FromCanonical::integer<std::uint64_t>(Canonical::integer(std::numeric_limits<std::uint64_t>::max()), "n"),
              std::numeric_limits<std::uint64_t>::max());
    EXPECT_EQ(FromCanonical::integer<std::int64_t>(Canonical::integer(std::numeric_limits<std::int64_t>::min()), "n"),
              std::numeric_limits<std::int64_t>::min());
    for (const std::string spelling : {"007", "+7", "-0", "7.0", "", " 7"}) {
        const std::string refusal =
            reason([&] { (void)FromCanonical::integer<std::uint32_t>(textOf(spelling), "job_reply.code"); });
        EXPECT_EQ(refusal.rfind("job_reply.code: ", 0), 0u) << refusal;
        EXPECT_NE(refusal.find("decimal digits"), std::string::npos) << spelling << ": " << refusal;
    }
    EXPECT_NE(reason([] { (void)FromCanonical::integer<std::uint8_t>(textOf("256"), "n"); }).find("past the range"),
              std::string::npos);
    EXPECT_NE(reason([] { (void)FromCanonical::integer<std::uint32_t>(textOf("-1"), "n"); }).find("past the range"),
              std::string::npos);
    EXPECT_NE(reason([] {
                  (void)FromCanonical::integer<std::int64_t>(textOf("-9223372036854775809"), "n");
              }).find("past the range"),
              std::string::npos);
    EXPECT_NE(reason([] {
                  (void)FromCanonical::integer<std::uint32_t>(JsonValue(SCE::Saved::JsonNumber{"7"}), "n");
              }).find("a number is not an integer"),
              std::string::npos);
}

TEST(TheCanonicalReaders, ReadABoolAsATruthValue) {
    EXPECT_TRUE(FromCanonical::boolean(Canonical::boolean(true), "b"));
    EXPECT_NE(reason([] { (void)FromCanonical::boolean(textOf("true"), "b"); }).find("a text is not a truth value"),
              std::string::npos);
}

/// A text is read within its bound in bytes, into the engine's text and into
/// a bounded one.
TEST(TheCanonicalReaders, ReadATextWithinItsBound) {
    EXPECT_EQ(FromCanonical::text<std::string>(Canonical::text("abc"), 3, "t"), "abc");
    const auto bounded = FromCanonical::text<SCE::Forge::OwnedText<3>>(Canonical::text("abc"), 3, "t");
    EXPECT_EQ(bounded.inner(), "abc");
    EXPECT_NE(
        reason([] { (void)FromCanonical::text<std::string>(Canonical::text("abcd"), 3, "t"); }).find("past the 3"),
        std::string::npos);
    EXPECT_NE(reason([] {
                  (void)FromCanonical::text<SCE::Forge::OwnedText<2>>(Canonical::text("abc"), 3, "t");
              }).find("past what its type holds"),
              std::string::npos);
}

/// A byte string is read from upper-case hex alone, the writer's spelling,
/// and back to the same text.
TEST(TheCanonicalReaders, ReadAByteStringFromUpperCaseHex) {
    const std::vector<std::uint8_t> octets{0x00, 0xab, 0x0f, 0xff};
    const JsonValue written = Canonical::bytes(octets);
    EXPECT_EQ(FromCanonical::bytes<std::vector<std::uint8_t>>(written, 4, "b"), octets);
    const auto bounded = FromCanonical::bytes<SCE::Forge::OwnedBytes<4>>(written, 4, "b");
    EXPECT_EQ(writeCanonical(Canonical::bytes(bounded)), writeCanonical(written));
    for (const std::string spelling : {"00ab", "0", "0G"}) {
        EXPECT_NE(reason([&] {
                      (void)FromCanonical::bytes<std::vector<std::uint8_t>>(textOf(spelling), 4, "b");
                  }).find("upper-case hex"),
                  std::string::npos)
            << spelling;
    }
    EXPECT_NE(
        reason([&] { (void)FromCanonical::bytes<std::vector<std::uint8_t>>(written, 3, "b"); }).find("past the 3"),
        std::string::npos);
}

enum class Color : std::uint8_t { Red = 0, Green = 1 };

/// Every number of the carrier is a value of an enum of an open set.
constexpr Color colorOf(std::uint8_t raw) noexcept {
    return static_cast<Color>(raw);
}

/// A closed enum is read by its declared names; an open one by its names and
/// by the numbers no variant declares, a declared number being the second
/// spelling of its variant.
TEST(TheCanonicalReaders, ReadAnEnumByItsDeclaredName) {
    const FromCanonical::Declared<Color> declared{{"red", Color::Red}, {"green", Color::Green}};
    EXPECT_EQ(FromCanonical::enumerated<Color>(textOf("green"), declared, "c"), Color::Green);
    EXPECT_NE(
        reason([&] { (void)FromCanonical::enumerated<Color>(textOf("blue"), declared, "c"); }).find("'blue' names no"),
        std::string::npos);
    EXPECT_EQ(FromCanonical::openEnumerated<Color>(textOf("red"), declared, &colorOf, "c"), Color::Red);
    EXPECT_EQ(FromCanonical::openEnumerated<Color>(textOf("7"), declared, &colorOf, "c"), static_cast<Color>(7));
    EXPECT_NE(reason([&] {
                  (void)FromCanonical::openEnumerated<Color>(textOf("1"), declared, &colorOf, "c");
              }).find("written by its name"),
              std::string::npos);
    EXPECT_NE(reason([&] {
                  (void)FromCanonical::openEnumerated<Color>(textOf("300"), declared, &colorOf, "c");
              }).find("past the range"),
              std::string::npos);
}

/// A record holds only the fields it declares; a field whose type is not
/// optional is present.
TEST(TheCanonicalReaders, ReadARecordOfItsDeclaredFields) {
    const JsonValue value = parseJson("{\"code\":\"0\",\"job\":\"7\"}");
    // The record is a view of `value`, held by value, so it holds no
    // reference to the names or the place it was handed.
    const FromCanonical::CanonicalRecord object = FromCanonical::record(value, {"job", "code"}, "job_reply");
    EXPECT_EQ(FromCanonical::integer<std::uint32_t>(FromCanonical::field(object, "job", "job_reply.job"), "job"), 7u);
    EXPECT_EQ(FromCanonical::optionalField(object, "note"), nullptr);
    const JsonValue stray = parseJson("{\"job\":\"7\",\"jobs\":\"1\"}");
    EXPECT_NE(
        reason([&] { (void)FromCanonical::record(stray, {"job", "code"}, "job_reply"); }).find("'jobs' is no field"),
        std::string::npos);
    const JsonValue shortOne = parseJson("{\"job\":\"7\"}");
    const FromCanonical::CanonicalRecord partial = FromCanonical::record(shortOne, {"job", "code"}, "job_reply");
    EXPECT_EQ(reason([&] { (void)FromCanonical::field(partial, "code", "job_reply.code"); }),
              "job_reply.code: the field is absent, and its type is not optional");
}

/// A list is read within its bounds, an absent element as null, into the
/// bounded list of its type.
TEST(TheCanonicalReaders, ReadAListWithinItsBounds) {
    const JsonValue value = parseJson("[\"1\",null]");
    std::vector<std::optional<std::uint8_t>> read;
    for (const JsonValue &item : FromCanonical::elements(value, 1, 2, "l")) {
        const JsonValue *held = FromCanonical::nullable(item);
        read.push_back(held == nullptr
                           ? std::nullopt
                           : std::optional<std::uint8_t>(FromCanonical::integer<std::uint8_t>(*held, "l[]")));
    }
    ASSERT_EQ(read.size(), 2u);
    EXPECT_EQ(read[0], std::optional<std::uint8_t>{1});
    EXPECT_EQ(read[1], std::nullopt);
    using Slots = SCE::Forge::OwnedList<std::optional<std::uint8_t>, 2>;
    const Slots bounded = FromCanonical::list<Slots>(read, "l");
    EXPECT_EQ(bounded.size(), 2u);
    EXPECT_NE(reason([&] { (void)FromCanonical::elements(value, 3, 4, "l"); }).find("outside its bounds, 3 to 4"),
              std::string::npos);
    using One = SCE::Forge::OwnedList<std::uint8_t, 1>;
    EXPECT_NE(reason([] { (void)FromCanonical::list<One>({1, 2}, "l"); }).find("past the 1"), std::string::npos);
}

}  // namespace
