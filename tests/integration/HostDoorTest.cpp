// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, D4 and D6: the C++ Interpreter serves every type a host
// registers through the door the generated engines have, and a Mesh request is
// one row of it.
//
// Until this door the Interpreter's processor and invoker sets were closed: a
// `<send>` or an `<invoke>` of any type but SCE's own raised error.execution,
// and a host had no call through which to serve one. What it has now is the
// generated engine's: an event processor and an invoker per type, shared by the
// sessions of the tree, the reserved prefix refused, and a host-run invocation
// kept by the shared ledger (core/HostInvokeLedger.h) with its token, its
// deadline and its one ending. Each test holds one of those apart:
//
//   * registration refuses the reserved types the shared table names, and the
//     router's own call is the one way under the prefix;
//   * a type nobody serves is still error.execution, for a send and an invoke;
//   * a served send hands the host its request and the replies arrive in order,
//     at once or when a delay elapses, and a <cancel> reaches a delayed one;
//   * a served invoke completes at once or later, fails, is cancelled when its
//     state exits, and is ended by its deadline, each exactly once, and an
//     ending of a start that is over is refused by its token;
//   * a Mesh request reaches the router the host registered, and with none it is
//     error.execution;
//   * an ending named by its session reaches the child session that started it.
//
// The requests themselves are held field by field to the generated engine's in
// HostRequestTest.cpp; this file is about the lifecycle.

#include "common/SendHelper.h"
#include "core/HostProcessor.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <memory>
#include <mutex>
#include <nlohmann/json.hpp>
#include <optional>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT must name the checkout: the reserved-type table is read from it"
#endif

namespace SCE::Tests {

namespace {

/// The type the documents below name for the host's processor and invoker.
constexpr const char *DECLARED_TYPE = "x-sce-host";

/// What the host's invoker saw, in call order, and the token of each start. The
/// deadline arrives on the scheduler's thread, so the record is guarded.
class InvokerLog {
public:
    void start(const HostInvokeRequest &request) {
        std::lock_guard<std::mutex> lock(mutex_);
        lines_.push_back("START " + request.invokeId);
        requests_.push_back(request);
    }

    void cancel(const HostInvokeCancel &cancel) {
        std::lock_guard<std::mutex> lock(mutex_);
        lines_.push_back("CANCEL " + cancel.invokeId + " " + std::to_string(cancel.token));
    }

    std::vector<std::string> lines() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return lines_;
    }

    std::vector<HostInvokeRequest> requests() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return requests_;
    }

    /// The latest start of @p invokeId, which a test that ends it needs.
    std::optional<HostInvokeRequest> latest(const std::string &invokeId) const {
        std::lock_guard<std::mutex> lock(mutex_);
        for (auto it = requests_.rbegin(); it != requests_.rend(); ++it) {
            if (it->invokeId == invokeId) {
                return *it;
            }
        }
        return std::nullopt;
    }

    bool saw(const std::string &line) const {
        const auto all = lines();
        return std::find(all.begin(), all.end(), line) != all.end();
    }

private:
    mutable std::mutex mutex_;
    std::vector<std::string> lines_;
    std::vector<HostInvokeRequest> requests_;
};

class HostDoorTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        if (sm_) {
            sm_->stop();
        }
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A machine for @p scxml with a scheduler and a dispatcher, loaded and not yet
    /// started: a host registers before `start()`, since the initial state's
    /// requests are made during it.
    std::shared_ptr<StateMachine> prepare(const std::string &scxml) {
        sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
        auto scheduler = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                try {
                    return target->send(event).get().isSuccess;
                } catch (...) {
                    return false;
                }
            });
        raiser_ = std::make_shared<EventRaiserImpl>();
        raiser_->setScheduler(scheduler);
        raiser_->setImmediateMode(false);
        sm_->setEventRaiser(raiser_);
        sm_->setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(raiser_)));
        EXPECT_TRUE(sm_->loadSCXMLFromString(scxml)) << "the document did not load";
        return sm_;
    }

    /// An invoker that records what it saw into @p log and answers a start with
    /// @p answer, or with nothing when there is none.
    static HostInvokeHandler recordingInvoker(InvokerLog &log,
                                              std::function<std::optional<HostInvokeResponse>()> answer = nullptr) {
        return [&log, answer](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
            if (event.start.has_value()) {
                log.start(*event.start);
                return answer ? answer() : std::nullopt;
            }
            if (event.cancel.has_value()) {
                log.cancel(*event.cancel);
            }
            return std::nullopt;
        };
    }

    /// The text of @p expr in the machine's data model.
    std::string read(const std::string &expr) const {
        const auto result = engine_->evaluateExpression(sm_->getSessionId(), "String(" + expr + ")").get();
        return result.isSuccess() ? result.getValue<std::string>() : std::string("<unreadable>");
    }

    /// Wait, up to five seconds, until the run ends; the top-level final it ended
    /// in, or empty.
    std::string terminalWithin() const {
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
        while (std::chrono::steady_clock::now() < deadline && !sm_->terminalState().has_value()) {
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        return sm_->terminalState().value_or("");
    }

    // What the host was handed. Members, so they outlive the machine a test
    // leaves running, which TearDown stops.
    InvokerLog invoker_;
    std::mutex sentMutex_;
    std::vector<HostSendRequest> sent_;

    /// A processor that records what it was handed into `sent_` and answers each
    /// send with @p answer's events.
    HostSendHandler recordingProcessor(std::function<std::vector<HostSendResponse>(const HostSendRequest &)> answer) {
        return [this, answer](const HostSendRequest &request) {
            {
                std::lock_guard<std::mutex> lock(sentMutex_);
                sent_.push_back(request);
            }
            return answer(request);
        };
    }

    std::vector<HostSendRequest> sent() {
        std::lock_guard<std::mutex> lock(sentMutex_);
        return sent_;
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
    std::shared_ptr<EventRaiserImpl> raiser_;
};

/// One invocation that the host may complete, fail or leave running; `leave` and
/// `back` exit its state and enter it again.
const char *const LIFECYCLE = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="running" name="host_door_lifecycle">
  <datamodel>
    <data id="ended" expr="''"/>
  </datamodel>
  <state id="running">
    <invoke id="job" type="x-sce-host" src="pane://job"/>
    <transition event="done.invoke.job" cond="_event.invokeid === 'job'" target="completed">
      <assign location="ended" expr="JSON.stringify(_event.data)"/>
    </transition>
    <transition event="error.invoke.job" cond="_event.invokeid === 'job'" target="failed">
      <assign location="ended" expr="_event.data + '|' + _event.origin + '|' + _event.origintype"/>
    </transition>
    <transition event="leave" target="away"/>
  </state>
  <state id="away">
    <transition event="back" target="running"/>
  </state>
  <final id="completed"/>
  <final id="failed"/>
</scxml>)";

}  // namespace

// sce-build/tests/fixtures/host_processor/reserved_type_cases.json: the table the
// build's declaration check and every runtime's registration read. Each case is
// asked of both registrations, so a registration that stopped consulting it fails.
TEST_F(HostDoorTest, RegistrationRefusesTheReservedTypesTheSharedTableNames) {
    const std::string path =
        std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/host_processor/reserved_type_cases.json";
    std::ifstream file(path);
    const auto cases = nlohmann::json::parse(file).at("cases");
    // A floor: an empty table would pass every assertion below.
    ASSERT_GE(cases.size(), 10u) << "the table was not read from " << path;
    for (const auto &row : cases) {
        const auto name = row.at("type").get<std::string>();
        const bool reserved = row.at("reserved").get<bool>();
        auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

        bool sendRefused = false;
        try {
            sm->registerEventProcessor(name, [](const HostSendRequest &) { return std::vector<HostSendResponse>{}; });
        } catch (const std::invalid_argument &) {
            sendRefused = true;
        }
        EXPECT_EQ(sendRefused, reserved) << "registerEventProcessor(\"" << name << "\")";
        EXPECT_EQ(sm->hasEventProcessor(name), !reserved) << name;

        bool invokeRefused = false;
        try {
            sm->registerInvoker(name, [](const HostInvokeEvent &) { return std::nullopt; });
        } catch (const std::invalid_argument &) {
            invokeRefused = true;
        }
        EXPECT_EQ(invokeRefused, reserved) << "registerInvoker(\"" << name << "\")";
        EXPECT_EQ(sm->hasInvoker(name), !reserved) << name;
    }

    // SCE_MESH.md section 19: the router is installed by the call that names
    // itself one, which is the one way under the prefix.
    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    ASSERT_TRUE(SendHelper::isReservedHostType(std::string(MESH_RPC_INVOKE_TYPE)));
    EXPECT_NO_THROW(sm->registerMeshRpcInvoker([](const HostInvokeEvent &) { return std::nullopt; }));
    EXPECT_TRUE(sm->hasInvoker(std::string(MESH_RPC_INVOKE_TYPE)));
}

