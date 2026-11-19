// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0101, Landing order step 4b: a CBOR map of an entry of every
// constructor of the language, generated for C++ from
// tests/forge/conformance/codec_cbor_composed/codec_cbor_composed.scxml with the
// enums, records and union it reaches, and held to the corpus the Rust
// conformance holds Rust to (tests/forge/conformance/numerical_reference.json,
// `codec_cbor_composed`; backends/rust/forge-conformance/tests/
// forge_cbor_composed.rs): each case written in its one form and read back, each
// variant encoding read to its value and written again in the one form, and each
// reject refused by the name of its class (section 3.3), leaving the cursor where
// it was. A value its types hold and no encoding carries is refused before a byte
// is written.
//
// A value is read from the corpus as the tree of docs/SCE_TEST_PROFILE.md section
// 12.5 the corpus writes it in: an integer the text of its digits, a byte string
// its upper-case hex, an enum the name of its variant (or the digits of a number
// an open enum does not name), a float a JSON number. Two values are compared by
// their encodings: the encode writes one form per value (section 2.1), so a
// value that writes a case's bytes is the case's value.

#include "codec_cbor_composed.h"

#include <nlohmann/json.hpp>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <fstream>
#include <limits>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

#ifndef REFERENCE_JSON_PATH
#error "REFERENCE_JSON_PATH must name tests/forge/conformance/numerical_reference.json"
#endif

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, std::string(what).c_str());                    \
        }                                                                                                              \
    } while (0)

using Json = nlohmann::json;
using Bytes = std::vector<std::uint8_t>;
using ::SCE::Forge::CodecError;
using ::SCE::Forge::OwnedList;
using ::SCE::Forge::OwnedText;
using ::SCE::Forge::SceCursor;

using Value = ::SCE::Generated::CodecCborComposed::CodecCborComposed;
namespace LevelUnit = ::SCE::Generated::CborComposedLevel;
using Level = LevelUnit::CborComposedLevel;
namespace ColorUnit = ::SCE::Generated::CborComposedColor;
using Color = ColorUnit::CborComposedColor;
namespace CodeUnit = ::SCE::Generated::CborComposedCode;
using Code = CodeUnit::CborComposedCode;
using Point = ::SCE::Generated::CborComposedPoint::CborComposedPointPayload;
using Reading = ::SCE::Generated::CborComposedReading::CborComposedReadingPayload;
using Card = ::SCE::Generated::CborComposedCard::CborComposedCardPayload;
using Batch = ::SCE::Generated::CborComposedBatch::CborComposedBatchPayload;
using Choice = ::SCE::Generated::CborComposedChoice::CborComposedChoice;

// docs/adr/0101, section 5.3: a composed entry is held in the type a schema unit
// holds it in, and lent; a number is presented by value, a text within its
// bound.
static_assert(std::is_same<decltype(std::declval<const Value &>().small()), std::int8_t>::value,
              "an int8 is presented as its type");
static_assert(std::is_same<decltype(std::declval<const Value &>().ratio()), float>::value,
              "a float32 is presented as a float");
static_assert(std::is_same<decltype(std::declval<const Value &>().reading()), const Reading &>::value,
              "a record is lent in its schema's payload type");
static_assert(std::is_same<decltype(std::declval<const Value &>().choice()), const Choice &>::value,
              "a union is lent in its unit's type");
static_assert(std::is_same<decltype(std::declval<const Value &>().choices()), const OwnedList<Choice, 3> &>::value,
              "a list is lent in the runtime's list of its bound");
static_assert(std::is_same<decltype(std::declval<const Value &>().sparse()),
                           const OwnedList<std::optional<std::uint16_t>, 3> &>::value,
              "a list's element that may be absent is an optional");
static_assert(std::is_same<decltype(std::declval<const Value &>().maybe()), const std::optional<Choice> &>::value,
              "an entry that may be absent is lent as an optional");
static_assert(std::is_same<decltype(std::declval<const Value &>().note()), std::optional<::SCE::Forge::Text<8>>>::value,
              "a text entry keeps its view");

/// The corpus's entry of the codec.
Json corpus() {
    std::ifstream in(REFERENCE_JSON_PATH);
    const Json reference = Json::parse(in);
    return reference.at("codecs").at("codec_cbor_composed");
}

Bytes bytes_of(const Json &v) {
    Bytes out;
    for (const auto &b : v) {
        out.push_back(b.get<std::uint8_t>());
    }
    return out;
}

/// An integer, written as the text of its digits.
template <typename T> T integer(const Json &v) {
    const std::string digits = v.get<std::string>();
    if constexpr (std::is_signed<T>::value) {
        return static_cast<T>(std::stoll(digits));
    } else {
        return static_cast<T>(std::stoull(digits));
    }
}

