// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.2.5 — C++ AOT compile+run gate for a `<send type>` the HOST serves.
//
// §scxml-6.2.5 makes the Event I/O Processor identifier extensible, so the set
// is open by design. SCE implemented two of them and refused everything else
// with `error.execution`. The Rust runtime grew a registry first, because the
// report came from a Rust consumer — and the build then REFUSED the same
// declaration for every other backend, C++ included, rather than emitting a
// dispatch with nowhere to land.
//
// That refusal was honest and it was also the whole gap. The worked example in
// `examples/ai_loop/` is driven by a C++ host and emits seven targetless
// `<send>`s whose events no transition takes: measured 2026-08-23, they land on
// the machine's own external queue and are counted as discarded, one per turn,
// while the host re-derives every one of them by polling the configuration. A
// document cannot declare an act its own engine has no door for.
//
// So this file is the C++ half of the pair `backends/rust/tests/tests/
// host_processor.rs` asks, against the same fixture and the same declared type:
//
//   * a registered handler receives the send, with the payload the author
//     wrote, and its reply arrives as an event — the feature working;
//   * the same machine with nothing registered raises `error.execution` — a
//     wiring mistake staying visible instead of reading as success.
//
// Both are needed. A gate holding only the first would pass on an engine that
// dispatched into an empty registry and called it delivered, which is exactly
// the silence being repaid.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml
// (shared with the Rust channel; `tests/CMakeLists.txt` compiles it here with
// the same `--host-processor` declaration `scripts/regen_host_processor.sh`
// passes there).

#include "statechart_host_processor_sm.h"

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

using Machine = SCE::Generated::statechart_host_processor::statechart_host_processor;

/// The type the fixture was compiled for. `tests/CMakeLists.txt` passes this
/// same string to `--host-processor`; a test that registered a different one
/// would measure nothing and pass, so the two spellings are asserted to be one
/// by the `refused` counter below rather than trusted.
constexpr const char *DECLARED_TYPE = "x-sce-host";

/// The fixture's `<assign>`s are the only witness: every outcome here leaves
/// the machine in the same single state, so the configuration cannot tell them
/// apart.
class HostProcessorAotTest : public ::testing::Test {
protected:
    void SetUp() override {
        SCE::JSEngine::instance().initialize();
    }

    void TearDown() override {
        SCE::JSEngine::instance().shutdown();
    }

    /// A machine whose datamodel can evaluate. Aliasing constructor with a
    /// no-op deleter — the provider owns the engine's lifetime and this
    /// `shared_ptr` is a non-owning view, the same idiom the rest of the AOT
    /// suite uses.
    ///
    /// Registration happens BEFORE this: the fixture's send fires on entry to
    /// its initial state, so a handler registered afterwards would be measuring
    /// a run that had already refused.
    static void boot(Machine &sm) {
        sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                               [](SCE::IScriptEngine *) {}));
        sm.initialize();
        sm.step();
    }
};

}  // namespace

TEST_F(HostProcessorAotTest, ARegisteredHandlerReceivesTheSendAndItsReplyArrives) {
    std::vector<SCE::HostSendRequest> seen;

    Machine sm;
    sm.registerEventProcessor(DECLARED_TYPE, [&seen](const SCE::HostSendRequest &req) {
        seen.push_back(req);
        // The request/reply shape: the reply becomes an event the document was
        // already waiting for, which is what lets a state DECLARE an act
        // instead of a host-side table performing it.
        return std::optional<SCE::HostSendResponse>(SCE::HostSendResponse{"turn.done", ""});
    });
    boot(sm);

    EXPECT_EQ(sm.getPolicy().served(), std::optional<int64_t>(1)) << "the handler's reply never reached the document";
    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(0)) << "a served send also raised error.execution";
    // The false-positive guard: an ordinary `<send>` in the same block must
    // still deliver. Without it a change that broke every send while leaving
    // the host branch intact would read as a pass.
    EXPECT_EQ(sm.getPolicy().plain(), std::optional<int64_t>(1))
        << "an ordinary <send> in the same block stopped delivering";

    ASSERT_EQ(seen.size(), 1u) << "the handler ran " << seen.size() << " times";
    EXPECT_EQ(seen[0].processorType, DECLARED_TYPE);
    EXPECT_EQ(seen[0].eventName, "watch.turn");
    // The payload the author wrote has to survive the crossing, or the document
    // can name an act but not parameterise it — which is most of the reason to
    // move an act into the document at all.
    ASSERT_EQ(seen[0].params.count("within"), 1u) << "the <param> did not reach the handler";
    EXPECT_EQ(seen[0].params.at("within"), std::vector<std::string>{"2500"});
    // §scxml-6.2.4: correlating a reply, or honouring a `<cancel>`, needs the
    // send id — auto-generated here because the fixture declares none.
    EXPECT_FALSE(seen[0].sendId.empty()) << "the request carried no send id";
}

// A handler may perform work and have nothing to say. That is not an error, and
// must not be reported as one — otherwise every fire-and-forget act costs the
// document a spurious `error.execution`.
TEST_F(HostProcessorAotTest, AHandlerThatAnswersNothingIsNotAnError) {
    Machine sm;
    sm.registerEventProcessor(DECLARED_TYPE,
                              [](const SCE::HostSendRequest &) { return std::optional<SCE::HostSendResponse>(); });
    boot(sm);

    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(0))
        << "a silent handler was reported as an unsupported processor";
    EXPECT_EQ(sm.getPolicy().served(), std::optional<int64_t>(0))
        << "no reply was sent, so no reply event should have arrived";
}

// The other half. The build declared the type, so codegen emitted a dispatch —
// but nothing was registered, so nobody performed the act. From the document's
// side that is indistinguishable from a processor the platform does not
// implement, and it gets the same event.
//
// This is the test that keeps the repair honest: without it the feature could
// dispatch into an empty registry and the document would proceed as though its
// act had been carried out.
TEST_F(HostProcessorAotTest, ADeclaredTypeWithNoHandlerStillRaisesErrorExecution) {
    Machine sm;
    boot(sm);

    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(1))
        << "an unregistered processor was silently treated as served";
    EXPECT_EQ(sm.getPolicy().served(), std::optional<int64_t>(0));
}

// Registering some other type does not serve this one. The registry is keyed,
// and a lookup that fell back to "any handler" would deliver a document's acts
// to a processor it never named.
TEST_F(HostProcessorAotTest, AHandlerRegisteredForAnotherTypeDoesNotServeThisOne) {
    Machine sm;
    sm.registerEventProcessor("x-some-other-host", [](const SCE::HostSendRequest &) {
        return std::optional<SCE::HostSendResponse>(SCE::HostSendResponse{"turn.done", ""});
    });
    boot(sm);

    EXPECT_EQ(sm.getPolicy().refused(), std::optional<int64_t>(1))
        << "a handler registered for another type served this one";
    EXPECT_EQ(sm.getPolicy().served(), std::optional<int64_t>(0));
}

}  // namespace SCE::Tests
