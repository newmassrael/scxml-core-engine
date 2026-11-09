// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C++ machine reads a composed payload at any depth (docs/adr/0086, section 6), run
// with no script engine, as the Rust, Kotlin, Go and Python machines do
// (backends/rust/tests/tests/a_composed_payload_is_read_at_any_depth.rs,
// AComposedPayloadIsReadAtAnyDepthTest.kt, a_composed_payload_is_read_at_any_depth_test.go,
// test_a_composed_payload_is_read_at_any_depth.py). The machine is generated from
// sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml where the test
// target is built (tests/CMakeLists.txt).
//
// A delivery whose every list is full is lifted into the machine's own types, and each
// read at depths 1 to 3 is what a host reads back. A delivery one past a bound at depth 2
// is refused whole (section 4.2): none of the content runs, and error.execution is raised
// once. A value a host injects through the seam is built in the types that hold every
// bound (section 7.4): within them it is read as any delivery, and one past a bound at any
// depth is no value of its type, so the host cannot build it and nothing is raised. A list
// taken whole into a variable is the machine's own copy.

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include <gtest/gtest.h>

#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "static_composed_payload_sm.h"

namespace SCE::Tests {
namespace {

namespace P = ::SCE::Generated::static_composed_payload;
using Machine = P::static_composed_payload;
using Item = P::StaticComposedPayloadItemRecord;
using Level = P::StaticComposedPayloadSignalLevelEnum;

/// One item of the batch as JSON: `tags` holds that many texts.
std::string item(int id, int tags) {
    std::string listed;
    for (int n = 0; n < tags; ++n) {
        listed += (n > 0 ? "," : "") + std::string("\"tag") + std::to_string(n) + "\"";
    }
    return "{\"id\":" + std::to_string(id) +
           ",\"amount\":\"9007199254740993\",\"label\":\"abcdefgh\",\"level\":\"high\",\"due\":null,"
           "\"blob\":\"wxyz\",\"tags\":[" +
           listed + "]}";
}

/// A delivery of `batch.sent` whose every list is full, the second item's `tags` holding
/// `secondTags` texts.
std::string batch(int secondTags) {
    return "{\"items\":[" + item(1, 4) + "," + item(2, secondTags) + "," + item(3, 4) +
           "],\"grid\":[[1,2],[3,4]],\"levels\":[\"low\",\"critical\"],\"maybe\":[7,null],\"spare\":[1,2],"
           "\"note\":\"abcd\",\"late\":[5,6]}";
}

/// The types the machine holds the batch's fields in (docs/adr/0086, section 7.4).
using Tag = decltype(Item::tags)::value_type;
using Tags = decltype(Item::tags);
using Payload = P::StaticComposedPayloadBatchSentPayload;
using Items = decltype(Payload::items);
using Late = decltype(Payload::late);

/// An item a host builds, its `tags` holding `tags` texts, at most the four its type holds.
Item hostItem(std::uint32_t id, int tags) {
    Tags words;
    for (int n = 0; n < tags; ++n) {
        const auto tag = Tag::make("t" + std::to_string(n));
        EXPECT_TRUE(tag.has_value() && words.push(*tag)) << "tag " << n << " lies within its bounds";
    }
    return Item{id,
                2,
                decltype(Item::label)::literal("abc"),
                Level::Low,
                std::nullopt,
                decltype(Item::blob)::literal({'w', 'x'}),
                words};
}

TEST(AComposedPayloadIsReadAtAnyDepthTest, AFullDeliveryIsReadAtEveryDepth) {
    Machine machine;
    EXPECT_EQ(machine.initialize(), std::nullopt);
    machine.raiseExternal("batch.sent", batch(4));
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 0u) << "a delivery of its type is no error";
    // Depth 1: the length of a list of the payload.
    EXPECT_EQ(machine.item_count(), 3u);
    // Depth 2: a loop over each item's list, and a field of an element read through its index.
    EXPECT_EQ(machine.tag_count(), 12u);
    EXPECT_EQ(machine.word_count(), 12u);
    EXPECT_EQ(machine.id_sum(), 6u);
    // A list of lists, walked at both depths and read by two indices.
    EXPECT_EQ(machine.cell_sum(), 10u);
    EXPECT_EQ(machine.cell(), std::uint8_t{3});
    // Depth 3: the length of the first item's `tags`.
    EXPECT_EQ(machine.first_tags(), 4u);
    // An element that may be absent, present.
    EXPECT_EQ(machine.first_maybe(), std::optional<std::uint16_t>(7));
    // A list taken whole into a list of a larger bound.
    EXPECT_EQ(machine.late(), (std::vector<std::uint8_t>{5, 6}));
}

TEST(AComposedPayloadIsReadAtAnyDepthTest, AListPastItsBoundAtDepthTwoIsRefusedWhole) {
    Machine machine;
    EXPECT_EQ(machine.initialize(), std::nullopt);
    machine.raiseExternal("batch.sent", batch(5));
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 1u) << "error.execution reached the document once";
    EXPECT_EQ(machine.item_count(), 0u) << "none of the content ran";
    EXPECT_TRUE(machine.late().empty()) << "nothing of the delivery was kept";
}

