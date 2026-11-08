// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The encode of a C++ codec printed from its plan, over a type the consumer
// owns (docs/adr/0081, step 4).
//
// A printed codec's value presents its elements through accessors and is made
// only by its constructor; its encode, `Wire::encode_source`, reads each
// element through a customization point: the member function of the element's
// name where the type has one, otherwise a function of that name the type's own
// namespace declares, found by argument-dependent lookup. The numerical harness
// holds the generated value to the reference bytes; this file holds a
// consumer's types to the generated value's bytes, one presenting its elements
// by member functions, one by free functions, one by both (the member is
// read), and states at compile time which types are sources.
//
// The codecs are tests/forge/resources/codec_simple_frame.scxml (big endian:
// msgId u8, length u8, payload u16), codec_little_endian.scxml (little
// endian: sensorId u8, value u16, status u8) and codec_length_ref.scxml
// (msgId u8, then the length of the payload, then a payload of at most 32
// bytes), whose headers the harness's generation writes. The last presents
// its payload as `Bytes<32>`, which holds the bound, and derives the length,
// which it does not present. codec_present_if_basic.scxml (a carrier whose bit
// 0 says whether a u16 `seq` follows) presents `seq` as an optional value and
// derives the bit; codec_length_ref_dotted_basic.scxml (a carrier whose high
// nibble holds the length of the payload after it) derives the nibble.

#include "codec_chain_depth_gated.h"
#include "codec_chain_depth_plain.h"
#include "codec_chain_flag_carrier.h"
#include "codec_chain_prev_carrier.h"
#include "codec_embed_basic.h"
#include "codec_ext_encoding_info.h"
#include "codec_init_syn_body.h"
#include "codec_init_syn_envelope.h"
#include "codec_length_ref.h"
#include "codec_length_ref_dotted_basic.h"
#include "codec_little_endian.h"
#include "codec_origin_envelope.h"
#include "codec_present_if_basic.h"
#include "codec_present_if_disjunction.h"
#include "codec_present_if_length_ref.h"
#include "codec_repeat_basic.h"
#include "codec_repeat_present_if_basic.h"
#include "codec_repeat_unconditional_count.h"
#include "codec_simple_frame.h"
#include "codec_variant_peek_basic.h"
#include "codec_zenoh_decl_subscriber.h"
#include "codec_zenoh_declaration.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_ext_envelope.h"
#include "codec_zenoh_hello.h"
#include "codec_zenoh_interest_body.h"
#include "codec_zenoh_locator.h"
#include "codec_zenoh_open_body.h"
#include "codec_zenoh_query_value.h"
#include "codec_zenoh_value_entry.h"
#include "codec_zenoh_value_zbuf.h"
#include "codec_zenoh_wireexpr.h"

#include <cstdint>
#include <cstdio>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace {

int failures = 0;

#define CHECK(cond, ...)                                                                                               \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            std::fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                  \
            std::fprintf(stderr, __VA_ARGS__);                                                                         \
            std::fprintf(stderr, "\n");                                                                                \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

}  // namespace

namespace consumer {

// A frame the consumer keeps in its own shape, presenting the codec's
// elements by member functions; the widths differ from the codec's, and each
// converts to the element's type.
struct MemberFrame {
    unsigned id;
    unsigned len;
    unsigned body;

    [[nodiscard]] std::uint8_t msgId() const noexcept {
        return static_cast<std::uint8_t>(id);
    }

    [[nodiscard]] std::uint8_t length() const noexcept {
        return static_cast<std::uint8_t>(len);
    }

    [[nodiscard]] std::uint16_t payload() const noexcept {
        return static_cast<std::uint16_t>(body);
    }
};

// A frame that declares nothing of the codec: the functions its namespace
// declares present it, without opening an SCE namespace.
struct PlainFrame {
    std::uint8_t id;
    std::uint8_t len;
    std::uint16_t body;
};

[[nodiscard]] std::uint8_t msgId(const PlainFrame &f) noexcept {
    return f.id;
}

[[nodiscard]] std::uint8_t length(const PlainFrame &f) noexcept {
    return f.len;
}

[[nodiscard]] std::uint16_t payload(const PlainFrame &f) noexcept {
    return f.body;
}

// A frame with a member function and a free function of one element's name:
// the member is the one read, as `std::ranges::begin` reads a range's member.
struct BothFrame {
    std::uint8_t id;
    std::uint8_t len;
    std::uint16_t body;

    [[nodiscard]] std::uint8_t msgId() const noexcept {
        return id;
    }

    [[nodiscard]] std::uint8_t length() const noexcept {
        return len;
    }

    [[nodiscard]] std::uint16_t payload() const noexcept {
        return body;
    }
};

[[nodiscard]] std::uint8_t msgId(const BothFrame &) noexcept {
    return 0xEE;
}

// A sensor reading presented by free functions, for the little-endian codec.
struct Reading {
    std::uint8_t sensor;
    std::uint16_t raw;
    std::uint8_t flags;
};

[[nodiscard]] std::uint8_t sensorId(const Reading &r) noexcept {
    return r.sensor;
}

[[nodiscard]] std::uint16_t value(const Reading &r) noexcept {
    return r.raw;
}

[[nodiscard]] std::uint8_t status(const Reading &r) noexcept {
    return r.flags;
}

// A frame whose reads may throw (a read the consumer did not declare
// `noexcept`): it is a source, and its encode is not `noexcept`.
struct ThrowingFrame {
    [[nodiscard]] std::uint8_t msgId() const {
        return 1;
    }

    [[nodiscard]] std::uint8_t length() const noexcept {
        return 2;
    }

    [[nodiscard]] std::uint16_t payload() const noexcept {
        return 3;
    }
};

// A frame that presents two of the three elements.
struct PartialFrame {
    [[nodiscard]] std::uint8_t msgId() const noexcept {
        return 1;
    }

    [[nodiscard]] std::uint8_t length() const noexcept {
        return 2;
    }
};

// A message that owns its payload within the codec's bound and presents it by
// a member function; the length is the codec's to derive, so it presents none.
struct Message {
    std::uint8_t id;
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 32> body;

    [[nodiscard]] std::uint8_t msgId() const noexcept {
        return id;
    }

    [[nodiscard]] const ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 32> &payload() const noexcept {
        return body;
    }
};

// A message whose payload is held to a tighter bound than the codec's, and
// presented by a function of its namespace: within the codec's bound by its
// type, so it converts unchecked.
struct ShortMessage {
    std::uint8_t id;
    ::SCE::Forge::Bytes<16> body;
};

[[nodiscard]] std::uint8_t msgId(const ShortMessage &m) noexcept {
    return m.id;
}

[[nodiscard]] ::SCE::Forge::Bytes<16> payload(const ShortMessage &m) noexcept {
    return m.body;
}

// A message whose payload no type bounds: the encode would have to refuse a
// long one, so it is no source.
struct UnboundedMessage {
    [[nodiscard]] std::uint8_t msgId() const noexcept {
        return 1;
    }

    [[nodiscard]] std::vector<std::uint8_t> payload() const {
        return {};
    }
};

// A message whose payload is held to a looser bound than the codec's: no
// source either, since its type admits a payload the codec has no frame for.
struct LongMessage {
    [[nodiscard]] std::uint8_t msgId() const noexcept {
        return 1;
    }

    [[nodiscard]] ::SCE::Forge::Bytes<64> payload() const noexcept {
        return {};
    }
};

// A reading whose sequence number is there at times: it presents it as an
// optional value, and presents its flags with the presence bit clear, which
// the encode derives.
struct SequencedReading {
    [[nodiscard]] std::uint8_t flags() const noexcept {
        return 0;
    }

    [[nodiscard]] std::optional<std::uint16_t> seq() const noexcept {
        return std::uint16_t{42};
    }
};

// A fragment of the consumer's own: a source of the fragment codec.
struct OwnFragment {
    std::uint16_t value;

    [[nodiscard]] std::uint16_t seq() const noexcept {
        return value;
    }
};

// Fragments held in a list of the consumer's own type, to a tighter bound than
// the codec's: a list of sources of the entry codec, which the codec writes
// entry by entry.
struct Fragments {
    [[nodiscard]] ::SCE::Forge::Bounded<std::vector<OwnFragment>, 4> frags() const {
        return *::SCE::Forge::Bounded<std::vector<OwnFragment>, 4>::make({OwnFragment{0x0102}, OwnFragment{0x0304}});
    }
};

// A locator of the consumer's own: a source of the locator codec, which an
// endpoint presents as its body.
struct OwnLocator {
    [[nodiscard]] ::SCE::Forge::Text<255> locator() const noexcept {
        return *::SCE::Forge::Text<255>::make(std::string_view("tcp/x"));
    }
};

// An endpoint presenting its locator as a source of the locator codec, not
// as the generated value: the body is encoded over what it presents.
struct Endpoint {
    [[nodiscard]] std::uint8_t tag() const noexcept {
        return 7;
    }

    [[nodiscard]] OwnLocator locator() const noexcept {
        return {};
    }
};

}  // namespace consumer

namespace detect {

/// Whether `T` presents an element named `len`.
template <class T, class = void> struct presents_len : std::false_type {};

template <class T> struct presents_len<T, std::void_t<decltype(std::declval<const T &>().len())>> : std::true_type {};

}  // namespace detect

namespace Frame = ::SCE::Generated::CodecSimpleFrame;
namespace Sensor = ::SCE::Generated::CodecLittleEndian;

static_assert(Frame::is_source<Frame::CodecSimpleFrame>::value, "the generated value is a source of its own codec");
static_assert(Frame::is_source<consumer::MemberFrame>::value, "member functions present a source");
static_assert(Frame::is_source<consumer::PlainFrame>::value, "functions of the type's namespace present a source");
static_assert(Frame::is_source<consumer::BothFrame>::value,
              "a member and a free function of one name present a source");
static_assert(!Frame::is_source<consumer::PartialFrame>::value, "a type missing an element is not a source");
static_assert(!Frame::is_source<int>::value, "a type presenting nothing is not a source");
static_assert(!Frame::is_source<consumer::Reading>::value, "a source of another codec is not a source of this one");
static_assert(Sensor::is_source<consumer::Reading>::value,
              "functions of the type's namespace present the little-endian codec");

// The encode throws only what a source's reads throw.
static_assert(Frame::is_source<consumer::ThrowingFrame>::value, "a read that may throw still presents");
static_assert(Frame::is_nothrow_source<consumer::MemberFrame>::value, "noexcept members read without throwing");
static_assert(!Frame::is_nothrow_source<consumer::ThrowingFrame>::value, "one read that may throw is enough");
static_assert(!Frame::is_nothrow_source<consumer::PartialFrame>::value, "a type that is no source is not nothrow");
static_assert(noexcept(Frame::Wire::encode_source(std::declval<const consumer::PlainFrame &>(),
                                                  std::declval<::SCE::Forge::SceSink &>())),
              "the encode over a nothrow source is noexcept");
static_assert(!noexcept(Frame::Wire::encode_source(std::declval<const consumer::ThrowingFrame &>(),
                                                   std::declval<::SCE::Forge::SceSink &>())),
              "the encode over a source that may throw is not noexcept");
