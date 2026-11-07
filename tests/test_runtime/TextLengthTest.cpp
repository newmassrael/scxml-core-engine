// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The TTCN-3 length of a text (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201
// 873-1 B.1.4.1 counts a universal charstring in characters, so a character
// of two, three or four UTF-8 bytes is one, as a character of one byte is.

#include <gtest/gtest.h>

#include <string>

#include "test/Length.h"

namespace {

using SCE::Test::textLength;

TEST(TextLength, AnEmptyTextHoldsNoCharacter) {
    EXPECT_EQ(textLength(""), 0u);
}

TEST(TextLength, AnAsciiTextHoldsOneCharacterPerByte) {
    EXPECT_EQ(textLength("started"), 7u);
}

// U+00E9, two bytes: C3 A9.
TEST(TextLength, ACharacterOfTwoBytesIsOne) {
    const std::string text = "caf\xC3\xA9";
    EXPECT_EQ(text.size(), 5u);
    EXPECT_EQ(textLength(text), 4u);
}

// U+20AC, three bytes: E2 82 AC.
TEST(TextLength, ACharacterOfThreeBytesIsOne) {
    const std::string text = "\xE2\x82\xAC"
                             "5";
    EXPECT_EQ(text.size(), 4u);
    EXPECT_EQ(textLength(text), 2u);
}

// U+1F600, four bytes: F0 9F 98 80.
TEST(TextLength, ACharacterOfFourBytesIsOne) {
    const std::string text = "a\xF0\x9F\x98\x80"
                             "b";
    EXPECT_EQ(text.size(), 6u);
    EXPECT_EQ(textLength(text), 3u);
}

// One character of each width in a row: ten bytes, four characters.
TEST(TextLength, CharactersOfEveryWidthAreCountedTogether) {
    const std::string text = "a\xC3\xA9\xE2\x82\xAC\xF0\x9F\x98\x80";
    EXPECT_EQ(text.size(), 10u);
    EXPECT_EQ(textLength(text), 4u);
}

// The count is a constant expression, so a generated matcher may fold it.
static_assert(SCE::Test::textLength("\xC3\xA9") == 1, "a character of two bytes is one");

}  // namespace
