// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/ActionExecutorImpl.h"
#include "actions/AssignAction.h"
#include "actions/CancelAction.h"
#include "actions/ForeachAction.h"
#include "actions/IfAction.h"
#include "actions/LogAction.h"
#include "actions/RaiseAction.h"
#include "actions/ScriptAction.h"
#include "actions/SendAction.h"
#include "common/AssignHelper.h"
#include "common/AssignmentExecutionHelper.h"
#include "common/EventMetadataHelper.h"
#include "common/EventTypeHelper.h"
#include "common/ForeachValidator.h"
#include "common/GuardHelper.h"
#include "common/IOProcessorHelper.h"
#include "common/NamelistHelper.h"
#include "common/SCXMLConstants.h"
#include "common/SendHelper.h"
#include "common/SendSchedulingHelper.h"
#include "common/StringUtils.h"
#include "common/UniqueIdGenerator.h"
#include "core/ForeachHelper.h"
#include "core/LogMacros.h"
#include "events/EventDescriptor.h"
#include "events/EventRaiserService.h"
#include "runtime/TypeRegistry.h"

#include "events/IEventDispatcher.h"
#include "events/InvokeEventTarget.h"
#include "events/ParentEventTarget.h"
#include "runtime/ExecutionContextImpl.h"
#include "scripting/ScriptResultUtils.h"
#include "scripting/SessionRegistry.h"
#include <atomic>
#include <cassert>
#include <chrono>
#include <regex>
#include <sstream>

namespace SCE {

ActionExecutorImpl::ActionExecutorImpl(const std::string &sessionId, IScriptEngine &scriptEngine,
                                       std::shared_ptr<IEventDispatcher> eventDispatcher)
    : scriptEngine_(scriptEngine), sessionId_(sessionId), eventDispatcher_(std::move(eventDispatcher)) {
    // EventRaiser will be injected via setEventRaiser() following dependency injection pattern
    SCE_LOG_DEBUG("ActionExecutorImpl created for session: {} at address: {}", sessionId_, static_cast<void *>(this));
}

ActionExecutorImpl::~ActionExecutorImpl() {
    // Unregister from SessionRegistry EventDispatcher registry for proper cleanup (RAII)
    if (eventDispatcher_) {
        try {
            SessionRegistry::instance().unregisterEventDispatcher(sessionId_);
            SCE_LOG_DEBUG("ActionExecutorImpl: Unregistered EventDispatcher for session: {} during destruction",
                          sessionId_);
        } catch (const std::exception &e) {
            SCE_LOG_WARN("ActionExecutorImpl: Failed to unregister EventDispatcher during destruction: {}", e.what());
        }
    }
    SCE_LOG_DEBUG("ActionExecutorImpl destroyed for session: {}", sessionId_);
}

bool ActionExecutorImpl::executeScript(const std::string &script) {
    if (script.empty()) {
        SCE_LOG_WARN("Attempted to execute empty script");
        return true;  // Empty script is considered successful
    }

    if (!isSessionReady()) {
        SCE_LOG_ERROR("Session {} not ready for script execution", sessionId_);
        return false;
    }

    try {
        // Ensure current event is available in JavaScript context
        ensureCurrentEventSet();

        auto result = scriptEngine_.executeScript(sessionId_, script).get();

        if (!result.isSuccess()) {
            handleJSError("script execution", "Script execution failed");
            return false;
        }

        SCE_LOG_DEBUG("Script executed successfully in session {}", sessionId_);
        return true;

    } catch (const std::exception &e) {
        handleJSError("script execution", e.what());
        return false;
    }
}

bool ActionExecutorImpl::assignVariable(const std::string &location, const std::string &expr) {
    // §scxml-5.4: Empty location check (shared with AOT via AssignHelper)
    // ARCHITECTURE.md: Zero Duplication - Use shared AssignHelper for cross-engine consistency
    if (!AssignHelper::isValidLocation(location)) {
        SCE_LOG_ERROR("W3C SCXML 5.4/B.2: {}", AssignHelper::getInvalidLocationErrorMessage(location));
        // §scxml-5.4: Raise error.execution for invalid location
        if (eventRaiser_) {
            eventRaiser_->raiseEvent("error.execution", AssignHelper::getInvalidLocationErrorMessage(location));
        }
        return false;
    }

    // §scxml-B-2-4: any ECMAScript left-hand-side expression is a legal location, so
    // member and index paths reach the script engine intact.
    // §scxml-B-2-7: <assign> replaces the existing value at 'location' with the result
    // of 'expr', and queues error.execution when it cannot (e.g. a read-only target).
    // Implementation-specific: Variable name format validation (Interpreter engine only)
    // Checks regex pattern for valid variable identifiers (not shared with AOT)
    if (!isValidLocation(location)) {
        SCE_LOG_ERROR("Invalid variable location: {}", location);
        // §scxml-5.4: Raise error.execution for invalid location
        if (eventRaiser_) {
            eventRaiser_->raiseEvent("error.execution", "Invalid assignment location: " + location);
        }
        return false;
    }

    if (!isSessionReady()) {
        SCE_LOG_ERROR("Session {} not ready for variable assignment", sessionId_);
        // §scxml-5.9: Raise error.execution for session not ready
        if (eventRaiser_) {
            eventRaiser_->raiseEvent("error.execution", "Session not ready for assignment");
        }
        return false;
    }

    try {
        // Transform numeric variable names to JavaScript-compatible identifiers
        std::string jsLocation = transformVariableName(location);

        // ARCHITECTURE.md: Zero Duplication - Use shared AssignmentExecutionHelper
        // §scxml-5.4 / §scxml-5.10: Assignment execution with proper system variable handling
        bool success = AssignmentExecutionHelper::executeAssignment(
            scriptEngine_, sessionId_, jsLocation, expr, [this, &location, &expr](const std::string &error) {
                handleJSError("assignment execution", error);
                // §scxml-5.9: Raise error.execution for assignment failure
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution",
                                             "Assignment failed - location: " + location + ", expr: " + expr);
                }
            });