static_assert(noexcept(std::declval<const Frame::CodecSimpleFrame &>().encode(std::declval<::SCE::Forge::SceSink &>())),
              "the generated value's encode stays noexcept");

namespace Length = ::SCE::Generated::CodecLengthRef;

// A byte string is presented within its bound, and a length the encode derives
// is no element (docs/adr/0081 §3).
static_assert(
    std::is_same<decltype(std::declval<const Length::CodecLengthRef &>().payload()), ::SCE::Forge::Bytes<32>>::value,
    "the payload is presented within its bound");
static_assert(!detect::presents_len<Length::CodecLengthRef>::value, "the derived length is no element");
static_assert(Length::is_source<Length::CodecLengthRef>::value, "the generated value is a source of its own codec");
static_assert(Length::is_source<consumer::Message>::value, "an owned payload within the bound presents it");
static_assert(Length::is_source<consumer::ShortMessage>::value, "a payload within a tighter bound presents it");
static_assert(!Length::is_source<consumer::UnboundedMessage>::value, "a payload no type bounds is no source");
static_assert(!Length::is_source<consumer::LongMessage>::value, "a payload of a looser bound is no source");
static_assert(Length::is_nothrow_source<consumer::Message>::value, "viewing an owned payload throws nothing");
static_assert(noexcept(std::declval<const Length::CodecLengthRef &>().encode(std::declval<::SCE::Forge::SceSink &>())),
              "the encode of a value that holds a byte string is noexcept");

