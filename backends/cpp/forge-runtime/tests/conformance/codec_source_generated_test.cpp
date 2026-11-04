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
// msgId u8, length u8, payload u16) and codec_little_endian.scxml (little
// endian: sensorId u8, value u16, status u8), whose headers the harness's
// generation writes.

#include "codec_little_endian.h"
#include "codec_simple_frame.h"

#include <cstdint>
#include <cstdio>
#include <optional>
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

}  // namespace consumer

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
    encode_source_reports_the_sinks_failure();
    if (failures != 0) {
        std::fprintf(stderr, "codec_source_generated_test: %d failure(s)\n", failures);
        return 1;
    }
    std::printf("codec_source_generated_test: ok\n");
    return 0;
}
