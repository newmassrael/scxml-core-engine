// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-G-7 `<sce:action>` — the Interpreter path.
//
// A generated machine names its host operations as an interface the host
// implements (NativeActionAotTest). The Interpreter loads a document at run
// time and has no interface to generate, so the host is `INativeActionHost`,
// installed on the machine before it starts.
//
// Before this existed, `ActionParser::parseActionNode` skipped an element that
// is not SCXML's (§scxml-4.10), and `<sce:action>` is SCE's, so a document that
// asked its host to do something had the request dropped with no event and no
// diagnostic — the one outcome that cannot be told from the host having done it.
// `cpp_interpreter_native_action_ctest.cases` puts that parser back and holds
// this test to turning red.
//
// What the cases measure:
//
//   * an action reaches the host by name, with each `<sce:arg>` computed as the
//     typed value it is (a number is an integer or a real, not a string);
//   * the action runs wherever executable content does — an `<onentry>`, which
//     `start()` performs, and inside an `<if>`;
//   * an action no one performs is `error.execution`: no host installed, a host
//     that provides no such operation, and an argument no host operation takes
//     — and in the last case the host is not called at all.

#include "actions/NativeAction.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/INativeActionHost.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <set>
#include <string>
#include <utility>
#include <vector>

namespace SCE {
namespace Tests {

namespace {

/// Every transition is targetless, so the machine stays in `idle` whatever
/// the action does and `error.execution` is what moves it to `failed`.
constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="idle">

  <datamodel>
    <data id="n" expr="2"/>
  </datamodel>

  <state id="idle">
    <onentry>
      <sce:action name="entered"/>
    </onentry>
    <transition event="go">
      <sce:action name="start">
        <sce:arg expr="n + 1"/>
        <sce:arg expr="'job'"/>
        <sce:arg expr="n === 2"/>
        <sce:arg expr="1.5"/>
      </sce:action>
    </transition>
    <transition event="guarded">
      <if cond="n === 2">
        <sce:action name="start"><sce:arg expr="n"/></sce:action>
      </if>
    </transition>
    <transition event="missing">
      <sce:action name="nobody_provides_this"/>
    </transition>
    <transition event="array">
      <sce:action name="start"><sce:arg expr="[1, 2]"/></sce:action>
    </transition>
    <transition event="unbound_arg">
      <sce:action name="start"><sce:arg expr="no_such_variable + 1"/></sce:action>
    </transition>
    <transition event="error.execution" target="failed"/>
  </state>

  <state id="failed"/>
</scxml>
)";

/// A host that provides `entered` and `start`, and records every call it is given.
class RecordingHost : public INativeActionHost {
public:
    bool performNativeAction(const std::string &name, const std::vector<ScriptValue> &args) override {
        calls.emplace_back(name, args);
        return provided.count(name) != 0;
    }

    std::vector<std::pair<std::string, std::vector<ScriptValue>>> calls;
    std::set<std::string> provided = {"entered", "start"};
};

}  // namespace

class NativeActionRunsUnderTheInterpreterTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
        sm_ = std::make_shared<StateMachine>(*engine_);
        raiser_ = std::make_shared<EventRaiserImpl>();
        sm_->setEventRaiser(raiser_);
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// The machine loaded with `host` installed first: an `<onentry>` of the
    /// initial state performs its actions during `start()`.
    void loadWith(std::shared_ptr<RecordingHost> host) {
        if (host) {
            sm_->setNativeActionHost(host);
        }
        ASSERT_TRUE(sm_->loadSCXMLFromString(DOCUMENT));
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<EventRaiserImpl> raiser_;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(NativeActionRunsUnderTheInterpreterTest, AnActionReachesTheHostWithEachArgumentAsTheTypedValueItIs) {
    auto host = std::make_shared<RecordingHost>();
    loadWith(host);
    ASSERT_TRUE(sm_->start());

    ASSERT_EQ(host->calls.size(), 1u) << "the initial state's <onentry> performed its action during start()";
    EXPECT_EQ(host->calls[0].first, "entered");
    EXPECT_TRUE(host->calls[0].second.empty());

    ASSERT_TRUE(sm_->processEvent("go").success);

    ASSERT_EQ(host->calls.size(), 2u);
    EXPECT_EQ(host->calls[1].first, "start");
    const auto &args = host->calls[1].second;
    ASSERT_EQ(args.size(), 4u) << "one value per <sce:arg>, in document order";
    ASSERT_TRUE(std::holds_alternative<int64_t>(args[0])) << "`n + 1` is an integer, not text";
    EXPECT_EQ(std::get<int64_t>(args[0]), 3);
    ASSERT_TRUE(std::holds_alternative<std::string>(args[1]));
    EXPECT_EQ(std::get<std::string>(args[1]), "job");
    ASSERT_TRUE(std::holds_alternative<bool>(args[2])) << "a comparison is a truth value";
    EXPECT_TRUE(std::get<bool>(args[2]));
    ASSERT_TRUE(std::holds_alternative<double>(args[3])) << "1.5 is a real";
    EXPECT_DOUBLE_EQ(std::get<double>(args[3]), 1.5);
    EXPECT_EQ(sm_->getCurrentState(), "idle") << "a performed action raises nothing";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AnActionRunsWhereverExecutableContentDoes) {
    auto host = std::make_shared<RecordingHost>();
    loadWith(host);
    ASSERT_TRUE(sm_->start());
    host->calls.clear();

    ASSERT_TRUE(sm_->processEvent("guarded").success);

    ASSERT_EQ(host->calls.size(), 1u) << "an action inside an <if> whose condition holds is performed";
    EXPECT_EQ(host->calls[0].first, "start");
    ASSERT_EQ(host->calls[0].second.size(), 1u);
    EXPECT_EQ(std::get<int64_t>(host->calls[0].second[0]), 2);
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AnActionNoHostPerformsIsAnExecutionError) {
    loadWith(nullptr);
    ASSERT_TRUE(sm_->start());
    ASSERT_EQ(sm_->getCurrentState(), "failed")
        << "the <onentry> action had no host to perform it, which is error.execution and not a dropped request";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AHostThatProvidesNoSuchOperationIsAnExecutionError) {
    auto host = std::make_shared<RecordingHost>();
    loadWith(host);
    ASSERT_TRUE(sm_->start());
    ASSERT_EQ(sm_->getCurrentState(), "idle");

    ASSERT_TRUE(sm_->processEvent("missing").success);

    ASSERT_EQ(host->calls.back().first, "nobody_provides_this")
        << "the host was asked, and answered that it provides none";
    EXPECT_EQ(sm_->getCurrentState(), "failed");
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AnArgumentNoHostOperationTakesStopsTheActionBeforeTheHost) {
    for (const char *event : {"array", "unbound_arg"}) {
        SetUp();
        auto host = std::make_shared<RecordingHost>();
        loadWith(host);
        ASSERT_TRUE(sm_->start());
        host->calls.clear();

        ASSERT_TRUE(sm_->processEvent(event).success);

        EXPECT_TRUE(host->calls.empty()) << event << ": the host is not called with a value it cannot take";
        EXPECT_EQ(sm_->getCurrentState(), "failed") << event;
        TearDown();
    }
}

}  // namespace Tests
}  // namespace SCE
