// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The saved-state model of the C++ runtime: the texts every backend reads (docs/adr/0045,
// section 8), the refusals each member that breaks a format is held to, the value types a
// state is made of, and the upgrade of a state saved before a record schema or an enum was
// revised (docs/adr/0044). Cross-language siblings:
// backends/go/runtime/saved_state_test.go and
// backends/python/tests/saved_state/test_saved_state.py.

#include "saved/SavedState.h"
#include "saved/SavedJson.h"
#include "saved/SavedValues.h"
#include "saved/SessionId.h"

#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <map>
#include <regex>
#include <sstream>
#include <string>

namespace {

using namespace SCE::Saved;

std::string readTrimmed(const std::filesystem::path &path) {
    std::ifstream file(path);
    std::stringstream buffer;
    buffer << file.rdbuf();
    std::string text = buffer.str();
    while (!text.empty() && (text.back() == '\n' || text.back() == '\r' || text.back() == ' ')) {
        text.pop_back();
    }
    return text;
}

/// The shared saved-state texts every backend reads: format 2 in `saved/` and the unchanged
/// format 1 in `saved/format1/`.
std::map<std::string, std::string> sharedSavedTexts() {
    const std::filesystem::path root = SCE_SAVED_FIXTURES_DIR;
    std::map<std::string, std::string> texts;
    for (const auto &directory : {root, root / "format1"}) {
        for (const auto &entry : std::filesystem::directory_iterator(directory)) {
            if (entry.is_regular_file() && entry.path().extension() == ".json") {
                texts[entry.path().string()] = readTrimmed(entry.path());
            }
        }
    }
    return texts;
}

std::string replaceFirst(std::string text, const std::string &from, const std::string &to) {
    const auto at = text.find(from);
    if (at != std::string::npos) {
        text.replace(at, from.size(), to);
    }
    return text;
}

std::string refusalOf(const std::string &text) {
    try {
        static_cast<void>(SavedState::fromJson(text));
    } catch (const StateRefusal &refused) {
        return refused.what();
    }
    return "";
}

const std::string kMinimal =
    R"("shape":"s","sessionid":"root","configuration":["a"],"current":"a","variables":{},)"
    R"("history":{},"pending":[],"invokes":[],"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0",)"
    R"("sendseq":"0","external":[])";

}  // namespace

TEST(SavedState, EverySharedSavedTextReadsAndWritesBackAsItWas) {
    const auto texts = sharedSavedTexts();
    ASSERT_GE(texts.size(), 20U) << "the shared saved states lost instances";
    for (const auto &[path, text] : texts) {
        EXPECT_EQ(SavedState::fromJson(text).toJson(), text) << path;
    }
}

TEST(SavedState, AFormatOneTextIsOfFormatOneAndHoldsNoSession) {
    const std::filesystem::path root = SCE_SAVED_FIXTURES_DIR;
    const SavedState saved = SavedState::fromJson(readTrimmed(root / "format1" / "static_invoke_working.json"));
    EXPECT_EQ(saved.format(), kSavedFormat1);
    EXPECT_FALSE(saved.invokes.ids.empty());
    EXPECT_TRUE(saved.sessionId.empty());
    EXPECT_FALSE(saved.boundSessionId().has_value());
}

TEST(SavedState, ATreeTextHoldsEveryChildWithItsSession) {
    const std::filesystem::path root = SCE_SAVED_FIXTURES_DIR;
    const SavedState saved = SavedState::fromJson(readTrimmed(root / "static_invoke_tree_working.json"));
    ASSERT_EQ(saved.format(), kSavedFormat2);
    ASSERT_FALSE(saved.invokes.children.empty());
    const SavedChild &child = saved.invokes.children[0];
    EXPECT_EQ(child.session->sessionId, saved.sessionId + "." + child.invokeId);
    EXPECT_EQ(saved.invokeStart(child.id), child.start);
}

