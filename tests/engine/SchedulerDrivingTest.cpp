// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief A host driving with the wrong entry point is told, rather than left waiting
 *
 * §scxml-6.2's delayed `<send>` reaches the external queue only through
 * `StaticExecutionEngine::tick()`. `step()` runs a macrostep and never consults
 * the scheduler, so a host that only ever calls it gets no delayed event, no
 * error and no warning — measured 2026-08-18 in C++, Go and Python alike, each
 * of them sitting in its initial state after two seconds of the wrong loop and
 * finishing immediately under the right one.
 *
 * The generator has always known which machines are in that position; it said
 * so only on the generate manifest, which a consumer compiling generated
 * sources never reads. Generated policies now declare
 * `NEEDS_EVENT_SCHEDULER`, and the engine counts the macrosteps taken while
 * nothing polled the scheduler, so the mistake becomes something a program can
 * see.
 *
 * The policies here are hand-written rather than generated, and that is the
 * point of two of these cases: the constant is *detected*
 * (`NeedsEventSchedulerTrait`) rather than required by `BaseStatePolicy`, so a
 * policy that predates it — the mesh worker harnesses, any consumer's own —
 * must keep compiling and must count nothing. A generated policy's spelling of
 * the same constant is measured on the codegen side, in
 * `sce-build/tests/tick_requirement_reaches_the_consumer.rs`, across all six
 * backends.
 */

#include <gtest/gtest.h>

#include "core/StatePolicyConcepts.h"
#include "static/StaticExecutionEngine.h"

namespace {

enum class TinyState { Waiting, Done };
enum class TinyEvent { Timeout };

/// The shape every policy below shares: two states, one event, no actions.
struct TinyPolicyBase {
    using State = TinyState;
    using Event = TinyEvent;

    static constexpr bool HAS_PARALLEL_STATES = false;

    static constexpr State initialState() noexcept {
        return TinyState::Waiting;
    }

    static constexpr bool isFinalState(State s) noexcept {
        return s == TinyState::Done;
    }

    static constexpr std::optional<State> getParent(State) noexcept {
        return std::nullopt;
    }

    static constexpr bool isCompoundState(State) noexcept {
        return false;
    }

    static constexpr State getInitialChild(State s) noexcept {
        return s;
    }

    /// §scxml-3.10: no compound state here, so there is no child to restore.
    State getInitialOrHistoryChild(State s) const noexcept {
        return s;
    }

    static std::string getStateName(State s) {
        return s == TinyState::Done ? "done" : "waiting";
    }

    static std::string getEventName(Event) {
        return "timeout";
    }

    static std::optional<Event> getEventFromName(const std::string &name) {
        return name == "timeout" ? std::optional<Event>{TinyEvent::Timeout} : std::nullopt;
    }

    // §scxml-3.13: the engine writes the last transition's shape back onto the
    // policy; the static_asserts in StaticExecutionEngine name these exactly.
    mutable bool lastTransitionIsInternal_ = false;
    mutable bool lastTransitionIsTargetless_ = false;
    mutable State lastTransitionSourceState_ = TinyState::Waiting;

    // §scxml-3.8 / 3.9: no document here has entry or exit content, so the
    // engine's calls land on empty bodies.
    template <typename Engine> void executeEntryActions(State, Engine &, std::optional<State> = std::nullopt) {}

    template <typename Engine> void executeExitActions(State, Engine &, const std::vector<State> &) {}

    template <typename Engine> void executeTransitionActions(Engine &) {}

