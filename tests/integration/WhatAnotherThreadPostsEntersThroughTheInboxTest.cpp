// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, D8, ARCHITECTURE.md "Engine Threading": the C++ Interpreter is
// confined to the thread running its macrosteps, and what another thread has for
// it enters through its inbox.
//
// The Interpreter runs no loop of a thread of its own: its owner is whichever
// thread is running its macrosteps, for as long as it runs them. Until this step a
// thread that found another one running the machine waited on a mutex for the
// whole macrostep, the scheduler's thread included, and a raise from another
// thread was put on the queue the running macrostep was reading. Now nothing
// waits: an input from a thread that finds the machine taken is posted, and the
// thread running it applies it at its next macrostep boundary by the operation a
// direct call makes. The cases hold that to its contract:
//
//   * posts from many threads while a macrostep runs are taken once each, in the
//     order the inbox took them, as a single-threaded replay of the same calls
//     takes them; and threads that race an idle machine keep each one's order;
//   * the external queue takes events in the order they arrived: a post made
//     while a macrostep runs is applied before an event that macrostep puts on
//     the queue after it, whether a `<send>` to itself or a host-run invocation's
//     `done.invoke`, and a delayed send the scheduler fires is one such post;
//   * a host-run invocation's ending posted from another thread raises what a
//     direct one raises, and is refused by the same token check;
//   * what is held when the run ends is applied as a direct call would be then,
//     and a post after the end is refused, answered and counted;
//   * an ending made on another thread for a child session, found through the
//     tree's directory, enters that session's queue ahead of an event its parent
//     sends it afterwards in the same macrostep (D2 as amended at step 10).
//
// The Interpreter has no owner-thread refusal for a host to meet, unlike the
// generated engine's debug check: every call that hands it an input is taken on
// the calling thread or posted, so no host call is made from the wrong thread.
// The owner-only steps inside it (`runMainEventLoop`, the drain) check the owner
// in debug builds, and every case below runs them.
//
// Fixture: inbox_probe.scxml, beside this file, the machine the generated
// engine's inbox test posts to; read here as written. `taken` is what the machine
// took, in order, as `ping:<data>;`, `done:<data>;` and `failed:<data>;`. The tree
// of the last case is `sce-build/tests/fixtures/static_datamodel/
// static_invoke_ending_order.scxml`, run as `sce-codegen lower` writes it.

#include "core/HostProcessor.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/INativeActionHost.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT must name the checkout: the fixture is read from it"
#endif

namespace SCE::Tests {

namespace {

/// The type the fixtures name for the host's invoker.
constexpr const char *TYPE = "x-sce-host";
/// The fixtures' host-run invocation.
constexpr const char *JOB = "job";

/// inbox_probe.scxml, which the generated engine's inbox test compiles.
std::string probePath() {
    return std::string(SCE_PROJECT_ROOT) + "/tests/integration/inbox_probe.scxml";
}

/// What a host's invoker saw, in call order: `start:<token>` and `cancel:<token>`.
/// Written on the thread running the machine and read by the case afterwards, and
/// guarded because a deadline or a cancel may arrive on another.
class InvokerLog {
public:
    void add(const std::string &line) {
        std::lock_guard<std::mutex> lock(mutex_);
        lines_.push_back(line);
    }

    std::vector<std::string> lines() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return lines_;
    }

private:
    mutable std::mutex mutex_;
    std::vector<std::string> lines_;
};

/// A latch one thread opens and another waits on, for a bounded time.
class Latch {
public:
    void open() {
        {
            std::lock_guard<std::mutex> lock(mutex_);
            opened_ = true;
        }
        changed_.notify_all();
    }