        if (!success) {
            return false;
        }

        SCE_LOG_DEBUG("Variable assigned: {} = {} (JS: {})", location, expr, jsLocation);
        return true;

    } catch (const std::exception &e) {
        handleJSError("variable assignment", e.what());
        // §scxml-5.9: Raise error.execution for assignment exception
        if (eventRaiser_) {
            eventRaiser_->raiseEvent("error.execution", std::string("Assignment exception: ") + e.what());
        }
        return false;
    }
}

std::string ActionExecutorImpl::evaluateExpression(const std::string &expression) {
    if (expression.empty()) {
        SCE_LOG_DEBUG("Empty expression, returning empty string");
        return "";
    }

    SCE_LOG_DEBUG("Evaluating expression: '{}'", expression);

    // CRITICAL: Check session ready state first - return empty string if session not ready
    // This ensures backward compatibility and matches expected behavior in tests
    if (!isSessionReady()) {
        SCE_LOG_DEBUG("Session not ready, returning empty string for expression: '{}'", expression);
        return "";
    }

    // SCXML compliance: Try JavaScript evaluation first (most accurate approach)
    // This follows W3C SCXML specification delegating expression evaluation to native data model
    // §scxml-5.9.4: SCE takes the runtime option — an ill-formed or illegally-valued
    // expression is not rejected at load time, it raises the error at the point where
    // the expression is evaluated.
    std::string jsResult;
    if (tryJavaScriptEvaluation(expression, jsResult)) {
        SCE_LOG_DEBUG("JavaScript evaluation succeeded: '{}' -> '{}'", expression, jsResult);
        return jsResult;
    }

    // §scxml-6.2: If JavaScript evaluation fails (e.g., undefined variable in namelist),
    // throw exception to propagate error up the call stack (test 553)
    // This ensures send actions with invalid namelist are properly aborted
    SCE_LOG_ERROR("JavaScript evaluation failed for expression: '{}'", expression);
    throw std::runtime_error("Failed to evaluate expression: " + expression);
}

void ActionExecutorImpl::log(const std::string &level, const std::string &message) {
    // §scxml-4.7.3: how the message is surfaced is platform-dependent, so it is routed
    // to the host logger and left with no effect on document interpretation.
    // Map SCXML log levels to our logging system
    if (level == "error") {
        SCE_LOG_ERROR("SCXML: {}", message);
    } else if (level == "warn") {
        SCE_LOG_WARN("SCXML: {}", message);
    } else if (level == "debug") {
        SCE_LOG_DEBUG("SCXML: {}", message);
    } else {
        SCE_LOG_INFO("SCXML: {}", message);
    }
}

bool ActionExecutorImpl::tryJavaScriptEvaluation(const std::string &expression, std::string &result) const {
    // Early return if session not ready - avoid unnecessary operations
    if (!isSessionReady()) {
        SCE_LOG_DEBUG("Session not ready for expression: '{}'", expression);
        return false;
    }

    try {
        // SCXML Compliance: Ensure _event variable is available for expressions
        // This is safe to call multiple times due to internal state checking
        const_cast<ActionExecutorImpl *>(this)->ensureCurrentEventSet();

        SCE_LOG_DEBUG("Attempting JavaScript evaluation: '{}'", expression);

        // IMPORTANT: Do NOT transform variable names here
        // TXMLConverter already transforms numeric IDs to varN format:
        //   - conf:location="1" -> location="var1"
        //   - conf:namelist="1" -> namelist="var1"
        //   - conf:expr="1" -> expr="1" (literal number, NOT variable reference)
        // Transforming again would incorrectly convert literal "1" to "var1"
        std::string jsExpression = expression;

        // Perform JavaScript evaluation using the engine
        auto jsResult = scriptEngine_.evaluateExpression(sessionId_, jsExpression).get();

        if (!jsResult.isSuccess()) {
            SCE_LOG_DEBUG("JavaScript evaluation failed for '{}': not a "
                          "valid expression or runtime error",
                          expression);
            return false;
        }

        // Convert JavaScript result to string using the integrated API
        result = ScriptResultUtils::resultToString(jsResult, &scriptEngine_, sessionId_, jsExpression);
        SCE_LOG_DEBUG("JavaScript evaluation successful: '{}' -> '{}' (JS: '{}')", expression, result, jsExpression);
        return true;

    } catch (const std::exception &e) {
        SCE_LOG_DEBUG("Exception during JavaScript evaluation: '{}', error: {}", expression, e.what());
        return false;
    } catch (...) {
        SCE_LOG_ERROR("Unknown exception during JavaScript evaluation: '{}'", expression);
        return false;
    }
}

std::string ActionExecutorImpl::interpretAsLiteral(const std::string &value) const {
    SCE_LOG_DEBUG("Processing literal value: '{}'", value);

    // Handle quoted string literals according to SCXML specification
    if (value.length() >= 2) {
        char first = value.front();
        char last = value.back();

        // Check for matching quotes (double or single)
        if ((first == '"' && last == '"') || (first == '\'' && last == '\'')) {
            std::string unquoted = value.substr(1, value.length() - 2);
            SCE_LOG_DEBUG("Unquoted string literal: '{}' -> '{}'", value, unquoted);
            return unquoted;
        }
    }

    // For all other values, return as-is (numbers, booleans, identifiers, etc.)
    // SCXML Specification: If a value cannot be evaluated as an expression,
    // it should be treated as a literal value
    SCE_LOG_DEBUG("Returning literal as-is: '{}'", value);
    return value;
}

bool ActionExecutorImpl::hasVariable(const std::string &location) {
    if (location.empty() || !isSessionReady()) {
        return false;
    }

    try {
        // Transform numeric variable names to JavaScript-compatible identifiers
        std::string jsLocation = transformVariableName(location);

        // W3C SCXML Compliance: Use engine's native hasVariable for reliable cross-engine support
        return scriptEngine_.hasVariable(sessionId_, jsLocation);

    } catch (const std::exception &e) {
        SCE_LOG_DEBUG("Error checking variable existence: {}", e.what());
        return false;
    }
}