TEST(SavedState, ATextTheFormatsDoNotCarryIsRefusedForTheMemberThatBreaksIt) {
    const std::vector<std::pair<std::string, std::string>> cases = {
        {R"({"format":)", "the saved state is not JSON (it ends where a value was expected)"},
        {"[]", "the saved state is not an object"},
        {"{}", "the saved state has no 'format'"},
        {R"({"format":3})", "the saved state is format 3, and this runtime reads formats 1 and 2"},
        {R"({"format":"2"})", R"(the saved state is format "2", and this runtime reads formats 1 and 2)"},
        {R"({"format":2,)" + replaceFirst(kMinimal, R"("root")", R"("a b")") + "}",
         "'sessionid' (a b) is not an XML Nmtoken, which a session id is"},
        {R"({"format":2,)" + replaceFirst(kMinimal, R"("invokestarts":{})", R"("invokestarts":{"w":"01"})") + "}",
         "'invokestarts.w' (01) is not a count of starts, a whole number from 1"},
        {R"({"format":2,)" + replaceFirst(kMinimal, R"("hostinvoketoken":"0")", R"("hostinvoketoken":"x")") + "}",
         "'hostinvoketoken' (x) is not a whole number a token can be"},
        {R"({"format":2,)" + kMinimal + R"(,"revisions":{}})",
         "'revisions' names no schema; a state whose record schemas are all in their first revision leaves it out"},
        {R"({"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},)"
         R"("pending":[{"due":"5","act":"parent","event":"e","data":"","sendid":"","origin":""}],"invokes":[],)"
         R"("hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]})",
         "'pending[0].act' is 'parent', which is not raise, internal or host, the acts of format 1"},
        {R"({"format":2,)" + replaceFirst(kMinimal, R"("pending":[])", R"("pending":[{"due":"5","act":"fax"}])") + "}",
         "'pending[0].act' is 'fax', which is not raise, internal, host, parent or child"},
    };
    for (const auto &[text, reason] : cases) {
        EXPECT_EQ(refusalOf(text), reason) << text;
    }
}

TEST(SavedState, AChildSessionIsRefusedByItsPath) {
    const auto withSession = [](const std::string &session) {
        return R"({"id":"w","start":"1","invokeid":"w","document":"w","session":{)" + session + "}}";
    };
    const auto holding = [](const std::string &child) {
        return replaceFirst(kMinimal, R"("invokes":[])", R"("invokes":[)" + child + "]");
    };
    const std::string bad = withSession(replaceFirst(kMinimal, R"("configuration":["a"])", R"("configuration":[1])"));
    EXPECT_EQ(refusalOf(R"({"format":2,)" + holding(bad) + "}"),
              "the session at invokes[0].session: 'configuration' is not a text");
    EXPECT_EQ(refusalOf(R"({"format":2,)" + holding(withSession(holding(bad))) + "}"),
              "the session at invokes[0].session.invokes[0].session: 'configuration' is not a text");
}

TEST(SavedState, AChildThatStatesAFormatIsRefused) {
    const std::string child =
        R"({"id":"w","start":"1","invokeid":"w","document":"w","session":{"format":2,)" + kMinimal + "}}";
    const std::string text =
        R"({"format":2,)" + replaceFirst(kMinimal, R"("invokes":[])", R"("invokes":[)" + child + "]") + "}";
    EXPECT_EQ(refusalOf(text), "'invokes[0].session' states a format, which only the root of a tree does");
}

TEST(SavedState, AJsonNumberKeepsTheSpellingItWasWrittenIn) {
    const std::string text = R"({"a":[1,-0,1.50,1E+2,9223372036854775808],"b":null,"c":"\u00e9\ud83d\ude00"})";
    // A character written as a \u escape is written as the character.
    EXPECT_EQ(writeJson(parseJson(text)),
              "{\"a\":[1,-0,1.50,1E+2,9223372036854775808],\"b\":null,\"c\":\"\xC3\xA9\xF0\x9F\x98\x80\"}");
}

