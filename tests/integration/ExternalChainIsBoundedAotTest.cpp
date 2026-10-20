// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine that answers an event by sending itself the next one, with no
// target, never lets the external queue empty — C++ AOT path.
//
// Every macrostep of such a machine ends, so `MAX_MACROSTEP_MICROSTEPS` never
// applies, and the main event loop takes the next external event whenever the
// queue is not empty: a host call that drains it did not return. The AOT
// engine's `runMainEventLoop` had that shape until it took the budget
// ARCHITECTURE.md "External-Event Budget" states as one contract for every
// engine. This driver holds this engine to it: the same outcomes, the same
// arithmetic, the same document as the Python, Rust, Go and Kotlin ones.
//
// `processEvent` handles the host's event directly and then runs the loop, so it
// starts with one event spent: the host's event is the first of its invocation
// whether an engine queues it first or processes it directly. The delayed
// outcomes run on `ManualClock`, so nothing here sleeps and the clock moves only
// where a case moves it. This engine hands a STATIC zero delay to its scheduler
// (Rust reads it as undelayed), so `zero` and `zero_expr` both reach the
// same-instant bound here.
//
// Fixture: tests/integration/external_chain_is_bounded.scxml. It is outside
// `integration_resources/` for the reason
// `scripts/regen_external_chain_is_bounded.sh` states.
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(external_chain_is_bounded ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "external_chain_is_bounded_sm.h"
#include "integration/ExternalEventBudgetTable.h"
#include "scripting/ScriptEngineProvider.h"

#include "common/SceClock.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::external_chain_is_bounded::external_chain_is_bounded;

/// The default the contract states, read from the table rather than back from
/// the engine: a test that asked the engine for its own limit would agree with
/// any limit, including one an edit moved by three orders of magnitude.
int64_t defaultBudget() {
    return externalEventBudgetInt({"default_budget"});
}

/// The machine on host-owned time. The clock is installed BEFORE `initialize()`:
/// the engine refuses it afterwards, because deadlines armed against one clock
/// do not compare with another.
std::unique_ptr<SM> started() {
    auto sm = std::make_unique<SM>();
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                  [](::SCE::IScriptEngine *) {}));
    }
    sm->setClock(std::make_shared<SCE::ManualClock>(0));
    sm->initialize();
    return sm;
}

/// The fixture's `<assign>`s are the only witness of how far a chain got: every
/// outcome leaves the machine in a state the configuration alone cannot tell
/// apart from the others.
#define COUNTER(sm, name) ((sm)->getPolicy().name().value_or(-1))

}  // namespace

TEST(ExternalChainIsBoundedAotTest, TheDefaultBudgetIsTheDocumentedOne) {
    auto sm = started();
    EXPECT_EQ(sm->maxExternalEventsPerCall(), defaultBudget());
    EXPECT_EQ(sm->truncatedEventChains(), 0u);
    EXPECT_FALSE(sm->lastTruncatedEvent().has_value());
}

/// This test returning at all is half the assertion: before the budget the call
/// did not.
TEST(ExternalChainIsBoundedAotTest, AChainThatCannotEndIsCutAtTheBudgetAndTheCallReturns) {
    auto sm = started();

    sm->processEvent(SM::Event::Spin);

    EXPECT_EQ(sm->truncatedEventChains(), externalEventBudgetCount({"spin", "truncated"}))
        << "the call handed control back with an event still queued, and said so; without the count the host sees a "
           "machine that is running and has returned, with no sign that anything went wrong";
    // The host's own event is the first of the invocation, so the budget buys the
    // host's event and then `budget - 1` links.
    EXPECT_EQ(COUNTER(sm, links), defaultBudget() - 1)
        << "the chain must run exactly as far as the budget allows: fewer means the call was cut early, more means "
           "the budget moved";
    ASSERT_TRUE(sm->lastTruncatedEvent().has_value());
    EXPECT_EQ(sm->lastTruncatedEvent().value(), SM::Event::Link)
        << "the count says a call did not reach quiet; this says what it was still taking";
    EXPECT_TRUE(sm->isRunning()) << "the chain was cut, not the machine: the document is legal, and refusing to run it "
                                    "forever is the engine's decision to report, not a reason to stop a machine "
                                    "whose other states still work";
}