namespace {

// The bytes `Wire`, a codec's encode over any source, writes for `src`.
template <class Wire, class S> std::vector<std::uint8_t> encoded(const S &src) {
    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    const auto error = Wire::encode_source(src, sink);
    CHECK(!error.has_value(), "encode_source over a vector sink refused");
    return out;
}

void the_generated_value_is_made_and_read_by_its_constructor_and_accessors() {
    constexpr Frame::CodecSimpleFrame frame{0x01, 0x02, 0x0304};
    static_assert(frame.msgId() == 0x01 && frame.length() == 0x02 && frame.payload() == 0x0304,
                  "the constructor takes the elements in declaration order");
    const std::vector<std::uint8_t> expected{0x01, 0x02, 0x03, 0x04};
    CHECK(frame.encode_to_vec() == expected, "the generated value encodes big endian");

    ::SCE::Forge::SceCursor cursor(expected.data(), expected.size());
    const auto decoded = Frame::CodecSimpleFrame::decode(cursor);
    CHECK(decoded.has_value(), "the generated value decodes its own bytes");
    if (decoded) {
        CHECK(decoded->msgId() == 0x01 && decoded->length() == 0x02 && decoded->payload() == 0x0304,
              "the decoded value presents the elements it was made from");
    }

    constexpr Frame::CodecSimpleFrame zero{};
    static_assert(zero.msgId() == 0 && zero.length() == 0 && zero.payload() == 0, "a default-made value holds zeros");
}

void a_consumer_type_encodes_as_the_generated_value_does() {
    const Frame::CodecSimpleFrame generated{0xA5, 0x10, 0xBEEF};
    const auto expected = generated.encode_to_vec();

    CHECK(encoded<Frame::Wire>(consumer::MemberFrame{0xA5, 0x10, 0xBEEF}) == expected,
          "member functions encode the generated value's bytes");
    CHECK(encoded<Frame::Wire>(consumer::PlainFrame{0xA5, 0x10, 0xBEEF}) == expected,
          "free functions encode the generated value's bytes");
    CHECK(encoded<Frame::Wire>(consumer::BothFrame{0xA5, 0x10, 0xBEEF}) == expected,
          "the member, not the free function, is read");
    CHECK(encoded<Frame::Wire>(generated) == expected,
          "the generated value encodes through encode_source as through encode");

    const Sensor::CodecLittleEndian reading{0x07, 0x1234, 0x80};
    const std::vector<std::uint8_t> little{0x07, 0x34, 0x12, 0x80};
    CHECK(reading.encode_to_vec() == little, "the generated value encodes little endian");
    CHECK(encoded<Sensor::Wire>(consumer::Reading{0x07, 0x1234, 0x80}) == little,
          "free functions encode the little-endian codec's bytes");
}

void a_length_is_derived_from_the_payload_the_source_presents() {
    const std::vector<std::uint8_t> body{0xDE, 0xAD, 0xBE};
    const auto payload = ::SCE::Forge::Bytes<32>::make(body);
    CHECK(payload.has_value(), "three bytes lie within the bound");
    if (!payload) {
        return;
    }
    const Length::CodecLengthRef generated{0x42, *payload};
    const std::vector<std::uint8_t> expected{0x42, 0x03, 0xDE, 0xAD, 0xBE};
    CHECK(generated.encode_to_vec() == expected, "the length is the payload's, written before it");
    CHECK(generated.payload() == body, "the value presents the payload it was made from");

    ::SCE::Forge::SceCursor cursor(expected.data(), expected.size());
    const auto decoded = Length::CodecLengthRef::decode(cursor);
    CHECK(decoded.has_value() && decoded->msgId() == 0x42 && decoded->payload() == body,
          "the decoded value presents the elements it was made from");

    const auto owned = ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 32>::make(body);
    CHECK(owned.has_value(), "an owned payload within the bound is made");
    if (owned) {
        CHECK(encoded<Length::Wire>(consumer::Message{0x42, *owned}) == expected,
              "an owned payload encodes the generated value's bytes");
    }
    const auto short_body = ::SCE::Forge::Bytes<16>::make(body);
    CHECK(short_body.has_value(), "three bytes lie within the tighter bound");
    if (short_body) {
        CHECK(encoded<Length::Wire>(consumer::ShortMessage{0x42, *short_body}) == expected,
              "a payload of a tighter bound encodes the generated value's bytes");
    }

    // The fewest bytes the bound admits: none, and a length of 0, through a
    // sink over the caller's buffer.
    const Length::CodecLengthRef empty{};
    std::uint8_t buffer[2] = {0xFF, 0xFF};
    ::SCE::Forge::SpanSink sink(buffer, sizeof buffer);
    CHECK(!empty.encode(sink).has_value() && sink.position() == 2 && buffer[0] == 0 && buffer[1] == 0,
          "an empty payload encodes as a zero length and nothing after it");
}

void a_payload_past_its_bound_cannot_be_made() {
    const std::vector<std::uint8_t> long_body(33, 0xAA);
    CHECK(!::SCE::Forge::Bytes<32>::make(long_body).has_value(), "33 bytes lie past the bound of 32");

    // A frame whose length says more than the bound is refused by the decode,
    // as the value could not hold it.
    std::vector<std::uint8_t> frame{0x01, 33};
    frame.insert(frame.end(), long_body.begin(), long_body.end());
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    CHECK(!Length::CodecLengthRef::decode(cursor).has_value() &&
              cursor.last_refusal() ==
                  std::optional<::SCE::Forge::CodecError>(::SCE::Forge::CodecError::LengthOutOfRange),
          "a length past the bound is refused as LengthOutOfRange");
}

namespace Gated = ::SCE::Generated::CodecPresentIfBasic;
namespace Dotted = ::SCE::Generated::CodecLengthRefDottedBasic;

static_assert(std::is_same<decltype(std::declval<const Gated::CodecPresentIfBasic &>().seq()),
                           std::optional<std::uint16_t>>::value,
              "an element a gate decides is presented as an optional value");
static_assert(Gated::is_source<consumer::SequencedReading>::value,
              "a type presenting the optional element is a source");

void a_presence_flag_is_derived_from_the_element_it_gates() {
    // The carrier's flag is what the element says, whatever the value passed.
    const Gated::CodecPresentIfBasic present{0x00, std::uint16_t{0x1234}};
    CHECK(present.encode_to_vec() == (std::vector<std::uint8_t>{0x01, 0x12, 0x34}),
          "a present element sets its flag and is written");
    const Gated::CodecPresentIfBasic absent{0x01, std::nullopt};
    CHECK(absent.encode_to_vec() == std::vector<std::uint8_t>{0x00}, "an absent element clears its flag");
    const Gated::CodecPresentIfBasic kept{0xF0, std::nullopt};
    CHECK(kept.encode_to_vec() == std::vector<std::uint8_t>{0xF0}, "the carrier's other bits are kept");

    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    CHECK(!Gated::Wire::encode_source(consumer::SequencedReading{}, sink).has_value() &&
              out == (std::vector<std::uint8_t>{0x01, 0x00, 0x2A}),
          "a consumer's optional element encodes as the generated value's does");

    const std::vector<std::uint8_t> frame{0x01, 0x12, 0x34};
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    const auto decoded = Gated::CodecPresentIfBasic::decode(cursor);
    CHECK(decoded.has_value() && decoded->seq() == std::optional<std::uint16_t>(0x1234) && decoded->has_seq(),
          "a decoded value holds the element and the carrier as read");
}

void a_length_held_in_a_flag_is_derived_from_the_bytes() {
    using Payload = decltype(std::declval<const Dotted::CodecLengthRefDottedBasic &>().payload());
    const std::vector<std::uint8_t> two{0xAA, 0xBB};
    const auto payload = Payload::make(two);
    CHECK(payload.has_value(), "two bytes lie within a bound of fifteen");
    if (!payload) {
        return;
    }
    // The carrier's length bits say none; the payload says two.
    const Dotted::CodecLengthRefDottedBasic value{0x01, *payload};
    CHECK(value.encode_to_vec() == (std::vector<std::uint8_t>{0x21, 0xAA, 0xBB}),
          "the length bits hold the payload's length beside the carrier's other bits");
}

namespace Locator = ::SCE::Generated::CodecZenohLocator;

static_assert(std::is_same<decltype(std::declval<const Locator::CodecZenohLocator &>().locator()),
                           ::SCE::Forge::Text<255>>::value,
              "a text is presented as a view held to its bound");

void a_text_is_made_of_utf8_within_its_bound_and_its_length_is_derived() {
    using LocatorText = ::SCE::Forge::Text<255>;
    CHECK(!LocatorText::make(std::string_view("tcp/\xFF")).has_value(), "a text that is not UTF-8 cannot be made");
    const auto text = LocatorText::make(std::string_view("tcp/x"));
    CHECK(text.has_value(), "five ASCII bytes lie within a bound of 255");
    if (!text) {
        return;
    }
    const Locator::CodecZenohLocator value{*text};
    CHECK(value.encode_to_vec() == (std::vector<std::uint8_t>{0x05, 't', 'c', 'p', '/', 'x'}),
          "the length is derived from the text, as a VLE before it");
    CHECK(value.locator() == std::string("tcp/x"), "the accessor views the text the value holds");

    const std::vector<std::uint8_t> frame{0x02, 0xC3, 0x28};
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    CHECK(!Locator::CodecZenohLocator::decode(cursor).has_value(), "a frame whose text is not UTF-8 is refused");
}

namespace Embed = ::SCE::Generated::CodecEmbedBasic;

static_assert(std::is_same<decltype(std::declval<const Embed::CodecEmbedBasic &>().locator()),
                           const Locator::CodecZenohLocator &>::value,
              "a body is presented by reference to the value of its codec");
static_assert(Embed::is_source<consumer::Endpoint>::value,
              "a type presenting its body as a source of the body's codec is a source");

void a_body_is_encoded_over_the_source_its_holder_presents() {
    const std::vector<std::uint8_t> expected{0x07, 0x05, 't', 'c', 'p', '/', 'x'};
    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    CHECK(!Embed::Wire::encode_source(consumer::Endpoint{}, sink).has_value() && out == expected,
          "a body presented as a source of its codec is written by that codec's encode");

    const auto text = ::SCE::Forge::Text<255>::make(std::string_view("tcp/x"));
    CHECK(text.has_value(), "the locator lies within its bound");
    if (!text) {
        return;
    }
    const Embed::CodecEmbedBasic value{7, Locator::CodecZenohLocator{*text}};
    CHECK(value.encode_to_vec() == expected, "the generated value holding its body encodes alike");

    ::SCE::Forge::SceCursor cursor(expected.data(), expected.size());
    const auto decoded = Embed::CodecEmbedBasic::decode(cursor);
    CHECK(decoded.has_value() && decoded->tag() == 7 && decoded->locator().locator() == std::string("tcp/x"),
          "a decoded value holds its body");

    ::SCE::Forge::CountingSink counter;
    CHECK(!Embed::Wire::encode_source(consumer::Endpoint{}, counter).has_value() &&
              counter.position() == expected.size(),
          "a counting sink counts the bytes an encode writes and keeps none");
}

namespace Wireexpr = ::SCE::Generated::CodecZenohWireexpr;
namespace Subscriber = ::SCE::Generated::CodecZenohDeclSubscriber;
namespace Interest = ::SCE::Generated::CodecZenohInterestBody;

void an_input_the_value_says_is_read_off_the_value_and_handed_on() {
    using Suffix = typename decltype(std::declval<const Wireexpr::CodecZenohWireexpr &>().suffix())::value_type;
    const auto suffix = Suffix::make(std::string_view("a/b"));
    CHECK(suffix.has_value(), "a three-byte suffix lies within its bound");
    if (!suffix) {
        return;
    }
    const Wireexpr::CodecZenohWireexpr named{5, *suffix};
    const Wireexpr::CodecZenohWireexpr bare{5, std::nullopt};
    CHECK(Wireexpr::input_n(named) == 1 && Wireexpr::input_n(bare) == 0,
          "the input is set exactly when the suffix it gates is there");
    CHECK(named.encode_to_vec() == (std::vector<std::uint8_t>{0x05, 0x03, 'a', '/', 'b'}),
          "a suffix is written after the id with its length, the encode handed no input");
    CHECK(bare.encode_to_vec() == std::vector<std::uint8_t>{0x05}, "no suffix, no length");

    const Subscriber::CodecZenohDeclSubscriber declaration{7, named};
    CHECK(Subscriber::input_n(declaration) == 1, "a declaration says the input its wire expression says");
    CHECK(declaration.encode_to_vec() == (std::vector<std::uint8_t>{0x07, 0x05, 0x03, 'a', '/', 'b'}),
          "a declaration writes its wire expression as the expression's own encode does");

    // A body a gate decides says the input where the value holds it; where
    // it does not, the flag keeps the bit the header presents.
    const Interest::CodecZenohInterestBody restricted{0x00, named};
    CHECK(restricted.encode_to_vec() == (std::vector<std::uint8_t>{0x30, 0x05, 0x03, 'a', '/', 'b'}),
          "N is what the key expression the value holds says, beside the flag that it is there");
    const Interest::CodecZenohInterestBody open{0x20, std::nullopt};
    CHECK(open.encode_to_vec() == std::vector<std::uint8_t>{0x20}, "no key expression, and N is the header's own");
}

namespace Fragmented = ::SCE::Generated::CodecRepeatBasic;
using Fragment = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem;

static_assert(std::is_same<decltype(std::declval<const Fragmented::CodecRepeatBasic &>().frags()),
                           ::SCE::Forge::List<Fragment, 32>>::value,
              "a list is presented as a view of its entries held to its bound");
static_assert(Fragmented::is_source<consumer::Fragments>::value,
              "a list of the consumer's own entry sources, within the bound, is a source");

void a_list_is_written_entry_by_entry_and_its_count_derived() {
    const std::vector<std::uint8_t> expected{0x02, 0x01, 0x02, 0x03, 0x04};
    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    CHECK(!Fragmented::Wire::encode_source(consumer::Fragments{}, sink).has_value() && out == expected,
          "each entry the consumer's list holds is written by the entry codec, the count before them");

    const std::vector<Fragment> entries{Fragment{0x0102}, Fragment{0x0304}};
    const auto frags = ::SCE::Forge::List<Fragment, 32>::make(entries);
    CHECK(frags.has_value(), "two entries lie within a bound of 32");
    if (!frags) {
        return;
    }
    const Fragmented::CodecRepeatBasic value{*frags};
    CHECK(value.encode_to_vec() == expected, "the generated value holding its list encodes alike");

    ::SCE::Forge::SceCursor cursor(expected.data(), expected.size());
    const auto decoded = Fragmented::CodecRepeatBasic::decode(cursor);
    CHECK(decoded.has_value() && decoded->frags().size() == 2 && decoded->frags()[1].seq() == 0x0304,
          "a decoded value holds its entries");
}

namespace GatedList = ::SCE::Generated::CodecRepeatPresentIfBasic;
namespace Hello = ::SCE::Generated::CodecZenohHello;

void a_count_a_gate_decides_is_derived_where_the_list_is_there() {
    // Bit 0 of the carrier says the list is there; its count follows, then
    // its entries. The carrier the value presents claims it is there.
    const std::vector<Fragment> entries{Fragment{0xBEEF}, Fragment{0xFACE}};
    const auto elems = ::SCE::Forge::List<Fragment, 32>::make(entries);
    const std::vector<Fragment> none;
    const auto empty = ::SCE::Forge::List<Fragment, 32>::make(none);
    CHECK(elems.has_value() && empty.has_value(), "two entries and none lie within a bound of 32");
    if (!elems || !empty) {
        return;
    }
    const std::vector<std::uint8_t> two{0x01, 0x02, 0xBE, 0xEF, 0xFA, 0xCE};
    const GatedList::CodecRepeatPresentIfBasic holds_two{0x00, *elems};
    const GatedList::CodecRepeatPresentIfBasic holds_none{0x00, *empty};
    const GatedList::CodecRepeatPresentIfBasic holds_no_list{0x01, std::nullopt};
    CHECK(holds_two.encode_to_vec() == two, "a list the value holds sets its flag and is counted before its entries");
    CHECK(holds_none.encode_to_vec() == (std::vector<std::uint8_t>{0x01, 0x00}),
          "an empty list the value holds is there, of count zero");
    CHECK(holds_no_list.encode_to_vec() == std::vector<std::uint8_t>{0x00},
          "no list writes neither its count nor its entries, and clears the flag the carrier presents");

    ::SCE::Forge::SceCursor cursor(two.data(), two.size());
    const auto decoded = GatedList::CodecRepeatPresentIfBasic::decode(cursor);
    CHECK(decoded.has_value() && decoded->elems().has_value() && decoded->elems()->size() == 2 &&
              (*decoded->elems())[1].seq() == 0xFACE,
          "a decoded value holds the entries its count said");

    // A hello says L, the flag its holder sets, where it holds locators; the
    // count is a VLE before them.
    const std::vector<std::uint8_t> zid_byte{0xAB};
    const auto zid = ::SCE::Forge::Bytes<16, 1>::make(zid_byte);
    const auto text = ::SCE::Forge::Text<255>::make(std::string_view("tcp/x"));
    CHECK(zid.has_value() && text.has_value(), "one byte and five ASCII bytes lie within their bounds");
    if (!zid || !text) {
        return;
    }
    const std::vector<Locator::CodecZenohLocator> one{Locator::CodecZenohLocator{*text}};
    const auto locators = ::SCE::Forge::List<Locator::CodecZenohLocator, ::SCE::Forge::kUnbounded>::make(one);
    CHECK(locators.has_value(), "one locator lies within no stated bound");
    if (!locators) {
        return;
    }
    const Hello::CodecZenohHello with{0x01, 0x02, *zid, *locators};
    const Hello::CodecZenohHello without{0x01, 0x02, *zid, std::nullopt};
    CHECK(with.encode_to_vec() == (std::vector<std::uint8_t>{0x01, 0x02, 0xAB, 0x01, 0x05, 't', 'c', 'p', '/', 'x'}),
          "the locators the value holds are counted before them");
    CHECK(without.encode_to_vec() == (std::vector<std::uint8_t>{0x01, 0x02, 0xAB}), "no locators writes no count");
    CHECK(Hello::input_l(with) == 1 && Hello::input_l(without) == 0,
          "L is whether the value holds locators, which its holder sets");
}

namespace ExtEntry = ::SCE::Generated::CodecZenohExtEntry;
using ExtUnit = ::SCE::Generated::CodecZenohExtUnit::CodecZenohExtUnit;
using ExtZint = ::SCE::Generated::CodecZenohExtZint::CodecZenohExtZint;
using ExtZbuf = ::SCE::Generated::CodecZenohExtZbuf::CodecZenohExtZbuf;

// A zint extension of the consumer's own: a source of the zint arm's codec.
struct OwnZint {
    [[nodiscard]] std::uint64_t value() const noexcept {
        return 7;
    }
};

// An extension whose body is the consumer's own variant: alternative `i` a
// source of the codec of arm `i`, the catch-all last.
struct OwnExtension {
    [[nodiscard]] std::uint8_t header() const noexcept {
        return 0x01;
    }