/// A byte string, written as its upper-case hex.
std::vector<std::uint8_t> hex(const Json &v) {
    const std::string text = v.get<std::string>();
    std::vector<std::uint8_t> out;
    for (std::size_t at = 0; at + 1 < text.size(); at += 2) {
        out.push_back(static_cast<std::uint8_t>(std::stoul(text.substr(at, 2), nullptr, 16)));
    }
    return out;
}

/// The member `key` of `object`, or none where it is left out or `null`: a
/// field or an entry that is absent.
const Json *member(const Json &object, const char *key) {
    const auto found = object.find(key);
    if (found == object.end() || found->is_null()) {
        return nullptr;
    }
    return &*found;
}

/// A list of at most `N` elements, each read by `each`.
template <typename T, std::size_t N, typename F> OwnedList<T, N> list(const Json &v, F each) {
    OwnedList<T, N> out;
    for (const auto &element : v) {
        const bool held = out.push(each(element));
        CHECK(held, "the case is within the list's bound");
    }
    return out;
}

/// The variant of a closed enum of the carrier `C` named by `v`, among the
/// values `from` makes of each number of the carrier.
template <typename E, typename C, typename From, typename Named> E closed(const Json &v, From from, Named named) {
    const std::string name = v.get<std::string>();
    for (long long n = std::numeric_limits<C>::min(); n <= static_cast<long long>(std::numeric_limits<C>::max()); ++n) {
        const auto e = from(static_cast<C>(n));
        if (e && named(*e) != nullptr && name == named(*e)) {
            return *e;
        }
    }
    CHECK(false, "no variant is named " + name);
    return E{};
}

Level level(const Json &v) {
    return closed<Level, std::int8_t>(v, LevelUnit::from_underlying, LevelUnit::declared_name);
}

Color color(const Json &v) {
    return closed<Color, std::uint8_t>(v, ColorUnit::from_underlying, ColorUnit::declared_name);
}

/// A value of the open enum: the variant of its name, or the number its digits
/// write, which no variant needs to name.
Code code(const Json &v) {
    const std::string text = v.get<std::string>();
    if (!text.empty() && text[0] >= '0' && text[0] <= '9') {
        return CodeUnit::from_underlying(static_cast<std::uint16_t>(std::stoul(text)));
    }
    for (std::uint32_t n = 0; n <= std::numeric_limits<std::uint16_t>::max(); ++n) {
        const Code c = CodeUnit::from_underlying(static_cast<std::uint16_t>(n));
        const char *named = CodeUnit::declared_name(c);
        if (named != nullptr && text == named) {
            return c;
        }
    }
    CHECK(false, "no variant is named " + text);
    return Code{};
}

Point point(const Json &v) {
    Point p{};
    p.x = integer<std::int16_t>(v.at("x"));
    p.y = integer<std::int16_t>(v.at("y"));
    return p;
}

Reading reading(const Json &v) {
    Reading r{};
    r.samples = list<std::uint8_t, 3>(v.at("samples"), integer<std::uint8_t>);
    if (const Json *label = member(v, "label")) {
        r.label = OwnedText<8>::make(label->get<std::string>()).value();
    }
    r.at = point(v.at("at"));
    r.color = color(v.at("color"));
    return r;
}

/// A value of the union: an object of one member, named by its case.
Choice choice(const Json &v) {
    CHECK(v.is_object() && v.size() == 1, "a union is an object of one member: " + v.dump());
    const auto only = v.begin();
    const std::string variant = only.key();
    const Json &record = only.value();
    if (variant == "idle") {
        return Choice{Choice::Idle{}};
    }
    if (variant == "card") {
        Card card{};
        card.token = hex(record.at("token"));
        card.amount = integer<std::uint32_t>(record.at("amount"));
        return Choice{Choice::Card{card}};
    }
    if (variant == "batch") {
        Batch batch{};
        batch.items = list<std::uint16_t, 3>(record.at("items"), integer<std::uint16_t>);
        return Choice{Choice::Batch{batch}};
    }
    CHECK(false, "no case is named " + variant);
    return Choice{};
}

/// What the value of a case is made of, in the order its constructor takes
/// them, so a test can change one part of a case and make the value again.
struct Parts {
    std::int8_t small;
    std::int64_t wide;
    float ratio;
    double precise;
    Level level;
    Color color;
    Code code;
    OwnedList<std::uint8_t, 4> octets;
    OwnedList<std::optional<std::uint16_t>, 3> sparse;
    OwnedList<OwnedList<std::uint8_t, 2>, 2> grid;
    OwnedList<bool, 2> flags;
    Reading reading;
    std::optional<Point> spare;
    Choice choice;
    OwnedList<Choice, 3> choices;
    std::optional<Choice> maybe;
    std::optional<std::string> note;
};