std::string ActionExecutorImpl::getSessionId() const {
    return sessionId_;
}

void ActionExecutorImpl::setEventRaiser(std::shared_ptr<IEventRaiser> eventRaiser) {
    SCE_LOG_DEBUG("ActionExecutorImpl: Setting EventRaiser - eventRaiser is: {}", eventRaiser ? "VALID" : "NULL");
    eventRaiser_ = eventRaiser;

    // Use centralized EventRaiserService to eliminate code duplication
    if (eventRaiser) {
        if (EventRaiserService::getInstance().registerEventRaiser(sessionId_, eventRaiser)) {
            SCE_LOG_DEBUG("ActionExecutorImpl: EventRaiser automatically registered via Service for session: {}",
                          sessionId_);
        } else {
            SCE_LOG_DEBUG("ActionExecutorImpl: EventRaiser already registered for session: {}", sessionId_);
        }
    }
}

void ActionExecutorImpl::setImmediateMode(bool immediate) {
    // §scxml-3.13: Control immediate mode for event raising (test 404)
    // Exit actions should queue events, not process them immediately
    if (eventRaiser_) {
        eventRaiser_->setImmediateMode(immediate);
        SCE_LOG_DEBUG("ActionExecutorImpl: Set immediate mode to {}", immediate);
    }
}

void ActionExecutorImpl::setCurrentEvent(const EventMetadata &metadata) {
    // §scxml-5.10: Set all event metadata fields
    currentEventName_ = metadata.name;
    currentEventData_ = metadata.data;
    currentSendId_ = metadata.sendId;
    currentInvokeId_ = metadata.invokeId;
    currentOriginType_ = metadata.originType;
    currentOriginSessionId_ = metadata.originSessionId;
    currentTypedData_ = metadata.typedData;

    // §scxml-5.10.1: Auto-detect event type if not provided
    // ARCHITECTURE.md: Zero Duplication - Uses EventTypeHelper for Single Source of Truth
    if (metadata.type.empty()) {
        // Default to false for isExternal since explicit type will be set by EventRaiser if needed
        currentEventType_ = EventTypeHelper::classifyEventType(metadata.name, false);
    } else {
        currentEventType_ = metadata.type;
    }

    // Update _event variable in JavaScript context
    ensureCurrentEventSet();
}

EventMetadata ActionExecutorImpl::getCurrentEvent() const {
    return EventMetadata(currentEventName_, currentEventData_, currentEventType_, currentSendId_, currentInvokeId_,
                         currentOriginType_, currentOriginSessionId_);
}

void ActionExecutorImpl::clearCurrentEvent() {
    currentEventName_.clear();
    currentEventData_.clear();
    currentSendId_.clear();
    currentInvokeId_.clear();
    currentOriginType_.clear();
    currentEventType_.clear();
    currentOriginSessionId_.clear();

    // Clear _event variable in JavaScript context by setting null event
    if (isSessionReady()) {
        try {
            std::shared_ptr<Event> nullEvent;
            auto result = scriptEngine_.setCurrentEvent(sessionId_, nullEvent).get();
            if (!result.isSuccess()) {
                SCE_LOG_DEBUG("Failed to clear current event");
            }
        } catch (const std::exception &e) {
            SCE_LOG_DEBUG("Error clearing current event: {}", e.what());
        }
    }
}

bool ActionExecutorImpl::isSessionReady() const {
    // W3C SCXML: Check if script engine session is available without blocking
    try {
        bool hasSessionResult = scriptEngine_.hasSession(sessionId_);
        SCE_LOG_DEBUG("ActionExecutorImpl: hasSession({}) returned: {}", sessionId_, hasSessionResult);
        return hasSessionResult;
    } catch (const std::exception &e) {
        SCE_LOG_WARN("Script engine not available for session check: {}", e.what());
        return false;
    }
}

void ActionExecutorImpl::setEventDispatcher(std::shared_ptr<IEventDispatcher> eventDispatcher) {
    // §scxml-6.2: Unregister old EventDispatcher if one exists
    if (eventDispatcher_) {
        try {
            SessionRegistry::instance().unregisterEventDispatcher(sessionId_);
            SCE_LOG_DEBUG("ActionExecutorImpl: Unregistered previous EventDispatcher for session: {}", sessionId_);
        } catch (const std::exception &e) {
            SCE_LOG_WARN("ActionExecutorImpl: Failed to unregister previous EventDispatcher: {}", e.what());
        }
    }

    // Store new EventDispatcher
    eventDispatcher_ = std::move(eventDispatcher);

    // §scxml-6.2: Register new EventDispatcher with SessionRegistry for automatic delayed event cancellation
    if (eventDispatcher_) {
        try {
            SessionRegistry::instance().registerEventDispatcher(sessionId_, eventDispatcher_);
            SCE_LOG_DEBUG("ActionExecutorImpl: Registered EventDispatcher for session: {}", sessionId_);
        } catch (const std::exception &e) {
            SCE_LOG_ERROR("ActionExecutorImpl: Failed to register EventDispatcher: {}", e.what());
        }
    }

    SCE_LOG_DEBUG("ActionExecutorImpl: Event dispatcher set for session: {}", sessionId_);
}

bool ActionExecutorImpl::isValidLocation(const std::string &location) const {
    if (location.empty()) {
        return false;
    }

    // Allow simple variable names and dot notation paths
    // This is a basic validation - could be enhanced
    // SCXML W3C Compliance: Support numeric data model IDs like "1", "2", "3"
    std::regex locationPattern("^([a-zA-Z_][a-zA-Z0-9_]*|[0-9]+)(\\.[a-zA-Z_][a-zA-Z0-9_]*)*$");
    return std::regex_match(location, locationPattern);
}

std::string ActionExecutorImpl::transformVariableName(const std::string &name) const {
    // Transform numeric variable names to valid JavaScript identifiers
    // "1" -> "var1", "2" -> "var2", etc.
    if (std::regex_match(name, std::regex("^\\d+$"))) {
        return "var" + name;
    }
    return name;
}

