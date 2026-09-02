// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-3.11 — what `StaticExecutionEngine::enterAt` accepts, and what it
// refuses.
//
// The door exists so a host can bring a machine back where it was, in a new
// process, without replaying the entry actions the earlier run already ran.
// Its positive half is asserted where a real document uses it
// (`AiLoopAotTest.ARunJournalledAsNamesResumesWhereItStopped`, which also
// measures that no act was performed). This file is the other half: the
// configurations that are NOT configurations of the document.
//
// Refusals are the part that has to be enumerated rather than sampled.
// Entering "near" the requested configuration is the one outcome this door
// must never produce, because nothing afterwards can detect it — the machine
// reports itself running, `getCurrentState()` answers, and the set behind
// those answers is one the document never describes. A gate holding only the
// accepting case would pass on an engine that accepted everything.
//
// The C++ sibling of the Rust runtime's `configuration_entry.rs`, asking the
// same questions of the same rules, so a set one engine accepts is one the
// other accepts.

#include "ai_loop_sm.h"
#include "statechart_native_action_sm.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <vector>

#include "core/ConfigurationHelper.h"
#include "scripting/JSEngine.h"
#include "scripting/ScriptEngineProvider.h"

namespace SCE::Tests {

namespace {

using Loop = SCE::Generated::ai_loop::ai_loop;
using LoopState = SCE::Generated::ai_loop::State;
using Linear = SCE::Generated::statechart_native_action::statechart_native_action;
using LinearState = SCE::Generated::statechart_native_action::State;
using LinearActions = SCE::Generated::statechart_native_action::StatechartNativeActionActions;
using SCE::Core::ConfigurationRejection;

/// A mid-run configuration of `ai_loop`: the turn cycle at work, the liveness
/// watch alive, the budget within. Written out rather than taken from a live
/// run because every case below is a MUTATION of it — one change each, so a
/// refusal names one rule.
std::vector<LoopState> atWork() {
    return {LoopState::Run,   LoopState::Drive, LoopState::Running, LoopState::Working,
            LoopState::Watch, LoopState::Alive, LoopState::Budget,  LoopState::Within};
}

/// A host for the linear machine. Its every effect is a `<sce:action>`, so it
/// cannot be constructed without one — which is the point of that seam and
/// merely plumbing here.
class SilentActions : public LinearActions {
public:
    void appendFragmentPayload(const std::vector<std::uint8_t> &, std::uint32_t) override {}

    void resetSlot() override {}

    void onIdleEntry() override {}

    void onAssemblingExit() override {}
};

class ConfigurationEntryAotTest : public ::testing::Test {
protected:
    void SetUp() override {
        SCE::JSEngine::instance().initialize();
    }

    void TearDown() override {
        SCE::JSEngine::instance().shutdown();
    }

    /// §scxml-5.3: an ACCEPTED entry declares the datamodel, so a document
    /// whose `<data>` carries initialisers needs its script engine in place
    /// first — the same requirement `initialize()` has, for the same reason.
    ///
    /// The refusals below deliberately do NOT call this: a refused entry
    /// returns before the declaration, so needing no engine is itself part of
    /// "validation runs before any mutation".
    static void giveItAnEngine(Loop &sm) {
        sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                               [](SCE::IScriptEngine *) {}));
    }
};

// The set written above is a configuration of the document, so it is accepted
// and the machine comes back holding exactly it. This is the baseline every
// refusal below is one mutation away from — without it, a validator that
// refused everything would pass every other case in this file.
TEST_F(ConfigurationEntryAotTest, AParallelConfigurationIsAccepted) {
    Loop sm;
    giveItAnEngine(sm);
    const auto configuration = atWork();

    EXPECT_EQ(sm.enterAt(configuration, LoopState::Working), ConfigurationRejection::None);
    EXPECT_EQ(sm.getPolicy().getActiveStates(), configuration);
    EXPECT_EQ(sm.getCurrentState(), LoopState::Working);
    EXPECT_TRUE(sm.isRunning());
}

// A machine with no `<parallel>` keeps no active set of its own — the engine
// rebuilds the hierarchy from `currentState_`. So the round trip has to work
// without `setActiveStates` existing at all, which is a different code path
// and the one most documents take.
TEST_F(ConfigurationEntryAotTest, ALinearConfigurationRoundTripsWithoutAPolicyActiveSet) {
    SilentActions host;
    Linear sm(host);

    EXPECT_EQ(sm.enterAt({LinearState::Assembling}, LinearState::Assembling), ConfigurationRejection::None);
    EXPECT_EQ(sm.getCurrentState(), LinearState::Assembling);
    EXPECT_EQ(sm.getActiveStates(), std::vector<LinearState>{LinearState::Assembling});
    EXPECT_TRUE(sm.isRunning());
}

