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

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <fstream>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <tuple>
#include <vector>

#include "common/SceClock.h"
#include "core/HostProcessor.h"

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT must name the checkout: the deadline table is read from it"
#endif
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

// W3C SCXML 6.4.1: an invoke with no id and an `idlocation` gets a generated
// `stateid.platformid` id, written to the location, handed to the host, and
// carried as `_event.invokeid` on the completion. The document names no
// specific `done.invoke.<id>`, so the completion arrives as the generic
// `done.invoke`, and `matched` counts it only when its invokeid is what the
// document stored.
TEST_F(HostInvokerAotTest, AnIdlocationHoldsTheIdTheHostIsHanded) {
    Machine sm;
    registerRecordingInvoker(sm);
    boot(sm);
    sm.processEvent(Event::Leave);

    bool handed = false;
    for (const auto &entry : log) {
        handed = handed || entry.rfind("START id=done._invoke_0 ", 0) == 0;
    }
    EXPECT_TRUE(handed) << "the host was not handed the generated id";
    EXPECT_EQ(sm.getPolicy().matched(), std::optional<int64_t>(1))
        << "the completion did not arrive, or its invokeid is not what idlocation holds";
}

// §scxml-6.2.4 / §scxml-6.4.1: an `idlocation` is a location expression, so the
// id is written the way `<assign>` writes (§scxml-5.4): `slot.id` and
// `slot.sid` are member paths, and land. `n.nope.deeper` cannot take a value,
// so each element raises error.execution and is abandoned (§scxml-5.9.2) — the
// host is never asked to start that invoke, and that message is never sent.
TEST_F(HostInvokerAotTest, AnIdlocationIsAssignedLikeALocation) {
    Machine sm;
    registerRecordingInvoker(sm);
    boot(sm);
    sm.processEvent(Event::Locate);
    sm.step();

    bool handed = false;
    for (const auto &entry : log) {
        handed = handed || entry.rfind("START id=locating._invoke_1 ", 0) == 0;
        EXPECT_EQ(entry.find("locating._invoke_2"), std::string::npos)
            << "an invoke whose idlocation could not take the id was started: " << entry;
    }
    EXPECT_TRUE(handed) << "the member-path invoke was not started";

    const auto raw = sm.getPolicy().slot();
    ASSERT_TRUE(raw.has_value()) << "the fixture declares `slot` as an object";
    const auto slot = nlohmann::json::parse(*raw);
    EXPECT_EQ(slot.at("id").get<std::string>(), "locating._invoke_1") << *raw;
    EXPECT_FALSE(slot.at("sid").get<std::string>().empty()) << "slot.sid did not receive the send id: " << *raw;

    EXPECT_EQ(sm.getPolicy().slotted(), std::optional<int64_t>(1));
    EXPECT_EQ(sm.getPolicy().pinged(), std::optional<int64_t>(1));
    EXPECT_EQ(sm.getPolicy().leaked(), std::optional<int64_t>(0))
        << "a send whose idlocation could not take the id was still sent";
    EXPECT_EQ(sm.getPolicy().lost(), std::optional<int64_t>(2));
}

