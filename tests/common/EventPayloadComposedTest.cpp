// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A payload value of any bounded type is read at its path, or refused there before
// anything of it is kept (docs/adr/0086, sections 4 and 14.3): every element kind, at
// list depths 1 to 3, each bound 2, with an empty value, a maximal one, one past the
// bound at the innermost and the outermost list, a bad element, `null` where the
// element is not optional, an unknown key and an older revision. The oracle is the
// construction: each case is built with the value or the path it must come to. A
// maximal value is written at exactly the size the calculus gives it and reads back
// as itself; a value a host built is refused before a raise encodes it.

#include "common/EventPayloadLift.h"

#include <gtest/gtest.h>

#include <cstdint>
#include <functional>
#include <limits>
#include <optional>
#include <string>
#include <vector>

using SCE::Common::EventPayloadBounds;
using SCE::Common::EventPayloadFields;
using SCE::Common::EventPayloadValue;
using SCE::Common::RefusalCode;
using Refusal = SCE::Common::EventPayloadRefusal;

namespace {

constexpr size_t kBound = 2;
constexpr size_t kTextBound = 3;
const std::string kControls(kTextBound, '\x01');

enum class Kind { U8, U16, U32, U64, I8, I16, I32, I64, F32, F64, Bool, Enum, Text, Bytes, Opt, Rec, RecList };

const Kind kKinds[] = {Kind::U8,   Kind::U16,   Kind::U32, Kind::U64, Kind::I8,     Kind::I16,
                       Kind::I32,  Kind::I64,   Kind::F32, Kind::F64, Kind::Bool,   Kind::Enum,
                       Kind::Text, Kind::Bytes, Kind::Opt, Kind::Rec, Kind::RecList};

const char *kindName(Kind kind) {
    static const char *const names[] = {
        "uint8",           "uint16",  "uint32",          "uint64", "int8", "int16",     "int32",
        "int64",           "float32", "float64",         "bool",   "enum", "string[3]", "bytes[3]",
        "optional<uint8>", "record",  "record of a list"};
    return names[static_cast<int>(kind)];
}

/** A kind, inside `depth` lists. `Rec` is `{a: uint8, b: string[3]}`, whose schema's later revision added
 * `c: uint8` with the default 9; `RecList` is `{tags: list<string[3]>[2]}`. */
struct Ty {
    Kind kind;
    int depth;
};

/** What a value of a type holds, as the model writes it. */
struct Held {
    enum class Tag { U, I, F, B, S, Bytes, Absent, Rec, List } tag = Tag::Absent;
    uint64_t u = 0;
    int64_t i = 0;
    double f = 0.0;
    bool b = false;
    std::string s;
    std::vector<uint8_t> bytes;
    std::vector<std::string> ids;  // a record's field ids, in field order
    std::vector<Held> items;       // a list's elements, or a record's values

