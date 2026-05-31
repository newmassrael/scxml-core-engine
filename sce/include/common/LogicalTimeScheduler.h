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
#include <queue>
#include <string>
#include <unordered_set>

namespace SCE::Common {

/**
 * @brief Logical time-based event scheduler for game loop integration
 *
 * Unlike SimpleScheduler which uses wall-clock time (std::chrono::steady_clock),
 * LogicalTimeScheduler uses externally-provided logical time. This enables:
 *
 * - Game pause: Timers pause when game loop stops advancing logical time
 * - Deterministic replay: Same logical time sequence = same timer behavior
 * - Time scaling: Slow-motion or fast-forward by adjusting time advancement rate
 *
 * §scxml-6.2: Implements delayed send pattern with logical time semantics
 *
 * Thread Safety: NOT thread-safe. Per W3C SCXML specification, state machines
 * process events sequentially within a single thread.
 *
 * @tparam EventType Event enum type from generated state machine
 *
 * @example Game loop integration
 * @code
 * LogicalTimeScheduler<Event> scheduler;
 * double logicalTimeMs = 0.0;
 * const double MS_PER_TIC = 1000.0 / 35.0;  // DOOM: 35 tics/sec
 *
 * // In game loop (P_Ticker):
 * logicalTimeMs += MS_PER_TIC;
 *
 * Event event;
 * std::string data;
 * while (scheduler.popReadyEvent(logicalTimeMs, event, data)) {
 *     stateMachine.raiseExternal(event, data);
 * }
 * @endcode
 */
template <typename EventType> class LogicalTimeScheduler {
public:
    /**
     * @brief Scheduled event with logical fire time
     */
    struct ScheduledEvent {
        EventType event;
        double fireTimeMs;      ///< Logical time when event should fire (milliseconds)
        std::string sendId;     ///< §scxml-6.3.1: Unique identifier for cancellation
        std::string eventData;  ///< §scxml-5.10: Event data from params

        ScheduledEvent(EventType evt, double fireTime, std::string id = "", std::string data = "")
            : event(evt), fireTimeMs(fireTime), sendId(std::move(id)), eventData(std::move(data)) {}

        // Comparator for priority queue (earlier times have higher priority)
        bool operator>(const ScheduledEvent &other) const {
            return fireTimeMs > other.fireTimeMs;
        }
    };

    /**
     * @brief Schedule an event at a specific logical time
     *
     * @param event Event to schedule
     * @param fireTimeMs Absolute logical time when event should fire
     * @param sendId Optional sendid for cancellation (generated if empty)
     * @param eventData Optional event data
     * @return The sendid assigned to this event
     */
    std::string scheduleEventAt(EventType event, double fireTimeMs, const std::string &sendId = "",
                                const std::string &eventData = "") {
        std::string actualSendId = sendId;
        if (actualSendId.empty()) {
            actualSendId = generateUniqueSendId();
        }

        queue_.push(ScheduledEvent(event, fireTimeMs, actualSendId, eventData));
        return actualSendId;
    }

    /**
     * @brief Schedule an event with delay from current logical time
     *
     * Convenience method for W3C SCXML <send delay="..."> pattern.
     *
     * @param event Event to schedule
     * @param currentTimeMs Current logical time
     * @param delayMs Delay in milliseconds from current time
     * @param sendId Optional sendid for cancellation
     * @param eventData Optional event data
     * @return The sendid assigned to this event
     */
    std::string scheduleEvent(EventType event, double currentTimeMs, std::chrono::milliseconds delayMs,
                              const std::string &sendId = "", const std::string &eventData = "") {
        double fireTimeMs = currentTimeMs + static_cast<double>(delayMs.count());
        return scheduleEventAt(event, fireTimeMs, sendId, eventData);
    }

    /**
     * @brief Check if any events are ready at the given logical time
     *
     * @param currentTimeMs Current logical time
     * @return true if events ready, false otherwise
     */
    bool hasReadyEvents(double currentTimeMs) const {
        if (queue_.empty()) {
            return false;
        }
        return queue_.top().fireTimeMs <= currentTimeMs;
    }

    /**
     * @brief Get next ready event at the given logical time
     *
     * Cancelled events are automatically filtered out.
     *
     * @param currentTimeMs Current logical time
     * @param outEvent Output parameter for event
     * @param outEventData Output parameter for event data
     * @return true if event retrieved, false if no ready events
     */
    bool popReadyEvent(double currentTimeMs, EventType &outEvent, std::string &outEventData) {
        while (!queue_.empty()) {
            if (queue_.top().fireTimeMs > currentTimeMs) {
                return false;  // No ready events yet
            }

            auto scheduledEvent = queue_.top();
            queue_.pop();

            // §scxml-6.3: Skip cancelled events
            if (!scheduledEvent.sendId.empty() && isCancelled(scheduledEvent.sendId)) {
                cancelledSendIds_.erase(scheduledEvent.sendId);  // Clean up
                continue;
            }

            outEvent = scheduledEvent.event;
            outEventData = scheduledEvent.eventData;
            return true;
        }
        return false;
    }

    /**
     * @brief Get next ready event (without event data) - backward compatibility
     */
    bool popReadyEvent(double currentTimeMs, EventType &outEvent) {
        std::string eventData;
        return popReadyEvent(currentTimeMs, outEvent, eventData);
    }

    /**
     * @brief Cancel a scheduled event by sendid
     *
     * §scxml-6.3: <cancel sendidexpr="..."/> cancels pending delayed send
     *
     * @param sendId The sendid of the event to cancel
     * @return true if sendid recorded for cancellation
     */
    bool cancelEvent(const std::string &sendId) {
        if (sendId.empty()) {
            return false;
        }
        cancelledSendIds_.insert(sendId);
        return true;
    }

    /**
     * @brief Check if a sendid has been cancelled
     */
    bool isCancelled(const std::string &sendId) const {
        return cancelledSendIds_.find(sendId) != cancelledSendIds_.end();
    }

    /**
     * @brief Check if scheduler has any pending events
     */
    bool hasPendingEvents() const {
        return !queue_.empty();
    }

    /**
     * @brief Get number of pending events
     */
    size_t getPendingCount() const {
        return queue_.size();
    }

    /**
     * @brief Get fire time of next pending event
     *
     * Useful for UI synchronization (e.g., countdown display).
     * Returns -1.0 if no pending events.
     *
     * @return Fire time in milliseconds, or -1.0 if no events pending
     */
    double getNextEventFireTimeMs() const {
        if (queue_.empty()) {
            return -1.0;
        }
        return queue_.top().fireTimeMs;
    }

    /**
     * @brief Clear all scheduled events and cancellation records
     */
    void clear() {
        while (!queue_.empty()) {
            queue_.pop();
        }
        cancelledSendIds_.clear();
    }

private:
    /**
     * @brief Generate unique sendid for event tracking
     */
    static std::string generateUniqueSendId() {
        static std::atomic<uint64_t> counter{0};
        return "logical_sendid_" + std::to_string(++counter);
    }

    std::priority_queue<ScheduledEvent, std::vector<ScheduledEvent>, std::greater<ScheduledEvent>> queue_;
    std::unordered_set<std::string> cancelledSendIds_;
};

}  // namespace SCE::Common
