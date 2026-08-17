// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + B.2: a payload a HOST injects reaches the datamodel as a
// value — C++ AOT.
//
// The edge nothing measured. Every other integration fixture drives its
// machine with `processEvent(Event)` and no metadata — measured 2026-08-16, on
// every channel — so the host-to-datamodel boundary was covered by no test at
// all. The W3C suite does not reach it either: its payloads originate inside
// the document (`<send><content>`, `<param>`, `<donedata>`), which every
// backend implements on a separate path from the one an embedder calls.
//
// It is the edge a supervising host actually uses: `examples/ai_loop` answers
// its machine with `{"done":true}` and the document selects on
// `_event.data.done`.
//
// Fixture:
// integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(event_data_arrives_as_sent ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "core/EventMetadata.h"
#include "event_data_arrives_as_sent_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <memory>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::event_data_arrives_as_sent::event_data_arrives_as_sent;

/// The fixture is a flat machine, so its configuration IS the current state —
/// `getActiveStates` is emitted only for machines that carry a `<parallel>`.
bool isActive(SM &sm, SM::State state) {
    return sm.getCurrentState() == state;
}

/// Rendered into every failure message: the fixture lands each way of failing
/// in a `<final>` of its own, so the state names which half broke.
std::string describe(SM &sm) {
    return std::string("[") + sm.getPolicy().getStateName(sm.getCurrentState()) + "]";
}

}  // namespace

TEST(EventDataArrivesAsSentAotTest, AHostsJsonPayloadIsAddressableAndItsTextStaysText) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();

    ASSERT_TRUE(isActive(sm, SM::State::Waiting))
        << "the fixture is supposed to start in `waiting`, so nothing below is testing what it "
           "claims. active: "
        << describe(sm);

    // A JSON object, the shape an embedder has when it holds structured data
    // and a state machine to give it to.
    sm.processEvent(SM::Event::Payload, SCE::Core::EventMetadata("payload", R"({"milestone":"refined","turns":2})"));

    EXPECT_FALSE(isActive(sm, SM::State::Mangled))
        << "the host sent a JSON object and the guard `_event.data.milestone === 'refined' && "
           "_event.data.turns === 2` did not hold, so the payload did not arrive as an object "
           "with those properties. active: "
        << describe(sm);
    ASSERT_TRUE(isActive(sm, SM::State::Heard))
        << "the payload guard neither matched nor mismatched — the machine is not in `heard`. "
           "active: "
        << describe(sm);

    // Text that is not JSON. The same call, and it must NOT be parsed into
    // something else: `hold the line` is the value the document compares
    // against, character for character.
    sm.processEvent(SM::Event::Note, SCE::Core::EventMetadata("note", "hold the line"));

    EXPECT_FALSE(isActive(sm, SM::State::Garbled))
        << "the host sent the text `hold the line` and `_event.data === 'hold the line'` did not "
           "hold, so a payload that is not JSON did not arrive as the string it was sent as. "
           "active: "
        << describe(sm);

    // Text that happens to be a valid expression. §scxml-B-2-8-1 gives the
    // payload three readings and none of them is "evaluate it": a payload is
    // what a host, a peer session or an HTTP sender put there, and running it
    // makes `_event.data` mean whatever the receiver's engine is written in.
    sm.processEvent(SM::Event::Arith, SCE::Core::EventMetadata("arith", "2 + 3"));

    EXPECT_FALSE(isActive(sm, SM::State::Evaluated))
        << "the host sent the text `2 + 3` and it arrived as 5 — the payload was run rather than "
           "read. active: "
        << describe(sm);
    EXPECT_TRUE(isActive(sm, SM::State::Settled))
        << "the arithmetic-shaped payload neither matched nor mismatched. active: " << describe(sm);
}

}  // namespace SCE::Tests