    bool operator==(const Held &other) const {
        return tag == other.tag && u == other.u && i == other.i && f == other.f && b == other.b && s == other.s &&
               bytes == other.bytes && ids == other.ids && items == other.items;
    }
};

Held unsignedHeld(uint64_t v) {
    Held h;
    h.tag = Held::Tag::U;
    h.u = v;
    return h;
}

Held signedHeld(int64_t v) {
    Held h;
    h.tag = Held::Tag::I;
    h.i = v;
    return h;
}

Held textHeld(std::string v) {
    Held h;
    h.tag = Held::Tag::S;
    h.s = std::move(v);
    return h;
}

Held listHeld(std::vector<Held> items) {
    Held h;
    h.tag = Held::Tag::List;
    h.items = std::move(items);
    return h;
}

Held recordHeld(std::vector<std::string> ids, std::vector<Held> values) {
    Held h;
    h.tag = Held::Tag::Rec;
    h.ids = std::move(ids);
    h.items = std::move(values);
    return h;
}

template <typename T> Refusal readUnsigned(const EventPayloadValue &v, Held &out) {
    T n{};
    const Refusal refusal = v.readInteger(n);
    out = unsignedHeld(n);
    return refusal;
}

template <typename T> Refusal readSigned(const EventPayloadValue &v, Held &out) {
    T n{};
    const Refusal refusal = v.readInteger(n);
    out = signedHeld(n);
    return refusal;
}

/** The reader of `ty`, one call per constructor, as the generator writes it. */
Refusal read(const Ty &ty, const EventPayloadValue &v, Held &out) {
    if (ty.depth > 0) {
        std::vector<Held> items;
        const Refusal refusal = v.readList(kBound, items, [&](const EventPayloadValue &e, Held &into) {
            return read(Ty{ty.kind, ty.depth - 1}, e, into);
        });
        out = listHeld(std::move(items));
        return refusal;
    }
    switch (ty.kind) {
    case Kind::U8:
        return readUnsigned<uint8_t>(v, out);
    case Kind::U16:
        return readUnsigned<uint16_t>(v, out);
    case Kind::U32:
        return readUnsigned<uint32_t>(v, out);
    case Kind::U64: {
        uint64_t n = 0;
        const Refusal refusal = v.readWide(n);
        out = unsignedHeld(n);
        return refusal;
    }
    case Kind::I8:
        return readSigned<int8_t>(v, out);
    case Kind::I16:
        return readSigned<int16_t>(v, out);
    case Kind::I32:
        return readSigned<int32_t>(v, out);
    case Kind::I64: {
        int64_t n = 0;
        const Refusal refusal = v.readWide(n);
        out = signedHeld(n);
        return refusal;
    }
    case Kind::F32: {
        float x = 0.0f;
        const Refusal refusal = v.readFloating(x);
        out.tag = Held::Tag::F;
        out.f = static_cast<double>(x);
        return refusal;
    }
    case Kind::F64: {
        double x = 0.0;
        const Refusal refusal = v.readFloating(x);
        out.tag = Held::Tag::F;
        out.f = x;
        return refusal;
    }
    case Kind::Bool: {
        bool x = false;
        const Refusal refusal = v.readBool(x);
        out.tag = Held::Tag::B;
        out.b = x;
        return refusal;
    }
    case Kind::Enum: {
        std::string name;
        Refusal refusal = v.readName(name);
        if (refusal.empty() && name != "low" && name != "medium" && name != "high") {
            refusal = v.unknownVariant(name, "Level");
        }
        out = textHeld(name);
        return refusal;
    }
    case Kind::Text: {
        std::string text;
        const Refusal refusal = v.readText(kTextBound, text);
        out = textHeld(text);
        return refusal;
    }
    case Kind::Bytes: {
        out.tag = Held::Tag::Bytes;
        return v.readBytes(kTextBound, out.bytes);
    }
    case Kind::Opt: {
        std::optional<uint8_t> present;
        const Refusal refusal =
            v.readOptional(present, [](const EventPayloadValue &p, uint8_t &n) { return p.readInteger(n); });
        out = present.has_value() ? unsignedHeld(*present) : Held{};
        return refusal;
    }
    case Kind::Rec:
        return v.readRecord("{\"c\":9}", [&](const EventPayloadFields &fields) {
            std::vector<Held> values(3);
            for (const char *id : {"a", "c"}) {
                EventPayloadValue field;
                Refusal refusal = fields.value(id, field);
                if (refusal.empty()) {
                    refusal = readUnsigned<uint8_t>(field, values[id[0] == 'a' ? 0 : 2]);
                }
                if (!refusal.empty()) {
                    return refusal;
                }
            }
            EventPayloadValue b;
            Refusal refusal = fields.value("b", b);
            std::string text;
            if (refusal.empty()) {
                refusal = b.readText(kTextBound, text);
            }
            values[1] = textHeld(text);
            out = recordHeld({"a", "b", "c"}, std::move(values));
            return refusal;
        });
    case Kind::RecList:
        return v.readRecord(nullptr, [&](const EventPayloadFields &fields) {
            EventPayloadValue tags;
            Refusal refusal = fields.value("tags", tags);
            Held held;
            if (refusal.empty()) {
                refusal = read(Ty{Kind::Text, 1}, tags, held);
            }
            out = recordHeld({"tags"}, {held});
            return refusal;
        });
    }
    return Refusal(RefusalCode::WrongKind, v.pointer(), "no reader");
}

/** The maximal value of `ty`: every list full, every text at its bound of control characters, every number at
 * its longest spelling. */
Held maximal(const Ty &ty) {
    if (ty.depth > 0) {
        return listHeld(std::vector<Held>(kBound, maximal(Ty{ty.kind, ty.depth - 1})));
    }
    Held h;
    switch (ty.kind) {
    case Kind::U8:
        return unsignedHeld(std::numeric_limits<uint8_t>::max());
    case Kind::U16:
        return unsignedHeld(std::numeric_limits<uint16_t>::max());
    case Kind::U32:
        return unsignedHeld(std::numeric_limits<uint32_t>::max());
    case Kind::U64:
        return unsignedHeld(std::numeric_limits<uint64_t>::max());
    case Kind::I8:
        return signedHeld(std::numeric_limits<int8_t>::min());
    case Kind::I16:
        return signedHeld(std::numeric_limits<int16_t>::min());
    case Kind::I32:
        return signedHeld(std::numeric_limits<int32_t>::min());
    case Kind::I64:
        return signedHeld(std::numeric_limits<int64_t>::min());
    // Seventeen digits after `0.00000`: the longest spelling of a number, which only the decimal band from 1e-6 to
    // 1e-5 reaches. The single crosses as the double it widens to.
    case Kind::F32:
        h.tag = Held::Tag::F;
        h.f = static_cast<double>(-1.2345677760094986e-06f);
        return h;
    case Kind::F64:
        h.tag = Held::Tag::F;
        h.f = -1.2345678901234567e-06;
        return h;
    case Kind::Bool:
        h.tag = Held::Tag::B;
        return h;
    case Kind::Enum:
        return textHeld("medium");
    case Kind::Text:
        return textHeld(kControls);
    case Kind::Bytes:
        h.tag = Held::Tag::Bytes;
        h.bytes = {1, 1, 1};
        return h;
    case Kind::Opt:
        // `null` (4 bytes) is longer than any uint8's digits.
        return Held{};
    case Kind::Rec:
        return recordHeld({"a", "b", "c"}, {unsignedHeld(255), textHeld(kControls), unsignedHeld(255)});
    case Kind::RecList:
        return recordHeld({"tags"}, {listHeld({textHeld(kControls), textHeld(kControls)})});
    }
    return h;
}

/** `held` as JSON, written by the runtime's writers, one call per constructor; a record's members are handed in
 * reverse to show that the writer orders them. */
std::string write(const Ty &ty, const Held &held) {
    if (ty.depth > 0) {
        return EventPayloadFields::list(held.items, [&](const Held &e) { return write(Ty{ty.kind, ty.depth - 1}, e); });
    }
    switch (held.tag) {
    case Held::Tag::U:
        return ty.kind == Kind::U64 ? EventPayloadFields::json(std::to_string(held.u))
                                    : EventPayloadFields::json(held.u);
    case Held::Tag::I:
        return ty.kind == Kind::I64 ? EventPayloadFields::json(std::to_string(held.i))
                                    : EventPayloadFields::json(held.i);
    case Held::Tag::F:
        return EventPayloadFields::json(held.f);
    case Held::Tag::B:
        return EventPayloadFields::json(held.b);
    case Held::Tag::S:
        return EventPayloadFields::json(held.s);
    case Held::Tag::Bytes:
        return EventPayloadFields::json(held.bytes);
    case Held::Tag::Absent:
        return "null";
    case Held::Tag::Rec: {
        std::vector<std::pair<std::string, std::string>> members;
        for (size_t i = held.ids.size(); i-- > 0;) {
            const std::string &id = held.ids[i];
            const Ty field = id == "b" ? Ty{Kind::Text, 0} : id == "tags" ? Ty{Kind::Text, 1} : Ty{Kind::U8, 0};
            members.emplace_back(id, write(field, held.items[i]));
        }
        return EventPayloadFields::record(std::move(members));
    }
    case Held::Tag::List:
        break;
    }
    return "?";
}

/** The largest JSON text of `ty` (docs/adr/0086, section 2.1). */
size_t largest(const Ty &ty) {
    const auto list = [](size_t size) { return 2 + kBound * size + (kBound - 1); };
    const auto record = [](const std::vector<std::pair<std::string, size_t>> &fields) {
        size_t total = 2 + fields.size() - 1;
        for (const auto &[id, size] : fields) {
            total += id.size() + 2 + 1 + size;
        }
        return total;
    };
    if (ty.depth > 0) {
        return list(largest(Ty{ty.kind, ty.depth - 1}));
    }
    const size_t text = 2 + 6 * kTextBound;
    switch (ty.kind) {
    case Kind::U8:
        return 3;
    case Kind::U16:
        return 5;
    case Kind::U32:
        return 10;
    case Kind::U64:
    case Kind::I64:
        return 22;
    case Kind::I8:
        return 4;
    case Kind::I16:
        return 6;
    case Kind::I32:
        return 11;
    case Kind::F32:
    case Kind::F64:
        return 25;
    case Kind::Bool:
        return 5;
    case Kind::Enum:
        return 2 + std::string("medium").size();
    case Kind::Text:
    case Kind::Bytes:
        return text;
    case Kind::Opt:
        return 4;
    case Kind::Rec:
        return record({{"a", 3}, {"b", text}, {"c", 3}});
    case Kind::RecList:
        return record({{"tags", list(text)}});
    }
    return 0;
}

std::string data(const std::string &list) {
    return "{\"v\":" + list + ",\"zz\":{\"unknown\":[1,null]}}";
}

/** The path of the innermost list along the last index at each depth. */
std::string innermost(int depth) {
    std::string path = "v";
    for (int i = 1; i < depth; ++i) {
        path += "/" + std::to_string(kBound - 1);
    }
    return path;
}

std::string join(const std::vector<std::string> &parts) {
    std::string out = "[";
    for (size_t i = 0; i < parts.size(); ++i) {
        out += (i > 0 ? "," : "") + parts[i];
    }
    return out + "]";
}

/** `held` with the element at the end of the last index at every depth replaced by `leaf`. */
std::string withLastLeaf(const Ty &ty, const Held &held, const std::string &leaf) {
    if (ty.depth == 0) {
        return leaf;
    }
    const Ty elem{ty.kind, ty.depth - 1};
    std::vector<std::string> parts;
    for (const Held &item : held.items) {
        parts.push_back(write(elem, item));
    }
    parts.back() = withLastLeaf(elem, held.items.back(), leaf);
    return join(parts);
}

/** The maximal value with one more element in the list at `level` (1 the outermost) along the last index. */
std::string onePast(const Ty &ty, const Held &held, int level) {
    const Ty elem{ty.kind, ty.depth - 1};
    std::vector<std::string> parts;
    for (const Held &item : held.items) {
        parts.push_back(write(elem, item));
    }
    if (level == 1) {
        parts.push_back(write(elem, held.items.front()));
    } else {
        parts.back() = onePast(elem, held.items.back(), level - 1);
    }
    return join(parts);
}

Held setLastLeaf(Held held, const Held &leaf) {
    if (held.tag != Held::Tag::List) {
        return leaf;
    }
    held.items.back() = setLastLeaf(held.items.back(), leaf);
    return held;
}

struct Bad {
    std::string text;
    std::string below;
    std::string sentence;
};

/** A bad element of `kind`, the path below the element it is refused at, and the sentence. */
Bad bad(Kind kind) {
    switch (kind) {
    case Kind::U8:
    case Kind::U16:
    case Kind::U32:
    case Kind::Opt:
        return {"256000000000", "", "does not fit the width"};
    case Kind::U64:
        return {"\"-1\"", "", "is not a whole number written as digits"};
    case Kind::I8:
    case Kind::I16:
    case Kind::I32:
        return {"1.5", "", "is not a whole number"};
    case Kind::I64:
        return {"9007199254740992", "", "past 2^53 - 1"};
    case Kind::F32:
        return {"1e39", "", "does not fit the width"};
    case Kind::F64:
        return {"true", "", "is not a number"};
    case Kind::Bool:
        return {"0", "", "is not a truth value"};
    case Kind::Enum:
        return {"\"lowest\"", "", "is not a variant of Level"};
    case Kind::Text:
        return {"\"abcd\"", "", "is 4 bytes, past the 3 its type declares"};
    case Kind::Bytes:
        return {"\"\\u0100\"", "", "carries a character above U+00FF"};
    case Kind::Rec:
        return {"{\"a\":\"x\",\"b\":\"\"}", "/a", "is not a number"};
    case Kind::RecList:
        return {"{\"tags\":[\"\",\"\",\"x\"]}", "/tags", "holds 3 elements, past the 2"};
    }
    return {};
}

/** What the read of `text` comes to: the value, or the refusal. */
Refusal lift(const Ty &ty, const std::string &text, Held &out, const char *defaults = nullptr) {
    EventPayloadFields fields;
    Refusal refusal = defaults == nullptr ? EventPayloadFields::decode(text, fields)
                                          : EventPayloadFields::decode(text, defaults, fields);
    if (!refusal.empty()) {
        return refusal;
    }
    EventPayloadValue v;
    refusal = fields.value("v", v);
    return refusal.empty() ? read(ty, v, out) : refusal;
}

}  // namespace

