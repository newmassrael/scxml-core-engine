// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0083, ARCHITECTURE.md "Engine Threading": the generated C++ engine is
// confined to its owner, and what another thread has for it enters through its
// inbox.
//
// The defect this repays: a host-run invocation's ending made on the host's own
// thread checked the token and erased the engine's running-invocation table
// beside the thread running a macrostep, and SCE's own Mesh deadline raised onto
// the external queue from the scheduler's thread, guarded (when built with
// SCE_THREAD_SAFE) on the queue alone. The cases hold the replacement to its
// contract:
//
//   * many threads post endings and external events while the owner runs
//     macrosteps, and what the machine takes, and what its host is told, are
//     those of a single-threaded replay of the same posts;
//   * an ending posted for a start that was cancelled or started again is
//     refused by the token check, as a direct one is;
//   * a post is taken before the run starts and refused once it is stopped;
//   * an owner-thread operation called from another thread is refused with an
//     error that names the inbox, and changes nothing (debug builds).
//
// Fixture: inbox_probe.scxml, beside this file. `taken` is what the machine took,
// in order, as `ping:<data>;`, `done:<data>;` and `failed:<data>;`.

#include "inbox_probe_sm.h"

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <functional>
#include <gtest/gtest.h>
#include <memory>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <string>
#include <thread>
#include <utility>
#include <variant>
#include <vector>

#include "common/ForwardedEvent.h"
#include "core/HostProcessor.h"
#include "scripting/JSEngine.h"
#include "scripting/ScriptEngineProvider.h"

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::inbox_probe::inbox_probe;
using Event = SCE::Generated::inbox_probe::Event;

/// The type the fixture was compiled for (`HOST_INVOKER` in tests/CMakeLists.txt).
constexpr const char *TYPE = "x-sce-host";
/// The fixture's one host-run invocation.
constexpr const char *JOB = "job";

/// `text` as the JSON string the machine reads back as `text` in `_event.data`.
std::string quoted(const std::string &text) {
    return "\"" + text + "\"";
}

/// The host's half: records `start:<token>` and `cancel:<token>` in the order the
/// engine asked, and runs the work past the call (it answers nothing on start).
/// Called only on the owner, which is the thread the cases read it from.
struct Host {
    std::vector<std::string> log;

    void serve(Machine &sm) {
        sm.registerInvoker(TYPE, [this](const SCE::HostInvokeEvent &event) {
            if (event.start.has_value()) {
                log.push_back("start:" + std::to_string(event.start->token));
            }
            if (event.cancel.has_value()) {
                log.push_back("cancel:" + std::to_string(event.cancel->token));
            }
            return std::optional<SCE::HostInvokeResponse>();
        });
    }

    /// The token of the latest start.
    uint64_t latestToken() const {
        for (auto it = log.rbegin(); it != log.rend(); ++it) {
            if (it->rfind("start:", 0) == 0) {
                return std::stoull(it->substr(6));
            }
        }
        ADD_FAILURE() << "`job` never started";
        return 0;
    }
};

/// A datamodel that can evaluate, entered and driven once on the calling thread,
/// which makes that thread the owner.
void boot(Machine &sm) {
    sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                           [](SCE::IScriptEngine *) {}));
    sm.initialize();
    sm.step();
}

/// An ending of start `token` of `job`: a completion when `done`, a failure
/// otherwise, with `data` as `_event.data`.
Machine::InboxInput ending(uint64_t token, bool done, const std::string &data) {
    return Machine::InboxHostInvokeEnding{/*sessionId=*/"",
                                          /*ending=*/SCE::HostInvokeEnding{/*processorType=*/TYPE,
                                                                           /*invokeId=*/JOB,
                                                                           /*token=*/token,
                                                                           /*done=*/done,
                                                                           /*data=*/data,
                                                                           /*origin=*/"",
                                                                           /*originType=*/""}};
}

/// `ping` with `data`, as the document's own event.
Machine::InboxInput ping(const std::string &data) {
    return Machine::InboxExternalEvent{Machine::EventWithMetadata(Event::Ping, data)};
}

/// `ping` with `data`, named by text, as a peer or a host that knows no enum names it.
Machine::InboxInput pingByName(const std::string &data) {
    SCE::Common::ForwardedEvent named;
    named.name = "ping";
    named.data = data;
    named.type = "external";
    return Machine::InboxExternalEventByName{named};
}

/// Post `input` from a thread of its own, which does not own `sm`; whether the inbox
/// took it. Joined before it returns, so the post happens-before what the caller does next.
bool postFromAnotherThread(Machine &sm, Machine::InboxInput input) {
    bool taken = false;
    std::thread poster([&sm, &taken, &input] { taken = sm.inbox().post(std::move(input)); });
    poster.join();
    return taken;
}

/// One post of the run below, with what the machine records if it takes it.
struct Post {
    Machine::InboxInput input;
    std::string recorded;
    bool isEnding = false;
};

