// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
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
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/SCXMLConstants.h"
#include "common/UniqueIdGenerator.h"
#include "common/UrlEncodingHelper.h"
#include <map>
#include <string>
#include <vector>

namespace SCE {

/**
 * @brief Helper functions for W3C SCXML <send> element processing
 *
 * Single Source of Truth for send action validation logic shared between:
 * - Interpreter engine (ActionExecutorImpl)
 * - AOT engine (StaticCodeGenerator)
 *
 * W3C SCXML References:
 * - 6.2: Send element semantics
 * - 5.10: Error handling for send
 */
class SendHelper {
public:
    /**
     * @brief Check if target validation failed (for error.execution detection)
     *
     * Single Source of Truth for target validation logic.
     * Used by AOT engine to determine if error.execution should be raised,
     * which stops execution of subsequent executable content per W3C SCXML 5.10.
     *
     * W3C SCXML 6.2: Target values starting with "!" are invalid.
     *
     * @param target Target to check
     * @return true if target is invalid (starts with '!')
     */
    static bool isInvalidTarget(const std::string &target) {
        // W3C SCXML 6.2: Target values starting with "!" are invalid
        return !target.empty() && target[0] == '!';
    }

    /**
     * @brief Check if target should use internal event queue (W3C SCXML C.1)
     *
     * Single Source of Truth for internal queue routing logic.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * Usage:
     * - Interpreter: EventTargetFactoryImpl::createTarget() (sce/src/events/EventTargetFactoryImpl.cpp)
     * - AOT: StaticCodeGenerator send.jinja2 template (tools/codegen/templates/actions/send.jinja2)
     *
     * W3C SCXML C.1 (test189, test495): Events with target="#_internal" must go to
     * the internal event queue, which has higher priority than external queue.
     *
     * @param target Target to check
     * @return true if target is "#_internal" (internal queue), false otherwise (external queue)
     */
    static bool isInternalTarget(const std::string &target) {
        // W3C SCXML C.1: #_internal indicates internal event queue
        return target == "#_internal";
    }

    /**
     * @brief Check if target is child invoke session (W3C SCXML 6.4)
     *
     * Single Source of Truth for child invoke target detection logic.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * Usage:
     * - Interpreter: EventTargetFactoryImpl::createTarget() (sce/src/events/EventTargetFactoryImpl.cpp)
     * - AOT: StaticCodeGenerator send.jinja2 template (tools/codegen/templates/actions/send.jinja2)
     *
     * W3C SCXML 6.4 (test192): Events with target="#_<invokeid>" must be routed
     * to the child state machine identified by <invokeid>.
     *
     * Examples:
     * - "#_invokedChild" → child invoke target (returns true)
     * - "#_parent" → parent target (returns false)
     * - "#_internal" → internal queue (returns false)
     * - "#_scxml_sessionid" → SCXML session target (returns false)
     *
     * @param target Target to check
     * @return true if target is child invoke (#_<invokeid>), false otherwise
     */
    static bool isChildInvokeTarget(const std::string &target) {
        // W3C SCXML 6.4: #_<invokeid> format indicates child invoke target
        // Must start with #_ but not be #_parent or #_internal
        if (!target.starts_with("#_")) {
            return false;
        }
        // Exclude special reserved targets
        if (target == "#_parent" || target == "#_internal") {
            return false;
        }
        // Exclude SCXML session targets (#_scxml_<sessionid>)
        if (target.starts_with("#_scxml_")) {
            return false;
        }
        // All other #_<invokeid> are child invoke targets
        return true;
    }

    /**
     * @brief Extract invoke ID from child target (W3C SCXML 6.4)
     *
     * Single Source of Truth for invoke ID extraction logic.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * W3C SCXML 6.4 (test192): Extract invokeid from target="#_<invokeid>".
     *
     * Example:
     * - "#_invokedChild" → "invokedChild"
     * - "#_myChild" → "myChild"
     *
     * @param target Child invoke target (must start with "#_")
     * @return Invoke ID (substring after "#_")
     */
    static std::string extractInvokeId(const std::string &target) {
        // W3C SCXML 6.4: Extract invokeid from #_<invokeid>
        if (target.starts_with("#_")) {
            return target.substr(2);  // Skip "#_" prefix
        }
        return target;  // Fallback: return as-is
    }

