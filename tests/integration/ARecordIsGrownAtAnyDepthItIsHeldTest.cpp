// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0112, D5, and docs/adr/0044: a record a saved state holds is read at the defaults
// of the fields a later revision of its schema added, at every depth it is held at.
// `static_grow_1` holds records of the first revision of their schemas in a record variable, an
// optional record and a record inside it, a list a record holds, a list variable, an optional variable and the case of
// a union; `static_grow_2` holds them at their second revision, which added a number with a
// default, a list, an optional record and an optional text. Its shape is the first version's,
// so it restores a state the first version saved with no migration.
//
// The Rust half is `backends/rust/tests/tests/a_record_is_grown_at_any_depth_it_is_held.rs`.

#include "common/SceClock.h"
#include "saved/SavedState.h"
#include "static_grow_1_sm.h"
#include "static_grow_2_sm.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

namespace SCE {
namespace Tests {

namespace {

using Version1 = ::SCE::Generated::static_grow_1::static_grow_1;
using Version2 = ::SCE::Generated::static_grow_2::static_grow_2;

/// The wall-clock moment both machines are saved and restored at.
constexpr std::int64_t kSavedAtMs = 1'700'000'000'000;

/// A state of the first version holding a record at every depth, as a process boundary carries
/// it: written to JSON and read back.
::SCE::Saved::SavedState savedByTheFirstVersion() {
    Version1 machine;
    machine.setClock(std::make_shared<::SCE::ManualClock>(0));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    const std::vector<std::pair<std::string, std::string>> events = {
        {"grow.holding", R"({"outer":{"id":7,"inner":{"n":1},"inners":[{"n":2},{"n":3}]}})"},
        {"grow.inner", R"({"n":4})"},
        {"grow.item", R"({"n":8})"},
        {"grow.maybe", R"({"inner":{"n":5}})"},
        {"grow.shaped", R"({"shape":{"kind":"single","n":6}})"},
    };
    for (const auto &[name, data] : events) {
        machine.raiseExternal(name, data);
        machine.step();
    }
    return ::SCE::Saved::SavedState::fromJson(machine.saveAt(kSavedAtMs).toJson());
}

/// The value `path` names in `state`: a variable, then a member or an element at each step.
const ::SCE::Saved::JsonValue &held(const ::SCE::Saved::SavedState &state, const std::string &variable,
                                    const std::vector<std::string> &path) {
    const ::SCE::Saved::JsonValue *value = state.variables.get(variable);
    EXPECT_NE(value, nullptr) << variable;
    for (const std::string &step : path) {
        if (!step.empty() && step[0] >= '0' && step[0] <= '9') {
            value = &value->asArray()[static_cast<std::size_t>(std::stoul(step))];
        } else {
            value = value->asObject().get(step);
            EXPECT_NE(value, nullptr) << variable << " has no " << step;
        }
    }
    return *value;
}

}  // namespace

TEST(ARecordIsGrownAtAnyDepthItIsHeld, EveryRecordTheFirstVersionSavedIsReadAtTheDefaultsItsSecondRevisionAdded) {
    const auto saved = savedByTheFirstVersion();
    Version2 restored;
    restored.setClock(std::make_shared<::SCE::ManualClock>(0));
    restored.restoreAt(saved, kSavedAtMs);
    const auto again = ::SCE::Saved::SavedState::fromJson(restored.saveAt(kSavedAtMs).toJson());
    // Each place, and whether the second revision of its record added a list and a record (every
    // one but the list variable's, whose element holds neither).
    const std::vector<std::tuple<std::string, std::vector<std::string>, bool>> places = {
        {"kept", {}, true},     {"outer", {}, true}, {"outer", {"inner"}, true}, {"outer", {"inners", "1"}, true},
        {"many", {"0"}, false}, {"maybe", {}, true}, {"shape", {}, true},
    };
    for (const auto &[variable, path, composed] : places) {
        const ::SCE::Saved::JsonValue &record = held(again, variable, path);
        ASSERT_TRUE(record.isObject()) << variable;
        const auto *rank = record.asObject().get("rank");
        ASSERT_NE(rank, nullptr) << variable << ": a number takes its default";
        EXPECT_EQ(rank->asNumber().text, "-2") << variable;
        if (composed) {
            const auto *tags = record.asObject().get("tags");
            ASSERT_NE(tags, nullptr) << variable << ": a list reads empty";
            EXPECT_TRUE(tags->isArray() && tags->asArray().empty()) << variable;
            const auto *leaf = record.asObject().get("leaf");
            ASSERT_NE(leaf, nullptr) << variable << ": a record a revision added reads absent";
            EXPECT_TRUE(leaf->isNull()) << variable;
        }
        const auto *memo = record.asObject().get("memo");
        ASSERT_NE(memo, nullptr) << variable << ": an optional text reads absent";
        EXPECT_TRUE(memo->isNull()) << variable;
    }
    // What the state now holds is the next version's: restored again, nothing grows twice.
    Version2 twice;
    twice.setClock(std::make_shared<::SCE::ManualClock>(0));
    EXPECT_NO_THROW(twice.restoreAt(again, kSavedAtMs));
}

}  // namespace Tests
}  // namespace SCE