    [[nodiscard]] std::variant<ExtUnit, OwnZint, ExtZbuf, ExtEntry::CodecZenohExtEntryDefault> body() const noexcept {
        return OwnZint{};
    }
};

static_assert(ExtEntry::is_source<OwnExtension>::value,
              "a variant of a source of each arm's codec in turn is a body a source may present");

void a_variant_derives_its_tag_from_the_arm_it_holds() {
    // The header says no arm; the arm the value holds says the tag.
    const ExtEntry::CodecZenohExtEntry entry{0x01,
                                             ExtEntry::CodecZenohExtEntryVariant{std::in_place_index<1>, ExtZint{5}}};
    CHECK(entry.encode_to_vec() == (std::vector<std::uint8_t>{0x21, 0x05}),
          "the zint arm sets the enc bits to its tag beside the header's other bits");

    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    CHECK(!ExtEntry::Wire::encode_source(OwnExtension{}, sink).has_value() &&
              out == (std::vector<std::uint8_t>{0x21, 0x07}),
          "a consumer's own arm is written by the arm's codec, its tag derived alike");

    const ExtEntry::CodecZenohExtEntry unknown{
        0x01, ExtEntry::CodecZenohExtEntryVariant{ExtEntry::CodecZenohExtEntryDefault{3, ExtUnit{}}}};
    CHECK(unknown.encode_to_vec() == std::vector<std::uint8_t>{0x61},
          "the catch-all writes the tag it holds into the enc bits");

    const std::vector<std::uint8_t> frame{0x21, 0x05};
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    const auto decoded = ExtEntry::CodecZenohExtEntry::decode(cursor);
    CHECK(decoded.has_value() && decoded->body().index() == 1 && std::get<1>(decoded->body()).value() == 5,
          "a decoded value holds the arm its tag names");
}

namespace DepthPlain = ::SCE::Generated::CodecChainDepthPlain;
namespace DepthEntryNs = ::SCE::Generated::CodecChainDepthEntry;
using DepthEntry = DepthEntryNs::CodecChainDepthEntry;

void a_chain_sets_the_flag_its_entries_end_on() {
    // The first entry presents Z clear and the last presents it set; the
    // chain sets it on every entry but the last, whatever each presents.
    using Chain = decltype(std::declval<const DepthPlain::CodecChainDepthPlain &>().extensions());
    const std::vector<DepthEntry> entries{DepthEntry{0x01}, DepthEntry{0x82}};
    const auto chain = Chain::make(entries);
    CHECK(chain.has_value(), "two entries lie within the chain's bound");
    if (!chain) {
        return;
    }
    const DepthPlain::CodecChainDepthPlain value{0x00, *chain};
    CHECK(value.encode_to_vec() == (std::vector<std::uint8_t>{0x00, 0x81, 0x02}),
          "Z is set on the first entry and cleared on the last");

    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    DepthEntryNs::Overrides closed{};
    closed.header_z = false;
    CHECK(!DepthEntryNs::Wire::encode_source(DepthEntry{0x83}, sink, closed).has_value() &&
              out == std::vector<std::uint8_t>{0x03},
          "a flag a parent sets replaces the one the entry presents");
}

namespace Envelope = ::SCE::Generated::CodecOriginEnvelope;
using Leaf = ::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf;
using Note = decltype(std::declval<const Envelope::CodecOriginEnvelope &>().note());
using Leaves = decltype(std::declval<const Envelope::CodecOriginEnvelope &>().items());

// A holder of the consumer's own whose optional body is the generated body's
// value where it is there; the header it presents claims the body is there,
// which the encode does not believe.
struct Holder {
    bool there;
    std::vector<Leaf> leaves;

    [[nodiscard]] std::uint8_t hdr() const noexcept {
        return 0x01;
    }

    [[nodiscard]] Note note() const noexcept {
        return *Note::make(std::string_view());
    }

    [[nodiscard]] Leaf required() const {
        return Leaf{};
    }

    [[nodiscard]] std::optional<Leaf> optional() const {
        return there ? std::optional<Leaf>{Leaf{}} : std::nullopt;
    }

    [[nodiscard]] Leaves items() const {
        return *Leaves::make(leaves);
    }
};

// The same holder presenting the body as always there: no source of a codec
// whose body a gate decides.
struct HolderWithoutGate {
    std::vector<Leaf> leaves;

    [[nodiscard]] std::uint8_t hdr() const noexcept {
        return 0;
    }

    [[nodiscard]] Note note() const noexcept {
        return *Note::make(std::string_view());
    }

    [[nodiscard]] Leaf required() const {
        return Leaf{};
    }

    [[nodiscard]] Leaf optional() const {
        return Leaf{};
    }

    [[nodiscard]] Leaves items() const {
        return *Leaves::make(leaves);
    }
};

static_assert(Envelope::is_source<Holder>::value,
              "a holder presenting a gated body as an optional source of its codec is a source");
static_assert(!Envelope::is_source<HolderWithoutGate>::value,
              "a holder presenting a gated body as always there is no source");

void a_body_a_gate_decides_is_written_where_it_is_present() {
    // The header's bit 0 says the optional body is there; the note's length and
    // its no bytes follow, the count of the list, then the required body, the
    // optional one where present, and the list. An empty leaf is its length of
    // no bytes, one zero byte.
    const std::vector<std::uint8_t> with{0x01, 0x00, 0x00, 0x00, 0x00};
    const std::vector<std::uint8_t> without{0x00, 0x00, 0x00, 0x00};
    const std::vector<Leaf> none;
    const Note empty = *Note::make(std::string_view());
    const Envelope::CodecOriginEnvelope present{0x00, empty, Leaf{}, Leaf{}, *Leaves::make(none)};
    const Envelope::CodecOriginEnvelope absent{0x01, empty, Leaf{}, std::nullopt, *Leaves::make(none)};
    CHECK(present.encode_to_vec() == with, "a present body is written and its flag set");
    CHECK(absent.encode_to_vec() == without, "an absent body is not written and its flag cleared");

    ::SCE::Forge::SceCursor cursor(with.data(), with.size());
    const auto decoded = Envelope::CodecOriginEnvelope::decode(cursor);
    CHECK(decoded.has_value() && decoded->optional().has_value(), "a decoded value holds the body it read");
    ::SCE::Forge::SceCursor absent_cursor(without.data(), without.size());
    const auto decoded_absent = Envelope::CodecOriginEnvelope::decode(absent_cursor);
    CHECK(decoded_absent.has_value() && !decoded_absent->optional().has_value(),
          "a decoded value holds no body it did not read");

    const Holder holds_none{false, {}};
    const Holder holds_one{true, {}};
    std::vector<std::uint8_t> out_none;
    ::SCE::Forge::VectorSink sink_none(out_none);
    CHECK(!Envelope::Wire::encode_source(holds_none, sink_none).has_value() && out_none == without,
          "the flag is derived from the optional the source presents, not from its header");
    std::vector<std::uint8_t> out_one;
    ::SCE::Forge::VectorSink sink_one(out_one);
    CHECK(!Envelope::Wire::encode_source(holds_one, sink_one).has_value() && out_one == with,
          "a body the source presents is written over the source it holds");
}

namespace DepthGated = ::SCE::Generated::CodecChainDepthGated;
using GatedChain = decltype(std::declval<const DepthGated::CodecChainDepthGated &>().extensions())::value_type;
using GatedPayload = decltype(std::declval<const DepthGated::CodecChainDepthGated &>().payload());

// A holder of the consumer's own whose chain is its own entries where it is
// there; the header it presents claims the chain is there, which the encode
// does not believe.
struct ChainHolder {
    std::optional<std::vector<DepthEntry>> entries;

    [[nodiscard]] std::uint8_t header() const noexcept {
        return 0x80;
    }

    [[nodiscard]] std::optional<GatedChain> extensions() const {
        if (!entries) {
            return std::nullopt;
        }
        return GatedChain::make(*entries);
    }

    [[nodiscard]] GatedPayload payload() const {
        static const std::vector<std::uint8_t> byte{0xAA};
        return *GatedPayload::make(byte);
    }
};

// The same holder presenting the chain as always there: no source of a codec
// whose chain a gate decides.
struct ChainHolderWithoutGate {
    std::vector<DepthEntry> entries;

    [[nodiscard]] std::uint8_t header() const noexcept {
        return 0x80;
    }

    [[nodiscard]] GatedChain extensions() const {
        return *GatedChain::make(entries);
    }

    [[nodiscard]] GatedPayload payload() const {
        static const std::vector<std::uint8_t> byte{0xAA};
        return *GatedPayload::make(byte);
    }
};

static_assert(DepthGated::is_source<ChainHolder>::value,
              "a holder presenting a gated chain as an optional list of entry sources is a source");
static_assert(!DepthGated::is_source<ChainHolderWithoutGate>::value,
              "a holder presenting a gated chain as always there is no source");

void a_list_a_gate_decides_is_walked_where_it_is_present() {
    // Z in bit 7 of the header says the chain is there; its entries follow,
    // Z set on each but the last, then the payload's length and its byte.
    const std::vector<std::uint8_t> with{0x80, 0x81, 0x02, 0x01, 0xAA};
    const std::vector<std::uint8_t> without{0x00, 0x01, 0xAA};
    const std::vector<DepthEntry> entries{DepthEntry{0x01}, DepthEntry{0x82}};
    const auto chain = GatedChain::make(entries);
    const std::vector<std::uint8_t> byte{0xAA};
    const auto payload = GatedPayload::make(byte);
    CHECK(chain.has_value() && payload.has_value(), "two entries and one byte lie within their bounds");
    if (!chain || !payload) {
        return;
    }
    const DepthGated::CodecChainDepthGated present{0x00, *chain, *payload};
    const DepthGated::CodecChainDepthGated absent{0x80, std::nullopt, *payload};
    CHECK(present.encode_to_vec() == with, "a present chain is walked and its flag set");
    CHECK(absent.encode_to_vec() == without, "an absent chain is not written and its flag cleared");

    ::SCE::Forge::SceCursor cursor(with.data(), with.size());
    const auto decoded = DepthGated::CodecChainDepthGated::decode(cursor);
    CHECK(decoded.has_value() && decoded->extensions().has_value() && decoded->extensions()->size() == 2 &&
              (*decoded->extensions())[1].header() == 0x02 && decoded->payload() == byte,
          "a decoded value holds the chain it read and the payload after it");
    ::SCE::Forge::SceCursor absent_cursor(without.data(), without.size());
    const auto decoded_absent = DepthGated::CodecChainDepthGated::decode(absent_cursor);
    CHECK(decoded_absent.has_value() && !decoded_absent->extensions().has_value() && decoded_absent->payload() == byte,
          "a decoded value holds no chain it did not read");

    const ChainHolder holds_none{std::nullopt};
    const ChainHolder holds_two{entries};
    std::vector<std::uint8_t> out_none;
    ::SCE::Forge::VectorSink sink_none(out_none);
    CHECK(!DepthGated::Wire::encode_source(holds_none, sink_none).has_value() && out_none == without,
          "the flag is derived from the optional the source presents, not from its header");
    std::vector<std::uint8_t> out_two;
    ::SCE::Forge::VectorSink sink_two(out_two);
    CHECK(!DepthGated::Wire::encode_source(holds_two, sink_two).has_value() && out_two == with,
          "a chain the source presents is walked over the entries it holds");
}

namespace ExtEnvelope = ::SCE::Generated::CodecZenohExtEnvelope;
using Extensions = decltype(std::declval<const ExtEnvelope::CodecZenohExtEnvelope &>().extensions());
namespace OpenBody = ::SCE::Generated::CodecZenohOpenBody;
using Cookie = decltype(std::declval<const OpenBody::CodecZenohOpenBody &>().cookie())::value_type;

static_assert(Extensions::most == ::SCE::Forge::kUnbounded,
              "a chain the protocol states no depth of is held to no bound");
static_assert(Cookie::most == ::SCE::Forge::kUnbounded,
              "a byte string the protocol states no size of is held to no bound");

// A holder of the consumer's own whose chain is held to a bound of its own,
// which lies within the codec's none.
struct TwoExtensions {
    std::vector<ExtEntry::CodecZenohExtEntry> entries;

    [[nodiscard]] std::uint8_t header_flags() const noexcept {
        return 0x00;
    }

