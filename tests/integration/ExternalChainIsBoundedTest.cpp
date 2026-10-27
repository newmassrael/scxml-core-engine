// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine that answers an event by sending itself the next one, with no
// target, never lets the external queue empty — C++ Interpreter path.
//
// Every macrostep of such a machine ends, so `MAX_MACROSTEP_MICROSTEPS` never
// applies, and the main event loop takes the next external event whenever the
// queue is not empty: `processEvent` did not return. This driver holds the
// Interpreter to the budget ARCHITECTURE.md "External-Event Budget" states as
// one contract for every engine: the same outcomes, the same arithmetic, the
// same document as the Python, Rust, Go, Kotlin, C11 and C++ AOT ones.
//
// `processEvent` handles the host's event directly and then runs the loop, so it
// starts with one event spent: the host's event is the first of its invocation
// whether an engine queues it first or processes it directly.
//
// Only `processEvent` and `start` run the loop here. This engine has no `tick`
// or `advance_time`: its scheduler delivers each delayed event from a thread of
// its own, in real time, as a call of its own. So `timed`, whose outcome is a
// clock the host moves, has no deterministic form on this engine and is not
// driven, rather than leaving a test that sleeps and calls it a bound.
//
// `zero` and `zero_expr` ARE driven, and for a reason measured rather than
// assumed (2026-10-02): a delay that is zero, written out or evaluated from an
// expression, never reaches this engine's scheduler. It is delivered straight to
// the external queue, so it is the drain's budget that bounds it and the call
// returns cut, with nothing left running on the scheduler's thread afterwards.
//
// This engine also runs more than one session in one process, which the
// generated engines do not: a parent and the child it invoked each have a loop
// and a budget of their own, and a chain that bounces between them has to be
// cut on one of them. `ExternalChainAcrossSessionsTest` holds that.
//
// Fixture: tests/integration/external_chain_is_bounded.scxml (outside
// `integration_resources/` for the reason
// `scripts/regen_external_chain_is_bounded.sh` states).

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "integration/ExternalEventBudgetTable.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <fstream>
#include <gtest/gtest.h>
#include <sstream>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

/// A parent that answers the host's `go` by pinging its invoked child, and a
/// child that answers every ping by sending `pong` to its parent, which pings
/// again. Every macrostep of either ends: only the queues keep it going.
constexpr const char *PING_PONG = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s0">
  <datamodel><data id="rounds" expr="0"/></datamodel>
  <state id="s0">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="c0">
          <state id="c0">
            <transition event="ping"><send target="#_parent" event="pong"/></transition>
          </state>
        </scxml>
      </content>
    </invoke>
    <transition event="go"><send target="#_kid" event="ping"/></transition>
    <transition event="pong">
      <assign location="rounds" expr="rounds + 1"/>
      <send target="#_kid" event="ping"/>
    </transition>
  </state>
</scxml>
)";

}  // namespace

/// The machine on this engine's own dispatcher chain, and the reading of its
/// datamodel. What a document is, and what the test does with it, is the
/// derived fixture's.
class InterpreterBudgetTest : public ::testing::Test {
protected:
    /// The default the contract states, read from the table rather than from
    /// the engine: a test that asked the engine for its own limit would agree
    /// with any limit, including one an edit moved by three orders of magnitude.
    static uint32_t defaultBudget() {
        return externalEventBudgetCount({"default_budget"});
    }

    /// Load @p document and start it.
    void startMachine(const std::string &document) {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        sm_ = std::make_shared<StateMachine>(*engine_);
        // §scxml-6.2: a `<send>` needs the dispatcher chain (scheduler -> target
        // factory -> dispatcher) to reach this session's own external queue, and
        // without it every link of the chain is an `error.execution` instead.
        scheduler_ = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                try {
                    return target->send(event).get().isSuccess;
                } catch (...) {
                    return false;
                }
            });
        raiser_ = std::make_shared<EventRaiserImpl>();
        raiser_->setScheduler(scheduler_);
        sm_->setEventRaiser(raiser_);
        sm_->setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler_, std::make_shared<EventTargetFactoryImpl>(raiser_)));
        ASSERT_TRUE(sm_->loadSCXMLFromString(document));
        ASSERT_TRUE(sm_->start());
    }

    void TearDown() override {
        sm_.reset();
        if (scheduler_) {
            scheduler_->shutdown(true);
        }
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// The fixture's `<assign>`s are the only witness of how far a chain got:
    /// every outcome leaves the machine in a state the configuration alone
    /// cannot tell apart from the others.
    std::string counter(const std::string &name) {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), name).get();
        EXPECT_TRUE(result.isSuccess()) << "the document declares `" << name << "` in its datamodel";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<EventSchedulerImpl> scheduler_;
    std::shared_ptr<EventRaiserImpl> raiser_;
    std::shared_ptr<StateMachine> sm_;
};

