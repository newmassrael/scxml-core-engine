// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, ARCHITECTURE.md "Engine Threading": the generated C++ engine is
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
//   * the external queue takes events in the order they arrived: a post made
//     while a macrostep runs is applied before an event the owner puts on that
//     queue after it, whether a host-run invocation's `done.invoke` or the
//     host's own `raiseExternal` (W3C test 201's shape), and a host's
//     `processEvent` takes its event after what was posted before the call;
//   * a post is taken before the run starts and refused once it is stopped;
//   * an owner-thread operation called from another thread is refused with an
//     error that names the inbox, and changes nothing (debug builds);
//   * an ending posted for a child session enters that session's queue in the
//     order it was posted, ahead of an event its parent sends it afterwards in
//     the same macrostep (D2 as amended at step 10).
//
// Fixture: inbox_probe.scxml, beside this file. `taken` is what the machine took,
// in order, as `ping:<data>;`, `done:<data>;` and `failed:<data>;`. The tree of
// the last case is `sce-build/tests/fixtures/static_datamodel/
// static_invoke_ending_order.scxml`.

#include "inbox_probe_sm.h"
#include "static_invoke_ending_order_sm.h"

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
#include "core/EventMetadata.h"
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
    EXPECT_EQ(sm.initialize(), std::nullopt);
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
                                                                           /*origin=*/std::nullopt}};
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

/// The tree whose host posts a child's ending inside the parent's macrostep.
using OrderTree = SCE::Generated::static_invoke_ending_order::static_invoke_ending_order;
/// The standard host type `worker` asks for the time.
constexpr const char *CLOCK = "sce-host-clock-now";
/// A completion of the clock, as the child's `ClockNowResult` reads it.
constexpr const char *NOW = R"({"error":"","utc_ms":1700000000000,"monotonic_ms":5})";

/// The host of `static_invoke_ending_order`, all on the owner but for one post. Its
/// clock records each start and answers nothing, so `clock` keeps running.
/// `beforeQuestion`, which the parent performs inside the macrostep that takes
/// `ask`, before it sends `question` to `worker`, posts the clock's ending naming
/// `worker`'s session to the root's inbox, from a thread it joins before it
/// returns: the post happens-before the parent's send.
struct EndingOrderHost : SCE::Generated::static_invoke_ending_order::StaticInvokeEndingOrderActions {
    OrderTree *tree = nullptr;
    std::vector<SCE::HostInvokeRequest> starts;
    /// The token of each start the host was told to stop.
    std::vector<uint64_t> cancels;
    std::optional<bool> postTaken;

    void serve(OrderTree &machine) {
        tree = &machine;
        machine.registerInvoker(CLOCK, [this](const SCE::HostInvokeEvent &event) {
            if (event.start.has_value()) {
                starts.push_back(*event.start);
            }
            if (event.cancel.has_value()) {
                cancels.push_back(event.cancel->token);
            }
            return std::optional<SCE::HostInvokeResponse>();
        });
    }

    /// The ending of the latest start of `clock`, completed with `NOW`, naming the
    /// session that started it.
    OrderTree::InboxHostInvokeEnding clockEnding() const {
        const SCE::HostInvokeRequest &clock = starts.back();
        return OrderTree::InboxHostInvokeEnding{/*sessionId=*/clock.sessionId,
                                                /*ending=*/SCE::HostInvokeEnding{/*processorType=*/CLOCK,
                                                                                 /*invokeId=*/"clock",
                                                                                 /*token=*/clock.token,
                                                                                 /*done=*/true,
                                                                                 /*data=*/NOW,
                                                                                 /*origin=*/std::nullopt}};
    }

    void beforeQuestion() override {
        if (tree == nullptr || starts.empty()) {
            ADD_FAILURE() << "the parent took `ask` before `worker` asked its host for the time";
            return;
        }
        postTaken = postAside(clockEnding());
    }