    [[nodiscard]] ::SCE::Forge::List<ExtEntry::CodecZenohExtEntry, 2, 1> extensions() const {
        return *::SCE::Forge::List<ExtEntry::CodecZenohExtEntry, 2, 1>::make(entries);
    }
};

static_assert(ExtEnvelope::is_source<TwoExtensions>::value,
              "a list of a bound of the consumer's own lies within a list of no bound");

void a_list_and_bytes_of_no_stated_bound_are_held_to_none() {
    // Two unit extensions, Z set on the first and cleared on the last.
    const std::vector<ExtEntry::CodecZenohExtEntry> entries{
        ExtEntry::CodecZenohExtEntry{0x01, ExtEntry::CodecZenohExtEntryVariant{std::in_place_index<0>, ExtUnit{}}},
        ExtEntry::CodecZenohExtEntry{0x02, ExtEntry::CodecZenohExtEntryVariant{std::in_place_index<0>, ExtUnit{}}}};
    const std::vector<std::uint8_t> frame{0x00, 0x81, 0x02};
    const auto extensions = Extensions::make(entries);
    CHECK(extensions.has_value(), "two entries lie within no bound");
    if (!extensions) {
        return;
    }
    const ExtEnvelope::CodecZenohExtEnvelope envelope{0x00, *extensions};
    CHECK(envelope.encode_to_vec() == frame, "each entry is written, the chain's flag set on all but the last");
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    const auto decoded = ExtEnvelope::CodecZenohExtEnvelope::decode(cursor);
    CHECK(decoded.has_value() && decoded->extensions().size() == 2, "a decoded value holds every entry it read");

    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    CHECK(!ExtEnvelope::Wire::encode_source(TwoExtensions{entries}, sink).has_value() && out == frame,
          "a consumer's list of a bound of its own encodes alike");

    // A cookie of a bound of the caller's own converts to one of no bound; the
    // cookie is there where the parent's A is clear.
    const std::vector<std::uint8_t> bytes{0xAB, 0xCD};
    const auto cookie = ::SCE::Forge::Bytes<4>::make(bytes);
    CHECK(cookie.has_value(), "two bytes lie within a bound of four");
    if (!cookie) {
        return;
    }
    const OpenBody::CodecZenohOpenBody body{1, 2, Cookie{*cookie}};
    const std::vector<std::uint8_t> open{0x01, 0x02, 0x02, 0xAB, 0xCD};
    CHECK(body.encode_to_vec() == open, "the cookie's length is derived and its bytes follow");
    ::SCE::Forge::SceCursor open_cursor(open.data(), open.size());
    const auto decoded_body = OpenBody::CodecZenohOpenBody::decode(open_cursor, 0);
    CHECK(decoded_body.has_value() && decoded_body->cookie().has_value() && *decoded_body->cookie() == bytes,
          "a decoded value holds the cookie it read");
}

namespace Declaration = ::SCE::Generated::CodecZenohDeclaration;
using UndeclQueryable = ::SCE::Generated::CodecZenohUndeclQueryable::CodecZenohUndeclQueryable;
using DeclFinal = ::SCE::Generated::CodecZenohDeclFinal::CodecZenohDeclFinal;

void a_flag_an_arm_says_is_derived_from_the_arm_the_value_holds() {
    // An undeclaration with no key expression says Z clear: the header the
    // value presents sets it, and the arm's word is what is written, beside
    // the arm's tag.
    const Declaration::CodecZenohDeclaration undeclaring{
        0x85, Declaration::CodecZenohDeclarationVariant{std::in_place_index<5>, UndeclQueryable{7, std::nullopt}}};
    CHECK(undeclaring.encode_to_vec() == (std::vector<std::uint8_t>{0x05, 0x07}),
          "Z is what the arm says, whatever the header presents");

    // An arm that says neither N nor Z leaves the bits the header presents.
    const Declaration::CodecZenohDeclaration final_arm{
        0xA0, Declaration::CodecZenohDeclarationVariant{std::in_place_index<8>, DeclFinal{}}};
    CHECK(final_arm.encode_to_vec() == std::vector<std::uint8_t>{0xBA},
          "a flag no arm the value holds says keeps the header's bit");
}

namespace InitSyn = ::SCE::Generated::CodecInitSynBody;

// A holder of the consumer's own whose group is a pair of its own, which
// presents the group's fields by name.
struct OwnSizes {
    std::uint8_t resolution;
    std::uint16_t batch;

    [[nodiscard]] std::uint8_t sn_res() const noexcept {
        return resolution;
    }

    [[nodiscard]] std::uint16_t batch_size() const noexcept {
        return batch;
    }
};

struct OwnInitSyn {
    std::optional<OwnSizes> sizes;

    [[nodiscard]] std::uint8_t version() const noexcept {
        return 0x09;
    }

    [[nodiscard]] std::optional<OwnSizes> s() const noexcept {
        return sizes;
    }
};

// The same holder presenting the group's fields one by one: no source of a
// codec whose fields one gate decides together.
struct OwnInitSynApart {
    [[nodiscard]] std::uint8_t version() const noexcept {
        return 0x09;
    }

    [[nodiscard]] std::optional<std::uint8_t> sn_res() const noexcept {
        return std::nullopt;
    }