/// The parts of the value the corpus writes as `v`.
Parts parts(const Json &v) {
    Parts p{};
    p.small = integer<std::int8_t>(v.at("small"));
    p.wide = integer<std::int64_t>(v.at("wide"));
    p.ratio = static_cast<float>(v.at("ratio").get<double>());
    p.precise = v.at("precise").get<double>();
    p.level = level(v.at("level"));
    p.color = color(v.at("color"));
    p.code = code(v.at("code"));
    p.octets = list<std::uint8_t, 4>(v.at("octets"), integer<std::uint8_t>);
    p.sparse = list<std::optional<std::uint16_t>, 3>(v.at("sparse"), [](const Json &e) {
        return e.is_null() ? std::optional<std::uint16_t>() : std::optional<std::uint16_t>(integer<std::uint16_t>(e));
    });
    p.grid = list<OwnedList<std::uint8_t, 2>, 2>(
        v.at("grid"), [](const Json &row) { return list<std::uint8_t, 2>(row, integer<std::uint8_t>); });
    p.flags = list<bool, 2>(v.at("flags"), [](const Json &b) { return b.get<bool>(); });
    p.reading = reading(v.at("reading"));
    if (const Json *spare = member(v, "spare")) {
        p.spare = point(*spare);
    }
    p.choice = choice(v.at("choice"));
    p.choices = list<Choice, 3>(v.at("choices"), choice);
    if (const Json *maybe = member(v, "maybe")) {
        p.maybe = choice(*maybe);
    }
    if (const Json *note = member(v, "note")) {
        p.note = note->get<std::string>();
    }
    return p;
}

/// The value made of `p`, by its constructor.
Value made(const Parts &p) {
    std::optional<::SCE::Forge::Text<8>> note;
    if (p.note) {
        note = ::SCE::Forge::Text<8>::make(std::string_view(*p.note));
        CHECK(note.has_value(), "the note is within its bound");
    }
    return Value(p.small, p.wide, p.ratio, p.precise, p.level, p.color, p.code, p.octets, p.sparse, p.grid, p.flags,
                 p.reading, p.spare, p.choice, p.choices, p.maybe, note);
}

/// The refusal the corpus names `name` (docs/adr/0101, section 3.3).
std::optional<CodecError> refusal(const std::string &name) {
    if (name == "need-more-bytes") {
        return CodecError::NeedMoreBytes;
    }
    if (name == "invalid-utf8") {
        return CodecError::InvalidUtf8;
    }
    if (name == "undeclared-enum-value") {
        return CodecError::UndeclaredEnumValue;
    }
    if (name == "cbor-malformed") {
        return CodecError::CborMalformed;
    }
    if (name == "cbor-wrong-type") {
        return CodecError::CborWrongType;
    }
    if (name == "cbor-duplicate-key") {
        return CodecError::CborDuplicateKey;
    }
    if (name == "cbor-unknown-variant") {
        return CodecError::CborUnknownVariant;
    }
    if (name == "cbor-union-entries") {
        return CodecError::CborUnionEntries;
    }
    if (name == "cbor-required-key-missing") {
        return CodecError::CborRequiredKeyMissing;
    }
    if (name == "cbor-too-deep") {
        return CodecError::CborTooDeep;
    }
    if (name == "cbor-out-of-range") {
        return CodecError::CborOutOfRange;
    }
    CHECK(false, "no refusal of a CBOR decode is named " + name);
    return std::nullopt;
}

/// The bytes `v` is written as: a value the corpus writes has an encoding.
Bytes encoded(const Value &v) {
    const auto bytes = v.encode_to_vec();
    CHECK(bytes.has_value(), "a value the corpus writes has an encoding");
    return bytes ? *bytes : Bytes{};
}

void every_case_is_written_in_its_one_form_and_read_back() {
    const Json cases = corpus().at("cases");
    CHECK(cases.size() == 3, "the smallest, the largest and the vectors");
    for (const auto &c : cases) {
        const std::string note = c.at("note").get<std::string>();
        const Bytes bytes = bytes_of(c.at("encoded"));
        CHECK(encoded(made(parts(c.at("decoded")))) == bytes, "a case is written in its one form: " + note);
        SceCursor cursor(bytes.data(), bytes.size());
        const auto read = Value::decode(cursor);
        CHECK(read.has_value(), "a case is read: " + note);
        if (read) {
            CHECK(encoded(*read) == bytes, "a case is read to the value that writes it: " + note);
        }
        CHECK(cursor.remaining() == 0, "a case is read whole: " + note);
    }
}

