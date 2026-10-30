// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// An arm compiled under a build feature (docs/adr/0072), compiled and run in
// both builds.
//
// This file is built twice (CMakeLists.txt): as it is, where the generated
// headers define SCE_FEATURE_GATED_ARM to 1, and with the macro defined to 0,
// where the gated arms' alternatives are not in their variants. The headers
// are generated at build time from the documents the Rust arm runs
// (backends/rust/forge-conformance/fixtures/codec_feature_gated_arm*.scxml),
// and this file holds the C++ codecs to what
// backends/rust/forge-conformance/tests/forge_feature_gated_arm.rs holds the
// Rust ones to.
//
// codec_feature_gated_arm dispatches on the low two bits of the next byte:
// 0x01 to arm A (the default arm), 0x02 to arm B, which names the feature
// `gated-arm`, and anything else to the catch-all, whose body is arm A's
// codec. codec_feature_gated_arm_no_catch_all dispatches on one bit: 0x00 to
// its arm A, 0x01 to its arm B under the same feature, and has no catch-all.
// Where the macro is 0, a frame of arm B is refused: decode returns
// std::nullopt and the cursor records CodecError::VariantArmNotBuilt, since
// the tag is one the document names and the catch-all must not read it.

#include "codec_feature_gated_arm.h"
#include "codec_feature_gated_arm_no_catch_all.h"

#include <cstdint>
#include <cstdio>
#include <optional>
#include <variant>
#include <vector>

#ifndef SCE_FEATURE_GATED_ARM
#error "the generated headers define SCE_FEATURE_GATED_ARM unless the build defined it"
#endif

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                         \
        }                                                                                                              \
    } while (0)

using ::SCE::Forge::CodecError;
using ::SCE::Forge::SceCursor;
using Gated = ::SCE::Generated::CodecFeatureGatedArm::CodecFeatureGatedArm;
using GatedVariant = ::SCE::Generated::CodecFeatureGatedArm::CodecFeatureGatedArmVariant;
using GatedDefault = ::SCE::Generated::CodecFeatureGatedArm::CodecFeatureGatedArmDefault;
using NoCatchAll = ::SCE::Generated::CodecFeatureGatedArmNoCatchAll::CodecFeatureGatedArmNoCatchAll;
using NoCatchAllVariant = ::SCE::Generated::CodecFeatureGatedArmNoCatchAll::CodecFeatureGatedArmNoCatchAllVariant;
using ArmA = ::SCE::Generated::CodecDefaultMarkerArmA::CodecDefaultMarkerArmA;
using PeekArmA = ::SCE::Generated::CodecPeekArmA::CodecPeekArmA;
#if SCE_FEATURE_GATED_ARM
using ArmB = ::SCE::Generated::CodecDefaultMarkerArmB::CodecDefaultMarkerArmB;
using PeekArmB = ::SCE::Generated::CodecPeekArmB::CodecPeekArmB;
#endif

// The gated arm's alternative is in the variant only where its macro is 1: arm
// A, arm B and the catch-all's wrapper, or arm A and the wrapper; arm A and arm
// B, or arm A alone.
static_assert(std::variant_size_v<GatedVariant> == (SCE_FEATURE_GATED_ARM ? 3 : 2),
              "arm B is an alternative exactly where its feature is on");
static_assert(std::variant_size_v<NoCatchAllVariant> == (SCE_FEATURE_GATED_ARM ? 2 : 1),
              "arm B is an alternative exactly where its feature is on");

// A frame arm B's codec writes: the header byte with tag 0x02, then a two-byte
// payload.
const std::vector<std::uint8_t> kArmBFrame{0x02, 0x12, 0x34};

// A frame the peek-byte arm B writes: the header byte with tag 0x01, then a
// two-byte payload.
const std::vector<std::uint8_t> kPeekArmBFrame{0x01, 0x12, 0x34};

// Arm A is the default arm in every build: the default value is arm A's, chosen
// by its type, and it decodes back into arm A.
void the_default_arm_is_held_by_every_build() {
    const Gated original{};
    CHECK(std::holds_alternative<ArmA>(original.body), "a default value holds arm A");
    const std::vector<std::uint8_t> bytes = original.encode_to_vec();
    CHECK((bytes == std::vector<std::uint8_t>{0x01, 0x00}),
          "the default value is arm A's (header 0x01, one payload byte)");
    SceCursor cursor(bytes.data(), bytes.size());
    const std::optional<Gated> decoded = Gated::decode(cursor);
    CHECK(decoded.has_value(), "arm A's frame decodes");
    if (!decoded.has_value()) {
        return;
    }
    CHECK(cursor.remaining() == 0, "arm A reads the whole frame");
    const ArmA *a = std::get_if<ArmA>(&decoded->body);
    CHECK(a != nullptr && a->header == 0x01 && a->payload == 0x00, "tag 0x01 decodes into arm A");

    const NoCatchAll peek_original{};
    CHECK(std::holds_alternative<PeekArmA>(peek_original.body), "a default value holds the peek-byte arm A");
}