    /**
     * @brief Check if target is HTTP URL (W3C SCXML C.2)
     *
     * Single Source of Truth for HTTP target detection logic.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * Usage:
     * - Interpreter: EventTargetFactoryImpl::createTarget() (sce/src/events/EventTargetFactoryImpl.cpp)
     * - AOT: StaticCodeGenerator send.jinja2 template (tools/codegen/templates/actions/send.jinja2)
     *
     * W3C SCXML C.2 (test509, test510, test513): BasicHTTP Event I/O Processor
     * accepts HTTP/HTTPS URLs as targets. Events sent to HTTP targets must:
     * - Use external event queue (not internal)
     * - Trigger HTTP POST request to target URL
     * - Validate HTTP 200 OK response (test513)
     *
     * ARCHITECTURE.md Static Hybrid Strategy:
     * HTTP URL targets are compatible with Static/Static Hybrid approach because:
     * - Target URL is known at compile-time (static string)
     * - HTTP infrastructure is external (W3CHttpTestServer)
     * - No engine mixing: AOT state machine + external HTTP server
     *
     * @param target Target to check
     * @return true if target is HTTP/HTTPS URL, false otherwise
     */
    static bool isHttpTarget(const std::string &target) {
        // W3C SCXML C.2: HTTP/HTTPS URLs indicate BasicHTTP Event I/O Processor
        return target.starts_with("http://") || target.starts_with("https://");
    }

    /**
     * @brief Validate send target according to W3C SCXML 6.2
     *
     * W3C SCXML 6.2 (tests 159, 194): Invalid target values (e.g., starting with "!")
     * must raise error.execution and stop subsequent executable content.
     *
     * This function reuses isInvalidTarget() to avoid code duplication.
     *
     * @param target Target string to validate
     * @param errorMsg Output parameter for error message if validation fails
     * @return true if valid, false if invalid (error.execution should be raised)
     */
    static bool validateTarget(const std::string &target, std::string &errorMsg) {
        if (isInvalidTarget(target)) {
            errorMsg = "Invalid target value: " + target;
            return false;
        }
        return true;
    }

    /**
     * @brief Check if target is unreachable or inaccessible (W3C SCXML C.1)
     *
     * Single Source of Truth for unreachable target detection logic.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * Usage:
     * - Interpreter: ActionExecutorImpl::executeSendAction() (sce/src/runtime/ActionExecutorImpl.cpp)
     * - AOT: StaticCodeGenerator send.jinja2 template (tools/codegen/templates/actions/send.jinja2)
     *
     * W3C SCXML C.1 (test 496): Empty or "undefined" target evaluation results
     * indicate unreachable or inaccessible target sessions, requiring error.communication.
     *
     * @param target Target string evaluated from targetexpr
     * @return true if target is unreachable (empty or "undefined"), false otherwise
     */
    static bool isUnreachableTarget(const std::string &target) {
        // W3C SCXML C.1: Empty or "undefined" targets are unreachable
        return target.empty() || target == "undefined";
    }

    /**
     * @brief Check if send type requires target attribute (W3C SCXML C.2)
     *
     * Single Source of Truth for BasicHTTP target requirement validation.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * Usage:
     * - Interpreter: ActionExecutorImpl::executeSendAction() (sce/src/runtime/ActionExecutorImpl.cpp)
     * - AOT: StaticCodeGenerator send.jinja2 template (tools/codegen/templates/actions/send.jinja2)
     *
     * W3C SCXML C.2 (test 577): BasicHTTP Event I/O Processor requires
     * target URL attribute. Missing target raises error.communication.
     *
     * @param sendType Event I/O Processor type URI
     * @return true if send type requires target attribute (BasicHTTP), false otherwise
     */
    static bool requiresTargetAttribute(const std::string &sendType) {
        // W3C SCXML C.2: BasicHTTP Event I/O Processor requires target URL
        return sendType == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor";
    }

    /**
     * @brief Check if send type is supported (W3C SCXML 6.2)
     *
     * Single Source of Truth for send type validation logic.
     * ARCHITECTURE.md: Zero Duplication - used by both Interpreter and AOT engines.
     *
     * Usage:
     * - Interpreter: ActionExecutorImpl::executeSendAction() (sce/src/runtime/ActionExecutorImpl.cpp)
     * - AOT: StaticCodeGenerator send.jinja2 template (tools/codegen/templates/actions/send.jinja2)
     *
     * W3C SCXML 6.2 (test 199): If the SCXML Processor does not support the type
     * that is specified in <send>, it MUST place the event error.execution on the
     * internal event queue.
     *
     * Supported Event I/O Processor types:
     * - http://www.w3.org/TR/scxml/#SCXMLEventProcessor (default)
     * - http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor
     *
     * @param sendType Event I/O Processor type URI (empty = default SCXMLEventProcessor)
     * @return true if send type is supported, false if unsupported (error.execution required)
     */
    static bool isSupportedSendType(const std::string &sendType) {
        // W3C SCXML 6.2: Empty or default type is SCXMLEventProcessor (always supported)
        if (sendType.empty() || sendType == Constants::SCXML_EVENT_PROCESSOR_TYPE) {
            return true;
        }
        // W3C SCXML C.2: BasicHTTP Event I/O Processor is supported
        if (sendType == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor") {
            return true;
        }
        // W3C SCXML 6.2: All other types are unsupported
        return false;
    }

