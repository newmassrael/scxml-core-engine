// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.2.4 + §scxml-6.3 — a `<send delay>` addressed to a HOST-served Event
// I/O Processor waits, and can be cancelled while it waits. C++ AOT path.
//
// §scxml-6.2.4 puts the wait before the dispatch and says nothing about which
// processor the send named; §scxml-6.2.5 makes that set open. Put together, a
// host-served send carrying a delay is an ordinary delayed send whose delivery
// happens to be somebody else's. It was not: every backend chose the host
// branch ahead of the delay branch in one `elif` chain per language, so the act
// was performed at the instant the block ran and `delay` was discarded — while
// the manifest went on answering `needs_event_scheduler: true`, telling the
// host to drive with `tick()` for a wait the engine had already thrown away.
//
// Driven entirely on `ManualClock`. Nothing here sleeps and nothing here can be
// decided by how loaded the build machine is: the host sets what time it is and
// the engine answers with the configuration that time implies. That matters
// more than usual on this axis, because a wall-clock version of the first case
// would pass on a slow machine for the wrong reason — the handler running
// "early" is only observable against a clock the test controls.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml
// (shared with the Rust / C11 / Go / Kotlin / Python channels;
// `tests/CMakeLists.txt` compiles it here with the same `--host-processor`
// declaration `scripts/regen_host_processor.sh` passes there).

#include "statechart_delayed_host_send_sm.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <vector>

#include "common/SceClock.h"
#include "core/HostProcessor.h"

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::statechart_delayed_host_send::statechart_delayed_host_send;
using State = SCE::Generated::statechart_delayed_host_send::State;

/// The type the fixture was compiled for; `tests/CMakeLists.txt` passes this
/// same string to `--host-processor`.
constexpr const char *DECLARED_TYPE = "x-sce-host";

/// A machine on host-owned time, with the handler each case decides on.
///
/// The clock is installed BEFORE `initialize()` — the engine refuses it
/// afterwards, because deadlines armed against one clock do not compare with
/// another — and registration happens before it too: the fixture's first send
/// is armed on entry to its initial state.
class DelayedHostSendAotTest : public ::testing::Test {
protected:
    /// What the handler saw, in call order: the engine's own reading of "now"
    /// at the moment it was asked to perform the act.
    ///
    /// The engine's clock rather than the test's bookkeeping, because that is
    /// the number the contract is about — a handler called at 0 ms for a
    /// `delay="200ms"` send is the defect, and any other witness (a counter, a
    /// wall-clock stamp) only says it happened, not when the engine thought it
    /// was.
    std::vector<uint64_t> calls;

    std::shared_ptr<SCE::ManualClock> clock = std::make_shared<SCE::ManualClock>(0);

    /// Register a handler that answers `turn.done` and records when it ran.
    void registerAnsweringHandler(Machine &sm) {
        sm.registerEventProcessor(DECLARED_TYPE, [this](const SCE::HostSendRequest &) {
            calls.push_back(clock->elapsedMs());
            return std::vector<SCE::HostSendResponse>{{"turn.done", ""}};
        });
    }

    static void boot(Machine &sm, const std::shared_ptr<SCE::ManualClock> &c) {
        sm.setClock(c);
        sm.initialize();
    }
};

}  // namespace

// The axis. `waiting` arms a host-served send for 200 ms and an ordinary one
// for 100 ms; the ordinary one must arrive first, which is only true if the
// host-served one waited.
//
// The `tooEarly` final state is what the document reaches when it did not: the
// handler's reply is on the queue before the machine has been anywhere, so
// `turn.done` wins the race its own `delay` was supposed to lose.
TEST_F(DelayedHostSendAotTest, AHostServedSendWaitsForItsDelay) {
    Machine sm;
    registerAnsweringHandler(sm);
    boot(sm, clock);

    // Nothing is due at 0 ms. This is the whole defect in one assertion: with
    // the host branch chosen ahead of the delay branch, `initialize()` has
    // already performed the act by the time this line runs.
    EXPECT_TRUE(calls.empty()) << "the handler was asked to perform a delay=\"200ms\" send at " << clock->elapsedMs()
                               << " ms. §scxml-6.2.4 makes the delay the wait the document asked for, and "
                                  "§scxml-6.2.5 does not exempt a host-served processor from it";
    EXPECT_EQ(sm.getCurrentState(), State::Waiting);

    // 100 ms: the ordinary `probe` is due, the host-served send is not.
    sm.advanceTimeMs(100);
    EXPECT_EQ(sm.getCurrentState(), State::Armed) << "the 100 ms `probe` did not arrive first";
    EXPECT_TRUE(calls.empty()) << "the host-served send was dispatched before its 200 ms deadline";

    // 200 ms: now it is due, and the handler's reply moves the machine on.
    sm.advanceTimeMs(100);
    ASSERT_EQ(calls.size(), 1u) << "the host-served send did not fire at its 200 ms deadline";
    EXPECT_EQ(calls[0], 200u);
    EXPECT_EQ(sm.getCurrentState(), State::Cancelling) << "the handler's `turn.done` did not reach the document";
}

