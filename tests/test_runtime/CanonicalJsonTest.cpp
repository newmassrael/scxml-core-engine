// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The JSON Canonicalization Scheme (RFC 8785) the test profile's C++ runtime
// writes its test log in (docs/SCE_TEST_PROFILE.md §12.5;
// SCE::Saved::writeCanonical). Each case is the twin of the Rust writer's of
// the same subject, in backends/rust/runtime/src/json.rs, with the same values
// and the same bytes.

#include <gtest/gtest.h>

#include <string>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"

namespace {

using SCE::Saved::JsonArray;
using SCE::Saved::JsonError;
using SCE::Saved::JsonNumber;
using SCE::Saved::JsonObject;
using SCE::Saved::JsonValue;
using SCE::Saved::writeCanonical;

using namespace std::string_literals;

JsonValue object(std::vector<std::pair<std::string, JsonValue>> members) {
    JsonObject out;
    for (auto &[key, value] : members) {
        out.set(key, value);
    }
    return JsonValue(std::move(out));
}

/// RFC 8785 section 3.2.3, the example of the RFC: members are sorted by their
/// names as UTF-16 code units, so U+1F600, whose first unit is 0xD83D, comes
/// before U+FB33, which an order of code points or of UTF-8 bytes would put
/// after it; U+0080 is written as it is, since only U+0000 to U+001F are
/// escaped.
TEST(CanonicalJson, MembersAreSortedByTheirUtf16CodeUnits) {
    const JsonValue value = object({
        {"\xE2\x82\xAC", "Euro Sign"},
        {"\r", "Carriage Return"},
        {"\xEF\xAC\xB3", "Hebrew Letter Dalet With Dagesh"},
        {"1", "One"},
        {"\xF0\x9F\x98\x80", "Emoji: Grinning Face"},
        {"\xC2\x80", "Control"},
        {"\xC3\xB6", "Latin Small Letter O With Diaeresis"},
    });
    EXPECT_EQ(writeCanonical(value), "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\xC2\x80\":\"Control\","
                                     "\"\xC3\xB6\":\"Latin Small Letter O With Diaeresis\","
                                     "\"\xE2\x82\xAC\":\"Euro Sign\","
                                     "\"\xF0\x9F\x98\x80\":\"Emoji: Grinning Face\","
                                     "\"\xEF\xAC\xB3\":\"Hebrew Letter Dalet With Dagesh\"}");
}

/// RFC 8785 section 3.2.2.2: a text is escaped as `JSON.stringify` escapes it,
/// the five short forms, `\u00xx` in lower-case hex for the other controls,
/// and nothing else: not `/`, not U+007F, not a character past U+FFFF.
TEST(CanonicalJson, ATextEscapesOnlyWhatJsonStringifyEscapes) {
    const JsonValue value("\b\t\n\f\r\"\\\0\x1f\x7f/\xF0\x9F\x98\x80"s);
    EXPECT_EQ(writeCanonical(value), "\"\\b\\t\\n\\f\\r\\\"\\\\\\u0000\\u001f\x7f/\xF0\x9F\x98\x80\"");
}

/// Nested arrays and objects are written with no whitespace, each object's
/// members sorted at its own level, an array's items in their order.
TEST(CanonicalJson, AValueHasNoWhitespaceAndSortsEveryLevel) {
    const JsonValue value = object({
        {"b", JsonValue(JsonArray{object({{"z", JsonValue()}, {"a", JsonValue(false)}}), JsonValue("y"),
                                  JsonValue(JsonArray{})})},
        {"a", JsonValue(JsonObject{})},
    });
    EXPECT_EQ(writeCanonical(value), "{\"a\":{},\"b\":[{\"a\":false,\"z\":null},\"y\",[]]}");
}

/// RFC 8785 section 3.2.2.3: a number is written as ECMAScript writes the
/// double it reads as; one past the range of a double is refused, as is a
/// text that is no number.
TEST(CanonicalJson, ANumberIsTheDoubleItReadsAs) {
    const std::vector<std::pair<std::string, std::string>> cases = {
        {"1E3", "1000"}, {"-0", "0"}, {"0.10", "0.1"}, {"1e21", "1e+21"}, {"1e-7", "1e-7"},
    };
    for (const auto &[spelling, written] : cases) {
        EXPECT_EQ(writeCanonical(JsonValue(JsonNumber{spelling})), written) << spelling;
    }
    try {
        writeCanonical(JsonValue(JsonNumber{"1e400"}));
        ADD_FAILURE() << "a number past the range of a double is written";
    } catch (const JsonError &refused) {
        EXPECT_NE(std::string(refused.what()).find("1e400"), std::string::npos) << refused.what();
    }
    EXPECT_THROW(writeCanonical(JsonValue(JsonNumber{"seven"})), JsonError);
}

/// A text that is not well-formed UTF-8 still has one place in the order: a
/// byte that begins no sequence stands for U+FFFD, which sorts after every
/// character of the Basic Multilingual Plane below it and before U+FFFF.
TEST(CanonicalJson, AMalformedNameStillSortsByTheCharacterItStandsFor) {
    const JsonValue value = object({
        {"\xFF", "malformed"},
        {"\xEF\xBF\xBC", "U+FFFC"},
        {"\xEF\xBF\xBF", "U+FFFF"},
    });
    EXPECT_EQ(writeCanonical(value),
              "{\"\xEF\xBF\xBC\":\"U+FFFC\",\"\xFF\":\"malformed\",\"\xEF\xBF\xBF\":\"U+FFFF\"}");
}

}  // namespace
