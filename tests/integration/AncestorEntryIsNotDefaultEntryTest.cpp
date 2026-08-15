// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.3 + Appendix D: a compound state entered only because the target
// lies inside it does not take its default initial child — Interpreter path.
//
// The defect this axis exists for was found on the AOT channel, and pinning it
// here is not an assumption that the Interpreter shares it. Appendix D asks two
// different questions — `addDescendantStatesToEnter` for the target,
// `addAncestorStatesToEnter` for everything between it and the LCCA — and an
// engine that answers both with one function puts two children of one compound
// state in the configuration. Each engine computes that entry set through its
// own code, so an engine that is right today is not thereby asked less.
//
// Sibling of `AncestorEntryIsNotDefaultEntryAotTest.cpp` (C++ AOT).
//
// Fixture:
// integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <fstream>
#include <gtest/gtest.h>
#include <sstream>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class AncestorEntryIsNotDefaultEntryTest : public ::testing::Test {
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

TEST_F(AncestorEntryIsNotDefaultEntryTest, AnAncestorEnteredOnTheWayToATargetTakesNoDefaultChild) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/ancestor_entry_is_not_default_entry/"
                                "ancestor_entry_is_not_default_entry.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

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

    ASSERT_TRUE(sm->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm->start());

    // The configuration alone cannot tell the two failures apart on this
    // engine. A compound state that holds one child at a time will have
    // REPLACED the spurious entry with the target by the time anyone looks, so
    // `by_default` reads as inactive while its `<onentry>` has already run.
    // The counters are the only witness to that, which is why they are in the
    // failure message next to the configuration.
    const auto describe = [&sm]() {
        std::string out;
        for (const auto &s : sm->getActiveStates()) {
            out += " " + s;
        }
        auto &engine = ScriptEngineProvider::getScriptEngine();
        const auto read = [&](const char *name) {
            auto result = engine.evaluateExpression(sm->getSessionId(), name).get();
            return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
        };
        return out + "  (defaulted=" + read("defaulted") + " lobbied=" + read("lobbied") + " idled=" + read("idled") +
               " targeted=" + read("targeted") + ")";
    };

    ASSERT_TRUE(sm->isStateActive("away"))
        << "the run has to start OUTSIDE the `<parallel>` for the first pass to be testing "
           "anything — a source already inside it leaves the ancestors active and the entry "
           "chain never reaches their defaults. active:"
        << describe();

    // The raiser is in queued mode (see setImmediateMode(false) above), so each
    // event sits on the external queue until the drain runs — the same
    // macrostep boundary the engine uses for its own sends.
    //
    // Pass one: the parallel is not active, so `run` is entered as a parallel
    // ancestor and `drive` and `outer` as compound ones.
    ASSERT_TRUE(sm->raiseExternalEvent("cross", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("chosen"))
        << "the transition named `chosen` and the machine did not enter it. active:" << describe();
    EXPECT_FALSE(sm->isStateActive("by_default"))
        << "`outer` has two children active at once. `by_default` is what `initial` names, and "
           "nothing targeted it — it was entered because the engine gave `outer` its default "
           "child while entering `outer` merely as an ancestor of `chosen`. active:"
        << describe();
    EXPECT_TRUE(sm->isStateActive("idle"))
        << "the region no entering state is inside must still be entered with its default — "
           "Appendix D's one exception for a parallel ancestor. active:"
        << describe();

    // Pass two: the parallel is already active now, so `run` and `drive` are
    // skipped and only `outer` is entered. That is a different branch of the
    // entry walk, and it is the one a running machine takes.
    ASSERT_TRUE(sm->raiseExternalEvent("back", ""));
    eventRaiser->processQueuedEvents();
    ASSERT_TRUE(sm->raiseExternalEvent("again", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_FALSE(sm->isStateActive("by_default"))
        << "`outer` took its default child on the second pass, where the `<parallel>` was "
           "already active and only `outer` itself was entered — the shape the worked example "
           "hits every time a person answers a dialog. active:"
        << describe();

    ASSERT_TRUE(sm->raiseExternalEvent("check", ""));
    eventRaiser->processQueuedEvents();

    EXPECT_TRUE(sm->isStateActive("settled"))
        << "`check` did not carry the machine to `settled`. The document checks its four "
           "clauses in document order and lands each in a `<final>` of its own, so the "
           "configuration below names which one broke: `failDefaulted` (a default nobody "
           "targeted), `failLobbied` (`drive`'s default taken while it was only an ancestor), "
           "`failIdled` (the untouched region did not get its default, or got it twice), "
           "`failTargeted` (a pass never reached the target). active:"
        << describe();
}

}  // namespace Tests
}  // namespace SCE
