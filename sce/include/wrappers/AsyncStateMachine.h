// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "dispatchers/IEventDispatcher.h"
#include <memory>
#include <stdexcept>

namespace SCE::Wrappers {

/**
 * @brief Asynchronous wrapper for generated state machines
 *
 * Wraps a generated AOT state machine to provide thread-safe asynchronous
 * event processing via an event dispatcher. Events are enqueued on the
 * dispatcher thread and processed asynchronously.
 *
 * Design Goals:
 * - Thread Safety: Safe event submission from any thread
 * - Zero Duplication: Delegates to underlying state machine
 * - Async-First: All events processed asynchronously
 * - Dispatcher Agnostic: Works with any IEventDispatcher implementation
 *
 * Architecture Compliance:
 * - Single Source of Truth: Generated state machine is authoritative
 * - Helper Pattern: Wrapper provides async convenience layer
 * - Zero Overhead (when not used): Template, header-only
 *
 * Example:
 * @code
 * // Create dispatcher
 * auto dispatcher = StdThreadDispatcher::create();
 * dispatcher->start();
 *
 * // Wrap generated state machine
 * AsyncStateMachine<traffic_light> sm(dispatcher);
 * sm.initialize();
 *
 * // Thread-safe async event posting (from any thread)
 * sm.postEvent(Event::Timer);
 *
 * // Run dispatcher event loop
 * std::thread eventThread([dispatcher]() {
 *     dispatcher->run();
 * });
 *
 * // Later: cleanup
 * dispatcher->stop();
 * eventThread.join();
 * @endcode
 *
 * @tparam StateMachine Generated state machine type (e.g., traffic_light)
 * @tparam EventType Event enum type (e.g., Event)
 */
template <typename StateMachine, typename EventType> class AsyncStateMachine {
public:
    /**
     * @brief Constructor with dispatcher
     *
     * @param dispatcher Event dispatcher for async processing
     *
     * @throws std::invalid_argument if dispatcher is null
     */
    explicit AsyncStateMachine(std::shared_ptr<SCE::Dispatchers::IEventDispatcher> dispatcher)
        : dispatcher_(std::move(dispatcher)), sm_() {
        if (!dispatcher_) {
            throw std::invalid_argument("AsyncStateMachine: Dispatcher cannot be null");
        }
    }

    /**
     * @brief Constructor with dispatcher and UserContext
     *
     * @tparam UserContext User-defined context type
     * @param dispatcher Event dispatcher for async processing
     * @param context User context for state machine
     *
     * @throws std::invalid_argument if dispatcher is null
     */
    template <typename UserContext>
    AsyncStateMachine(std::shared_ptr<SCE::Dispatchers::IEventDispatcher> dispatcher, UserContext &context)
        : dispatcher_(std::move(dispatcher)), sm_(context) {
        if (!dispatcher_) {
            throw std::invalid_argument("AsyncStateMachine: Dispatcher cannot be null");
        }
    }

    // Disable copy (state machine is not copyable)
    AsyncStateMachine(const AsyncStateMachine &) = delete;
    AsyncStateMachine &operator=(const AsyncStateMachine &) = delete;

    // Disable move (state machine may not be movable)
    AsyncStateMachine(AsyncStateMachine &&) = delete;
    AsyncStateMachine &operator=(AsyncStateMachine &&) = delete;

    /**
     * @brief Initialize state machine
     *
     * Synchronously initializes the state machine. Must be called
     * before posting events.
     *
     * @note This is synchronous - runs on caller's thread
     */
    void initialize() {
        sm_.initialize();
    }

    /**
     * @brief Post event asynchronously
     *
     * Enqueues event for processing on dispatcher thread.
     * Thread-safe: Can be called from any thread.
     *
     * The event will be:
     * 1. Queued in dispatcher
     * 2. Raised to state machine external queue
     * 3. Processed via step() on dispatcher thread
     *
     * @param event Event to post
     *
     * @throws std::runtime_error if dispatcher not running
     */
    void postEvent(EventType event) {
        if (!dispatcher_->isRunning()) {
            throw std::runtime_error("AsyncStateMachine: Cannot post event, dispatcher not running");
        }

        dispatcher_->enqueue([this, event]() {
            sm_.raiseExternal(event);
            sm_.step();
        });
    }

    /**
     * @brief Post event with data asynchronously
     *
     * Enqueues event with data for processing on dispatcher thread.
     * Thread-safe: Can be called from any thread.
     *
     * @param event Event to post
     * @param data Event data (copied for thread safety)
     *
     * @throws std::runtime_error if dispatcher not running
     */
    void postEvent(EventType event, std::string data) {
        if (!dispatcher_->isRunning()) {
            throw std::runtime_error("AsyncStateMachine: Cannot post event, dispatcher not running");
        }

        // Capture data by value for thread safety
        dispatcher_->enqueue([this, event, data = std::move(data)]() {
            sm_.raiseExternal(event, data);
            sm_.step();
        });
    }

    /**
     * @brief Get current state
     *
     * Returns current state of state machine.
     *
     * @return Current state
     *
     * @warning NOT thread-safe. Only call when dispatcher is stopped
     *          or from within a task enqueued on the dispatcher.
     *          Reading from another thread while dispatcher is running
     *          causes data race (undefined behavior).
     */
    auto getCurrentState() const {
        return sm_.getCurrentState();
    }

    /**
     * @brief Check if state machine is in final state
     *
     * @return true if in final state
     *
     * @warning NOT thread-safe. Only call when dispatcher is stopped
     *          or from within a task enqueued on the dispatcher.
     *          Reading from another thread while dispatcher is running
     *          causes data race (undefined behavior).
     */
    bool isInFinalState() const {
        return sm_.isInFinalState();
    }

    /**
     * @brief Get reference to underlying state machine
     *
     * Provides direct access to state machine for synchronous operations.
     *
     * @return Reference to wrapped state machine
     *
     * @warning Direct access bypasses async safety. Only use when
     *          dispatcher is not running or for read-only operations.
     */
    StateMachine &getStateMachine() {
        return sm_;
    }

    /**
     * @brief Get const reference to underlying state machine
     *
     * @return Const reference to wrapped state machine
     */
    const StateMachine &getStateMachine() const {
        return sm_;
    }

    /**
     * @brief Get dispatcher
     *
     * @return Shared pointer to dispatcher
     */
    std::shared_ptr<SCE::Dispatchers::IEventDispatcher> getDispatcher() const {
        return dispatcher_;
    }

private:
    std::shared_ptr<SCE::Dispatchers::IEventDispatcher> dispatcher_;
    StateMachine sm_;
};

}  // namespace SCE::Wrappers
