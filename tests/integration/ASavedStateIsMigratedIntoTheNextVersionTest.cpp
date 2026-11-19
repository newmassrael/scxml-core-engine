// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0109, D11 (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Saving and restoring"): a state
// the first version of a document saved is restored into the next version through the
// migration the next version declares. `static_migration_1` stands in `waiting` with `count`
// and `total` moved; `static_migration_2` renamed the state `pending`, widened `total` from
// int32 to int64 and added `ceiling`, and declares the step from the first version's
// committed shape to its own. The restore carries the state by that step, and a state of a
// shape no step starts from is still refused as one of another document.
//
// The Rust half is `backends/rust/tests/tests/a_saved_state_is_migrated_into_the_next_version.rs`.

#include "common/SceClock.h"
#include "saved/SavedState.h"
#include "static_migration_1_sm.h"
#include "static_migration_2_sm.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace SCE {
namespace Tests {

namespace {

using Version1 = ::SCE::Generated::static_migration_1::static_migration_1;
using Version2 = ::SCE::Generated::static_migration_2::static_migration_2;

/// The wall-clock moment both machines are saved and restored at.
constexpr std::int64_t kSavedAtMs = 1'700'000'000'000;

template <typename Machine> void send(Machine &machine, const std::string &name) {
    machine.raiseExternal(name);
    machine.step();
}

/// A state of the first version, standing in `waiting` after three ticks, as a process
/// boundary carries it: written to JSON and read back.
::SCE::Saved::SavedState savedByVersion1() {
    Version1 machine;
    machine.setClock(std::make_shared<::SCE::ManualClock>(0));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    send(machine, "go");
    for (int tick = 0; tick < 3; ++tick) {
        send(machine, "tick");
    }
    return ::SCE::Saved::SavedState::fromJson(machine.saveAt(kSavedAtMs).toJson());
}

std::unique_ptr<Version2> restoredIntoVersion2(const ::SCE::Saved::SavedState &saved) {
    auto machine = std::make_unique<Version2>();
    machine->setClock(std::make_shared<::SCE::ManualClock>(0));
    machine->restoreAt(saved, kSavedAtMs);
    return machine;
}

std::string stateOf(const Version2 &machine) {
    return Version2::PolicyType::getStateName(machine.getCurrentState());
}

}  // namespace

TEST(ASavedStateIsMigratedIntoTheNextVersion, AStateOfTheFirstVersionRestoresIntoTheNextThroughItsStep) {
    const auto saved = savedByVersion1();
    ASSERT_EQ(saved.configuration, std::vector<std::string>{"waiting"}) << saved.toJson();

    const auto restored = restoredIntoVersion2(saved);
    EXPECT_EQ(stateOf(*restored), "pending") << "`waiting` is renamed `pending`";
    EXPECT_EQ(restored->count(), 3u) << "a variable kept as it was";
    EXPECT_EQ(restored->total(), -6) << "an int32 widened to an int64 holds the value it held";
    EXPECT_EQ(restored->ceiling(), 16u) << "a variable the step adds holds the constant the step gives it";
}

TEST(ASavedStateIsMigratedIntoTheNextVersion, AMigratedMachineRunsOnAsTheNextVersionAndSavesUnderItsShape) {
    auto restored = restoredIntoVersion2(savedByVersion1());
    send(*restored, "tick");
    EXPECT_EQ(restored->count(), 4u) << "`pending` takes the tick `waiting` took";
    EXPECT_EQ(restored->total(), -8);

    Version2 fresh;
    fresh.setClock(std::make_shared<::SCE::ManualClock>(0));
    EXPECT_EQ(fresh.initialize(), std::nullopt);
    const auto again = restored->saveAt(kSavedAtMs);
    EXPECT_EQ(again.shape, fresh.saveAt(kSavedAtMs).shape)
        << "a restored machine saves under the shape of the version it runs";
    EXPECT_NO_THROW(restoredIntoVersion2(::SCE::Saved::SavedState::fromJson(again.toJson())))
        << "a state the next version saved restores with no step applied";
}

TEST(ASavedStateIsMigratedIntoTheNextVersion, AStateOfAShapeNoStepStartsFromIsRefusedByItsShape) {
    auto saved = savedByVersion1();
    saved.shape = std::string(64, '0');
    try {
        restoredIntoVersion2(saved);
        FAIL() << "a state of a shape no step starts from was restored";
    } catch (const ::SCE::Saved::StateRefusal &refusal) {
        EXPECT_NE(std::string(refusal.what()).find("of a document of shape"), std::string::npos) << refusal.what();
    }
}

}  // namespace Tests
}  // namespace SCE
