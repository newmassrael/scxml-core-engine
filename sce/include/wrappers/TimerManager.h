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

#include <chrono>
#include <functional>
#include <optional>
#include <string>
#include <unordered_map>

namespace SCE::Wrappers {

/**
 * @brief High-level timer management API for AOT-generated state machines
 *
 * Provides intuitive timer operations on top of StaticExecutionEngine's scheduleEvent().
 *
 * Key features:
 * - Type-safe timer identification via TimerID enum
 * - Periodic and one-shot timer support
 * - Timer lifecycle management (start/stop/restart)
 * - Automatic event generation on timer expiration
 * - Built on existing EventScheduler infrastructure
 *
 * §scxml-6.2: Implements delayed send pattern with timer semantics
 *
 * @note Thread Safety: NOT thread-safe. Per W3C SCXML specification, state machines
 *       process events sequentially within a single thread. Each state machine instance
 *       and its TimerManager must be accessed from only one thread.
 *
 *       For multi-threaded applications: Create separate state machine instances per
 *       thread. Use EventRaiserRegistry for cross-thread communication (thread-safe).
 *
 * @tparam SM Generated state machine type (must have TimerID and Event enums)
 *
 * @example Basic Usage
 * @code
 * // In generated code: enum class TimerID { HEARTBEAT, TIMEOUT };
 * using SM = GeneratedStateMachine;
 * SM sm(userContext);
 * TimerManager<SM> timers(sm);
 *
 * // Register timer -> event mapping
 * timers.registerTimer(TimerID::HEARTBEAT, Event::HEARTBEAT_TICK);
 *
 * // Start periodic timer (fires every 1000ms)
 * timers.startTimer(TimerID::HEARTBEAT, 1000ms, true);
 *
 * // Check timer status
 * if (timers.isTimerRunning(TimerID::HEARTBEAT)) {
 *     timers.stopTimer(TimerID::HEARTBEAT);
 * }
 *
 * // One-shot timer (fires once after 500ms)
 * timers.registerTimer(TimerID::TIMEOUT, Event::TIMEOUT);
 * timers.startTimer(TimerID::TIMEOUT, 500ms, false);
 * @endcode
 *
 * @example Integration with runUntilCompletion
 * @code
 * SM sm(userContext);
 * TimerManager<SM> timers(sm);
 *
 * timers.registerTimer(TimerID::HEARTBEAT, Event::HEARTBEAT_TICK);
 * timers.startTimer(TimerID::HEARTBEAT, 100ms, true);
 *
 * sm.initialize();
 * sm.runUntilCompletion(5000ms, 10ms);  // Timers fire automatically during polling
 * @endcode
 *
 * @example Multi-threaded usage (separate state machine instances)
 * @code
 * // Thread 1: Independent state machine instance
 * void deviceThread1() {
 *     DeviceSM device1;
 *     TimerManager<DeviceSM> timers1(device1);
 *     timers1.registerTimer(TimerID::HEARTBEAT, Event::TICK);
 *     timers1.startTimer(TimerID::HEARTBEAT, 100ms, true);
 *
 *     device1.initialize();
 *     while (!device1.isInFinalState()) {
 *         device1.tick();
 *         timers1.processExpiredTimers();
 *     }
 * }
 *
 * // Thread 2: Separate state machine instance (no shared state)
 * void deviceThread2() {
 *     DeviceSM device2;  // Independent instance
 *     TimerManager<DeviceSM> timers2(device2);
 *     // ... same pattern as thread 1
 * }
 *
 * // Cross-thread communication: Use EventRaiserRegistry (thread-safe)
 * // device1 → device2: eventRaiser.raiseEvent(device2Id, Event::MESSAGE);
 * @endcode
 */
template <typename SM> class TimerManager {
public:
    using TimerID = typename SM::TimerID;
    using Event = typename SM::Event;

    /**
     * @brief Construct timer manager for a state machine
     *
     * @param stateMachine Reference to the state machine instance
     */
    explicit TimerManager(SM &stateMachine) : stateMachine_(stateMachine) {}

    /**
     * @brief Register a timer with its associated event
     *
     * Maps a timer ID to an event that will be raised when the timer expires.
     * Must be called before startTimer().
     *
     * §scxml-6.2: Establishes timer -> delayed send mapping
     *
     * @param timerID Timer identifier (from generated TimerID enum)
     * @param event Event to raise on timer expiration
     *
     * @example
     * @code
     * timers.registerTimer(TimerID::HEARTBEAT, Event::HEARTBEAT_TICK);
     * timers.registerTimer(TimerID::TIMEOUT, Event::OPERATION_TIMEOUT);
     * @endcode
     */
    void registerTimer(TimerID timerID, Event event) {
        timerEventMap_[timerID] = event;
    }

    /**
     * @brief Start a timer with specified interval
     *
     * Schedules the timer's associated event for future delivery.
     * For periodic timers, automatically reschedules after each expiration.
     *
     * §scxml-6.2: Uses scheduleEvent() with delay
     *
     * @param timerID Timer to start (must be registered first)
     * @param interval Delay before timer expiration
     * @param periodic If true, timer auto-restarts after expiration
     * @throws std::runtime_error if timer not registered
     *
     * @example
     * @code
     * // One-shot timer (fires once)
     * timers.startTimer(TimerID::TIMEOUT, 5000ms, false);
     *
     * // Periodic timer (fires every 100ms)
     * timers.startTimer(TimerID::HEARTBEAT, 100ms, true);
     * @endcode
     */
    void startTimer(TimerID timerID, std::chrono::milliseconds interval, bool periodic = false) {
        auto eventIt = timerEventMap_.find(timerID);
        if (eventIt == timerEventMap_.end()) {
            throw std::runtime_error("Timer not registered: call registerTimer() first");
        }

        Event event = eventIt->second;

        // Store timer metadata with last schedule time for periodic tracking
        auto now = std::chrono::steady_clock::now();
        std::string baseSendId = generateTimerSendId(timerID);
        TimerInfo info{interval, periodic, baseSendId, true, now, 0};

        // §scxml-6.2: Schedule delayed event with sequence number for consistency
        std::string uniqueSendId = baseSendId + "_" + std::to_string(info.sequenceCounter);
        stateMachine_.scheduleEvent(event, interval, uniqueSendId);
        ++info.sequenceCounter;  // Increment BEFORE storing in map

        activeTimers_[timerID] = info;
    }

    /**
     * @brief Stop a running timer
     *
     * Cancels the timer's scheduled event. For periodic timers, prevents future recurrence.
     *
     * §scxml-6.3: Uses cancelEvent() with sendId
     *
     * @param timerID Timer to stop
     * @return true if timer was running and stopped, false if not running
     *
     * @example
     * @code
     * if (timers.isTimerRunning(TimerID::HEARTBEAT)) {
     *     timers.stopTimer(TimerID::HEARTBEAT);
     * }
     * @endcode
     */
    bool stopTimer(TimerID timerID) {
        auto it = activeTimers_.find(timerID);
        if (it == activeTimers_.end() || !it->second.isRunning) {
            return false;
        }

        // §scxml-6.3: Cancel scheduled event
        // For periodic timers, cancel the currently scheduled event (sequenceCounter - 1)
        // For one-shot timers, cancel the only scheduled event (sequenceCounter - 1)
        if (it->second.sequenceCounter > 0) {
            std::string actualSendId = it->second.sendId + "_" + std::to_string(it->second.sequenceCounter - 1);
            stateMachine_.cancelEvent(actualSendId);
        }

        it->second.isRunning = false;
        activeTimers_.erase(it);
        return true;
    }

    /**
     * @brief Restart a timer with its original interval
     *
     * Convenience method to stop and restart a timer.
     * Preserves the timer's periodic/one-shot mode.
     *
     * @param timerID Timer to restart
     * @return true if timer was restarted, false if not previously started
     *
     * @example
     * @code
     * // Reset watchdog timer
     * timers.restartTimer(TimerID::WATCHDOG);
     * @endcode
     */
    bool restartTimer(TimerID timerID) {
        auto it = activeTimers_.find(timerID);
        if (it == activeTimers_.end()) {
            return false;
        }

        // Preserve timer configuration
        TimerInfo info = it->second;

        // Cancel current scheduled event
        stopTimer(timerID);

        // Restart with preserved sequence counter to avoid sendId collision
        auto now = std::chrono::steady_clock::now();
        info.lastScheduleTime = now;
        info.isRunning = true;

        // Schedule next event with continuing sequence
        Event event = timerEventMap_[timerID];
        std::string uniqueSendId = info.sendId + "_" + std::to_string(info.sequenceCounter);
        stateMachine_.scheduleEvent(event, info.interval, uniqueSendId);
        ++info.sequenceCounter;

        activeTimers_[timerID] = info;
        return true;
    }

    /**
     * @brief Check if a timer is currently running
     *
     * @param timerID Timer to check
     * @return true if timer is active, false otherwise
     *
     * @example
     * @code
     * if (!timers.isTimerRunning(TimerID::HEARTBEAT)) {
     *     timers.startTimer(TimerID::HEARTBEAT, 1000ms, true);
     * }
     * @endcode
     */
    bool isTimerRunning(TimerID timerID) const {
        auto it = activeTimers_.find(timerID);
        return it != activeTimers_.end() && it->second.isRunning;
    }

    /**
     * @brief Process timer expiration (for periodic timer re-scheduling)
     *
     * Call this after state machine tick() to handle periodic timer renewal.
     * This method checks if any periodic timers expired and reschedules them.
     *
     * Periodic timers are re-scheduled with consistent intervals. Small timing
     * variations may occur due to polling intervals and system load, but the
     * implementation maintains stable long-term periodicity.
     *
     * §scxml-6.2: Implements periodic timer semantics on top of one-shot delayed send
     *
     * @note Requires using sm.tick() (not sm.step()) to consume scheduled events
     * @note Not needed when using runUntilCompletion() - timers handled automatically
     *
     * @example Manual step() loop
     * @code
     * while (!sm.isInFinalState()) {
     *     sm.step();
     *     timers.processExpiredTimers();  // Re-schedule periodic timers
     *     std::this_thread::sleep_for(10ms);
     * }
     * @endcode
     */
    void processExpiredTimers() {
        auto now = std::chrono::steady_clock::now();

        for (auto &[timerID, info] : activeTimers_) {
            // Skip non-periodic or stopped timers
            if (!info.isRunning || !info.periodic) {
                continue;
            }

            // Check if enough time has passed to schedule the next periodic event
            auto timeSinceLastSchedule =
                std::chrono::duration_cast<std::chrono::milliseconds>(now - info.lastScheduleTime);

            if (timeSinceLastSchedule >= info.interval) {
                // Time for next periodic event - schedule it
                auto eventIt = timerEventMap_.find(timerID);
                if (eventIt != timerEventMap_.end()) {
                    Event event = eventIt->second;

                    // Use unique sendId for each periodic occurrence
                    std::string uniqueSendId = info.sendId + "_" + std::to_string(info.sequenceCounter);
                    stateMachine_.scheduleEvent(event, info.interval, uniqueSendId);

                    // Update last schedule time (advance by interval for next check)
                    // Note: Uses relative increment to maintain long-term periodicity
                    info.lastScheduleTime += info.interval;
                    ++info.sequenceCounter;
                }
            }
        }
    }

    /**
     * @brief Stop all running timers
     *
     * Convenience method to cancel all active timers.
     * Useful for state machine shutdown or reset scenarios.
     *
     * @example
     * @code
     * timers.stopAllTimers();
     * sm.shutdown();
     * @endcode
     */
    void stopAllTimers() {
        std::vector<TimerID> timersToStop;
        for (const auto &[timerID, info] : activeTimers_) {
            if (info.isRunning) {
                timersToStop.push_back(timerID);
            }
        }

        for (const auto &timerID : timersToStop) {
            stopTimer(timerID);
        }
    }

    /**
     * @brief Get number of currently running timers
     *
     * @return Count of active timers
     */
    size_t getActiveTimerCount() const {
        size_t count = 0;
        for (const auto &[_, info] : activeTimers_) {
            if (info.isRunning) {
                ++count;
            }
        }
        return count;
    }

    /**
     * @brief Get timer interval (if timer exists)
     *
     * @param timerID Timer to query
     * @return Timer interval, or nullopt if timer not found
     */
    std::optional<std::chrono::milliseconds> getTimerInterval(TimerID timerID) const {
        auto it = activeTimers_.find(timerID);
        if (it != activeTimers_.end()) {
            return it->second.interval;
        }
        return std::nullopt;
    }

    /**
     * @brief Check if timer is periodic
     *
     * @param timerID Timer to query
     * @return true if timer is periodic, false if one-shot or not found
     */
    bool isTimerPeriodic(TimerID timerID) const {
        auto it = activeTimers_.find(timerID);
        return it != activeTimers_.end() && it->second.periodic;
    }

private:
    /**
     * @brief Timer metadata for lifecycle management
     */
    struct TimerInfo {
        std::chrono::milliseconds interval;
        bool periodic;
        std::string sendId;
        bool isRunning;
        std::chrono::steady_clock::time_point lastScheduleTime;  // Track last schedule time for periodic timers
        uint64_t sequenceCounter;                                // Unique sequence number for periodic events
    };

    /**
     * @brief Generate unique sendId for timer
     *
     * §scxml-6.3.1: SendId format for timer identification
     *
     * @param timerID Timer identifier
     * @return Unique sendId string
     */
    std::string generateTimerSendId(TimerID timerID) const {
        return "timer_" + std::to_string(static_cast<int>(timerID));
    }

    SM &stateMachine_;                                     // Reference to state machine
    std::unordered_map<TimerID, Event> timerEventMap_;     // Timer -> Event mapping
    std::unordered_map<TimerID, TimerInfo> activeTimers_;  // Active timer tracking
};

}  // namespace SCE::Wrappers