/// What producer `producer` posts as its `k`th input: external events, typed and
/// by name, and now and then an ending of `token`, completed or failed. Each
/// data names its producer and its place, so the record says which post it came
/// from.
Post postOf(int producer, int k, uint64_t token) {
    const std::string label = "p" + std::to_string(producer) + "-" + std::to_string(k);
    switch (k % 7) {
    case 3:
        return Post{ending(token, true, quoted(label)), "done:" + label + ";", true};
    case 5:
        return Post{ending(token, false, quoted(label)), "failed:" + label + ";", true};
    case 1:
        return Post{pingByName(quoted(label)), "ping:" + label + ";", false};
    default:
        return Post{ping(quoted(label)), "ping:" + label + ";", false};
    }
}

/// What the machine must take from `posted`, in the order the inbox took it: every
/// event, and of the endings only the first, since it ends `job` and the token
/// check refuses the rest.
std::string expectedFrom(const std::vector<Post> &posted) {
    bool ended = false;
    std::string expected;
    for (const Post &post : posted) {
        if (post.isEnding) {
            if (ended) {
                continue;
            }
            ended = true;
        }
        expected += post.recorded;
    }
    return expected;
}

class WhatAnotherThreadPostsEntersThroughTheInbox : public ::testing::Test {
protected:
    void SetUp() override {
        SCE::JSEngine::instance().initialize();
    }

    void TearDown() override {
        SCE::JSEngine::instance().shutdown();
    }
};

}  // namespace

// Four threads post fifty inputs each while the owner, woken by the waker the host
// installed, drives the machine. The order the inbox took the posts in is recorded
// under one lock with the post, so it is the order of the inbox itself; the machine
// must take them in that order, and a machine of its own replaying the same posts
// on one thread must end where this one did.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox,
       PostsFromManyThreadsDuringMacrostepsAreTakenAsASingleThreadedReplayTakesThem) {
    constexpr int PRODUCERS = 4;
    // 200 posts in all: far below the inbox's capacity, so none is refused whatever
    // the scheduler does to the owner.
    constexpr int PER_PRODUCER = 50;

    Machine sm;
    Host host;
    host.serve(sm);
    std::mutex wakeMutex;
    std::condition_variable woken;
    std::size_t wakes = 0;
    sm.inbox().wakeWith([&wakeMutex, &woken, &wakes] {
        {
            std::lock_guard<std::mutex> lock(wakeMutex);
            ++wakes;
        }
        woken.notify_one();
    });
    boot(sm);
    const uint64_t token = host.latestToken();

    std::mutex order;
    std::vector<Post> posted;
    std::vector<std::string> refused;
    std::atomic<int> running{PRODUCERS};
    std::atomic<bool> released{false};
    std::vector<std::thread> producers;
    for (int producer = 0; producer < PRODUCERS; ++producer) {
        producers.emplace_back([&, producer] {
            while (!released.load(std::memory_order_acquire)) {
                std::this_thread::yield();
            }
            for (int k = 0; k < PER_PRODUCER; ++k) {
                Post post = postOf(producer, k, token);
                std::lock_guard<std::mutex> lock(order);
                if (sm.inbox().post(post.input)) {
                    posted.push_back(std::move(post));
                } else {
                    refused.push_back(post.recorded);
                }
            }
            running.fetch_sub(1, std::memory_order_acq_rel);
            {
                std::lock_guard<std::mutex> lock(wakeMutex);
            }
            woken.notify_one();
        });
    }
    released.store(true, std::memory_order_release);

    // The owner: asleep until a post wakes it, then a macrostep, for as long as any
    // producer runs. Bounded, so a lost wake shows as a slow case and not a hung one.
    std::size_t seen = 0;
    while (running.load(std::memory_order_acquire) > 0) {
        {
            std::unique_lock<std::mutex> lock(wakeMutex);
            woken.wait_for(lock, std::chrono::milliseconds(100),
                           [&] { return wakes != seen || running.load(std::memory_order_acquire) == 0; });
            seen = wakes;
        }
        sm.step();
    }
    for (std::thread &producer : producers) {
        producer.join();
    }
    // What the last producer posted after the owner's last macrostep.
    sm.step();

    ASSERT_TRUE(refused.empty()) << "an open inbox below its capacity refused " << refused.size() << " posts";
    ASSERT_EQ(posted.size(), static_cast<std::size_t>(PRODUCERS * PER_PRODUCER));
    {
        std::lock_guard<std::mutex> lock(wakeMutex);
        EXPECT_EQ(wakes, posted.size()) << "the waker was not called once for each post the inbox took";
    }
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>(expectedFrom(posted)))
        << "the machine did not take the posts in the order the inbox took them";
    // The first ending ended `job`, so the host was told of its start and nothing else.
    EXPECT_EQ(host.log, std::vector<std::string>{"start:" + std::to_string(token)})
        << "an ending was not applied, or a refused one was";

    Machine replay;
    Host replayHost;
    replayHost.serve(replay);
    boot(replay);
    for (const Post &post : posted) {
        ASSERT_TRUE(replay.inbox().post(post.input)) << "the replay's inbox refused a post";
    }
    replay.step();
    EXPECT_EQ(replay.getPolicy().taken(), sm.getPolicy().taken())
        << "the run differs from a single-threaded replay of its posts";
    EXPECT_EQ(replayHost.log, host.log) << "the invocation differs from a single-threaded replay of the posts";
}