    /**
     * @brief Validate BasicHTTP send parameters (W3C SCXML C.2)
     *
     * Single Source of Truth for BasicHTTP validation logic.
     * Checks if required target attribute is present for event processors that require it.
     *
     * @param sendType Event I/O Processor type URI
     * @param target Static target value (empty if not specified)
     * @param targetExpr Dynamic target expression (empty if not specified)
     * @param errorMsg Output parameter for error message if validation fails
     * @return true if valid, false if validation failed (error.communication should be raised)
     */
    static bool validateBasicHttpSend(const std::string &sendType, const std::string &target,
                                      const std::string &targetExpr, std::string &errorMsg) {
        if (requiresTargetAttribute(sendType) && target.empty() && targetExpr.empty()) {
            errorMsg = "BasicHTTPEventProcessor requires target attribute";
            return false;
        }
        return true;
    }

    /**
     * @brief Generate unique sendid (Single Source of Truth)
     *
     * Used by both Interpreter and AOT engines to ensure consistent sendid format.
     * Delegates to centralized UniqueIdGenerator for thread-safe, collision-free IDs.
     *
     * W3C SCXML 6.2: Each send action must have a unique sendid for tracking.
     *
     * @return Unique sendid string (format: "send_timestamp_counter")
     */
    static std::string generateSendId() {
        return UniqueIdGenerator::generateSendId();
    }

    /**
     * @brief Send event to parent state machine (Single Source of Truth)
     *
     * W3C SCXML 6.2: Handles <send target="#_parent"> semantics for child-to-parent
     * event communication in invoked state machines.
     *
     * Single Source of Truth for #_parent event routing shared between:
     * - Interpreter engine (ParentEventTarget)
     * - AOT engine (StaticCodeGenerator generated code)
     *
     * This template function enables compile-time type-safe parent event routing
     * in statically generated code while maintaining zero-overhead abstraction.
     *
     * @tparam ParentStateMachine Parent state machine type (CRTP)
     * @tparam EventType Parent's Event enum type
     * @param parent Pointer to parent state machine (nullptr-safe)
     * @param event Event to send to parent
     * @return true if event was sent successfully, false if parent is null
     */
    template <typename ParentStateMachine, typename EventType>
    static bool sendToParent(ParentStateMachine *parent, EventType event) {
        if (parent) {
            // W3C SCXML 6.2: Send to parent's external event queue
            parent->raiseExternal(event);
            return true;
        }
        return false;
    }

    /**
     * @brief Send event to parent with invokeid metadata (W3C SCXML 6.4.1)
     *
     * W3C SCXML 6.4.1 (test338): When a child sends an event to its parent,
     * the _event.invokeid field must be set to the invokeid of the invoke
     * that created the child.
     *
     * @param parent Parent state machine pointer
     * @param event Event to send
     * @param invokeId Invokeid of the child (from parent's invoke element)
     * @return true if event was sent successfully, false if parent is null
     */
    template <typename ParentStateMachine, typename EventType>
    static bool sendToParent(ParentStateMachine *parent, EventType event, const std::string &invokeId) {
        LOG_DEBUG("SendHelper::sendToParent called - parent={}, event={}, invokeId={}", (void *)parent,
                  static_cast<int>(event), invokeId);
        if (parent) {
            // W3C SCXML 6.4.1: Create event with invokeid metadata
            typename ParentStateMachine::EventWithMetadata eventWithMetadata(event);
            eventWithMetadata.invokeId = invokeId;

            // W3C SCXML 6.2: Send to parent's external event queue
            LOG_DEBUG("SendHelper::sendToParent - calling parent->raiseExternal()");
            parent->raiseExternal(eventWithMetadata);
            LOG_DEBUG("SendHelper::sendToParent - parent->raiseExternal() completed");
            return true;
        }
        LOG_DEBUG("SendHelper::sendToParent - parent is nullptr, not sending event");
        return false;
    }