    /// Whether it was opened within @p within.
    bool waitFor(std::chrono::milliseconds within) {
        std::unique_lock<std::mutex> lock(mutex_);
        return changed_.wait_for(lock, within, [this] { return opened_; });
    }

private:
    std::mutex mutex_;
    std::condition_variable changed_;
    bool opened_ = false;
};

/// One call a producer made, with what the machine records if it takes it.
struct Call {
    /// 0 for `processEvent("ping", data)`, 1 for `completeHostInvoke`, 2 for `failHostInvoke`.
    int kind = 0;
    /// The event's data: a number, which `_event.data` reads back as written.
    std::string data;
    /// What `taken` gains if the machine takes it.
    std::string recorded;
};

/// What producer @p producer calls as its @p k th call: pings, and now and then an
/// ending of `job`, completed or failed. Each data names its producer and its
/// place, so the record says which call it came from.
Call callOf(int producer, int k) {
    const std::string data = std::to_string(producer * 1000 + k);
    switch (k % 7) {
    case 3:
        return Call{1, data, "done:" + data + ";"};
    case 5:
        return Call{2, data, "failed:" + data + ";"};
    default:
        return Call{0, data, "ping:" + data + ";"};
    }
}

/// Make @p call on @p sm, with @p token as the ending's token.
void make(StateMachine &sm, const Call &call, uint64_t token) {
    switch (call.kind) {
    case 1:
        (void)sm.completeHostInvoke(TYPE, JOB, token, call.data);
        break;
    case 2:
        (void)sm.failHostInvoke(TYPE, JOB, token, call.data);
        break;
    default:
        (void)sm.processEvent("ping", call.data);
        break;
    }
}

/// What the machine must take from @p calls, in their order: every ping, and of
/// the endings only the first, since it ends `job` and the token check refuses
/// the rest.
std::string expectedFrom(const std::vector<Call> &calls) {
    bool ended = false;
    std::string expected;
    for (const Call &call : calls) {
        if (call.kind != 0) {
            if (ended) {
                continue;
            }
            ended = true;
        }
        expected += call.recorded;
    }
    return expected;
}

/// The standard host type the child of `static_invoke_ending_order` asks for the time.
constexpr const char *CLOCK = "sce-host-clock-now";
/// A completion of the clock, as the child's `ClockNowResult` reads it.
constexpr const char *NOW = R"({"error":"","utc_ms":1700000000000,"monotonic_ms":5})";

#ifndef __EMSCRIPTEN__
/// `sce-codegen lower <fixture>`, as static_datamodel_lowered_test spawns it: the
/// lowered document on stdout, or empty when the command failed or refused.
std::string lowered(const std::string &fixture) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    EXPECT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    if (bin == nullptr) {
        return "";
    }
    const std::string command = std::string("\"") + bin + "\" lower \"" + fixture + "\" 2>/dev/null";
    FILE *pipe = popen(command.c_str(), "r");
    if (pipe == nullptr) {
        return "";
    }
    std::string document;
    char buffer[4096];
    while (std::fgets(buffer, sizeof(buffer), pipe) != nullptr) {
        document += buffer;
    }
    return pclose(pipe) == 0 ? document : std::string();
}
#endif

/// What the host of `static_invoke_ending_order` saw and did, shared with the
/// handlers the machine holds, which may run after the case has returned (a stop
/// cancels `clock`): the clock's starts, and what the post `beforeQuestion` made
/// was answered.
struct EndingOrderRecord {
    std::mutex mutex;
    std::vector<HostInvokeRequest> starts;
    std::optional<HostInvokeEndOutcome> posted;

    std::optional<HostInvokeRequest> latestStart() {
        std::lock_guard<std::mutex> lock(mutex);
        if (starts.empty()) {
            return std::nullopt;
        }
        return starts.back();
    }
};

/// The host of the parent of `static_invoke_ending_order`. `beforeQuestion`, which
/// the parent performs inside the macrostep that takes `ask`, before it sends
/// `question` to `worker`, ends the clock of `worker` from a thread it joins before
/// it returns, naming `worker`'s session to the root: the ending happens-before the
/// parent's send.
class EndingOrderHost : public INativeActionHost {
public:
    EndingOrderHost(std::weak_ptr<StateMachine> root, std::shared_ptr<EndingOrderRecord> record)
        : root_(std::move(root)), record_(std::move(record)) {}

    ActionRealization realization() const override {
        return ActionRealization::Performs;
    }

