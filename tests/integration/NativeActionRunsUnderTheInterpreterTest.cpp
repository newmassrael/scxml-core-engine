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
//     — and in the last case the host is not called at all;
//   * a child session of an `<invoke>` is given the host its parent's host answers
//     for it, named by the invoke's id and the stem of the document it came from
//     (docs/adr/0005, decision 6) — and a parent with no host, or a host that
//     answers none, leaves the child without one, so that its first action raises
//     `error.execution` in the child;
//   * an action states its effect, and the Interpreter refuses what the build tool
//     refuses (docs/adr/0038, decisions 1, 2 and 6): an action with no effect or a
//     third word, one name that is a duty at one site and a notice at another, and
//     each of the five ways a `released-by` cannot be honoured, as the same
//     diagnostic codes, key fragments and sentences;
//   * a document that states a duty is not started with a host that declares it
//     only simulates the duties, unless the caller accepted such a host by name —
//     wherever the duty sits, and for a child the parent's host answers.

#include "actions/NativeAction.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "factory/NodeFactory.h"
#include "parsing/Diagnostic.h"
#include "parsing/SCXMLParser.h"
#include "parsing/SemanticError.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/INativeActionHost.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <memory>
#include <mutex>
#include <set>
#include <string>
#include <thread>
#include <unistd.h>
#include <utility>
#include <vector>

namespace SCE {
namespace Tests {

namespace {

/// Every transition is targetless, so the machine stays in `idle` whatever
/// the action does and `error.execution` is what moves it to `failed`. Every
/// action is a notice, which any host takes, a recorder that simulates included.
constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="idle">

  <datamodel>
    <data id="n" expr="2"/>
  </datamodel>

  <state id="idle">
    <onentry>
      <sce:action name="entered" effect="notice"/>
    </onentry>
    <transition event="go">
      <sce:action name="start" effect="notice">
        <sce:arg expr="n + 1"/>
        <sce:arg expr="'job'"/>
        <sce:arg expr="n === 2"/>
        <sce:arg expr="1.5"/>
      </sce:action>
    </transition>
    <transition event="guarded">
      <if cond="n === 2">
        <sce:action name="start" effect="notice"><sce:arg expr="n"/></sce:action>
      </if>
    </transition>
    <transition event="missing">
      <sce:action name="nobody_provides_this" effect="notice"/>
    </transition>
    <transition event="array">
      <sce:action name="start" effect="notice"><sce:arg expr="[1, 2]"/></sce:action>
    </transition>
    <transition event="unbound_arg">
      <sce:action name="start" effect="notice"><sce:arg expr="no_such_variable + 1"/></sce:action>
    </transition>
    <transition event="absent">
      <sce:action name="start" effect="notice">
        <sce:arg expr="null"/>
        <sce:arg expr="n"/>
      </sce:action>
    </transition>
    <transition event="error.execution" target="failed"/>
  </state>

  <state id="failed"/>
</scxml>
)";

/// A machine that states a duty and gives it back: `idle` takes `start_worker` on
/// entry and releases it through `stop_worker` on its `<onexit>`, beside the notice
/// `note_idle`.
constexpr const char *DUTY_DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="idle">
  <state id="idle">
    <onentry>
      <sce:action name="start_worker" effect="duty" released-by="stop_worker"/>
      <sce:action name="note_idle" effect="notice"/>
    </onentry>
    <onexit>
      <sce:action name="stop_worker" effect="duty"/>
    </onexit>
    <transition event="go" target="done"/>
  </state>
  <state id="done"/>
</scxml>
)";

/// A host that provides `entered`, `start` and the operations of `DUTY_DOCUMENT`
/// and the placements below, and records every call it is given. It says what it
/// does with duties as the test asks: a recorder simulates, which is what the
/// scenario drivers install it as.
class RecordingHost : public INativeActionHost {
public:
    explicit RecordingHost(ActionRealization realization = ActionRealization::Simulates) : realization_(realization) {}

