// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "runtime/IEventRaiser.h"
#include "runtime/StateSnapshot.h"
#include <functional>
#include <string>
#include <utility>
#include <vector>

namespace SCE {
namespace Test {

/**
 * @brief Mock implementation of IEventRaiser for testing
 *
 * Records all raised events and can optionally delegate to a callback
 */
class MockEventRaiser : public IEventRaiser {
public:
    /**
     * @brief Constructor with optional callback
     * @param callback Optional callback for event handling
     */
    explicit MockEventRaiser(std::function<bool(const std::string &, const std::string &)> callback = nullptr);

    /**
     * @brief Destructor
     */
    virtual ~MockEventRaiser() = default;

    // IEventRaiser interface
    bool raiseEvent(const std::string &eventName, const std::string &eventData = "") override;
    bool raiseEvent(const std::string &eventName, const std::string &eventData,
                    const std::string &originSessionId) override;
    bool raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &sendId,
                    bool unused) override;
    bool raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &originSessionId,
                    const std::string &invokeId) override;
    bool raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &originSessionId,
                    const std::string &invokeId, const std::string &originType) override;
    bool raiseInternalEvent(const std::string &eventName, const std::string &eventData) override;
    bool raiseExternalEvent(const std::string &eventName, const std::string &eventData) override;
    bool isReady() const override;
    void setImmediateMode(bool immediate) override;
    bool isImmediateModeEnabled() const override;
    void processQueuedEvents() override;
    bool processNextQueuedEvent() override;
    bool hasQueuedEvents() const override;
    bool processNextInternalEvent() override;
    bool hasQueuedInternalEvents() const override;

    void getEventQueues(std::vector<EventSnapshot> &outInternal,
                        std::vector<EventSnapshot> &outExternal) const override;

    std::shared_ptr<class IEventScheduler> getScheduler() const override;

    size_t cancelEventsForSession(const std::string &originSessionId) override;

    // Test inspection methods
    const std::vector<std::pair<std::string, std::string>> &getRaisedEvents() const;

    /// Origin recorded for each raised event, index-aligned with
    /// getRaisedEvents(). Empty string for the overloads that carry no
    /// origin. Recorded because `_event.origin` is a W3C-specified VALUE
    /// (§scxml-C-1 fixes it to the sender's `_ioprocessors` location), not
    /// merely routing bookkeeping — a mock that discards it can only assert
    /// which queue an event reached, never what the receiver would read.
    const std::vector<std::string> &getRaisedOrigins() const;
    void clearEvents();
    int getEventCount() const;

    // IEventRaiser interface
    void shutdown() override {}

    // Test configuration
    void setCallback(std::function<bool(const std::string &, const std::string &)> callback);
    void setReady(bool ready);

private:
    std::vector<std::pair<std::string, std::string>> raisedEvents_;
    std::vector<std::string> raisedOrigins_;
    /// Origin handed to the overload currently delegating into the 2-arg
    /// recorder. Cleared on every record so a later origin-less raise cannot
    /// inherit an earlier one.
    std::string pendingOrigin_;
    std::function<bool(const std::string &, const std::string &)> callback_;
    bool ready_ = true;
};

}  // namespace Test
}  // namespace SCE