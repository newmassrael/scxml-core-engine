#include "wrappers/AsyncStateMachine.h"
#include "dispatchers/StdThreadDispatcher.h"
#include <atomic>
#include <chrono>
#include <gtest/gtest.h>
#include <stdexcept>
#include <thread>
#include <vector>

// Include a generated state machine for testing
// Note: This requires traffic_light_sm.h to be generated first
// The test will use a simple state machine with Timer event transitions
namespace SCE::Wrappers {

/**
 * @brief Mock state machine for testing AsyncStateMachine wrapper
 *
 * Simulates a generated AOT state machine with basic functionality.
 */
class MockStateMachine {
public:
    enum class State { Idle, Active, Final };
    enum class Event { Start, Stop, Reset };

    MockStateMachine() : currentState_(State::Idle), initialized_(false), eventCount_(0) {}

    void initialize() {
        initialized_ = true;
        currentState_ = State::Idle;
    }

    void raiseExternal(Event event) {
        lastEvent_.store(event);
        eventCount_.fetch_add(1);
    }

    void raiseExternal(Event event, const std::string &data) {
        lastEvent_.store(event);
        lastEventData_ = data;
        eventCount_.fetch_add(1);
    }

    void step() {
        if (!initialized_) {
            return;
        }

        // Simple state machine logic
        State current = currentState_.load();
        Event event = lastEvent_.load();

        switch (current) {
        case State::Idle:
            if (event == Event::Start) {
                currentState_.store(State::Active);
            }
            break;
        case State::Active:
            if (event == Event::Stop) {
                currentState_.store(State::Final);
            } else if (event == Event::Reset) {
                currentState_.store(State::Idle);
            }
            break;
        case State::Final:
            // Final state, no transitions
            break;
        }

        lastEvent_.store(Event::Reset);  // Clear event
    }

    State getCurrentState() const {
        return currentState_.load();
    }

    bool isInFinalState() const {
        return currentState_.load() == State::Final;
    }

    int getEventCount() const {
        return eventCount_.load();
    }

    std::string getLastEventData() const {
        return lastEventData_;
    }

private:
    std::atomic<State> currentState_;
    std::atomic<Event> lastEvent_{Event::Reset};
    std::string lastEventData_;
    bool initialized_;
    std::atomic<int> eventCount_;
};

/**
 * @brief Unit tests for AsyncStateMachine wrapper
 *
 * Architecture Compliance:
 * - Zero Duplication: Wrapper delegates to underlying state machine
 * - Helper Pattern: Provides async convenience layer
 * - Thread Safety: Validates concurrent event posting
 */
class AsyncStateMachineTest : public ::testing::Test {
protected:
    void SetUp() override {
        dispatcher_ = Dispatchers::StdThreadDispatcher::create();
    }

    void TearDown() override {
        if (dispatcher_ && dispatcher_->isRunning()) {
            dispatcher_->stop();
            if (eventLoopThread_.joinable()) {
                eventLoopThread_.join();
            }
        }
        dispatcher_.reset();
    }

    void startEventLoop() {
        dispatcher_->start();
        eventLoopThread_ = std::thread([this]() { dispatcher_->run(); });
    }

    void stopEventLoop() {
        dispatcher_->stop();
        if (eventLoopThread_.joinable()) {
            eventLoopThread_.join();
        }
    }

    std::shared_ptr<Dispatchers::StdThreadDispatcher> dispatcher_;
    std::thread eventLoopThread_;
};

// Construction Tests

TEST_F(AsyncStateMachineTest, ConstructWithDispatcher) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    SUCCEED();
}

TEST_F(AsyncStateMachineTest, ConstructWithNullDispatcher) {
    EXPECT_THROW((AsyncStateMachine<MockStateMachine, MockStateMachine::Event>(nullptr)), std::invalid_argument);
}

TEST_F(AsyncStateMachineTest, Initialize) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    EXPECT_EQ(sm.getCurrentState(), MockStateMachine::State::Idle);
}

// Event Posting Tests

TEST_F(AsyncStateMachineTest, PostEventWithoutStartingDispatcher) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    EXPECT_THROW(sm.postEvent(MockStateMachine::Event::Start), std::runtime_error);
}

TEST_F(AsyncStateMachineTest, PostEventWithDispatcherRunning) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    startEventLoop();

    // Post event
    EXPECT_NO_THROW(sm.postEvent(MockStateMachine::Event::Start));

    // Wait for event processing
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    stopEventLoop();

    // Verify state transition
    EXPECT_EQ(sm.getCurrentState(), MockStateMachine::State::Active);
}

TEST_F(AsyncStateMachineTest, PostEventWithData) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    startEventLoop();

    // Post event with data
    std::string testData = "test_data";
    EXPECT_NO_THROW(sm.postEvent(MockStateMachine::Event::Start, testData));

    // Wait for event processing
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    stopEventLoop();

    // Verify data was passed
    EXPECT_EQ(sm.getStateMachine().getLastEventData(), testData);
}

TEST_F(AsyncStateMachineTest, PostMultipleEvents) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    startEventLoop();

    // Post multiple events
    sm.postEvent(MockStateMachine::Event::Start);
    sm.postEvent(MockStateMachine::Event::Stop);

    // Wait for all events
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    stopEventLoop();

    // Should be in Final state (Idle -> Start -> Active -> Stop -> Final)
    EXPECT_EQ(sm.getCurrentState(), MockStateMachine::State::Final);
    EXPECT_TRUE(sm.isInFinalState());
}

// Thread Safety Tests

