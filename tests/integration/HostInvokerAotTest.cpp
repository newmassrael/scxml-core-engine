// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.1 — C++ AOT compile+run gate for an `<invoke type>` the HOST runs.
//
// The clause leaves the invokable set to the platform in the same words
// §scxml-6.2.5 uses for `<send>`, so the set is open by design. SCE implemented
// the SCXML processor and refused everything else with `error.execution`. The
// send half of that gap was repaid across six backends; this one stayed
// Rust-only, and the generator refused `--host-invoker` for C++ by name rather
// than emit a start nothing could service.
//
// The refusal was honest, which is what made it a coverage debt rather than a
// silent drop. Now the C++ AOT engine carries the registry
// (`StaticExecutionEngine::registerInvoker`) and this file is the channel that
// says so.
//
// An invoke is not a send: it has a LIFETIME. The scenarios below hold the
// outcomes apart, because the configuration alone cannot:
//
//   * a registered invoker is STARTED with what the document wrote;
//   * leaving the state CANCELS it — the half no configuration assertion can
//     see, because the machine looks correct whether or not the host was told
//     to stop;
//   * a cancel is delivered once, and only for an invocation that started;
//   * a declared type with nothing registered raises `error.execution`.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml
// (shared with the Rust and Go channels; `tests/CMakeLists.txt` compiles it
// here with the same `--host-invoker` declaration
// `scripts/regen_host_processor.sh` passes there).

#include "statechart_host_invoker_sm.h"

#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "core/HostProcessor.h"
#include "scripting/JSEngine.h"
#include "scripting/ScriptEngineProvider.h"

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::statechart_host_invoker::statechart_host_invoker;
using Event = SCE::Generated::statechart_host_invoker::Event;

/// The type the fixture was compiled for. `tests/CMakeLists.txt` passes this
/// same string to `--host-invoker`; a test registering a different one would
/// measure nothing and pass, so the `refused` counter is asserted rather than
/// the registration trusted.
constexpr const char *DECLARED_TYPE = "x-sce-host";

class HostInvokerAotTest : public ::testing::Test {
protected:
    void SetUp() override {
        SCE::JSEngine::instance().initialize();
    }

    void TearDown() override {
        SCE::JSEngine::instance().shutdown();
    }

    /// What the invoker saw, in call order, so the ORDER of start and cancel is
    /// assertable rather than only their arrival.
    std::vector<std::string> log;

    /// A recording invoker. Answers a completion on start so the
    /// `done.invoke` path is exercised too.
    void registerRecordingInvoker(Machine &sm) {
        sm.registerInvoker(DECLARED_TYPE, [this](const SCE::HostInvokeEvent &ev) {
            if (ev.start.has_value()) {
                std::string within = "absent";
                const auto it = ev.start->params.find("within");
                if (it != ev.start->params.end() && !it->second.empty()) {
                    within = it->second.front();
                }
                log.push_back("START id=" + ev.start->invokeId + " type=" + ev.start->processorType +
                              " src=" + ev.start->src + " within=" + within);
                SCE::HostInvokeResponse response;
                response.doneData = "ok";
                return std::optional<SCE::HostInvokeResponse>(response);
            }
            if (ev.cancel.has_value()) {
                log.push_back("CANCEL id=" + ev.cancel->invokeId);
            }
            return std::optional<SCE::HostInvokeResponse>();
        });
    }

    /// A machine whose datamodel can evaluate. Registration happens BEFORE
    /// this: the fixture's invoke runs at the end of the entry macrostep, so an
    /// invoker registered afterwards would be measuring a run that had already
    /// refused.
    // Every start the running invoker saw: (invokeId, token), in order.
    std::vector<std::pair<std::string, uint64_t>> starts;

    // An invoker whose work outlives the call: it answers nothing on start, so
    // each invocation stays running until the test completes or cancels it.
    // Records the lines the recording invoker does and every start's token for
    // a later completeHostInvoke().
    void registerRunningInvoker(Machine &sm) {
        sm.registerInvoker(DECLARED_TYPE, [this](const SCE::HostInvokeEvent &ev) {
            if (ev.start.has_value()) {
                log.push_back("START id=" + ev.start->invokeId);
                starts.emplace_back(ev.start->invokeId, ev.start->token);
            }
            if (ev.cancel.has_value()) {
                log.push_back("CANCEL id=" + ev.cancel->invokeId);
            }
            return std::optional<SCE::HostInvokeResponse>();
        });
    }

