// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "core/EventOrigin.h"
#include "core/LogMacros.h"
#include "scripting/IScriptEngine.h"
#include <optional>
#include <string>

namespace SCE {

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
     * @brief Execute finalize script with _event context (§scxml-6.5)
     *
     * Single Source of Truth for finalize execution logic.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * Usage:
     * - Interpreter: StateMachine::processEvent() (sce/src/runtime/StateMachine.cpp)
     * - AOT: executeFinalizeForChildEvent() (tools/codegen/templates/utility_methods.jinja2)
     *
     * §scxml-6.5 (test 233): "If there is a finalize handler in the instance of invoke
     * that created the service that generated the event, the SCXML Processor MUST execute
     * the code in that finalize handler right before it removes the event from the event
     * queue for processing."
     *
     * §scxml-5.10 (test 233): Finalize scripts have access to _event.data fields.
     * The _event variable must be set BEFORE finalize execution.
     *
     * @param jsEngine JSEngine instance for script execution
     * @param sessionId JSEngine session ID
     * @param finalizeScript Finalize script to execute
     * @param eventName Event name for _event.name
     * @param eventData Event data for _event.data (JSON string)
     * @param sendId Event sendid for _event.sendid
     * @param origin The event's reply address, read as _event.origin and
     *        _event.origintype; none reads both empty (docs/adr/0090)
     * @param invokeId Event invokeid for _event.invokeid
     * @return true if successful, false on error
     */
    static bool executeFinalizeWithEvent(IScriptEngine &jsEngine, const std::string &sessionId,
                                         const ScriptSource &finalizeScript, const std::string &eventName,
                                         const std::string &eventData, const std::string &sendId = "",
                                         const std::optional<Core::EventOrigin> &origin = std::nullopt,
                                         const std::string &invokeId = "") {
        SCE_LOG_DEBUG("FinalizeHelper: Executing finalize for event '{}' with data: '{}'", eventName, eventData);

        // §scxml-6.5: Set _event BEFORE finalize execution
        // Finalize scripts need access to _event.data.fieldName (test 233)
        //
        // Checked rather than discarded: if the binding fails the finalize
        // script still runs, against a stale or absent `_event`, and reports
        // success. That is the same failure the sibling call below refuses.
        auto eventResult =
            jsEngine
                .setCurrentEvent(sessionId, SetCurrentEventArgs{eventName, eventData, "external", sendId,
                                                                Core::originAddressOf(origin),
                                                                Core::originProcessorOf(origin), invokeId})
                .get();
        if (!eventResult.status.isSuccess()) {
            SCE_LOG_ERROR("FinalizeHelper: Failed to bind _event before finalize: {}",
                          eventResult.status.getErrorMessage());
            return false;
        }

        // Execute finalize script
        auto result = jsEngine.executeScript(sessionId, finalizeScript).get();

        if (!result.isSuccess()) {
            SCE_LOG_ERROR("FinalizeHelper: Script execution failed: {}", result.getErrorMessage());
            return false;
        }

        SCE_LOG_DEBUG("FinalizeHelper: Finalize executed successfully for event '{}'", eventName);
        return true;
    }
};

}  // namespace SCE
