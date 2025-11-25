#include "dispatchers/IEventDispatcher.h"
#include "dispatchers/StdThreadDispatcher.h"
#include "traffic_light_sm.h"
#include "wrappers/AsyncStateMachine.h"
#include <chrono>
#include <iostream>
#include <thread>

using namespace SCE::Generated::traffic_light;
using namespace SCE::Dispatchers;
using namespace SCE::Wrappers;

int main() {
    std::cout << "=== Async Dispatcher Example ===" << "\n\n";

    // Create dispatcher
    auto dispatcher = StdThreadDispatcher::create();
    dispatcher->start();

    std::cout << "✓ Dispatcher started" << "\n\n";

    // Create async state machine wrapper
    AsyncStateMachine<traffic_light, Event> light(dispatcher);
    light.initialize();

    std::cout << "✓ State machine initialized" << "\n";
    std::cout << "  Initial state: ";
    switch (light.getCurrentState()) {
    case State::Red:
        std::cout << "Red\n";
        break;
    case State::Green:
        std::cout << "Green\n";
        break;
    case State::Yellow:
        std::cout << "Yellow\n";
        break;
    default:
        std::cout << "Unknown\n";
        break;
    }

    // Start dispatcher event loop in background thread
    std::cout << "\n✓ Starting event loop thread" << "\n\n";
    std::thread eventLoopThread([dispatcher]() { dispatcher->run(); });

    // Post events asynchronously from main thread
    std::cout << "--- Posting events from main thread ---" << "\n";

    // Event 1: Red -> Green
    std::cout << "[Main] Posting Timer event (Red -> Green)" << "\n";
    light.postEvent(Event::Timer);
    std::this_thread::sleep_for(std::chrono::milliseconds(100));  // Give time to process

    std::cout << "  Current state: ";
    switch (light.getCurrentState()) {
    case State::Green:
        std::cout << "Green ✓\n";
        break;
    default:
        std::cout << "Unexpected state\n";
        break;
    }

    // Event 2: Green -> Yellow
    std::cout << "\n[Main] Posting Timer event (Green -> Yellow)" << "\n";
    light.postEvent(Event::Timer);
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    std::cout << "  Current state: ";
    switch (light.getCurrentState()) {
    case State::Yellow:
        std::cout << "Yellow ✓\n";
        break;
    default:
        std::cout << "Unexpected state\n";
        break;
    }

    // Event 3: Yellow -> Red
    std::cout << "\n[Main] Posting Timer event (Yellow -> Red)" << "\n";
    light.postEvent(Event::Timer);
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    std::cout << "  Current state: ";
    switch (light.getCurrentState()) {
    case State::Red:
        std::cout << "Red ✓\n";
        break;
    default:
        std::cout << "Unexpected state\n";
        break;
    }

    // Test multi-threaded event posting
    std::cout << "\n--- Testing multi-threaded event posting ---" << "\n";
    std::cout << "Spawning 3 threads posting events concurrently..." << "\n";

    std::vector<std::thread> threads;
    for (int i = 0; i < 3; ++i) {
        threads.emplace_back([&light, i]() {
            std::this_thread::sleep_for(std::chrono::milliseconds(50 * i));
            std::cout << "  [Thread " << i << "] Posting Timer event" << "\n";
            light.postEvent(Event::Timer);
        });
    }

    for (auto &t : threads) {
        t.join();
    }

    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    std::cout << "\n✓ All events processed" << "\n";
    std::cout << "  Final state: ";
    switch (light.getCurrentState()) {
    case State::Green:
        std::cout << "Green\n";
        break;
    case State::Yellow:
        std::cout << "Yellow\n";
        break;
    case State::Red:
        std::cout << "Red\n";
        break;
    default:
        std::cout << "Unknown\n";
        break;
    }

    // Cleanup
    std::cout << "\n--- Cleanup ---" << "\n";
    std::cout << "Stopping dispatcher..." << "\n";
    dispatcher->stop();
    eventLoopThread.join();

    std::cout << "✓ Dispatcher stopped" << "\n";
    std::cout << "\n=== Example Complete ===" << "\n";

    return 0;
}