TEST(SavedState, AJsonObjectKeepsTheOrderItsMembersWereWrittenIn) {
    const JsonValue value = parseJson(R"({"z":1,"a":2,"m":3})");
    std::string order;
    for (const std::string &key : value.asObject().keys()) {
        order += key;
    }
    EXPECT_EQ(order, "zam");
}

TEST(SavedState, ATextThatIsNotOneJsonValueIsRefused) {
    const std::vector<std::pair<std::string, std::string>> cases = {
        {R"({"a":1,"a":2})", R"(not JSON (the field "a" appears twice))"},
        {"1 2", "more than one JSON value"},
        {"[1,]", "not JSON (unexpected ']')"},
        {R"("\q")", R"(not JSON (unknown escape '\q'))"},
        {R"("\u12")", R"(not JSON (a \u escape is short))"},
        {R"("\u12zz")", R"(not JSON (a \u escape is not hex))"},
        {"-", "not JSON (a number has no digits)"},
        {R"({"a" 1})", "not JSON (a ':' was expected)"},
        {"tru", "not JSON (expected 'true')"},
    };
    for (const auto &[text, want] : cases) {
        try {
            parseJson(text);
            FAIL() << text << " was read";
        } catch (const JsonError &err) {
            EXPECT_EQ(std::string(err.what()), want) << text;
        }
    }
}

TEST(SavedState, ALoneSurrogateReadsAsTheReplacementCharacter) {
    EXPECT_EQ(parseJson(R"("\ud800x\udc00")").asString(), "\xEF\xBF\xBD"
                                                          "x"
                                                          "\xEF\xBF\xBD");
}

TEST(SavedState, EveryValueTypeIsWrittenAsItsSavedFormAndReadBackAsItWas) {
    EXPECT_EQ(writeJson(savedWriteUint8(200)), "200");
    EXPECT_EQ(writeJson(savedWriteInt64(INT64_MIN)), "\"-9223372036854775808\"");
    EXPECT_EQ(writeJson(savedWriteUint64(UINT64_MAX)), "\"18446744073709551615\"");
    EXPECT_EQ(writeJson(savedWriteFloat32(0.3F)), "0.30000001192092896");
    EXPECT_EQ(writeJson(savedWriteFloat64(5.0)), "5");
    EXPECT_EQ(writeJson(savedWriteFloat64(1e21)), "1e+21");
    EXPECT_EQ(writeJson(savedWriteBytes({0, 0xE9, 0xFF})), "\"\\u0000\xC3\xA9\xC3\xBF\"");
    EXPECT_EQ(savedReadUint8(parseJson("255"), "x"), 255);
    EXPECT_EQ(savedReadInt8(parseJson("-128"), "x"), -128);
    EXPECT_EQ(savedReadUint32(parseJson("4294967295"), "x"), 4294967295U);
    EXPECT_EQ(savedReadInt64(parseJson("\"-9223372036854775808\""), "x"), INT64_MIN);
    EXPECT_EQ(savedReadUint64(parseJson("\"18446744073709551615\""), "x"), UINT64_MAX);
    EXPECT_EQ(savedReadFloat64(parseJson("1.5"), "x"), 1.5);
    EXPECT_TRUE(std::isinf(savedReadFloat32(parseJson("\"-Infinity\""), "x")));
    EXPECT_TRUE(std::isinf(savedReadFloat64(parseJson("1e999"), "x")));
    EXPECT_TRUE(savedReadBool(parseJson("true"), "x"));
    // A 32-bit real is rounded to the single it is.
    EXPECT_EQ(savedReadFloat32(parseJson("0.3"), "x"), 0.3F);
}