    // The token of the latest start of `invokeId`.
    uint64_t tokenOf(const std::string &invokeId) const {
        for (auto it = starts.rbegin(); it != starts.rend(); ++it) {
            if (it->first == invokeId) {
                return it->second;
            }
        }
        ADD_FAILURE() << "`" << invokeId << "` never started";
        return 0;
    }

    std::vector<std::string> cancels() const {
        std::vector<std::string> out;
        for (const auto &entry : log) {
            if (entry.rfind("CANCEL", 0) == 0) {
                out.push_back(entry);
            }
        }
        return out;
    }

    static void boot(Machine &sm) {
        sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                               [](SCE::IScriptEngine *) {}));
        sm.initialize();
        sm.step();
    }
};

}  // namespace

TEST_F(HostInvokerAotTest, ARegisteredInvokerIsStartedWithWhatTheDocumentWrote) {
    Machine sm;
    registerRecordingInvoker(sm);
    boot(sm);

    // The fixture counts a completion only when its `_event.invokeid` names the
    // invocation (§scxml-5.10.1), so each counter is also that assertion.
    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(1))
        << "done.invoke.probe never reached the document, or arrived without its invokeid";
    EXPECT_EQ(sm.getPolicy().started2(), std::optional<int64_t>(1))
        << "done.invoke.probe2 never reached the document, or arrived without its invokeid";
    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(0))
        << "a started invocation also raised error.execution";
    // The false-positive guard: ordinary entry content must still run. Without
    // it a change that broke the entry chain while leaving the invoke arm
    // working would read as a pass.
    EXPECT_EQ(sm.getPolicy().entered(), std::optional<int64_t>(1)) << "the entry chain stopped running";

    ASSERT_EQ(log.size(), 2u) << "invoker calls: " << log.size();
    // `src` and `<param>` are how §scxml-6.4.1 lets the document say WHAT to
    // invoke and with what. A request carrying neither would let a document
    // name an invocation it cannot describe. Each invocation is started as
    // itself: `probe2` begins with `probe`, and a dispatch that matched by
    // substring started `probe` twice.
    EXPECT_EQ(log[0], std::string("START id=probe type=") + DECLARED_TYPE + " src=pane://turn within=2500")
        << "the start request lost part of what the document wrote";
    EXPECT_EQ(log[1], std::string("START id=probe2 type=") + DECLARED_TYPE + " src=pane://other within=absent")
        << "the second invocation was not started as itself";
}

// The invocation ends with the state that started it. Without this the host is
// told to begin work and never told to stop — which no configuration assertion
// can detect, because the machine looks correct either way.
// W3C SCXML 6.4.1: `srcexpr`, `namelist`, `<param expr>` and `<content expr>`
// are read from the data model when the invocation starts. The request used
// to carry the literal params alone, so a document that computed what to
// invoke handed the host an empty description.
TEST_F(HostInvokerAotTest, WhatTheRequestSaysIsEvaluatedWhenTheInvocationStarts) {
    Machine sm;
    std::vector<SCE::HostInvokeRequest> starts;
    sm.registerInvoker(DECLARED_TYPE, [&starts](const SCE::HostInvokeEvent &ev) {
        if (ev.start.has_value()) {
            starts.push_back(*ev.start);
        }
        return std::optional<SCE::HostInvokeResponse>();
    });
    boot(sm);
    starts.clear();  // `probe` / `probe2`, which the case above already reads
    sm.processEvent(Event::Evaluate);

    // `req3`'s srcexpr cannot be evaluated, so it is never started.
    ASSERT_EQ(starts.size(), 2u) << "started " << starts.size() << " invocations";
    EXPECT_EQ(starts[0].invokeId, "req");
    EXPECT_EQ(starts[1].invokeId, "req2");
    EXPECT_EQ(starts[0].src, "pane://dyn") << "srcexpr was not evaluated";
    // A repeated name keeps both values in document order; the `<param>` that
    // failed is absent (W3C SCXML 5.7.1) while the invocation still started.
    const std::map<std::string, std::vector<std::string>> expected{{"n", {"7"}}, {"twice", {"a", "8"}}};
    EXPECT_EQ(starts[0].params, expected) << "params";
    EXPECT_EQ(starts[1].content, "body:7") << "content";
    // One error.execution for the dropped `<param>`, one for `req3`.
    EXPECT_EQ(sm.getPolicy().dropped(), std::optional<int64_t>(2)) << "a failed evaluation was not reported";
}

