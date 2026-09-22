// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// A host guard deciding a transition, in code the generator emits.
//
// Fixture: tests/integration/test_thermostat.scxml, compiled for C++ by
// `tests/CMakeLists.txt`. The guard is a native `cond="cpp:…"` on an
// `<sce:context>` object and every effect is an `<sce:action>`, so the machine
// needs no script engine and the host is the only thing that decides or acts.
//
// ⚠ This test used to include a header kept by hand, written for an API the
// generator no longer emits (`ThermostatBase<Derived>`, reached through
// `derived()`), while the document beside it called functions it never
// declared. The build compiled the header and never ran the generator, so the
// workflow this file names was not the one it exercised. Nothing else runs a
// native guard: `examples/smart_light` is the only other document writing one,
// and it is generated, not executed.
//
// What the cases measure:
//
//   * the guard is asked on the event that names it, and its answer alone
//     decides whether the machine moves;
//   * W3C SCXML 3.13: a transition's effects run in document order — the
//     source's `<onexit>`, the transition's own content, the target's
//     `<onentry>` (Appendix D `microstep`);
//   * an event no transition names asks no guard and runs no effect.

#include "test_thermostat_sm.h"

#include <gtest/gtest.h>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

namespace G = SCE::Generated::test_thermostat;

/// One record for both halves of the host, so a case can assert the order in
/// which the machine asked the guard and ran the effects.
using Log = std::vector<std::string>;

/// The `<sce:context id="climate">` object the native guard reads.
struct Climate {
    Log *log = nullptr;
    bool coolDecision = true;

    bool shouldCool() {
        log->push_back("shouldCool");
        return coolDecision;
    }
};

using Machine = G::test_thermostat<Climate>;

/// Host implementation of the generated operations.
class Host : public G::TestThermostatActions {
public:
    explicit Host(Log &log) : log_(log) {}

    void onEnterIdle() override {
        log_.push_back("onEnterIdle");
    }

    void onEnterCooling() override {
        log_.push_back("onEnterCooling");
    }

    void onExitCooling() override {
        log_.push_back("onExitCooling");
    }

    void startCooling() override {
        log_.push_back("startCooling");
    }

    void stopCooling() override {
        log_.push_back("stopCooling");
    }

private:
    Log &log_;
};

class StaticCodegenIntegrationTest : public ::testing::Test {
protected:
    Log log;
    Climate climate{&log};
    Host host{log};
    // The host and the context are CONSTRUCTOR arguments: `idle`'s `<onentry>`
    // acts on the initial entry, so either one installed afterwards would
    // arrive one act too late.
    Machine sm{host, climate};

    void send(G::Event event) {
        sm.raiseExternal(event);
        sm.step();
    }
};

}  // namespace

TEST_F(StaticCodegenIntegrationTest, InitialEntryRunsTheInitialStatesOnentry) {
    sm.initialize();

    EXPECT_EQ(log, (Log{"onEnterIdle"}));
    EXPECT_EQ(sm.getCurrentState(), G::State::Idle);
}

TEST_F(StaticCodegenIntegrationTest, AGuardThatHoldsTakesTheTransition) {
    sm.initialize();
    log.clear();
    climate.coolDecision = true;

    send(G::Event::Temp_high);

    EXPECT_EQ(log, (Log{"shouldCool", "startCooling", "onEnterCooling"}));
    EXPECT_EQ(sm.getCurrentState(), G::State::Cooling);
}

TEST_F(StaticCodegenIntegrationTest, AGuardThatFailsLeavesTheMachineWhereItWas) {
    sm.initialize();
    log.clear();
    climate.coolDecision = false;

    send(G::Event::Temp_high);

    EXPECT_EQ(log, (Log{"shouldCool"})) << "only the guard may run when it refuses the transition";
    EXPECT_EQ(sm.getCurrentState(), G::State::Idle);
}

TEST_F(StaticCodegenIntegrationTest, EffectsRunExitThenTransitionThenEntry) {
    sm.initialize();
    climate.coolDecision = true;
    send(G::Event::Temp_high);
    log.clear();

    send(G::Event::Temp_normal);

    EXPECT_EQ(log, (Log{"onExitCooling", "stopCooling", "onEnterIdle"}));
    EXPECT_EQ(sm.getCurrentState(), G::State::Idle);
}

TEST_F(StaticCodegenIntegrationTest, AnEventNoTransitionNamesAsksNothing) {
    sm.initialize();
    log.clear();

    send(G::Event::Temp_normal);

    EXPECT_TRUE(log.empty()) << "an event idle has no transition for ran " << log.size() << " host call(s)";
    EXPECT_EQ(sm.getCurrentState(), G::State::Idle);
}

}  // namespace SCE::Tests
