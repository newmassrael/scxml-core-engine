// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// Timer Example: Demonstrates TimerManager usage with AOT state machine

#include "timer_example_sm.h"
#include "wrappers/TimerManager.h"
#include <chrono>
#include <iostream>
#include <thread>

using namespace std::chrono_literals;
using namespace SCE::Generated::timer_example;

/**
 * @brief Timer identifiers for this example
 *
 * TimerID enum defines type-safe timer identifiers used by TimerManager.
 * Each TimerID maps to a specific event in the state machine.
 */
enum class TimerID : uint8_t {
    HEARTBEAT,  // Periodic timer for system alive checks (500ms interval)
    TIMEOUT     // One-shot timer for operation timeout (3000ms)
};

/**
 * @brief Extend generated SM with TimerID enum for TimerManager compatibility
 */
struct TimerExampleSM : public timer_example {
    using TimerID = ::TimerID;           // Inject TimerID into SM namespace
    using timer_example::timer_example;  // Inherit constructors
};

int main() {
    std::cout << "=== Timer Example ===" << std::endl;
    std::cout << "Demonstrates periodic and one-shot timers" << std::endl << std::endl;

    // Create state machine (no user context needed for this example)
    TimerExampleSM sm;

    // Create timer manager
    SCE::Wrappers::TimerManager<TimerExampleSM> timers(sm);

    // Register timers with their events
    std::cout << "[Setup] Registering timers..." << std::endl;
    timers.registerTimer(TimerID::HEARTBEAT, Event::Heartbeat_tick);
    timers.registerTimer(TimerID::TIMEOUT, Event::Operation_timeout);

    // Initialize state machine
    std::cout << "[Init] Initializing state machine..." << std::endl;
    try {
        sm.initialize();
    } catch (const std::exception &e) {
        std::cerr << "ERROR: State machine initialization failed: " << e.what() << std::endl;
        return 1;
    }
    std::cout << "Initial state: idle" << std::endl << std::endl;

    // Start operation
    std::cout << "[Event] start → running" << std::endl;
    sm.raiseExternal(Event::Start);
    sm.tick();  // Use tick() to process events and check scheduler

    // Start timers
    std::cout << "[Timer] Starting HEARTBEAT (periodic, 500ms)" << std::endl;
    std::cout << "[Timer] Starting TIMEOUT (one-shot, 3000ms)" << std::endl << std::endl;
    try {
        timers.startTimer(TimerID::HEARTBEAT, 500ms, true);  // Periodic: fires every 500ms
        timers.startTimer(TimerID::TIMEOUT, 3000ms, false);  // One-shot: fires once after 3s
    } catch (const std::exception &e) {
        std::cerr << "ERROR: Timer start failed: " << e.what() << std::endl;
        return 1;
    }

    // Run for 3.5 seconds with manual tick() loop
    // IMPORTANT: Must use tick() not step() - tick() consumes scheduled events from scheduler
    std::cout << "[Running] Processing events for 3.5 seconds..." << std::endl;

    auto startTime = std::chrono::steady_clock::now();
    auto endTime = startTime + 3500ms;

    int heartbeatCount = 0;

    while (std::chrono::steady_clock::now() < endTime) {
        // Check for ready scheduled events
        if (sm.hasReadyEvents()) {
            auto beforeState = sm.getCurrentState();

            // CRITICAL: Use tick() to consume scheduled events from SimpleScheduler
            // tick() = popReadyEvent() from scheduler + raiseExternal() + processEventQueues()
            // step() = processEventQueues() only (doesn't consume from scheduler!)
            sm.tick();

            // Process expired periodic timers for automatic re-scheduling
            // Checks if periodic timers have expired and schedules next occurrence
            timers.processExpiredTimers();

            auto afterState = sm.getCurrentState();

            // Detect heartbeat events (state doesn't change - targetless transition)
            // NOTE: This detection method assumes only heartbeat events keep state in Running
            //       For production code, consider tracking event types or using counters
            if (beforeState == State::Running && afterState == State::Running) {
                ++heartbeatCount;
                std::cout << "[Heartbeat #" << heartbeatCount << "] Tick at "
                          << std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() -
                                                                                   startTime)
                                 .count()
                          << "ms" << std::endl;
            }

            // Detect timeout transition
            if (afterState == State::Timeout) {
                std::cout << "[Timeout] Operation timed out at "
                          << std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() -
                                                                                   startTime)
                                 .count()
                          << "ms" << std::endl;
                break;
            }
        }

        // Poll at 10ms intervals
        std::this_thread::sleep_for(10ms);
    }

    // Stop all timers
    std::cout << std::endl << "[Cleanup] Stopping all timers..." << std::endl;
    timers.stopAllTimers();

    std::cout << "\n=== Timer Example Complete ===" << std::endl;
    std::cout << "Heartbeat fired: " << heartbeatCount << " times" << std::endl;
    std::cout << "Timeout occurred: " << (sm.getCurrentState() == State::Timeout ? "yes" : "no") << std::endl;
    std::cout << "Active timers: " << timers.getActiveTimerCount() << std::endl;

    return 0;
}
