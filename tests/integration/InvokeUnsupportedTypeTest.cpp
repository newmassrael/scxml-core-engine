// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.1: an `<invoke>` naming a processor the platform does not
// support places `error.execution` on the internal event queue.
//
// The failure this pins is a *substitution*, not an omission. A factory that
// answers every unknown type with the SCXML handler starts a child session
// the author never asked for: the type value is discarded, the invoke either
// half-starts or fails on a missing `src`, and the author's
// `<transition event="error.execution">` never fires. Nothing in the machine
// distinguishes "the platform does not speak this protocol" from "the invoke
// ran" — the type attribute becomes decorative.
//
// SCE_MESH.md §9.5 leans on this observable from the other direction: it
// argues that a foreign SCXML 1.0 processor meeting SCE's own
// `<invoke type="sce:mesh-rpc">` degrades gracefully because §6.4.1 turns an
// unknown type into `error.execution` that the author already handles. That
// argument only holds if SCE itself honours the rule for types *it* does not
// know, which is what these two tests measure — the factory contract
// directly, and the event the machine actually observes end to end.
//
// Sibling of `InvokeUnsupportedTypeAotTest.cpp` (C++ AOT channel). Both
// engines ship in production, so each is held to the clause independently
// against one canonical fixture.
//
// Fixture: integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/InvokeExecutor.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>
#include <thread>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

// A type URI no SCE processor claims. Deliberately not a plausible typo of
// the SCXML URI so the test cannot pass by accident through prefix matching.
// Same value as the canonical fixture's `<invoke type>` — the factory-level
// boundary below and the end-to-end run must be asked about one type, or a
// refusal proven here would say nothing about the machine measured there.
constexpr const char *UNSUPPORTED_TYPE = "urn:example:no-such-processor";

}  // namespace

class InvokeUnsupportedTypeTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        if (engine_) {
            engine_->shutdown();
        }
    }

    IScriptEngine *engine_;
};

// §scxml-6.4.1: the supported-type set is closed, and a member outside it
// yields no handler at all rather than the SCXML one.
TEST_F(InvokeUnsupportedTypeTest, FactoryRefusesUnsupportedTypeInsteadOfSubstitutingScxml) {
    EXPECT_FALSE(InvokeHandlerFactory::isSupportedType(UNSUPPORTED_TYPE));
    EXPECT_EQ(InvokeHandlerFactory::createHandler(UNSUPPORTED_TYPE, *engine_), nullptr)
        << "an unsupported type must not resolve to the SCXML handler — substituting a different "
           "processor discards the author's `type` and hides the §6.4.1 error.execution";

    // The supported set still resolves, so the refusal above is a boundary,
    // not a blanket failure.
    EXPECT_TRUE(InvokeHandlerFactory::isSupportedType("scxml"));
    EXPECT_TRUE(InvokeHandlerFactory::isSupportedType("http://www.w3.org/TR/scxml/"));
    EXPECT_TRUE(InvokeHandlerFactory::isSupportedType("http://www.w3.org/TR/scxml"));
    EXPECT_NE(InvokeHandlerFactory::createHandler("scxml", *engine_), nullptr);
    EXPECT_NE(InvokeHandlerFactory::createHandler("http://www.w3.org/TR/scxml/", *engine_), nullptr);
}

// §scxml-6.4.1: end to end — the machine observes error.execution and the
// author's handler fires.
TEST_F(InvokeUnsupportedTypeTest, UnsupportedTypeRaisesErrorExecutionOnTheInternalQueue) {
    const std::string fixture =
        std::string(SCE_PROJECT_ROOT) + "/integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    const std::string scxml = buffer.str();
    ASSERT_NE(scxml.find(UNSUPPORTED_TYPE), std::string::npos)
        << "the canonical fixture no longer names " << UNSUPPORTED_TYPE
        << ", so the factory-level boundary asserted above and this end-to-end run are "
           "no longer asking about the same type";

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
            (void)event;
            try {
                return target->send(event).get().isSuccess;
            } catch (...) {
                return false;
            }
        });
    auto eventRaiser = std::make_shared<EventRaiserImpl>();
    eventRaiser->setScheduler(scheduler);
    eventRaiser->setImmediateMode(false);
    sm->setEventRaiser(eventRaiser);
    sm->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the machine stayed in `probe`, so `<invoke type=\"" << UNSUPPORTED_TYPE << "\">` produced no "
        << "error.execution. §scxml-6.4.1 requires an unsupported type to place error.execution on the "
        << "internal event queue; a processor that instead substitutes the SCXML handler leaves the "
        << "author's handler unreachable and the unsupported type indistinguishable from a working one.";
}

}  // namespace Tests
}  // namespace SCE