TEST(EventPayloadComposed, EveryElementKindAtEveryDepthIsReadOrRefusedAtItsPath) {
    std::vector<std::string> failures;
    size_t cases = 0;
    // `want` is the value the case must read, or nullptr where it must be refused at `path`, saying `sentence`.
    const auto expect = [&](const std::string &label, const Ty &ty, const std::string &text, const Held *want,
                            const std::string &path, const std::string &sentence, const char *defaults) {
        ++cases;
        Held got;
        const std::string refusal = lift(ty, text, got, defaults).reason();
        if (want != nullptr) {
            if (!refusal.empty() || !(got == *want)) {
                failures.push_back(label + ": " + (refusal.empty() ? "read another value" : refusal));
            }
        } else if (refusal.find("'" + path + "'") == std::string::npos || refusal.find(sentence) == std::string::npos) {
            failures.push_back(label + ": " + (refusal.empty() ? "read a value" : refusal) + ", the oracle says '" +
                               path + "' " + sentence);
        }
    };
    const std::string pastBound = "holds 3 elements, past the 2 its type declares";
    for (const Kind kind : kKinds) {
        for (int depth = 1; depth <= 3; ++depth) {
            const Ty ty{kind, depth};
            const Held full = maximal(ty);
            const std::string leaf = innermost(depth) + "/" + std::to_string(kBound - 1);
            const auto label = [&](const std::string &name) {
                return std::string(kindName(kind)) + " at depth " + std::to_string(depth) + ", " + name;
            };
            const Held empty = listHeld({});
            expect(label("empty"), ty, data("[]"), &empty, "", "", nullptr);
            expect(label("maximal"), ty, data(write(ty, full)), &full, "", "", nullptr);
            expect(label("one past the innermost bound"), ty, data(onePast(ty, full, depth)), nullptr, innermost(depth),
                   pastBound, nullptr);
            expect(label("one past the outermost bound"), ty, data(onePast(ty, full, 1)), nullptr, "v", pastBound,
                   nullptr);
            if (kind == Kind::Opt) {
                // The maximal list of optionals is every element absent; a present one among them is read as
                // present.
                const Held present = setLastLeaf(full, unsignedHeld(7));
                expect(label("a present element among absent ones"), ty, data(withLastLeaf(ty, full, "7")), &present,
                       "", "", nullptr);
            } else {
                expect(label("null where the element is not optional"), ty, data(withLastLeaf(ty, full, "null")),
                       nullptr, leaf, "is null, and its type is not optional", nullptr);
            }
            const Bad b = bad(kind);
            expect(label("a bad element"), ty, data(withLastLeaf(ty, full, b.text)), nullptr, leaf + b.below,
                   b.sentence, nullptr);
            // A key no schema declares, inside a record element and beside the field, is stepped over.
            const std::string unknown =
                kind == Kind::Rec
                    ? withLastLeaf(ty, full, R"({"zz":[{"q":1}],"c":255,"b":"\u0001\u0001\u0001","a":255})")
                    : write(ty, full);
            expect(label("an unknown key"), ty, data(unknown), &full, "", "", nullptr);
            // An older revision: a record element without the field its schema's later revision added reads that
            // field's default, and a payload without a list a later revision added reads it empty.
            if (kind == Kind::Rec) {
                const Held older =
                    setLastLeaf(full, recordHeld({"a", "b", "c"}, {unsignedHeld(1), textHeld(""), unsignedHeld(9)}));
                expect(label("an element of an older revision"), ty, data(withLastLeaf(ty, full, R"({"a":1,"b":""})")),
                       &older, "", "", nullptr);
            } else {
                expect(label("a delivery of the revision before the list"), ty, R"({"zz":1})", &empty, "", "",
                       R"({"v":[]})");
            }
        }
    }
    EXPECT_EQ(cases, 17u * 3u * 8u);
    std::string all;
    for (const std::string &failure : failures) {
        all += failure + "\n";
    }
    EXPECT_TRUE(failures.empty()) << all;
}