TEST(SavedState, AValueOfAnotherKindOrAnotherWidthIsRefusedNotConverted) {
    const auto refused = [](const std::string &text, const auto &read) {
        try {
            read(parseJson(text));
        } catch (const StateRefusal &err) {
            return std::string(err.what());
        }
        return std::string();
    };
    const auto uint8Of = [](const JsonValue &v) { return savedReadUint8(v, "x"); };
    const auto int64Of = [](const JsonValue &v) { return savedReadInt64(v, "x"); };
    const auto stringOf = [](const JsonValue &v) { return savedReadString(v, "x"); };
    const auto boolOf = [](const JsonValue &v) { return savedReadBool(v, "x"); };
    EXPECT_EQ(refused("256", uint8Of), "'x' (256) is not a whole number its type can hold");
    EXPECT_EQ(refused("-1", uint8Of), "'x' (-1) is not a whole number its type can hold");
    EXPECT_EQ(refused("1.5", uint8Of), "'x' (1.5) is not a whole number its type can hold");
    EXPECT_EQ(refused("\"1\"", uint8Of), "'x' is not a number");
    EXPECT_EQ(refused("null", uint8Of), "'x' is absent (null), and its type is not optional");
    EXPECT_EQ(refused("5", int64Of), "'x' is not a 64-bit whole number written as a text");
    EXPECT_EQ(refused("\"9223372036854775808\"", int64Of),
              "'x' (9223372036854775808) is not a whole number its type can hold");
    EXPECT_EQ(refused("1", stringOf), "'x' is not a text");
    EXPECT_EQ(refused("\"true\"", boolOf), "'x' is not a truth value");
}

TEST(SavedState, ATextAndAListAreHeldToTheBoundsTheMachineHas) {
    const auto message = [](const auto &read) {
        try {
            read();
        } catch (const StateRefusal &err) {
            return std::string(err.what());
        }
        return std::string();
    };
    EXPECT_EQ(message([] { savedReadBoundedString(JsonValue("h\xC3\xA9llo"), "t", 5); }),
              "'t' holds 6 UTF-8 bytes, past the 5 it is bounded by");
    EXPECT_EQ(message([] { savedReadBytes(JsonValue("a\xC4\x80"), "b", 8); }),
              "'b' holds U+0100, which is past U+00FF and so no byte");
    EXPECT_EQ(message([] { savedReadBytes(JsonValue("abc"), "b", 2); }),
              "'b' holds 3 bytes, past the 2 it is bounded by");
    const JsonValue items = parseJson("[1,2,300]");
    EXPECT_EQ(message([&] {
                  savedReadList(items, "l", 8,
                                [](const JsonValue &v, const std::string &w) { return savedReadUint8(v, w); });
              }),
              "'l[2]' (300) is not a whole number its type can hold");
    EXPECT_EQ(message([&] {
                  savedReadList(items, "l", 2,
                                [](const JsonValue &v, const std::string &w) { return savedReadUint8(v, w); });
              }),
              "'l' holds 3 elements, past the 2 it is bounded by");
}