    [[nodiscard]] std::optional<std::uint16_t> batch_size() const noexcept {
        return std::nullopt;
    }
};

static_assert(InitSyn::is_source<OwnInitSyn>::value,
              "a holder presenting the group as an optional of its own pair is a source");
static_assert(!InitSyn::is_source<OwnInitSynApart>::value, "a holder presenting the group's fields apart is no source");

void fields_one_gate_decides_are_held_and_written_together() {
    // The group is there: its fields follow the version, and the S the value
    // says is set; it is not: the version alone, and S clear.
    const std::vector<std::uint8_t> with{0x09, 0x02, 0x01, 0x00};
    const std::vector<std::uint8_t> without{0x09};
    const InitSyn::CodecInitSynBody present{0x09, InitSyn::CodecInitSynBodyS{0x02, 0x0100}};
    const InitSyn::CodecInitSynBody absent{0x09, std::nullopt};
    CHECK(present.encode_to_vec() == with, "a group that is there is written field by field");
    CHECK(absent.encode_to_vec() == without, "a group that is not there writes none of its fields");
    CHECK(InitSyn::input_s(present) == 1 && InitSyn::input_s(absent) == 0,
          "the value says S from whether it holds the group");

    ::SCE::Forge::SceCursor cursor(with.data(), with.size());
    const auto decoded = InitSyn::CodecInitSynBody::decode(cursor, 1);
    CHECK(decoded.has_value() && decoded->s().has_value() && decoded->s()->sn_res() == 0x02 &&
              decoded->s()->batch_size() == 0x0100,
          "a decoded value holds the group it read");
    ::SCE::Forge::SceCursor absent_cursor(without.data(), without.size());
    const auto decoded_absent = InitSyn::CodecInitSynBody::decode(absent_cursor, 0);
    CHECK(decoded_absent.has_value() && !decoded_absent->s().has_value(),
          "a decoded value holds no group it did not read");

    const OwnInitSyn own{OwnSizes{0x02, 0x0100}};
    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    CHECK(!InitSyn::Wire::encode_source(own, sink).has_value() && out == with,
          "a consumer's own pair encodes as the generated group does");
}

namespace InitSynEnvelope = ::SCE::Generated::CodecInitSynEnvelope;

void a_flag_every_arm_says_is_the_arms_word() {
    // Both arms hold an InitSyn body, which says S from its group: the
    // header's S is the arm's, set or clear, whatever the header presents.
    const InitSynEnvelope::CodecInitSynEnvelope sized{
        0x00, InitSynEnvelope::CodecInitSynEnvelopeVariant{
                  std::in_place_index<0>, InitSyn::CodecInitSynBody{0x02, InitSyn::CodecInitSynBodyS{0x55, 0xCAFE}}}};
    CHECK(sized.encode_to_vec() == (std::vector<std::uint8_t>{0x41, 0x02, 0x55, 0xCA, 0xFE}),
          "an arm that holds the group sets S beside the arm's tag");
    const InitSynEnvelope::CodecInitSynEnvelope bare{
        0x40, InitSynEnvelope::CodecInitSynEnvelopeVariant{std::in_place_index<0>,
                                                           InitSyn::CodecInitSynBody{0x02, std::nullopt}}};
    CHECK(bare.encode_to_vec() == (std::vector<std::uint8_t>{0x01, 0x02}),
          "an arm that holds no group clears the S the header presents");
}

namespace FlagEntry = ::SCE::Generated::CodecChainFlagEntry;
namespace FlagCarrier = ::SCE::Generated::CodecChainFlagCarrier;
namespace PrevEntry = ::SCE::Generated::CodecChainPrevEntry;
namespace PrevCarrier = ::SCE::Generated::CodecChainPrevCarrier;
using FlagEntryValue = FlagEntry::CodecChainFlagEntry;
using PrevEntryValue = PrevEntry::CodecChainPrevEntry;

static_assert(
    std::is_same<FlagEntry::CodecChainFlagEntryWide, std::variant<FlagEntry::CodecChainFlagEntryWideWide,
                                                                  FlagEntry::CodecChainFlagEntryWideNotWide>>::value,
    "fields on each side of one gate are a variant of an arm for each side");

void fields_on_each_side_of_a_gate_are_the_arm_the_value_holds() {
    // `wide` holds the long value, and does not hold the short one; the
    // encode derives it from the arm, so no caller hands it.
    const FlagEntryValue wide{0x03, FlagEntry::CodecChainFlagEntryWideWide{0x1234}};
    const FlagEntryValue narrow{0x03, FlagEntry::CodecChainFlagEntryWideNotWide{0x11}};
    CHECK(wide.encode_to_vec() == (std::vector<std::uint8_t>{0x03, 0x12, 0x34}), "the arm that holds writes its field");
    CHECK(narrow.encode_to_vec() == (std::vector<std::uint8_t>{0x03, 0x11}), "the other arm writes its own");
    CHECK(FlagEntry::input_wide(wide) == 1 && FlagEntry::input_wide(narrow) == 0,
          "the input is which arm the value holds");
    CHECK(std::holds_alternative<FlagEntry::CodecChainFlagEntryWideNotWide>(FlagEntryValue{}.wide()),
          "a value made by default holds the arm present while no gate holds");

    const std::vector<std::uint8_t> frame{0x03, 0x12, 0x34};
    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    const auto decoded = FlagEntryValue::decode(cursor, 1);
    const auto *arm = decoded ? std::get_if<FlagEntry::CodecChainFlagEntryWideWide>(&decoded->wide()) : nullptr;
    CHECK(arm != nullptr && arm->long_value() == 0x1234, "a decode handed the gate holds the arm it gives");
}

void a_list_is_held_to_the_law_its_entries_say_its_flag_by() {
    // Every entry says `wide`, which the carrier binds to `head.W`: the list's
    // start is the flag, and an entry that says otherwise has no frame.
    const std::vector<FlagEntryValue> wide_entries{
        FlagEntryValue{0x03, FlagEntry::CodecChainFlagEntryWideWide{0x1234}},
        FlagEntryValue{0x05, FlagEntry::CodecChainFlagEntryWideWide{0xABCD}}};
    using Entries = ::SCE::Forge::List<FlagEntryValue, 4, 1>;
    using LawfulEntries = ::SCE::Forge::Lawful<Entries, FlagCarrier::CodecChainFlagCarrierEntriesLaw>;
    const auto entries = Entries::make(wide_entries);
    CHECK(entries.has_value(), "two entries lie within 1 to 4");
    if (!entries) {
        return;
    }
    CHECK(!LawfulEntries::make(false, *entries).has_value(), "wide entries do not follow a clear flag");
    const auto lawful = LawfulEntries::make(true, *entries);
    CHECK(lawful.has_value(), "wide entries follow a set flag");
    if (!lawful) {
        return;
    }
    const std::vector<std::uint8_t> frame{0x01, 0x83, 0x12, 0x34, 0x05, 0xAB, 0xCD};
    const FlagCarrier::CodecChainFlagCarrier value{0x00, *lawful};
    CHECK(value.encode_to_vec() == frame, "the flag is the list's start, and Z is set on all but the last entry");

    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    const auto decoded = FlagCarrier::CodecChainFlagCarrier::decode(cursor);
    CHECK(decoded.has_value() && decoded->entries().size() == 2 && decoded->entries().start(),
          "a decoded list starts from the flag the frame gave");

    // Each entry says `after_marker` set exactly when the entry before is the
    // marker 0x4, and `tagged` as the carrier's `head.T`.
    const std::vector<PrevEntryValue> chain{
        PrevEntryValue{0x04, PrevEntry::CodecChainPrevEntryAfterMarkerNotAfterMarker{0x11}, std::nullopt},
        PrevEntryValue{0x03, PrevEntry::CodecChainPrevEntryAfterMarkerAfterMarker{0x1234}, std::nullopt}};
    const std::vector<PrevEntryValue> stays{
        PrevEntryValue{0x04, PrevEntry::CodecChainPrevEntryAfterMarkerNotAfterMarker{0x11}, std::nullopt},
        PrevEntryValue{0x03, PrevEntry::CodecChainPrevEntryAfterMarkerNotAfterMarker{0x22}, std::nullopt}};
    using PrevEntries = ::SCE::Forge::List<PrevEntryValue, 4, 1>;
    using LawfulPrev = ::SCE::Forge::Lawful<PrevEntries, PrevCarrier::CodecChainPrevCarrierEntriesLaw>;
    const auto prev = PrevEntries::make(chain);
    const auto wrong = PrevEntries::make(stays);
    CHECK(prev.has_value() && wrong.has_value(), "two entries lie within 1 to 4");
    if (!prev || !wrong) {
        return;
    }
    const LawfulPrev::start_type untagged{false, ::SCE::Forge::FixedStart{}};
    CHECK(!LawfulPrev::make(untagged, *wrong).has_value(), "the entry right after the marker says it set");
    const auto held = LawfulPrev::make(untagged, *prev);
    CHECK(held.has_value(), "the entry after the marker is wide");
    if (!held) {
        return;
    }
    const PrevCarrier::CodecChainPrevCarrier carrier{0x01, *held};
    CHECK(carrier.encode_to_vec() == (std::vector<std::uint8_t>{0x00, 0x84, 0x11, 0x03, 0x12, 0x34}),
          "the carrier's T is the list's start, which clears the bit it presents");
}

namespace LengthRef = ::SCE::Generated::CodecPresentIfLengthRef;
using LengthRefValue = LengthRef::CodecPresentIfLengthRef;
using LengthRefGroup = std::decay_t<decltype(std::declval<const LengthRefValue &>().flags_has_payload())>;
using WithPayload = std::variant_alternative_t<0, LengthRefGroup>;
using WithoutPayload = std::variant_alternative_t<1, LengthRefGroup>;
namespace EncodingInfo = ::SCE::Generated::CodecExtEncodingInfo;
using EncodingInfoValue = EncodingInfo::CodecExtEncodingInfo;
using EncodingInfoGroup = std::decay_t<decltype(std::declval<const EncodingInfoValue &>().combined_id_has_schema())>;
using WithSchema = std::variant_alternative_t<0, EncodingInfoGroup>;
using WithoutSchema = std::variant_alternative_t<1, EncodingInfoGroup>;
using OwnedBytes = ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 32>;

// The encode keeps the view of a byte string it reads off an arm past the
// read, so a read must present what lives on: a reference or a view, and not
// a list that owns its bytes returned by value, which would die under it.
static_assert(::SCE::Forge::Detail::Lasting<const OwnedBytes &>::value, "a reference lives on");
static_assert(::SCE::Forge::Detail::Lasting<::SCE::Forge::Bytes<32>>::value, "a view lives on");
static_assert(!::SCE::Forge::Detail::Lasting<OwnedBytes>::value, "an owned list by value does not");
static_assert(!::SCE::Forge::Detail::Lasting<std::vector<std::uint8_t>>::value, "a vector by value does not");

void a_byte_string_on_one_side_of_a_gate_is_the_arm_that_holds_it() {
    // The length is derived from the bytes on the side that holds them, and
    // is what the other side states where they are not.
    const std::vector<std::uint8_t> three{0xAA, 0xBB, 0xCC};
    const auto payload = ::SCE::Forge::Bytes<32>::make(three);
    CHECK(payload.has_value(), "three bytes lie within a bound of 32");
    if (!payload) {
        return;
    }
    const LengthRefValue with{0x00, WithPayload{*payload}};
    const LengthRefValue without{0x00, WithoutPayload{0x07}};
    const std::vector<std::uint8_t> with_frame{0x01, 0x03, 0xAA, 0xBB, 0xCC};
    const std::vector<std::uint8_t> without_frame{0x00, 0x07};
    const std::vector<std::uint8_t> default_frame{0x00, 0x00};
    CHECK(with.encode_to_vec() == with_frame, "the bytes set their flag and their length is derived");
    CHECK(without.encode_to_vec() == without_frame, "the side without bytes writes the length it states");
    CHECK(std::holds_alternative<WithoutPayload>(LengthRefValue{}.flags_has_payload()),
          "a value made by default holds the side present while no gate holds");
    CHECK(LengthRefValue{}.encode_to_vec() == default_frame, "and states a length of zero");

    ::SCE::Forge::SceCursor cursor(with_frame.data(), with_frame.size());
    const auto decoded = LengthRefValue::decode(cursor);
    const auto *held = decoded ? std::get_if<WithPayload>(&decoded->flags_has_payload()) : nullptr;
    CHECK(held != nullptr && held->payload() == three, "a frame whose flag is set holds the bytes it read");
    CHECK(decoded.has_value() && decoded->encode_to_vec() == with_frame, "and encodes back to it");

    ::SCE::Forge::SceCursor cursor_without(without_frame.data(), without_frame.size());
    const auto decoded_without = LengthRefValue::decode(cursor_without);
    const auto *stated = decoded_without ? std::get_if<WithoutPayload>(&decoded_without->flags_has_payload()) : nullptr;
    CHECK(stated != nullptr && stated->payload_size() == 0x07,
          "a frame whose flag is clear holds the length it states");

    // A VLE carrier: bit 0 of its value says whether the schema is there.
    const std::vector<std::uint8_t> schema_bytes{0x10, 0x20};
    const auto schema = ::SCE::Forge::Bytes<64>::make(schema_bytes);
    CHECK(schema.has_value(), "two bytes lie within a bound of 64");
    if (!schema) {
        return;
    }
    const EncodingInfoValue info{0x04, WithSchema{*schema}};
    const EncodingInfoValue bare{0x04, WithoutSchema{0x00}};
    const std::vector<std::uint8_t> info_frame{0x05, 0x02, 0x10, 0x20};
    const std::vector<std::uint8_t> bare_frame{0x04, 0x00};
    CHECK(info.encode_to_vec() == info_frame, "the schema sets bit 0 of the carrier and is counted");
    CHECK(bare.encode_to_vec() == bare_frame, "no schema clears it");
    ::SCE::Forge::SceCursor info_cursor(info_frame.data(), info_frame.size());
    const auto decoded_info = EncodingInfoValue::decode(info_cursor);
    const auto *with_schema = decoded_info ? std::get_if<WithSchema>(&decoded_info->combined_id_has_schema()) : nullptr;
    CHECK(with_schema != nullptr && with_schema->schema() == schema_bytes, "a decoded carrier holds the schema");
}

namespace Zbuf = ::SCE::Generated::CodecZenohValueZbuf;
using ZbufValue = Zbuf::CodecZenohValueZbuf;
using ZbufGroup = std::decay_t<decltype(std::declval<const ZbufValue &>().after_shm())>;
using ZbufSliced = std::variant_alternative_t<0, ZbufGroup>;
using ZbufFlat = std::variant_alternative_t<1, ZbufGroup>;
using Slice = ::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice;
using SliceList = ::SCE::Forge::Bounded<std::vector<Slice>, ::SCE::Forge::kUnbounded>;
using SliceView = ::SCE::Forge::List<Slice, ::SCE::Forge::kUnbounded>;
using ZbufBytes = ::SCE::Forge::Bytes<4294967295>;

namespace zbuf_consumer {

/// The arm that holds the slices, which it returns by reference.
struct Sliced {
    std::uint64_t logical;
    std::uint64_t enc;
    SliceList list;

    [[nodiscard]] std::uint64_t value_len() const noexcept {
        return logical;
    }

    [[nodiscard]] std::uint64_t encoding() const noexcept {
        return enc;
    }

    [[nodiscard]] const SliceList &slices() const noexcept {
        return list;
    }
};

/// The arm that holds the bytes whole.
struct Flat {
    ZbufBytes bytes;

    [[nodiscard]] ZbufBytes value() const noexcept {
        return bytes;
    }
};

/// A buffer the consumer holds as either arm.
struct Buffer {
    std::variant<Sliced, Flat> arm;