TEST(EventPayloadComposed, AMaximalValueIsWrittenAtExactlyItsLargestSize) {
    for (const Kind kind : kKinds) {
        for (int depth = 0; depth <= 3; ++depth) {
            const Ty ty{kind, depth};
            const Held full = maximal(ty);
            const std::string text = write(ty, full);
            const size_t size = largest(ty);
            EXPECT_EQ(text.size(), size) << kindName(kind) << " at depth " << depth << ": " << text;
            Held back;
            EXPECT_EQ(lift(ty, data(text), back), "") << kindName(kind) << " at depth " << depth;
            EXPECT_TRUE(back == full) << kindName(kind) << " at depth " << depth << ": " << text;
        }
    }
}

TEST(EventPayloadComposed, ATopLevelTextAndByteStringAreHeldToTheirBoundWhenRead) {
    const std::string wire = R"({"note": "abcde", "blob": "abcde", "t": "éé", "n": null})";
    EventPayloadFields fields;
    ASSERT_EQ(EventPayloadFields::decode(wire, fields), "");
    EventPayloadValue v;
    std::string text;
    std::vector<uint8_t> bytes;
    ASSERT_EQ(fields.value("note", v), "");
    EXPECT_EQ(v.readText(4, text), "'note' is 5 bytes, past the 4 its type declares");
    ASSERT_EQ(fields.value("blob", v), "");
    EXPECT_EQ(v.readBytes(4, bytes), "'blob' is 5 bytes, past the 4 its type declares");
    EXPECT_TRUE(bytes.empty()) << "nothing of a refused value is kept";
    // A text's bound counts UTF-8 bytes: two characters, four bytes.
    ASSERT_EQ(fields.value("t", v), "");
    EXPECT_NE(v.readText(3, text), "");
    EXPECT_EQ(v.readText(4, text), "");
    EXPECT_EQ(text, "éé");
    ASSERT_EQ(fields.value("n", v), "");
    int64_t n = 0;
    EXPECT_EQ(v.readWide(n), "'n' is null, and its type is not optional");
    EXPECT_EQ(fields.value("gone", v), "the event's data has no 'gone'");
}