void a_variant_encoding_is_read_and_written_again_in_its_one_form() {
    const Json variants = corpus().at("variants");
    CHECK(!variants.empty(), "the corpus holds variant encodings");
    for (const auto &variant : variants) {
        const std::string why = variant.at("why").get<std::string>();
        const Bytes bytes = bytes_of(variant.at("encoded"));
        const Bytes canonical = bytes_of(variant.at("canonical"));
        SceCursor cursor(bytes.data(), bytes.size());
        const auto read = Value::decode(cursor);
        CHECK(read.has_value(), "a variant encoding is read: " + why);
        if (read) {
            CHECK(encoded(*read) == canonical, "a variant encoding is written again in its one form: " + why);
        }
        CHECK(encoded(made(parts(variant.at("decoded")))) == canonical,
              "the value a variant encoding reads to is written in its one form: " + why);
    }
}

void what_is_no_value_of_the_type_is_refused_by_the_name_of_its_class() {
    const Json rejects = corpus().at("rejects");
    CHECK(!rejects.empty(), "the corpus holds rejects");
    for (const auto &reject : rejects) {
        const std::string why = reject.at("why").get<std::string>();
        const Bytes bytes = bytes_of(reject.at("encoded"));
        const auto kind = refusal(reject.at("error").get<std::string>());
        SceCursor cursor(bytes.data(), bytes.size());
        const auto read = Value::decode(cursor);
        CHECK(!read.has_value() && kind.has_value() && read.error() == *kind, "refused by its class: " + why);
        CHECK(cursor.remaining() == bytes.size(), "the cursor stays where it was: " + why);
    }
}

/// The `card` case of a token of `len` bytes, which the card's unit holds in a
/// `std::vector` and its type bounds at four.
Choice card_of(std::uint8_t len) {
    Card card{};
    for (std::uint8_t b = 1; b <= len; ++b) {
        card.token.push_back(b);
    }
    card.amount = 0;
    return Choice{Choice::Card{card}};
}

/// What encoding `v` into an empty vector answers, and whether the vector is
/// still empty after it.
std::pair<std::optional<CodecError>, bool> encoded_into_empty(const Value &v) {
    Bytes out;
    ::SCE::Forge::VectorSink sink(out);
    const auto refused = v.encode(sink);
    return {refused, out.empty()};
}

void a_value_its_types_hold_and_no_encoding_carries_is_refused_before_a_byte() {
    const Parts smallest = parts(corpus().at("cases").at(0).at("decoded"));
    const std::pair<const char *, void (*)(Parts &)> changes[] = {
        {"no flag, below the lower count 1", [](Parts &p) { p.flags = OwnedList<bool, 2>{}; }},
        {"no union in choices, below the lower count 1", [](Parts &p) { p.choices = OwnedList<Choice, 3>{}; }},
        {"a card of a long token in choice", [](Parts &p) { p.choice = card_of(5); }},
        {"a card of a long token in maybe", [](Parts &p) { p.maybe = card_of(5); }},
        {"a card of a long token in choices",
         [](Parts &p) {
             p.choices = OwnedList<Choice, 3>{};
             const bool idle = p.choices.push(Choice{Choice::Idle{}});
             const bool card = p.choices.push(card_of(5));
             CHECK(idle && card, "within the bound of three");
         }},
    };
    for (const auto &[why, change] : changes) {
        Parts p = smallest;
        change(p);
        const auto [refused, untouched] = encoded_into_empty(made(p));
        CHECK(refused == std::optional<CodecError>(CodecError::CborOutOfRange), why);
        CHECK(untouched, std::string(why) + ": the encode wrote before it refused");
    }
    // A token at its bound has an encoding.
    Parts p = smallest;
    p.choice = card_of(4);
    CHECK(made(p).encode_to_vec().has_value(), "a token at its bound is written");
}

void a_nan_is_written_as_the_one_half_precision_nan() {
    Parts p = parts(corpus().at("cases").at(0).at("decoded"));
    p.precise = std::numeric_limits<double>::quiet_NaN();
    p.ratio = std::numeric_limits<float>::quiet_NaN();
    const Bytes bytes = encoded(made(p));
    // Key 2 and key 3, each `f9 7e 00` (docs/adr/0086, section 9.2).
    const Bytes ratio{0x02, 0xf9, 0x7e, 0x00, 0x03, 0xf9, 0x7e, 0x00};
    CHECK(std::search(bytes.begin(), bytes.end(), ratio.begin(), ratio.end()) != bytes.end(),
          "both floats are written as f9 7e 00");
}

}  // namespace

int main() {
    every_case_is_written_in_its_one_form_and_read_back();
    a_variant_encoding_is_read_and_written_again_in_its_one_form();
    what_is_no_value_of_the_type_is_refused_by_the_name_of_its_class();
    a_value_its_types_hold_and_no_encoding_carries_is_refused_before_a_byte();
    a_nan_is_written_as_the_one_half_precision_nan();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("cbor_composed_generated_test: all checks passed");
    return 0;
}