    bool performNativeAction(const std::string &name, const std::vector<ScriptValue> &) override {
        if (name != "beforeQuestion") {
            return false;
        }
        const std::shared_ptr<StateMachine> root = root_.lock();
        const std::optional<HostInvokeRequest> clock = record_->latestStart();
        if (!root || !clock.has_value()) {
            ADD_FAILURE() << "the parent took `ask` before `worker` asked its host for the time";
            return true;
        }
        std::optional<HostInvokeEndOutcome> outcome;
        std::thread host([&root, &clock, &outcome] {
            outcome = root->completeHostInvokeIn(clock->sessionId, CLOCK, "clock", clock->token, NOW);
        });
        host.join();
        std::lock_guard<std::mutex> lock(record_->mutex);
        record_->posted = outcome;
        return true;
    }

private:
    std::weak_ptr<StateMachine> root_;
    std::shared_ptr<EndingOrderRecord> record_;
};

class WhatAnotherThreadPostsEntersThroughTheInterpretersInbox : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        // A case's invoker and completion callback refer to the case's locals, which
        // are gone by now; a stop that cancels an invocation still running must not
        // reach them.
        for (const auto &machine : machines_) {
            machine->registerInvoker(TYPE, [](const HostInvokeEvent &) { return std::optional<HostInvokeResponse>(); });
            machine->setCompletionCallback(nullptr);
            machine->stop();
        }
        machines_.clear();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A machine with a scheduler, a raiser and a dispatcher of its own, loaded by
    /// @p load and not yet started. The scheduler's execution opens `fired_` once
    /// it has handed a delayed event to its target.
    std::shared_ptr<StateMachine> prepare(const std::function<bool(StateMachine &)> &load) {
        auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
        const std::shared_ptr<Latch> fired = fired_;
        auto scheduler = std::make_shared<EventSchedulerImpl>(
            [fired](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                bool sent = false;
                try {
                    sent = target->send(event).get().isSuccess;
                } catch (...) {
                    sent = false;
                }
                fired->open();
                return sent;
            });
        auto raiser = std::make_shared<EventRaiserImpl>();
        raiser->setScheduler(scheduler);
        sm->setEventRaiser(raiser);
        sm->setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(raiser)));
        EXPECT_TRUE(load(*sm)) << "the document did not load";
        schedulers_.push_back(scheduler);
        raisers_.push_back(raiser);
        machines_.push_back(sm);
        return sm;
    }

    std::shared_ptr<StateMachine> prepareProbe() {
        return prepare([](StateMachine &sm) { return sm.loadSCXML(probePath()); });
    }

    std::shared_ptr<StateMachine> prepareText(const std::string &scxml) {
        return prepare([scxml](StateMachine &sm) { return sm.loadSCXMLFromString(scxml); });
    }

    /// The text of `taken` in @p sm's data model.
    std::string taken(const StateMachine &sm) const {
        const auto result = engine_->evaluateExpression(sm.getSessionId(), "String(taken)").get();
        return result.isSuccess() ? result.getValue<std::string>() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<Latch> fired_ = std::make_shared<Latch>();
    std::vector<std::shared_ptr<EventSchedulerImpl>> schedulers_;
    std::vector<std::shared_ptr<EventRaiserImpl>> raisers_;
    std::vector<std::shared_ptr<StateMachine>> machines_;
};

}  // namespace