TEST(EventPayloadComposed, AListPastItsBoundIsRefusedBeforeAnElementIsRead) {
    const std::string wire = R"({"v": [1, 2, "x"], "s": "x"})";
    EventPayloadFields fields;
    ASSERT_EQ(EventPayloadFields::decode(wire, fields), "");
    EventPayloadValue v;
    ASSERT_EQ(fields.value("v", v), "");
    int asked = 0;
    std::vector<uint8_t> out;
    EXPECT_EQ(v.readList(2, out,
                         [&](const EventPayloadValue &e, uint8_t &into) {
                             ++asked;
                             return e.readInteger(into);
                         }),
              "'v' holds 3 elements, past the 2 its type declares");
    EXPECT_EQ(asked, 0) << "no element was read before the bound was held";
    ASSERT_EQ(fields.value("s", v), "");
    EXPECT_EQ(v.readList(2, out, [](const EventPayloadValue &e, uint8_t &into) { return e.readInteger(into); }),
              "'s' is not a list");
    // A list that never closes is no JSON text, and is refused whole before any
    // field of it is read (docs/adr/0087, section 2.3).
    const Refusal open = EventPayloadFields::decode(R"({"v": [1, 2], "open": [1, )", fields);
    EXPECT_EQ(open.code(), RefusalCode::NotJson) << open.reason();
    EXPECT_EQ(open.pointer(), "");
}