    ActionRealization realization() const override {
        return realization_;
    }

    bool performNativeAction(const std::string &name, const std::vector<ScriptValue> &args) override {
        calls.emplace_back(name, args);
        return provided.count(name) != 0;
    }

    std::vector<std::pair<std::string, std::vector<ScriptValue>>> calls;
    std::set<std::string> provided = {"entered", "start", "start_worker", "stop_worker", "note_idle", "hold"};

private:
    ActionRealization realization_;
};

/// The host a child session is given: it provides `entered`, and a child performs
/// its actions on a thread of its own, so what it was asked is read under the lock
/// it is written under.
class ChildHost : public INativeActionHost {
public:
    explicit ChildHost(ActionRealization realization = ActionRealization::Simulates) : realization_(realization) {}

    ActionRealization realization() const override {
        return realization_;
    }

    bool performNativeAction(const std::string &name, const std::vector<ScriptValue> &) override {
        std::lock_guard<std::mutex> lock(mutex_);
        calls_.push_back(name);
        return name == "entered";
    }

    std::vector<std::string> calls() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return calls_;
    }

private:
    ActionRealization realization_;
    mutable std::mutex mutex_;
    std::vector<std::string> calls_;
};

/// A parent's host that performs nothing itself and answers `child` — which may be
/// none — for every child, recording what it was asked.
class ParentHost : public INativeActionHost {
public:
    explicit ParentHost(std::shared_ptr<INativeActionHost> child) : child_(std::move(child)) {}

    ActionRealization realization() const override {
        return ActionRealization::Simulates;
    }

    bool performNativeAction(const std::string &, const std::vector<ScriptValue> &) override {
        return false;
    }

    std::shared_ptr<INativeActionHost> hostForChild(const std::string &invokeId, const std::string &document) override {
        std::lock_guard<std::mutex> lock(mutex_);
        asked_.emplace_back(invokeId, document);
        return child_;
    }

    std::vector<std::pair<std::string, std::string>> asked() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return asked_;
    }

private:
    std::shared_ptr<INativeActionHost> child_;
    mutable std::mutex mutex_;
    std::vector<std::pair<std::string, std::string>> asked_;
};

/// A parent whose child comes from a document, as its `src` names it.
constexpr const char *PARENT_OF_A_DOCUMENT_CHILD = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="working">
  <state id="working">
    <invoke type="scxml" id="job" src="hosted.scxml"/>
    <transition event="done.invoke.job" target="reported"/>
  </state>
  <state id="reported"/>
</scxml>
)";

/// The child: it performs `entered` as it is entered, and if no one performs it the
/// `error.execution` that follows ends it, which its parent is told.
constexpr const char *HOSTED_CHILD = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="busy">
  <state id="busy">
    <onentry>
      <sce:action name="entered" effect="notice"/>
    </onentry>
    <transition event="error.execution" target="failed"/>
  </state>
  <final id="failed"/>
</scxml>
)";

/// The same child, whose `entered` is a duty.
constexpr const char *HOSTED_DUTY_CHILD = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="busy">
  <state id="busy">
    <onentry>
      <sce:action name="entered" effect="duty"/>
    </onentry>
    <transition event="error.execution" target="failed"/>
  </state>
  <final id="failed"/>
</scxml>
)";

/// A document whose states are `states` (a `<datamodel>` may lead them), the
/// first of which is `s`.
std::string documentOf(const std::string &states) {
    return std::string(R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="s">
)") + states +
           "\n</scxml>\n";
}