// Eight threads make sixty calls each while the machine's first macrostep is held
// open on the thread that started it: pings, completions and failures of `job`.
// Every call finds the machine taken, so every one is posted. The order the calls
// were made in is recorded under one lock with the call, which is the order the
// inbox took them in; the machine must take them in that order once the macrostep
// is done, and a machine of its own given the same calls one at a time, on one
// thread, must end where this one did.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInterpretersInbox,
       PostsFromManyThreadsDuringAMacrostepAreTakenAsASingleThreadedReplayTakesThem) {
    constexpr int PRODUCERS = 8;
    // 480 calls in all: several of the inbox's segments, none refused (the inbox
    // is unbounded).
    constexpr int PER_PRODUCER = 60;

    auto sm = prepareProbe();
    InvokerLog log;
    std::mutex order;
    std::vector<Call> called;
    bool producersRan = false;
    sm->registerInvoker(TYPE, [&](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (event.start.has_value()) {
            const uint64_t token = event.start->token;
            log.add("start:" + std::to_string(token));
            if (!producersRan) {
                producersRan = true;
                // On the thread running the macrostep that started `job`: the
                // producers find the machine taken for as long as this runs.
                std::vector<std::thread> producers;
                for (int producer = 0; producer < PRODUCERS; ++producer) {
                    producers.emplace_back([&, producer, token] {
                        for (int k = 0; k < PER_PRODUCER; ++k) {
                            const Call call = callOf(producer, k);
                            std::lock_guard<std::mutex> lock(order);
                            make(*sm, call, token);
                            called.push_back(call);
                        }
                    });
                }
                for (std::thread &producer : producers) {
                    producer.join();
                }
            }
        }
        if (event.cancel.has_value()) {
            log.add("cancel:" + std::to_string(event.cancel->token));
        }
        return std::nullopt;
    });
    ASSERT_TRUE(sm->start());

    ASSERT_TRUE(producersRan) << "`job` never started, so nothing was posted";
    ASSERT_EQ(called.size(), static_cast<std::size_t>(PRODUCERS * PER_PRODUCER));
    EXPECT_EQ(sm->refusedInboxPosts(), 0u) << "an open inbox refused a post";
    EXPECT_EQ(taken(*sm), expectedFrom(called))
        << "the machine did not take the posts once each in the order the inbox took them";
    // The first ending ended `job`, so the host was told of its start and nothing
    // else: no refused ending reached it, and no cancel followed the ending.
    const std::vector<std::string> lines = log.lines();
    ASSERT_EQ(lines.size(), 1u) << "the host saw more than the start of `job`";
    EXPECT_EQ(lines.front().rfind("start:", 0), 0u);

    // The same calls, one at a time on this thread, each a direct call.
    auto replay = prepareProbe();
    InvokerLog replayLog;
    uint64_t replayToken = 0;
    replay->registerInvoker(TYPE, [&](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (event.start.has_value()) {
            replayToken = event.start->token;
            replayLog.add("start:" + std::to_string(replayToken));
        }
        if (event.cancel.has_value()) {
            replayLog.add("cancel:" + std::to_string(event.cancel->token));
        }
        return std::nullopt;
    });
    ASSERT_TRUE(replay->start());
    for (const Call &call : called) {
        make(*replay, call, replayToken);
    }
    EXPECT_EQ(taken(*replay), taken(*sm)) << "the run differs from a single-threaded replay of its calls";
    EXPECT_EQ(replayLog.lines().size(), lines.size())
        << "the invocation differs from a single-threaded replay of the calls";
}

// Eight threads hand an idle machine sixty events each, with nothing holding it:
// each call either finds the machine free and runs it, or finds another thread
// running it and is posted. Whichever way each went, every event is taken once,
// and each thread's events are taken in the order that thread handed them over,
// a post included: a call that takes the machine applies what was posted before
// it ahead of its own event.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInterpretersInbox,
       ThreadsRacingAnIdleMachineHaveEachEventTakenOnceAndEachThreadsInOrder) {
    constexpr int PRODUCERS = 8;
    constexpr int PER_PRODUCER = 60;

    auto sm = prepareProbe();
    sm->registerInvoker(TYPE, [](const HostInvokeEvent &) { return std::optional<HostInvokeResponse>(); });
    ASSERT_TRUE(sm->start());

    std::atomic<bool> released{false};
    std::vector<std::thread> producers;
    for (int producer = 0; producer < PRODUCERS; ++producer) {
        producers.emplace_back([&, producer] {
            while (!released.load(std::memory_order_acquire)) {
                std::this_thread::yield();
            }
            for (int k = 0; k < PER_PRODUCER; ++k) {
                (void)sm->processEvent("ping", std::to_string(producer * 1000 + k));
            }
        });
    }
    released.store(true, std::memory_order_release);
    for (std::thread &producer : producers) {
        producer.join();
    }
    EXPECT_EQ(sm->refusedInboxPosts(), 0u) << "an open inbox refused a post";

    // `taken` is `ping:<producer * 1000 + k>;` once per event taken.
    const std::string record = taken(*sm);
    std::map<int, std::vector<int>> takenBy;
    std::size_t at = 0;
    std::size_t entries = 0;
    while (at < record.size()) {
        const std::size_t end = record.find(';', at);
        ASSERT_NE(end, std::string::npos) << "an entry of `taken` is not closed: " << record.substr(at);
        const std::string entry = record.substr(at, end - at);
        ASSERT_EQ(entry.rfind("ping:", 0), 0u) << "the machine took something that was not handed to it: " << entry;
        const int id = std::stoi(entry.substr(5));
        takenBy[id / 1000].push_back(id % 1000);
        ++entries;
        at = end + 1;
    }
    EXPECT_EQ(entries, static_cast<std::size_t>(PRODUCERS * PER_PRODUCER)) << "an event was lost or taken twice";
    for (int producer = 0; producer < PRODUCERS; ++producer) {
        std::vector<int> inOrder;
        for (int k = 0; k < PER_PRODUCER; ++k) {
            inOrder.push_back(k);
        }
        EXPECT_EQ(takenBy[producer], inOrder) << "producer " << producer << "'s events were not taken in its order";
    }
}