void ActionExecutorImpl::handleJSError(const std::string &operation, const std::string &errorMessage) const {
    SCE_LOG_ERROR("JavaScript {} failed in session {}: {}", operation, sessionId_, errorMessage);
}

bool ActionExecutorImpl::ensureCurrentEventSet() {
    if (!isSessionReady()) {
        return false;
    }

    try {
        // _event should only be updated during event processing
        // For assign actions, we should not update _event as it's not related to current event context
        // This prevents violating the read-only _event specification during variable assignments

        // Skip _event update during assign actions - only update when processing actual events
        if (currentEventName_.empty()) {
            SCE_LOG_DEBUG("Skipping _event update - no current event in context");
            return true;
        }

        // Create Event object and use setCurrentEvent API
        // §scxml-5.10: Use the event type set by setCurrentEvent()
        // This is separate from originType - eventType is "internal", "platform", or "external"
        // while originType is the processor URI
        std::string eventType = currentEventType_.empty() ? "internal" : currentEventType_;

        auto event = std::make_shared<Event>(currentEventName_, eventType);

        if (!currentEventData_.empty()) {
            // Set raw JSON data for the new architecture
            event->setRawJsonData(currentEventData_);
        }

        // §scxml-5.10: Set typed event data if available (engine-agnostic, avoids JSON round-trip)
        if (currentTypedData_.has_value()) {
            event->setTypedData(currentTypedData_.value());
        }

        // §scxml-5.10: Set event metadata using EventMetadataHelper (Single Source of Truth)
        // ARCHITECTURE.md: Zero Duplication Principle - shared logic with AOT engine
        //
        // §scxml-C-1: `_event.origin` is the sender's published `_ioprocessors`
        // location, not its bare session id — and this is the one place that
        // publishes it, so this is where the id becomes a location. The
        // conversion itself lives in `IOProcessorHelper::publishedOrigin`,
        // which the AOT engine's own boundary calls too: both engines had to
        // answer this, and a second spelling of the rule is how they would
        // stop agreeing.
        SCE::Common::EventMetadataHelper::setEventMetadata(
            *event,
            IOProcessorHelper::publishedOrigin(currentOriginSessionId_),  // origin (test336)
            currentOriginType_,                                           // originType (test253, 331, 352, 372)
            currentSendId_,                                               // sendId (test332)
            currentInvokeId_                                              // invokeId (test338)
        );

        auto result = scriptEngine_.setCurrentEvent(sessionId_, event).get();
        return result.isSuccess();

    } catch (const std::exception &e) {
        SCE_LOG_DEBUG("Error setting current event: {}", e.what());
        return false;
    }
}

// High-level action execution methods (Command pattern)

bool ActionExecutorImpl::executeScriptAction(const ScriptAction &action) {
    SCE_LOG_DEBUG("Executing script action: {}", action.getId());
    return executeScript(action.getContent());
}

bool ActionExecutorImpl::executeAssignAction(const AssignAction &action) {
    SCE_LOG_DEBUG("Executing assign action: {}", action.getId());
    return assignVariable(action.getLocation(), action.getExpr());
}

bool ActionExecutorImpl::executeLogAction(const LogAction &action) {
    // §scxml-4.7.1: <log> lets an application emit a logging or debug message.
    SCE_LOG_DEBUG("Executing log action: {}", action.getId());

    try {
        // Evaluate the expression to get the log message
        std::string message;
        if (!action.getExpr().empty()) {
            message = evaluateExpression(action.getExpr());
            if (message.empty()) {
                SCE_LOG_WARN("Log expression evaluated to empty string: {}", action.getExpr());
                message = action.getExpr();  // Fallback to raw expression
            }
        }

        // Add label prefix if specified
        if (!action.getLabel().empty()) {
            message = action.getLabel() + ": " + message;
        }

        // Log with specified level
        std::string level = action.getLevel().empty() ? "info" : action.getLevel();
        log(level, message);

        return true;
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Failed to execute log action: {}", e.what());

        // §scxml-5.9: Raise error.execution event for expression evaluation failure
        if (eventRaiser_) {
            eventRaiser_->raiseEvent("error.execution", std::string("Log action failed: ") + e.what());
        }

        return false;
    }
}

bool ActionExecutorImpl::executeRaiseAction(const RaiseAction &action) {
    SCE_LOG_DEBUG("ActionExecutorImpl: Executing raise action: {} with event: '{}'", action.getId(), action.getEvent());

    if (action.getEvent().empty()) {
        SCE_LOG_ERROR("Raise action has empty event name");
        return false;
    }

    try {
        // Evaluate data expression if provided
        std::string eventData;
        if (!action.getData().empty()) {
            eventData = evaluateExpression(action.getData());
            if (eventData.empty()) {
                SCE_LOG_WARN("Raise action data expression evaluated to empty: {}", action.getData());
                eventData = action.getData();  // Fallback to raw data
            }
        }

        SCE_LOG_DEBUG("ActionExecutorImpl: Calling raiseEvent with event: '{}', data: '{}', EventRaiser instance: {}",
                      action.getEvent(), eventData, (void *)eventRaiser_.get());
        if (!eventRaiser_) {
            SCE_LOG_ERROR("ActionExecutorImpl: EventRaiser not available - incomplete setup");
            return false;
        }
        // §scxml-4.2.2: the generated event goes to the rear of this session's
        // internal event queue, never straight to the front.
        bool result = eventRaiser_->raiseEvent(action.getEvent(), eventData);
        SCE_LOG_DEBUG("ActionExecutorImpl: eventRaiser returned: {}", result);
        return result;
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Failed to execute raise action: {}", e.what());
        return false;
    }
}