/// The half that makes the count mean something: a chain that ends on its own is
/// not refused, however close to the budget it comes. `bounded` is the host's
/// event and five laps, six in all.
TEST(ExternalChainIsBoundedAotTest, TheBudgetIsExactForAChainThatEndsByItself) {
    auto exactly = started();
    ASSERT_TRUE(exactly->setMaxExternalEventsPerCall(externalEventBudgetCount({"exact", "budget"})));
    exactly->processEvent(SM::Event::Bounded);
    EXPECT_EQ(COUNTER(exactly, laps), externalEventBudgetInt({"exact", "laps"}));
    EXPECT_EQ(exactly->truncatedEventChains(), externalEventBudgetCount({"exact", "truncated"}))
        << "a call that takes exactly the budget and empties the queue refused nothing: a long chain is not a runaway";
    EXPECT_FALSE(exactly->lastTruncatedEvent().has_value());

    auto oneShort = started();
    ASSERT_TRUE(oneShort->setMaxExternalEventsPerCall(externalEventBudgetCount({"one_short", "budget"})));
    oneShort->processEvent(SM::Event::Bounded);
    EXPECT_EQ(COUNTER(oneShort, laps), externalEventBudgetInt({"one_short", "laps"})) << "one `lap` was left queued";
    EXPECT_EQ(oneShort->truncatedEventChains(), externalEventBudgetCount({"one_short", "truncated"}));
    ASSERT_TRUE(oneShort->lastTruncatedEvent().has_value());
    EXPECT_EQ(oneShort->lastTruncatedEvent().value(), SM::Event::Lap);
}

/// What the refusal did with the events it would not take: it left them queued.
/// An engine that dropped the queue stops short and never finishes; one that ran
/// the chain anyway finishes it in the first call.
TEST(ExternalChainIsBoundedAotTest, ARefusedCallLeavesTheQueueSoTheNextCallFinishesTheChain) {
    auto sm = started();
    ASSERT_TRUE(sm->setMaxExternalEventsPerCall(externalEventBudgetCount({"resume", "budget"})));

    sm->processEvent(SM::Event::Resume);
    EXPECT_EQ(sm->truncatedEventChains(), externalEventBudgetCount({"resume", "truncated"}));
    EXPECT_EQ(COUNTER(sm, beats), externalEventBudgetInt({"resume", "beats_after_first_call"}))
        << "the host's event and the beats the budget bought";

    sm->processEvent(SM::Event::Poke);
    EXPECT_EQ(COUNTER(sm, beats), externalEventBudgetInt({"resume", "beats_after_second_call"}))
        << "the second call took the beats the first left on the queue, each in a budget of its own, and finished";
    EXPECT_EQ(COUNTER(sm, pokes), externalEventBudgetInt({"resume", "pokes"}))
        << "and the host's second event was heard";
    EXPECT_EQ(sm->truncatedEventChains(), externalEventBudgetCount({"resume", "truncated"}))
        << "the second call ended the way the clause says: nothing more is counted";
}

/// `delay="0ms"` is due at the instant being processed. This engine hands it to
/// its scheduler, so a tick pops the entry, its handler arms another due at the
/// same reading, and the tick that is popping finds it. Each pass takes one
/// event, so a budget on the drain alone never trips — this is the case
/// ARCHITECTURE.md rule 5 exists for. This test returning at all is the
/// assertion.
TEST(ExternalChainIsBoundedAotTest, AChainThroughAStaticDelayOfZeroDoesNotKeepTheTickFromReturning) {
    auto sm = started();
    const uint32_t budget = externalEventBudgetCount({"static_zero", "budget"});
    ASSERT_TRUE(sm->setMaxExternalEventsPerCall(budget));

    sm->processEvent(SM::Event::Zero);
    EXPECT_EQ(sm->truncatedEventChains(), externalEventBudgetCount({"static_zero", "truncated_after_entry"}))
        << "entering the state arms one entry and pops none: nothing has been cut yet";

    sm->tick();

    EXPECT_EQ(sm->truncatedEventChains(), externalEventBudgetCount({"static_zero", "truncated"}))
        << "the tick popped entries due at its own reading until the budget, left the due one waiting and said so";
    EXPECT_EQ(COUNTER(sm, blinks), static_cast<int64_t>(budget))
        << "the budget of pops at one reading, no more and no fewer";
    ASSERT_TRUE(sm->lastTruncatedEvent().has_value());
    EXPECT_EQ(sm->lastTruncatedEvent().value(), SM::Event::Blink);
}