    [[nodiscard]] const std::variant<Sliced, Flat> &after_shm() const noexcept {
        return arm;
    }
};

}  // namespace zbuf_consumer

static_assert(Zbuf::is_source<ZbufValue>::value, "the generated value is a source of its own codec");
static_assert(Zbuf::is_source<zbuf_consumer::Buffer>::value,
              "a list one arm holds, of the consumer's own type within the bound, is a source");
static_assert(Zbuf::is_nothrow_source<zbuf_consumer::Buffer>::value, "keeping a list by reference throws nothing");
static_assert(std::is_same<decltype(std::declval<const ZbufSliced &>().slices()), SliceView>::value,
              "an arm presents its list as a view of its entries held to its bound");

void a_list_on_one_side_of_a_gate_is_kept_as_the_arm_presents_it() {
    // The encode keeps a read it makes on one arm's path only: what a
    // reference names, by its address; what a read by value returns, whole.
    // A slice's bytes are a view, so what it views is named first.
    const std::vector<std::uint8_t> first_bytes{0xAA, 0xBB};
    const std::vector<std::uint8_t> second_bytes{0xCC};
    const auto first = ZbufBytes::make(first_bytes);
    const auto second = ZbufBytes::make(second_bytes);
    CHECK(first.has_value() && second.has_value(), "a slice's bytes lie within their bound");
    if (!first || !second) {
        return;
    }
    const std::vector<Slice> two{Slice{0x01, *first}, Slice{0x02, *second}};
    const auto held = SliceList::make(two);
    CHECK(held.has_value(), "two entries lie within no bound");
    if (!held) {
        return;
    }
    ::SCE::Forge::Detail::Kept<const SliceList &> by_address;
    CHECK(!by_address.has_value(), "a read not yet kept is not there");
    by_address.keep(*held);
    CHECK(by_address.has_value() && &*by_address == &*held, "a reference is kept as the object it names");
    ::SCE::Forge::Detail::Kept<SliceView> by_value;
    by_value.keep(SliceView(*held));
    CHECK(by_value && by_value->size() == 2, "a view returned by value is kept whole");

    // Sliced: the logical length the arm states, the encoding, the count
    // derived from the slices, then each slice.
    const std::vector<std::uint8_t> sliced_frame{0x05, 0x03, 0x02, 0x01, 0x02, 0xAA, 0xBB, 0x02, 0x01, 0xCC};
    const zbuf_consumer::Buffer sliced{zbuf_consumer::Sliced{0x05, 0x03, *held}};
    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    CHECK(!Zbuf::Wire::encode_source(sliced, sink).has_value() && out == sliced_frame,
          "the arm that holds the slices writes them, their count derived");
    CHECK(Zbuf::input_after_shm(sliced) == 1, "the arm holding the slices says after_shm");

    // Flat: the length derived from the bytes, then the bytes.
    const std::vector<std::uint8_t> three{0x10, 0x20, 0x30};
    const std::vector<std::uint8_t> flat_frame{0x03, 0x10, 0x20, 0x30};
    const auto whole_bytes = ZbufBytes::make(three);
    CHECK(whole_bytes.has_value(), "three bytes lie within their bound");
    if (!whole_bytes) {
        return;
    }
    const zbuf_consumer::Buffer flat{zbuf_consumer::Flat{*whole_bytes}};
    std::vector<std::uint8_t> flat_out;
    ::SCE::Forge::VectorSink flat_sink(flat_out);
    CHECK(!Zbuf::Wire::encode_source(flat, flat_sink).has_value() && flat_out == flat_frame,
          "the arm that holds the bytes writes them, their length derived");
    CHECK(Zbuf::input_after_shm(flat) == 0, "the arm holding the bytes does not say after_shm");

    CHECK(std::holds_alternative<ZbufFlat>(ZbufValue{}.after_shm()),
          "a value made by default holds the side present while no gate holds");
    CHECK(ZbufValue{}.encode_to_vec() == (std::vector<std::uint8_t>{0x00}), "and states a length of zero");

    ::SCE::Forge::SceCursor cursor(sliced_frame.data(), sliced_frame.size());
    const auto decoded = ZbufValue::decode(cursor, 1);
    const auto *slices = decoded ? std::get_if<ZbufSliced>(&decoded->after_shm()) : nullptr;
    CHECK(slices != nullptr && slices->slices().size() == 2 && slices->slices()[1].kind() == 0x02 &&
              slices->value_len() == 0x05,
          "a frame after the marker holds the slices it read and the length it states");
    CHECK(decoded.has_value() && decoded->encode_to_vec() == sliced_frame, "and encodes back to it");

    ::SCE::Forge::SceCursor flat_cursor(flat_frame.data(), flat_frame.size());
    const auto decoded_flat = ZbufValue::decode(flat_cursor, 0);
    const auto *whole = decoded_flat ? std::get_if<ZbufFlat>(&decoded_flat->after_shm()) : nullptr;
    CHECK(whole != nullptr && whole->value() == three, "a frame with no marker holds the bytes it read");
    CHECK(decoded_flat.has_value() && decoded_flat->encode_to_vec() == flat_frame, "and encodes back to it");
}

namespace ValueEntry = ::SCE::Generated::CodecZenohValueEntry;
using ValueEntryValue = ValueEntry::CodecZenohValueEntry;
using ValueEntryBody = std::variant<ExtUnit, ExtZint, zbuf_consumer::Buffer, ValueEntry::CodecZenohValueEntryDefault>;

namespace value_entry_consumer {

/// An extension whose body is the consumer's own variant, the ZBuf arm a
/// buffer of its own.
struct Extension {
    std::uint8_t head;
    ValueEntryBody arm;

    [[nodiscard]] std::uint8_t header() const noexcept {
        return head;
    }

