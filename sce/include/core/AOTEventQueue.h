// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "core/EventQueueManager.h"

namespace SCE::Core {

/**
 * @brief Internal event queue adapter for AOT engine
 *
 * Wraps EventQueueManager<Event> with unified interface
 * usable by EventProcessingAlgorithms.
 * Satisfies EventQueueAdapter concept (core/EventQueueConcept.h).
 *
 * @tparam EventType Event type (usually enum class Event)
 *
 * @example
 * @code
 * EventQueueManager<Event> eventQueue_;
 * AOTEventQueue<Event> adapter(eventQueue_);
 *
 * EventProcessingAlgorithms::processInternalEventQueue(
 *     adapter,
 *     [this](Event e) { return processInternalEvent(e); }
 * );
 * @endcode
 */
template <typename EventType> class AOTEventQueue {
public:
    /**
     * @brief Constructor
     * @param queue EventQueueManager reference
     */
    explicit AOTEventQueue(EventQueueManager<EventType> &queue) : queue_(queue) {}

    /**
     * @brief Check if queue has events
     * @return true if queue has events
     */
    bool hasEvents() const {
        return queue_.hasEvents();
    }

    /**
     * @brief Pop next event from queue (FIFO)
     * @return Popped event
     */
    EventType popNext() {
        return queue_.pop();
    }

private:
    EventQueueManager<EventType> &queue_;
};

}  // namespace SCE::Core
