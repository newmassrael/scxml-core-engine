// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "common/EventDataHelper.h"
#include <gtest/gtest.h>
#include <nlohmann/json.hpp>

using json = nlohmann::json;

namespace SCE {
namespace Test {

// W3C SCXML 5.10: Event data construction tests
class EventDataHelperTest : public ::testing::Test {
protected:
    void SetUp() override {}

    void TearDown() override {}
};

// Test single param name with single value
TEST_F(EventDataHelperTest, SingleParamSingleValue) {
    std::map<std::string, std::vector<std::string>> params;
    params["key1"].push_back("value1");

    std::string result = EventDataHelper::buildJsonFromParams(params);

    // Parse JSON to verify structure
    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    EXPECT_TRUE(parsed.contains("key1"));
    EXPECT_EQ(parsed["key1"], "value1");  // Single value stored as string
}

// Test multiple param names with single values each
TEST_F(EventDataHelperTest, MultipleParamsSingleValues) {
    std::map<std::string, std::vector<std::string>> params;
    params["key1"].push_back("value1");
    params["key2"].push_back("value2");

    std::string result = EventDataHelper::buildJsonFromParams(params);

    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    EXPECT_TRUE(parsed.contains("key1"));
    EXPECT_TRUE(parsed.contains("key2"));
    EXPECT_EQ(parsed["key1"], "value1");
    EXPECT_EQ(parsed["key2"], "value2");
}

// W3C Test 178: Duplicate param names - multiple values should be stored as array
TEST_F(EventDataHelperTest, DuplicateParamNames_Test178) {
    std::map<std::string, std::vector<std::string>> params;
    params["Var1"].push_back("2");
    params["Var1"].push_back("3");

    std::string result = EventDataHelper::buildJsonFromParams(params);

    // Parse JSON to verify structure
    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    EXPECT_TRUE(parsed.contains("Var1"));

    // W3C Test 178: Multiple values with same key should be array
    EXPECT_TRUE(parsed["Var1"].is_array());
    EXPECT_EQ(parsed["Var1"].size(), 2);
    EXPECT_EQ(parsed["Var1"][0], "2");
    EXPECT_EQ(parsed["Var1"][1], "3");
}

// Test mixed: some params with single values, some with multiple
TEST_F(EventDataHelperTest, MixedSingleAndMultipleValues) {
    std::map<std::string, std::vector<std::string>> params;
    params["single"].push_back("value1");
    params["multiple"].push_back("val1");
    params["multiple"].push_back("val2");
    params["multiple"].push_back("val3");

    std::string result = EventDataHelper::buildJsonFromParams(params);

    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;

    // Single value should be string
    EXPECT_TRUE(parsed.contains("single"));
    EXPECT_TRUE(parsed["single"].is_string());
    EXPECT_EQ(parsed["single"], "value1");

    // Multiple values should be array
    EXPECT_TRUE(parsed.contains("multiple"));
    EXPECT_TRUE(parsed["multiple"].is_array());
    EXPECT_EQ(parsed["multiple"].size(), 3);
    EXPECT_EQ(parsed["multiple"][0], "val1");
    EXPECT_EQ(parsed["multiple"][1], "val2");
    EXPECT_EQ(parsed["multiple"][2], "val3");
}

// Test empty params
TEST_F(EventDataHelperTest, EmptyParams) {
    std::map<std::string, std::vector<std::string>> params;

    std::string result = EventDataHelper::buildJsonFromParams(params);

    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    EXPECT_TRUE(parsed.is_object());
    EXPECT_TRUE(parsed.empty());  // Should be empty object {}
}

// Test numeric values (as strings, per W3C SCXML)
TEST_F(EventDataHelperTest, NumericValuesAsStrings) {
    std::map<std::string, std::vector<std::string>> params;
    params["number"].push_back("42");
    params["numbers"].push_back("1");
    params["numbers"].push_back("2");
    params["numbers"].push_back("3");

    std::string result = EventDataHelper::buildJsonFromParams(params);

    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    EXPECT_EQ(parsed["number"], "42");
    EXPECT_TRUE(parsed["numbers"].is_array());
    EXPECT_EQ(parsed["numbers"].size(), 3);
}

// buildEventDataJson: prefers typed JSON when typedParams non-empty.
TEST_F(EventDataHelperTest, BuildEventDataJson_PrefersTypedWhenAvailable) {
    std::map<std::string, std::vector<std::string>> stringParams;
    stringParams["force"].push_back("42");

    std::map<std::string, ScriptValue> typedParams;
    typedParams["force"] = static_cast<int64_t>(42);

    std::string result = EventDataHelper::buildEventDataJson(stringParams, typedParams);

    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    EXPECT_TRUE(parsed["force"].is_number_integer());  // Typed path: number, not "42"
    EXPECT_EQ(parsed["force"], 42);
}

// buildEventDataJson: falls back to string-only when typedParams empty.
TEST_F(EventDataHelperTest, BuildEventDataJson_FallsBackToStringOnly) {
    std::map<std::string, std::vector<std::string>> stringParams;
    stringParams["force"].push_back("42");

    std::map<std::string, ScriptValue> typedParams;  // empty

    std::string result = EventDataHelper::buildEventDataJson(stringParams, typedParams);

    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    EXPECT_TRUE(parsed["force"].is_string());  // String-only path: "42", not 42
    EXPECT_EQ(parsed["force"], "42");
}

// buildEventDataJson: a name written twice keeps both values, and its
// neighbour still arrives typed.
//
// The two maps disagree in shape — `stringParams` holds a vector per name
// because a document may repeat `<param name="x">` (W3C test 178),
// `typedParams` holds one value per name because that is all the send path
// records. Choosing one map wholesale is wrong in both directions:
// taking the typed map published `tag` as "b" and lost "a", and taking the
// string map turned `force` into "42", which an ECMAScript receiver compares
// against 42 and finds unequal.
TEST_F(EventDataHelperTest, BuildEventDataJson_KeepsDuplicateValuesAndTypesTheRest) {
    std::map<std::string, std::vector<std::string>> stringParams;
    stringParams["tag"].push_back("a");
    stringParams["tag"].push_back("b");
    stringParams["force"].push_back("42");

    std::map<std::string, ScriptValue> typedParams;
    typedParams["force"] = static_cast<int64_t>(42);
    // What the send path actually records for a repeated name: the last one.
    typedParams["tag"] = std::string("b");

    std::string result = EventDataHelper::buildEventDataJson(stringParams, typedParams);

    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    ASSERT_TRUE(parsed["tag"].is_array()) << "the repeated param collapsed to one value: " << result;
    ASSERT_EQ(parsed["tag"].size(), 2);
    EXPECT_EQ(parsed["tag"][0], "a");
    EXPECT_EQ(parsed["tag"][1], "b");
    EXPECT_TRUE(parsed["force"].is_number_integer()) << "the neighbour lost its type: " << result;
    EXPECT_EQ(parsed["force"], 42);
}

// buildEventDataJson: a typed value with no stringified twin still arrives.
TEST_F(EventDataHelperTest, BuildEventDataJson_KeepsTypedOnlyNames) {
    std::map<std::string, std::vector<std::string>> stringParams;
    stringParams["named"].push_back("x");

    std::map<std::string, ScriptValue> typedParams;
    typedParams["only"] = true;

    std::string result = EventDataHelper::buildEventDataJson(stringParams, typedParams);

    json parsed;
    ASSERT_NO_THROW(parsed = json::parse(result)) << "Invalid JSON: " << result;
    EXPECT_EQ(parsed["named"], "x");
    ASSERT_TRUE(parsed.contains("only")) << "a typed-only name was dropped because a sibling had a string: " << result;
    EXPECT_TRUE(parsed["only"].is_boolean());
    EXPECT_EQ(parsed["only"], true);
}

}  // namespace Test
}  // namespace SCE
