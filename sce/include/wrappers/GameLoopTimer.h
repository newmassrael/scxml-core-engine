// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/LogicalTimeScheduler.h"
#include <chrono>
#include <cstdint>
#include <string>

namespace SCE::Wrappers {

/**
 * @brief Game loop-integrated timer manager for AOT-generated state machines
 *
 * Bridges game timing (tic-based) with SCXML event scheduling (millisecond-based).
 * Uses logical time instead of wall-clock time, enabling:
 *
 * - Game pause: Timers pause when processTic() is not called
 * - Deterministic replay: Same tic sequence = same timer behavior
 * - Time scaling: Adjust MS_PER_TIC for slow-motion effects
 *
 * Designed for integration with game engines like DOOM (35 tics/sec = 28.57ms/tic).
 *
 * W3C SCXML 6.2: Implements delayed send with game-synchronized timing
 *
 * Thread Safety: NOT thread-safe. Call from game loop thread only.
 *
 * @tparam SM Generated state machine type
 * @tparam TICRATE Game tic rate in Hz (default: 35 for DOOM)
 *
 * @example DOOM integration
 * @code
 * // In game initialization:
 * using GameSM = SCE::Generated::game_state::game_state;
 * GameSM gameSM;
 * GameLoopTimer<GameSM, 35> gameTimer(gameSM);  // 35 Hz = DOOM
 *
 * // In P_Ticker() (called 35 times/sec):
 * void P_Ticker(void) {
 *     // ... game logic ...
 *     gameTimer.processTic();  // Advances time by 28.57ms, fires ready events
 * }
 *
 * // Schedule event from SCXML <send delay="500ms">:
 * gameTimer.scheduleByMs(Event::TIMEOUT, std::chrono::milliseconds(500));
 *
 * // Or schedule by game tics (e.g., weapon cooldown = 10 tics):
 * gameTimer.scheduleByTics(Event::FIRE_COMPLETE, 10);
 * @endcode
 *
 * @example Variable timestep (requestAnimationFrame)
 * @code
 * // For browsers or engines with variable frame times:
 * double lastTime = 0;
 * void gameLoop(double timestamp) {
 *     double deltaMs = lastTime > 0 ? timestamp - lastTime : 0;
 *     lastTime = timestamp;
 *
 *     gameTimer.processTime(deltaMs);  // Variable timestep
 *     requestAnimationFrame(gameLoop);
 * }
 * @endcode
 */
template <typename SM, uint32_t TICRATE = 35> class GameLoopTimer {
public:
    /// Milliseconds per tic (compile-time constant for optimization)
    static constexpr double MS_PER_TIC = 1000.0 / static_cast<double>(TICRATE);

    using Event = typename SM::Event;

    /**
     * @brief Construct game loop timer for a state machine
     *
     * @param stateMachine Reference to the state machine instance
     */
    explicit GameLoopTimer(SM &stateMachine)
        : stateMachine_(stateMachine), scheduler_(), currentTic_(0), logicalTimeMs_(0.0) {}

    /**
     * @brief Process one game tic
     *
     * Call this from your game's main loop (e.g., P_Ticker in DOOM).
     * Advances logical time by MS_PER_TIC and fires all ready events.
     *
     * @return Number of events fired this tic
     */
    size_t processTic() {
        ++currentTic_;
        logicalTimeMs_ += MS_PER_TIC;
        return fireReadyEvents();
    }

    /**
     * @brief Process elapsed time (variable timestep)
     *
     * For engines with variable frame times (e.g., requestAnimationFrame).
     * Advances logical time by the given delta and fires all ready events.
     *
     * @param deltaMs Elapsed milliseconds since last call
     * @return Number of events fired
     */
    size_t processTime(double deltaMs) {
        if (deltaMs <= 0) {
            return 0;
        }
        logicalTimeMs_ += deltaMs;
        // Update tic counter (approximate, for debugging only)
        currentTic_ = static_cast<uint64_t>(logicalTimeMs_ / MS_PER_TIC);
        return fireReadyEvents();
    }

    /**
     * @brief Schedule event by tic count
     *
     * Schedules event to fire after the specified number of game tics.
     * Useful for game-specific timing (e.g., weapon cooldowns, AI delays).
     *
     * @param event Event to schedule
     * @param delayTics Number of game tics to wait
     * @param sendId Optional sendid for cancellation
     * @param eventData Optional event data
     * @return The sendid assigned to this event
     *
     * @example Weapon fire cooldown (10 tics at 35Hz = 286ms)
     * @code
     * timer.scheduleByTics(Event::FIRE_COMPLETE, 10);
     * @endcode
     */
    std::string scheduleByTics(Event event, uint32_t delayTics, const std::string &sendId = "",
                               const std::string &eventData = "") {
        double delayMs = static_cast<double>(delayTics) * MS_PER_TIC;
        double fireTimeMs = logicalTimeMs_ + delayMs;
        return scheduler_.scheduleEventAt(event, fireTimeMs, sendId, eventData);
    }

    /**
     * @brief Schedule event by milliseconds
     *
     * W3C SCXML 6.2 compliant delay scheduling.
     * Converts milliseconds to logical time for scheduling.
     *
     * @param event Event to schedule
     * @param delay Delay duration
     * @param sendId Optional sendid for cancellation
     * @param eventData Optional event data
     * @return The sendid assigned to this event
     *
     * @example W3C SCXML <send delay="500ms">
     * @code
     * timer.scheduleByMs(Event::TIMEOUT, std::chrono::milliseconds(500));
     * @endcode
     */
    std::string scheduleByMs(Event event, std::chrono::milliseconds delay, const std::string &sendId = "",
                             const std::string &eventData = "") {
        return scheduler_.scheduleEvent(event, logicalTimeMs_, delay, sendId, eventData);
    }

    /**
     * @brief Cancel a scheduled event
     *
     * W3C SCXML 6.2.5: <cancel sendidexpr="..."/>
     *
     * @param sendId The sendid of the event to cancel
     * @return true if sendid recorded for cancellation
     */
    bool cancel(const std::string &sendId) {
        return scheduler_.cancelEvent(sendId);
    }

    /**
     * @brief Get current tic count
     *
     * Useful for debugging and game-specific logic.
     *
     * @return Number of tics processed since construction
     */
    uint64_t getCurrentTic() const {
        return currentTic_;
    }

    /**
     * @brief Get current logical time in milliseconds
     *
     * @return Logical time elapsed since construction
     */
    double getLogicalTimeMs() const {
        return logicalTimeMs_;
    }

    /**
     * @brief Get number of pending events
     */
    size_t getPendingCount() const {
        return scheduler_.getPendingCount();
    }

    /**
     * @brief Check if any events are pending
     */
    bool hasPendingEvents() const {
        return scheduler_.hasPendingEvents();
    }

    /**
     * @brief Get remaining time until next event fires
     *
     * Useful for UI synchronization (e.g., countdown timer display).
     * Returns -1.0 if no pending events.
     *
     * @return Remaining time in milliseconds, or -1.0 if no events pending
     */
    double getRemainingTimeMs() const {
        double nextFireTime = scheduler_.getNextEventFireTimeMs();
        if (nextFireTime < 0) {
            return -1.0;
        }
        double remaining = nextFireTime - logicalTimeMs_;
        return remaining > 0 ? remaining : 0;
    }

    /**
     * @brief Clear all scheduled events
     */
    void clear() {
        scheduler_.clear();
    }

    /**
     * @brief Reset timer state (time and events)
     *
     * Useful for game restart or level change.
     */
    void reset() {
        clear();
        currentTic_ = 0;
        logicalTimeMs_ = 0.0;
    }

    /**
     * @brief Set logical time directly
     *
     * For save/load game state or time travel debugging.
     *
     * @param timeMs New logical time in milliseconds
     */
    void setLogicalTime(double timeMs) {
        logicalTimeMs_ = timeMs;
        currentTic_ = static_cast<uint64_t>(timeMs / MS_PER_TIC);
    }

    /**
     * @brief Get reference to underlying scheduler
     *
     * For advanced use cases requiring direct scheduler access.
     */
    SCE::Common::LogicalTimeScheduler<Event> &getScheduler() {
        return scheduler_;
    }

    /**
     * @brief Get const reference to underlying scheduler
     */
    const SCE::Common::LogicalTimeScheduler<Event> &getScheduler() const {
        return scheduler_;
    }

private:
    /**
     * @brief Fire all events ready at current logical time
     *
     * @return Number of events fired
     */
    size_t fireReadyEvents() {
        size_t count = 0;
        Event event;
        std::string eventData;

        while (scheduler_.popReadyEvent(logicalTimeMs_, event, eventData)) {
            if (eventData.empty()) {
                stateMachine_.raiseExternal(event);
            } else {
                stateMachine_.raiseExternal(event, eventData);
            }
            ++count;
        }

        // Process raised events
        if (count > 0) {
            stateMachine_.step();
        }

        return count;
    }

    SM &stateMachine_;
    SCE::Common::LogicalTimeScheduler<Event> scheduler_;
    uint64_t currentTic_;
    double logicalTimeMs_;
};

}  // namespace SCE::Wrappers