TEST_F(ConfigurationEntryAotTest, AnEmptyConfigurationIsRefused) {
    Loop sm;
    EXPECT_EQ(sm.enterAt({}, LoopState::Working), ConfigurationRejection::Empty) << "a machine is never in nothing";
}

// §scxml-3.11: a compound state holds exactly one active child. `priming` and
// `working` are both children of `running`, and a run stands in one of them.
TEST_F(ConfigurationEntryAotTest, TwoSiblingsOfOneRegionAreRefused) {
    Loop sm;
    auto configuration = atWork();
    configuration.push_back(LoopState::Priming);

    EXPECT_EQ(sm.enterAt(configuration, LoopState::Working), ConfigurationRejection::CompoundChildCount)
        << "`running` was given two active children, which is a configuration the document has no "
           "reading for";
}

// §scxml-3.11: a `<parallel>` holds EVERY region. Dropping one is the shape a
// host produces when it journals only the region it cares about.
TEST_F(ConfigurationEntryAotTest, AParallelWithARegionMissingIsRefused) {
    Loop sm;
    const std::vector<LoopState> configuration{LoopState::Run,     LoopState::Drive, LoopState::Running,
                                               LoopState::Working, LoopState::Watch, LoopState::Alive};

    EXPECT_EQ(sm.enterAt(configuration, LoopState::Working), ConfigurationRejection::ParallelRegionMissing)
        << "`budget` is a region of `run` and a run is always in all three at once";
}

// The set has to be ancestor-closed: a state is active only if its parent is.
TEST_F(ConfigurationEntryAotTest, AConfigurationThatSkipsAnAncestorIsRefused) {
    Loop sm;
    const std::vector<LoopState> configuration{LoopState::Run,   LoopState::Drive, LoopState::Working,
                                               LoopState::Watch, LoopState::Alive, LoopState::Budget,
                                               LoopState::Within};

    EXPECT_EQ(sm.enterAt(configuration, LoopState::Working), ConfigurationRejection::AncestorMissing)
        << "`working` is a child of `running`, which the set does not hold";
}

// Checked before the arity counts, because a duplicate would otherwise read as
// a second child and the refusal would name the wrong rule.
TEST_F(ConfigurationEntryAotTest, ARepeatedStateIsRefused) {
    Loop sm;
    auto configuration = atWork();
    configuration.push_back(LoopState::Working);

    EXPECT_EQ(sm.enterAt(configuration, LoopState::Working), ConfigurationRejection::Duplicate);
}

// §scxml-3.11: a configuration closes on exactly one root. `converged` is a
// top-level `<final>`, so a set holding both it and `run` describes two
// machines.
TEST_F(ConfigurationEntryAotTest, TwoRootsAreRefused) {
    Loop sm;
    auto configuration = atWork();
    configuration.push_back(LoopState::Converged);

    EXPECT_EQ(sm.enterAt(configuration, LoopState::Working), ConfigurationRejection::RootCount);
}

TEST_F(ConfigurationEntryAotTest, ACurrentStateOutsideTheConfigurationIsRefused) {
    Loop sm;
    EXPECT_EQ(sm.enterAt(atWork(), LoopState::Priming), ConfigurationRejection::CurrentNotActive)
        << "the current state is the one the machine is standing in, so it is in the set by "
           "definition";
}

// §scxml-3.11 makes the current state the ATOMIC state the engine descended to.
// A compound one is the shape a host produces when it journals the ancestor
// rather than the leaf.
TEST_F(ConfigurationEntryAotTest, ANonAtomicCurrentStateIsRefused) {
    Loop sm;
    EXPECT_EQ(sm.enterAt(atWork(), LoopState::Running), ConfigurationRejection::CurrentNotAtomic);
}

// The claim that makes every refusal above safe to act on: validation runs
// BEFORE any mutation, so a host that gets a rejection still holds the machine
// it had. Without this the door could half-enter, and a host reading a
// rejection would be told nothing happened while the engine had already moved.
TEST_F(ConfigurationEntryAotTest, ARefusedEntryLeavesTheEngineUntouched) {
    Loop sm;
    const auto before = sm.getCurrentState();

    EXPECT_EQ(sm.enterAt({}, LoopState::Working), ConfigurationRejection::Empty);

    EXPECT_EQ(sm.getCurrentState(), before) << "a refused entry moved the current state";
    EXPECT_FALSE(sm.isRunning()) << "a refused entry started the machine";
    EXPECT_TRUE(sm.getPolicy().getActiveStates().empty()) << "a refused entry wrote an active set";
}

}  // namespace

}  // namespace SCE::Tests