// The generic `done.invoke` is a host completion too when its invokeid names a
// host-run invoke, so raised around completeHostInvoke() it is refused like
// the specific name is.
TEST_F(HostInvokerAotTest, AGenericDoneInvokeRaisedTheOldWayIsRefused) {
    Machine sm;
    registerRunningInvoker(sm);
    boot(sm);
    sm.processEvent(Event::Leave);

    const auto doneEvent = sm.getPolicy().getEventFromName("done.invoke");
    ASSERT_TRUE(doneEvent.has_value()) << "a host-invoker build declares the generic done.invoke";
    sm.raiseExternal(Machine::EventWithMetadata(*doneEvent, "x", "", "", "external", "", "done._invoke_0"));
    sm.step();

    EXPECT_EQ(sm.getPolicy().matched(), std::optional<int64_t>(0))
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

// `perm` completed with `doneData` in a machine driven into `typed`: the
// counters `granted`, `denied` and `unreadable`.
TEST_F(HostInvokerAotTest, ATypedCompletionIsReadAsItsRecord) {
    // SCE Accepted Subset §2.12: `sce:result` makes `perm`'s completion a
    // `PermResult` record, so its guards read `granted` as a typed field —
    // true and false each select their own transition — and a completion
    // whose data is not that record is refused as any typed payload the data
    // does not fit is: error.execution, and neither guard fires.
    const auto complete = [this](const std::string &doneData) {
        Machine sm;
        starts.clear();
        registerRunningInvoker(sm);
        boot(sm);
        sm.processEvent(Event::Type);
        EXPECT_TRUE(sm.completeHostInvoke(DECLARED_TYPE, "perm", tokenOf("perm"), doneData))
            << "a running invocation's completion was refused";
        sm.step();
        return std::vector<std::optional<int64_t>>{sm.getPolicy().granted(), sm.getPolicy().denied(),
                                                   sm.getPolicy().unreadable()};
    };
    using Counts = std::vector<std::optional<int64_t>>;
    EXPECT_EQ(complete(R"({"granted":true})"), (Counts{1, 0, 0})) << "granted";
    EXPECT_EQ(complete(R"({"granted":false})"), (Counts{0, 1, 0})) << "denied";
    EXPECT_EQ(complete("yes"), (Counts{0, 0, 1})) << "not the record";
}

namespace {

/// A machine on a ManualClock whose invoker answers nothing and records, per
/// start, whether the request still carried the deadline param, driven into
/// `timed`. The clock is installed before initialize(), which arms against it.
struct Timed {
    std::shared_ptr<SCE::ManualClock> clock = std::make_shared<SCE::ManualClock>(0);
    std::vector<std::string> log;
    std::vector<std::pair<std::string, uint64_t>> starts;
    Machine sm;

    Timed() {
        sm.registerInvoker(DECLARED_TYPE, [this](const SCE::HostInvokeEvent &ev) {
            if (ev.start.has_value()) {
                const bool carried = ev.start->params.count(std::string(SCE::HOST_INVOKE_DEADLINE_PARAM)) > 0;
                log.push_back("START id=" + ev.start->invokeId + " deadline-param=" + (carried ? "true" : "false"));
                starts.emplace_back(ev.start->invokeId, ev.start->token);
            }
            if (ev.cancel.has_value()) {
                log.push_back("CANCEL id=" + ev.cancel->invokeId);
            }
            return std::optional<SCE::HostInvokeResponse>();
        });
        sm.setClock(clock);
        sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                               [](SCE::IScriptEngine *) {}));
        sm.initialize();
        sm.step();
        sm.processEvent(Event::Time);
    }

    bool logged(const std::string &line) const {
        return std::find(log.begin(), log.end(), line) != log.end();
    }

    uint64_t tokenOf(const std::string &invokeId) const {
        for (auto it = starts.rbegin(); it != starts.rend(); ++it) {
            if (it->first == invokeId) {
                return it->second;
            }
        }
        ADD_FAILURE() << "`" << invokeId << "` never started";
        return 0;
    }
};

}  // namespace

// A deadline that passes while the invocation is still running ends it: the
// host is told to stop, the document receives `error.invoke.slow` with
// `_event.data` "deadline", and a reply afterwards is refused. The param is the
// engine's — the host never sees it.
TEST_F(HostInvokerAotTest, ADeadlineThatPassesEndsTheInvocation) {
    Timed t;
    EXPECT_TRUE(t.logged("START id=slow deadline-param=false"))
        << "the host was handed the deadline param, or `slow` never started";

    // A `<cancel>` of the empty send id must not reach the deadline.
    t.sm.processEvent(Event::Forget);
    t.sm.advanceTimeMs(49);
    EXPECT_EQ(t.sm.getPolicy().expired(), std::optional<int64_t>(0)) << "expired early";
    t.sm.advanceTimeMs(1);
    EXPECT_EQ(t.sm.getPolicy().expired(), std::optional<int64_t>(1));
    EXPECT_TRUE(t.logged("CANCEL id=slow")) << "the host was not told to stop";

    EXPECT_FALSE(t.sm.completeHostInvoke(DECLARED_TYPE, "slow", t.tokenOf("slow"), "late"))
        << "a reply after the deadline was accepted";
    t.sm.step();
    EXPECT_EQ(t.sm.getPolicy().finished(), std::optional<int64_t>(0));
}

