// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <atomic>
#include <chrono>
#include <map>
#include <memory>
#include <regex>
#include <string>
#include <unordered_map>

namespace SCE {

/**
 * @brief Helper for W3C SCXML <send> delay parsing
 *
 * Single Source of Truth for delay parsing logic shared between:
 * - Interpreter engine (EventSchedulerImpl)
 * - AOT engine (StaticExecutionEngine)
 *
 * W3C SCXML References:
 * - 6.2: Send element delay/delayexpr semantics
 */
class SendSchedulingHelper {
public:
    /**
     * @brief Parse W3C SCXML delay string to milliseconds
     *
     * W3C SCXML 6.2: Delay formats - "5s", "100ms", "2min", ".5s", "0.5s"
     *
     * @param delayStr Delay specification (e.g., "5s", "100ms", "2min")
     * @return Delay in milliseconds, 0 if invalid or empty
     */
    static std::chrono::milliseconds parseDelayString(const std::string &delayStr) {
        if (delayStr.empty()) {
            return std::chrono::milliseconds{0};
        }

        std::regex delayPattern(R"((\d*\.?\d+)\s*(ms|s|min|h|sec|seconds?|minutes?|hours?)?)");
        std::smatch match;

        if (!std::regex_match(delayStr, match, delayPattern)) {
            return std::chrono::milliseconds{0};
        }

        double value = std::stod(match[1].str());
        std::string unit = match[2].str();

        if (unit.empty() || unit == "s" || unit == "sec" || unit == "second" || unit == "seconds") {
            return std::chrono::milliseconds(static_cast<long long>(value * 1000));
        } else if (unit == "ms") {
            return std::chrono::milliseconds(static_cast<long long>(value));
        } else if (unit == "min" || unit == "minute" || unit == "minutes") {
            return std::chrono::milliseconds(static_cast<long long>(value * 60000));
        } else if (unit == "h" || unit == "hour" || unit == "hours") {
            return std::chrono::milliseconds(static_cast<long long>(value * 3600000));
        }

        return std::chrono::milliseconds{0};
    }
};

/**
 * @brief Core event queue using std::map for W3C SCXML compliant scheduling
 *
 * Single Source of Truth (Zero Duplication) for event scheduling logic:
 * - Interpreter engine (EventSchedulerImpl): Same pattern with thread-safety wrapper
 * - AOT engine (PullScheduler): Direct delegation to this class
 *
 * Design Rationale - Why std::map over std::priority_queue:
 * - W3C SCXML 6.3 requires actual event cancellation, not lazy marking
 * - std::map supports O(log n) deletion by iterator
 * - std::priority_queue does NOT support deletion (only lazy cancellation workaround)
 *
 * Performance:
 * - Insert: O(log n)
 * - Get next (begin): O(1)
 * - Pop (erase begin): O(log n) amortized
 * - Cancel by sendId: O(log n) - ACTUAL removal, not lazy!
 *
 * Thread-safety: NOT thread-safe (caller must provide synchronization)
 *
 * @tparam EventType The event type (enum for AOT, EventDescriptor for Interpreter)
 * @tparam EventDataType Additional data (std::string for AOT, shared_ptr<IEventTarget> for Interpreter)
 */
template <typename EventType, typename EventDataType = std::string> class SchedulerQueueCore {
public:
    using TimePoint = std::chrono::steady_clock::time_point;

    /**
     * @brief Scheduled event entry
     */
    struct ScheduledEntry {
        EventType event;
        TimePoint fireTime;
        std::string sendId;
        EventDataType eventData;
        uint64_t sequenceNum;  // FIFO ordering for same fireTime

        ScheduledEntry(EventType evt, TimePoint fire, std::string id, EventDataType data, uint64_t seq)
            : event(std::move(evt)), fireTime(fire), sendId(std::move(id)), eventData(std::move(data)),
              sequenceNum(seq) {}
    };

    using EntryPtr = std::shared_ptr<ScheduledEntry>;

    /**
     * @brief Key for map ordering (fireTime, sequenceNum)
     */
    struct OrderKey {
        TimePoint fireTime;
        uint64_t sequenceNum;

        bool operator<(const OrderKey &other) const {
            if (fireTime != other.fireTime) {
                return fireTime < other.fireTime;
            }
            return sequenceNum < other.sequenceNum;
        }
    };

    /**
     * @brief Schedule an event for future delivery
     *
     * W3C SCXML 6.2: Delayed send
     * W3C SCXML 6.3: If sendId exists, REMOVES old event (not lazy marking!)
     *
     * @param event Event to schedule
     * @param fireTime When the event should fire
     * @param sendId Unique identifier for cancellation
     * @param eventData Additional event data
     * @return The sendId assigned
     */
    std::string schedule(EventType event, TimePoint fireTime, const std::string &sendId,
                         EventDataType eventData = EventDataType{}) {
        std::string actualSendId = sendId.empty() ? generateUniqueSendId() : sendId;

        // W3C SCXML 6.3: Cancel existing event with same sendId (ACTUAL removal)
        cancel(actualSendId);

        // Create and insert new entry
        uint64_t seqNum = sequenceCounter_++;
        auto entry =
            std::make_shared<ScheduledEntry>(std::move(event), fireTime, actualSendId, std::move(eventData), seqNum);

        OrderKey key{fireTime, seqNum};
        auto it = queue_.emplace(key, entry);
        sendIdIndex_[actualSendId] = it.first;

        return actualSendId;
    }

    /**
     * @brief Cancel a scheduled event by sendId
     *
     * W3C SCXML 6.3: ACTUALLY REMOVES the event (not lazy marking!)
     *
     * @param sendId The sendId to cancel
     * @return true if found and removed
     */
    bool cancel(const std::string &sendId) {
        if (sendId.empty()) {
            return false;
        }

        auto indexIt = sendIdIndex_.find(sendId);
        if (indexIt == sendIdIndex_.end()) {
            return false;
        }

        queue_.erase(indexIt->second);  // ACTUAL removal!
        sendIdIndex_.erase(indexIt);
        return true;
    }

    /**
     * @brief Check if any events are ready
     */
    bool hasReadyEvents() const {
        return !queue_.empty() && queue_.begin()->first.fireTime <= std::chrono::steady_clock::now();
    }

    /**
     * @brief Check if any events are ready at specified time
     */
    bool hasReadyEvents(TimePoint now) const {
        return !queue_.empty() && queue_.begin()->first.fireTime <= now;
    }

    /**
     * @brief Pop and return the next ready event
     */
    bool popReadyEvent(EventType &outEvent, EventDataType &outEventData) {
        std::string sendId;
        return popReadyEventImpl(std::chrono::steady_clock::now(), outEvent, outEventData, sendId);
    }

    /**
     * @brief Pop and return the next ready event (with sendId output)
     */
    bool popReadyEvent(EventType &outEvent, EventDataType &outEventData, std::string &outSendId) {
        return popReadyEventImpl(std::chrono::steady_clock::now(), outEvent, outEventData, outSendId);
    }

    /**
     * @brief Pop at specified time
     */
    bool popReadyEvent(TimePoint now, EventType &outEvent, EventDataType &outEventData) {
        std::string sendId;
        return popReadyEventImpl(now, outEvent, outEventData, sendId);
    }

    bool hasPendingEvents() const {
        return !queue_.empty();
    }

    size_t size() const {
        return queue_.size();
    }

    void clear() {
        queue_.clear();
        sendIdIndex_.clear();
    }

    bool hasEvent(const std::string &sendId) const {
        return sendIdIndex_.count(sendId) > 0;
    }

    TimePoint getNextFireTime() const {
        return queue_.empty() ? TimePoint::max() : queue_.begin()->first.fireTime;
    }

private:
    bool popReadyEventImpl(TimePoint now, EventType &outEvent, EventDataType &outEventData, std::string &outSendId) {
        if (queue_.empty() || queue_.begin()->first.fireTime > now) {
            return false;
        }

        auto it = queue_.begin();
        outEvent = std::move(it->second->event);
        outEventData = std::move(it->second->eventData);
        outSendId = it->second->sendId;

        sendIdIndex_.erase(it->second->sendId);
        queue_.erase(it);
        return true;
    }

    static std::string generateUniqueSendId() {
        static std::atomic<uint64_t> counter{0};
        return "auto_" + std::to_string(++counter);
    }

    std::map<OrderKey, EntryPtr> queue_;
    std::unordered_map<std::string, typename std::map<OrderKey, EntryPtr>::iterator> sendIdIndex_;
    uint64_t sequenceCounter_ = 0;
};

/**
 * @brief Pull-based event scheduler for AOT-generated state machines
 *
 * Lightweight wrapper around SchedulerQueueCore.
 * Zero Duplication: All scheduling logic in SchedulerQueueCore.
 *
 * Design: Pull-based (caller pulls ready events) vs Push-based (EventSchedulerImpl)
 * Thread-safety: Not thread-safe (AOT state machines are single-threaded)
 */
template <typename EventType> class PullScheduler {
public:
    std::string scheduleEvent(EventType event, std::chrono::milliseconds delay, const std::string &sendId = "",
                              const std::string &eventData = "") {
        auto fireTime = std::chrono::steady_clock::now() + delay;
        return core_.schedule(std::move(event), fireTime, sendId, eventData);
    }

    bool hasReadyEvents() const {
        return core_.hasReadyEvents();
    }

    bool popReadyEvent(EventType &outEvent, std::string &outEventData) {
        return core_.popReadyEvent(outEvent, outEventData);
    }

    bool popReadyEvent(EventType &outEvent) {
        std::string eventData;
        return core_.popReadyEvent(outEvent, eventData);
    }

    bool hasPendingEvents() const {
        return core_.hasPendingEvents();
    }

    bool cancelEvent(const std::string &sendId) {
        return core_.cancel(sendId);
    }

    bool isCancelled(const std::string &sendId) const {
        return !core_.hasEvent(sendId);
    }

    void clear() {
        core_.clear();
    }

private:
    SchedulerQueueCore<EventType, std::string> core_;
};

}  // namespace SCE