// A type nobody serves is error.execution for a send and for an invoke, as it
// was before the door, and a processor or invoker registered for another type
// does not serve it.
TEST_F(HostDoorTest, ATypeNobodyServesRaisesErrorExecution) {
    auto sm = prepare(R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="s" name="host_door_unserved">
  <datamodel>
    <data id="errors" expr="0"/>
    <data id="seen" expr="''"/>
  </datamodel>
  <state id="s">
    <onentry>
      <send type="x-sce-host" event="host.note"/>
    </onentry>
    <invoke id="job" type="x-sce-host"/>
    <transition event="error.execution">
      <assign location="errors" expr="errors + 1"/>
      <assign location="seen" expr="seen + _event.data + ';'"/>
    </transition>
  </state>
</scxml>)");
    sm->registerEventProcessor("x-some-other-host", recordingProcessor([](const HostSendRequest &) {
                                   return std::vector<HostSendResponse>{};
                               }));
    sm->registerInvoker("x-some-other-host", recordingInvoker(invoker_));
    ASSERT_TRUE(sm->start());

    EXPECT_EQ(read("errors"), "2") << "seen: " << read("seen");
    EXPECT_NE(read("seen").find("<invoke type='x-sce-host'> names an external service this platform does not support"),
              std::string::npos)
        << read("seen");
    EXPECT_TRUE(sent().empty()) << "a processor registered for another type served this one";
    EXPECT_TRUE(invoker_.lines().empty()) << "an invoker registered for another type was asked to start this one";
}

// The host's processor takes the whole send: it is handed the request and the
// events it answers arrive on the external queue in the order it wrote them.
TEST_F(HostDoorTest, AServedSendHandsTheHostItsRequestAndItsRepliesArriveInOrder) {
    auto sm = prepare(R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="s" name="host_door_send">
  <datamodel>
    <data id="n" expr="7"/>
    <data id="replies" expr="''"/>
  </datamodel>
  <state id="s">
    <onentry>
      <send id="note" type="x-sce-host" event="host.note" target="pane://note" namelist="n">
        <param name="k" expr="'v'"/>
      </send>
    </onentry>
    <transition event="reply.one">
      <assign location="replies" expr="replies + 'one,'"/>
    </transition>
    <transition event="reply.two" target="done">
      <assign location="replies" expr="replies + 'two'"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>)");
    sm->registerEventProcessor(DECLARED_TYPE, recordingProcessor([](const HostSendRequest &) {
                                   return std::vector<HostSendResponse>{{"reply.one", ""}, {"reply.two", ""}};
                               }));
    ASSERT_TRUE(sm->start());

    const auto seen = sent();
    ASSERT_EQ(seen.size(), 1u) << "the processor was not handed the send exactly once";
    EXPECT_EQ(seen[0].processorType, DECLARED_TYPE);
    EXPECT_EQ(seen[0].eventName, "host.note");
    EXPECT_EQ(seen[0].target, "pane://note");
    EXPECT_EQ(seen[0].sendId, "note");
    EXPECT_EQ(seen[0].params.at("k"), std::vector<std::string>{"v"});
    EXPECT_EQ(seen[0].params.at("n"), std::vector<std::string>{"7"});
    EXPECT_EQ(seen[0].eventData, R"({"k":"v","n":7})");
    EXPECT_EQ(seen[0].invokeId, "");
    EXPECT_EQ(sm->terminalState().value_or(""), "done") << "the replies never arrived";
    EXPECT_EQ(read("replies"), "one,two") << "the replies arrived out of the order the processor wrote them in";
}