// A list short of its lower count is refused as tooFew, naming the count, before an
// element is read (docs/adr/0086, section 15.5).
TEST(EventPayloadComposed, AListShortOfItsLowerCountIsRefusedBeforeAnElementIsRead) {
    const std::string wire = R"({"v": ["x"], "w": [1, 2], "e": []})";
    EventPayloadFields fields;
    ASSERT_EQ(EventPayloadFields::decode(wire, fields), "");
    EventPayloadValue v;
    ASSERT_EQ(fields.value("v", v), "");
    int asked = 0;
    std::vector<uint8_t> out;
    const Refusal shortList = v.readListWithin(2, 3, out, [&](const EventPayloadValue &e, uint8_t &into) {
        ++asked;
        return e.readInteger(into);
    });
    EXPECT_EQ(shortList.reason(), "'v' holds 1 elements, fewer than the 2 its type declares");
    EXPECT_EQ(shortList.code(), RefusalCode::TooFew);
    EXPECT_EQ(shortList.pointer(), "/v");
    EXPECT_EQ(shortList.bound(), std::optional<uint32_t>(2u));
    EXPECT_EQ(asked, 0) << "no element was read before the count was held";
    ASSERT_EQ(fields.value("w", v), "");
    EXPECT_EQ(
        v.readListWithin(2, 3, out, [](const EventPayloadValue &e, uint8_t &into) { return e.readInteger(into); }), "");
    EXPECT_EQ(out.size(), 2u);
    ASSERT_EQ(fields.value("e", v), "");
    EXPECT_EQ(v.readListWithin(1, 3, out, [](const EventPayloadValue &e, uint8_t &into) { return e.readInteger(into); })
                  .code(),
              RefusalCode::TooFew);
    EXPECT_EQ(v.readList(3, out, [](const EventPayloadValue &e, uint8_t &into) { return e.readInteger(into); }), "");
}