class ExternalChainIsBoundedTest : public InterpreterBudgetTest {
protected:
    void SetUp() override {
        const std::string fixture =
            std::string(SCE_PROJECT_ROOT) + "/tests/integration/external_chain_is_bounded.scxml";
        std::ifstream in(fixture);
        ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
        std::ostringstream buffer;
        buffer << in.rdbuf();

        startMachine(buffer.str());
        ASSERT_EQ(sm_->getCurrentState(), "idle");
    }
};

class ExternalChainAcrossSessionsTest : public InterpreterBudgetTest {
protected:
    void SetUp() override {
        startMachine(PING_PONG);
        ASSERT_EQ(sm_->getCurrentState(), "s0");
    }
};

TEST_F(ExternalChainIsBoundedTest, TheDefaultBudgetIsTheDocumentedOne) {
    EXPECT_EQ(sm_->getMaxExternalEventsPerCall(), defaultBudget());
    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, 0u);
    EXPECT_EQ(sm_->getStatistics().lastTruncatedEvent, "")
        << "nothing has been refused before the machine has done anything";
}

/// This test returning at all is half the assertion: before the budget the call
/// did not.
TEST_F(ExternalChainIsBoundedTest, AChainThatCannotEndIsCutAtTheBudgetAndTheCallReturns) {
    sm_->processEvent("spin");

    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"spin", "truncated"}))
        << "the call handed control back with an event still queued, and said so; without the count the host sees a "
           "machine that is running and has returned, with no sign that anything went wrong";
    // The host's own event is the first of the invocation, so the budget buys the
    // host's event and then `budget - 1` links.
    EXPECT_EQ(counter("links"), std::to_string(defaultBudget() - 1))
        << "the chain must run exactly as far as the budget allows: fewer means the call was cut early, more means "
           "the budget moved";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedEvent, "link")
        << "the count says a call did not reach quiet; this says what it was still taking";
    EXPECT_TRUE(sm_->isRunning())
        << "the chain was cut, not the machine: the document is legal, and refusing to run it "
           "forever is the engine's decision to report, not a reason to stop a machine "
           "whose other states still work";
}

/// The half that makes the count mean something: a chain that ends on its own is
/// not refused, however close to the budget it comes. `bounded` is the host's
/// event and five laps, six in all.
TEST_F(ExternalChainIsBoundedTest, TheBudgetIsExactForAChainThatEndsByItself) {
    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(externalEventBudgetCount({"exact", "budget"})));
    sm_->processEvent("bounded");

    EXPECT_EQ(counter("laps"), std::to_string(externalEventBudgetInt({"exact", "laps"})));
    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"exact", "truncated"}))
        << "a call that takes exactly the budget and empties the queue refused nothing: a long chain is not a runaway";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedEvent, "");
}

TEST_F(ExternalChainIsBoundedTest, OneEventShortOfTheChainLeavesTheLastLapQueued) {
    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(externalEventBudgetCount({"one_short", "budget"})));
    sm_->processEvent("bounded");

    EXPECT_EQ(counter("laps"), std::to_string(externalEventBudgetInt({"one_short", "laps"})))
        << "one `lap` was left queued";
    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"one_short", "truncated"}));
    EXPECT_EQ(sm_->getStatistics().lastTruncatedEvent, "lap");
}

/// What the refusal did with the events it would not take: it left them queued.
/// An engine that dropped the queue stops short and never finishes; one that ran
/// the chain anyway finishes it in the first call.
TEST_F(ExternalChainIsBoundedTest, ARefusedCallLeavesTheQueueSoTheNextCallFinishesTheChain) {
    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(externalEventBudgetCount({"resume", "budget"})));

    sm_->processEvent("resume");
    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"resume", "truncated"}));
    EXPECT_EQ(counter("beats"), std::to_string(externalEventBudgetInt({"resume", "beats_after_first_call"})))
        << "the host's event and the beats the budget bought";

    sm_->processEvent("poke");
    EXPECT_EQ(counter("beats"), std::to_string(externalEventBudgetInt({"resume", "beats_after_second_call"})))
        << "the second call took the beats the first left on the queue, each in a budget of its own, and finished";
    EXPECT_EQ(counter("pokes"), std::to_string(externalEventBudgetInt({"resume", "pokes"})))
        << "and the host's second event was heard";
    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"resume", "truncated"}))
        << "the second call ended the way the clause says: nothing more is counted";
}