/// The same chain through `delayexpr="'0ms'"`, which has no static value an
/// engine could read as undelayed.
TEST(ExternalChainIsBoundedAotTest, AChainThroughADelayExpressionThatIsZeroDoesNotKeepTheTickFromReturning) {
    auto sm = started();
    ASSERT_TRUE(sm->setMaxExternalEventsPerCall(externalEventBudgetCount({"zero_expr", "budget"})));

    sm->processEvent(SM::Event::Zero_expr);
    EXPECT_EQ(sm->truncatedEventChains(), externalEventBudgetCount({"zero_expr", "truncated_after_entry"}))
        << "entering the state pops nothing: nothing has been cut yet";
    sm->tick();

    EXPECT_EQ(sm->truncatedEventChains(), externalEventBudgetCount({"zero_expr", "truncated_after_tick"}));
    EXPECT_EQ(COUNTER(sm, exprs), externalEventBudgetInt({"zero_expr", "exprs"}))
        << "the budget of pops at one reading, no more and no fewer";
}

/// Eight pulses, each due one millisecond after the last. They are due at later
/// instants, so a legitimate time-driven workload is not a runaway however small
/// the budget: three here, against eight events.
TEST(ExternalChainIsBoundedAotTest, AChainThatIsFiniteBecauseTheClockIsIsNotRefused) {
    const uint32_t budget = externalEventBudgetCount({"timed", "budget"});
    const int64_t pulseCount = externalEventBudgetInt({"timed", "pulses"});
    const int64_t intervalMs = externalEventBudgetInt({"timed", "interval_ms"});
    const uint32_t truncated = externalEventBudgetCount({"timed", "truncated"});

    auto walked = started();
    ASSERT_TRUE(walked->setMaxExternalEventsPerCall(budget));
    walked->processEvent(SM::Event::Timed);
    for (int64_t i = 0; i < pulseCount; ++i) {
        walked->advanceTimeMs(static_cast<uint64_t>(intervalMs));
    }
    EXPECT_EQ(COUNTER(walked, pulses), pulseCount);
    EXPECT_EQ(walked->truncatedEventChains(), truncated)
        << "each pulse came in a tick of its own, at an instant of its own";

    auto jumped = started();
    ASSERT_TRUE(jumped->setMaxExternalEventsPerCall(budget));
    jumped->processEvent(SM::Event::Timed);
    jumped->advanceTimeMs(static_cast<uint64_t>(pulseCount * intervalMs));
    EXPECT_EQ(COUNTER(jumped, pulses), pulseCount);
    EXPECT_EQ(jumped->truncatedEventChains(), truncated)
        << "eight entries came due on the way to one reading, seven of them at earlier instants: a clock that moved a "
           "long way is bounded by how far it moved, and is not a chain that does not end";
}

TEST(ExternalChainIsBoundedAotTest, AHostChoosesTheBudgetAndABudgetThatTakesNoEventIsRefused) {
    auto sm = started();
    const uint32_t chosen = externalEventBudgetCount({"host", "budget"});
    ASSERT_TRUE(sm->setMaxExternalEventsPerCall(chosen));
    EXPECT_EQ(sm->maxExternalEventsPerCall(), chosen);
    for (const auto &refused : externalEventBudgetValue({"host", "refused"})) {
        // The engine takes an unsigned budget, so a negative one the table lists
        // cannot be written to it: the type refuses it before any call does.
        const int64_t budget = refused.get<int64_t>();
        if (budget < 0) {
            continue;
        }
        EXPECT_FALSE(sm->setMaxExternalEventsPerCall(static_cast<uint32_t>(budget)))
            << "a budget of " << budget << " takes no event and was accepted";
        EXPECT_EQ(sm->maxExternalEventsPerCall(), chosen) << "a refused budget changes nothing";
    }
}

}  // namespace SCE::Tests