    /// Post @p ending through the root's handle from a thread of its own, joined
    /// before it returns; whether it was taken.
    bool postAside(const OrderTree::InboxHostInvokeEnding &ending) const {
        const OrderTree::Inbox inbox = tree->inbox();
        bool taken = false;
        std::thread host([&inbox, &ending, &taken] { taken = inbox.post(ending); });
        host.join();
        return taken;
    }
};

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

// Eight threads post sixty inputs each while the owner, woken by the waker the host
// installed, drives the machine. The order the inbox took the posts in is recorded
// under one lock with the post, so it is the order of the inbox itself; the machine
// must take them in that order, and a machine of its own replaying the same posts
// on one thread must end where this one did.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox,
       PostsFromManyThreadsDuringMacrostepsAreTakenAsASingleThreadedReplayTakesThem) {
    constexpr int PRODUCERS = 8;
    // 480 posts in all: several of the queue's segments, so a run where the owner
    // falls behind grows the inbox and one where it keeps up retires segments, and
    // none is refused either way (the inbox is unbounded).
    constexpr int PER_PRODUCER = 60;

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

    ASSERT_TRUE(refused.empty()) << "an open inbox refused " << refused.size() << " posts";
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

// W3C test 201's shape, without the HTTP listener: inside the macrostep that
// enters `working`, `job`'s start reaches the host on the owner, and the host
// posts `ping` from a thread it joins before it answers, as a BasicHTTP send
// waits for the listener whose thread raises the event. The host then answers
// the start with a completion, whose `done.invoke.job` the engine puts on the
// external queue itself. `ping` arrived first, so it is taken first: the engine
// applies what its inbox holds before it puts its own event on the external
// queue, and not only at the macrostep's boundary. The join is the only
// ordering; nothing sleeps.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox, APostMadeBeforeTheEnginesOwnDoneInvokeIsTakenFirst) {
    Machine sm;
    std::optional<bool> postTaken;
    sm.registerInvoker(TYPE, [&sm, &postTaken](const SCE::HostInvokeEvent &event) {
        if (!event.start.has_value()) {
            return std::optional<SCE::HostInvokeResponse>();
        }
        postTaken = postFromAnotherThread(sm, ping(quoted("posted")));
        SCE::HostInvokeResponse completed;
        completed.doneData = quoted("job");
        return std::optional<SCE::HostInvokeResponse>(completed);
    });
    boot(sm);

    ASSERT_EQ(postTaken, std::optional<bool>(true)) << "`job` never started, or the inbox refused the post";
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>("ping:posted;done:job;"))
        << "the engine's own done.invoke overtook an event posted before it";
    EXPECT_EQ(sm.refusedInboxPosts(), 0u);
}

// The same order for an event the host raises itself on the owner, as the W3C
// HTTP harness does with what its listener did not post: posted during the same
// call before the host's own `raiseExternal`, `ping:posted` is taken before
// `ping:own`.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox, APostMadeBeforeTheHostsOwnRaiseOnTheOwnerIsTakenFirst) {
    Machine sm;
    std::optional<bool> postTaken;
    sm.registerInvoker(TYPE, [&sm, &postTaken](const SCE::HostInvokeEvent &event) {
        if (event.start.has_value()) {
            postTaken = postFromAnotherThread(sm, ping(quoted("posted")));
            sm.raiseExternal(Event::Ping, quoted("own"));
        }
        return std::optional<SCE::HostInvokeResponse>();
    });
    boot(sm);

    ASSERT_EQ(postTaken, std::optional<bool>(true)) << "`job` never started, or the inbox refused the post";
    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>("ping:posted;ping:own;"))
        << "the host's own raise overtook an event posted before it";
    EXPECT_EQ(sm.refusedInboxPosts(), 0u);
}

