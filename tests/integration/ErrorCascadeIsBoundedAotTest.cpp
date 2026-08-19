// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2 says an error event nothing matches is ignored. It says
// nothing about an error event something DOES match, answered by a handler
// that fails the same way every time: the failure raises `error.execution`,
// the same transition answers it, and the drain never empties. C++ AOT path.
//
// That is not a hang, which is what makes it worth an accessor. Measured
// 2026-08-19 on the Python engine and a two-line document: 37,000 links a
// second, configuration unmoved, `isRunning()` true — the reading an
// unattended supervisor takes as a healthy idle machine while a core is
// pinned. `UnhandledErrorIsObservableAotTest.cpp` owns the error nobody
// answered; this owns the error answered by a handler that cannot handle it.
//
// The fixture separates a chain that STOPS by itself (`settle`, three links,
// then its guard stops matching) from one that cannot (`spin`). Both are runs
// of errors, and only the second is a defect — a ceiling that could not tell
// them apart would report every document that fails often as broken.
//
// Fixture: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml
// (canonical, shared with the Interpreter / C11 / Rust / Go / Kotlin / Python channels).
//
// Regeneration: the generated header is built by CMake via
//   sce_generate_static_integration_test(error_cascade_is_bounded ...)

#include "error_cascade_is_bounded_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::error_cascade_is_bounded::error_cascade_is_bounded;

/// The ceiling the engine applies, spelled here rather than read back from it.
/// A test that asked the engine for its own limit would agree with any limit,
/// including one an edit moved by three orders of magnitude — and the number is
/// exactly what this fixture exists to pin.
constexpr int64_t MAX_LINKS = 100;

std::unique_ptr<SM> started() {
    auto sm = std::make_unique<SM>();
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                  [](::SCE::IScriptEngine *) {}));
    }
    sm->initialize();
    return sm;
}

}  // namespace

/// The axis: a handler that answers its own failure with the same failure is
/// stopped, and the host is told.
///
/// This test returning at all is half the assertion. Before the ceiling existed
/// it did not: the same call ran until the harness was killed.
TEST(ErrorCascadeIsBoundedAotTest, AHandlerThatCannotHandleItsErrorIsStopped) {
    auto sm = started();
    ASSERT_EQ(sm->errorCascadeEvents(), 0u) << "nothing has been refused before the machine has done anything";

    sm->processEvent(SM::Event::Spin);

    EXPECT_EQ(sm->getPolicy().runs().value_or(-1), MAX_LINKS)
        << "`runaway`'s handler must run exactly as many times as the engine allows links in a chain — fewer "
           "means the document was cut off early, more means the ceiling moved";
    EXPECT_EQ(sm->getPolicy().ticks().value_or(-1), MAX_LINKS)
        << "every link's handler also raises the author's own `tick`, and every one of them must be delivered. An "
           "engine that counted those as links would refuse at half the depth; one that let them end the chain "
           "would never refuse at all — and a handler that logs before it fails is an ordinary document";
    EXPECT_EQ(sm->errorCascadeEvents(), 1u)
        << "the handler's <assign> failed again on the last allowed link, and the error it raised is the one the "
           "engine refused to queue. Without that count the host sees a machine that is running, in a plausible "
           "state, with nothing to say about the core it is burning";
    ASSERT_TRUE(sm->lastErrorCascadeEvent().has_value()) << "the engine counted a refusal but reports no event to name";
    EXPECT_EQ(sm->lastErrorCascadeEvent().value(), SM::Event::Error_execution)
        << "a count alone does not name the repair: error.execution is a handler whose own content fails, "
           "error.communication one that answers an unreachable target by talking to it again";
    EXPECT_TRUE(sm->isRunning()) << "the chain was cut, not the machine";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Runaway)
        << "the handler is targetless, so nothing here may move the machine";
}

/// The other half, and the one that makes the count mean something: a chain
/// that ends by itself must pass through untouched.
TEST(ErrorCascadeIsBoundedAotTest, AChainThatEndsOnItsOwnIsNotRefused) {
    auto sm = started();

    sm->processEvent(SM::Event::Settle);

    EXPECT_EQ(sm->getPolicy().repairs().value_or(-1), 3)
        << "`settling`'s handler repairs three times and then its `repairs < 3` guard stops matching. Three links "
           "is what a real repair strategy looks like, and the engine must not have interrupted it";
    EXPECT_EQ(sm->errorCascadeEvents(), 0u)
        << "nothing was refused: the chain ended on the document's own terms. A ceiling that fired here would "
           "report every document that fails often as one that cannot stop failing";
    EXPECT_FALSE(sm->lastErrorCascadeEvent().has_value()) << "nothing was refused, so there is no last one to name";
    EXPECT_EQ(sm->unhandledErrorEvents(), 1u)
        << "the fourth error found no matching transition once the guard closed, which is the ordinary clause — "
           "the two counts answer different questions and this document produces exactly one of each";
}

/// A single failure with nobody to answer it is not a chain. The chain is
/// measured handler-to-handler, not failure-to-failure.
TEST(ErrorCascadeIsBoundedAotTest, OneErrorNobodyAnsweredIsNotAChain) {
    auto sm = started();

    for (int i = 0; i < 5; ++i) {
        sm->processEvent(SM::Event::Boom);
    }

    EXPECT_EQ(sm->unhandledErrorEvents(), 5u) << "five failures, none of them answered — the clause's own case";
    EXPECT_EQ(sm->errorCascadeEvents(), 0u)
        << "no handler ran, so no handler raised anything: a count keyed off how OFTEN a document fails would "
           "already be at five here";
}

/// The machine is still a machine afterwards. Cutting the chain must not cost
/// the document the states that work.
TEST(ErrorCascadeIsBoundedAotTest, TheMachineStillAnswersAfterItsChainIsCut) {
    auto sm = started();

    sm->processEvent(SM::Event::Spin);
    ASSERT_EQ(sm->errorCascadeEvents(), 1u) << "precondition: this test is about what happens AFTER a refusal";

    sm->processEvent(SM::Event::Poke);

    EXPECT_EQ(sm->getPolicy().pokes().value_or(-1), 1)
        << "`runaway` answers `poke` with a targetless transition, and it ran — an engine that stopped the machine "
           "to end the chain would leave the host with a dead document instead of a bounded one";
    EXPECT_EQ(sm->errorCascadeEvents(), 1u)
        << "`poke` raises nothing, so the count that was already there is all there is: the refusal is a fact "
           "about the past, not a mode";
}

/// A second chain starts from zero. The depth is a property of the chain, not
/// of the machine's whole life.
TEST(ErrorCascadeIsBoundedAotTest, ASecondChainStartsFromZero) {
    auto sm = started();

    sm->processEvent(SM::Event::Spin);
    sm->processEvent(SM::Event::Reset);
    ASSERT_EQ(sm->getCurrentState(), SM::State::Idle) << "`reset` is the fixture's way back out of the chain";

    sm->processEvent(SM::Event::Spin);

    EXPECT_EQ(sm->getPolicy().runs().value_or(-1), 2 * MAX_LINKS)
        << "the second entry into `runaway` must buy the document a full chain again. A depth carried across the "
           "drains would stop this one at its first link and leave the counter at "
        << MAX_LINKS;
    EXPECT_EQ(sm->errorCascadeEvents(), 2u)
        << "two chains, two refusals — a count that saturates at one would read as a machine that recovered";
}

}  // namespace SCE::Tests
