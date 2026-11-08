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
// once. A value a host injects through the seam is held to every bound before anything is
// sent (section 5.4): within them it is read as any delivery, and one past a bound at any
// depth is answered as the refusal naming its path, and nothing is raised. A list taken
// whole into a variable is the machine's own copy.

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

/// An item a host builds, its `tags` holding `tags` texts.
Item hostItem(std::uint32_t id, int tags) {
    std::vector<std::string> words;
    for (int n = 0; n < tags; ++n) {
        words.push_back("t" + std::to_string(n));
    }
    return Item{id, 2, "abc", Level::Low, std::nullopt, std::vector<std::uint8_t>{'w', 'x'}, words};
}

TEST(AComposedPayloadIsReadAtAnyDepthTest, AFullDeliveryIsReadAtEveryDepth) {
    Machine machine;
    machine.initialize();
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
    EXPECT_EQ(machine.cell(), 3u);
    // Depth 3: the length of the first item's `tags`.
    EXPECT_EQ(machine.first_tags(), 4u);
    // An element that may be absent, present.
    EXPECT_EQ(machine.first_maybe(), std::optional<std::uint16_t>(7));
    // A list taken whole into a list of a larger bound.
    EXPECT_EQ(machine.late(), (std::vector<std::uint8_t>{5, 6}));
}

TEST(AComposedPayloadIsReadAtAnyDepthTest, AListPastItsBoundAtDepthTwoIsRefusedWhole) {
    Machine machine;
    machine.initialize();
    machine.raiseExternal("batch.sent", batch(5));
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 1u) << "error.execution reached the document once";
    EXPECT_EQ(machine.item_count(), 0u) << "none of the content ran";
    EXPECT_TRUE(machine.late().empty()) << "nothing of the delivery was kept";
}

TEST(AComposedPayloadIsReadAtAnyDepthTest, AValueAHostInjectsWithinItsBoundsIsReadAsAnyDelivery) {
    Machine machine;
    machine.initialize();
    std::vector<std::uint8_t> late{5, 6};
    const std::string refused =
        machine.raiseBatchSent(std::vector<Item>{hostItem(4, 2)}, std::vector<std::vector<std::uint8_t>>{{1, 2}},
                               std::vector<Level>{Level::Low},
                               std::vector<std::optional<std::uint16_t>>{std::optional<std::uint16_t>(9), std::nullopt},
                               std::nullopt, "ab", late);
    EXPECT_EQ(refused, "") << "a value within every bound is raised";
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 0u);
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

TEST(AComposedPayloadIsReadAtAnyDepthTest, AListAHostInjectsOnePastItsBoundIsRefusedBeforeAnythingIsSent) {
    // `late` is a list<uint8>[2]: three elements are no value of its type, and the seam
    // answers the refusal before anything is sent.
    Machine machine;
    machine.initialize();
    const std::string refused = machine.raiseBatchSent(std::vector<Item>{hostItem(4, 2)}, {}, {}, {}, std::nullopt,
                                                       "ab", std::vector<std::uint8_t>{5, 6, 7});
    EXPECT_EQ(refused, "'late' holds 3 elements, past the 2 its type declares");
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 0u) << "nothing was sent, so nothing was refused";
    EXPECT_EQ(machine.item_count(), 0u);
    EXPECT_TRUE(machine.late().empty());
}

TEST(AComposedPayloadIsReadAtAnyDepthTest, ARecordAHostBuildsOnePastAnInnerBoundIsRefusedByItsPath) {
    // `tags` is a list<string[8]>[4] inside each item: an item of five is refused at the
    // path of the list inside the element, and a tag of nine bytes at the path of the tag.
    Machine machine;
    machine.initialize();
    EXPECT_EQ(machine.raiseBatchSent(std::vector<Item>{hostItem(1, 5)}, {}, {}, {}, std::nullopt, "ab", {}),
              "'items/0/tags' holds 5 elements, past the 4 its type declares");
    Item nine = hostItem(1, 0);
    nine.tags.push_back("ninebytes");
    EXPECT_EQ(machine.raiseBatchSent(std::vector<Item>{nine}, {}, {}, {}, std::nullopt, "ab", {}),
              "'items/0/tags/0' is 9 bytes, past the 8 its type declares");
    // A top-level text past its bound is refused at its own name.
    EXPECT_EQ(machine.raiseBatchSent({}, {}, {}, {}, std::nullopt, "abcde", {}),
              "'note' is 5 bytes, past the 4 its type declares");
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 0u) << "nothing was sent, so nothing was refused";
    EXPECT_EQ(machine.item_count(), 0u);
}

}  // namespace
}  // namespace SCE::Tests