// A delay is the send's: the processor is handed the send when it elapses, and
// a <cancel> of its id reaches it while it waits, so the processor never sees it.
TEST_F(HostDoorTest, ADelayedHostSendIsPerformedWhenDueAndACancelReachesIt) {
    auto sm = prepare(R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="s" name="host_door_delayed_send">
  <state id="s">
    <onentry>
      <send id="later" type="x-sce-host" event="host.later" delay="50ms"/>
      <send id="never" type="x-sce-host" event="host.never" delay="300ms"/>
      <cancel sendid="never"/>
    </onentry>
    <transition event="reply.later" target="done"/>
  </state>
  <final id="done"/>
</scxml>)");
    sm->registerEventProcessor(DECLARED_TYPE, recordingProcessor([](const HostSendRequest &request) {
                                   // `host.<x>` is answered with `reply.<x>`.
                                   return std::vector<HostSendResponse>{{"reply." + request.eventName.substr(5), ""}};
                               }));
    const auto startedAt = std::chrono::steady_clock::now();
    ASSERT_TRUE(sm->start());
    EXPECT_TRUE(sent().empty()) << "a delayed send was performed before start() returned";

    EXPECT_EQ(terminalWithin(), "done") << "the delayed send was never performed";
    EXPECT_GE(std::chrono::steady_clock::now() - startedAt, std::chrono::milliseconds(40));
    // Past the cancelled send's delay: it must still not have been performed.
    std::this_thread::sleep_for(std::chrono::milliseconds(400));
    std::vector<std::string> events;
    for (const auto &request : sent()) {
        events.push_back(request.eventName);
    }
    EXPECT_EQ(events, std::vector<std::string>{"host.later"}) << "the cancelled send reached the processor";
}

// An invoker that answers the start with its completion ends the run through the
// door a later completion takes: done.invoke.<id>, naming the invocation, with
// the data the host gave.
TEST_F(HostDoorTest, AnInvokerThatCompletesAtOnceEndsTheRunWithDoneInvoke) {
    auto sm = prepare(LIFECYCLE);
    sm->registerInvoker(DECLARED_TYPE, recordingInvoker(invoker_, [] {
                            HostInvokeResponse response;
                            response.doneData = R"({"answer":42})";
                            return std::optional<HostInvokeResponse>(response);
                        }));
    ASSERT_TRUE(sm->start());

    EXPECT_EQ(sm->terminalState().value_or(""), "completed") << "done.invoke.job never reached the document";
    EXPECT_EQ(read("ended"), R"({"answer":42})");
    const auto request = invoker_.latest("job");
    ASSERT_TRUE(request.has_value()) << "the invoker was never asked to start `job`";
    EXPECT_EQ(request->processorType, DECLARED_TYPE);
    EXPECT_EQ(request->src, "pane://job");
    EXPECT_EQ(request->sessionId, sm->getSessionId()) << "the request does not name the session that made it";
    // The run is over, so a completion now is refused and the exit cancels nothing.
    EXPECT_FALSE(sm->completeHostInvoke(DECLARED_TYPE, "job", request->token, "{}"));
    EXPECT_FALSE(invoker_.saw("CANCEL job " + std::to_string(request->token)));
}

// The ordinary case: the work outlives the start, and the host completes it later
// with the token the start carried. One completion ends it; a second is refused.
TEST_F(HostDoorTest, ALaterCompletionEndsTheRunOnce) {
    auto sm = prepare(LIFECYCLE);
    sm->registerInvoker(DECLARED_TYPE, recordingInvoker(invoker_));
    ASSERT_TRUE(sm->start());
    ASSERT_TRUE(sm->isStateActive("running"));
    const auto request = invoker_.latest("job");
    ASSERT_TRUE(request.has_value()) << "the invoker was never asked to start `job`";

    EXPECT_FALSE(sm->completeHostInvoke(DECLARED_TYPE, "job", request->token + 1, "{}"))
        << "a completion with another start's token was accepted";
    EXPECT_TRUE(sm->isStateActive("running"));

    EXPECT_TRUE(sm->completeHostInvoke(DECLARED_TYPE, "job", request->token, R"({"answer":7})"));
    EXPECT_EQ(sm->terminalState().value_or(""), "completed");
    EXPECT_EQ(read("ended"), R"({"answer":7})");
    EXPECT_FALSE(sm->completeHostInvoke(DECLARED_TYPE, "job", request->token, "{}")) << "a run was completed twice";
}

// A failure takes the same door: error.invoke.<id>, with the data, origin and
// origin type the host named, and the run is over.
TEST_F(HostDoorTest, AFailureRaisesErrorInvokeNamingTheOriginTheHostGave) {
    auto sm = prepare(LIFECYCLE);
    sm->registerInvoker(DECLARED_TYPE, recordingInvoker(invoker_));
    ASSERT_TRUE(sm->start());
    const auto request = invoker_.latest("job");
    ASSERT_TRUE(request.has_value());

    EXPECT_TRUE(sm->failHostInvoke(DECLARED_TYPE, "job", request->token, R"("no")", "host://job", DECLARED_TYPE));
    EXPECT_EQ(sm->terminalState().value_or(""), "failed") << "error.invoke.job never reached the document";
    EXPECT_EQ(read("ended"), "no|host://job|x-sce-host");
    EXPECT_FALSE(sm->completeHostInvoke(DECLARED_TYPE, "job", request->token, "{}"))
        << "a completion after the failure was accepted";
}