// The raise's check refuses a list short of the place's lower count, the path in front
// (docs/adr/0086, section 15.6).
TEST(EventPayloadComposed, AListWrittenShortOfItsLowerCountIsRefused) {
    const auto none = [](const std::string &, int) { return std::string{}; };
    EXPECT_EQ(EventPayloadBounds::listWithin("items", std::vector<int>{1, 2}, 2, 3, none), "");
    EXPECT_EQ(EventPayloadBounds::listWithin("items/1/tags", std::vector<int>{}, 1, 3, none),
              "'items/1/tags' holds 0 elements, fewer than the 1 its type declares");
    EXPECT_EQ(EventPayloadBounds::listWithin("items", std::vector<int>{1, 2, 3, 4}, 2, 3, none),
              "'items' holds 4 elements, past the 3 its type declares");
}

TEST(EventPayloadComposed, ARecordValueNamesItsMissingFieldAndReadsItsLaterFieldsFromDefaults) {
    const std::string wire = R"({"r": {"a": 1, "zz": [null]}, "s": 5})";
    EventPayloadFields fields;
    ASSERT_EQ(EventPayloadFields::decode(wire, fields), "");
    EventPayloadValue r;
    ASSERT_EQ(fields.value("r", r), "");
    EXPECT_EQ(r.readRecord(nullptr,
                           [](const EventPayloadFields &rec) {
                               EventPayloadValue b;
                               return rec.value("b", b);
                           }),
              "the event's data has no 'r/b'");
    uint8_t b = 0;
    EXPECT_EQ(r.readRecord(R"({"b": 9})", [&](const EventPayloadFields &rec) { return rec.readInteger("b", b); }), "");
    EXPECT_EQ(b, 9u);
    EventPayloadValue s;
    ASSERT_EQ(fields.value("s", s), "");
    EXPECT_EQ(s.readRecord(nullptr, [](const EventPayloadFields &) { return Refusal{}; }), "'s' is not a record");
    EXPECT_NE(r.readRecord("[]", [](const EventPayloadFields &) { return Refusal{}; }), "");
}

TEST(EventPayloadComposed, AListAndARecordAreWrittenInTheirOrderAndByTheirKeyBytes) {
    const auto number = [](int n) { return EventPayloadFields::json(n); };
    EXPECT_EQ(EventPayloadFields::list(std::vector<int>{}, number), "[]");
    EXPECT_EQ(EventPayloadFields::list(std::vector<int>{2, 1}, number), "[2,1]");
    EXPECT_EQ(EventPayloadFields::record({{"b", "2"}, {"aa", "1"}, {"B", "3"}}), R"({"B":3,"aa":1,"b":2})");
    EXPECT_EQ(EventPayloadFields::wire({EventPayloadFields::raw("x", R"([1,{"a":null}])")}), R"({"x":[1,{"a":null}]})");
}

namespace {

/** The tag of `Shape`, read as a generated reader of an enum reads it. */
enum class ShapeKind { Dot, Circle, Disc };

/** A union over `ShapeKind` whose `circle` and `disc` both hold one record, its radius. */
struct Shape {
    ShapeKind kind = ShapeKind::Dot;
    uint16_t radius = 0;
};

Refusal readShape(const std::string &wire, Shape &out) {
    EventPayloadFields fields;
    Refusal refusal = EventPayloadFields::decode(wire, fields);
    EventPayloadValue s;
    if (refusal.empty()) {
        refusal = fields.value("s", s);
    }
    if (!refusal.empty()) {
        return refusal;
    }
    return s.readUnion<ShapeKind>(
        "kind",
        [](const EventPayloadValue &tag, ShapeKind &kind) {
            std::string name;
            Refusal named = tag.readName(name);
            if (!named.empty()) {
                return named;
            }
            if (name == "dot") {
                kind = ShapeKind::Dot;
            } else if (name == "circle") {
                kind = ShapeKind::Circle;
            } else if (name == "disc") {
                kind = ShapeKind::Disc;
            } else {
                return tag.unknownVariant(name, "ShapeKind");
            }
            return Refusal{};
        },
        [&](const ShapeKind &kind, const EventPayloadValue &object) {
            out = Shape{kind, 0};
            if (kind == ShapeKind::Dot) {
                return Refusal{};
            }
            return object.readRecord(
                nullptr, [&](const EventPayloadFields &record) { return record.readInteger("radius", out.radius); });
        });
}

}  // namespace