bool ActionExecutorImpl::executeIfAction(const IfAction &action) {
    SCE_LOG_DEBUG("Executing if action: {}", action.getId());

    try {
        const auto &branches = action.getBranches();
        if (branches.empty()) {
            SCE_LOG_WARN("If action has no branches");
            return true;  // Empty if is valid but does nothing
        }

        // §scxml-4.3.2: execute the first partition in document order whose defining
        // tag has a 'cond' that evaluates to true; <else> defines an unconditional one.
        for (const auto &branch : branches) {
            bool shouldExecute = false;

            if (branch.isElseBranch) {
                // Else branch - always execute
                shouldExecute = true;
                SCE_LOG_DEBUG("Executing else branch");
            } else if (!branch.condition.empty()) {
                // Evaluate condition
                shouldExecute = evaluateCondition(branch.condition);
                SCE_LOG_DEBUG("Condition '{}' evaluated to: {}", branch.condition, shouldExecute);
            } else {
                SCE_LOG_WARN("Branch has empty condition and is not else branch");
                continue;
            }

            if (shouldExecute) {
                // Execute all actions in this branch
                bool allSucceeded = true;

                // Create execution context for nested actions
                auto sharedThis = std::shared_ptr<IActionExecutor>(this, [](IActionExecutor *) {});
                ExecutionContextImpl context(sharedThis, sessionId_);

                for (const auto &branchAction : branch.actions) {
                    if (branchAction && !branchAction->execute(context)) {
                        SCE_LOG_ERROR("Failed to execute action in if branch");
                        allSucceeded = false;
                    }
                }
                return allSucceeded;  // Stop after first matching branch
            }
        }

        // No branch matched
        SCE_LOG_DEBUG("No branch condition matched in if action");
        return true;
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Failed to execute if action: {}", e.what());
        return false;
    }
}

bool ActionExecutorImpl::evaluateCondition(const std::string &condition) {
    // §scxml-5.9: Conditional expressions in <if> elements
    // ARCHITECTURE.md: Zero Duplication - Use shared GuardHelper for conditional evaluation
    if (condition.empty()) {
        return true;  // Empty condition is always true
    }

    auto result = GuardHelper::evaluateGuard(scriptEngine_, sessionId_, condition);

    if (!result.has_value()) {
        // §scxml-5.9: Evaluation failed → raise error.execution AND return false
        SCE_LOG_ERROR("W3C SCXML 5.9: Guard evaluation failed: '{}'", condition);

        if (eventRaiser_) {
            eventRaiser_->raiseEvent("error.execution", "Guard evaluation failed: " + condition);
        }
        return false;
    }

    return *result;
}