// A delayed `<send>` comes due on the scheduler's thread while the macrostep that
// armed it is held open on the thread that started the machine, and is posted.
// The hold is released with a completion, whose `done.invoke.hold` the macrostep
// then puts on the external queue itself. `tick` arrived first, so it is applied
// before that event joins the queue and is taken first, not left for the
// macrostep's boundary behind it. Relies on timing only to let the 10 ms delay
// pass while the hold waits, up to five seconds, for the scheduler to fire it.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInterpretersInbox,
       ADelayedSendTheSchedulerFiresDuringAMacrostepIsTakenBeforeWhatTheMacrostepQueuesAfter) {
    auto sm = prepareText(R"scxml(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="holding" name="inbox_delayed_send">
  <datamodel>
    <data id="taken" expr="''"/>
  </datamodel>
  <state id="holding">
    <onentry>
      <send event="tick" delay="10ms"/>
    </onentry>
    <invoke id="hold" type="x-sce-host"/>
    <transition event="tick">
      <assign location="taken" expr="taken + 'tick;'"/>
    </transition>
    <transition event="done.invoke.hold">
      <assign location="taken" expr="taken + 'done;'"/>
    </transition>
  </state>
</scxml>)scxml");
    const std::shared_ptr<Latch> fired = fired_;
    bool firedWhileHeld = false;
    sm->registerInvoker(TYPE, [&](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (!event.start.has_value()) {
            return std::nullopt;
        }
        // The macrostep that armed `tick` is still running on this thread.
        firedWhileHeld = fired->waitFor(std::chrono::seconds(5));
        HostInvokeResponse completed;
        completed.doneData = "{}";
        return completed;
    });
    ASSERT_TRUE(sm->start());

    ASSERT_TRUE(firedWhileHeld) << "the scheduler did not fire `tick` while the macrostep was held";
    EXPECT_EQ(taken(*sm), "tick;done;")
        << "the macrostep's own done.invoke overtook the scheduler's event that arrived before it";
    EXPECT_EQ(sm->refusedInboxPosts(), 0u);
}