/// `full` with the first `from` written `to`.
std::string replacedOnce(std::string full, const std::string &from, const std::string &to) {
    const std::size_t at = full.find(from);
    if (at != std::string::npos) {
        full.replace(at, from.size(), to);
    }
    return full;
}

// Every bound of the schema is held where the delivery is read (docs/adr/0086, section
// 4.4): a text and a byte string inside an element, the top-level text, and the count of
// the top-level list, each one past its bound in a delivery otherwise full, are refused
// whole.
TEST(AComposedPayloadIsReadAtAnyDepthTest, EveryBoundOfTheSchemaIsHeldWhereTheDeliveryIsRead) {
    const std::string full = batch(4);
    const std::vector<std::string> cases = {
        replacedOnce(full, "\"label\":\"abcdefgh\"", "\"label\":\"abcdefghi\""),
        replacedOnce(full, "\"blob\":\"wxyz\"", "\"blob\":\"vwxyz\""),
        replacedOnce(full, "\"note\":\"abcd\"", "\"note\":\"abcde\""),
        replacedOnce(full, "\"items\":[", "\"items\":[" + item(0, 1) + ","),
    };
    for (const std::string &delivery : cases) {
        ASSERT_NE(delivery, full) << "the case changes the delivery";
        Machine machine;
        EXPECT_EQ(machine.initialize(), std::nullopt);
        machine.raiseExternal("batch.sent", delivery);
        StaticScenario::settleRounds(machine);
        EXPECT_EQ(machine.refusals(), 1u) << "error.execution reached the document once: " << delivery;
        EXPECT_EQ(machine.item_count(), 0u) << "none of the content ran: " << delivery;
        EXPECT_TRUE(machine.late().empty()) << "nothing of the delivery was kept: " << delivery;
    }
}

TEST(AComposedPayloadIsReadAtAnyDepthTest, AValueAHostInjectsWithinItsBoundsIsReadAsAnyDelivery) {
    Machine machine;
    EXPECT_EQ(machine.initialize(), std::nullopt);
    std::vector<std::uint8_t> late{5, 6};
    const auto items = Items::make(std::vector<Item>{hostItem(4, 2)});
    const auto heldLate = Late::make(late);
    ASSERT_TRUE(items.has_value() && heldLate.has_value()) << "each list lies within its bound";
    using Grid = decltype(Payload::grid);
    using Levels = decltype(Payload::levels);
    using Maybe = decltype(Payload::maybe);
    machine.raiseBatchSent(*items, Grid::literal({Grid::value_type::literal({1, 2})}), Levels::literal({Level::Low}),
                           Maybe::literal({std::optional<std::uint16_t>(9), std::nullopt}), std::nullopt,
                           decltype(Payload::note)::literal("ab"), *heldLate);
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 0u) << "a value within every bound is raised and read";
    EXPECT_EQ(machine.item_count(), 1u);
    EXPECT_EQ(machine.tag_count(), 2u);
    EXPECT_EQ(machine.id_sum(), 4u);
    EXPECT_EQ(machine.cell_sum(), 3u);
    EXPECT_EQ(machine.first_maybe(), std::optional<std::uint16_t>(9));
    EXPECT_EQ(machine.late(), (std::vector<std::uint8_t>{5, 6}));
    // The variable holds a copy of its own: what the host does to its list afterwards does
    // not reach the one the machine keeps.
    late.push_back(7);
    EXPECT_EQ(machine.late(), (std::vector<std::uint8_t>{5, 6}));
}

TEST(AComposedPayloadIsReadAtAnyDepthTest, AValuePastABoundAtAnyDepthIsNoValueAHostCanBuild) {
    // Each bound of the batch is its type's (docs/adr/0086, section 7.4), so a value past
    // one is refused where the host would make it, before there is anything to send:
    // `late` is a list<uint8>[2], `tags` a list<string[8]>[4] inside each item, `note` a
    // string[4].
    EXPECT_FALSE(Late::make(std::vector<std::uint8_t>{5, 6, 7}).has_value()) << "three elements past two";
    Tags four;
    for (int n = 0; n < 4; ++n) {
        EXPECT_TRUE(four.push(Tag::literal("t"))) << "tag " << n << " lies within four";
    }
    EXPECT_FALSE(four.push(Tag::literal("t"))) << "a fifth tag is refused, the list left as it was";
    EXPECT_EQ(four.size(), 4u);
    EXPECT_FALSE(Tag::make(std::string("ninebytes")).has_value()) << "nine bytes past eight";
    EXPECT_FALSE(decltype(Payload::note)::make(std::string("abcde")).has_value()) << "five bytes past four";
    // Nothing was built past a bound, so nothing reached the machine.
    Machine machine;
    EXPECT_EQ(machine.initialize(), std::nullopt);
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 0u);
    EXPECT_EQ(machine.item_count(), 0u);
    EXPECT_TRUE(machine.late().empty());
}

}  // namespace
}  // namespace SCE::Tests