/// What the document is refused with: the first typed diagnostic the parser
/// recorded, or null when it was accepted or refused without one.
std::unique_ptr<parsing::Diagnostic> refusalOf(const std::string &document) {
    SCXMLParser parser(std::make_shared<NodeFactory>());
    const auto model = parser.parseContent(document);
    EXPECT_EQ(model, nullptr) << "the document is refused:\n" << document;
    const auto &diagnostics = parser.getDiagnostics();
    if (diagnostics.empty()) {
        return nullptr;
    }
    return diagnostics.front()->clone();
}

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
        if (!dir_.empty()) {
            std::error_code ec;
            std::filesystem::remove_all(dir_, ec);
        }
    }

    /// The parent of a document child, loaded from its file — a `src` is resolved
    /// against the document that names it — with `host` installed, and wired to a
    /// dispatcher so that its child can answer it. `simulated` installs the host as
    /// one the caller accepts by name; `child` is the text of the document the child
    /// is loaded from.
    void loadParentOfADocumentChild(std::shared_ptr<INativeActionHost> host, bool simulated = false,
                                    const char *child = HOSTED_CHILD) {
        dir_ = std::filesystem::temp_directory_path() / ("sce-child-host-interpreter-" + std::to_string(::getpid()));
        std::error_code ec;
        std::filesystem::remove_all(dir_, ec);
        std::filesystem::create_directories(dir_);
        {
            std::ofstream(dir_ / "parent.scxml") << PARENT_OF_A_DOCUMENT_CHILD;
            std::ofstream(dir_ / "hosted.scxml") << child;
        }
        auto scheduler = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                try {
                    return target->send(event).get().isSuccess;
                } catch (...) {
                    return false;
                }
            });
        raiser_->setScheduler(scheduler);
        raiser_->setImmediateMode(false);
        sm_->setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(raiser_)));
        if (host) {
            if (simulated) {
                sm_->setSimulatedNativeActionHost(host);
            } else {
                sm_->setNativeActionHost(host);
            }
        }
        ASSERT_TRUE(sm_->loadSCXML((dir_ / "parent.scxml").string()));
    }

    /// Whether `done` comes true within `within`, with the parent's queue run on
    /// every look: a child answers its parent on a thread of its own.
    bool waitFor(const std::function<bool()> &done, std::chrono::milliseconds within = std::chrono::seconds(5)) {
        const auto deadline = std::chrono::steady_clock::now() + within;
        while (std::chrono::steady_clock::now() < deadline) {
            raiser_->processQueuedEvents();
            if (done()) {
                return true;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        return false;
    }

    std::filesystem::path dir_;

    /// The machine loaded with `host` installed first: an `<onentry>` of the
    /// initial state performs its actions during `start()`.
    void loadWith(std::shared_ptr<RecordingHost> host) {
        if (host) {
            sm_->setNativeActionHost(host);
        }
        ASSERT_TRUE(sm_->loadSCXMLFromString(DOCUMENT));
    }

    /// The document of a duty, with no host installed yet.
    void loadDutyDocument() {
        ASSERT_TRUE(sm_->loadSCXMLFromString(DUTY_DOCUMENT));
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

// An argument declared `optional<T>` that is absent evaluates to `null`, which is a value a host
// operation takes (docs/adr/0055, step 10): the host is told it is absent, where the generated hosts
// are handed their language's absent optional, and the action is not refused. `undefined` and an
// array stay refused above.
TEST_F(NativeActionRunsUnderTheInterpreterTest, AnArgumentThatIsNullIsHandedToTheHostAsAbsent) {
    auto host = std::make_shared<RecordingHost>();
    loadWith(host);
    ASSERT_TRUE(sm_->start());
    host->calls.clear();

    ASSERT_TRUE(sm_->processEvent("absent").success);

    ASSERT_EQ(host->calls.size(), 1u) << "the host was called: `null` is an argument a host takes";
    EXPECT_EQ(host->calls[0].first, "start");
    const auto &args = host->calls[0].second;
    ASSERT_EQ(args.size(), 2u);
    EXPECT_TRUE(std::holds_alternative<ScriptNull>(args[0])) << "an absent argument is `null`, not `undefined`";
    ASSERT_TRUE(std::holds_alternative<int64_t>(args[1])) << "a present one beside it is the value it is";
    EXPECT_EQ(std::get<int64_t>(args[1]), 2);
    EXPECT_EQ(sm_->getCurrentState(), "idle") << "a performed action raises nothing";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AChildIsGivenTheHostItsParentsHostAnswersForItsDocument) {
    auto child = std::make_shared<ChildHost>();
    auto parentHost = std::make_shared<ParentHost>(child);
    loadParentOfADocumentChild(parentHost);
    ASSERT_TRUE(sm_->start());

    ASSERT_TRUE(waitFor([&] { return !child->calls().empty(); }))
        << "the child's first action never reached a host: it was not built with the one its parent answered";
    EXPECT_EQ(child->calls(), std::vector<std::string>{"entered"});
    const std::vector<std::pair<std::string, std::string>> expected = {{"job", "hosted"}};
    EXPECT_EQ(parentHost->asked(), expected)
        << "the parent's host is asked once, with the invoke's id and the stem of the document it names";

    // The action was performed, so nothing failed and the child is still running: it
    // would otherwise have ended, and its parent been told.
    EXPECT_FALSE(waitFor([&] { return sm_->getCurrentState() == "reported"; }, std::chrono::milliseconds(300)))
        << "the child's action was refused although its host performs it";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AHostThatAnswersNoHostLeavesTheChildWithoutOne) {
    auto parentHost = std::make_shared<ParentHost>(nullptr);
    loadParentOfADocumentChild(parentHost);
    ASSERT_TRUE(sm_->start());

    ASSERT_TRUE(waitFor([&] { return sm_->getCurrentState() == "reported"; }))
        << "a child with no host performs its action as a machine with none does: error.execution, which "
           "ended it, and its parent was told";
    EXPECT_EQ(parentHost->asked().size(), 1u) << "it was asked, and answered none";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AParentWithNoHostLeavesItsChildWithoutOne) {
    loadParentOfADocumentChild(nullptr);
    ASSERT_TRUE(sm_->start());

    ASSERT_TRUE(waitFor([&] { return sm_->getCurrentState() == "reported"; }))
        << "no host anywhere: the child's action is error.execution, which ended it";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AHostThatSaysNothingOfChildrenAnswersNone) {
    ChildHost host;
    EXPECT_EQ(host.hostForChild("job", "hosted"), nullptr);
}

// ---------------------------------------------------------------------------
// docs/adr/0038, decision 6: the Interpreter reads the two attributes, and a
// document that states a duty is not started with a host that only simulates.
// ---------------------------------------------------------------------------

TEST_F(NativeActionRunsUnderTheInterpreterTest, AnActionCarriesItsEffectAndItsRelease) {
    SCXMLParser parser(std::make_shared<NodeFactory>());
    const auto model = parser.parseContent(DUTY_DOCUMENT);
    ASSERT_NE(model, nullptr);
    const auto *idle = model->findStateById("idle");
    ASSERT_NE(idle, nullptr);

    ASSERT_EQ(idle->getEntryActionBlocks().size(), 1u);
    const auto &entry = idle->getEntryActionBlocks()[0];
    ASSERT_EQ(entry.size(), 2u);
    const auto take = std::dynamic_pointer_cast<NativeAction>(entry[0]);
    const auto note = std::dynamic_pointer_cast<NativeAction>(entry[1]);
    ASSERT_NE(take, nullptr);
    ASSERT_NE(note, nullptr);
    EXPECT_EQ(take->getEffect(), NativeAction::Effect::Duty);
    EXPECT_EQ(take->getReleasedBy(), "stop_worker");
    EXPECT_EQ(note->getEffect(), NativeAction::Effect::Notice);
    EXPECT_TRUE(note->getReleasedBy().empty()) << "a notice names no release";

    const auto copy = std::dynamic_pointer_cast<NativeAction>(take->clone());
    ASSERT_NE(copy, nullptr);
    EXPECT_EQ(copy->getEffect(), NativeAction::Effect::Duty) << "a copy keeps what the action states";
    EXPECT_EQ(copy->getReleasedBy(), "stop_worker");
    EXPECT_NE(take->getDescription().find("effect=\"duty\""), std::string::npos) << take->getDescription();
    EXPECT_NE(take->getDescription().find("released-by=\"stop_worker\""), std::string::npos) << take->getDescription();
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, ADocumentThatStatesADutyStartsWithAHostThatPerforms) {
    auto host = std::make_shared<RecordingHost>(ActionRealization::Performs);
    sm_->setNativeActionHost(host);
    loadDutyDocument();

    ASSERT_TRUE(sm_->start()) << "a host that declares it performs the duties is the one a duty is for";

    ASSERT_EQ(host->calls.size(), 2u);
    EXPECT_EQ(host->calls[0].first, "start_worker");
    EXPECT_EQ(host->calls[1].first, "note_idle");
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AHostThatSimulatesIsRefusedADocumentThatStatesADuty) {
    auto host = std::make_shared<RecordingHost>(ActionRealization::Simulates);
    sm_->setNativeActionHost(host);
    loadDutyDocument();

    EXPECT_FALSE(sm_->start()) << "a recorder installed the plain way does not run a machine that depends on its host";

    EXPECT_FALSE(sm_->isRunning());
    EXPECT_TRUE(host->calls.empty()) << "it is refused before anything of the machine runs, so the duty on entry "
                                        "never reaches the recorder";

    // The refusal is of the host, not of the machine: a host that performs starts it.
    auto performing = std::make_shared<RecordingHost>(ActionRealization::Performs);
    sm_->setNativeActionHost(performing);
    ASSERT_TRUE(sm_->start());
    EXPECT_EQ(performing->calls.size(), 2u);
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AHostThatSimulatesIsAcceptedWhenTheCallerSaysSoByName) {
    auto host = std::make_shared<RecordingHost>(ActionRealization::Simulates);
    sm_->setSimulatedNativeActionHost(host);
    loadDutyDocument();

    ASSERT_TRUE(sm_->start());

    ASSERT_EQ(host->calls.size(), 2u) << "the machine runs, and the recorder is told what it was asked";
    EXPECT_EQ(host->calls[0].first, "start_worker");
    EXPECT_TRUE(sm_->acceptsSimulatedNativeActionHost());
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, TheAcceptanceBelongsToTheHostItWasGivenFor) {
    auto first = std::make_shared<RecordingHost>(ActionRealization::Simulates);
    sm_->setSimulatedNativeActionHost(first);
    loadDutyDocument();
    ASSERT_TRUE(sm_->acceptsSimulatedNativeActionHost());

    // A host installed the plain way afterwards is not covered by what the caller
    // said about the one before it.
    auto second = std::make_shared<RecordingHost>(ActionRealization::Simulates);
    sm_->setNativeActionHost(second);

    EXPECT_FALSE(sm_->acceptsSimulatedNativeActionHost());
    EXPECT_FALSE(sm_->start());
    EXPECT_TRUE(second->calls.empty());
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, ADocumentOfNoticesTakesAHostThatSimulates) {
    auto host = std::make_shared<RecordingHost>(ActionRealization::Simulates);
    loadWith(host);

    ASSERT_TRUE(sm_->start()) << "a machine that states no duty takes any host, a recorder installed the plain way "
                                 "included";

    ASSERT_EQ(host->calls.size(), 1u);
    EXPECT_EQ(host->calls[0].first, "entered");
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AMachineWithNoHostIsNotRefusedWhateverItsDocumentStates) {
    loadDutyDocument();

    EXPECT_TRUE(sm_->start()) << "no host was installed, so there is none to declare it only simulates; the duty on "
                                 "entry is an error.execution where it runs, as an action no one performs is";
    EXPECT_EQ(sm_->getCurrentState(), "idle");
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, ADutyIsFoundWhereverTheDocumentPutsIt) {
    const std::string duty = R"(<sce:action name="hold" effect="duty"/>)";
    const std::string notice = R"(<sce:action name="hold" effect="notice"/>)";
    // Each placement has a hole for the one action; the same document with a notice
    // in it is the control that the refusal is of the duty and not of the shape.
    const std::vector<std::pair<const char *, std::string>> placements = {
        {"an <onentry>", R"(<state id="s"><onentry>@</onentry></state>)"},
        {"an <onexit>", R"(<state id="s"><onexit>@</onexit></state>)"},
        {"a <transition>", R"(<state id="s"><transition event="go">@</transition></state>)"},
        {"an <initial>",
         R"(<state id="s"><initial><transition target="a">@</transition></initial><state id="a"/></state>)"},
        {"a history's default",
         R"(<state id="s" initial="a"><history id="h"><transition target="a">@</transition></history><state id="a"/></state>)"},
        {"an <if> of an <onentry>", R"(<state id="s"><onentry><if cond="true">@</if></onentry></state>)"},
        {"a <foreach> of an <onentry>",
         R"(<datamodel><data id="xs" expr="[1]"/></datamodel>)"
         R"(<state id="s"><onentry><foreach array="xs" item="i">@</foreach></onentry></state>)"},
    };
    const auto fill = [](std::string states, const std::string &action) {
        states.replace(states.find('@'), 1, action);
        return documentOf(states);
    };

    for (const auto &[label, states] : placements) {
        SCOPED_TRACE(label);

        SetUp();
        auto simulating = std::make_shared<RecordingHost>(ActionRealization::Simulates);
        sm_->setNativeActionHost(simulating);
        ASSERT_TRUE(sm_->loadSCXMLFromString(fill(states, duty)));
        EXPECT_FALSE(sm_->start()) << "a duty in " << label << " is a duty";
        EXPECT_TRUE(simulating->calls.empty());
        TearDown();

        SetUp();
        auto control = std::make_shared<RecordingHost>(ActionRealization::Simulates);
        sm_->setNativeActionHost(control);
        ASSERT_TRUE(sm_->loadSCXMLFromString(fill(states, notice)));
        EXPECT_TRUE(sm_->start()) << "a notice in " << label << " takes a recorder";
        TearDown();
    }
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AChildThatStatesADutyIsRefusedTheSimulatingHostItsParentAnswers) {
    auto child = std::make_shared<ChildHost>(ActionRealization::Simulates);
    auto parentHost = std::make_shared<ParentHost>(child);
    loadParentOfADocumentChild(parentHost, /*simulated=*/false, HOSTED_DUTY_CHILD);

    ASSERT_TRUE(sm_->start()) << "the parent states no duty, so its recorder is no obstacle to it";

    EXPECT_FALSE(waitFor([&] { return !child->calls().empty(); }, std::chrono::milliseconds(400)))
        << "the child states a duty and its host simulates: it is checked by the rule its parent is, and refused "
           "before its first action reaches the host";
    EXPECT_NE(sm_->getCurrentState(), "reported") << "a child that never started never ended";
    EXPECT_EQ(parentHost->asked().size(), 1u) << "the host was asked for, and then refused by the child's start";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AChildIsAcceptedTheSimulatingHostWhenItsParentWas) {
    auto child = std::make_shared<ChildHost>(ActionRealization::Simulates);
    auto parentHost = std::make_shared<ParentHost>(child);
    loadParentOfADocumentChild(parentHost, /*simulated=*/true, HOSTED_DUTY_CHILD);
    ASSERT_TRUE(sm_->start());

    ASSERT_TRUE(waitFor([&] { return !child->calls().empty(); }))
        << "the acceptance the caller gave the parent covers the children it starts";
    EXPECT_EQ(child->calls(), std::vector<std::string>{"entered"});
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AChildThatStatesADutyTakesTheHostThatPerformsItsParentAnswers) {
    auto child = std::make_shared<ChildHost>(ActionRealization::Performs);
    auto parentHost = std::make_shared<ParentHost>(child);
    loadParentOfADocumentChild(parentHost, /*simulated=*/false, HOSTED_DUTY_CHILD);
    ASSERT_TRUE(sm_->start());

    ASSERT_TRUE(waitFor([&] { return !child->calls().empty(); }))
        << "a child's host that performs the duties needs no acceptance from anyone";
    EXPECT_EQ(child->calls(), std::vector<std::string>{"entered"});
}

// ---------------------------------------------------------------------------
// docs/adr/0038, decisions 1 and 2: what the build tool refuses at the element
// and between elements, the Interpreter refuses as the same diagnostic.
// ---------------------------------------------------------------------------

TEST_F(NativeActionRunsUnderTheInterpreterTest, AnActionThatStatesNoEffectIsRefusedAtTheElement) {
    const std::string document = documentOf(R"(<state id="s"><onentry><sce:action name="x"/></onentry></state>)");

    const auto refusal = refusalOf(document);

    ASSERT_NE(refusal, nullptr);
    EXPECT_EQ(std::string(refusal->code()), "validation/missing-attribute");
    EXPECT_EQ(refusal->keyFragments(), (std::vector<std::string>{"<sce:action>", "effect"}))
        << "the fragments the build tool derives its id from";
    EXPECT_EQ(refusal->to_json().at("fix").at("attr").get<std::string>(), "effect");

    ASSERT_FALSE(sm_->loadSCXMLFromString(document)) << "the load that starts a machine refuses it";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AnEffectOfOnlyWhitespaceIsNoEffect) {
    const auto refusal =
        refusalOf(documentOf(R"(<state id="s"><onentry><sce:action name="x" effect="  "/></onentry></state>)"));

    ASSERT_NE(refusal, nullptr);
    EXPECT_EQ(std::string(refusal->code()), "validation/missing-attribute");
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AnEffectThatIsNeitherDutyNorNoticeIsRefusedWithBothAsCandidates) {
    for (const char *word : {"must", "Duty", "duties"}) {
        SCOPED_TRACE(word);
        const auto refusal =
            refusalOf(documentOf(std::string(R"(<state id="s"><onentry><sce:action name="x" effect=")") + word +
                                 R"("/></onentry></state>)"));

        ASSERT_NE(refusal, nullptr);
        EXPECT_EQ(std::string(refusal->code()), "validation/invalid-attribute");
        EXPECT_EQ(refusal->keyFragments(), (std::vector<std::string>{"<sce:action>", "effect", word}));
        const auto record = refusal->to_json();
        EXPECT_EQ(record.at("actual").get<std::string>(), word);
        EXPECT_EQ(record.at("fix").at("candidates"), nlohmann::ordered_json::array({"duty", "notice"}));
    }
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, OneNameCannotBeADutyAtOneSiteAndANoticeAtAnother) {
    const auto refusal = refusalOf(documentOf(R"(<state id="s"><onentry><sce:action name="f" effect="duty"/></onentry>)"
                                              R"(<onexit><sce:action name="f" effect="notice"/></onexit></state>)"));

    ASSERT_NE(refusal, nullptr);
    EXPECT_EQ(std::string(refusal->code()), "validation/native-action-signature-conflict");
    EXPECT_EQ(refusal->keyFragments(),
              (std::vector<std::string>{"f", "effect 'notice' here disagrees with 'duty' at another call site: one "
                                             "host operation is either a duty or a notice"}))
        << "the site that contradicts is the later one, and the sentence is the build tool's";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, TheSameEffectAtEverySiteOfANameIsAccepted) {
    SCXMLParser parser(std::make_shared<NodeFactory>());
    const auto model = parser.parseContent(
        documentOf(R"(<state id="s"><onentry><sce:action name="f" effect="duty"/></onentry>)"
                   R"(<transition event="go"><sce:action name="f" effect="duty"/></transition></state>)"));
    EXPECT_NE(model, nullptr) << "one name, one effect, however many sites";
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, AReleaseThatCannotBeHonouredIsRefusedWithTheReasonItHas) {
    struct Case {
        const char *label;
        std::string states;
        const char *name;
        const char *releasedBy;
        const char *detail;
    };

    const std::vector<Case> cases = {
        {"off an <onentry>",
         R"(<state id="s"><transition event="go" target="t">)"
         R"(<sce:action name="take" effect="duty" released-by="give"/></transition></state>)"
         R"(<state id="t"><onexit><sce:action name="give" effect="duty"/></onexit></state>)",
         "take", "give",
         "released-by is stated only on an action of an <onentry>: it names the action of the same state's <onexit> "
         "that gives back what the entry took"},
        {"on a notice",
         R"(<state id="s"><onentry><sce:action name="note" effect="notice" released-by="give"/></onentry>)"
         R"(<onexit><sce:action name="give" effect="duty"/></onexit></state>)",
         "note", "give", "a notice takes nothing to release; released-by belongs to an action whose effect is duty"},
        {"naming no action",
         R"(<state id="s"><onentry><sce:action name="take" effect="duty" released-by="nobody"/></onentry></state>)",
         "take", "nobody", "names no <sce:action> of this document"},
        {"performed on another state's <onexit>",
         R"(<state id="s"><onentry><sce:action name="take" effect="duty" released-by="give"/></onentry></state>)"
         R"(<state id="t"><onexit><sce:action name="give" effect="duty"/></onexit></state>)",
         "take", "give",
         "'give' is not performed on the <onexit> of state 's', whose <onentry> takes this duty: a way out of the "
         "state would not release it"},
        {"performed on the <onentry> and not the <onexit>",
         R"(<state id="s"><onentry><sce:action name="take" effect="duty" released-by="give"/>)"
         R"(<sce:action name="give" effect="duty"/></onentry></state>)",
         "take", "give",
         "'give' is not performed on the <onexit> of state 's', whose <onentry> takes this duty: a way out of the "
         "state would not release it"},
        {"naming a notice",
         R"(<state id="s"><onentry><sce:action name="take" effect="duty" released-by="give"/></onentry>)"
         R"(<onexit><sce:action name="give" effect="notice"/></onexit></state>)",
         "take", "give",
         "'give' is a notice, which the host may drop, so the duty it is to release would stay held; make it a duty"},
    };

    for (const auto &c : cases) {
        SCOPED_TRACE(c.label);
        const auto refusal = refusalOf(documentOf(c.states));

        ASSERT_NE(refusal, nullptr);
        EXPECT_EQ(std::string(refusal->code()), "validation/native-action-release");
        const auto *release = dynamic_cast<const parsing::SemanticNativeActionRelease *>(refusal.get());
        ASSERT_NE(release, nullptr);
        EXPECT_EQ(release->name(), c.name);
        EXPECT_EQ(release->releasedBy(), c.releasedBy);
        EXPECT_EQ(release->detail(), c.detail) << "the sentence is the build tool's, word for word";
        EXPECT_EQ(refusal->keyFragments(), (std::vector<std::string>{c.name, c.releasedBy, c.detail}))
            << "the fragments the build tool derives its id from";
        EXPECT_EQ(refusal->to_json().at("actual").get<std::string>(), c.releasedBy);
    }
}

TEST_F(NativeActionRunsUnderTheInterpreterTest, ADutyMayStateNoReleaseAndLastTheMachinesLife) {
    SCXMLParser parser(std::make_shared<NodeFactory>());
    const auto model = parser.parseContent(
        documentOf(R"(<state id="s"><onentry><sce:action name="take" effect="duty"/></onentry></state>)"));
    EXPECT_NE(model, nullptr) << "a duty with no released-by is held for the machine's life";
}

}  // namespace Tests
}  // namespace SCE