// §scxml-6.3: a `<cancel>` drops a delayed send that has not been dispatched. A
// host-served one is not exempt, and the witness is host-side: the handler must
// never be asked to perform the cancelled act at all.
//
// This is the half that says which queue the deferred send is in. An engine
// that honoured the delay by any private means — a side list, a timer thread —
// would pass the case above and fail here, because `<cancel sendid>` reaches
// the scheduler and nothing else.
TEST_F(DelayedHostSendAotTest, ACancelDropsAPendingHostServedSend) {
    Machine sm;
    registerAnsweringHandler(sm);
    boot(sm, clock);

    sm.advanceTimeMs(100);  // probe     -> armed
    sm.advanceTimeMs(100);  // turn.done -> cancelling (arms h2 for 400)
    sm.advanceTimeMs(100);  // settle    -> cancelPending (cancels h2)
    ASSERT_EQ(sm.getCurrentState(), State::CancelPending)
        << "the second round did not reach the state that runs <cancel sendid=\"h2\">";

    // 400 ms: h2's deadline. It was cancelled at 300, so nothing may happen.
    sm.advanceTimeMs(100);
    EXPECT_EQ(calls.size(), 1u) << "the handler was asked to perform `h2` at 400 ms after <cancel sendid=\"h2\"> ran "
                                   "at 300 ms. A host-served act that a document cancelled must not reach the host: "
                                   "the side effect is the point of the act, and the document cannot take it back";
    EXPECT_NE(sm.getCurrentState(), State::CancelLost) << "`turn.done` arrived for the cancelled send";

    // 500 ms: `finish`. The verdict is itself scheduled, so a channel whose
    // tick loop stopped working fails here rather than passing by not moving.
    sm.advanceTimeMs(100);
    EXPECT_EQ(sm.getCurrentState(), State::Pass) << "the machine did not reach `pass`";
}

// A deferred act whose handler was never registered is still an act nobody
// performed, and §scxml-6.2 reports that as `error.execution` — at the moment it
// was to be performed, not at the moment it was armed.
//
// The immediate path raises this at the send site. The deferred path cannot:
// the send site has already returned by the time the deadline arrives, so the
// engine owes the report. Without this case a wiring mistake on a delayed send
// is perfect silence — the document waits for a reply that no longer has anyone
// to come from.
TEST_F(DelayedHostSendAotTest, ADeferredSendWithNoHandlerReportsItWhenItComesDue) {
    Machine sm;
    boot(sm, clock);

    // At 100 ms the machine is in `armed`, whose `error.execution` transition is
    // the witness. Nothing has reported anything yet: the send was armed, not
    // performed, so there is nothing to report.
    sm.advanceTimeMs(100);
    ASSERT_EQ(sm.getCurrentState(), State::Armed)
        << "the report arrived before the send was due; error.execution must be raised when the act was to be "
           "performed, not when it was armed";

    // 200 ms: the deadline. Nobody is registered, so nobody performs it, and
    // §scxml-6.2 says so.
    sm.advanceTimeMs(100);
    EXPECT_NE(sm.getCurrentState(), State::Cancelling)
        << "nothing was registered to perform the act, yet `turn.done` arrived";
    EXPECT_EQ(sm.getCurrentState(), State::Unserved)
        << "the deadline passed with no handler registered and nothing was reported. The send site that raises this "
           "for an immediate send returned when the send was armed, so whatever holds the deferred act owes the "
           "report — without it a wiring mistake on a delayed send is perfect silence";
}

// The engine must be able to say when the deferred host send comes due, or a
// host driving on `timeUntilNextScheduled()` sleeps straight past it.
//
// A deferred act kept anywhere the deadline query cannot see would leave this
// answering "nothing owed" at 0 ms while an act was owed at 200.
TEST_F(DelayedHostSendAotTest, TheEngineSaysWhenTheDeferredHostSendIsDue) {
    Machine sm;
    registerAnsweringHandler(sm);
    boot(sm, clock);

    auto due = sm.timeUntilNextScheduled();
    ASSERT_TRUE(due.has_value()) << "two delayed sends are armed at 0 ms, so a deadline is owed";
    EXPECT_EQ(due->count(), 100) << "the nearer of the two armed sends is the 100 ms `probe`";

    sm.advanceTimeMs(100);
    due = sm.timeUntilNextScheduled();
    ASSERT_TRUE(due.has_value()) << "the host-served send is still pending at 100 ms, so a deadline is owed";
    EXPECT_EQ(due->count(), 100) << "at 100 ms the host-served send is 100 ms out. A host sleeping on this answer "
                                    "must land on the deferred act, not past it";
}

}  // namespace SCE::Tests