// The state's exit stops the run: the host is told, with the start's token. The
// state entered again starts the invocation again under a new token, and an
// ending of the first run is refused while one of the second is taken.
TEST_F(HostDoorTest, TheStateExitCancelsTheRunAndAStaleTokenIsRefused) {
    auto sm = prepare(LIFECYCLE);
    sm->registerInvoker(DECLARED_TYPE, recordingInvoker(invoker_));
    ASSERT_TRUE(sm->start());
    const auto first = invoker_.latest("job");
    ASSERT_TRUE(first.has_value());

    sm->processEvent("leave");
    EXPECT_TRUE(sm->isStateActive("away"));
    EXPECT_TRUE(invoker_.saw("CANCEL job " + std::to_string(first->token))) << "the host was not told to stop `job`";

    sm->processEvent("back");
    const auto second = invoker_.latest("job");
    ASSERT_TRUE(second.has_value());
    EXPECT_NE(second->token, first->token) << "a start again carried the first start's token";

    EXPECT_FALSE(sm->completeHostInvoke(DECLARED_TYPE, "job", first->token, "{}"))
        << "the cancelled run's completion was accepted";
    EXPECT_TRUE(sm->isStateActive("running"));
    EXPECT_EQ(sm->completeHostInvokeIn(sm->getSessionId(), DECLARED_TYPE, "job", first->token),
              HostInvokeEndOutcome::NotRunning);
    EXPECT_EQ(sm->completeHostInvokeIn(sm->getSessionId(), DECLARED_TYPE, "job", second->token, R"({"answer":1})"),
              HostInvokeEndOutcome::Delivered);
    EXPECT_EQ(sm->terminalState().value_or(""), "completed");
}

// A deadline is the engine's: the host is not handed it, and once it passes with
// the run still going the host is told to stop and the document receives
// error.invoke.<id> with the data "deadline". A reply afterwards is refused. A
// deadline that is not a whole number of milliseconds starts nothing.
TEST_F(HostDoorTest, ADeadlinePassedEndsTheRunAndAReplyAfterwardsIsRefused) {
    auto sm = prepare(R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="timed" name="host_door_deadline">
  <datamodel>
    <data id="misdated" expr="0"/>
  </datamodel>
  <state id="timed">
    <invoke id="slow" type="x-sce-host">
      <param name="_sce_deadline_ms" expr="50"/>
    </invoke>
    <invoke id="undated" type="x-sce-host">
      <param name="_sce_deadline_ms" expr="'soon'"/>
    </invoke>
    <transition event="error.execution">
      <assign location="misdated" expr="misdated + 1"/>
    </transition>
    <transition event="error.invoke.slow" cond="_event.invokeid === 'slow' &amp;&amp; _event.data === 'deadline'"
                target="expired"/>
  </state>
  <final id="expired"/>
</scxml>)");
    sm->registerInvoker(DECLARED_TYPE, recordingInvoker(invoker_));
    ASSERT_TRUE(sm->start());
    const auto request = invoker_.latest("slow");
    ASSERT_TRUE(request.has_value()) << "`slow` never started";
    EXPECT_TRUE(request->params.find(std::string(HOST_INVOKE_DEADLINE_PARAM)) == request->params.end())
        << "the host was handed the deadline";
    EXPECT_FALSE(invoker_.latest("undated").has_value()) << "an invocation with an unreadable deadline was started";
    EXPECT_EQ(read("misdated"), "1");

    EXPECT_EQ(terminalWithin(), "expired") << "the deadline never ended `slow`";
    EXPECT_TRUE(invoker_.saw("CANCEL slow " + std::to_string(request->token)))
        << "the host was not told to stop the run its deadline ended";
    EXPECT_FALSE(sm->completeHostInvoke(DECLARED_TYPE, "slow", request->token, "{}"))
        << "a reply after the deadline was accepted";
}

