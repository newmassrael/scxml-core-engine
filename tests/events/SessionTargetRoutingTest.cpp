// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML C.1: `#_scxml_<sessionid>` names the session an event is
// delivered to.
//
// The corpus cannot reach this. Test 350 says so in its own comment — "A
// session should be able to send an event to itself using its own session
// ID as the target" — and test 336 likewise ("In this case it's the same
// session"). Both pass whatever the target session id says, because both
// send to the session they already are. Nothing in the W3C suite sends
// ACROSS sessions, so the routing the URI exists for is untested.
//
// The routing itself is not new machinery: `createExternalTarget` already
// resolves a session id to that session's `IEventRaiser` through
// `EventRaiserService`. What this fixture pins is that the id resolved is
// the one the URI NAMES rather than the one doing the sending.

#include "common/IOProcessorHelper.h"
#include "events/EventDescriptor.h"
#include "events/EventRaiserService.h"
#include "events/EventTargetFactoryImpl.h"
#include "mocks/MockEventRaiser.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateSnapshot.h"
#include "scripting/ScriptEngineProvider.h"
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <vector>

namespace {

using SCE::EventDescriptor;
using SCE::EventRaiserService;
using SCE::EventTargetFactoryImpl;
using SCE::Test::MockEventRaiser;

/// Two co-resident sessions, each with its own raiser, plus the factory's
/// own fallback raiser so "fell back" is distinguishable from "routed".
class SessionTargetRouting : public ::testing::Test {
protected:
    void SetUp() override {
        fallback_ = std::make_shared<MockEventRaiser>();
        senderRaiser_ = std::make_shared<MockEventRaiser>();
        peerRaiser_ = std::make_shared<MockEventRaiser>();

        // `EventRaiserService::registerEventRaiser` refuses a session the
        // script engine does not know — registration is DEFERRED, not
        // rejected loudly, so a fixture that skips this step registers
        // nothing and every routing assertion below fails for the wrong
        // reason. Two live sessions are what makes "the peer" a real
        // address rather than a string.
        auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
        ASSERT_TRUE(engine.createSession(kSender)) << "the sending session must exist";
        ASSERT_TRUE(engine.createSession(kPeer)) << "the peer session must exist";

        auto &service = EventRaiserService::getInstance();
        ASSERT_TRUE(service.registerEventRaiser(kSender, senderRaiser_));
        ASSERT_TRUE(service.registerEventRaiser(kPeer, peerRaiser_));

        factory_ = std::make_shared<EventTargetFactoryImpl>(fallback_);
    }

    void TearDown() override {
        auto &service = EventRaiserService::getInstance();
        service.unregisterEventRaiser(kSender);
        service.unregisterEventRaiser(kPeer);
        auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
        engine.destroySession(kSender);
        engine.destroySession(kPeer);
    }

    /// Send one event through the target the factory builds for `targetUri`,
    /// as seen from session `kSender`.
    static EventDescriptor eventNamed(const std::string &name, const std::string &targetUri) {
        EventDescriptor event;
        event.eventName = name;
        event.target = targetUri;
        return event;
    }

    static constexpr const char *kSender = "session-sender";
    static constexpr const char *kPeer = "session-peer";