// A tag no arm names reaches the catch-all in every build.
void a_tag_the_document_does_not_name_reaches_the_catch_all() {
    const std::vector<std::uint8_t> frame{0x03, 0x07};
    SceCursor cursor(frame.data(), frame.size());
    const std::optional<Gated> decoded = Gated::decode(cursor);
    CHECK(decoded.has_value(), "the catch-all decodes");
    if (!decoded.has_value()) {
        return;
    }
    const GatedDefault *d = std::get_if<GatedDefault>(&decoded->body);
    CHECK(d != nullptr && d->tag == 0x03 && d->body.header == 0x03 && d->body.payload == 0x07,
          "tag 0x03 reaches the catch-all, which keeps the tag");
    CHECK(!cursor.last_refusal().has_value(), "a frame the catch-all reads is not refused");
}

// Without a catch-all, the arm every build holds decodes in every build.
void without_a_catch_all_the_ungated_arm_decodes_in_every_build() {
    const std::vector<std::uint8_t> frame{0x00, 0x07};
    SceCursor cursor(frame.data(), frame.size());
    const std::optional<NoCatchAll> decoded = NoCatchAll::decode(cursor);
    CHECK(decoded.has_value(), "arm A decodes");
    if (!decoded.has_value()) {
        return;
    }
    const PeekArmA *a = std::get_if<PeekArmA>(&decoded->body);
    CHECK(a != nullptr && a->header == 0x00 && a->payload == 0x07, "tag 0x00 decodes into arm A");
}

#if SCE_FEATURE_GATED_ARM

// With the feature on, tag 0x02 decodes into arm B, which reads its two-byte
// payload, and the frame re-encodes byte for byte.
void where_the_feature_is_on_its_tag_decodes_into_its_arm() {
    SceCursor cursor(kArmBFrame.data(), kArmBFrame.size());
    const std::optional<Gated> decoded = Gated::decode(cursor);
    CHECK(decoded.has_value(), "arm B's frame decodes");
    if (!decoded.has_value()) {
        return;
    }
    CHECK(cursor.remaining() == 0, "arm B reads the whole frame");
    const ArmB *b = std::get_if<ArmB>(&decoded->body);
    CHECK(b != nullptr && b->header == 0x02 && b->payload == 0x1234, "tag 0x02 decodes into arm B");
    CHECK(decoded->encode_to_vec() == kArmBFrame, "arm B's frame re-encodes byte for byte");
}

// Without a catch-all, the feature on: tag 0x01 decodes into arm B.
void without_a_catch_all_the_feature_on_decodes_the_gated_arm() {
    SceCursor cursor(kPeekArmBFrame.data(), kPeekArmBFrame.size());
    const std::optional<NoCatchAll> decoded = NoCatchAll::decode(cursor);
    CHECK(decoded.has_value(), "the peek-byte arm B's frame decodes");
    if (!decoded.has_value()) {
        return;
    }
    CHECK(cursor.remaining() == 0, "arm B reads the whole frame");
    const PeekArmB *b = std::get_if<PeekArmB>(&decoded->body);
    CHECK(b != nullptr && b->header == 0x01 && b->payload == 0x1234, "tag 0x01 decodes into arm B");
}

#else

// With the feature off, arm B does not exist, and tag 0x02 is refused: it is a
// tag the document names, and reading the frame with the catch-all's body (arm
// A's codec) would take a frame of arm B for one of arm A.
void where_the_feature_is_off_its_tag_is_refused() {
    SceCursor cursor(kArmBFrame.data(), kArmBFrame.size());
    CHECK(!Gated::decode(cursor).has_value(), "tag 0x02 is refused where gated-arm is off");
    CHECK(cursor.last_refusal() == std::optional<CodecError>(CodecError::VariantArmNotBuilt),
          "the refusal is VariantArmNotBuilt, not a frame short of bytes");
}

// Without a catch-all, the feature off: tag 0x01 is refused, and the decode
// reaches no branch the build-time proof of the arms called unreachable.
void without_a_catch_all_the_feature_off_refuses_the_gated_arms_tag() {
    SceCursor cursor(kPeekArmBFrame.data(), kPeekArmBFrame.size());
    CHECK(!NoCatchAll::decode(cursor).has_value(), "tag 0x01 is refused where gated-arm is off");
    CHECK(cursor.last_refusal() == std::optional<CodecError>(CodecError::VariantArmNotBuilt),
          "the refusal is VariantArmNotBuilt, not a frame short of bytes");
}

#endif

}  // namespace

int main() {
    the_default_arm_is_held_by_every_build();
    a_tag_the_document_does_not_name_reaches_the_catch_all();
    without_a_catch_all_the_ungated_arm_decodes_in_every_build();
#if SCE_FEATURE_GATED_ARM
    where_the_feature_is_on_its_tag_decodes_into_its_arm();
    without_a_catch_all_the_feature_on_decodes_the_gated_arm();
#else
    where_the_feature_is_off_its_tag_is_refused();
    without_a_catch_all_the_feature_off_refuses_the_gated_arms_tag();
#endif

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("feature_gated_arm_generated_test (SCE_FEATURE_GATED_ARM=%d): all checks passed\n",
                SCE_FEATURE_GATED_ARM);
    return 0;
}
