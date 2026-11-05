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

#include "codec_length_ref.h"
#include "codec_length_ref_dotted_basic.h"
#include "codec_little_endian.h"
#include "codec_present_if_basic.h"
#include "codec_simple_frame.h"

#include <cstdint>
#include <cstdio>
#include <optional>
#include <type_traits>
#include <utility>
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
    encode_source_reports_the_sinks_failure();
    if (failures != 0) {
        std::fprintf(stderr, "codec_source_generated_test: %d failure(s)\n", failures);
        return 1;
    }
    std::printf("codec_source_generated_test: ok\n");
    return 0;
}