    std::shared_ptr<MockEventRaiser> fallback_;
    std::shared_ptr<MockEventRaiser> senderRaiser_;
    std::shared_ptr<MockEventRaiser> peerRaiser_;
    std::shared_ptr<EventTargetFactoryImpl> factory_;
};

/// W3C SCXML C.1: an event addressed to another live session is delivered
/// to THAT session's queue.
///
/// This is the case `_event.origin` exists for: a parent that received an
/// event from an invoked child holds the child's session id, and sending
/// back to it must reach the child. Delivering to the sender instead is
/// silent misrouting — the send reports success and the peer waits forever.
TEST_F(SessionTargetRouting, AnEventAddressedToAPeerSessionReachesThatPeer) {
    auto target = factory_->createTarget(std::string("#_scxml_") + kPeer, kSender);
    ASSERT_NE(target, nullptr) << "a live peer session must be addressable";

    auto result = target->send(eventNamed("toPeer", std::string("#_scxml_") + kPeer));
    result.wait();

    EXPECT_EQ(peerRaiser_->getRaisedEvents().size(), 1u)
        << "the event was addressed to the peer session and must land there";
    EXPECT_TRUE(senderRaiser_->getRaisedEvents().empty()) << "the sending session is not the addressee";
    EXPECT_TRUE(fallback_->getRaisedEvents().empty())
        << "a resolvable session must not fall back to the factory's default raiser";
}

/// The case the W3C corpus DOES cover (tests 190/350): a session naming its
/// own id reaches its own external queue. Pinned so the cross-session fix
/// cannot be made by breaking the self case.
TEST_F(SessionTargetRouting, AnEventAddressedToItsOwnSessionStillReachesItself) {
    auto target = factory_->createTarget(std::string("#_scxml_") + kSender, kSender);
    ASSERT_NE(target, nullptr);

    auto result = target->send(eventNamed("toSelf", std::string("#_scxml_") + kSender));
    result.wait();

    EXPECT_EQ(senderRaiser_->getRaisedEvents().size(), 1u)
        << "a session addressing itself keeps reaching its own queue";
    EXPECT_TRUE(peerRaiser_->getRaisedEvents().empty());
}

/// A session id nobody is registered under is not a delivery address.
///
/// The failure that must NOT happen is the quiet one: routing it to the
/// sender's own queue makes an unreachable peer look like a delivered
/// event, which is indistinguishable from success at every layer above.
TEST_F(SessionTargetRouting, AnEventAddressedToAnUnknownSessionIsNotDeliveredToTheSender) {
    auto target = factory_->createTarget("#_scxml_session-that-does-not-exist", kSender);

    if (target != nullptr) {
        auto result = target->send(eventNamed("toGhost", "#_scxml_session-that-does-not-exist"));
        result.wait();
    }

    EXPECT_TRUE(senderRaiser_->getRaisedEvents().empty())
        << "an event for an unknown session must not be delivered to the sender instead";
    EXPECT_TRUE(peerRaiser_->getRaisedEvents().empty());
    EXPECT_TRUE(fallback_->getRaisedEvents().empty()) << "nor to the factory's default raiser";
}

/// W3C SCXML C.1: "The 'origin' field of the event raised in the receiving
/// session MUST match the value of the 'location' field inside the entry for
/// the SCXML Event I/O Processor in the `_ioprocessors` system variable in
/// the sending session."
///
/// That location is `sce://scxml/<sessionid>` (`IOProcessorHelper::
/// scxmlLocation`). A bare session id satisfies no reader: it is not what the
/// sender publishes, and §C.1's point is that the receiver can send BACK to
/// it — which the next test exercises.
///
/// ⚠ The receiving raiser here is a real `EventRaiserImpl`, not the mock the
/// routing tests use. `InternalEventTarget::send` dynamic-casts to that
/// concrete type and only the matching branch carries origin and origintype
/// at all; a mock therefore observes an empty origin no matter what the
/// production path does. Measuring the contract requires being on it.
TEST_F(SessionTargetRouting, TheOriginTheReceiverSeesIsTheSendersPublishedLocation) {
    auto realPeer = std::make_shared<SCE::EventRaiserImpl>();
    realPeer->setImmediateMode(false);  // keep the raise queued so it can be read back
    auto &service = EventRaiserService::getInstance();
    service.unregisterEventRaiser(kPeer);
    ASSERT_TRUE(service.registerEventRaiser(kPeer, realPeer));

    auto target = factory_->createTarget(std::string("#_scxml_") + kPeer, kSender);
    ASSERT_NE(target, nullptr);
    auto result = target->send(eventNamed("toPeer", std::string("#_scxml_") + kPeer));
    result.wait();

    std::vector<SCE::EventSnapshot> internalQueue;
    std::vector<SCE::EventSnapshot> externalQueue;
    realPeer->getEventQueues(internalQueue, externalQueue);
    ASSERT_EQ(externalQueue.size(), 1u) << "the addressed session queued the event";
    EXPECT_EQ(externalQueue[0].origin, SCE::IOProcessorHelper::scxmlLocation(kSender))
        << "the receiver must read the sender's published _ioprocessors location";
}

/// The half of §C.1 that makes the origin field worth having: what the
/// receiver reads must work as a `<send>` target.
///
/// Test 336 is supposed to prove this and cannot — it sends to its OWN
/// origin, so any value that routes back to the sender passes, the empty
/// string included.
TEST_F(SessionTargetRouting, ThePublishedLocationWorksAsASendTarget) {
    auto target = factory_->createTarget(SCE::IOProcessorHelper::scxmlLocation(kPeer), kSender);
    ASSERT_NE(target, nullptr) << "the location a session publishes must be addressable";

    auto result = target->send(eventNamed("backToPeer", SCE::IOProcessorHelper::scxmlLocation(kPeer)));
    result.wait();

    EXPECT_EQ(peerRaiser_->getRaisedEvents().size(), 1u)
        << "an event addressed to a published location reaches that session";
    EXPECT_TRUE(senderRaiser_->getRaisedEvents().empty());
    EXPECT_TRUE(fallback_->getRaisedEvents().empty());
}

/// §scxml-6.4.3: destroying a session cancels the events it queued — and
/// only those.
///
/// This lives beside the origin tests because it reads the same field.
/// `QueuedEvent::origin` holds the sender's published location, so a
/// comparison written against the raw session id callers pass would match
/// nothing and cancel nothing, silently: the count comes back zero, which is
/// also what "that session had queued nothing" looks like. Nothing exercised
/// this path before, so the two spellings could disagree undetected.
TEST_F(SessionTargetRouting, CancellingASessionRemovesOnlyThatSessionsQueuedEvents) {
    auto raiser = std::make_shared<SCE::EventRaiserImpl>();
    raiser->setImmediateMode(false);

    ASSERT_TRUE(raiser->raiseEvent("fromSender", "", kSender));
    ASSERT_TRUE(raiser->raiseEvent("fromPeer", "", kPeer));

    EXPECT_EQ(raiser->cancelEventsForSession(kSender), 1u) << "the sending session had exactly one event queued";

    std::vector<SCE::EventSnapshot> internalQueue;
    std::vector<SCE::EventSnapshot> externalQueue;
    raiser->getEventQueues(internalQueue, externalQueue);
    std::vector<std::string> survivors;
    for (const auto &snapshot : internalQueue) {
        survivors.push_back(snapshot.name);
    }
    for (const auto &snapshot : externalQueue) {
        survivors.push_back(snapshot.name);
    }
    ASSERT_EQ(survivors.size(), 1u) << "the other session's event must survive";
    EXPECT_EQ(survivors[0], "fromPeer");
}

/// §scxml-C-1, the other end of the mapping: what a session PUBLISHES as its
/// SCXML Event I/O Processor location is what a receiver reads as
/// `_event.origin`.
///
/// The two halves are only a mapping if both are pinned. The receiver's half
/// is asserted above; this asserts the publisher's — that the value reaching
/// the datamodel is `IOProcessorHelper`'s verbatim, not a re-spelling the
/// script engine invented on the way in. W3C test 569 is the only coverage
/// the corpus offers and it asks `cond="_ioprocessors['scxml'].location"`,
/// which any non-empty string satisfies.
TEST_F(SessionTargetRouting, ThePublishedLocationIsWhatTheDatamodelReads) {
    auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
    // The same pair StateMachine performs when it sets a session up, so this
    // measures the engine's publishing rather than a private arrangement.
    auto setup = engine.setupSystemVariables(kSender, "routingProbe", SCE::IOProcessorHelper::build(kSender)).get();
    ASSERT_TRUE(setup.isSuccess()) << setup.getErrorMessage();

    auto published = engine.evaluateExpression(kSender, "_ioprocessors['scxml'].location").get();
    ASSERT_TRUE(published.isSuccess()) << published.getErrorMessage();
    EXPECT_EQ(published.getValue<std::string>(), SCE::IOProcessorHelper::scxmlLocation(kSender))
        << "the datamodel must publish the location the engine routes on";

    // And that published string is exactly what a receiver is told, which is
    // what makes the two halves one mapping rather than two conventions.
    auto realPeer = std::make_shared<SCE::EventRaiserImpl>();
    realPeer->setImmediateMode(false);
    auto &service = EventRaiserService::getInstance();
    service.unregisterEventRaiser(kPeer);
    ASSERT_TRUE(service.registerEventRaiser(kPeer, realPeer));

    auto target = factory_->createTarget(std::string("#_scxml_") + kPeer, kSender);
    ASSERT_NE(target, nullptr);
    target->send(eventNamed("mapped", std::string("#_scxml_") + kPeer)).wait();

    std::vector<SCE::EventSnapshot> internalQueue;
    std::vector<SCE::EventSnapshot> externalQueue;
    realPeer->getEventQueues(internalQueue, externalQueue);
    ASSERT_EQ(externalQueue.size(), 1u);
    EXPECT_EQ(externalQueue[0].origin, published.getValue<std::string>())
        << "origin and the sender's published location are the same value, per C.1";
}

}  // namespace