TEST(SavedState, AnUpgradeGivesARecordTheDefaultsOfTheFieldsALaterRevisionAdded) {
    const std::string digest(64, 'a');
    SavedShape shape;
    shape.first = "first";
    shape.revised = "revised";
    shape.format2First = "first2";
    shape.format2Revised = "revised2";
    RecordLineage record;
    record.event = "shown";
    record.revision = 2;
    record.revisions = {digest};
    record.added = {AddedField{"weekday", 2, "0"}};
    record.holders = {RecordHolder{"one", false}, RecordHolder{"many", true}};
    shape.records = {record};

    const std::string given = R"({"one":{"year":2027},"many":[{"year":1},{"year":2}]})";
    SavedState saved;
    saved.shape = "first2";
    saved.variables = parseJson(given).asObject();
    saved.invokes.resumed = true;
    const SavedState upgraded = upgradeSavedState(saved, shape);
    EXPECT_EQ(writeJson(JsonValue(upgraded.variables)),
              R"({"one":{"year":2027,"weekday":0},"many":[{"year":1,"weekday":0},{"year":2,"weekday":0}]})");
    EXPECT_EQ(writeJson(JsonValue(saved.variables)), given) << "the upgrade changed the state it was given";

    // A record that already holds a field its revision does not declare.
    saved.variables = parseJson(R"({"one":{"year":1,"weekday":3},"many":[]})").asObject();
    try {
        upgradeSavedState(saved, shape);
        FAIL() << "upgraded";
    } catch (const StateRefusal &err) {
        EXPECT_EQ(std::string(err.what()),
                  "'one' holds 'weekday', which revision 1 of 'shown' does not declare: revision 2 added it");
    }
    // A state of another shape.
    saved.shape = "other";
    try {
        upgradeSavedState(saved, shape);
        FAIL() << "upgraded";
    } catch (const StateRefusal &err) {
        EXPECT_EQ(std::string(err.what()),
                  "the saved state is of a document of shape other, and this machine's is first2");
    }
    // A revision later than the machine's.
    saved.shape = "revised2";
    saved.revisions = {SavedRevision{"shown", 3, digest}};
    try {
        upgradeSavedState(saved, shape);
        FAIL() << "upgraded";
    } catch (const StateRefusal &err) {
        EXPECT_EQ(std::string(err.what()),
                  "the saved state was saved at revision 3 of 'shown', later than this machine's 2: a machine does "
                  "not restore what it does not know");
    }
}

// docs/adr/0055, section 6: the third revision of `shown` widened `year`, a field of the first,
// and `weekday`, which the second added, to optional. A record saved before the widening holds
// the field as a value, so a null in it is refused, naming the field and the revision; saved at
// the widening, null is absent.
TEST(SavedState, ANullSavedBeforeAFieldWasWidenedIsRefused) {
    const std::string second(64, '2');
    const std::string third(64, '3');
    SavedShape shape;
    shape.first = "first";
    shape.revised = "revised";
    shape.format2First = "first2";
    shape.format2Revised = "revised2";
    RecordLineage record;
    record.event = "shown";
    record.revision = 3;
    record.revisions = {second, third};
    record.added = {AddedField{"weekday", 2, "0"}};
    record.widened = {WidenedField{"year", 1, 3}, WidenedField{"weekday", 2, 3}};
    record.holders = {RecordHolder{"one", false}, RecordHolder{"many", true}};
    shape.records = {record};

    const auto savedAt = [&](int revision, const std::string &one, const std::string &many) {
        SavedState saved;
        saved.shape = revision == 1 ? "first2" : "revised2";
        saved.variables = parseJson(R"({"one":)" + one + R"(,"many":)" + many + "}").asObject();
        saved.invokes.resumed = true;
        if (revision > 1) {
            saved.revisions = {SavedRevision{"shown", revision, revision == 2 ? second : third}};
        }
        return saved;
    };
    const auto refusal = [&](const SavedState &saved) {
        try {
            static_cast<void>(upgradeSavedState(saved, shape));
        } catch (const StateRefusal &err) {
            return std::string(err.what());
        }
        return std::string();
    };

    // A record variable, a field of the first revision, saved at the first.
    EXPECT_EQ(refusal(savedAt(1, R"({"year":null})", "[]")),
              "'one.year' is null, which revision 1 of 'shown' holds as a value: revision 3 widened it");
    // An element of a list of records.
    EXPECT_NE(refusal(savedAt(1, R"({"year":4})", R"([{"year":1},{"year":null}])")).find("'many[1].year' is null"),
              std::string::npos);
    // A field the second revision added, saved at the second.
    EXPECT_EQ(refusal(savedAt(2, R"({"year":4,"weekday":null})", "[]")),
              "'one.weekday' is null, which revision 2 of 'shown' holds as a value: revision 3 widened it");
    // The field the second added, held by a record saved at the first, is refused as a field that
    // revision does not declare, which is the truer reason.
    EXPECT_NE(refusal(savedAt(1, R"({"year":4,"weekday":null})", "[]")).find("'one' holds 'weekday'"),
              std::string::npos);

    // Saved at the revision that widened both, null is absent.
    const SavedState kept = savedAt(3, R"({"year":null,"weekday":null})", R"([{"year":null,"weekday":2}])");
    EXPECT_EQ(writeJson(JsonValue(upgradeSavedState(kept, shape).variables)), writeJson(JsonValue(kept.variables)));
    // A record saved before the widening, each field present, restores as it was.
    EXPECT_EQ(writeJson(JsonValue(upgradeSavedState(savedAt(1, R"({"year":4})", R"([{"year":1}])"), shape).variables)),
              R"({"one":{"year":4,"weekday":0},"many":[{"year":1,"weekday":0}]})");
    // What is no record where one is held is left to the variable's own read.
    EXPECT_EQ(writeJson(JsonValue(upgradeSavedState(savedAt(1, "null", "[{}]"), shape).variables)),
              R"({"one":null,"many":[{"weekday":0}]})");
}

