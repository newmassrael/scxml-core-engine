// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// How a byte string rides in a text, on the C++ payload's wire (docs/adr/0005,
// decision 2): each byte is the character of its code point, so a byte above 0x7F is
// the two bytes of its character in the UTF-8 text and one byte here. Linked against
// googletest alone, which holds the two headers to what a generated machine needs of
// them: no runtime library.

#include "common/EventPayloadLift.h"
#include "common/Latin1Bytes.h"

#include <gtest/gtest.h>

#include <cstdint>
#include <string>
#include <vector>

namespace {

using SCE::Common::EventPayloadFields;

std::vector<uint8_t> everyByte() {
    std::vector<uint8_t> bytes;
    for (unsigned value = 0; value < 256u; ++value) {
        bytes.push_back(static_cast<uint8_t>(value));
    }
    return bytes;
}

// What a payload's field reads as bytes, or why it does not.
std::string readFrame(const std::string &wire, std::vector<uint8_t> &out) {
    EventPayloadFields fields;
    const std::string refusal = EventPayloadFields::decode(wire, fields);
    if (!refusal.empty()) {
        return refusal;
    }
    return fields.readBytes("frame", out);
}

}  // namespace

// Every one of the 256 values is one character and back.
TEST(EventPayloadBytes, EveryByteIsOneCharacterAndBack) {
    const auto bytes = everyByte();
    const std::string text = SCE::Latin1Bytes::textOf(bytes);
    // 128 bytes of ASCII are themselves, and the other 128 are the two bytes of a
    // character in U+0080..U+00FF.
    EXPECT_EQ(text.size(), 128u + 2u * 128u);
    std::vector<uint8_t> back;
    EXPECT_EQ(SCE::Latin1Bytes::bytesOf(text, back), "");
    EXPECT_EQ(back, bytes);
}

TEST(EventPayloadBytes, AByteAboveAsciiIsTheTwoBytesOfItsCharacter) {
    EXPECT_EQ(SCE::Latin1Bytes::textOf({0xFFu}), "\xC3\xBF");
    EXPECT_EQ(SCE::Latin1Bytes::textOf({0x80u}), "\xC2\x80");
    EXPECT_EQ(SCE::Latin1Bytes::textOf({'a', 0xE9u}), "a\xC3\xA9");
}

// A character past U+00FF is no byte, and a text that is not UTF-8 is no text.
TEST(EventPayloadBytes, ACharacterPastU00FFOrNoTextAtAllIsRefused) {
    const struct {
        std::string text;
        std::string why;
    } cases[] = {
        {"\xC4\x80", "carries a character above U+00FF, which no single byte spells"},          // U+0100
        {"\xE2\x82\xAC", "carries a character above U+00FF, which no single byte spells"},      // U+20AC
        {"\xF0\x9F\x98\x80", "carries a character above U+00FF, which no single byte spells"},  // U+1F600
        {"\x80", "is not valid UTF-8 text"},                                                    // a continuation alone
        {"\xC3", "is not valid UTF-8 text"},                                                    // a lead with no end
        {"\xC0\x80", "is not valid UTF-8 text"},                                                // overlong
        {"\xC3\x28", "is not valid UTF-8 text"},  // a lead and no continuation
    };

    for (const auto &c : cases) {
        std::vector<uint8_t> out = {1, 2, 3};
        EXPECT_EQ(SCE::Latin1Bytes::bytesOf(c.text, out), c.why) << c.text;
        // What was held is left as it was.
        EXPECT_EQ(out, (std::vector<uint8_t>{1, 2, 3})) << c.text;
    }
}

// The inject seam writes the wire a payload is read from: every byte survives it.
TEST(EventPayloadBytes, TheWireAnInjectSeamWritesReadsBackByteForByte) {
    const auto bytes = everyByte();
    const std::string wire = EventPayloadFields::wire({EventPayloadFields::field("frame", bytes)});
    std::vector<uint8_t> out;
    EXPECT_EQ(readFrame(wire, out), "");
    EXPECT_EQ(out, bytes);
}

// A producer that escapes what is not ASCII (a Python host's `json.dumps` does) and one
// that writes the characters as they are spell the same bytes.
TEST(EventPayloadBytes, AnEscapedAndAWrittenCharacterSpellTheSameByte) {
    std::vector<uint8_t> escaped;
    std::vector<uint8_t> written;
    std::vector<uint8_t> mixed;
    EXPECT_EQ(readFrame("{\"frame\":\"\\u00ff\\u0080a\"}", escaped), "");
    EXPECT_EQ(readFrame("{\"frame\":\"\xC3\xBF\xC2\x80"
                        "a\"}",
                        written),
              "");
    EXPECT_EQ(readFrame("{\"frame\":\"\xC3\xBF\\u0080a\"}", mixed), "");
    EXPECT_EQ(escaped, (std::vector<uint8_t>{0xFFu, 0x80u, 'a'}));
    EXPECT_EQ(written, escaped);
    EXPECT_EQ(mixed, escaped);
}

TEST(EventPayloadBytes, ACharacterPastU00FFInAPayloadNamesTheField) {
    const std::string why = "'frame' carries a character above U+00FF, which no single byte spells";
    std::vector<uint8_t> out;
    // Escaped, as a pair of halves, and written as it is.
    EXPECT_EQ(readFrame("{\"frame\":\"\\u0100\"}", out), why);
    EXPECT_EQ(readFrame("{\"frame\":\"\\ud83d\\ude00\"}", out), why);
    EXPECT_EQ(readFrame("{\"frame\":\"\xC4\x80\"}", out), why);
    EXPECT_EQ(readFrame("{\"frame\":\"\xE2\x82\xAC\"}", out), why);
    EXPECT_EQ(readFrame("{\"frame\":\"\xF0\x9F\x98\x80\"}", out), why);
}

// A text field is UTF-8 whichever way it is spelled, as the script engine reads it: an
// escape is the character it names and not a byte of it.
TEST(EventPayloadBytes, AnEscapeInATextFieldIsTheCharacterItNames) {
    EventPayloadFields fields;
    const std::string wire = "{\"label\":\"\\u00e9\\u20ac\\ud83d\\ude00\\u0041\"}";
    ASSERT_EQ(EventPayloadFields::decode(wire, fields), "");
    std::string label;
    EXPECT_EQ(fields.readString("label", label), "");
    EXPECT_EQ(label, "\xC3\xA9\xE2\x82\xAC\xF0\x9F\x98\x80"
                     "A");
}

TEST(EventPayloadBytes, HalfACharacterOrABadEscapeIsNoText) {
    EventPayloadFields fields;
    std::string label;
    for (const char *wire : {"{\"label\":\"\\ud83d\"}", "{\"label\":\"\\ude00\"}", "{\"label\":\"\\ud83dx\"}",
                             "{\"label\":\"\\ud83d\\u0041\"}", "{\"label\":\"\\u12g4\"}", "{\"label\":\"\\u12\"}"}) {
        const std::string data = wire;
        ASSERT_EQ(EventPayloadFields::decode(data, fields), "");
        EXPECT_NE(fields.readString("label", label), "") << wire;
    }
}