TEST_F(HostInvokerAotTest, LeavingTheStateCancelsTheInvocation) {
    Machine sm;
    // Still running when the state exits — a completed invocation has nothing
    // left to cancel (ACompletedInvocationIsNotCancelled).
    registerRunningInvoker(sm);
    boot(sm);
    sm.processEvent(Event::Leave);

    EXPECT_EQ(sm.getPolicy().ended(), std::optional<int64_t>(1)) << "the machine never left the invoking state";
    // Both invocations end with the state, each told once.
    int probeCancels = 0;
    int probe2Cancels = 0;
    for (const auto &entry : log) {
        probeCancels += entry == "CANCEL id=probe" ? 1 : 0;
        probe2Cancels += entry == "CANCEL id=probe2" ? 1 : 0;
    }
    EXPECT_EQ(probeCancels, 1) << "cancel for probe reached the invoker " << probeCancels << " times";
    EXPECT_EQ(probe2Cancels, 1) << "cancel for probe2 reached the invoker " << probe2Cancels << " times";
}

// A cancel is delivered once, and only for an invocation that started.
//
// The engine, not the emitted code, owns that judgement: the exit chain calls
// cancelHostInvoke unconditionally, so if the engine did not track what
// started, a state that exits before its macrostep settles would have the host
// tearing down work it never began.
//
// Asserted at the engine surface rather than through the fixture, for the
// reason the Rust channel records: driving the machine cannot produce the
// "never started" case, because every host call that advances it runs a
// macrostep and the pending invoke executes at the end of that macrostep.
TEST_F(HostInvokerAotTest, CancelIsNotDeliveredForAnInvocationThatNeverStarted) {
    Machine sm;
    registerRunningInvoker(sm);

    EXPECT_FALSE(sm.cancelHostInvoke(DECLARED_TYPE, "probe"))
        << "a cancel was reported for an invocation that never started";
    EXPECT_TRUE(log.empty()) << "the invoker was called for an invocation that never started";

    // Now let one start, cancel it, and cancel again: the second call has
    // nothing left to do. A registry that answered twice would have the host
    // tear down the same work twice.
    boot(sm);
    EXPECT_TRUE(sm.cancelHostInvoke(DECLARED_TYPE, "probe")) << "a started invocation reported nothing to cancel";
    EXPECT_FALSE(sm.cancelHostInvoke(DECLARED_TYPE, "probe")) << "the same invocation was cancelled twice";

    int cancels = 0;
    for (const auto &entry : log) {
        if (entry.rfind("CANCEL", 0) == 0) {
            cancels++;
        }
    }
    EXPECT_EQ(cancels, 1) << "cancel reached the invoker " << cancels << " times";
}

// The other half. The build declared the type, so codegen emitted a start —
// but nothing was registered, so no process was run. Same event as an
// unsupported type, because from the document's side it is the same fact.
//
// This is the scenario that keeps the repair honest: without it the feature
// could start nothing and the document would proceed as though its process
// were running.
// W3C SCXML 6.4: `done.invoke` says the invoked process is over, so leaving
// the state afterwards has nothing to stop. Both invocations here complete
// synchronously; neither is cancelled.
TEST_F(HostInvokerAotTest, ACompletedInvocationIsNotCancelled) {
    Machine sm;
    registerRecordingInvoker(sm);
    boot(sm);
    sm.processEvent(Event::Leave);

    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(1));
    EXPECT_TRUE(cancels().empty()) << "a completed invocation was cancelled";
}

