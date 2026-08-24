// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-G-7 `<sce:action>` — C++ AOT compile+run gate for native host dispatch.
//
// A native action names a host operation and lowers to a DIRECT call on a
// generated interface: no script engine, no registry lookup, nothing to
// register at run time. That is what the construct is for — it is engine-free
// by definition and never degrades to a runtime fallback — and it is also why
// this gate cannot be a text comparison. The emitted call either reaches the
// host with the values the event carried or it does not, and only running it
// says which.
//
// Fixture: sce-build/tests/fixtures/event_schema/statechart_native_action.scxml,
// the same document the Rust, Go, Kotlin, Python and C11 channels drive
// (`tests/CMakeLists.txt` compiles it here).
//
// What the cases measure:
//
//   * `appendFragmentPayload` reads two typed `_event.data` fields — a `bytes`
//     payload lowered to `const std::vector<uint8_t>&`, a `uint32` offset —
//     bound from the event's typed payload;
//   * `resetSlot` takes no arguments;
//   * `onIdleEntry` and `onAssemblingExit` appear in NO transition, so they
//     prove the engine-free entry/exit path and that an eventless-only action
//     still gets a generated interface method;
//   * an event raised BY NAME carries no typed payload, and the arg-bearing
//     action must not fire against a value it would take for data. That last
//     one is the half a configuration assertion cannot see: the machine
//     reaches `assembling` either way.

#include "statechart_native_action_sm.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <vector>

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::statechart_native_action::statechart_native_action;
using Actions = SCE::Generated::statechart_native_action::StatechartNativeActionActions;
using State = SCE::Generated::statechart_native_action::State;
using Event = SCE::Generated::statechart_native_action::Event;

/// Host implementation of the generated operations. Records every dispatch so
/// a case can assert the engine-free call path fired with the arguments the
/// event carried.
class Recorder : public Actions {
public:
    void appendFragmentPayload(const std::vector<std::uint8_t> &payload, std::uint32_t offset) override {
        appended.push_back(payload);
        offsets.push_back(offset);
    }

    void resetSlot() override {
        ++resets;
    }

    void onIdleEntry() override {
        ++idleEntries;
    }

    void onAssemblingExit() override {
        ++assemblingExits;
    }

    std::vector<std::vector<std::uint8_t>> appended;
    std::vector<std::uint32_t> offsets;
    int resets = 0;
    int idleEntries = 0;
    int assemblingExits = 0;
};

class NativeActionAotTest : public ::testing::Test {};

TEST_F(NativeActionAotTest, NativeActionDispatchesTypedPayloadToHostInterface) {
    Recorder host;
    // The host is a CONSTRUCTOR argument and the machine has no other
    // constructor: `idle`'s `<onentry>` performs an act, so a host installed
    // after construction would arrive one act too late. That is a compile-time
    // fact here rather than a comment.
    Machine sm(host);
    sm.initialize();

    EXPECT_EQ(sm.getCurrentState(), State::Idle);
    // `<onentry>` of the initial state fires on entry — the engine-free
    // entry-effect path, with no transition having to carry the action.
    EXPECT_EQ(host.idleEntries, 1) << "onIdleEntry must fire on the initial entry to idle";

    // Per-event typed inject: `fragment.received` with a bytes payload and an
    // offset. The transition fires appendFragmentPayload.
    const std::vector<std::uint8_t> abc{'a', 'b', 'c'};
    sm.raiseFragmentReceived(abc, 7);
    sm.step();

    EXPECT_EQ(sm.getCurrentState(), State::Assembling) << "fragment.received must move idle -> assembling";
    ASSERT_EQ(host.appended.size(), 1u) << "appendFragmentPayload fired " << host.appended.size() << " times";
    EXPECT_EQ(host.appended[0], abc) << "the typed bytes payload did not arrive natively";
    ASSERT_EQ(host.offsets.size(), 1u);
    EXPECT_EQ(host.offsets[0], 7u) << "the typed uint32 offset did not arrive";

    // `reset` fires the no-argument resetSlot and returns to idle. Exiting
    // `assembling` fires its `<onexit>` effect; re-entering `idle` fires
    // `<onentry>` a second time.
    sm.raiseExternal(Event::Reset);
    sm.step();

    EXPECT_EQ(sm.getCurrentState(), State::Idle);
    EXPECT_EQ(host.resets, 1) << "resetSlot must have fired once";
    EXPECT_EQ(host.assemblingExits, 1) << "onAssemblingExit must fire when leaving assembling";
    EXPECT_EQ(host.idleEntries, 2) << "re-entering idle must fire its <onentry> again";
}

/// An event raised by NAME carries no typed payload. The transition still
/// fires — the guard is the event name — but the arg-bearing action has
/// nothing to read, and handing the host a value it would take for data is the
/// one outcome this seam must never produce.
///
/// Asserted on the host's record rather than on the configuration, because the
/// machine reaches `assembling` either way.
TEST_F(NativeActionAotTest, NativeActionDoesNotFireWithoutItsTypedPayload) {
    Recorder host;
    Machine sm(host);
    sm.initialize();

    // Raised as a bare enum: no `typedPayload` rides with it, which is exactly
    // what a host reaching past the generated `raiseFragmentReceived` seam
    // would produce.
    sm.raiseExternal(Event::Fragment_received);
    sm.step();

    EXPECT_EQ(sm.getCurrentState(), State::Assembling)
        << "the transition is guarded by the event name and must still be taken";
    EXPECT_EQ(host.appended.size(), 0u) << "appendFragmentPayload fired without a typed payload to read";
    // The eventless effects still ran: they read no payload, so nothing about
    // this delivery should have stopped them.
    EXPECT_EQ(host.idleEntries, 1);
}

}  // namespace

}  // namespace SCE::Tests