// W3C test 201's shape, without the HTTP listener: inside a macrostep, an event
// arrives from another thread while the machine's own thread waits for it (here a
// host-served `<send>` whose processor posts from a thread it joins, as a BasicHTTP
// send waits for the listener that raises the event), and the macrostep then sends
// an event to itself. The posted event arrived first, so it is taken first: the
// machine applies what its inbox holds before it puts its own event on the
// external queue. The join is the only ordering; nothing sleeps.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInterpretersInbox, AnEventPostedBeforeTheMachinesOwnSendIsTakenFirst) {
    auto sm = prepareText(R"scxml(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="s" name="inbox_arrival_order">
  <datamodel>
    <data id="taken" expr="''"/>
  </datamodel>
  <state id="s">
    <onentry>
      <send type="x-sce-host" event="host.call" target="pane://call"/>
      <send event="own"/>
    </onentry>
    <transition event="posted">
      <assign location="taken" expr="taken + 'posted;'"/>
    </transition>
    <transition event="own">
      <assign location="taken" expr="taken + 'own;'"/>
    </transition>
  </state>
</scxml>)scxml");
    std::optional<StateMachine::TransitionResult> posted;
    sm->registerEventProcessor(TYPE, [&](const HostSendRequest &) {
        // On the thread running the macrostep the `<onentry>` belongs to.
        std::thread other([&] { posted = sm->processEvent("posted"); });
        other.join();
        return std::vector<HostSendResponse>{};
    });
    ASSERT_TRUE(sm->start());

    ASSERT_TRUE(posted.has_value()) << "the processor was never handed the send";
    EXPECT_FALSE(posted->success) << "an event posted during a macrostep was answered as taken by it";
    EXPECT_EQ(taken(*sm), "posted;own;") << "the machine's own send overtook an event that arrived before it";
    EXPECT_EQ(sm->refusedInboxPosts(), 0u);
}

// An ending of `job` made on another thread while the machine's macrostep runs is
// posted, and the thread running the machine applies it at the boundary by the
// call a direct one makes: a stale token is refused, the first ending of the
// running start is taken and raises its event, and an ending after it finds the
// start over. A call posted answers whether the inbox took it; the token is
// judged when it is applied. A machine given the same endings directly raises the
// same events.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInterpretersInbox,
       AnEndingPostedFromAnotherThreadRaisesWhatADirectOneRaises) {
    auto posted = prepareProbe();
    InvokerLog postedLog;
    bool stale = false;
    bool failed = false;
    bool late = false;
    std::optional<HostInvokeEndOutcome> namedLate;
    bool endingsRan = false;
    posted->registerInvoker(TYPE, [&](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (event.start.has_value()) {
            const uint64_t token = event.start->token;
            postedLog.add("start:" + std::to_string(token));
            if (!endingsRan) {
                endingsRan = true;
                std::thread host([&, token] {
                    stale = posted->completeHostInvoke(TYPE, JOB, token + 1, "7");
                    failed = posted->failHostInvoke(TYPE, JOB, token, "8");
                    late = posted->completeHostInvoke(TYPE, JOB, token, "9");
                    namedLate = posted->completeHostInvokeIn(posted->getSessionId(), TYPE, JOB, token, "10");
                });
                host.join();
            }
        }
        if (event.cancel.has_value()) {
            postedLog.add("cancel:" + std::to_string(event.cancel->token));
        }
        return std::nullopt;
    });
    ASSERT_TRUE(posted->start());

    ASSERT_TRUE(endingsRan);
    EXPECT_TRUE(stale) << "the inbox refused a post; the token is the owner's to judge";
    EXPECT_TRUE(failed);
    EXPECT_TRUE(late);
    EXPECT_EQ(namedLate, std::optional<HostInvokeEndOutcome>(HostInvokeEndOutcome::Delivered))
        << "an ending posted to the session it names did not answer that the inbox took it";
    EXPECT_EQ(taken(*posted), "failed:8;") << "the posted endings did not raise what the token check allows";

    auto direct = prepareProbe();
    InvokerLog directLog;
    uint64_t directToken = 0;
    direct->registerInvoker(TYPE, [&](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (event.start.has_value()) {
            directToken = event.start->token;
            directLog.add("start:" + std::to_string(directToken));
        }
        if (event.cancel.has_value()) {
            directLog.add("cancel:" + std::to_string(event.cancel->token));
        }
        return std::nullopt;
    });
    ASSERT_TRUE(direct->start());
    EXPECT_FALSE(direct->completeHostInvoke(TYPE, JOB, directToken + 1, "7"));
    EXPECT_TRUE(direct->failHostInvoke(TYPE, JOB, directToken, "8"));
    EXPECT_FALSE(direct->completeHostInvoke(TYPE, JOB, directToken, "9"));
    EXPECT_EQ(direct->completeHostInvokeIn(direct->getSessionId(), TYPE, JOB, directToken, "10"),
              HostInvokeEndOutcome::NotRunning);

    EXPECT_EQ(taken(*direct), taken(*posted)) << "a posted ending raised what a direct one does not";
    EXPECT_EQ(directLog.lines().size(), postedLog.lines().size())
        << "the host was told of a posted ending's invocation what a direct one does not tell it";
}