// A host that finishes later reports it with the start's token, and the
// completion is taken once: a second report of the same run finds nothing,
// and the state's exit then cancels only the invocation still running.
TEST_F(HostInvokerAotTest, ALateCompletionIsAcceptedExactlyOnce) {
    Machine sm;
    registerRunningInvoker(sm);
    boot(sm);
    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(0));

    const uint64_t token = tokenOf("probe");
    EXPECT_TRUE(sm.completeHostInvoke(DECLARED_TYPE, "probe", token, "ok"))
        << "a running invocation's completion was refused";
    sm.step();
    // The fixture counts it only when `_event.invokeid` names the invocation
    // (W3C SCXML 5.10.1), so this is that assertion too.
    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(1));
    EXPECT_FALSE(sm.completeHostInvoke(DECLARED_TYPE, "probe", token, "again")) << "the same run completed twice";
    sm.step();
    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(1));

    sm.processEvent(Event::Leave);
    EXPECT_EQ(cancels(), std::vector<std::string>{"CANCEL id=probe2"})
        << "only the invocation still running is cancelled";
}

// W3C SCXML 6.4: once the state has exited, what the cancelled process sends
// is ignored. The host's reply arrives after the cancel and is refused.
TEST_F(HostInvokerAotTest, ACompletionAfterTheCancelIsRefused) {
    Machine sm;
    registerRunningInvoker(sm);
    boot(sm);
    const uint64_t token = tokenOf("probe");
    sm.processEvent(Event::Leave);

    EXPECT_FALSE(sm.completeHostInvoke(DECLARED_TYPE, "probe", token, "late"))
        << "a cancelled run's completion was accepted";
    sm.step();
    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(0));
}

// Re-entering the state starts the same `<invoke>` again under the same id.
// The first run's late reply carries the first start's token and is refused;
// the second run's is accepted.
TEST_F(HostInvokerAotTest, ARestartedInvokeRefusesTheFirstRunsReply) {
    Machine sm;
    registerRunningInvoker(sm);
    boot(sm);
    const uint64_t first = tokenOf("probe");
    sm.processEvent(Event::Leave);
    sm.processEvent(Event::Again);
    const uint64_t second = tokenOf("probe");
    ASSERT_NE(first, second) << "a restart reused the first start's token";

    EXPECT_FALSE(sm.completeHostInvoke(DECLARED_TYPE, "probe", first, "stale"))
        << "the first run's reply was taken for the second run's";
    sm.step();
    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(0));
    EXPECT_TRUE(sm.completeHostInvoke(DECLARED_TYPE, "probe", second, "ok"));
    sm.step();
    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(1));
}

// A host-run invocation's `done.invoke` raised through the ordinary
// external-event API skipped the running check, so the engine refuses it and
// counts the refusal. The metadata names the invocation, so without the
// refusal the fixture's guarded transition would take it.
TEST_F(HostInvokerAotTest, ADoneInvokeRaisedTheOldWayIsRefusedAndCounted) {
    Machine sm;
    registerRunningInvoker(sm);
    boot(sm);

    const auto doneEvent = sm.getPolicy().getEventFromName("done.invoke.probe");
    ASSERT_TRUE(doneEvent.has_value()) << "the fixture declares done.invoke.probe";
    sm.raiseExternal(Machine::EventWithMetadata(*doneEvent, "x", "", "", "external", "", "probe"));
    sm.step();

    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(0))
        << "a completion that skipped the running check reached the document";
    EXPECT_EQ(sm.refusedHostInvokeCompletions(), 1u);
}

TEST_F(HostInvokerAotTest, ADeclaredTypeWithNoInvokerStillRaisesErrorExecution) {
    Machine sm;
    boot(sm);

    // One error.execution per invocation nobody ran.
    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(2))
        << "an unregistered invoker was silently treated as started";
    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(0))
        << "done.invoke arrived for an invocation nobody ran";
}

// Registering some other type does not run this one. The registry is keyed, and
// a lookup that fell back to "any invoker" would hand a document's process to
// one it never named.
TEST_F(HostInvokerAotTest, AnInvokerRegisteredForAnotherTypeDoesNotRunThisOne) {
    Machine sm;
    sm.registerInvoker("x-some-other-host", [this](const SCE::HostInvokeEvent &) {
        log.push_back("WRONG");
        return std::optional<SCE::HostInvokeResponse>();
    });
    boot(sm);

    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(0)) << "an invoker for a different type ran this one";
    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(2)) << "the unregistered type was not reported";
    EXPECT_TRUE(log.empty()) << "the other type's invoker was called";
}

}  // namespace SCE::Tests