// The discriminator: a completion before the deadline is the outcome, and the
// deadline that comes due afterwards does nothing — no cancel, no
// `error.invoke`, and nothing left for the host to tick toward.
TEST_F(HostInvokerAotTest, ACompletionBeforeTheDeadlineDisarmsIt) {
    Timed t;
    EXPECT_TRUE(t.sm.completeHostInvoke(DECLARED_TYPE, "slow", t.tokenOf("slow"), "ok"));
    t.sm.step();
    EXPECT_EQ(t.sm.getPolicy().finished(), std::optional<int64_t>(1));
    EXPECT_FALSE(t.sm.timeUntilNextScheduled().has_value())
        << "the disarmed deadline is still keeping the host ticking";

    t.sm.advanceTimeMs(100);
    EXPECT_EQ(t.sm.getPolicy().expired(), std::optional<int64_t>(0));
    EXPECT_FALSE(t.logged("CANCEL id=slow")) << "a completed invocation was cancelled by its deadline";
}

// §scxml-6.4.1: a deadline that is not a whole number of milliseconds is an
// argument that cannot be evaluated — error.execution, and the host is never
// asked to start the invocation.
TEST_F(HostInvokerAotTest, ADeadlineThatIsNotMillisecondsStartsNothing) {
    Timed t;
    EXPECT_EQ(t.sm.getPolicy().misdated(), std::optional<int64_t>(1));
    for (const auto &line : t.log) {
        EXPECT_NE(line.rfind("START id=undated", 0), 0u) << "an invocation with an unreadable deadline was started";
    }
}

// Every runtime reads a deadline's text by one grammar, held to one table.
// strtoull would not do: it skips leading whitespace and reads a sign, and a
// deadline this backend honours while another refuses it makes a document
// depend on where it was compiled.
TEST_F(HostInvokerAotTest, ADeadlineIsReadByTheSharedTable) {
    const std::string path =
        std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json";
    std::ifstream in(path);
    ASSERT_TRUE(in.is_open()) << "the shared table is not readable: " << path;
    const auto table = nlohmann::json::parse(in);
    const auto &accepted = table.at("accepted");
    const auto &refused = table.at("refused");
    // A floor: an empty table would pass every assertion below.
    ASSERT_FALSE(accepted.empty() || refused.empty()) << "the table is empty";
    for (const auto &pair : accepted) {
        const auto written = pair.at(0).get<std::string>();
        const auto ms = std::stoull(pair.at(1).get<std::string>());
        EXPECT_EQ(SCE::parseHostInvokeDeadlineMs(written), std::optional<uint64_t>(ms)) << '"' << written << '"';
    }
    for (const auto &value : refused) {
        const auto written = value.get<std::string>();
        EXPECT_FALSE(SCE::parseHostInvokeDeadlineMs(written).has_value()) << '"' << written << "\" was accepted";
    }
}

namespace {

using PermRequest = SCE::Generated::statechart_host_invoker::PermRequest;
using PermResult = SCE::Generated::statechart_host_invoker::PermResult;

/// A host implementing the generated interface; its work outlives the call.
struct PermHost : SCE::Generated::statechart_host_invoker::XSceHostInvoker {
    std::vector<std::pair<PermRequest, uint64_t>> starts;
    std::vector<uint64_t> cancels;

    std::optional<PermResult> startPerm(const PermRequest &request, uint64_t token) override {
        starts.emplace_back(request, token);
        return std::nullopt;
    }

    void cancelPerm(uint64_t token) override {
        cancels.push_back(token);
    }
};

}  // namespace

// SCE Accepted Subset §2.12: through the generated interface a host is handed
// `perm`'s request as its `PermRequest` record — the datamodel's values at
// their declared types — and completes it with a `PermResult`, which the
// document reads as that record. An invoke of the same type the document does
// not type still reaches the host, through the fallback.
TEST_F(HostInvokerAotTest, ATypedRequestReachesItsInvokerAsItsRecord) {
    Machine sm;
    auto host = std::make_shared<PermHost>();
    sm.registerXSceHostInvoker(host, [this](const SCE::HostInvokeEvent &ev) {
        if (ev.start.has_value()) {
            log.push_back("START id=" + ev.start->invokeId);
        }
        return std::optional<SCE::HostInvokeResponse>();
    });
    boot(sm);
    sm.processEvent(Event::Type);
    ASSERT_EQ(host->starts.size(), 1U) << "perm started once";
    PermRequest expected;
    expected.scope = "calendar";
    expected.level = 2;
    EXPECT_EQ(host->starts[0].first, expected);
    EXPECT_NE(std::find(log.begin(), log.end(), "START id=probe"), log.end())
        << "the untyped `probe` never reached the fallback";
    const uint64_t token = host->starts[0].second;
    PermResult granted;
    granted.granted = true;
    EXPECT_TRUE(sm.completePerm(token, granted));
    sm.step();
    EXPECT_EQ(sm.getPolicy().granted(), std::optional<int64_t>(1));
    EXPECT_EQ(sm.getPolicy().unreadable(), std::optional<int64_t>(0));
    // A completion is accepted once: the token now names nothing running.
    EXPECT_FALSE(sm.completePerm(token, granted));
}