bool ActionExecutorImpl::executeSendAction(const SendAction &action) {
    SCE_LOG_DEBUG("Executing send action: {}", action.getId());

    try {
        // CRITICAL: Complete ALL script engine operations first to avoid deadlock
        // Evaluate all expressions before calling EventDispatcher

        // §scxml-5.10 & 6.2.4: Generate and store sendid BEFORE validation
        //
        // IMPORTANT DESIGN DECISION: sendid generation moved before event/type validation
        // Rationale:
        //   1. §scxml-5.10 requirement: error.execution events from failed sends
        //      MUST include the sendid field (test 332)
        //   2. §scxml-6.2.3 requirement: idlocation variable must be set even
        //      when send fails (test 332: compares idlocation sendid == _event.sendid)
        //   3. If we generate sendid AFTER validation, failed sends cannot include
        //      sendid in error events or idlocation variables
        //
        // This ordering ensures proper W3C compliance while maintaining the ability
        // to include sendid in all error scenarios.
        std::string sendId;
        if (!action.getSendId().empty()) {
            sendId = action.getSendId();
        } else {
            // Generate unique sendid as required by SCXML specification
            sendId = generateUniqueSendId();
        }

        // §scxml-6.2.3: Store sendid in idlocation variable if specified
        // This happens BEFORE validation so the variable is set even if send fails
        if (!action.getIdLocation().empty()) {
            try {
                assignVariable(action.getIdLocation(), "'" + sendId + "'");
                SCE_LOG_DEBUG("ActionExecutorImpl: Stored sendid '{}' in variable '{}'", sendId,
                              action.getIdLocation());
            } catch (const std::exception &e) {
                SCE_LOG_ERROR("ActionExecutorImpl: Failed to store sendid in idlocation '{}': {}",
                              action.getIdLocation(), e.what());
            }
        }

        // §scxml-6.2 (test 174): Evaluate type or typeexpr for send action
        std::string sendType = action.getType();
        if (sendType.empty() && !action.getTypeExpr().empty()) {
            // §scxml-6.2: typeexpr uses current datamodel value (not initial value)
            sendType = evaluateExpression(action.getTypeExpr());
            SCE_LOG_DEBUG("ActionExecutorImpl: Evaluated typeexpr '{}' to type: '{}'", action.getTypeExpr(), sendType);
        }

        // §scxml-C-2 (test 577): Check if this is HTTP event processor (needed for validation)
        bool isHttpEventProcessor = (sendType.find("BasicHTTPEventProcessor") != std::string::npos ||
                                     sendType == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor");

        // Determine event name
        std::string eventName;
        if (!action.getEvent().empty()) {
            eventName = action.getEvent();
        } else if (!action.getEventExpr().empty()) {
            eventName = evaluateExpression(action.getEventExpr());
            if (eventName.empty()) {
                SCE_LOG_ERROR("Send action eventexpr evaluated to empty: {}", action.getEventExpr());
                // §scxml-5.10: Generate error.execution event with sendid for failed send
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution",
                                             "Send action eventexpr evaluated to empty: " + action.getEventExpr(),
                                             sendId, false /* overload discriminator for sendId variant */);
                }
                return false;
            }
        } else {
            // §scxml-C-2: For HTTP event processors, event name is optional when content is provided
            // The content will be sent as the HTTP message body

            if (!isHttpEventProcessor) {
                // For non-HTTP processors, event name is required
                SCE_LOG_ERROR("Send action has no event or eventexpr");
                // §scxml-5.10: Generate error.execution event with sendid for failed send
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", "Send action has no event or eventexpr", sendId,
                                             false /* overload discriminator for sendId variant */);
                }
                return false;
            }
            // For HTTP processors, leave eventName empty - content will be sent as HTTP body
            SCE_LOG_DEBUG("ActionExecutorImpl: HTTP send without event name - content will be sent as HTTP body");
        }

        // Determine target with W3C SCXML type processing compliance
        std::string target = action.getTarget();
        if (target.empty() && !action.getTargetExpr().empty()) {
            target = evaluateExpression(action.getTargetExpr());
        }

        // §scxml-6.2 (tests 159, 194): Validate target format using shared helper
        // Invalid target values (e.g., starting with "!") must raise error.execution
        std::string targetErrorMsg;
        if (!SendHelper::validateTarget(target, targetErrorMsg)) {
            SCE_LOG_ERROR("ActionExecutorImpl: {}", targetErrorMsg);
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution", targetErrorMsg, sendId,
                                         false /* overload discriminator for sendId variant */);
            }
            return false;
        }

        // §scxml-C-1 (test 496): Check for unreachable target using SendHelper (ARCHITECTURE.md Zero Duplication)
        // Note: Only applies when targetexpr is explicitly set, not for normal internal sends
        if (!action.getTargetExpr().empty() && SendHelper::isUnreachableTarget(target)) {
            SCE_LOG_ERROR("ActionExecutorImpl: Send target evaluation resulted in invalid target: '{}'", target);
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.communication",
                                         "Target session does not exist or is inaccessible: " + action.getTargetExpr(),
                                         sendId, false /* overload discriminator for sendId variant */);
            }
            return false;
        }

        // §scxml-C-2 (test 577): Validate BasicHTTP send using SendHelper (Zero Duplication)
        std::string errorMsg;
        if (!SendHelper::validateBasicHttpSend(sendType, target, action.getTargetExpr(), errorMsg)) {
            SCE_LOG_ERROR("ActionExecutorImpl: {}", errorMsg);
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.communication", errorMsg, sendId, false);
            }
            return false;
        }

        // §scxml-6.2 (test 199): Validate send type using SendHelper (Zero Duplication)
        // ARCHITECTURE.md: Single Source of Truth - both Interpreter and AOT use SendHelper
        if (!SendHelper::isSupportedSendType(sendType)) {
            SCE_LOG_ERROR("ActionExecutorImpl: Unsupported send type: {}", sendType);
            // §scxml-5.10: Generate error.execution event with sendid for failed send
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution", "Unsupported send type: " + sendType, sendId,
                                         false /* overload discriminator for sendId variant */);
            }
            return false;
        }

        // §scxml-6.2.4: All send actions without explicit target go to external queue
        // The type attribute doesn't affect queue routing - it's for event processor selection
        // Only explicit target="#_internal" goes to internal queue
        if (target.empty()) {
            // W3C SCXML: send with no target → external queue (regardless of type)
            SCE_LOG_DEBUG(
                "ActionExecutorImpl: [W3C193 DEBUG] Send event '{}' with type '{}' → external queue (no target "
                "specified)",
                action.getEvent(), action.getType());
        } else {
            SCE_LOG_DEBUG("ActionExecutorImpl: [W3C193 DEBUG] Send event '{}' with type '{}' → target '{}' specified",
                          action.getEvent(), action.getType(), target);
        }

        // Evaluate data if provided
        std::string eventData;
        if (!action.getData().empty()) {
            eventData = evaluateExpression(action.getData());
        }

        // §scxml-C-1: Build event data from namelist and params (Test 354, 178)
        // §scxml-6.2.6: the message body is authored one of two mutually exclusive
        // ways — 'event' with 'namelist' and <param> children, or a single <content>
        // child — and the interpreter forwards it to the target without altering it.
        // W3C SCXML: Supports duplicate param names - all values must be included (Test 178)
        std::map<std::string, std::vector<std::string>> evaluatedParams;
        std::map<std::string, std::vector<ScriptValue>> typedParams;

        // Step 1: Evaluate namelist variables using NamelistHelper (Zero Duplication Principle)
        const std::string &namelist = action.getNamelist();
        if (!namelist.empty()) {
            SCE_LOG_DEBUG("ActionExecutorImpl: Evaluating namelist: '{}'", namelist);

            bool success = NamelistHelper::evaluateNamelist(
                scriptEngine_, sessionId_, namelist, evaluatedParams,
                [this, &sendId](const std::string &errorMsg) {
                    SCE_LOG_ERROR("ActionExecutorImpl: {}", errorMsg);
                    // §scxml-6.2: If evaluation of send's arguments
                    // produces an error, the Processor MUST discard the
                    // message without attempting to deliver it (test 553)
                    if (eventRaiser_) {
                        eventRaiser_->raiseEvent("error.execution", errorMsg, sendId, false);
                    }
                },
                &typedParams);

            if (!success) {
                return false;
            }

            SCE_LOG_DEBUG("ActionExecutorImpl: Namelist evaluation complete");
        }

        // Step 2: Evaluate param elements (W3C SCXML Test 186, 354)
        // Note: params can override namelist values (evaluated after namelist)
        const auto &params = action.getParamsWithExpr();
        if (!params.empty()) {
            SCE_LOG_DEBUG("ActionExecutorImpl: Evaluating {} param elements", params.size());

            size_t paramCount = 0;
            for (const auto &param : params) {
                paramCount++;
                // §scxml-5.7.1: a <param> carries 'expr' or 'location'; the
                // value of the named location is what the send carries, so
                // SendParam::valueExpr() picks whichever the document wrote.
                const std::string &paramValueExpr = param.valueExpr();
                try {
                    // Evaluate and preserve both string (for JSON serialization) and ScriptValue (for typed pipeline)
                    auto evalResult = scriptEngine_.evaluateExpression(sessionId_, paramValueExpr).get();
                    // §scxml-5.7.1: an expression that will not evaluate costs
                    // the document two things — the pair is left out AND the
                    // failure is reported. Pushing the failed reading under the
                    // param's name was the opposite of "ignore the name and
                    // value": a receiver asking whether the field arrived was
                    // told yes, and nothing on any queue said otherwise. The
                    // message itself still goes: the "discard the message" rule
                    // in the <send> section governs that element's own
                    // arguments, and W3C test343 is the fixture that separates
                    // the two — an illegal <param> raises the error AND the
                    // event still arrives, carrying empty data.
                    if (!evalResult.isSuccess()) {
                        SCE_LOG_ERROR("ActionExecutorImpl: Failed to evaluate param '{}' expr '{}'", param.name,
                                      paramValueExpr);
                        if (eventRaiser_) {
                            eventRaiser_->raiseEvent("error.execution", "<send> <param name='" + param.name +
                                                                            "'> expr failed to evaluate");
                        }
                        continue;
                    }
                    std::string paramValue =
                        ScriptResultUtils::resultToString(evalResult, &scriptEngine_, sessionId_, paramValueExpr);
                    evaluatedParams[param.name].push_back(
                        paramValue);  // W3C SCXML: Support duplicate param names (Test 178)
                    // Preserve ScriptValue for engine-agnostic typed data pipeline
                    typedParams[param.name].push_back(evalResult.getInternalValue());
                    SCE_LOG_DEBUG("ActionExecutorImpl: Param[{}] {}={} (expr: '{}')", paramCount, param.name,
                                  paramValue, paramValueExpr);
                } catch (const std::exception &e) {
                    SCE_LOG_ERROR("ActionExecutorImpl: Failed to evaluate param '{}' expr '{}': {}", param.name,
                                  paramValueExpr, e.what());
                    // §scxml-5.7.1: a throwing engine is the same failure as a
                    // result that reports one — same two halves, same answer.
                    if (eventRaiser_) {
                        eventRaiser_->raiseEvent("error.execution",
                                                 "<send> <param name='" + param.name + "'> expr failed to evaluate");
                    }
                }
            }

            SCE_LOG_DEBUG("ActionExecutorImpl: Param evaluation complete: {} params processed", paramCount);
        }

        // Parse delay (evaluate delay expression if needed)
        std::chrono::milliseconds delay{0};
        if (!action.getDelay().empty()) {
            delay = SendSchedulingHelper::parseDelayString(action.getDelay());
        } else if (!action.getDelayExpr().empty()) {
            std::string delayStr = evaluateExpression(action.getDelayExpr());
            if (!delayStr.empty()) {
                delay = SendSchedulingHelper::parseDelayString(delayStr);
            }
        }

        // ALL script engine operations complete - now safe to call EventDispatcher

        if (eventDispatcher_) {
            SCE_LOG_DEBUG("ActionExecutorImpl: Using event dispatcher for send action");

            // Create event descriptor
            EventDescriptor event;
            event.eventName = eventName;
            event.target = target;
            event.data = eventData;
            event.delay = delay;
            event.sendId = sendId;
            event.sessionId = sessionId_;     // §scxml-6.2: Track session for delayed event cancellation
            event.params = evaluatedParams;   // W3C SCXML compliant: params evaluated at send time
            event.typedParams = typedParams;  // Engine-agnostic typed params (avoids JSON round-trip)
            // §scxml-C-2: Set content for HTTP body
            // §scxml-5.6: <content> is the container whose data is handed to the
            // external service named by the send target.
            event.content = action.getContent();
            // §scxml-5.10: Set event type for origintype field (test 253, 331, 352, 372)
            event.type = sendType.empty() ? Constants::SCXML_EVENT_PROCESSOR_TYPE : sendType;

            // [EVENT ROUTING] Log parent→child and child→parent event sending
            if (target.find("#_invoked") != std::string::npos || target.find("#_parent") != std::string::npos) {
                SCE_LOG_INFO("[EVENT ROUTING] Session '{}' sending event '{}' to target '{}' with data '{}'",
                             sessionId_, eventName, target, eventData);
            }

            // Send via dispatcher (handles both immediate and delayed events)
            auto resultFuture = eventDispatcher_->sendEvent(event);

            // §scxml-6.2: Fire-and-forget send semantics with proper resource cleanup
            // CRITICAL: Must call get() to ensure thread cleanup and prevent WASM memory leak
            // The sendId is already set immediately by EventSchedulerImpl, so this won't block
            try {
                SCE_LOG_INFO("ActionExecutorImpl: BEFORE future.get() - event: '{}'", eventName);

                auto result = resultFuture.get();

                SCE_LOG_INFO("ActionExecutorImpl: AFTER future.get() - event: '{}', success: {}", eventName,
                             result.isSuccess);

                if (result.isSuccess) {
                    SCE_LOG_DEBUG("ActionExecutorImpl: Send action queued successfully for event: {} (sendId: {})",
                                  eventName, result.sendId);
                } else {
                    SCE_LOG_WARN("ActionExecutorImpl: Send action failed: {}", result.errorMessage);
                    // §scxml-C-1: a send naming a session that does not exist
                    // or is inaccessible MUST put error.communication on the
                    // sending session's internal queue. The dispatcher already
                    // knows — it answers TARGET_NOT_FOUND — but until this the
                    // knowledge stopped at the log, so a document could not
                    // transition on a failure it is entitled to observe.
                    //
                    // Only that one error type maps here. The others describe
                    // failures of a target that WAS found (a timeout, a
                    // refused connection), and §scxml-5.10 gives those their
                    // own treatment; widening this would make every transport
                    // hiccup indistinguishable from a dead address.
                    if (result.errorType == SendResult::ErrorType::TARGET_NOT_FOUND && eventRaiser_) {
                        eventRaiser_->raiseEvent("error.communication", result.errorMessage, sendId,
                                                 false /* overload discriminator for sendId variant */);
                    }
                }
            } catch (const std::exception &e) {
                SCE_LOG_ERROR("ActionExecutorImpl: Exception while getting send result: {}", e.what());
            }

            // SCXML 6.2.4: "Fire and forget" semantics - event is queued regardless of delivery status
            return true;
        } else {
            // SCXML 3.12.1: Generate error.execution event instead of throwing
            SCE_LOG_ERROR("ActionExecutorImpl: EventDispatcher not available for send action - generating error event");

            // §scxml-5.10: Generate error.execution event with sendid for failed send
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution", "EventDispatcher not available for send action", sendId,
                                         false /* overload discriminator for sendId variant */);
            }

            // SCXML send actions should follow fire-and-forget - infrastructure failures don't affect action success
            return true;  // Fire and forget semantics
        }

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Failed to execute send action: {}", e.what());
        return false;
    }
}