    /**
     * @brief Send event to parent state machine with origin (W3C SCXML 6.5 finalize)
     *
     * W3C SCXML 6.5: Set origin to child session ID for finalize execution.
     * Finalize runs BEFORE the event is processed, with access to _event.data.
     *
     * @tparam ParentStateMachine Parent state machine type
     * @tparam EventType Parent's event enum type
     * @param parent Pointer to parent state machine
     * @param event Event to send
     * @param invokeId Invokeid of the child (from parent's invoke element)
     * @param childSessionId Child's session ID for finalize origin matching
     * @return true if event was sent successfully, false if parent is null
     */
    template <typename ParentStateMachine, typename EventType>
    static bool sendToParentWithOrigin(ParentStateMachine *parent, EventType event, const std::string &invokeId,
                                       const std::string &childSessionId, const std::string &eventData = "") {
        LOG_DEBUG("SendHelper::sendToParentWithOrigin called - parent={}, event={}, invokeId={}, childSessionId={}, "
                  "eventData='{}'",
                  (void *)parent, static_cast<int>(event), invokeId, childSessionId, eventData);
        if (parent) {
            // W3C SCXML 6.4.1: Create event with invokeid metadata
            // W3C SCXML 6.5: Add origin (child session ID) for finalize support
            // W3C SCXML 5.10: Add event data from params/namelist (test 233)
            typename ParentStateMachine::EventWithMetadata eventWithMetadata(event, eventData);
            eventWithMetadata.invokeId = invokeId;
            eventWithMetadata.origin = childSessionId;  // W3C SCXML 6.5: For finalize matching
            eventWithMetadata.originType =
                SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE;  // W3C SCXML C.1: SCXML Event I/O Processor (test 253)

            // W3C SCXML 6.2: Send to parent's external event queue
            LOG_DEBUG("SendHelper::sendToParentWithOrigin - calling parent->raiseExternal()");
            parent->raiseExternal(eventWithMetadata);
            LOG_DEBUG("SendHelper::sendToParentWithOrigin - parent->raiseExternal() completed");
            return true;
        }
        LOG_DEBUG("SendHelper::sendToParentWithOrigin - parent is nullptr, not sending event");
        return false;
    }

    /**
     * @brief Store sendid in idlocation variable (Single Source of Truth)
     *
     * W3C SCXML 6.2.4 (test 183): The idlocation attribute specifies a variable
     * where the generated sendid should be stored for later reference.
     *
     * This method encapsulates the idlocation storage logic shared between:
     * - Interpreter engine (ActionExecutorImpl::executeSendAction)
     * - AOT engine (StaticCodeGenerator::generateActionCode for SEND)
     *
     * @param jsEngine JSEngine instance for variable operations
     * @param sessionId Session identifier
     * @param idLocation Variable name to store sendid (empty = no storage)
     * @param sendId Generated sendid value to store
     */
    template <typename JSEngineType>
    static void storeInIdLocation(JSEngineType &jsEngine, const std::string &sessionId, const std::string &idLocation,
                                  const std::string &sendId) {
        if (!idLocation.empty()) {
            jsEngine.setVariable(sessionId, idLocation, sendId);
        }
    }

    /**
     * @brief Build HTTP POST body with W3C SCXML C.2 compliance
     *
     * Single Source of Truth for HTTP POST body generation shared between:
     * - Interpreter engine (HttpEventTarget::send)
     * - AOT engine (StaticExecutionEngine::raiseExternal)
     *
     * W3C SCXML C.2 (test 531): Ensures "single instance" of _scxmleventname parameter.
     * When eventName is present, it becomes the _scxmleventname parameter, and any
     * _scxmleventname in params is skipped to avoid duplication.
     *
     * ARCHITECTURE.md: Zero Duplication Principle
     * - Both Interpreter and AOT use this function for identical HTTP POST body generation
     * - Single Source of Truth eliminates behavior divergence between engines
     * - Bug fixes in HTTP POST encoding only need to be made once
     *
     * @param eventName Event name (becomes _scxmleventname parameter if non-empty)
     * @param params Additional parameters (may contain _scxmleventname)
     * @return Form-encoded POST body (e.g., "_scxmleventname=test&param1=value1")
     */
    static std::string buildHttpPostBody(const std::string &eventName,
                                         const std::map<std::string, std::vector<std::string>> &params) {
        std::string payload;
        bool firstParam = true;

        // W3C SCXML C.2: Add event name as _scxmleventname parameter
        if (!eventName.empty()) {
            payload = "_scxmleventname=" + UrlEncodingHelper::urlEncode(eventName);
            firstParam = false;
        }

        // W3C SCXML: Support duplicate param names (Test 178)
        // Each value in the vector must be added as a separate param
        for (auto it = params.begin(); it != params.end(); ++it) {
            // W3C SCXML C.2: Avoid duplicate _scxmleventname in HTTP POST body
            // Event name already added above via eventName parameter
            // W3C requires "single instance" of _scxmleventname parameter
            // Only skip if eventName is present (Zero Duplication with HttpEventTarget logic)
            if (it->first == "_scxmleventname" && !eventName.empty()) {
                continue;
            }

            for (const auto &value : it->second) {
                if (!firstParam) {
                    payload += "&";
                }
                firstParam = false;
                payload += UrlEncodingHelper::urlEncode(it->first) + "=" + UrlEncodingHelper::urlEncode(value);
            }
        }

        return payload;
    }
};

}  // namespace SCE
