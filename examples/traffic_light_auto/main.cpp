// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include <chrono>
#include <iostream>
#include <thread>

// Generated state machine
#include "traffic_light_sm.h"

// Async dispatcher with timer support
#include "dispatchers/StdThreadDispatcher.h"
#include "wrappers/AsyncStateMachine.h"

using namespace SCE::Generated::traffic_light;
using namespace SCE::Dispatchers;
using namespace SCE::Wrappers;

int main() {
    std::cout << "=== Traffic Light Example (Auto Timer) ===\n\n";

    // Create dispatcher with timer support
    auto dispatcher = StdThreadDispatcher::create();
    dispatcher->start();

    // Start event loop in background thread
    std::thread eventLoop([&dispatcher]() {
        try {
            dispatcher->run();
        } catch (const std::exception &e) {
            std::cerr << "Event loop error: " << e.what() << std::endl;
        }
    });

    // Wrap generated state machine
    AsyncStateMachine<traffic_light, Event> light(dispatcher);
    light.initialize();

    std::cout << "State machine initialized\n";
    std::cout << "Starting automatic timer test (3 seconds each state)\n\n";

    // Simple test: One timer that triggers state change
    std::cout << "[Main] Current state: Red\n";
    std::cout << "[Main] Setting timer for 3 seconds...\n";

    dispatcher->startTimer(
        1,     // Timer ID
        3000,  // 3 seconds
        [&light]() {
            std::cout << "[Timer] Timer expired! Posting Timer event...\n";
            try {
                light.postEvent(Event::Timer);
            } catch (const std::exception &e) {
                std::cerr << "[Timer] Error posting event: " << e.what() << std::endl;
            }
        },
        false  // One-shot
    );

    // Wait for timer to fire
    std::this_thread::sleep_for(std::chrono::milliseconds(3500));
    std::cout << "[Main] Timer should have fired\n";

    // Stop dispatcher cleanly
    std::cout << "[Main] Stopping dispatcher...\n";
    dispatcher->stop();

    // Wait for event loop thread
    if (eventLoop.joinable()) {
        eventLoop.join();
    }

    std::cout << "\n=== Test Complete ===\n";
    return 0;
}