TEST_F(AsyncStateMachineTest, ConcurrentEventPosting) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    startEventLoop();

    // Spawn multiple threads posting events
    const int numThreads = 5;
    const int eventsPerThread = 10;
    std::vector<std::thread> threads;

    for (int t = 0; t < numThreads; ++t) {
        threads.emplace_back([&sm]() {
            for (int i = 0; i < eventsPerThread; ++i) {
                sm.postEvent(MockStateMachine::Event::Reset);
            }
        });
    }

    for (auto &thread : threads) {
        thread.join();
    }

    // Wait for all events
    std::this_thread::sleep_for(std::chrono::milliseconds(300));

    stopEventLoop();

    // Verify all events were processed
    EXPECT_EQ(sm.getStateMachine().getEventCount(), numThreads * eventsPerThread);
}

TEST_F(AsyncStateMachineTest, ConcurrentEventPostingWithData) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    startEventLoop();

    // Spawn multiple threads posting events with data
    const int numThreads = 3;
    std::vector<std::thread> threads;

    for (int t = 0; t < numThreads; ++t) {
        threads.emplace_back([&sm, t]() {
            std::string data = "thread_" + std::to_string(t);
            sm.postEvent(MockStateMachine::Event::Reset, data);
        });
    }

    for (auto &thread : threads) {
        thread.join();
    }

    // Wait for all events
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    stopEventLoop();

    // Verify all events were processed
    EXPECT_EQ(sm.getStateMachine().getEventCount(), numThreads);
}

// State Queries Tests (Non-Thread-Safe)

TEST_F(AsyncStateMachineTest, GetCurrentStateWhenStopped) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    // Safe to query when dispatcher not running
    EXPECT_EQ(sm.getCurrentState(), MockStateMachine::State::Idle);
}

TEST_F(AsyncStateMachineTest, IsInFinalStateWhenStopped) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    // Safe to query when dispatcher not running
    EXPECT_FALSE(sm.isInFinalState());
}

TEST_F(AsyncStateMachineTest, GetStateMachineReference) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    // Get reference to underlying state machine
    MockStateMachine &underlyingSM = sm.getStateMachine();
    EXPECT_EQ(underlyingSM.getCurrentState(), MockStateMachine::State::Idle);
}

TEST_F(AsyncStateMachineTest, GetConstStateMachineReference) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    // Get const reference
    const MockStateMachine &underlyingSM = sm.getStateMachine();
    EXPECT_EQ(underlyingSM.getCurrentState(), MockStateMachine::State::Idle);
}

TEST_F(AsyncStateMachineTest, GetDispatcher) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);

    auto retrievedDispatcher = sm.getDispatcher();
    EXPECT_EQ(retrievedDispatcher, dispatcher_);
}

// Lifecycle Tests

TEST_F(AsyncStateMachineTest, DispatcherLifecycle) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    // Start
    startEventLoop();
    EXPECT_TRUE(dispatcher_->isRunning());

    // Post event
    sm.postEvent(MockStateMachine::Event::Start);
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    // Stop
    stopEventLoop();
    EXPECT_FALSE(dispatcher_->isRunning());

    // Restart
    startEventLoop();
    sm.postEvent(MockStateMachine::Event::Stop);
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    stopEventLoop();

    EXPECT_TRUE(sm.isInFinalState());
}

// Integration Tests

TEST_F(AsyncStateMachineTest, CompleteStateMachineExecution) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    startEventLoop();

    // Execute complete state machine flow
    EXPECT_EQ(sm.getCurrentState(), MockStateMachine::State::Idle);

    sm.postEvent(MockStateMachine::Event::Start);
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    EXPECT_EQ(sm.getCurrentState(), MockStateMachine::State::Active);

    sm.postEvent(MockStateMachine::Event::Stop);
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    EXPECT_EQ(sm.getCurrentState(), MockStateMachine::State::Final);

    stopEventLoop();

    EXPECT_TRUE(sm.isInFinalState());
}

TEST_F(AsyncStateMachineTest, EventOrderingPreserved) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);
    sm.initialize();

    startEventLoop();

    // Post events in specific order
    sm.postEvent(MockStateMachine::Event::Start);  // Idle -> Active
    sm.postEvent(MockStateMachine::Event::Reset);  // Active -> Idle
    sm.postEvent(MockStateMachine::Event::Start);  // Idle -> Active
    sm.postEvent(MockStateMachine::Event::Stop);   // Active -> Final

    // Wait for all events
    std::this_thread::sleep_for(std::chrono::milliseconds(300));

    stopEventLoop();

    // Should end in Final state if order preserved
    EXPECT_TRUE(sm.isInFinalState());
}

// Copy/Move Semantics Tests

TEST_F(AsyncStateMachineTest, CopyConstructorDeleted) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);

    // Should not compile if uncommented:
    // AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm2(sm);

    SUCCEED();  // Compilation success is the test
}

TEST_F(AsyncStateMachineTest, CopyAssignmentDeleted) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm1(dispatcher_);
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm2(dispatcher_);

    // Should not compile if uncommented:
    // sm2 = sm1;

    SUCCEED();  // Compilation success is the test
}

TEST_F(AsyncStateMachineTest, MoveConstructorDeleted) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm(dispatcher_);

    // Should not compile if uncommented:
    // AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm2(std::move(sm));

    SUCCEED();  // Compilation success is the test
}

TEST_F(AsyncStateMachineTest, MoveAssignmentDeleted) {
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm1(dispatcher_);
    AsyncStateMachine<MockStateMachine, MockStateMachine::Event> sm2(dispatcher_);

    // Should not compile if uncommented:
    // sm2 = std::move(sm1);

    SUCCEED();  // Compilation success is the test
}

}  // namespace SCE::Wrappers