// A union's tag is read first, wherever it stands, and names the case whose record is read
// from the same object (docs/adr/0092, section 4): two cases that hold one record stay apart
// by their tag, a key or a `null` the case does not declare is stepped over, and each refusal
// points at the tag or at the case's field.
TEST(EventPayloadComposed, AUnionReadsItsTagFirstAndThenTheRecordOfItsCase) {
    const struct {
        const char *wire;
        ShapeKind kind;
        uint16_t radius;
    } read[] = {
        {R"({"s":{"kind":"dot"}})", ShapeKind::Dot, 0},
        {R"({"s":{"radius":2,"kind":"disc"}})", ShapeKind::Disc, 2},
        {R"({"s":{"kind":"circle","radius":2,"width":null,"text":"x"}})", ShapeKind::Circle, 2},
        {R"({"s":{"kind":"dot","radius":null}})", ShapeKind::Dot, 0},
    };

    for (const auto &c : read) {
        Shape shape;
        EXPECT_EQ(readShape(c.wire, shape), "") << c.wire;
        EXPECT_EQ(shape.kind, c.kind) << c.wire;
        EXPECT_EQ(shape.radius, c.radius) << c.wire;
    }

    const struct {
        const char *wire;
        RefusalCode code;
        const char *pointer;
    } refused[] = {
        {R"({"s":[]})", RefusalCode::WrongKind, "/s"},
        {R"({"s":null})", RefusalCode::NullNotOptional, "/s"},
        {R"({"s":{"radius":2}})", RefusalCode::Missing, "/s/kind"},
        {R"({"s":{"kind":null,"radius":null}})", RefusalCode::NullNotOptional, "/s/kind"},
        {R"({"s":{"kind":3,"radius":"x"}})", RefusalCode::WrongKind, "/s/kind"},
        {R"({"s":{"kind":"oval","radius":2}})", RefusalCode::UnknownVariant, "/s/kind"},
        {R"({"s":{"kind":"circle","width":2}})", RefusalCode::Missing, "/s/radius"},
        {R"({"s":{"kind":"disc","radius":null}})", RefusalCode::NullNotOptional, "/s/radius"},
    };

    for (const auto &c : refused) {
        Shape shape;
        const Refusal refusal = readShape(c.wire, shape);
        EXPECT_EQ(refusal.code(), c.code) << c.wire << ": " << refusal.reason();
        EXPECT_EQ(refusal.pointer(), c.pointer) << c.wire << ": " << refusal.reason();
    }
}

// A union is written as one object, its tag among the fields of its case, every key ascending
// by its bytes (docs/adr/0092, section 2.1).
TEST(EventPayloadComposed, AUnionIsWrittenWithItsTagAmongTheFieldsOfItsCase) {
    EXPECT_EQ(EventPayloadFields::unionCase("kind", "none", {}), R"({"kind":"none"})");
    EXPECT_EQ(EventPayloadFields::unionCase("kind", "relativeWeeks", {{"count", "2"}}),
              R"({"count":2,"kind":"relativeWeeks"})");
    EXPECT_EQ(EventPayloadFields::unionCase("kind", "absolute", {{"year", "2027"}, {"month", "3"}, {"day", "3"}}),
              R"({"day":3,"kind":"absolute","month":3,"year":2027})");
}

TEST(EventPayloadComposed, AValueAHostBuiltIsCheckedAtEveryDepthBeforeItIsRaised) {
    const auto tags = [](const std::string &path, const std::vector<std::string> &items) {
        return EventPayloadBounds::list(path, items, 2, [](const std::string &p, const std::string &s) {
            return EventPayloadBounds::text(p, s, 3);
        });
    };
    using Items = std::vector<std::vector<std::string>>;
    EXPECT_EQ(EventPayloadBounds::list("items", Items{{"a"}, {"abc", "ab"}}, 3, tags), "");
    EXPECT_EQ(EventPayloadBounds::list("items", Items{{}, {}, {}, {}}, 3, tags),
              "'items' holds 4 elements, past the 3 its type declares");
    EXPECT_EQ(EventPayloadBounds::list("items", Items{{"a"}, {"a", "b", "c"}}, 3, tags),
              "'items/1' holds 3 elements, past the 2 its type declares");
    EXPECT_EQ(EventPayloadBounds::list("items", Items{{"a"}, {"a", "abcd"}}, 3, tags),
              "'items/1/1' is 4 bytes, past the 3 its type declares");
    EXPECT_EQ(EventPayloadBounds::bytes("blob", std::vector<uint8_t>(5, 0), 4),
              "'blob' is 5 bytes, past the 4 its type declares");
    std::vector<std::string> checked;
    const auto present = [&](const std::string &p, const std::string &) {
        checked.push_back(p);
        return std::string{};
    };
    EXPECT_EQ(EventPayloadBounds::optional("due", std::optional<std::string>{}, present), "");
    EXPECT_EQ(EventPayloadBounds::optional("due", std::optional<std::string>{"x"}, present), "");
    EXPECT_EQ(checked, std::vector<std::string>{"due"}) << "an absent value holds nothing to check";
}