// What a cancelled process sends is ignored, and a restarted `<invoke>` is a
// different process under the same id (the engine's token check). A posted
// ending is judged when the owner applies it, by the same check a direct one
// meets.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox, APostedEndingOfACancelledOrRestartedStartIsRefusedAsADirectOneIs) {
    Machine sm;
    Host host;
    host.serve(sm);
    boot(sm);
    const uint64_t first = host.latestToken();
    sm.processEvent(Event::Leave);
    sm.processEvent(Event::Go);
    const uint64_t second = host.latestToken();
    ASSERT_NE(first, second) << "a restart reused the first start's token";

    EXPECT_TRUE(postFromAnotherThread(sm, ending(first, true, quoted("stale"))))
        << "the inbox refused a post; the token is the owner's to judge";
    sm.step();
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>("")) << "the cancelled start's ending was taken";

    EXPECT_TRUE(postFromAnotherThread(sm, ending(second, true, quoted("fresh"))));
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>(""))
        << "a post was applied before the owner reached a macrostep boundary";
    sm.step();
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>("done:fresh;"));

    // Ended once, so a later ending of the same start finds nothing to end.
    EXPECT_TRUE(postFromAnotherThread(sm, ending(second, false, quoted("late"))));
    sm.step();
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>("done:fresh;")) << "the same start ended twice";
    EXPECT_EQ(host.log, (std::vector<std::string>{"start:" + std::to_string(first), "cancel:" + std::to_string(first),
                                                  "start:" + std::to_string(second)}));
}

// The inbox is open from the moment the engine is built, so what is posted
// before the run starts is applied at its first boundary; a run that was stopped
// takes no more, and the refusal is both answered and counted.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox, APostBeforeTheRunIsTakenByItAndOneAfterTheStopIsRefused) {
    Machine sm;
    Host host;
    host.serve(sm);
    EXPECT_TRUE(postFromAnotherThread(sm, ping(quoted("early")))) << "the inbox of an engine not yet run refused";
    boot(sm);
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>("ping:early;"))
        << "what was posted before the run was not taken by it";
    EXPECT_EQ(sm.refusedInboxPosts(), 0u);

    sm.stop();
    EXPECT_FALSE(postFromAnotherThread(sm, ping(quoted("late")))) << "a stopped engine's inbox took a post";
    EXPECT_EQ(sm.refusedInboxPosts(), 1u) << "the refusal was not counted";
}

// An owner-thread operation called from a thread that does not own the engine is
// refused before it changes anything, and the error names the inbox, the door
// that thread should have used. A debug build's check.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox, AnOwnerThreadOperationFromAnotherThreadIsRefusedAndNamesTheInbox) {
#ifdef NDEBUG
    GTEST_SKIP() << "the owner check is made in debug builds";
#else
    Machine sm;
    Host host;
    host.serve(sm);
    boot(sm);
    const uint64_t token = host.latestToken();

    const auto refusal = [](const std::function<void()> &call) {
        std::string what;
        std::thread other([&call, &what] {
            try {
                call();
            } catch (const std::logic_error &error) {
                what = error.what();
            }
        });
        other.join();
        return what;
    };
    const std::vector<std::pair<std::string, std::function<void()>>> calls{
        {"completeHostInvoke", [&] { sm.completeHostInvoke(TYPE, JOB, token, quoted("other")); }},
        {"failHostInvoke", [&] { sm.failHostInvoke(TYPE, JOB, token, quoted("other")); }},
        {"cancelHostInvoke", [&] { sm.cancelHostInvoke(TYPE, JOB); }},
        {"raiseExternal", [&] { sm.raiseExternal(Event::Ping, quoted("other")); }},
        {"step", [&] { sm.step(); }},
        {"tick", [&] { sm.tick(); }},
        {"processEvent", [&] { sm.processEvent(Event::Leave); }},
        {"stop", [&] { sm.stop(); }},
    };
    for (const auto &[name, call] : calls) {
        SCOPED_TRACE(name);
        const std::string what = refusal(call);
        EXPECT_NE(what.find(name), std::string::npos) << "not refused, or the refusal does not name the call: " << what;
        EXPECT_NE(what.find("inbox"), std::string::npos) << "the refusal does not name the inbox: " << what;
    }

    // Nothing the refused calls asked for happened: `job` is still running, the
    // machine is still in `working`, and the owner's own completion is taken.
    EXPECT_EQ(host.log, std::vector<std::string>{"start:" + std::to_string(token)})
        << "a refused call reached the host";
    EXPECT_TRUE(sm.completeHostInvoke(TYPE, JOB, token, quoted("owner")))
        << "a refused completion ended the invocation";
    sm.step();
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>("done:owner;"))
        << "a refused raise reached the queue, or the owner's completion did not";
#endif
}

}  // namespace SCE::Tests
