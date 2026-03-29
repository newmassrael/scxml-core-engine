#pragma once

#include "runtime/EventRaiserImpl.h"
#include <memory>

namespace SCE::Core {

/**
 * @brief Internal event queue adapter for Interpreter engine
 *
 * Wraps EventRaiserImpl with unified interface
 * usable by EventProcessingAlgorithms.
 * Satisfies EventQueueAdapter concept (core/EventQueueConcept.h).
 *
 * @note Since EventRaiserImpl's processNextQueuedEvent() processes
 *       events internally via callback, popNext() returns only
 *       processing success status, not actual event value.
 *
 * @example
 * @code
 * std::shared_ptr<EventRaiserImpl> eventRaiser_;
 * InterpreterEventQueue adapter(eventRaiser_);
 *
 * EventProcessingAlgorithms::processInternalEventQueue(
 *     adapter,
 *     [](bool) { return true; }  // EventRaiser handles internally
 * );
 * @endcode
 */
class InterpreterEventQueue {
public:
    /**
     * @brief Constructor
     * @param raiser EventRaiserImpl shared_ptr
     */
    explicit InterpreterEventQueue(std::shared_ptr<EventRaiserImpl> raiser) : raiser_(raiser) {}

    /**
     * @brief Check if queue has events
     * @return true if queue has events
     */
    bool hasEvents() const {
        return raiser_ && raiser_->hasQueuedEvents();
    }

    /**
     * @brief Process next event from queue
     *
     * Calls EventRaiserImpl::processNextQueuedEvent() to
     * process events internally via callback.
     *
     * @return Processing success status (does not return actual event value)
     */
    bool popNext() {
        return raiser_ && raiser_->processNextQueuedEvent();
    }

private:
    std::shared_ptr<EventRaiserImpl> raiser_;
};

}  // namespace SCE::Core