// The run ends at a top-level final. An event and an ending posted from another
// thread during the macrostep that reaches it, while the state's exit cancels
// `job`, are held when the run ends, and applied then as a direct call would be:
// the event finds the machine stopped and is counted as unseen, and the ending
// finds `job` cancelled and raises nothing. The inbox closes with the run, so a
// post made after it, from the completion callback that runs on the thread
// still holding the machine, is refused, answered, and counted twice: as a
// refused post and as an event never seen.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInterpretersInbox,
       WhatIsHeldWhenTheRunEndsIsAppliedThenAndAPostAfterItIsRefused) {
    auto sm = prepareText(R"scxml(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="working" name="inbox_run_ends">
  <state id="working">
    <invoke id="job" type="x-sce-host"/>
    <transition event="finish" target="end"/>
  </state>
  <final id="end"/>
</scxml>)scxml");
    InvokerLog log;
    std::optional<StateMachine::TransitionResult> held;
    bool heldEnding = false;
    sm->registerInvoker(TYPE, [&](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (event.start.has_value()) {
            log.add("start:" + std::to_string(event.start->token));
        }
        if (event.cancel.has_value()) {
            const uint64_t token = event.cancel->token;
            log.add("cancel:" + std::to_string(token));
            // The exit of `working`, in the macrostep `finish` started, which runs
            // on the thread that handed `finish` over.
            std::thread host([&, token] {
                held = sm->processEvent("held");
                heldEnding = sm->completeHostInvoke(TYPE, JOB, token, "{}");
            });
            host.join();
        }
        return std::nullopt;
    });
    std::optional<StateMachine::TransitionResult> late;
    sm->setCompletionCallback([&] {
        std::thread host([&] { late = sm->processEvent("late"); });
        host.join();
    });
    ASSERT_TRUE(sm->start());
    const uint32_t unseenBefore = sm->getStatistics().unseenExternalEvents;

    EXPECT_TRUE(sm->processEvent("finish").success);
    EXPECT_EQ(sm->terminalState().value_or(""), "end");

    ASSERT_TRUE(held.has_value()) << "`job` was never cancelled, so nothing was posted";
    EXPECT_FALSE(held->success) << "an event posted during a macrostep was answered as taken by it";
    EXPECT_TRUE(heldEnding) << "the inbox of a running machine refused an ending";
    ASSERT_TRUE(late.has_value()) << "the completion callback never ran";
    EXPECT_FALSE(late->success) << "an event posted after the run ended was taken";

    const StateMachine::Statistics stats = sm->getStatistics();
    EXPECT_EQ(stats.unseenExternalEvents, unseenBefore + 2)
        << "the event held when the run ended, or the one posted after it, was not counted as unseen";
    EXPECT_EQ(stats.lastUnseenEventName, "late");
    EXPECT_EQ(sm->refusedInboxPosts(), 1u) << "the post after the end was not refused, or a held one was";
    // The ending held at the end found `job` cancelled: the host was told of its
    // start and its cancel, and of nothing after.
    const std::vector<std::string> lines = log.lines();
    ASSERT_EQ(lines.size(), 2u);
    EXPECT_EQ(lines[0].rfind("start:", 0), 0u);
    EXPECT_EQ(lines[1].rfind("cancel:", 0), 0u);
}