TEST_F(ExternalChainIsBoundedTest, AHostChoosesTheBudgetAndABudgetThatTakesNoEventIsRefused) {
    const uint32_t chosen = externalEventBudgetCount({"host", "budget"});
    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(chosen));
    EXPECT_EQ(sm_->getMaxExternalEventsPerCall(), chosen);
    for (const auto &refused : externalEventBudgetValue({"host", "refused"})) {
        // The engine takes an unsigned budget, so a negative one the table lists
        // cannot be written to it: the type refuses it before any call does.
        const int64_t budget = refused.get<int64_t>();
        if (budget < 0) {
            continue;
        }
        EXPECT_FALSE(sm_->setMaxExternalEventsPerCall(static_cast<uint32_t>(budget)))
            << "a budget of " << budget << " takes no event and was accepted";
        EXPECT_EQ(sm_->getMaxExternalEventsPerCall(), chosen) << "a refused budget changes nothing";
    }
}

/// `delay="0ms"` is due at the instant being processed. This engine delivers it
/// straight to the external queue, where the drain's budget bounds it, so the
/// call returns cut: the host's event and `budget - 1` blinks.
TEST_F(ExternalChainIsBoundedTest, AChainThroughAStaticDelayOfZeroIsCutAtTheBudgetAndTheCallReturns) {
    const uint32_t budget = externalEventBudgetCount({"static_zero", "budget"});
    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(budget));

    sm_->processEvent("zero");

    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"static_zero", "truncated"}));
    EXPECT_EQ(counter("blinks"), std::to_string(budget - 1))
        << "the host's event and `budget - 1` blinks, no more and no fewer";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedEvent, "blink");
}

/// The same chain through `delayexpr="'0ms'"`, which has no static value an
/// engine could read as undelayed. The expression evaluates to zero and this
/// engine delivers it the same way.
TEST_F(ExternalChainIsBoundedTest, AChainThroughADelayExpressionThatIsZeroIsCutAtTheBudgetAndTheCallReturns) {
    const uint32_t budget = externalEventBudgetCount({"zero_expr", "budget"});
    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(budget));

    sm_->processEvent("zero_expr");

    EXPECT_EQ(sm_->getStatistics().truncatedEventChains,
              externalEventBudgetCount({"zero_expr", "truncated_after_tick"}));
    EXPECT_EQ(counter("exprs"), std::to_string(budget - 1))
        << "the host's event and `budget - 1` blinks, no more and no fewer";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedEvent, "blink");
}

/// The budget a host sets governs the next invocation, not the one that already
/// ended: a host that was handed a cut call raises the budget and calls again.
TEST_F(ExternalChainIsBoundedTest, ABudgetSetBetweenCallsGovernsTheNextOne) {
    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(externalEventBudgetCount({"budget_between_calls", "first_budget"})));
    sm_->processEvent("resume");
    EXPECT_EQ(counter("beats"),
              std::to_string(externalEventBudgetInt({"budget_between_calls", "beats_after_first_call"})))
        << "the host's event and two beats, then cut";
    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"resume", "truncated"}));

    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(externalEventBudgetCount({"budget_between_calls", "second_budget"})));
    sm_->processEvent("poke");
    EXPECT_EQ(counter("beats"), std::to_string(externalEventBudgetInt({"resume", "beats_after_second_call"})))
        << "the new budget took the whole remainder of the chain in one call";
    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"resume", "truncated"}))
        << "and that call was not cut";
}

/// A parent and the child it invoked each run a loop of their own, and neither
/// generated engine has that shape. Each answers the other's event with one of its
/// own, so no macrostep of either fails to end and neither queue ever stays
/// empty. The parent's loop takes every `pong` the child sends it, so it is the
/// parent's budget that cuts the chain: the host's `go` and forty-nine rounds.
TEST_F(ExternalChainAcrossSessionsTest, AChainThatBouncesBetweenAParentAndItsChildIsCutOnTheParentsBudget) {
    ASSERT_TRUE(sm_->setMaxExternalEventsPerCall(externalEventBudgetCount({"across_sessions", "budget"})));

    sm_->processEvent("go");

    EXPECT_EQ(sm_->getStatistics().truncatedEventChains, externalEventBudgetCount({"across_sessions", "truncated"}))
        << "the call handed control back with a `pong` still queued, and said so";
    EXPECT_EQ(counter("rounds"), std::to_string(externalEventBudgetInt({"across_sessions", "rounds"})))
        << "the host's event and forty-nine rounds, no more and no fewer";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedEvent, "pong");
    EXPECT_TRUE(sm_->isRunning());
}

}  // namespace Tests
}  // namespace SCE