bool ActionExecutorImpl::executeCancelAction(const CancelAction &action) {
    SCE_LOG_DEBUG("Executing cancel action: {} in session: '{}'", action.getId(), sessionId_);

    try {
        // Determine sendId to cancel
        std::string sendId;
        if (!action.getSendId().empty()) {
            sendId = action.getSendId();
        } else if (!action.getSendIdExpr().empty()) {
            sendId = evaluateExpression(action.getSendIdExpr());
            if (sendId.empty()) {
                SCE_LOG_ERROR("Cancel action sendidexpr evaluated to empty: {}", action.getSendIdExpr());
                return false;
            }
        } else {
            SCE_LOG_ERROR("Cancel action has no sendid or sendidexpr");
            return false;
        }

        // SCXML Event System: Use event dispatcher if available
        if (eventDispatcher_) {
            SCE_LOG_DEBUG("ActionExecutorImpl: Using event dispatcher for cancel action - sendId: '{}', session: '{}'",
                          sendId, sessionId_);

            bool cancelled = eventDispatcher_->cancelEvent(sendId, sessionId_);
            if (cancelled) {
                SCE_LOG_INFO("ActionExecutorImpl: Successfully cancelled event with sendId: {}", sendId);
                return true;
            } else {
                SCE_LOG_INFO("ActionExecutorImpl: Event with sendId '{}' not found or already executed", sendId);
                // W3C SCXML: Cancelling non-existent events is not an error
                return true;
            }
        } else {
            // Fallback to basic event raising behavior
            SCE_LOG_INFO("Cancel action for sendId: {} (no event dispatcher available - no-op)", sendId);
            // Without a dispatcher, we can't cancel anything, but this is not an error
            return true;
        }

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Failed to execute cancel action: {}", e.what());
        return false;
    }
}

