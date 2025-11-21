#pragma once

#include "StateMachine.h"
#include "events/IEventDispatcher.h"
#include "runtime/IEventRaiser.h"
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Builder pattern for StateMachine construction with dependency injection
 *
 * Builder creates StateMachine with injected dependencies.
 * Caller is responsible for wrapping in StateMachineContext and managing
 * EventRaiser/EventDispatcher lifecycle separately.
 */
class StateMachineBuilder {
private:
    std::shared_ptr<IEventDispatcher> eventDispatcher_;
    std::shared_ptr<IEventRaiser> eventRaiser_;
    std::string sessionId_;
    SchedulerMode schedulerMode_ = SchedulerMode::AUTOMATIC;  // W3C SCXML 3.13: Default to normal mode

public:
    StateMachineBuilder() = default;

    /**
     * @brief Set EventDispatcher for send actions and delayed events
     * @param eventDispatcher Shared pointer to event dispatcher
     * @return Reference to builder for method chaining
     */
    StateMachineBuilder &withEventDispatcher(std::shared_ptr<IEventDispatcher> eventDispatcher) {
        eventDispatcher_ = eventDispatcher;
        return *this;
    }

    /**
     * @brief Set EventRaiser for raise actions and internal events
     * @param eventRaiser Shared pointer to event raiser
     * @return Reference to builder for method chaining
     */
    StateMachineBuilder &withEventRaiser(std::shared_ptr<IEventRaiser> eventRaiser) {
        eventRaiser_ = eventRaiser;
        return *this;
    }

    /**
     * @brief Set session ID for StateMachine (required for invoke scenarios)
     * @param sessionId Pre-existing session ID to use
     * @return Reference to builder for method chaining
     */
    StateMachineBuilder &withSessionId(const std::string &sessionId) {
        sessionId_ = sessionId;
        return *this;
    }

    /**
     * @brief Set scheduler mode for parent-child inheritance (W3C SCXML 3.13)
     *
     * Enables parent state machine to propagate MANUAL mode to child invoke sessions
     * for interactive debugging with time-travel support.
     *
     * @param schedulerMode AUTOMATIC for normal execution, MANUAL for interactive debugging
     * @return Reference to builder for method chaining
     */
    StateMachineBuilder &withSchedulerMode(SchedulerMode schedulerMode) {
        schedulerMode_ = schedulerMode;
        return *this;
    }

    /**
     * @brief Build StateMachine with dependency injection
     *
     * Returns StateMachine shared_ptr for callback safety.
     * Caller is responsible for wrapping in StateMachineContext and managing
     * EventRaiser/EventDispatcher lifecycle (e.g., via TestResources).
     *
     * @return Shared pointer to StateMachine (for callback safety)
     * @throws std::runtime_error if required dependencies are missing
     */
    std::shared_ptr<StateMachine> build() {
        std::shared_ptr<StateMachine> stateMachine;

        // Create StateMachine with or without session ID
        if (!sessionId_.empty()) {
            stateMachine = std::make_shared<StateMachine>(sessionId_);
        } else {
            stateMachine = std::make_shared<StateMachine>();
        }

        // Inject dependencies after construction
        if (eventDispatcher_) {
            stateMachine->setEventDispatcher(eventDispatcher_);
        }

        if (eventRaiser_) {
            stateMachine->setEventRaiser(eventRaiser_);

            // W3C SCXML 3.13: Apply scheduler mode for parent-child inheritance
            // Get scheduler from EventRaiser and set mode (MANUAL for interactive debugging)
            auto scheduler = eventRaiser_->getScheduler();
            if (scheduler) {
                scheduler->setMode(schedulerMode_);
            }
        }

        return stateMachine;
    }
};

}  // namespace SCE