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

    EXPECT_EQ(sm.getPolicy().started(), std::optional<int64_t>(1)) << "done.invoke never reached the document";
    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(0))
        << "a started invocation also raised error.execution";
    // The false-positive guard: ordinary entry content must still run. Without
    // it a change that broke the entry chain while leaving the invoke arm
    // working would read as a pass.
    EXPECT_EQ(sm.getPolicy().entered(), std::optional<int64_t>(1)) << "the entry chain stopped running";

    ASSERT_EQ(log.size(), 1u) << "invoker calls: " << log.size();
    // `src` and `<param>` are how §scxml-6.4.1 lets the document say WHAT to
    // invoke and with what. A request carrying neither would let a document
    // name an invocation it cannot describe.
    EXPECT_EQ(log[0], std::string("START id=probe type=") + DECLARED_TYPE + " src=pane://turn within=2500")
        << "the start request lost part of what the document wrote";
}

// The invocation ends with the state that started it. Without this the host is
// told to begin work and never told to stop — which no configuration assertion
// can detect, because the machine looks correct either way.
TEST_F(HostInvokerAotTest, LeavingTheStateCancelsTheInvocation) {
    Machine sm;
    registerRecordingInvoker(sm);
    boot(sm);
    sm.processEvent(Event::Leave);

    EXPECT_EQ(sm.getPolicy().ended(), std::optional<int64_t>(1)) << "the machine never left the invoking state";
    ASSERT_FALSE(log.empty());
    EXPECT_EQ(log.back(), "CANCEL id=probe") << "no cancel reached the invoker";
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
    registerRecordingInvoker(sm);

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
TEST_F(HostInvokerAotTest, ADeclaredTypeWithNoInvokerStillRaisesErrorExecution) {
    Machine sm;
    boot(sm);

    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(1))
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
    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(1)) << "the unregistered type was not reported";
    EXPECT_TRUE(log.empty()) << "the other type's invoker was called";
}

}  // namespace SCE::Tests