    [[nodiscard]] const ValueEntryBody &body() const noexcept {
        return arm;
    }
};

}  // namespace value_entry_consumer

static_assert(ValueEntry::is_source<value_entry_consumer::Extension>::value,
              "a variant whose ZBuf arm is the consumer's own buffer is a body a source may present");
static_assert(std::is_same<decltype(ValueEntry::input_after_shm(std::declval<const ValueEntryValue &>())),
                           std::optional<std::uint8_t>>::value,
              "an input one arm says is answered optionally");

void an_input_an_arm_says_is_read_off_the_arm_the_value_holds() {
    const std::vector<std::uint8_t> first_bytes{0xAA, 0xBB};
    const std::vector<std::uint8_t> second_bytes{0xCC};
    const auto first = ZbufBytes::make(first_bytes);
    const auto second = ZbufBytes::make(second_bytes);
    const std::vector<std::uint8_t> three{0x10, 0x20, 0x30};
    const auto whole_bytes = ZbufBytes::make(three);
    CHECK(first.has_value() && second.has_value() && whole_bytes.has_value(), "the bytes lie within their bound");
    if (!first || !second || !whole_bytes) {
        return;
    }
    const std::vector<Slice> two{Slice{0x01, *first}, Slice{0x02, *second}};
    const auto held = SliceList::make(two);
    CHECK(held.has_value(), "two entries lie within no bound");
    if (!held) {
        return;
    }

    // The ZBuf arm says after_shm as its own codec does; the header's enc
    // bits are its tag, beside the header's other bits.
    const value_entry_consumer::Extension sliced{
        0x01, ValueEntryBody{std::in_place_index<2>, zbuf_consumer::Buffer{zbuf_consumer::Sliced{0x05, 0x03, *held}}}};
    const value_entry_consumer::Extension flat{
        0x01, ValueEntryBody{std::in_place_index<2>, zbuf_consumer::Buffer{zbuf_consumer::Flat{*whole_bytes}}}};
    const value_entry_consumer::Extension unit{0x01, ValueEntryBody{std::in_place_index<0>, ExtUnit{}}};
    CHECK(ValueEntry::input_after_shm(sliced) == std::optional<std::uint8_t>(1),
          "an arm holding the slices says after_shm");
    CHECK(ValueEntry::input_after_shm(flat) == std::optional<std::uint8_t>(0),
          "an arm holding the bytes says it clear");
    CHECK(!ValueEntry::input_after_shm(unit).has_value(), "an arm that does not read it says nothing");

    const std::vector<std::uint8_t> sliced_frame{0x41, 0x05, 0x03, 0x02, 0x01, 0x02, 0xAA, 0xBB, 0x02, 0x01, 0xCC};
    std::vector<std::uint8_t> out;
    ::SCE::Forge::VectorSink sink(out);
    CHECK(!ValueEntry::Wire::encode_source(sliced, sink).has_value() && out == sliced_frame,
          "the ZBuf arm is written by its codec, the header's enc bits its tag");

    ::SCE::Forge::SceCursor cursor(sliced_frame.data(), sliced_frame.size());
    const auto decoded = ValueEntryValue::decode(cursor, 1);
    CHECK(decoded.has_value() && decoded->body().index() == 2 &&
              ValueEntry::input_after_shm(*decoded) == std::optional<std::uint8_t>(1),
          "a frame after the marker decodes to the ZBuf arm, which says after_shm");
    CHECK(decoded.has_value() && decoded->encode_to_vec() == sliced_frame, "and encodes back to it");

    const std::vector<std::uint8_t> unit_frame{0x01};
    ::SCE::Forge::SceCursor unit_cursor(unit_frame.data(), unit_frame.size());
    const auto decoded_unit = ValueEntryValue::decode(unit_cursor, 0);
    CHECK(decoded_unit.has_value() && !ValueEntry::input_after_shm(*decoded_unit).has_value(),
          "a decoded unit arm says nothing of after_shm");
}

namespace QueryValue = ::SCE::Generated::CodecZenohQueryValue;
using QueryValueValue = QueryValue::CodecZenohQueryValue;
using QueryExts = std::decay_t<decltype(std::declval<const QueryValueValue &>().exts())>;

void a_chain_holds_entries_that_say_an_input_in_some_arms_to_its_law() {
    // after_shm is set exactly when the entry before is the marker 0x4; an
    // entry whose arm does not read it says nothing and follows any state.
    const std::vector<std::uint8_t> sliced_frame{0x41, 0x05, 0x03, 0x02, 0x01, 0x02, 0xAA, 0xBB, 0x02, 0x01, 0xCC};
    ::SCE::Forge::SceCursor sliced_cursor(sliced_frame.data(), sliced_frame.size());
    const auto sliced = ValueEntryValue::decode(sliced_cursor, 1);
    CHECK(sliced.has_value(), "an entry holding the slices decodes after the marker");
    if (!sliced) {
        return;
    }
    const ValueEntryValue marker{0x04, ValueEntry::CodecZenohValueEntryVariant{std::in_place_index<0>, ExtUnit{}}};
    const ValueEntryValue unit{0x01, ValueEntry::CodecZenohValueEntryVariant{std::in_place_index<0>, ExtUnit{}}};

    const std::vector<ValueEntryValue> after_marker{marker, *sliced};
    const std::vector<ValueEntryValue> unmarked{unit, *sliced};
    const std::vector<ValueEntryValue> silent{marker, unit};
    const auto after_marker_list = QueryExts::list_type::make(after_marker);
    const auto unmarked_list = QueryExts::list_type::make(unmarked);
    const auto silent_list = QueryExts::list_type::make(silent);
    CHECK(after_marker_list.has_value() && unmarked_list.has_value() && silent_list.has_value(),
          "two entries lie within the chain's bound");
    if (!after_marker_list || !unmarked_list || !silent_list) {
        return;
    }
    CHECK(!QueryExts::make(QueryExts::start_type{}, *unmarked_list).has_value(),
          "the slices say after_shm where no marker came before");
    CHECK(QueryExts::make(QueryExts::start_type{}, *silent_list).has_value(),
          "an entry whose arm does not read after_shm follows the marker");
    const auto lawful = QueryExts::make(QueryExts::start_type{}, *after_marker_list);
    CHECK(lawful.has_value(), "the slices after the marker follow the law");
    if (!lawful) {
        return;
    }

    const std::vector<std::uint8_t> frame{0x84, 0x41, 0x05, 0x03, 0x02, 0x01, 0x02, 0xAA, 0xBB, 0x02, 0x01, 0xCC};
    const QueryValueValue value{*lawful};
    CHECK(value.encode_to_vec() == frame, "each entry is written by its codec, Z set on all but the last");

    ::SCE::Forge::SceCursor cursor(frame.data(), frame.size());
    const auto decoded = QueryValueValue::decode(cursor);
    CHECK(decoded.has_value() && decoded->exts().size() == 2 && decoded->exts()[1].body().index() == 2,
          "a decoded chain hands the entry after the marker after_shm and holds its slices");
    CHECK(decoded.has_value() && decoded->encode_to_vec() == frame, "and encodes back to it");
}

namespace PeekBasic = ::SCE::Generated::CodecVariantPeekBasic;
using PeekBasicValue = PeekBasic::CodecVariantPeekBasic;
using PeekArmA = ::SCE::Generated::CodecPeekArmA::CodecPeekArmA;
using PeekArmB = ::SCE::Generated::CodecPeekArmB::CodecPeekArmB;

void a_tag_the_arms_own_header_holds_is_written_by_the_arm() {
    // The arm's own header holds the tag, so the arm the value holds writes it
    // with the rest of itself and the value writes nothing of its own; an arm
    // made by default holds the kind its codec declares.
    CHECK(PeekArmA{}.encode_to_vec() == (std::vector<std::uint8_t>{0x00, 0x00}) &&
              PeekArmB{}.encode_to_vec() == (std::vector<std::uint8_t>{0x01, 0x00, 0x00}),
          "an arm made by default holds the kind that names it");
    const PeekBasicValue a{PeekBasic::CodecVariantPeekBasicVariant{std::in_place_index<0>, PeekArmA{0x00, 0x55}}};
    const PeekBasicValue b{PeekBasic::CodecVariantPeekBasicVariant{std::in_place_index<1>, PeekArmB{0x01, 0x1234}}};
    const std::vector<std::uint8_t> a_frame{0x00, 0x55};
    const std::vector<std::uint8_t> b_frame{0x01, 0x12, 0x34};
    CHECK(a.encode_to_vec() == a_frame, "the first arm writes its header and its payload");
    CHECK(b.encode_to_vec() == b_frame, "the second arm writes its own");
    CHECK(PeekBasicValue{}.encode_to_vec() == (std::vector<std::uint8_t>{0x00, 0x00}),
          "a value made by default holds the arm marked default");

    ::SCE::Forge::SceCursor cursor(b_frame.data(), b_frame.size());
    const auto decoded = PeekBasicValue::decode(cursor);
    const auto *held = decoded ? std::get_if<1>(&decoded->body()) : nullptr;
    CHECK(held != nullptr && held->payload() == 0x1234, "the peeked kind picks the arm, which reads it as its header");
    CHECK(decoded.has_value() && decoded->encode_to_vec() == b_frame, "and encodes back to it");
}

namespace Disjunction = ::SCE::Generated::CodecPresentIfDisjunction;
using DisjunctionValue = Disjunction::CodecPresentIfDisjunction;
using DisjunctionGroup =
    typename std::decay_t<decltype(std::declval<const DisjunctionValue &>().flags_wants_a_or_wants_b())>::value_type;
using WantsA = std::variant_alternative_t<0, DisjunctionGroup>;
using WantsB = std::variant_alternative_t<1, DisjunctionGroup>;
using WantsBoth = std::variant_alternative_t<2, DisjunctionGroup>;

void a_flag_of_a_disjunction_is_derived_from_the_arm_the_value_holds() {
    // Each arm sets the flags it holds and clears the others; bits of the
    // carrier no gate names are the value's own.
    const DisjunctionValue a{0xF0, DisjunctionGroup{WantsA{0x1234}}};
    const DisjunctionValue b{0x00, DisjunctionGroup{WantsB{0x1234}}};
    const DisjunctionValue both{0x00, DisjunctionGroup{WantsBoth{0x1234}}};
    const DisjunctionValue none{0x03, std::nullopt};
    const std::vector<std::uint8_t> a_frame{0xF1, 0x12, 0x34};
    const std::vector<std::uint8_t> b_frame{0x02, 0x12, 0x34};
    const std::vector<std::uint8_t> both_frame{0x03, 0x12, 0x34};
    const std::vector<std::uint8_t> none_frame{0x00};
    CHECK(a.encode_to_vec() == a_frame, "the first arm sets wants_a alone and keeps the carrier's other bits");
    CHECK(b.encode_to_vec() == b_frame, "the second arm sets wants_b alone");
    CHECK(both.encode_to_vec() == both_frame, "the third arm sets both");
    CHECK(none.encode_to_vec() == none_frame, "no arm clears both, whatever bits the value presents");

    ::SCE::Forge::SceCursor cursor(b_frame.data(), b_frame.size());
    const auto decoded = DisjunctionValue::decode(cursor);
    const auto *held = decoded && decoded->flags_wants_a_or_wants_b()
                           ? std::get_if<WantsB>(&*decoded->flags_wants_a_or_wants_b())
                           : nullptr;
    CHECK(held != nullptr && held->seq() == 0x1234, "a frame with wants_b alone holds the second arm");
    CHECK(decoded.has_value() && decoded->encode_to_vec() == b_frame, "and encodes back to it");
}

namespace OneCount = ::SCE::Generated::CodecRepeatUnconditionalCount;
using OneCountValue = OneCount::CodecRepeatUnconditionalCount;
using OneCountH = OneCount::CodecRepeatUnconditionalCountOptionsHH;
using OneCountNotH = OneCount::CodecRepeatUnconditionalCountOptionsHNotH;
using OneCountElem = ::SCE::Generated::CodecRepeatElem::CodecRepeatElem;
using OneCountList = ::SCE::Forge::List<OneCountElem, 64>;
using OneCountPair = ::SCE::Forge::Aligned<OneCountList, OneCountList>;

void two_lists_of_one_count_are_held_as_one_and_counted_once() {
    const std::vector<OneCountElem> links{OneCountElem{0x0001}, OneCountElem{0x0002}};
    const std::vector<OneCountElem> weights{OneCountElem{0x0003}, OneCountElem{0x0004}};
    const std::vector<OneCountElem> one{OneCountElem{0x0005}};
    const auto links_list = OneCountList::make(links);
    const auto weights_list = OneCountList::make(weights);
    const auto one_list = OneCountList::make(one);
    CHECK(links_list && weights_list && one_list, "each list lies within its bound");
    if (!links_list || !weights_list || !one_list) {
        return;
    }
    // The two are made only of two lists of one count, so the one count the
    // encode writes is the count of both.
    CHECK(!OneCountPair::make(*links_list, *one_list).has_value(), "two lists of different counts are no pair");
    const auto pair = OneCountPair::make(*links_list, *weights_list);
    CHECK(pair.has_value(), "two lists of one count are");
    if (!pair) {
        return;
    }

    // With the second list: the flag set, the one count, then each list.
    const OneCountValue both{0xF0, OneCountH{*pair}};
    const std::vector<std::uint8_t> both_frame{0xF1, 0x02, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04};
    CHECK(both.encode_to_vec() == both_frame, "the arm of both sets H and writes the count once, then both lists");

    // Without it: the flag clear, the count of the first, then the first.
    const OneCountValue first{0xF1, OneCountNotH{*links_list}};
    const std::vector<std::uint8_t> first_frame{0xF0, 0x02, 0x00, 0x01, 0x00, 0x02};
    CHECK(first.encode_to_vec() == first_frame, "the arm of the first alone clears H and counts it");

    CHECK(std::holds_alternative<OneCountNotH>(OneCountValue{}.options_h()),
          "a value made by default holds the arm present while no gate holds");
    CHECK(OneCountValue{}.encode_to_vec() == (std::vector<std::uint8_t>{0x00, 0x00}), "and counts no entry");

    ::SCE::Forge::SceCursor cursor(both_frame.data(), both_frame.size());
    const auto decoded = OneCountValue::decode(cursor);
    const auto *held = decoded ? std::get_if<OneCountH>(&decoded->options_h()) : nullptr;
    CHECK(held != nullptr && held->links_weights().size() == 2 && held->links_weights().second()[1].seq() == 0x0004,
          "a frame with H holds both lists read by the one count");
    CHECK(decoded.has_value() && decoded->encode_to_vec() == both_frame, "and encodes back to it");
}

void encode_source_reports_the_sinks_failure() {
    std::uint8_t buffer[3] = {};
    ::SCE::Forge::SpanSink sink(buffer, sizeof buffer);
    const auto error = Frame::Wire::encode_source(consumer::PlainFrame{1, 2, 0x0304}, sink);
    CHECK(error.has_value() && *error == ::SCE::Forge::CodecError::BufferOverflow,
          "a four-byte frame into a three-byte sink is the sink's overflow");
}

}  // namespace

int main() {
    the_generated_value_is_made_and_read_by_its_constructor_and_accessors();
    a_consumer_type_encodes_as_the_generated_value_does();
    a_length_is_derived_from_the_payload_the_source_presents();
    a_payload_past_its_bound_cannot_be_made();
    a_presence_flag_is_derived_from_the_element_it_gates();
    a_length_held_in_a_flag_is_derived_from_the_bytes();
    a_text_is_made_of_utf8_within_its_bound_and_its_length_is_derived();
    a_body_is_encoded_over_the_source_its_holder_presents();
    an_input_the_value_says_is_read_off_the_value_and_handed_on();
    a_list_is_written_entry_by_entry_and_its_count_derived();
    a_count_a_gate_decides_is_derived_where_the_list_is_there();
    a_variant_derives_its_tag_from_the_arm_it_holds();
    a_chain_sets_the_flag_its_entries_end_on();
    a_body_a_gate_decides_is_written_where_it_is_present();
    a_list_a_gate_decides_is_walked_where_it_is_present();
    a_list_and_bytes_of_no_stated_bound_are_held_to_none();
    a_flag_an_arm_says_is_derived_from_the_arm_the_value_holds();
    fields_one_gate_decides_are_held_and_written_together();
    a_flag_every_arm_says_is_the_arms_word();
    fields_on_each_side_of_a_gate_are_the_arm_the_value_holds();
    a_list_is_held_to_the_law_its_entries_say_its_flag_by();
    a_byte_string_on_one_side_of_a_gate_is_the_arm_that_holds_it();
    a_list_on_one_side_of_a_gate_is_kept_as_the_arm_presents_it();
    an_input_an_arm_says_is_read_off_the_arm_the_value_holds();
    a_chain_holds_entries_that_say_an_input_in_some_arms_to_its_law();
    a_flag_of_a_disjunction_is_derived_from_the_arm_the_value_holds();
    a_tag_the_arms_own_header_holds_is_written_by_the_arm();
    two_lists_of_one_count_are_held_as_one_and_counted_once();
    encode_source_reports_the_sinks_failure();
    if (failures != 0) {
        std::fprintf(stderr, "codec_source_generated_test: %d failure(s)\n", failures);
        return 1;
    }
    std::printf("codec_source_generated_test: ok\n");
    return 0;
}
