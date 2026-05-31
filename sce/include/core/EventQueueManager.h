// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <deque>
#include <functional>
#include <stdexcept>
#include <string>

#ifdef SCE_THREAD_SAFE
#include <mutex>
#endif

namespace SCE::Core {

/**
 * @brief §scxml-3.13: Internal Event Queue Management
 *
 * This class implements the W3C SCXML internal event queue semantics.
 * Internal events are placed at the back of the queue and processed
 * in FIFO order before external events (macrostep completion).
 *
 * Design Goals:
 * - Single source of truth for event queue logic
 * - Shared between static and interpreter engines
 * - Zero overhead when not used (inline methods)
 * - Template-based for type safety
 * - Optional thread safety via SCE_THREAD_SAFE build flag
 *
 * Thread Safety:
 * - When SCE_THREAD_SAFE is defined, all operations are protected by mutex
 * - Allows safe event submission from multiple threads
 * - Zero overhead when thread safety is disabled (default)
 *
 * W3C SCXML References:
 * - Section 3.12.1: Internal Events
 * - §scxml-D: Algorithm for SCXML Interpretation
 */
template <typename EventType = std::string> class EventQueueManager {
public:
    /**
     * @brief Raise an internal event (§scxml-D-mainEventLoop)
     *
     * Internal events are placed at the back of the internal event queue.
     * They are processed before external events but after currently queued
     * internal events (FIFO ordering).
     *
     * Thread-safe when SCE_THREAD_SAFE is enabled.
     *
     * @param event Event to raise internally
     */
    void raise(const EventType &event) {
#ifdef SCE_THREAD_SAFE
        std::lock_guard<std::mutex> lock(mutex_);
#endif
        queue_.push_back(event);
    }

    /**
     * @brief Check if internal queue has events
     *
     * Thread-safe when SCE_THREAD_SAFE is enabled.
     *
     * @return true if queue is not empty
     */
    bool hasEvents() const {
#ifdef SCE_THREAD_SAFE
        std::lock_guard<std::mutex> lock(mutex_);
#endif
        return !queue_.empty();
    }

    /**
     * @brief Get number of queued events
     *
     * Thread-safe when SCE_THREAD_SAFE is enabled.
     *
     * @return Number of events in queue
     */
    size_t size() const {
#ifdef SCE_THREAD_SAFE
        std::lock_guard<std::mutex> lock(mutex_);
#endif
        return queue_.size();
    }

    /**
     * @brief Pop next internal event from queue (FIFO)
     *
     * Thread-safe when SCE_THREAD_SAFE is enabled.
     *
     * @return Next event from front of queue
     * @throws std::runtime_error if queue is empty
     */
    EventType pop() {
#ifdef SCE_THREAD_SAFE
        std::lock_guard<std::mutex> lock(mutex_);
#endif
        if (queue_.empty()) {
            throw std::runtime_error("EventQueueManager: Cannot pop from empty queue");
        }
        EventType event = queue_.front();
        queue_.pop_front();
        return event;
    }

    /**
     * @brief Clear all queued events
     *
     * Thread-safe when SCE_THREAD_SAFE is enabled.
     */
    void clear() {
#ifdef SCE_THREAD_SAFE
        std::lock_guard<std::mutex> lock(mutex_);
#endif
        queue_.clear();
    }

    /**
     * @brief Process all internal events with a handler (§scxml-D-mainEventLoop)
     *
     * Processes all queued internal events in FIFO order. This implements
     * the macrostep completion logic where all internal events generated
     * during state entry are processed before external events.
     *
     * The handler is called for each event and should return true if a
     * transition was taken, false otherwise.
     *
     * Thread-safe when SCE_THREAD_SAFE is enabled.
     *
     * @param handler Function to process each event: bool handler(EventType)
     */
    template <typename Handler> void processAll(Handler handler) {
#ifdef SCE_THREAD_SAFE
        std::lock_guard<std::mutex> lock(mutex_);
#endif
        while (!queue_.empty()) {
            EventType event = queue_.front();
            queue_.pop_front();
            handler(event);
        }
    }

private:
    std::deque<EventType> queue_;  // FIFO ordering per §scxml-3.13

#ifdef SCE_THREAD_SAFE
    mutable std::mutex mutex_;  // Mutex for thread-safe operations (mutable for const methods)
#endif
};

}  // namespace SCE::Core