// docs/adr/0084, D6: a Mesh request is served by the router the host registered,
// with the request SCE_MESH.md section 19 gives it, and ended by it. With no
// router it is a type nobody serves.
TEST_F(HostDoorTest, AMeshRequestReachesTheRouterAndWithoutOneRaisesErrorExecution) {
    const char *const document = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="asking" name="host_door_mesh">
  <datamodel>
    <data id="peer" expr="'#motor'"/>
  </datamodel>
  <state id="asking">
    <invoke id="ask" type="sce:mesh-rpc" srcexpr="peer">
      <param name="_mesh_event" expr="'service.request.force'"/>
      <param name="_mesh_deadline_ms" expr="250"/>
      <param name="force" expr="1 + 2"/>
    </invoke>
    <transition event="done.invoke.ask" cond="_event.invokeid === 'ask'" target="answered"/>
    <transition event="error.execution" target="refused"/>
  </state>
  <final id="answered"/>
  <final id="refused"/>
</scxml>)";

    {
        auto sm = prepare(document);
        sm->registerMeshRpcInvoker(recordingInvoker(invoker_));
        ASSERT_TRUE(sm->start());
        const auto request = invoker_.latest("ask");
        ASSERT_TRUE(request.has_value()) << "the router was never asked";
        EXPECT_EQ(request->processorType, std::string(MESH_RPC_INVOKE_TYPE));
        EXPECT_EQ(request->src, "#motor");
        EXPECT_EQ(request->params.at(std::string(MESH_EVENT_PARAM)), std::vector<std::string>{"service.request.force"});
        EXPECT_EQ(request->params.at(std::string(MESH_DEADLINE_PARAM)), std::vector<std::string>{"250"});
        EXPECT_EQ(request->eventData, R"({"force":3})") << "an envelope field reached the payload";
        EXPECT_TRUE(sm->isStateActive("asking"));

        EXPECT_TRUE(sm->completeHostInvoke(std::string(MESH_RPC_INVOKE_TYPE), "ask", request->token, "{}",
                                           "mesh://motor", std::string(MESH_RPC_INVOKE_TYPE)));
        EXPECT_EQ(sm->terminalState().value_or(""), "answered");
    }
    {
        auto sm = prepare(document);
        ASSERT_TRUE(sm->start());
        EXPECT_EQ(sm->terminalState().value_or(""), "refused") << "a Mesh request with no router was not refused";
    }
}

// docs/adr/0045: the invokers serve every session of the tree, and an ending is
// delivered to the session it names, here a child the root started, which holds
// the run in its own ledger.
TEST_F(HostDoorTest, AnEndingNamedByItsSessionReachesTheChildThatStartedIt) {
    auto sm = prepare(R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="parent" name="host_door_tree">
  <state id="parent">
    <invoke id="child" type="scxml">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="working">
          <state id="working">
            <invoke id="job" type="x-sce-host"/>
            <transition event="done.invoke.job" target="finished"/>
          </state>
          <final id="finished"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>
  <final id="done"/>
</scxml>)");
    sm->registerInvoker(DECLARED_TYPE, recordingInvoker(invoker_));
    ASSERT_TRUE(sm->start());

    const auto request = invoker_.latest("job");
    ASSERT_TRUE(request.has_value()) << "the child's invocation never reached the invoker its root registered";
    ASSERT_FALSE(request->sessionId.empty());
    EXPECT_NE(request->sessionId, sm->getSessionId()) << "the request names the root, not the child that made it";

    // The root holds no run of that id: the child does.
    EXPECT_FALSE(sm->completeHostInvoke(DECLARED_TYPE, "job", request->token, "{}"));
    EXPECT_EQ(sm->completeHostInvokeIn("no-such-session", DECLARED_TYPE, "job", request->token),
              HostInvokeEndOutcome::NoSuchSession);
    EXPECT_EQ(sm->completeHostInvokeIn(request->sessionId, DECLARED_TYPE, "job", request->token + 1),
              HostInvokeEndOutcome::NotRunning);
    EXPECT_EQ(sm->completeHostInvokeIn(request->sessionId, DECLARED_TYPE, "job", request->token),
              HostInvokeEndOutcome::Delivered);

    // The child finished, and its done.invoke waits on the root's queue for the
    // root's next turn.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && !sm->terminalState().has_value()) {
        raiser_->processQueuedEvents();
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    EXPECT_EQ(sm->terminalState().value_or(""), "done") << "the child never finished on the delivered completion";
}

}  // namespace SCE::Tests
