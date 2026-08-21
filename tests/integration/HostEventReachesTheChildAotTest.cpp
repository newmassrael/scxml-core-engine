// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: autoforward is owed to the external event, not to the door it
// came through — C++ AOT path.
//
// This engine has two doors an external event can come through:
// `processNextExternalEvent()` pops the external queue, and `processEvent()`
// hands the event straight to `executeTransition`. Appendix D's
// `mainEventLoop` binds the preliminary step (`applyFinalize` + the autoforward
// `send`) to the external event it is about to select transitions for, so the
// step belongs to both doors.
//
// Measured 2026-08-21: it was written inline in the drain, so `processEvent()`
// skipped it. The identical machine forwarded a `hostPing` raised onto the
// queue and dropped one handed to `processEvent()` — same fixture, same event,
// only the door differed. The four sibling `autoforward_*` stems all drive
// through the drain, so none of them could see it, and `truncatedMacrosteps()`
// had already been recorded at both doors for exactly the same reason.
//
// Sibling of `HostEventReachesTheChildTest.cpp` (Interpreter channel).
//
// Fixture: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(host_event_reaches_the_child ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "host_event_reaches_the_child_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::host_event_reaches_the_child::host_event_reaches_the_child;
using State = SCE::Generated::host_event_reaches_the_child::State;
using Event = SCE::Generated::host_event_reaches_the_child::Event;

bool isVerdict(State state) {
    return state == State::Pass || state == State::Fail;
}

/// Drive the machine until the child's handshake has moved it to `armed`, the
/// one state that can be handed an event from outside. Bounded rather than
/// timed: every tick here is the machine's own work, so a machine that has not
/// arrived after this many is not slow, it is not going to.
void driveToArmed(SM &sm) {
    for (int i = 0; i < 50 && sm.getCurrentState() != State::Armed && !isVerdict(sm.getCurrentState()); ++i) {
        sm.tick();
    }
}

void drain(SM &sm) {
    for (int i = 0; i < 50 && !isVerdict(sm.getCurrentState()); ++i) {
        sm.tick();
    }
}

}  // namespace

TEST(HostEventReachesTheChildAotTest, AnEventTheHostHandsOverReachesTheAutoforwardChild) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned by
        // the ScriptEngineProvider singleton; the shared_ptr is a non-owning
        // view. Mirrors SimpleAotTest's W3C-AOT pattern.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    driveToArmed(sm);
    ASSERT_EQ(sm.getCurrentState(), State::Armed)
        << "the probe child never sent `ready`, so the fixture never reached the state where a "
           "host event can be handed over — this is a broken handshake, not a forwarding verdict";

    // The axis: the host-facing door, not `raiseExternal` + `tick`.
    sm.processEvent(Event::HostPing);

    drain(sm);
    ASSERT_TRUE(isVerdict(sm.getCurrentState()))
        << "the machine reached no verdict — the probe child answered neither, so neither "
           "`hostPing` nor `marker` reached it";

    EXPECT_EQ(sm.getCurrentState(), State::Pass)
        << "the probe child answered `sawMarkerOnly`, so the event the host handed to "
           "`processEvent` was never forwarded to it: the child only ever saw the `marker` the "
           "parent's own transition body sent. W3C Appendix D `mainEventLoop` runs the autoforward "
           "`send` against the external event before it selects transitions for it, whichever door "
           "the event arrived through — an engine that runs that step only in its queue drain "
           "leaves an `autoforward` child blind to everything its host delivers.";
}

}  // namespace SCE::Tests