TEST(SavedState, AnEnumVariantALaterRevisionAddedIsRefusedInAStateSavedBeforeIt) {
    SavedShape shape;
    shape.format2First = "first2";
    shape.format2Revised = "revised2";
    EnumLineage lineage;
    lineage.key = "enum:status";
    lineage.revision = 2;
    lineage.revisions = {std::string(64, 'b')};
    lineage.added = {AddedVariant{"Done", 2}};
    lineage.holders = {EnumHolder{"item", std::string("status"), false}, EnumHolder{"state", std::nullopt, false}};
    shape.enums = {lineage};
    SavedState saved;
    saved.shape = "first2";
    saved.variables = parseJson(R"({"item":{"status":"Done"},"state":"Open"})").asObject();
    saved.invokes.resumed = true;
    try {
        upgradeSavedState(saved, shape);
        FAIL() << "upgraded";
    } catch (const StateRefusal &err) {
        EXPECT_EQ(
            std::string(err.what()),
            "'item.status' holds 'Done', which revision 1 of 'enum:status' does not declare: revision 2 added it");
    }
}

TEST(SavedState, ASessionIdIsAnXmlNmtokenAndADrawnOneIsAVersion4Uuid) {
    for (const std::string id : {"fixture-root", "a.b:c_d",
                                 "\xC3\xA9"
                                 "1",
                                 "x\xC2\xB7y"}) {
        EXPECT_FALSE(SessionId::refusalOf(id).has_value()) << id;
    }
    EXPECT_EQ(SessionId::refusalOf("").value(),
              "the session id is empty, and a session id is one name character or more (an XML Nmtoken)");
    EXPECT_EQ(SessionId::refusalOf("a b").value(),
              "the session id holds U+0020, which is not a name character: a session id is an XML Nmtoken "
              "(letters, digits, '.', '-', '_', ':')");
    EXPECT_EQ(SessionId::refusalOf("\xFF").value(),
              "the session id holds U+FFFD, which is not a name character: a session id is an XML Nmtoken "
              "(letters, digits, '.', '-', '_', ':')");
    const std::regex pattern("^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$");
    const std::string first = SessionId::draw();
    const std::string second = SessionId::draw();
    EXPECT_TRUE(std::regex_match(first, pattern)) << first;
    EXPECT_TRUE(SessionId::isNmtoken(first));
    EXPECT_NE(first, second);
}

TEST(SavedState, ARefusalInsideAChildNamesTheSessionItIsAbout) {
    const StateRefusal inner = refusalWithin(StateRefusal("'x' is not a text"), "invokes[1].session");
    EXPECT_EQ(std::string(inner.what()), "the session at invokes[1].session: 'x' is not a text");
    const StateRefusal outer = refusalWithin(inner, "invokes[0].session");
    EXPECT_EQ(std::string(outer.what()), "the session at invokes[0].session.invokes[1].session: 'x' is not a text");
}