// SCE Accepted Subset §2.12, W3C SCXML 6.4.1: a request value its record's
// field cannot hold is an argument that cannot be evaluated. `retype` sets
// `level` to a text and re-enters `typed`: the running start is cancelled, and
// the new one raises error.execution and is never handed to the host.
TEST_F(HostInvokerAotTest, ARequestThatDoesNotFitItsRecordStartsNothing) {
    Machine sm;
    auto host = std::make_shared<PermHost>();
    sm.registerXSceHostInvoker(host,
                               [](const SCE::HostInvokeEvent &) { return std::optional<SCE::HostInvokeResponse>(); });
    boot(sm);
    sm.processEvent(Event::Type);
    ASSERT_EQ(host->starts.size(), 1U);
    const uint64_t first = host->starts[0].second;
    sm.processEvent(Event::Retype);
    EXPECT_EQ(host->cancels, std::vector<uint64_t>{first}) << "the running start was cancelled";
    EXPECT_EQ(host->starts.size(), 1U) << "the misfit request was started";
    EXPECT_EQ(sm.getPolicy().unreadable(), std::optional<int64_t>(1));
}

// The start site's check and the adapter's reading are one rule: every value
// the check accepts is spelled as text its field's type parses back to the
// same value, and a value the field cannot hold is refused rather than
// narrowed.
TEST_F(HostInvokerAotTest, ARequestFieldIsCheckedAndReadBackByOneRule) {
    const auto fractional = [](double d) {
        char buffer[32];
        std::snprintf(buffer, sizeof(buffer), "%.17g", d);
        return std::string(buffer);
    };
    const auto type = [](SCE::RequestFieldKind kind, std::size_t cap = 0) { return SCE::RequestFieldType{kind, cap}; };
    const auto roundTrip = [&](const ::ScriptValue &value, SCE::RequestFieldType ty) {
        std::string refusal;
        const auto text = SCE::requestFieldWire(value, "f", ty, fractional, refusal);
        EXPECT_TRUE(text.has_value()) << refusal;
        SCE::HostInvokeRequest request;
        request.invokeId = "perm";
        request.params["f"] = {text.value_or("")};
        return request;
    };
    using K = SCE::RequestFieldKind;
    EXPECT_EQ(SCE::requestField<uint8_t>(roundTrip(int64_t{255}, type(K::Uint8)), "f"), 255);
    EXPECT_EQ(SCE::requestField<int16_t>(roundTrip(-3.0, type(K::Int16)), "f"), -3);
    EXPECT_EQ(SCE::requestField<double>(roundTrip(0.1, type(K::Float64)), "f"), 0.1);
    EXPECT_EQ(SCE::requestField<float>(roundTrip(0.1, type(K::Float32)), "f"), 0.1f);
    EXPECT_EQ(SCE::requestField<bool>(roundTrip(false, type(K::Bool)), "f"), false);
    EXPECT_EQ(SCE::requestField<std::string>(roundTrip(std::string("a b"), type(K::String)), "f"), "a b");
    EXPECT_EQ(SCE::requestField<uint64_t>(roundTrip(18446744073709549568.0, type(K::Uint64)), "f"),
              18446744073709549568ULL);

    const std::vector<std::tuple<::ScriptValue, SCE::RequestFieldType, const char *>> refused = {
        {int64_t{256}, type(K::Uint8), "past the width"},
        {int64_t{-1}, type(K::Uint32), "below zero"},
        {1.5, type(K::Int32), "not whole"},
        {18446744073709551616.0, type(K::Uint64), "past every width"},
        {std::nan(""), type(K::Float64), "not finite"},
        {1e39, type(K::Float32), "past float"},
        {std::string("2"), type(K::Uint8), "a text"},
        {int64_t{1}, type(K::Bool), "a number"},
        {int64_t{1}, type(K::String), "a number"},
        {std::string("abc"), type(K::Bytes, 2), "past cap"},
        {std::string("\xC4\x80"), type(K::Bytes, 8), "no byte"},
    };
    for (const auto &[value, ty, why] : refused) {
        std::string refusal;
        EXPECT_FALSE(SCE::requestFieldWire(value, "f", ty, fractional, refusal).has_value()) << why;
    }
}

}  // namespace SCE::Tests
