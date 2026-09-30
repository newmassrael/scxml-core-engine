// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.2: a delay is a property of the send, not of the
// processor it names — a delayed BasicHTTP send is POSTed when due, and
// <cancel> reaches it while it waits — C++ Interpreter.
//
// The other channels stand in for the HTTP transport with a recording callback,
// and this one does the same at the seam the Interpreter offers for it: the
// factory that turns a target URI into the object a send is handed to. Every
// `http://` target is answered by a recorder, everything else by the real
// factory, so what is observed is what the Interpreter decided — when a request
// was made, in what order, with which parameters, and that a cancelled one never
// was — and not what a socket did. No port is bound, so nothing here can meet
// the listener the C11 driver of this fixture owns.
//
// The clock is the machine's own wall clock, so only bounds that cannot be
// broken by a slow machine are asserted: a request is never made BEFORE its
// delay, and the sends written without a delay are made in the caller's own
// turn, on its thread, before start() returns.
//
// Fixture:
// integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "events/IEventTarget.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

/// One request the Interpreter handed to the BasicHTTP transport.
struct PostedRequest {
    std::string target;
    std::string eventName;
    std::string sendId;
    std::map<std::string, std::vector<std::string>> params;
    std::chrono::steady_clock::time_point at;
    /// The thread that handed it to the transport: a send made at once runs on
    /// the caller's, a queued one on the scheduler's.
    std::thread::id madeOn;
};

/// The transport: keeps what it was handed, in the order it was handed it.
class RecordingTransport {
public:
    void record(const EventDescriptor &event) {
        std::lock_guard<std::mutex> lock(mutex_);
        posted_.push_back({event.target, event.eventName, event.sendId, event.params, std::chrono::steady_clock::now(),
                           std::this_thread::get_id()});
    }

    std::vector<PostedRequest> posted() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return posted_;
    }

private:
    mutable std::mutex mutex_;
    std::vector<PostedRequest> posted_;
};

class RecordingHttpTarget : public IEventTarget {
public:
    explicit RecordingHttpTarget(RecordingTransport &transport) : transport_(transport) {}

    std::future<SendResult> send(const EventDescriptor &event) override {
        transport_.record(event);
        std::promise<SendResult> answered;
        SendResult result;
        result.isSuccess = true;
        result.sendId = event.sendId;
        answered.set_value(std::move(result));
        return answered.get_future();
    }

    std::string getTargetType() const override {
        return "http";
    }

    bool canHandle(const std::string &targetUri) const override {
        return targetUri.rfind("http://", 0) == 0 || targetUri.rfind("https://", 0) == 0;
    }

    std::vector<std::string> validate() const override {
        return {};
    }

    std::string getDebugInfo() const override {
        return "RecordingHttpTarget";
    }

private:
    RecordingTransport &transport_;
};

/// The real factory for everything but an http(s) target, which goes to the recorder.
class RecordingTransportFactory : public IEventTargetFactory {
public:
    RecordingTransportFactory(std::shared_ptr<IEventTargetFactory> real, RecordingTransport &transport)
        : real_(std::move(real)), transport_(transport) {}

    std::shared_ptr<IEventTarget> createTarget(const std::string &targetUri,
                                               const std::string &sessionId = "") override {
        if (targetUri.rfind("http://", 0) == 0 || targetUri.rfind("https://", 0) == 0) {
            return std::make_shared<RecordingHttpTarget>(transport_);
        }
        return real_->createTarget(targetUri, sessionId);
    }

    void registerTargetType(const std::string &scheme,
                            std::function<std::shared_ptr<IEventTarget>(const std::string &)> creator) override {
        real_->registerTargetType(scheme, std::move(creator));
    }

    bool isSchemeSupported(const std::string &scheme) const override {
        return real_->isSchemeSupported(scheme);
    }

    std::vector<std::string> getSupportedSchemes() const override {
        return real_->getSupportedSchemes();
    }

private:
    std::shared_ptr<IEventTargetFactory> real_;
    RecordingTransport &transport_;
};

std::string events(const std::vector<PostedRequest> &posted) {
    std::string out;
    for (const auto &request : posted) {
        out += (out.empty() ? "" : ",") + request.eventName;
    }
    return out;
}

}  // namespace

class ADelayedHttpSendIsPostedWhenDueTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(ADelayedHttpSendIsPostedWhenDueTest, ADelayedHttpSendIsPostedWhenDue) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/a_delayed_http_send_is_posted_when_due/"
                                "a_delayed_http_send_is_posted_when_due.scxml";

    RecordingTransport transport;
    sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
            try {
                return target->send(event).get().isSuccess;
            } catch (...) {
                return false;
            }
        });
    auto eventRaiser = std::make_shared<EventRaiserImpl>();
    eventRaiser->setScheduler(scheduler);
    eventRaiser->setImmediateMode(false);
    sm_->setEventRaiser(eventRaiser);
    sm_->setEventDispatcher(std::make_shared<EventDispatcherImpl>(
        scheduler,
        std::make_shared<RecordingTransportFactory>(std::make_shared<EventTargetFactoryImpl>(eventRaiser), transport)));

    ASSERT_TRUE(sm_->loadSCXML(fixture)) << "canonical fixture not loadable: " << fixture;
    const auto startedAt = std::chrono::steady_clock::now();
    ASSERT_TRUE(sm_->start());

    // A zero wait, written or evaluated, is no deferral: the sends written
    // without a delay, with `0s`, and with a `delayexpr` of `0ms` are already
    // made when start() returns — not left for a later turn of the scheduler.
    const auto atOnce = transport.posted();
    ASSERT_GE(atOnce.size(), 3u) << "made by the time start() returned: " << events(atOnce);
    EXPECT_EQ(atOnce[0].eventName, "now");
    EXPECT_EQ(atOnce[1].eventName, "zero");
    EXPECT_EQ(atOnce[2].eventName, "zeroexpr");
    // Made in the caller's own turn, and not handed to the scheduler: one
    // queued with no wait is made on the scheduler's thread within
    // microseconds, so the list read above cannot tell it from a send made
    // at once — the thread that made it can.
    for (std::size_t i = 0; i < 3; ++i) {
        EXPECT_EQ(atOnce[i].madeOn, std::this_thread::get_id())
            << "`" << atOnce[i].eventName << "` was queued for the scheduler, not made at once";
    }

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "the run must end in `done`";

    const auto posted = transport.posted();
    EXPECT_EQ(events(posted), "now,zero,zeroexpr,later,dynamic")
        << "each is made once and in order, and the cancelled one never is";
    ASSERT_EQ(posted.size(), 5u);

    EXPECT_EQ(posted[3].target, "http://127.0.0.1:18081/later");
    EXPECT_EQ(posted[3].sendId, "later");
    // Never BEFORE the delay; 10ms of slack under it for the millisecond
    // granularity the delay is counted in.
    EXPECT_GE(posted[3].at - startedAt, std::chrono::milliseconds(90)) << "`later` was made before its 100ms delay";

    EXPECT_EQ(posted[4].target, "http://127.0.0.1:18081/dynamic") << "a targetexpr is read when the send is made";
    EXPECT_GE(posted[4].at - startedAt, std::chrono::milliseconds(190)) << "`dynamic` was made before its 200ms delay";
    ASSERT_TRUE(posted[4].params.count("k") == 1) << "the <param> the send carried must reach the transport";
    EXPECT_EQ(posted[4].params.at("k"), std::vector<std::string>({"v"}));
}

}  // namespace Tests
}  // namespace SCE
