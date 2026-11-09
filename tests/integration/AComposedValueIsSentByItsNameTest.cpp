// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C++ machine sends a composed value by its name (docs/adr/0086, section 5.2), run with
// no script engine, as the Rust, Kotlin, Go and Python machines do
// (backends/rust/tests/tests/a_composed_value_is_sent_by_its_name.rs,
// AComposedValueIsSentByItsNameTest.kt, a_composed_value_is_sent_by_its_name_test.go,
// test_a_composed_value_is_sent_by_its_name.py). The machine is generated from
// sce-build/tests/fixtures/static_datamodel/static_composed_send.scxml where the test
// target is built (tests/CMakeLists.txt).
//
// The first delivery of `batch.sent` is forwarded to the machine itself: four lists and
// an optional list named as the parts of the payload they are, and a list variable the
// machine took the payload's `late` into. The delivery that comes back is lifted as any
// delivery is (section 4), so it is refused whole if anything written is outside its
// type; it is not, and every value read from it at depths 1 to 3 is what the first
// delivery held.

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include <gtest/gtest.h>

#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "static_composed_send_sm.h"

namespace SCE::Tests {
namespace {

using Machine = ::SCE::Generated::static_composed_send::static_composed_send;

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

TEST(AComposedValueIsSentByItsNameTest, ADeliveryForwardedByItsNamesComesBackWhole) {
    Machine machine;
    EXPECT_EQ(machine.initialize(), std::nullopt);
    // Every list full, an element of `maybe` absent and `spare` present.
    machine.raiseExternal("batch.sent", "{\"items\":[" + item(1, 4) + "," + item(2, 3) + "," + item(3, 4) +
                                            "],\"grid\":[[1,2],[3,4]],\"levels\":[\"low\",\"critical\"],"
                                            "\"maybe\":[7,null],\"spare\":[1,2],\"note\":\"abcd\",\"late\":[5,6]}");
    StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 1u) << "the delivery the machine sent itself fits its schema, and the one refusal "
                                         "is the list sent short of its field's lower count";
    // A list variable sent to a field that states a lower count (docs/adr/0086, section 15.6): sent
    // empty, refused whole and never delivered; sent with its one element, delivered, and
    // forwarded once more with its scalar `hops`, a scalar field of a payload that holds a list.
    EXPECT_EQ(machine.counted_deliveries(), 2u);
    EXPECT_EQ(machine.counted_sum(), 7u);
    EXPECT_EQ(machine.counted_hops(), 3u);
    EXPECT_EQ(machine.deliveries(), 2u) << "the forwarded delivery came back on the internal queue";
    // Depth 1: the lengths of a list of records and a list of enum values.
    EXPECT_EQ(machine.item_count(), 3u);
    EXPECT_EQ(machine.level_count(), 2u);
    // Depth 2: each item's list, and a field of an element read by its index.
    EXPECT_EQ(machine.tag_count(), 11u);
    EXPECT_EQ(machine.id_sum(), 6u);
    // A list of lists, walked at both depths.
    EXPECT_EQ(machine.cell_sum(), 10u);
    // An element that may be absent, present.
    EXPECT_EQ(machine.first_maybe(), std::optional<std::uint16_t>(7));
    // The list variable, sent by its name.
    EXPECT_EQ(machine.late_sum(), 11u);
    EXPECT_EQ(machine.late(), (std::vector<std::uint8_t>{5, 6}));
}

}  // namespace
}  // namespace SCE::Tests