bool ActionExecutorImpl::executeForeachAction(const ForeachAction &action) {
    // §scxml-4.6.1: <foreach> walks a collection in the data model and runs the
    // actions it contains once for each item.
    SCE_LOG_DEBUG("Executing foreach action: {}", action.getId());

    if (!isSessionReady()) {
        SCE_LOG_ERROR("Session {} not ready for foreach action execution", sessionId_);
        if (eventRaiser_ && eventRaiser_->isReady()) {
            eventRaiser_->raiseEvent("error.execution", "Session not ready");
        }
        return false;
    }

    // Get array expression and item variable
    std::string arrayExpr = action.getArray();
    std::string itemVar = action.getItem();
    std::string indexVar = action.getIndex();

    // §scxml-4.6: Validate array and item attributes
    std::string validationError;
    if (!SCE::Validation::validateForeachAttributes(arrayExpr, itemVar, validationError)) {
        SCE_LOG_ERROR("Foreach validation failed: {}", validationError);
        if (eventRaiser_ && eventRaiser_->isReady()) {
            eventRaiser_->raiseEvent("error.execution", validationError);
        }
        return false;
    }

    // Transform numeric variable names for array expression
    std::string jsArrayExpr = transformVariableName(arrayExpr);

    // §scxml-4.6: Use ForeachHelper as Single Source of Truth
    // ARCHITECTURE.md: Zero Duplication Principle - shared logic between Interpreter and AOT engines
    bool success = Core::ForeachHelper::executeForeachWithActions(
        scriptEngine_, sessionId_, jsArrayExpr, transformVariableName(itemVar),
        indexVar.empty() ? "" : transformVariableName(indexVar), [&](size_t i) -> bool {
            // Execute nested actions for this iteration
            auto sharedThis = std::shared_ptr<IActionExecutor>(this, [](IActionExecutor *) {});
            ExecutionContextImpl context(sharedThis, sessionId_);

            for (const auto &nestedAction : action.getIterationActions()) {
                if (nestedAction && !nestedAction->execute(context)) {
                    SCE_LOG_ERROR("Failed to execute action in foreach iteration {}", i);
                    if (eventRaiser_ && eventRaiser_->isReady()) {
                        eventRaiser_->raiseEvent("error.execution", "Failed to execute nested action in foreach");
                    }
                    return false;  // §scxml-4.6: Stop foreach execution on error
                }
            }
            return true;  // Continue to next iteration
        });

    // W3C SCXML compliance: Generate error.execution event on failure
    if (!success) {
        SCE_LOG_ERROR("Foreach action execution failed for array expression: {}", arrayExpr);
        if (eventRaiser_ && eventRaiser_->isReady()) {
            eventRaiser_->raiseEvent("error.execution", "Foreach execution failed");
        }
    }

    return success;
}

bool ActionExecutorImpl::setLoopVariable(const std::string &varName, const std::string &value, size_t iteration) {
    // ARCHITECTURE.md: Logic Commonization - Use shared ForeachHelper
    // Single Source of Truth for foreach variable setting logic
    try {
        // Transform numeric variable names to JavaScript-compatible identifiers
        std::string jsVarName = transformVariableName(varName);

        // Use shared ForeachHelper logic (eliminates code duplication with AOT engine)
        bool success = SCE::Core::ForeachHelper::setLoopVariable(scriptEngine_, sessionId_, jsVarName, value);

        if (success) {
            SCE_LOG_DEBUG("Set foreach variable: {} = {} (JS: {}, iteration {})", varName, value, jsVarName, iteration);
        } else {
            SCE_LOG_ERROR("Failed to set foreach variable {} = {} at iteration {}", varName, value, iteration);
        }

        return success;

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Exception setting foreach variable {} at iteration {}: {}", varName, iteration, e.what());
        return false;
    }
}

std::string ActionExecutorImpl::generateUniqueSendId() const {
    // REFACTOR: Use centralized UniqueIdGenerator instead of duplicate logic
    return UniqueIdGenerator::generateSendId();
}

}  // namespace SCE