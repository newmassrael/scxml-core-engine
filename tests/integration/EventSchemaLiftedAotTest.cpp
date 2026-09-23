// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A — the OTHER carrier, the C++ twin of the Rust
// `event_schema_native.rs` lifted cases, the Go `event_schema_lifted` package,
// the Kotlin `EventSchemaLiftedTest`, the Python `test_event_schema_lifted.py`
// and the C11 `c11_integration_event_schema_lifted` gate.
//
// A schema'd event's typed payload is filled by ONE producer: the generated
// `raiseJobCompleted` seam. Every other producer — `<send>` with `<param>`, an
// invoke forwarding an event either way, autoforward, BasicHTTP, mesh — fills
// `EventWithMetadata::data`, and until 2026-09-22 a natively lowered guard
// could read nothing but the typed carrier. The same guard therefore answered
// differently depending on where its event came from, and a typed payload
// could not cross an invoke boundary at all.
//
// ⚠ What a refusal does is not a policy chosen here: it is what the SCRIPT
// ENGINE answers for the same guard on the same data (W3C SCXML 3.13, measured
// on this document — no data, a missing field and a value of another type each
// give `error.execution` and a guard that does not fire). A native lowering
// that answered differently would make the optimisation observable, which is
// the one thing it may not be.
//
// Fixture: sce-build/tests/fixtures/event_schema/statechart_lifted.scxml, the
// same document the Rust, Go, Kotlin, Python and C11 channels drive
// (`tests/CMakeLists.txt` compiles it here).

#include "statechart_lifted_sm.h"

#include <gtest/gtest.h>
#include <string>

namespace SCE::Tests {

namespace {

using Machine = ::SCE::Generated::statechart_lifted::statechart_lifted;

// The event as every producer but the inject seam delivers it: its fields on
// the `data` wire, with no typed payload riding along.
void deliverData(Machine &sm, const std::string &data) {
    typename Machine::EventWithMetadata event(Machine::PolicyType::Event::Job_completed, data);
    sm.raiseExternal(event);
    sm.step();
}

}  // namespace

TEST(EventSchemaLiftedAotTest, APayloadOnTheDataWireFiresTheSameGuard) {
    Machine sm;
    sm.initialize();

    deliverData(sm, R"({"elapsed_ms": 0})");

    EXPECT_EQ(sm.getCurrentState(), Machine::State::Done)
        << "a payload that arrived on the `data` wire must satisfy the same "
           "native guard the inject seam's typed payload does";
}

TEST(EventSchemaLiftedAotTest, TheInjectSeamStillFiresItsOwnGuard) {
    Machine sm;
    sm.initialize();

    sm.raiseJobCompleted(0u);
    sm.step();

    EXPECT_EQ(sm.getCurrentState(), Machine::State::Done)
        << "the typed inject seam must still fire the guard it was built for";
}

TEST(EventSchemaLiftedAotTest, AValueOfAnotherTypeIsRefusedAsTheScriptEngineRefusesIt) {
    Machine sm;
    sm.initialize();

    deliverData(sm, R"({"elapsed_ms": "nought"})");

    EXPECT_EQ(sm.getCurrentState(), Machine::State::Refused)
        << "a text where the schema declares a number must raise error.execution "
           "and leave the guard unfired";
}

TEST(EventSchemaLiftedAotTest, AnEventWithNoDataIsRefusedTheSameWay) {
    Machine sm;
    sm.initialize();

    deliverData(sm, "");

    EXPECT_EQ(sm.getCurrentState(), Machine::State::Refused)
        << "an event carrying no data cannot answer a guard that reads a field "
           "of it";
}

TEST(EventSchemaLiftedAotTest, AFieldTheDataDoesNotNameIsRefused) {
    Machine sm;
    sm.initialize();

    deliverData(sm, R"({"other": 0})");

    EXPECT_EQ(sm.getCurrentState(), Machine::State::Refused)
        << "data that names none of the schema's fields cannot answer the guard";
}

TEST(EventSchemaLiftedAotTest, APayloadTheGuardRejectsIsNotAnError) {
    Machine sm;
    sm.initialize();

    // The payload reads perfectly; the comparison is simply false. Nothing
    // failed, so nothing is raised — the machine waits.
    deliverData(sm, R"({"elapsed_ms": 5})");

    EXPECT_EQ(sm.getCurrentState(), Machine::State::Waiting)
        << "a well-typed payload the guard rejects must not route the machine "
           "to the error handler";
}

}  // namespace SCE::Tests