// A host's own call on the owner is an arrival too. `processEvent` hands its event
// straight to the machine when nothing is held; here `ping:posted` was posted, and
// the posting thread joined, before the host's call, so it arrived first and is
// taken first, and the host's event joins the external queue behind it rather than
// overtaking it.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox, APostMadeBeforeTheHostsProcessEventIsTakenFirst) {
    Machine sm;
    Host host;
    host.serve(sm);
    boot(sm);

    ASSERT_TRUE(postFromAnotherThread(sm, ping(quoted("posted"))));
    sm.processEvent(Event::Ping, SCE::Core::EventMetadata("ping", quoted("own")));

    EXPECT_EQ(sm.getPolicy().taken(), std::optional<std::string>("ping:posted;ping:own;"))
        << "the host's processEvent overtook an event posted before it";
    EXPECT_EQ(sm.refusedInboxPosts(), 0u);
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

// D2 as amended at step 10: an input that names a session is resolved when it is
// posted. Inside the macrostep that takes `ask`, the host posts the ending of
// `worker`'s `clock`, naming `worker`, and the parent then sends `question` to
// `worker`. The ending was posted first, so `worker` takes `done.invoke.clock`
// first and the parent hears `stamped` (1) before `answer` (2). An ending held in
// the root's inbox until the root's next boundary would reach `worker`'s queue
// behind `question`, and the parent would hear them the other way round.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox, AChildsEndingPostedBeforeItsParentsSendIsTakenBeforeTheSend) {
    EndingOrderHost host;
    OrderTree tree(host);
    host.serve(tree);
    EXPECT_EQ(tree.initialize(), std::nullopt);
    for (int round = 0; round < 8 && host.starts.empty(); ++round) {
        tree.tick();
    }
    ASSERT_EQ(host.starts.size(), 1u) << "`worker` never asked its host for the time";

    tree.raiseExternal("ask");
    for (int round = 0; round < 16 && (tree.stamped_at() == 0 || tree.answer_at() == 0); ++round) {
        tree.tick();
    }

    ASSERT_EQ(host.postTaken, std::optional<bool>(true))
        << "the parent never performed `beforeQuestion`, or the tree refused the ending it posted";
    EXPECT_EQ(tree.stamped_at(), 1u)
        << "`worker` did not take the ending of `clock` first, though it was posted before the parent's `question`";
    EXPECT_EQ(tree.answer_at(), 2u)
        << "`worker` answered the parent's `question` before it took the ending posted ahead of it";
}

// D2: a child session's run ends when its parent cancels it, as every run ends.
// The ending of `worker`'s `clock` is posted, naming `worker`, and before any drive
// applies it the root is stopped, which exits `working` and cancels `worker`. Its
// run is over, so `clock` is over: the host is told to stop it, and the ending held
// in `worker`'s inbox is applied then and refused by the token check, so nothing
// reaches the parent. `worker` has left the tree's directory, so an ending naming
// it afterwards is refused where it is posted.
TEST_F(WhatAnotherThreadPostsEntersThroughTheInbox, AnEndingHeldForAChildItsParentCancelsIsAppliedAsTheRunEnds) {
    EndingOrderHost host;
    OrderTree tree(host);
    host.serve(tree);
    EXPECT_EQ(tree.initialize(), std::nullopt);
    for (int round = 0; round < 8 && host.starts.empty(); ++round) {
        tree.tick();
    }
    ASSERT_EQ(host.starts.size(), 1u) << "`worker` never asked its host for the time";
    const uint64_t token = host.starts.front().token;

    ASSERT_TRUE(host.postAside(host.clockEnding())) << "`worker`'s inbox refused an ending while it ran";
    tree.stop();

    EXPECT_EQ(host.cancels, std::vector<uint64_t>{token})
        << "cancelling `worker` did not end its `clock` and tell the host to stop it";
    EXPECT_EQ(tree.stamped_at(), 0u) << "the ending held for the cancelled `worker` reached its parent";
    EXPECT_FALSE(host.postAside(host.clockEnding()))
        << "an ending naming `worker` was taken after its parent cancelled it";
}

}  // namespace SCE::Tests
