// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of RSM (Reactive State Machine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE

#pragma once

#include "common/Logger.h"
#include "scripting/JSEngine.h"
#include <string>

namespace RSM {

/**
 * @brief Helper functions for W3C SCXML finalize handler execution
 *
 * Single Source of Truth for finalize execution logic shared between:
 * - Interpreter engine (StateMachine.cpp)
 * - AOT engine (generated code via templates/utility_methods.jinja2)
 *
 * W3C SCXML References:
 * - 6.5: Finalize element semantics
 * - 5.10: Event data access in finalize handlers
 */
class FinalizeHelper {
public:
    /**
     * @brief Execute finalize script with _event context (W3C SCXML 6.5)
     *
     * Single Source of Truth for finalize execution logic.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * Usage:
     * - Interpreter: StateMachine::processEvent() (rsm/src/runtime/StateMachine.cpp)
     * - AOT: executeFinalizeForChildEvent() (tools/codegen/templates/utility_methods.jinja2)
     *
     * W3C SCXML 6.5 (test 233): "If there is a finalize handler in the instance of invoke
     * that created the service that generated the event, the SCXML Processor MUST execute
     * the code in that finalize handler right before it removes the event from the event
     * queue for processing."
     *
     * W3C SCXML 5.10 (test 233): Finalize scripts have access to _event.data fields.
     * The _event variable must be set BEFORE finalize execution.
     *
     * @param jsEngine JSEngine instance for script execution
     * @param sessionId JSEngine session ID
     * @param finalizeScript Finalize script to execute
     * @param eventName Event name for _event.name
     * @param eventData Event data for _event.data (JSON string)
     * @param sendId Event sendid for _event.sendid
     * @param origin Event origin for _event.origin
     * @param originType Event origin type for _event.origintype
     * @param invokeId Event invokeid for _event.invokeid
     * @return true if successful, false on error
     */
    static bool executeFinalizeWithEvent(JSEngine &jsEngine, const std::string &sessionId,
                                         const std::string &finalizeScript, const std::string &eventName,
                                         const std::string &eventData, const std::string &sendId = "",
                                         const std::string &origin = "", const std::string &originType = "",
                                         const std::string &invokeId = "") {
        LOG_DEBUG("FinalizeHelper: Executing finalize for event '{}' with data: '{}'", eventName, eventData);

        // W3C SCXML 6.5: Set _event BEFORE finalize execution
        // Finalize scripts need access to _event.data.fieldName (test 233)
        jsEngine.setCurrentEvent(sessionId, eventName, eventData, "external", sendId, origin, originType, invokeId)
            .get();

        // Execute finalize script
        auto result = jsEngine.executeScript(sessionId, finalizeScript).get();

        if (!JSEngine::isSuccess(result)) {
            LOG_ERROR("FinalizeHelper: Script execution failed: {}", JSEngine::resultToString(result));
            return false;
        }

        LOG_DEBUG("FinalizeHelper: Finalize executed successfully for event '{}'", eventName);
        return true;
    }
};

}  // namespace RSM
