// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0096 — C++ AOT path: a generated machine whose script session
// cannot be established does not start, and its host is told why.
//
// The script engine is a double over the build's own engine (the one
// `ScriptEngineProvider` names, which every other AOT test of this fixture runs
// on). It fails, on request, one of the three steps that establish a session —
// creating it, loading the shared library into it, binding the system
// variables — and counts every call that runs a document's expressions, so "no
// executable content ran" is read from the engine the machine runs its content
// in. The parent hands the child the engine it was given, so the child's
// session is asked of the same double.
//
// Fixture:
// integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml,
// reused for its shape: a parent in `s0` with `<data ticks>` that invokes an
// inline child, whose `<onentry>` sends `tick` to the parent 110 times. The
// child declares `<data>` of its own, so it has a script session of its own to
// fail (a child without one never asks for a session). A start that ran would
// initialize `ticks`, enter `s0` and start the child; a child that ran would
// send `tick`.
//
// A machine that never started still answers `getActiveStates()` with the
// initial configuration its constructor set, so "no state was entered" is read
// from what entering would have done: content run, the child asked for, the
// run marked running.
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_child_may_send_many_events_in_one_tick ...)`.

#include "a_child_may_send_many_events_in_one_tick_sm.h"
#include "common/NoParent.h"
#include "common/ScriptSessionFailure.h"
#include "core/ConfigurationHelper.h"
#include "scripting/IScriptEngine.h"
#include "scripting/ScriptEngineProvider.h"

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <future>
#include <gtest/gtest.h>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_child_may_send_many_events_in_one_tick::a_child_may_send_many_events_in_one_tick;

/// Which step of establishing a session the double fails.
enum class Fail {
    /// None: the double is the build's engine, counted.
    Never,
    /// Every session's creation fails as the engine's own failure.
    Create,
    /// Every session's creation fails as a shared library that did not load.
    Library,
    /// Every bind of the system variables fails.
    SystemVariables,
    /// The first session asked for, the parent's, is created; every later
    /// one, a child's, fails to be.
    CreateAfterFirst,
};

const char *nameOf(Fail fail) {
    switch (fail) {
    case Fail::Never:
        return "never";
    case Fail::Create:
        return "create";
    case Fail::Library:
        return "library";
    case Fail::SystemVariables:
        return "system-variables";
    case Fail::CreateAfterFirst:
        return "create-after-first";
    }
    return "";
}

/// The build's engine, with one step of establishing a session failing on
/// request and every call that runs the document's expressions counted.
class FailingEngine final : public ::SCE::IScriptEngine {
public:
    explicit FailingEngine(Fail fail) : inner_(::SCE::ScriptEngineProvider::getScriptEngine()), fail_(fail) {}

    /// Calls that ran a document's expressions or wrote its datamodel.
    std::size_t ran() const {
        return ran_.load();
    }

    /// Every session id `createSessionChecked` was asked for, in order.
    std::vector<std::string> asked() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return asked_;
    }

    /// Every session id `destroySession` was asked for, in order.
    std::vector<std::string> destroyed() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return destroyed_;
    }

    // ── The steps that establish a session ──────────────────────────────

    std::optional<::SCE::ScriptSessionFailure> createSessionChecked(const std::string &sessionId,
                                                                    const std::string &parentSessionId = "") override {
        bool first = false;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            asked_.push_back(sessionId);
            first = asked_.size() == 1;
        }
        switch (fail_) {
        case Fail::Create:
            return ::SCE::ScriptSessionFailure{::SCE::ScriptSessionStage::Create, "the double creates no session"};
        case Fail::Library:
            return ::SCE::ScriptSessionFailure{::SCE::ScriptSessionStage::Library, "the double loads no library"};
        case Fail::CreateAfterFirst:
            if (!first) {
                return ::SCE::ScriptSessionFailure{::SCE::ScriptSessionStage::Create,
                                                   "the double creates no second session"};
            }
            break;
        case Fail::Never:
        case Fail::SystemVariables:
            break;
        }
        return inner_.createSessionChecked(sessionId, parentSessionId);
    }

    std::future<::SCE::ScriptResult>
    setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                         const std::vector<::SCE::IOProcessorDescriptor> &ioProcessors) override {
        if (fail_ == Fail::SystemVariables) {
            std::promise<::SCE::ScriptResult> refused;
            refused.set_value(::SCE::ScriptResult::createError("the double binds no system variable"));
            return refused.get_future();
        }
        return inner_.setupSystemVariables(sessionId, sessionName, ioProcessors);
    }

    bool destroySession(const std::string &sessionId) override {
        {
            std::lock_guard<std::mutex> lock(mutex_);
            destroyed_.push_back(sessionId);
        }
        return inner_.destroySession(sessionId);
    }

    bool hasSession(const std::string &sessionId) const override {
        return inner_.hasSession(sessionId);
    }

    // ── What runs a document's expressions, counted ────────────────────

    std::future<::SCE::ScriptResult> setVariable(const std::string &sessionId, const std::string &name,
                                                 const ::ScriptValue &value) override {
        ++ran_;
        return inner_.setVariable(sessionId, name, value);
    }

    std::future<::SCE::ScriptResult> setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                                      const std::string &xmlContent) override {
        ++ran_;
        return inner_.setVariableAsDOM(sessionId, name, xmlContent);
    }

    std::future<::SCE::SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                              const std::shared_ptr<::SCE::Event> &event) override {
        ++ran_;
        return inner_.setCurrentEvent(sessionId, event);
    }

    std::future<::SCE::SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                              const ::SCE::SetCurrentEventArgs &args) override {
        ++ran_;
        return inner_.setCurrentEvent(sessionId, args);
    }

    // ── The rest, as the build's engine answers it ─────────────────────

    ::SCE::ScriptLanguage nativeLanguage() const override {
        return inner_.nativeLanguage();
    }

    bool acceptsLanguage(::SCE::ScriptLanguage language) const override {
        return inner_.acceptsLanguage(language);
    }

    std::future<::SCE::ScriptResult> getVariable(const std::string &sessionId, const std::string &name) override {
        return inner_.getVariable(sessionId, name);
    }

    bool hasVariable(const std::string &sessionId, const std::string &variableName) const override {
        return inner_.hasVariable(sessionId, variableName);
    }

    bool isVariablePreInitialized(const std::string &sessionId, const std::string &variableName) const override {
        return inner_.isVariablePreInitialized(sessionId, variableName);
    }

    bool registerGlobalFunction(const std::string &functionName,
                                std::function<::ScriptValue(const std::vector<::ScriptValue> &)> callback) override {
        return inner_.registerGlobalFunction(functionName, std::move(callback));
    }

    bool bindNativeObject(const std::string &sessionId, const std::string &objectName,
                          const std::vector<std::pair<std::string, NativeMethod>> &methods) override {
        return inner_.bindNativeObject(sessionId, objectName, methods);
    }

    std::string getEngineInfo() const override {
        return inner_.getEngineInfo();
    }

    size_t getMemoryUsage() const override {
        return inner_.getMemoryUsage();
    }

    void collectGarbage() override {
        inner_.collectGarbage();
    }

    void setStateQueryCallback(StateQueryCallback callback, const std::string &sessionId) override {
        inner_.setStateQueryCallback(std::move(callback), sessionId);
    }

    bool initialize() override {
        return inner_.initialize();
    }

    bool isInitialized() const override {
        return inner_.isInitialized();
    }

    // The build's engine is the test program's, shared by every test in it:
    // the double does not stop or clear it.
    void shutdown() override {}

    void reset() override {}

protected:
    std::future<::SCE::ScriptResult> doExecuteScript(const std::string &sessionId,
                                                     const ::SCE::ScriptSource &script) override {
        ++ran_;
        return inner_.executeScript(sessionId, script);
    }

    std::future<::SCE::ScriptResult> doEvaluateExpression(const std::string &sessionId,
                                                          const ::SCE::ScriptSource &expression) override {
        ++ran_;
        return inner_.evaluateExpression(sessionId, expression);
    }

    std::future<::SCE::ScriptResult> doValidateExpression(const std::string &sessionId,
                                                          const ::SCE::ScriptSource &expression) override {
        return inner_.validateExpression(sessionId, expression);
    }

    std::future<::SCE::ScriptResult> doAssign(const std::string &sessionId, const ::SCE::ScriptSource &location,
                                              const ::SCE::ScriptSource &expr) override {
        ++ran_;
        return inner_.assign(sessionId, location, expr);
    }

private:
    ::SCE::IScriptEngine &inner_;
    const Fail fail_;
    std::atomic<std::size_t> ran_{0};
    mutable std::mutex mutex_;
    std::vector<std::string> asked_;
    std::vector<std::string> destroyed_;
};

/// The fixture's machine over a double that fails as `fail` says, and the
/// double, to read afterwards.
struct Run {
    explicit Run(Fail failing) : fail(failing), engine(std::make_shared<FailingEngine>(failing)) {
        sm.setScriptEngine(engine);
    }

    /// The machine's own session id.
    std::string session() {
        return sm.getPolicy().sessionIdText();
    }

    /// A refused start left nothing behind: not running, no final state, no
    /// content run, no child asked for, no session kept.
    void expectNothingStarted() {
        EXPECT_FALSE(sm.isRunning()) << nameOf(fail) << ": a refused start leaves the machine not running";
        EXPECT_EQ(sm.terminalState(), std::nullopt) << nameOf(fail) << ": a refused start reaches no final state";
        EXPECT_EQ(engine->ran(), 0u) << nameOf(fail)
                                     << ": a refused start evaluates nothing: no <data>, no <onentry>, no cond";
        for (const std::string &asked : engine->asked()) {
            EXPECT_EQ(asked, session()) << nameOf(fail) << ": a refused start enters no state, so starts no child";
        }
        EXPECT_FALSE(engine->hasSession(session())) << nameOf(fail) << ": a refused start keeps no session";
    }

    Fail fail;
    std::shared_ptr<FailingEngine> engine;
    SM sm;
};

}  // namespace

// The control: the double fails nothing, so the machine is the fixture's, and
// its child's ticks arrive. Without it the cases below could pass over a double
// that breaks every machine.
TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest, AStartOverTheDoubleThatFailsNothingRunsTheFixture) {
    Run run(Fail::Never);
    ASSERT_EQ(run.sm.initialize(), std::nullopt) << "a session that is established starts the machine";
    run.sm.processEvent(SM::Event::Finish);
    EXPECT_EQ(run.sm.terminalState(), std::optional<SM::State>(SM::State::Done));
    EXPECT_EQ(run.sm.ticks(), std::optional<int64_t>(110));
    EXPECT_GT(run.engine->ran(), 0u);
}

TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest, AStartWhoseSessionCannotBeEstablishedIsRefusedWithTheStep) {
    for (const auto &[fail, stage] : {std::pair{Fail::Create, ::SCE::ScriptSessionStage::Create},
                                      std::pair{Fail::Library, ::SCE::ScriptSessionStage::Library},
                                      std::pair{Fail::SystemVariables, ::SCE::ScriptSessionStage::SystemVariables}}) {
        Run run(fail);
        const auto refused = run.sm.initialize();
        ASSERT_TRUE(refused.has_value()) << nameOf(fail) << ": a session that cannot be established refuses the start";
        EXPECT_EQ(refused->stage, stage) << nameOf(fail) << ": the refusal names the step";
        EXPECT_FALSE(refused->message.empty()) << nameOf(fail) << ": the refusal carries the engine's words";
        run.expectNothingStarted();
    }
}

// A session that failed is not marked established: the next start asks for it
// again and is refused again, rather than finding it "done" and running the
// machine over no session.
TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest, ARefusedStartIsRefusedAgainRatherThanRunWithoutASession) {
    for (const Fail fail : {Fail::Create, Fail::Library, Fail::SystemVariables}) {
        Run run(fail);
        EXPECT_TRUE(run.sm.initialize().has_value()) << nameOf(fail);
        EXPECT_TRUE(run.sm.initialize().has_value())
            << nameOf(fail) << ": a second start over the same failing engine is refused too";
        EXPECT_EQ(run.engine->asked().size(), 2u) << nameOf(fail) << ": each start asks for the session once";
        run.expectNothingStarted();
    }
}

TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest, ASessionWhoseSystemVariablesCannotBeBoundIsReleased) {
    Run run(Fail::SystemVariables);
    EXPECT_TRUE(run.sm.initialize().has_value());
    const std::vector<std::string> only{run.session()};
    EXPECT_EQ(run.engine->asked(), only);
    EXPECT_EQ(run.engine->destroyed(), only) << "the session the failed bind was made in is destroyed";
}

TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest, ARefusedMachineTakesNoEvent) {
    Run run(Fail::Create);
    EXPECT_TRUE(run.sm.initialize().has_value());
    run.sm.processEvent(SM::Event::Finish);
    EXPECT_EQ(run.sm.unseenExternalEvents(), 1u)
        << "the event is refused as one posted to a machine that is not running";
    run.expectNothingStarted();
}

TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest, TheCheckedRootStartIsRefusedForTheSession) {
    Run run(Fail::Library);
    const auto refusal = run.sm.initializeAsRoot();
    ASSERT_TRUE(refusal.has_value()) << "the root start of a machine whose session fails is refused";
    EXPECT_EQ(refusal->kind(), ::SCE::Common::RootStartRefusal::Kind::ScriptSession);
    ASSERT_NE(refusal->scriptSessionFailure(), nullptr) << "a refusal for the session carries the failure";
    EXPECT_EQ(refusal->scriptSessionFailure()->stage, ::SCE::ScriptSessionStage::Library);
    EXPECT_STREQ(::SCE::Common::reason(*refusal), "the machine's script session could not be established");
    run.expectNothingStarted();
}

TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest, AResumedStartWhoseSessionCannotBeEstablishedIsRefused) {
    Run run(Fail::Create);
    const auto verdict = run.sm.enterAt({SM::State::S0}, SM::State::S0);
    EXPECT_EQ(verdict, ::SCE::Core::ConfigurationRejection::ScriptSessionUnavailable);
    ASSERT_TRUE(verdict.scriptSessionFailure.has_value()) << "the entry's refusal carries the failure";
    EXPECT_EQ(verdict.scriptSessionFailure->stage, ::SCE::ScriptSessionStage::Create);
    run.expectNothingStarted();
}

TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest,
     AChildWhoseSessionFailsLeavesItsParentWithErrorExecutionAndNoChild) {
    Run run(Fail::CreateAfterFirst);
    ASSERT_EQ(run.sm.initialize(), std::nullopt) << "the parent's own session is established";
    run.sm.step();

    const std::vector<std::string> asked = run.engine->asked();
    ASSERT_EQ(asked.size(), 2u) << "the parent asked for its own session and the child's";
    EXPECT_FALSE(run.engine->hasSession(asked[1])) << "the child whose session failed keeps none";
    EXPECT_FALSE(run.sm.getPolicy().isChildSessionRunning(asked[1]))
        << "the child whose start was refused is not kept as a session of the parent";
    // §scxml-6.4: the invoke ends without a child, with error.execution, which
    // no transition of `s0` takes.
    EXPECT_EQ(run.sm.lastUnhandledError(), std::optional<SM::Event>(SM::Event::Error_execution));
    EXPECT_EQ(run.sm.unhandledErrorEvents(), 1u);
    // No child ran, so no `tick` and no `done.invoke` ever arrived.
    EXPECT_EQ(run.sm.ticks(), std::optional<int64_t>(0));
    EXPECT_EQ(run.sm.discardedExternalEvents(), 0u);

    // The parent itself runs on.
    EXPECT_TRUE(run.sm.isRunning());
    EXPECT_EQ(run.sm.getCurrentState(), SM::State::S0);
    run.sm.processEvent(SM::Event::Finish);
    EXPECT_EQ(run.sm.terminalState(), std::optional<SM::State>(SM::State::Done));
}

// The failure's words: the step, named as every backend names it, and the
// engine's message.
TEST(AMachineWhoseScriptSessionFailsDoesNotStartAotTest, TheFailureNamesItsStepAndTheEnginesWords) {
    EXPECT_STREQ(::SCE::scriptSessionStageName(::SCE::ScriptSessionStage::Create), "create");
    EXPECT_STREQ(::SCE::scriptSessionStageName(::SCE::ScriptSessionStage::Library), "library");
    EXPECT_STREQ(::SCE::scriptSessionStageName(::SCE::ScriptSessionStage::SystemVariables), "system-variables");
    const ::SCE::ScriptSessionFailure failure{::SCE::ScriptSessionStage::Create, "out of memory"};
    EXPECT_EQ(failure.describe(), "the script session could not be established at step 'create': out of memory");
}

}  // namespace SCE::Tests
