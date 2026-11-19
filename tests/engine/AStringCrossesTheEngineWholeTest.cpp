// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A text crosses the script engine's boundary whole. QuickJS writes U+0000 as the
// byte 0x00, so a text or a byte string that holds one, read or made as a C
// string, ends there: a value the data model hands the machine (an evaluated
// expression, an item a <foreach> binds, a <param>) and a value the machine
// hands the data model (a variable it sets, a native method's answer) would be
// cut short without a word. Where a boundary cannot carry a 0x00 the value is
// refused, never cut short (docs/SCE_ACCEPTED_SUBSET.md, the byte string as a
// <param>); this one can, so it carries every value with its length.

#include "scripting/JSEngine.h"
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace SCE {
namespace Tests {

class AStringCrossesTheEngineWholeTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &JSEngine::instance();
        engine_->reset();
        ASSERT_TRUE(engine_->createSession(sessionId_, "")) << "no session to run in";
    }

    void TearDown() override {
        if (engine_) {
            engine_->destroySession(sessionId_);
        }
    }

    /// The text `expression` evaluates to, as it leaves the engine; empty with a
    /// failure when it evaluates to no text.
    std::string textOf(const std::string &expression) {
        auto result = engine_->evaluateExpression(sessionId_, expression).get();
        EXPECT_TRUE(result.isSuccess()) << expression << ": " << result.getErrorMessage();
        const auto *text = std::get_if<std::string>(&result.getInternalValue());
        EXPECT_NE(text, nullptr) << expression << " is no text";
        return text ? *text : std::string();
    }

    /// Whether `expression` evaluates to `true` in the engine.
    bool holds(const std::string &expression) {
        auto result = engine_->evaluateExpression(sessionId_, expression).get();
        EXPECT_TRUE(result.isSuccess()) << expression << ": " << result.getErrorMessage();
        const auto *answer = std::get_if<bool>(&result.getInternalValue());
        return answer != nullptr && *answer;
    }

    JSEngine *engine_ = nullptr;
    const std::string sessionId_ = "a_string_crosses_the_engine_whole";
};

// A text holding U+0000 leaves the engine whole: three characters, the middle one
// the byte 0x00, where a C string would have ended after `a`.
TEST_F(AStringCrossesTheEngineWholeTest, ATextHoldingUPlus0000LeavesTheEngineWhole) {
    EXPECT_EQ(std::string("a\0b", 3), textOf("'a\\u0000b'"));
    // ... and so does a byte string's text, a character to a byte, its bytes past
    // 0x7f among it: each is the character of that code point in UTF-8.
    EXPECT_EQ(std::string("\xc3\xbf\0\xc2\x80", 5), textOf("'\\u00ff\\u0000\\u0080'"));
}

// A text holding U+0000 enters the engine whole: set as a variable, it is three
// characters, the middle one U+0000; and read back out it is the text it was.
TEST_F(AStringCrossesTheEngineWholeTest, ATextHoldingUPlus0000EntersTheEngineWhole) {
    ASSERT_TRUE(engine_->setVariable(sessionId_, "held", ScriptValue(std::string("a\0b", 3))).get().isSuccess());
    EXPECT_TRUE(holds("held.length === 3 && held.charCodeAt(0) === 97 && held.charCodeAt(1) === 0 && "
                      "held.charCodeAt(2) === 98"));
    EXPECT_EQ(std::string("a\0b", 3), textOf("held"));

    // A byte string's bytes past 0x7f with a 0x00 among them, as the text the
    // engine handed out above, come back as the same three code points.
    ASSERT_TRUE(
        engine_->setVariable(sessionId_, "bytes", ScriptValue(std::string("\xc3\xbf\0\xc2\x80", 5))).get().isSuccess());
    EXPECT_TRUE(holds("bytes.length === 3 && bytes.charCodeAt(0) === 255 && bytes.charCodeAt(1) === 0 && "
                      "bytes.charCodeAt(2) === 128"));
}

// A record's field and a key cross whole too: an object read out of the engine
// keeps a field that holds U+0000 and a key that does, and set back in, both are
// as they were.
TEST_F(AStringCrossesTheEngineWholeTest, ARecordCrossesTheEngineWithEveryTextWhole) {
    auto read = engine_->evaluateExpression(sessionId_, "({frame: 'a\\u0000b', 'k\\u0000y': 1})").get();
    ASSERT_TRUE(read.isSuccess()) << read.getErrorMessage();
    const auto *record = std::get_if<std::shared_ptr<ScriptObject>>(&read.getInternalValue());
    ASSERT_NE(record, nullptr);
    ASSERT_TRUE(*record);
    const auto frame = (*record)->properties.find("frame");
    ASSERT_NE(frame, (*record)->properties.end());
    const auto *text = std::get_if<std::string>(&frame->second);
    ASSERT_NE(text, nullptr) << "the field is no text";
    EXPECT_EQ(std::string("a\0b", 3), *text);
    EXPECT_NE((*record)->properties.find(std::string("k\0y", 3)), (*record)->properties.end())
        << "the key that holds U+0000 was cut short";

    ASSERT_TRUE(engine_->setVariable(sessionId_, "record", read.getInternalValue()).get().isSuccess());
    EXPECT_TRUE(holds("record.frame === 'a\\u0000b' && record['k\\u0000y'] === 1 && Object.keys(record).length === 2"));
}

// A native method is handed a text whole and answers one whole: the bridge a host's
// object is bound through carries each argument and its answer by its length.
TEST_F(AStringCrossesTheEngineWholeTest, ANativeMethodIsHandedAndAnswersATextWhole) {
    std::string handed;
    const std::vector<std::pair<std::string, IScriptEngine::NativeMethod>> methods = {
        {"echo", [&handed](const std::vector<ScriptValue> &args) -> ScriptValue {
             if (!args.empty()) {
                 if (const auto *text = std::get_if<std::string>(&args[0])) {
                     handed = *text;
                 }
             }
             return ScriptValue(std::string("x\0y", 3));
         }}};
    ASSERT_TRUE(engine_->bindNativeObject(sessionId_, "host", methods));
    EXPECT_TRUE(holds("host.echo('a\\u0000b') === 'x\\u0000y'")) << "the answer was cut short";
    EXPECT_EQ(std::string("a\0b", 3), handed) << "the argument was cut short";
}

}  // namespace Tests
}  // namespace SCE