    // §scxml-3.12: `timeout` moves waiting to done; nothing else transitions.
    // What is under test is which entry point delivers that event, not the
    // transition itself.
    template <typename Engine> bool processTransition(State &state, Event event, Engine &) {
        if (state == TinyState::Waiting && event == TinyEvent::Timeout) {
            lastTransitionSourceState_ = state;
            state = TinyState::Done;
            return true;
        }
        return false;
    }
};

/// What codegen emits for a document carrying a delayed `<send>`.
struct SchedulerDrivenPolicy : TinyPolicyBase {
    static constexpr bool NEEDS_EVENT_SCHEDULER = true;
};

/// What it emits for a document that carries none.
struct PlainPolicy : TinyPolicyBase {
    static constexpr bool NEEDS_EVENT_SCHEDULER = false;
};

/// A policy written before the constant existed. It must still compile, and it
/// must be read as "no requirement" rather than as a missing member.
struct LegacyPolicy : TinyPolicyBase {};

using SchedulerDrivenEngine = SCE::Static::StaticExecutionEngine<SchedulerDrivenPolicy>;
using PlainEngine = SCE::Static::StaticExecutionEngine<PlainPolicy>;
using LegacyEngine = SCE::Static::StaticExecutionEngine<LegacyPolicy>;

/// The detection reads the declared value, and reads its absence as `false`.
///
/// Asserted at run time rather than with `static_assert` on purpose. The
/// compile-time form is the stronger check and was written first — but a
/// mutation that breaks the trait then stops the tree compiling, which the
/// mutation harness can only report as INCONCLUSIVE ("the tests never ran"),
/// so the contract would have had no witness that a change to it goes red.
/// What stays compile-time is the part that has to: `LegacyEngine` below
/// instantiates the whole engine against a policy that declares nothing, so a
/// trait turned into a requirement fails the build regardless of this test.
TEST(SchedulerDriving, AbsentConstantReadsAsNoRequirement) {
    EXPECT_TRUE(SCE::Core::NeedsEventSchedulerTrait<SchedulerDrivenPolicy>::value)
        << "a policy that declares the requirement is read as declaring it";
    EXPECT_FALSE(SCE::Core::NeedsEventSchedulerTrait<PlainPolicy>::value)
        << "a policy that declares no requirement is read as declaring none";
    EXPECT_FALSE(SCE::Core::NeedsEventSchedulerTrait<LegacyPolicy>::value)
        << "a policy predating the constant compiles and counts as no requirement";
}

TEST(SchedulerDriving, StepsBeforeAnyTickAreCounted) {
    SchedulerDrivenEngine engine;
    engine.initialize();
    EXPECT_EQ(engine.unattendedSchedulerSteps(), 0u) << "nothing has been stepped yet";

    engine.step();
    engine.step();
    engine.step();

    EXPECT_EQ(engine.unattendedSchedulerSteps(), 3u)
        << "every macrostep taken before a tick on a scheduler-driven machine counts";
}

TEST(SchedulerDriving, AHostThatTicksIsNotAccused) {
    SchedulerDrivenEngine engine;
    engine.initialize();
    engine.step();
    ASSERT_EQ(engine.unattendedSchedulerSteps(), 1u);

    engine.tick();
    for (int i = 0; i < 10; ++i) {
        engine.step();
    }

    EXPECT_EQ(engine.unattendedSchedulerSteps(), 1u)
        << "tick is a superset of step, so mixing them is a legitimate driving loop; "
           "the count freezes at what happened before the first tick";
}

TEST(SchedulerDriving, AMachineWithNoDelayedSendIsNeverCounted) {
    PlainEngine engine;
    engine.initialize();
    for (int i = 0; i < 32; ++i) {
        engine.step();
    }
    EXPECT_EQ(engine.unattendedSchedulerSteps(), 0u)
        << "this machine needs no scheduler, so a step loop is the right way to drive it";
}

TEST(SchedulerDriving, ALegacyPolicyIsNeverCounted) {
    LegacyEngine engine;
    engine.initialize();
    for (int i = 0; i < 32; ++i) {
        engine.step();
    }
    EXPECT_EQ(engine.unattendedSchedulerSteps(), 0u)
        << "detection, not requirement: a policy without the constant is not accused";
}

}  // namespace