// A stop closes the inbox with the run, and a run restored running from a step
// that recorded it so takes posts again, as a started one does: an event posted
// from another thread while the restored machine runs a macrostep is taken, not
// refused as one posted after the end would be.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInterpretersInbox, ARunRestoredRunningTakesPostsAsAStartedOneDoes) {
    auto sm = prepareProbe();
    int starts = 0;
    std::optional<StateMachine::TransitionResult> posted;
    sm->registerInvoker(TYPE, [&](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (event.start.has_value() && ++starts == 2) {
            // The macrostep `go` started, on the restored run, is running here.
            std::thread host([&] { posted = sm->processEvent("ping", "1"); });
            host.join();
        }
        return std::nullopt;
    });
    ASSERT_TRUE(sm->start());
    sm->stop();
    ASSERT_TRUE(sm->restoreFromSnapshot({"running", "working"}, /*running=*/true, std::nullopt));

    sm->processEvent("leave");
    sm->processEvent("go");

    ASSERT_EQ(starts, 2) << "entering `working` again did not start `job` again";
    ASSERT_TRUE(posted.has_value());
    EXPECT_FALSE(posted->success) << "an event posted during a macrostep was answered as taken by it";
    EXPECT_EQ(sm->refusedInboxPosts(), 0u) << "the restored run's inbox was still closed";
    EXPECT_EQ(taken(*sm), "ping:1;") << "the event posted to the restored run was not taken";
}

// D2 as amended at step 10: an ending made on another thread for a child session
// is handed to that session when it is made, found in the tree's directory rather
// than by walking the tree from the calling thread. Inside the macrostep that takes
// `ask`, the host ends `worker`'s `clock`, naming `worker`, and the parent then
// sends `question` to `worker`. The ending came first, so `worker` takes
// `done.invoke.clock` first and the parent hears `stamped` (1) before `answer` (2).
// A child session runs on threads other than the case's, so the case waits, for a
// bounded time, for what it reads.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInterpretersInbox,
       AChildsEndingMadeBeforeItsParentsSendIsTakenBeforeTheSend) {
#ifdef __EMSCRIPTEN__
    GTEST_SKIP() << "the case spawns `sce-codegen lower`, which a WASM build cannot";
#else
    const std::string document = lowered(std::string(SCE_PROJECT_ROOT) +
                                         "/sce-build/tests/fixtures/static_datamodel/static_invoke_ending_order.scxml");
    ASSERT_FALSE(document.empty()) << "`sce-codegen lower` did not lower static_invoke_ending_order";

    auto sm = prepareText(document);
    const auto record = std::make_shared<EndingOrderRecord>();
    sm->registerInvoker(CLOCK, [record](const HostInvokeEvent &event) -> std::optional<HostInvokeResponse> {
        if (event.start.has_value()) {
            std::lock_guard<std::mutex> lock(record->mutex);
            record->starts.push_back(*event.start);
        }
        return std::nullopt;
    });
    sm->setNativeActionHost(std::make_shared<EndingOrderHost>(sm, record));

    const auto valueOf = [this, &sm](const std::string &variable) {
        const auto result = engine_->evaluateExpression(sm->getSessionId(), "String(" + variable + ")").get();
        return result.isSuccess() ? result.getValue<std::string>() : std::string("<unreadable>");
    };
    const auto settles = [](const std::function<bool()> &done) {
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
        while (std::chrono::steady_clock::now() < deadline) {
            if (done()) {
                return true;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        return done();
    };

    ASSERT_TRUE(sm->start());
    ASSERT_TRUE(settles([&record] { return record->latestStart().has_value(); }))
        << "`worker` never asked its host for the time";

    sm->processEvent("ask", "");
    ASSERT_TRUE(settles([&valueOf] { return valueOf("stamped_at") != "0" && valueOf("answer_at") != "0"; }))
        << "the parent did not hear both `stamped` and `answer`: stamped_at is " << valueOf("stamped_at")
        << " and answer_at " << valueOf("answer_at");
    {
        std::lock_guard<std::mutex> lock(record->mutex);
        ASSERT_EQ(record->posted, std::optional<HostInvokeEndOutcome>(HostInvokeEndOutcome::Delivered))
            << "the parent never performed `beforeQuestion`, or `worker` did not take the ending made for it";
    }
    EXPECT_EQ(valueOf("stamped_at"), "1")
        << "`worker` did not take the ending of `clock` first, though it was made before the parent's `question`";
    EXPECT_EQ(valueOf("answer_at"), "2")
        << "`worker` answered the parent's `question` before it took the ending made ahead of it";
#endif
}

}  // namespace SCE::Tests
