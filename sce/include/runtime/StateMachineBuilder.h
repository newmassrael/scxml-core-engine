// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "StateMachine.h"
#include "events/IEventDispatcher.h"
#include "runtime/IEventRaiser.h"
#include <memory>
#include <optional>
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
    IScriptEngine *scriptEngine_ = nullptr;
    std::shared_ptr<IEventDispatcher> eventDispatcher_;
    std::shared_ptr<IEventRaiser> eventRaiser_;
    std::string sessionId_;
    std::string basicHttpAccessUri_;
    /// Absent means the builder leaves the scheduler's mode as its owner set it. A scheduler can
    /// be shared (a child session built for `<invoke>` uses its parent's), and a builder that
    /// wrote a default into it would switch the parent's MANUAL clock back to the wall clock.
    std::optional<SchedulerMode> schedulerMode_;

public:
    StateMachineBuilder() = default;

    /**
     * @brief Set script engine for pluggable engine support
     * @param scriptEngine Script engine implementation (QuickJS, Lua, etc.)
     * @return Reference to builder for method chaining
     */
    StateMachineBuilder &withScriptEngine(IScriptEngine &scriptEngine) {
        scriptEngine_ = &scriptEngine;
        return *this;
    }

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
     * @brief Declare the inbound BasicHTTP endpoint serving the session
     *
     * §scxml-C-2-3: publishes the Basic HTTP Event I/O Processor entry of
     * `_ioprocessors` with this URI as its 'location'. Omit it when no
     * BasicHTTP listener serves the session, and no entry is published.
     *
     * @param accessUri URI external components post events to
     * @return Reference to builder for method chaining
     */
    StateMachineBuilder &withBasicHttpAccessUri(const std::string &accessUri) {
        basicHttpAccessUri_ = accessUri;
        return *this;
    }

    /**
     * @brief Set scheduler mode for parent-child inheritance
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
    std::shared_ptr<StateMachine> build();
};

}  // namespace SCE
