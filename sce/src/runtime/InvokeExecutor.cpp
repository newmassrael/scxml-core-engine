// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/InvokeExecutor.h"
#include "SCXMLTypes.h"
#include "common/SCXMLConstants.h"
#include "common/DatamodelValidationHelper.h"
#include "core/InvokeHelper.h"
#include "core/LogMacros.h"
#include "common/UniqueIdGenerator.h"
#include "events/EventDescriptor.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "model/InvokeNode.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/InvokeExecutor.h"
#include "runtime/StateMachine.h"
#include "runtime/StateMachineBuilder.h"
#include "runtime/StateMachineContext.h"
#include "scripting/IScriptEngine.h"
#include "scripting/ScriptResultUtils.h"
#include "scripting/SessionRegistry.h"
#include <cerrno>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <random>
#include <sstream>

namespace SCE {

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * @brief Extract parent state ID from invoke ID
 *
 * §scxml-6.4: Invoke ID format is "stateId.platformid" (e.g., "s0.invoke_2")
 * Zero Duplication: Single source of truth for invoke ID parsing logic
 *
 * @param invokeId Full invoke ID
 * @return Parent state ID (portion before first dot)
 */
static std::string extractParentStateIdFromInvokeId(const std::string &invokeId) {
    auto dotPos = invokeId.find('.');
    if (dotPos == std::string::npos) {
        // §scxml-6.4: If no dot found, entire string is the state ID
        // This handles custom invoke IDs without platform suffix
        SCE_LOG_DEBUG("InvokeExecutor: Invoke ID has no dot separator, using entire ID as parent state: {}", invokeId);
        return invokeId;
    }
    return invokeId.substr(0, dotPos);
}

// ============================================================================
// SCXMLInvokeHandler Implementation
// ============================================================================

SCXMLInvokeHandler::SCXMLInvokeHandler(IScriptEngine &scriptEngine) : scriptEngine_(scriptEngine) {}

SCXMLInvokeHandler::~SCXMLInvokeHandler() {
    // Cancel all active sessions on destruction
    for (const auto &[invokeid, session] : activeSessions_) {
        if (session.isActive) {
            scriptEngine_.destroySession(session.sessionId);
        }
    }
}

std::string SCXMLInvokeHandler::startInvoke(const std::shared_ptr<IInvokeNode> &invoke,
                                            const std::string &parentSessionId,
                                            std::shared_ptr<IEventDispatcher> eventDispatcher) {
    if (!invoke) {
        SCE_LOG_ERROR("SCXMLInvokeHandler: Cannot start invoke - invoke node is null");
        return "";
    }

    // Generate unique child session ID
    std::string childSessionId = SessionRegistry::instance().generateSessionIdString("session_");

    SCE_LOG_DEBUG("SCXMLInvokeHandler: startInvoke called with parent session: {}, generated child session: {}",
              parentSessionId, childSessionId);

    // Delegate to internal method with session creation required
    return startInvokeInternal(invoke, parentSessionId, eventDispatcher, childSessionId, false, false);
}

std::string SCXMLInvokeHandler::startInvokeWithSessionId(const std::shared_ptr<IInvokeNode> &invoke,
                                                         const std::string &parentSessionId,
                                                         std::shared_ptr<IEventDispatcher> eventDispatcher,
                                                         const std::string &childSessionId, bool isRestoration) {
    if (!invoke) {
        SCE_LOG_ERROR("SCXMLInvokeHandler: Cannot start invoke - invoke node is null");
        return "";
    }

    SCE_LOG_DEBUG(
        "SCXMLInvokeHandler: startInvokeWithSessionId called with parent session: {}, pre-allocated child session: {}",
        parentSessionId, childSessionId);

    // Check if session exists (architectural fix for timing)
    bool sessionExists = scriptEngine_.hasSession(childSessionId);

    // Delegate to internal method with session existence information
    return startInvokeInternal(invoke, parentSessionId, eventDispatcher, childSessionId, sessionExists, isRestoration);
}

std::string SCXMLInvokeHandler::startInvokeInternal(const std::shared_ptr<IInvokeNode> &invoke,
                                                    const std::string &parentSessionId,
                                                    std::shared_ptr<IEventDispatcher> eventDispatcher,
                                                    const std::string &childSessionId, bool sessionAlreadyExists,
                                                    bool isRestoration) {
    // §scxml-6.4: Generate invoke ID with state ID for "stateid.platformid" format (test 224)
    std::string invokeid = invoke->getId().empty() ? generateInvokeId(invoke->getStateId()) : invoke->getId();

    // Store generated ID back to InvokeNode for later cleanup (fixes test 207, 233, 234, 237, 252, 338, 422)
    if (invoke->getId().empty()) {
        invoke->setId(invokeid);
    }

    SCE_LOG_INFO(
        "SCXMLInvokeHandler: Starting invoke - invokeid: {}, childSession: {}, parentSession: {}, sessionExists: {}",
        invokeid, childSessionId, parentSessionId, sessionAlreadyExists);

    // Get invoke content (SCXML document)
    std::string scxmlContent = invoke->getContent();
    SCE_LOG_DEBUG("SCXMLInvokeHandler: Invoke content length: {}, has src: {}, has srcexpr: {}, has contentexpr: {}",
              scxmlContent.length(), !invoke->getSrc().empty(), !invoke->getSrcExpr().empty(),
              !invoke->getContentExpr().empty());

    // W3C SCXML test 530: Handle contentexpr evaluation
    if (scxmlContent.empty() && !invoke->getContentExpr().empty()) {
        // Evaluate contentexpr in parent session context
        auto future = scriptEngine_.evaluateExpression(parentSessionId, invoke->getContentExpr());
        auto result = future.get();

        if (result.isSuccess()) {
            scxmlContent = result.getValue<std::string>();
            SCE_LOG_DEBUG("SCXMLInvokeHandler: contentexpr '{}' evaluated to content of length {}",
                      invoke->getContentExpr(), scxmlContent.length());
        } else {
            SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to evaluate contentexpr '{}': {}", invoke->getContentExpr(),
                      result.getErrorMessage());
            return "";
        }
    }

    // Handle srcexpr evaluation first
    if (scxmlContent.empty() && !invoke->getSrcExpr().empty()) {
        // Evaluate srcexpr in parent session context
        auto future = scriptEngine_.evaluateExpression(parentSessionId, invoke->getSrcExpr());
        auto result = future.get();

        if (result.isSuccess()) {
            std::string evaluatedSrc = result.getValue<std::string>();
            SCE_LOG_DEBUG("SCXMLInvokeHandler: srcexpr '{}' evaluated to '{}'", invoke->getSrcExpr(), evaluatedSrc);

            // Remove surrounding quotes if present
            if (evaluatedSrc.length() >= 2 && evaluatedSrc.front() == '\'' && evaluatedSrc.back() == '\'') {
                evaluatedSrc = evaluatedSrc.substr(1, evaluatedSrc.length() - 2);
            }

            // Load SCXML content from file
            scxmlContent = loadSCXMLFromFile(evaluatedSrc, parentSessionId);
            if (scxmlContent.empty()) {
                SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to load SCXML from srcexpr file: {}", evaluatedSrc);
                return "";
            }
        } else {
            SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to evaluate srcexpr '{}': {}", invoke->getSrcExpr(),
                      result.getErrorMessage());
            return "";
        }
    }
    // Handle static src attribute
    else if (scxmlContent.empty() && !invoke->getSrc().empty()) {
        SCE_LOG_INFO("SCXMLInvokeHandler: Loading SCXML from src file: {}", invoke->getSrc());
        scxmlContent = loadSCXMLFromFile(invoke->getSrc(), parentSessionId);
        if (scxmlContent.empty()) {
            SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to load SCXML from src file: {}", invoke->getSrc());
            return "";
        }
    }

    if (scxmlContent.empty()) {
        SCE_LOG_ERROR("SCXMLInvokeHandler: No content, src, or srcexpr specified for invoke: {}", invokeid);
        return "";
    }

    // Create session if it doesn't exist
    if (!sessionAlreadyExists) {
        bool sessionCreated = scriptEngine_.createSession(childSessionId, parentSessionId);
        if (!sessionCreated) {
            SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to create child session: {}", childSessionId);
            return "";
        }
    }

    // §scxml-6.4: Handle idlocation attribute - store invoke ID in parent session
    if (!invoke->getIdLocation().empty()) {
        scriptEngine_.setVariable(parentSessionId, invoke->getIdLocation(), ScriptValue{invokeid});
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Set idlocation '{}' = '{}' in parent session '{}'", invoke->getIdLocation(),
                  invokeid, parentSessionId);
    }

    // Set special variables in child session
    scriptEngine_.setVariable(childSessionId, "_invokeid", ScriptValue{invokeid});
    scriptEngine_.setVariable(childSessionId, "_parent", ScriptValue{parentSessionId});

    // §scxml-6.2 compliance: Register EventDispatcher for delayed event cancellation
    if (eventDispatcher) {
        SessionRegistry::instance().registerEventDispatcher(childSessionId, eventDispatcher);
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Registered EventDispatcher for child session: {}", childSessionId);
    } else {
        SCE_LOG_WARN("SCXMLInvokeHandler: No EventDispatcher provided for session: {}", childSessionId);
    }

    // W3C SCXML: Create EventRaiser for #_parent target support
    auto childEventRaiser = std::make_shared<EventRaiserImpl>();

    // Share parent's EventScheduler with child for consistent scheduler mode
    // Parent and child must use the same scheduler so child inherits MANUAL mode for interactive debugging
    if (auto parentSM = parentStateMachine_.lock()) {
        if (auto parentEventRaiser = parentSM->getEventRaiser()) {
            if (auto parentScheduler = parentEventRaiser->getScheduler()) {
                childEventRaiser->setScheduler(parentScheduler);
                SCE_LOG_DEBUG("InvokeExecutor: Set child EventRaiser scheduler from parent for session: {}",
                          childSessionId);
            } else {
                SCE_LOG_WARN("InvokeExecutor: Parent EventRaiser has no scheduler for session: {}", childSessionId);
            }
        }
    }

    // Store session information for tracking
    InvokeSession session;
    session.invokeid = invokeid;
    session.sessionId = childSessionId;
    session.parentSessionId = parentSessionId;
    session.eventDispatcher = eventDispatcher;
    session.smContext = nullptr;  // Will be created later
    session.isActive = true;
    session.autoForward = invoke->isAutoForward();
    session.finalizeScript =
        invoke->getFinalize();  // §scxml-6.5.2: Store finalize handler for execution before processing child events
    session.scxmlContent = scxmlContent;  // §scxml-3.11: Store SCXML content for snapshot restoration

    // Build StateMachine with dependency injection, then wrap in RAII context
    // Inherit the parent's script engine — child invokes share the engine
    // instance so cross-session evaluation honors a single registry.
    StateMachineBuilder builder;
    auto stateMachine = builder.withScriptEngine(scriptEngine_)
                            .withSessionId(childSessionId)
                            .withEventDispatcher(eventDispatcher)
                            .withEventRaiser(childEventRaiser)
                            .build();

    // Wrap in StateMachineContext for RAII cleanup (shared ownership)
    auto smContext = std::make_unique<StateMachineContext>(stateMachine);

    // Get weak_ptr for thread-safe callbacks (prevents use-after-free)
    std::weak_ptr<StateMachine> weakChildSM = smContext->getShared();

    SCE_LOG_DEBUG("SCXMLInvokeHandler: Created child StateMachine with StateMachineBuilder for session: {}",
              childSessionId);

    // §scxml-6.4.3: Register completion callback for done.invoke generation
    // This callback is invoked AFTER the child's final state onexit handlers complete
    // IMPORTANT: Use weak_ptr to prevent accessing destroyed parent StateMachine (thread-safe)
    // §scxml-6.4.3: Completion callback registered for both normal execution and snapshot restoration
    // Child state machines must send done.invoke when reaching final state after being restored
    std::weak_ptr<StateMachine> weakParentSM = parentStateMachine_;
    weakChildSM.lock()->setCompletionCallback([weakParentSM, weakChildSM, invokeid, childSessionId, parentSessionId,
                                               eventDispatcher]() {
        SCE_LOG_INFO("SCXMLInvokeHandler: Child completion callback invoked - invokeid: {}, session: {}", invokeid,
                 childSessionId);
        SCE_LOG_INFO("SCXMLInvokeHandler: Parent check - weakPtr valid: {}, parentSessionId: {}", !weakParentSM.expired(),
                 parentSessionId);

        // W3C SCXML Test 192: Check if parent StateMachine is in final state (thread-safe with weak_ptr)
        // If parent already completed, don't send done.invoke (it would be ignored anyway)
        auto parentSM = weakParentSM.lock();
        if (!parentSM) {
            SCE_LOG_DEBUG("SCXMLInvokeHandler: Parent StateMachine destroyed, skipping done.invoke.{}", invokeid);
            return;
        }

        if (parentSM->isInFinalState()) {
            SCE_LOG_DEBUG("SCXMLInvokeHandler: Parent already in final state, skipping done.invoke.{}", invokeid);
            return;
        }

        // §scxml-6.4.3: Generate done.invoke.id event
        // ARCHITECTURE.md: Use InvokeHelper for Single Source of Truth (Zero Duplication with AOT)
        std::string doneEvent = SCE::Core::InvokeHelper::createDoneInvokeEventName(invokeid);

        // §scxml-3.7 / §scxml-6.4: done.invoke is NOT a <send> element - automatic platform event
        // ARCHITECTURE.md Zero Duplication: Match AOT raiseExternal() (bypasses EventScheduler)
        // Prevents §scxml-6.2 cancellation (only <send> delayed messages are cancelled)
        // Note: EXTERNAL priority auto-detected for done.* events (EventRaiserImpl.cpp:148)
        auto parentEventRaiser = parentSM->getEventRaiser();
        if (parentEventRaiser) {
            // §scxml-5.5 + 6.4.3: carry the child's donedata payload on
            // `done.invoke.<id>._event.data`. The child stashed it at top-level
            // `<final>` entry via `StateMachine::enterState` into
            // `pendingDonedataAtFinal_`; we read it back here via
            // `donedataAtFinal()`. Mirror of the AOT `invoke_methods.jinja2`
            // completion callback which reads `child_..._->donedataAtFinal()`
            // from `StaticExecutionEngine`. typedData is re-hydrated on the
            // parent side inside `EventRaiserImpl::raiseEventWithPriority`
            // (§scxml-B-2 JSON auto-parse), so we only thread the JSON
            // string through the public `IEventRaiser` interface.
            std::string eventData;
            if (auto childSM = weakChildSM.lock()) {
                eventData = childSM->donedataAtFinal();
            }

            bool success =
                parentEventRaiser->raiseEvent(doneEvent,       // event name (done.invoke.*)
                                              eventData,       // event data
                                              childSessionId,  // origin session
                                              invokeid,        // invoke ID
                                              "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"  // origin type
                );

            if (success) {
                SCE_LOG_INFO("SCXMLInvokeHandler: {} sent directly to parent's external queue (session: {}, matches "
                         "AOT raiseExternal)",
                         doneEvent, parentSessionId);
            } else {
                SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to send {} to parent session: {}", doneEvent, parentSessionId);
            }
        } else {
            SCE_LOG_WARN("SCXMLInvokeHandler: Child reached final state but no parent EventRaiser available for: {}",
                     doneEvent);
        }

        // §scxml-6.2: Cancel pending delayed sends when child session terminates
        // "If the SCXML session terminates before the delay interval has elapsed,
        // the SCXML Processor MUST discard the message without attempting to deliver it."
        // This applies only to <send> element messages, not done.invoke (W3C SCXML Test 187)
        if (eventDispatcher) {
            size_t cancelledCount = eventDispatcher->cancelEventsForSession(childSessionId);
            SCE_LOG_INFO("SCXMLInvokeHandler: Cancelled {} pending delayed send(s) for terminated child session: {}",
                     cancelledCount, childSessionId);
        }
    });
    SCE_LOG_DEBUG("SCXMLInvokeHandler: Registered completion callback for invoke: {}", invokeid);

    // Set up EventRaiser callback to child StateMachine's processEvent
    SCE_LOG_DEBUG("SCXMLInvokeHandler: Setting EventRaiser callback for session: {}, EventRaiser: {}", childSessionId,
              (void *)childEventRaiser.get());

    // Reuse weakParentSM from completion callback above
    childEventRaiser->setEventCallback([weakChildSM, weakParentSM, childSessionId](
                                           const std::string &eventName, const std::string &eventData) -> bool {
        // Thread-safe callback: use weak_ptr to prevent use-after-free
        auto childSM = weakChildSM.lock();
        if (!childSM) {
            SCE_LOG_DEBUG("EventRaiser callback skipped - child StateMachine destroyed, session: {}, event: '{}'",
                      childSessionId, eventName);
            return false;
        }

        SCE_LOG_DEBUG("EventRaiser callback executing - session: {}, event: '{}', isRunning: {}", childSessionId, eventName,
                  childSM->isRunning() ? "true" : "false");

        // W3C SCXML: Don't process events if parent is in final state (thread-safe with weak_ptr)
        auto parentSM = weakParentSM.lock();
        if (parentSM && parentSM->isInFinalState()) {
            SCE_LOG_DEBUG("EventRaiser callback skipped - parent in final state, session: {}, event: '{}'", childSessionId,
                      eventName);
            return false;
        }

        // W3C SCXML: Don't process events if child is in final state
        if (childSM->isRunning() && !childSM->isInFinalState()) {
            auto result = childSM->processEvent(eventName, eventData);
            SCE_LOG_DEBUG("EventRaiser callback result - session: {}, event: '{}', success: {}", childSessionId, eventName,
                      result.success);
            return result.success;
        }
        SCE_LOG_DEBUG("EventRaiser callback skipped - session: {}, event: '{}', StateMachine not running or in final state",
                  childSessionId, eventName);
        return false;
    });

    SCE_LOG_DEBUG("SCXMLInvokeHandler: EventRaiser callback setup complete for session: {}", childSessionId);

    // Assign StateMachineContext to session before loading (keeps StateMachine alive)
    session.smContext = std::move(smContext);

    // Load SCXML content into child StateMachine
    SCE_LOG_DEBUG(
        "SCXMLInvokeHandler: Loading SCXML content into child StateMachine - invokeid: {}, content size: {} bytes",
        invokeid, scxmlContent.length());
    if (!session.smContext->get()->loadSCXMLFromString(scxmlContent)) {
        SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to load SCXML content for invoke: {}", invokeid);
        scriptEngine_.destroySession(childSessionId);
        return "";
    }
    SCE_LOG_DEBUG("SCXMLInvokeHandler: Successfully loaded SCXML content for invoke: {}", invokeid);

    // §scxml-6.4: Set invoke data AFTER loading but BEFORE starting
    // This ensures namelist/param values override child's datamodel initial values

    // §scxml-6.4: Get child's datamodel variable names for validation
    // "If the name of a param element or the key of a namelist item do not match the name of a data
    // element in the invoked process, the Processor MUST NOT add the value to the invoked session's data model"
    std::set<std::string> childDatamodelVars;
    auto childModel = session.smContext->get()->getModel();
    if (childModel) {
        childDatamodelVars = childModel->getDataModelVariableNames();
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Child session {} has {} datamodel variables", childSessionId,
                  childDatamodelVars.size());
    }

    // §scxml-6.4: Handle namelist attribute - pass datamodel variables by name
    const std::string &namelist = invoke->getNamelist();
    if (!namelist.empty()) {
        std::istringstream iss(namelist);
        std::string varName;
        while (iss >> varName) {
            // §scxml-6.4: Namelist variable must exist in parent datamodel.
            // Bridges Lua's nil-for-undeclared gap (JS throws ReferenceError, Lua returns nil).
            if (!scriptEngine_.hasVariable(parentSessionId, varName)) {
                SCE_LOG_ERROR(
                    "SCXMLInvokeHandler: Namelist variable '{}' not defined in parent session: invoke cancelled",
                    varName);
                scriptEngine_.destroySession(childSessionId);
                return "";
            }

            // §scxml-6.4: Evaluate variable in parent session
            auto future = scriptEngine_.getVariable(parentSessionId, varName);
            auto result = future.get();

            if (!ScriptResultUtils::isSuccess(result)) {
                SCE_LOG_ERROR(
                    "SCXMLInvokeHandler: Failed to evaluate namelist variable '{}' in parent session: invoke cancelled",
                    varName);
                // §scxml-6.4: If evaluation of invoke's arguments produces an error,
                // the Processor MUST terminate processing of the element (test 554)
                scriptEngine_.destroySession(childSessionId);
                return "";
            }

            // §scxml-6.4: Only set variable if it exists in child's datamodel
            // ARCHITECTURE.md Zero Duplication: Use DatamodelValidationHelper
            if (!DatamodelValidationHelper::isVariableDeclaredInChild(varName, childDatamodelVars)) {
                SCE_LOG_DEBUG("SCXMLInvokeHandler: Skipping namelist variable '{}' - not defined in child's datamodel",
                          varName);
                continue;
            }

            setInvokeDataVariable(childSessionId, varName, result.getInternalValue(), "namelist");
        }
    }

    // §scxml-6.4: Set up invoke parameters in child session data model
    const auto &params = invoke->getParams();
    for (const auto &[name, expr, location] : params) {
        if (!name.empty()) {
            // §scxml-6.4: Only set variable if it exists in child's datamodel
            // ARCHITECTURE.md Zero Duplication: Use DatamodelValidationHelper
            if (!DatamodelValidationHelper::isVariableDeclaredInChild(name, childDatamodelVars)) {
                SCE_LOG_DEBUG("SCXMLInvokeHandler: Skipping param '{}' - not defined in child's datamodel", name);
                continue;
            }

            auto future = scriptEngine_.evaluateExpression(parentSessionId, expr);
            auto result = future.get();

            if (!ScriptResultUtils::isSuccess(result)) {
                SCE_LOG_ERROR(
                    "SCXMLInvokeHandler: Failed to evaluate param expression '{}' in parent session: invoke cancelled",
                    expr);
                // §scxml-6.4: If evaluation of invoke's arguments produces an error,
                // the Processor MUST terminate processing of the element
                scriptEngine_.destroySession(childSessionId);
                return "";
            }

            setInvokeDataVariable(childSessionId, name, result.getInternalValue(), "param");
        }
    }

    // Add session to activeSessions_
    activeSessions_.emplace(invokeid, std::move(session));

    // Get reference to the session we just added (session was moved, can't use it anymore)
    auto &activeSession = activeSessions_[invokeid];

    // §scxml-5.10 test 338: Register invoke mapping BEFORE starting child
    // This ensures mapping is available when child's final state onentry sends events
    // For pre-allocated sessions (sessionAlreadyExists=true), mapping may already be registered by InvokeExecutor
    SCE_LOG_INFO("[INVOKE MAPPING] sessionAlreadyExists={}, isRestoration={}, parent={}, invoke={}, child={}",
             sessionAlreadyExists, isRestoration, parentSessionId, invokeid, childSessionId);

    if (!sessionAlreadyExists) {
        SessionRegistry::instance().registerInvokeMapping(parentSessionId, invokeid, childSessionId);
        SCE_LOG_INFO("[INVOKE MAPPING REGISTERED] parent={}, invoke={} -> child={}", parentSessionId, invokeid,
                 childSessionId);
    } else {
        SCE_LOG_WARN("[INVOKE MAPPING SKIPPED] sessionAlreadyExists=true - mapping should exist already");
    }

    // W3C SCXML Test 233, 234: Register finalize script BEFORE starting child
    // Separate storage ensures script remains available even after invoke cancellation
    if (!activeSession.finalizeScript.empty()) {
        std::lock_guard<std::mutex> lock(finalizeScriptsMutex_);
        finalizeScripts_[childSessionId] = activeSession.finalizeScript;
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Registered finalize script for child session: {}", childSessionId);
    }

    // §scxml-3.11: Start child StateMachine or restore state based on mode
    if (!isRestoration) {
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Starting child StateMachine for invoke: {}", invokeid);
        if (!activeSession.smContext->get()->start()) {
            SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to start child StateMachine for invoke: {}", invokeid);
            activeSessions_.erase(invokeid);
            // Unregister mapping if we registered it above
            if (!sessionAlreadyExists) {
                SessionRegistry::instance().unregisterInvokeMapping(parentSessionId, invokeid);
            }
            // Remove finalize script if registered
            {
                std::lock_guard<std::mutex> lock(finalizeScriptsMutex_);
                finalizeScripts_.erase(childSessionId);
            }
            scriptEngine_.destroySession(childSessionId);
            return "";
        }
        SCE_LOG_INFO("SCXMLInvokeHandler: Child StateMachine started successfully for invoke: {} (session: {})", invokeid,
                 childSessionId);
    } else {
        // Restoration mode: Skip start() - state will be restored by restoreChildState()
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Skipping start() for invoke restoration: {} (state will be restored separately)",
                  invokeid);
    }

    // §scxml-6.4.3: done.invoke generation is now handled by completion callback
    // The callback ensures proper event ordering: child onexit → done.invoke
    // No need for synchronous done.invoke generation here

    SCE_LOG_INFO("SCXMLInvokeHandler: Successfully started SCXML invoke: {} with session: {} and running StateMachine",
             invokeid, childSessionId);

    return invokeid;
}

bool SCXMLInvokeHandler::cancelInvoke(const std::string &invokeid) {
    auto it = activeSessions_.find(invokeid);
    if (it == activeSessions_.end()) {
        SCE_LOG_WARN("SCXMLInvokeHandler: Cannot cancel invoke - not found: {}", invokeid);
        return false;
    }

    InvokeSession &session = it->second;
    if (!session.isActive) {
        SCE_LOG_WARN("SCXMLInvokeHandler: Invoke already inactive: {}", invokeid);
        return false;
    }

    SCE_LOG_DEBUG("SCXMLInvokeHandler: Cancelling invoke: {} with session: {}", invokeid, session.sessionId);

    // W3C SCXML Test 252: Track cancelled child session FIRST to enable filtering
    // This must happen BEFORE cancelEventsForSession() to prevent race condition:
    // 1. Add to cancelledChildSessions_ first
    // 2. Any processEvent() calls will be filtered by shouldFilterCancelledInvokeEvent()
    // 3. Then cancelEventsForSession() removes queued events
    // Use bounded FIFO cache to prevent memory leak
    size_t cacheSize;
    {
        std::lock_guard<std::mutex> lock(cancelledSessionsMutex_);

        // Only add if not already present (prevents duplicate entries in deque)
        if (cancelledChildSessions_.insert(session.sessionId).second) {
            // Successfully inserted new entry - add to FIFO order
            cancelledSessionsOrder_.push_back(session.sessionId);

            // Enforce bounded cache by removing oldest entries
            if (cancelledSessionsOrder_.size() > MAX_CANCELLED_SESSIONS) {
                std::string oldest = cancelledSessionsOrder_.front();
                cancelledSessionsOrder_.pop_front();
                cancelledChildSessions_.erase(oldest);
                SCE_LOG_DEBUG("SCXMLInvokeHandler: Evicted oldest cancelled session from cache: {}", oldest);
            }
        }
        cacheSize = cancelledSessionsOrder_.size();
    }
    SCE_LOG_DEBUG("SCXMLInvokeHandler: Added cancelled child session to filter list: {} (cache size: {})",
              session.sessionId, cacheSize);

    // Cancel pending events in child's EventDispatcher
    if (session.eventDispatcher) {
        size_t cancelledEvents = session.eventDispatcher->cancelEventsForSession(session.sessionId);
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Cancelled {} pending events for session: {}", cancelledEvents,
                  session.sessionId);
    }

    // W3C SCXML Test 252: Cancel queued events in parent's EventRaiser
    // This prevents processing events that child sent before invoke was cancelled
    // Must happen AFTER adding to cancelledChildSessions_ to enable filtering
    if (auto parentSM = parentStateMachine_.lock()) {
        if (auto parentEventRaiser = parentSM->getEventRaiser()) {
            size_t cancelledQueued = parentEventRaiser->cancelEventsForSession(session.sessionId);
            if (cancelledQueued > 0) {
                SCE_LOG_INFO("SCXMLInvokeHandler: Cancelled {} queued event(s) in parent EventRaiser for session: {}",
                         cancelledQueued, session.sessionId);
            }
        }
    }

    // With weak_ptr callbacks, no synchronization needed - callbacks safely check weak_ptr::lock()
    // Unregister invoke mapping from JSEngine
    SessionRegistry::instance().unregisterInvokeMapping(session.parentSessionId, invokeid);

    // Destroy the child session (RAII cleanup with thread-safe weak_ptr callbacks)
    // Callbacks use weak_ptr::lock() which returns nullptr if StateMachine is destroyed
    // Child's onexit handlers may send events to parent during destruction - these will be filtered
    scriptEngine_.destroySession(session.sessionId);

    // Mark as inactive
    session.isActive = false;

    // W3C SCXML Test 233, 234: Remove finalize script after cancellation
    // Script is no longer needed once invoke is cancelled
    {
        std::lock_guard<std::mutex> lock(finalizeScriptsMutex_);
        finalizeScripts_.erase(session.sessionId);
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Removed finalize script for cancelled child session: {}", session.sessionId);
    }

    SCE_LOG_INFO("SCXMLInvokeHandler: Successfully cancelled invoke: {}", invokeid);
    return true;
}

bool SCXMLInvokeHandler::isInvokeActive(const std::string &invokeid) const {
    auto it = activeSessions_.find(invokeid);
    return it != activeSessions_.end() && it->second.isActive;
}

std::string SCXMLInvokeHandler::getType() const {
    return "scxml";
}

void SCXMLInvokeHandler::setInvokeDataVariable(const std::string &childSessionId, const std::string &varName,
                                               const ScriptValue &value, const std::string &source) {
    scriptEngine_.setVariable(childSessionId, varName, value);
    SCE_LOG_INFO("SCXMLInvokeHandler: Set {} variable '{}' in child session", source, varName);
}

bool SCXMLInvokeHandler::shouldFilterCancelledInvokeEvent(const std::string &childSessionId) const {
    // W3C SCXML Test 252: Filter events from cancelled invoke child sessions
    bool shouldFilter;
    {
        std::lock_guard<std::mutex> lock(cancelledSessionsMutex_);
        shouldFilter = cancelledChildSessions_.find(childSessionId) != cancelledChildSessions_.end();
    }
    if (shouldFilter) {
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Filtering event from cancelled child session: {}", childSessionId);
    }
    return shouldFilter;
}

std::string SCXMLInvokeHandler::generateInvokeId(const std::string &stateId) const {
    // §scxml-6.4: Use centralized UniqueIdGenerator with state ID for "stateid.platformid" format
    return UniqueIdGenerator::generateInvokeId(stateId);
}

// ============================================================================
// InvokeHandlerFactory Implementation
// ============================================================================

std::shared_ptr<IInvokeHandler> InvokeHandlerFactory::createHandler(const std::string &type,
                                                                    IScriptEngine &scriptEngine) {
    // W3C SCXML: Check if type matches SCXML invoke processor
    if (type == "scxml" || type == Constants::SCXML_INVOKE_PROCESSOR_URI ||
        type == std::string(Constants::SCXML_INVOKE_PROCESSOR_URI)
                   .substr(0, std::string(Constants::SCXML_INVOKE_PROCESSOR_URI).length() - 1)) {
        return std::make_shared<SCXMLInvokeHandler>(scriptEngine);
    }

    SCE_LOG_WARN("InvokeHandlerFactory: Unknown invoke type: {}, falling back to SCXML handler", type);
    return std::make_shared<SCXMLInvokeHandler>(scriptEngine);
}

// ============================================================================
// InvokeExecutor Implementation
// ============================================================================

InvokeExecutor::InvokeExecutor(IScriptEngine &scriptEngine, std::shared_ptr<IEventDispatcher> eventDispatcher)
    : scriptEngine_(scriptEngine), eventDispatcher_(eventDispatcher) {
    SCE_LOG_DEBUG("InvokeExecutor: Created with eventDispatcher: {}", eventDispatcher ? "provided" : "null");
}

InvokeExecutor::~InvokeExecutor() {
    size_t cancelled = cancelAllInvokes();
    if (cancelled > 0) {
        SCE_LOG_INFO("InvokeExecutor: Cancelled {} active invokes on destruction", cancelled);
    }
}

bool InvokeExecutor::executeInvokes(const std::vector<std::shared_ptr<IInvokeNode>> &invokes,
                                    const std::string &sessionId) {
    if (invokes.empty()) {
        SCE_LOG_DEBUG("InvokeExecutor: No invokes to execute for session: {}", sessionId);
        return true;
    }

    SCE_LOG_DEBUG("InvokeExecutor: Executing {} invokes for session: {}", invokes.size(), sessionId);

    bool allSucceeded = true;
    std::vector<std::string> sessionInvokeIds;

    for (const auto &invoke : invokes) {
        std::string invokeid = executeInvoke(invoke, sessionId);
        if (!invokeid.empty()) {
            sessionInvokeIds.push_back(invokeid);
            SCE_LOG_DEBUG("InvokeExecutor: Successfully started invoke: {} for session: {}", invokeid, sessionId);
        } else {
            SCE_LOG_ERROR("InvokeExecutor: Failed to start invoke for session: {}", sessionId);
            allSucceeded = false;
        }
    }

    // Track invokes by session for cleanup
    if (!sessionInvokeIds.empty()) {
        sessionInvokes_[sessionId] = sessionInvokeIds;
    }

    return allSucceeded;
}

std::string InvokeExecutor::executeInvoke(const std::shared_ptr<IInvokeNode> &invoke, const std::string &sessionId) {
    if (!invoke) {
        SCE_LOG_ERROR("InvokeExecutor: Cannot execute null invoke node");
        return "";
    }

    SCE_LOG_DEBUG("InvokeExecutor: executeInvoke called - session: {}, invokeId: {}, type: {}", sessionId, invoke->getId(),
              invoke->getType());

    std::string invokeType = invoke->getType();

    // W3C SCXML 1.0: Handle typeexpr attribute for dynamic type evaluation
    if (invokeType.empty() && !invoke->getTypeExpr().empty()) {
        try {
            auto future = scriptEngine_.evaluateExpression(sessionId, invoke->getTypeExpr());
            auto jsResult = future.get();
            if (!jsResult.isSuccess()) {
                SCE_LOG_ERROR("InvokeExecutor: Failed to evaluate typeexpr '{}': {}", invoke->getTypeExpr(),
                          jsResult.getErrorMessage());
                return "";
            }
            invokeType = ScriptResultUtils::resultToString(jsResult, &scriptEngine_, sessionId, invoke->getTypeExpr());
            SCE_LOG_DEBUG("InvokeExecutor: Evaluated typeexpr '{}' to type: '{}'", invoke->getTypeExpr(), invokeType);
        } catch (const std::exception &e) {
            SCE_LOG_ERROR("InvokeExecutor: Failed to evaluate typeexpr '{}': {}", invoke->getTypeExpr(), e.what());
            return "";
        }
    }

    if (invokeType.empty()) {
        invokeType = "scxml";  // Default to SCXML type
    }

    SCE_LOG_DEBUG("InvokeExecutor: Executing invoke of type: {} for session: {}", invokeType, sessionId);

    // §scxml-6.4: Generate invoke ID if not specified, BEFORE handler execution (test 224)
    // This ensures we can pre-register handler to prevent race conditions with immediate child completion
    std::string invokeid = invoke->getId();
    if (invokeid.empty()) {
        invokeid = generateInvokeId(invoke->getStateId());
        invoke->setId(invokeid);
        SCE_LOG_DEBUG("InvokeExecutor: Generated invoke ID: {}", invokeid);
    }

    // Check if invoke is already active to prevent duplicate execution
    if (isInvokeActive(invokeid)) {
        SCE_LOG_WARN("InvokeExecutor: Invoke {} already active, skipping duplicate execution", invokeid);
        return invokeid;
    }

    // Create appropriate handler using factory pattern
    auto handler = InvokeHandlerFactory::createHandler(invokeType, scriptEngine_);
    if (!handler) {
        SCE_LOG_ERROR("InvokeExecutor: Failed to create handler for invoke type: {}", invokeType);
        return "";
    }

    // W3C SCXML Test 192: Set parent StateMachine for completion callback (thread-safe)
    auto scxmlHandler = std::dynamic_pointer_cast<SCXMLInvokeHandler>(handler);
    auto parentSM = parentStateMachine_.lock();
    if (scxmlHandler && parentSM) {
        scxmlHandler->setParentStateMachine(parentSM);
    }

    // §scxml-5.10: Pre-register handler and invoke mapping BEFORE child starts (test 338)
    // This ensures mapping is available when child immediately completes and sends events
    invokeHandlers_[invokeid] = handler;
    SCE_LOG_DEBUG("InvokeExecutor: Pre-registered handler for invoke: {}", invokeid);

    std::string childSessionId = SessionRegistry::instance().generateSessionIdString("session_");
    SessionRegistry::instance().registerInvokeMapping(sessionId, invokeid, childSessionId);
    SCE_LOG_DEBUG("InvokeExecutor: Pre-registered invoke mapping - parent: {}, invoke: {}, child: {}", sessionId, invokeid,
              childSessionId);

    // Start invoke with pre-allocated child session ID
    std::string returnedId =
        handler->startInvokeWithSessionId(invoke, sessionId, eventDispatcher_, childSessionId, false);
    if (returnedId.empty()) {
        SCE_LOG_ERROR("InvokeExecutor: Handler failed to start invoke of type: {}", invokeType);

        // Cleanup: Remove both handler registration and invoke mapping
        invokeHandlers_.erase(invokeid);
        SessionRegistry::instance().unregisterInvokeMapping(sessionId, invokeid);

        return "";
    }

    // Track handler for cancellation (invokeid should match returnedId)

    SCE_LOG_INFO("InvokeExecutor: Successfully executed invoke: {} of type: {} for session: {}", invokeid, invokeType,
             sessionId);

    return invokeid;
}

bool InvokeExecutor::cancelInvoke(const std::string &invokeid) {
    auto handlerIt = invokeHandlers_.find(invokeid);
    if (handlerIt == invokeHandlers_.end()) {
        SCE_LOG_WARN("InvokeExecutor: Cannot cancel invoke - not found: {}", invokeid);
        return false;
    }

    bool cancelled = handlerIt->second->cancelInvoke(invokeid);
    if (cancelled) {
        cleanupInvoke(invokeid);
    }

    return cancelled;
}

size_t InvokeExecutor::cancelInvokesForSession(const std::string &sessionId) {
    auto sessionIt = sessionInvokes_.find(sessionId);
    if (sessionIt == sessionInvokes_.end()) {
        SCE_LOG_DEBUG("InvokeExecutor: No invokes to cancel for session: {}", sessionId);
        return 0;
    }

    size_t cancelledCount = 0;
    const auto &invokeIds = sessionIt->second;

    SCE_LOG_DEBUG("InvokeExecutor: Cancelling {} invokes for session: {}", invokeIds.size(), sessionId);

    for (const std::string &invokeid : invokeIds) {
        if (cancelInvoke(invokeid)) {
            cancelledCount++;
        }
    }

    // Remove session tracking
    sessionInvokes_.erase(sessionIt);

    SCE_LOG_INFO("InvokeExecutor: Cancelled {}/{} invokes for session: {}", cancelledCount, invokeIds.size(), sessionId);

    return cancelledCount;
}

size_t InvokeExecutor::cancelAllInvokes() {
    size_t cancelledCount = 0;

    // Cancel all active invokes
    for (auto &[invokeid, handler] : invokeHandlers_) {
        if (handler->isInvokeActive(invokeid)) {
            if (handler->cancelInvoke(invokeid)) {
                cancelledCount++;
            }
        }
    }

    // Clear all tracking
    invokeHandlers_.clear();
    sessionInvokes_.clear();

    SCE_LOG_INFO("InvokeExecutor: Cancelled {} invokes", cancelledCount);
    return cancelledCount;
}

bool InvokeExecutor::isInvokeActive(const std::string &invokeid) const {
    // Simply check if the invoke ID is registered in invokeHandlers_
    // This handles both pre-registered (to prevent duplicates) and fully active invokes
    auto handlerIt = invokeHandlers_.find(invokeid);
    return handlerIt != invokeHandlers_.end();
}

std::string InvokeExecutor::getStatistics() const {
    std::ostringstream stats;
    stats << "InvokeExecutor Statistics:\n";
    stats << "  Active invokes: " << invokeHandlers_.size() << "\n";
    stats << "  Sessions with invokes: " << sessionInvokes_.size() << "\n";
    stats << "  EventDispatcher: " << (eventDispatcher_ ? "available" : "null") << "\n";

    return stats.str();
}

void InvokeExecutor::setEventDispatcher(std::shared_ptr<IEventDispatcher> eventDispatcher) {
    eventDispatcher_ = eventDispatcher;
    SCE_LOG_DEBUG("InvokeExecutor: EventDispatcher set: {}", eventDispatcher ? "provided" : "null");
}

void InvokeExecutor::setParentStateMachine(std::shared_ptr<StateMachine> stateMachine) {
    parentStateMachine_ = stateMachine;  // Store as weak_ptr
    SCE_LOG_DEBUG("InvokeExecutor: Parent StateMachine set: {}", (void *)stateMachine.get());

    // Forward to all handlers that support it (currently only SCXML handler)
    for (const auto &[invokeid, handler] : invokeHandlers_) {
        auto scxmlHandler = std::dynamic_pointer_cast<SCXMLInvokeHandler>(handler);
        if (scxmlHandler) {
            scxmlHandler->setParentStateMachine(stateMachine);
        }
    }
}

std::string InvokeExecutor::generateInvokeId(const std::string &stateId) const {
    // §scxml-6.4: Use centralized UniqueIdGenerator with state ID for "stateid.platformid" format
    return UniqueIdGenerator::generateInvokeId(stateId);
}

std::vector<std::shared_ptr<StateMachine>> InvokeExecutor::getAutoForwardSessions(const std::string &parentSessionId) {
    std::vector<std::shared_ptr<StateMachine>> result;

    // Iterate through all handlers and collect autoForward sessions
    for (const auto &[invokeid, handler] : invokeHandlers_) {
        // Check if handler is SCXML type
        if (handler->getType() == "scxml") {
            auto scxmlHandler = std::dynamic_pointer_cast<SCXMLInvokeHandler>(handler);
            if (scxmlHandler) {
                auto sessions = scxmlHandler->getAutoForwardSessions(parentSessionId);
                result.insert(result.end(), sessions.begin(), sessions.end());
            }
        }
    }

    return result;
}

std::vector<std::shared_ptr<StateMachine>> InvokeExecutor::getAllInvokedSessions(const std::string &parentSessionId) {
    std::vector<std::shared_ptr<StateMachine>> result;

    // Iterate through all handlers and collect ALL active sessions (for visualization)
    for (const auto &[invokeid, handler] : invokeHandlers_) {
        // Check if handler is SCXML type
        if (handler->getType() == "scxml") {
            auto scxmlHandler = std::dynamic_pointer_cast<SCXMLInvokeHandler>(handler);
            if (scxmlHandler) {
                auto sessions = scxmlHandler->getAllInvokedSessions(parentSessionId);
                result.insert(result.end(), sessions.begin(), sessions.end());
            }
        }
    }

    return result;
}

void InvokeExecutor::cleanupInvoke(const std::string &invokeid) {
    // Remove from handler tracking
    invokeHandlers_.erase(invokeid);

    // Remove from session tracking
    for (auto &[sessionId, invokeIds] : sessionInvokes_) {
        auto it = std::find(invokeIds.begin(), invokeIds.end(), invokeid);
        if (it != invokeIds.end()) {
            invokeIds.erase(it);
            if (invokeIds.empty()) {
                sessionInvokes_.erase(sessionId);
            }
            break;
        }
    }

    SCE_LOG_DEBUG("InvokeExecutor: Cleaned up invoke: {}", invokeid);
}

std::string InvokeExecutor::getFinalizeScriptForChildSession(const std::string &childSessionId) const {
    SCE_LOG_DEBUG("InvokeExecutor: Looking up finalize script for child session: {}", childSessionId);

    // Iterate through all handlers to find the one with matching child session
    for (const auto &[invokeid, handler] : invokeHandlers_) {
        // Check if handler is SCXML type
        if (handler->getType() == "scxml") {
            auto scxmlHandler = std::dynamic_pointer_cast<SCXMLInvokeHandler>(handler);
            if (scxmlHandler) {
                std::string finalizeScript = scxmlHandler->getFinalizeScriptForChildSession(childSessionId);
                if (!finalizeScript.empty()) {
                    return finalizeScript;
                }
            }
        }
    }

    return "";
}

bool InvokeExecutor::shouldFilterCancelledInvokeEvent(const std::string &childSessionId) const {
    // W3C SCXML Test 252: Check all handlers to see if child session is from cancelled invoke
    for (const auto &[invokeid, handler] : invokeHandlers_) {
        if (handler->getType() == "scxml") {
            auto scxmlHandler = std::dynamic_pointer_cast<SCXMLInvokeHandler>(handler);
            if (scxmlHandler && scxmlHandler->shouldFilterCancelledInvokeEvent(childSessionId)) {
                return true;
            }
        }
    }
    return false;
}

void SCXMLInvokeHandler::setParentStateMachine(std::shared_ptr<StateMachine> stateMachine) {
    parentStateMachine_ = stateMachine;  // Store as weak_ptr
    SCE_LOG_DEBUG("SCXMLInvokeHandler: Parent StateMachine set: {}", (void *)stateMachine.get());
}

std::string SCXMLInvokeHandler::loadSCXMLFromFile(const std::string &filepath, const std::string &parentSessionId) {
    std::string cleanPath = filepath;

    // Remove "file:" prefix if present
    if (cleanPath.starts_with("file:")) {
        cleanPath = cleanPath.substr(5);
    }

    // Security: Validate path to prevent directory traversal attacks
    if (cleanPath.find("..") != std::string::npos) {
        SCE_LOG_ERROR("SCXMLInvokeHandler: Invalid file path - directory traversal detected: '{}'", cleanPath);
        return "";
    }

    // Security: Only allow .scxml and .txml file extensions
    std::filesystem::path pathCheck(cleanPath);
    std::string ext = pathCheck.extension().string();
    if (!ext.empty() && ext != ".scxml" && ext != ".txml") {
        SCE_LOG_ERROR("SCXMLInvokeHandler: Invalid file extension - only .scxml and .txml allowed: '{}'", cleanPath);
        return "";
    }

    // Handle relative path resolution using parent session file path
    std::filesystem::path resolvedPath;
    if (std::filesystem::path(cleanPath).is_relative()) {
        // Get parent session's file path for relative resolution
        std::string parentFilePath = SCE::SessionRegistry::instance().getSessionFilePath(parentSessionId);

        if (!parentFilePath.empty()) {
            // Resolve relative to parent SCXML file directory
            std::filesystem::path parentDir = std::filesystem::path(parentFilePath).parent_path();
            resolvedPath = parentDir / cleanPath;

            // Security: Normalize path and validate it stays within allowed boundaries
            try {
                resolvedPath = std::filesystem::weakly_canonical(resolvedPath);
                std::filesystem::path normalizedParentDir = std::filesystem::weakly_canonical(parentDir);

                // Check if resolved path is still within the parent directory tree
                auto relativePath = std::filesystem::relative(resolvedPath, normalizedParentDir);
                if (relativePath.string().starts_with("..")) {
                    SCE_LOG_ERROR("SCXMLInvokeHandler: Security violation - path escapes parent directory: '{}'",
                              resolvedPath.string());
                    return "";
                }
            } catch (const std::filesystem::filesystem_error &e) {
                SCE_LOG_ERROR("SCXMLInvokeHandler: Filesystem error during path normalization: {}", e.what());
                return "";
            }

            SCE_LOG_DEBUG("SCXMLInvokeHandler: Resolving relative path '{}' to '{}' (parent: '{}')", cleanPath,
                      resolvedPath.string(), parentFilePath);
        } else {
            // Production engine requires proper file path tracking - no fallback
            SCE_LOG_ERROR(
                "SCXMLInvokeHandler: No parent file path found for session '{}' - cannot resolve relative path: '{}'",
                parentSessionId, cleanPath);
            return "";
        }
    } else {
        resolvedPath = cleanPath;
    }

    // Check if SCXML or TXML file exists
    std::filesystem::path finalPath = resolvedPath;
    if (!finalPath.has_extension()) {
        // Try .scxml first, then .txml
        std::filesystem::path scxmlPath = finalPath;
        scxmlPath += ".scxml";
        std::filesystem::path txmlPath = finalPath;
        txmlPath += ".txml";

        if (std::filesystem::exists(scxmlPath)) {
            finalPath = scxmlPath;
        } else if (std::filesystem::exists(txmlPath)) {
            finalPath = txmlPath;
        } else {
            SCE_LOG_ERROR("SCXMLInvokeHandler: SCXML/TXML file not found: '{}' (tried .scxml and .txml)",
                      finalPath.string());
            return "";
        }
    } else {
        // Extension specified, verify file exists
        if (!std::filesystem::exists(finalPath)) {
            SCE_LOG_ERROR("SCXMLInvokeHandler: File not found: '{}'", finalPath.string());
            return "";
        }
    }

    SCE_LOG_DEBUG("SCXMLInvokeHandler: Found file at '{}'", finalPath.string());

    // Read file content
    std::string content;

#ifdef __EMSCRIPTEN__
    // §scxml-6.4: Use C-style file I/O for Emscripten virtual filesystem compatibility
    // Emscripten's MEMFS supports fopen/fread but not std::ifstream
    FILE *file = fopen(finalPath.string().c_str(), "rb");
    if (!file) {
        SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to read file '{}': {}", finalPath.string(), strerror(errno));
        return "";
    }

    // Get file size
    fseek(file, 0, SEEK_END);
    long fileSize = ftell(file);
    fseek(file, 0, SEEK_SET);

    // Read entire file
    content.resize(fileSize);
    size_t bytesRead = fread(&content[0], 1, fileSize, file);
    fclose(file);

    if (bytesRead != static_cast<size_t>(fileSize)) {
        SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to read complete file: '{}' (read {} of {} bytes)", finalPath.string(),
                  bytesRead, fileSize);
        return "";
    }
#else
    // Native build: Use C++ std::ifstream
    std::ifstream file(finalPath);
    if (!file.is_open()) {
        SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to open file: '{}'", finalPath.string());
        return "";
    }

    content = std::string((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    file.close();
#endif

    if (content.empty()) {
        SCE_LOG_WARN("SCXMLInvokeHandler: File is empty: '{}'", finalPath.string());
        return "";
    }

    // Production engine only handles pure SCXML files

    SCE_LOG_INFO("SCXMLInvokeHandler: Successfully loaded SCXML content from file: '{}' ({} bytes)", finalPath.string(),
             content.length());

    return content;
}

std::vector<std::shared_ptr<StateMachine>>
SCXMLInvokeHandler::getAutoForwardSessions(const std::string &parentSessionId) {
    std::vector<std::shared_ptr<StateMachine>> result;
    for (auto &[invokeid, session] : activeSessions_) {
        if (session.isActive && session.parentSessionId == parentSessionId && session.autoForward) {
            if (session.smContext) {
                // §scxml-6.4: Use shared_ptr to prevent use-after-free during autoForward iteration (test 338)
                // Child might reach final state during processEvent, triggering cleanup
                result.push_back(session.smContext->getShared());
            }
        }
    }
    return result;
}

std::vector<std::shared_ptr<StateMachine>>
SCXMLInvokeHandler::getAllInvokedSessions(const std::string &parentSessionId) {
    std::vector<std::shared_ptr<StateMachine>> result;
    for (auto &[invokeid, session] : activeSessions_) {
        if (session.isActive && session.parentSessionId == parentSessionId) {
            if (session.smContext) {
                // Return ALL active children for visualization, not just autoForward ones
                result.push_back(session.smContext->getShared());
            }
        }
    }
    return result;
}

std::string SCXMLInvokeHandler::getFinalizeScriptForChildSession(const std::string &childSessionId) const {
    // W3C SCXML Test 233, 234: Lookup from separate finalize script storage
    // This ensures script is available even after invoke cancellation
    std::lock_guard<std::mutex> lock(finalizeScriptsMutex_);
    auto it = finalizeScripts_.find(childSessionId);
    if (it != finalizeScripts_.end()) {
        SCE_LOG_DEBUG("SCXMLInvokeHandler: Found finalize script for child session: {}", childSessionId);
        return it->second;
    }

    SCE_LOG_DEBUG("SCXMLInvokeHandler: No finalize script found for child session: {}", childSessionId);
    return "";
}

std::shared_ptr<StateSnapshot> SCXMLInvokeHandler::captureChildState() const {
    // §scxml-3.11: Capture child state machine configuration
    // Find active session for this handler
    for (const auto &[invokeid, session] : activeSessions_) {
        if (!session.isActive || !session.smContext) {
            continue;
        }

        // Get child StateMachine from context
        auto childSM = session.smContext->get();
        if (!childSM) {
            continue;
        }

        // Create child snapshot
        auto childSnapshot = std::make_shared<StateSnapshot>();

        // Capture active states (preserve document order for time-travel debugging)
        childSnapshot->activeStates = childSM->getActiveStates();

        // Capture event queues from child's EventRaiser
        auto childEventRaiser = childSM->getEventRaiser();
        if (childEventRaiser) {
            childEventRaiser->getEventQueues(childSnapshot->internalQueue, childSnapshot->externalQueue);
            SCE_LOG_DEBUG("SCXMLInvokeHandler: Captured child event queues - internal: {}, external: {}",
                      childSnapshot->internalQueue.size(), childSnapshot->externalQueue.size());
        }

        // §scxml-3.11: Capture child's nested invocations recursively
        auto childInvokeExecutor = childSM->getInvokeExecutor();
        if (childInvokeExecutor) {
            childInvokeExecutor->captureInvokeState(childSnapshot->activeInvokes);
            SCE_LOG_DEBUG("SCXMLInvokeHandler: Captured {} child nested invocations", childSnapshot->activeInvokes.size());
        }

        // Capture datamodel (simplified - would need JSEngine integration for full implementation)
        // Full implementation would extract datamodel via JSEngine

        // §scxml-6.2: Capture child's scheduled events for accurate restoration
        // Parent and child share the same scheduler (via eventDispatcher)
        // Filter by child sessionId to get only child's scheduled events
        if (session.eventDispatcher) {
            auto eventDispatcherImpl = std::dynamic_pointer_cast<EventDispatcherImpl>(session.eventDispatcher);
            if (eventDispatcherImpl) {
                auto scheduler = eventDispatcherImpl->getScheduler();
                if (scheduler) {
                    auto allScheduledEvents = scheduler->getScheduledEvents();

                    // Filter events belonging to this child session
                    for (const auto &event : allScheduledEvents) {
                        if (event.sessionId == session.sessionId) {
                            // Convert ScheduledEventInfo to ScheduledEventSnapshot
                            std::map<std::string, std::string> paramsMap;
                            for (const auto &[paramName, paramValues] : event.params) {
                                if (!paramValues.empty()) {
                                    paramsMap[paramName] = paramValues[0];
                                }
                            }

                            childSnapshot->scheduledEvents.emplace_back(
                                event.eventName, event.sendId, event.originalDelay.count(), event.remainingTime.count(),
                                event.sessionId, event.targetUri, event.eventType, event.eventData, event.content,
                                paramsMap);
                        }
                    }

                    SCE_LOG_DEBUG("SCXMLInvokeHandler: Captured {} child scheduled events for session {}",
                              childSnapshot->scheduledEvents.size(), session.sessionId);
                }
            }
        }

        SCE_LOG_DEBUG("SCXMLInvokeHandler: Captured complete child state - {} active states, {} queued events, {} "
                  "scheduled events",
                  childSnapshot->activeStates.size(),
                  childSnapshot->internalQueue.size() + childSnapshot->externalQueue.size(),
                  childSnapshot->scheduledEvents.size());

        return childSnapshot;
    }

    return nullptr;
}

void SCXMLInvokeHandler::restoreChildState(const StateSnapshot &childSnapshot, const std::string &childSessionId) {
    // §scxml-3.11: Restore child configuration without side effects
    // ARCHITECTURE.md: Zero Duplication - delegates to StateMachine::restoreFromSnapshot()

    // Find session by child session ID
    for (auto &[invokeid, session] : activeSessions_) {
        if (session.sessionId != childSessionId || !session.smContext) {
            continue;
        }

        auto childSM = session.smContext->get();
        if (!childSM) {
            continue;
        }

        // Complete restoration using Template Method pattern
        // ARCHITECTURE.md: Single Source of Truth - StateMachine handles restoration lifecycle
        // This automatically handles: JS environment init, state restoration, running flag
        if (!childSM->restoreFromSnapshot(childSnapshot.activeStates)) {
            SCE_LOG_ERROR("SCXMLInvokeHandler: Failed to restore child state for session {}", childSessionId);
            return;
        }

        // Restore child's event queues
        auto childEventRaiser = childSM->getEventRaiser();
        if (childEventRaiser) {
            auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(childEventRaiser);
            if (eventRaiserImpl) {
                eventRaiserImpl->clearQueue();
                for (const auto &eventSnapshot : childSnapshot.internalQueue) {
                    eventRaiserImpl->raiseInternalEvent(eventSnapshot.name, eventSnapshot.data);
                }
                for (const auto &eventSnapshot : childSnapshot.externalQueue) {
                    eventRaiserImpl->raiseExternalEvent(eventSnapshot.name, eventSnapshot.data);
                }
                SCE_LOG_DEBUG("SCXMLInvokeHandler: Restored child event queues - internal: {}, external: {}",
                          childSnapshot.internalQueue.size(), childSnapshot.externalQueue.size());

                // CRITICAL FIX: Enable immediate mode for child EventRaiser after restoration
                // §scxml-6.4: Child state machines must process parent events immediately (Test 192)
                // Without this, events from parent get queued but never processed in interactive mode
                // Root cause: EventRaiserImpl defaults to immediateMode=false in constructor
                // During normal invoke, child processes events immediately via callback
                // After reset(), new EventRaiser instance needs immediateMode=true to match normal behavior
                eventRaiserImpl->setImmediateMode(true);
                SCE_LOG_INFO("SCXMLInvokeHandler: Enabled immediate mode for restored child EventRaiser (session: {})",
                         childSessionId);
            }
        }

        // §scxml-3.11: Restore child's nested invocations recursively
        auto childInvokeExecutor = childSM->getInvokeExecutor();
        if (childInvokeExecutor && !childSnapshot.activeInvokes.empty()) {
            // Get shared_ptr from StateMachineContext for nested invocation restoration
            auto childSMShared = session.smContext->getShared();
            if (childSMShared) {
                childInvokeExecutor->restoreInvokeState(childSnapshot.activeInvokes, childSMShared);
                SCE_LOG_DEBUG("SCXMLInvokeHandler: Restored {} child nested invocations",
                          childSnapshot.activeInvokes.size());
            }
        }

        // §scxml-6.2: Restore child's scheduled events for accurate time-travel debugging
        if (session.eventDispatcher && !childSnapshot.scheduledEvents.empty()) {
            auto eventDispatcherImpl = std::dynamic_pointer_cast<EventDispatcherImpl>(session.eventDispatcher);
            if (eventDispatcherImpl) {
                auto scheduler = eventDispatcherImpl->getScheduler();
                if (scheduler) {
                    // Cancel all current child scheduled events
                    auto currentScheduledEvents = scheduler->getScheduledEvents();
                    for (const auto &event : currentScheduledEvents) {
                        if (event.sessionId == childSessionId) {
                            scheduler->cancelEvent(event.sendId);
                            SCE_LOG_DEBUG(
                                "SCXMLInvokeHandler: Canceled child scheduled event '{}' (sendId: {}) before restore",
                                event.eventName, event.sendId);
                        }
                    }

                    // Recreate scheduled events from snapshot with accurate remainingTime
                    for (const auto &eventSnapshot : childSnapshot.scheduledEvents) {
                        // §scxml-6.2.3: Recreate send operation with remaining time
                        EventDescriptor event;
                        event.eventName = eventSnapshot.eventName;
                        event.data = eventSnapshot.eventData;
                        event.type = eventSnapshot.eventType;
                        event.target = eventSnapshot.targetUri;
                        event.sendId = eventSnapshot.sendId;
                        event.sessionId = childSessionId;  // Preserve child session ID

                        // §scxml-6.2: Restore params for _event.data construction
                        for (const auto &[paramName, paramValue] : eventSnapshot.params) {
                            event.params[paramName] = {paramValue};
                        }

                        // Schedule with remaining time (not original delay)
                        auto delay = std::chrono::milliseconds(eventSnapshot.remainingTimeMs);
                        auto future = session.eventDispatcher->sendEventDelayed(event, delay);

                        SCE_LOG_DEBUG("SCXMLInvokeHandler: Restored child scheduled event '{}' (sendId: {}, remainingTime: "
                                  "{}ms, originalDelay: {}ms)",
                                  eventSnapshot.eventName, eventSnapshot.sendId, eventSnapshot.remainingTimeMs,
                                  eventSnapshot.originalDelayMs);
                    }

                    SCE_LOG_DEBUG("SCXMLInvokeHandler: Restored {} child scheduled events for session {}",
                              childSnapshot.scheduledEvents.size(), childSessionId);
                }
            }
        }

        SCE_LOG_DEBUG("SCXMLInvokeHandler: Successfully restored complete child state - {} active states, {} queued "
                  "events for session {}",
                  childSnapshot.activeStates.size(),
                  childSnapshot.internalQueue.size() + childSnapshot.externalQueue.size(), childSessionId);
        return;
    }

    SCE_LOG_WARN("SCXMLInvokeHandler: Could not restore child state - session {} not found", childSessionId);
}

std::string SCXMLInvokeHandler::getChildSessionId() const {
    // Return first active session's ID
    for (const auto &[invokeid, session] : activeSessions_) {
        if (session.isActive) {
            return session.sessionId;
        }
    }
    return "";
}

std::string SCXMLInvokeHandler::getSCXMLContent() const {
    // Return SCXML content from first active session
    for (const auto &[invokeid, session] : activeSessions_) {
        if (session.isActive) {
            return session.scxmlContent;
        }
    }
    return "";
}

bool SCXMLInvokeHandler::getAutoForward() const {
    // §scxml-6.4: Return autoForward flag from first active session
    // Matches getSCXMLContent() pattern - assumes single active invoke per handler
    for (const auto &[invokeid, session] : activeSessions_) {
        if (session.isActive) {
            return session.autoForward;
        }
    }
    return false;
}

void InvokeExecutor::captureInvokeState(std::vector<InvokeSnapshot> &out) const {
    // §scxml-3.11: Capture all active invocations
    // Zero Duplication: Iterate through handlers and delegate to them

    for (const auto &[invokeid, handler] : invokeHandlers_) {
        if (!handler->isInvokeActive(invokeid)) {
            continue;
        }

        // Only handle SCXML invokes (can be extended for other types)
        auto scxmlHandler = std::dynamic_pointer_cast<SCXMLInvokeHandler>(handler);
        if (!scxmlHandler) {
            SCE_LOG_DEBUG("InvokeExecutor: Skipping non-SCXML invoke: {}", invokeid);
            continue;
        }

        // Create invoke snapshot
        InvokeSnapshot snapshot;
        snapshot.invokeId = invokeid;
        snapshot.parentStateId = extractParentStateIdFromInvokeId(invokeid);
        snapshot.childSessionId = scxmlHandler->getChildSessionId();
        snapshot.type = scxmlHandler->getType();
        snapshot.scxmlContent = scxmlHandler->getSCXMLContent();
        snapshot.finalizeScript = scxmlHandler->getFinalizeScriptForChildSession(
            snapshot.childSessionId);                           // §scxml-6.5: Capture finalize script (Test 233)
        snapshot.autoForward = scxmlHandler->getAutoForward();  // §scxml-6.4: Capture autoforward flag (Test 229)

        // Capture child state recursively
        snapshot.childState = scxmlHandler->captureChildState();

        out.push_back(snapshot);

        SCE_LOG_DEBUG("InvokeExecutor: Captured invoke state - invokeId: {}, childSession: {}", invokeid,
                  snapshot.childSessionId);
    }

    SCE_LOG_INFO("InvokeExecutor: Captured {} active invocations", out.size());
}

void InvokeExecutor::restoreInvokeState(const std::vector<InvokeSnapshot> &invokes,
                                        std::shared_ptr<StateMachine> parentSM) {
    // §scxml-3.11: Restore invoke configuration without side effects
    // Zero Duplication: Use captured SCXML content directly (no src/srcexpr re-evaluation)

    // CRITICAL FIX: Cancel ALL existing invokes before restoration (Test 192 reset bug)
    // Without this, old invoke sessions remain active in JSEngine with stale invoke mappings
    // causing event routing to fail when parent sends events to child after reset()
    SCE_LOG_INFO("InvokeExecutor: Cancelling {} existing invocations before restoration", invokeHandlers_.size());

    // Create a copy of invoke IDs to avoid iterator invalidation during cancellation
    std::vector<std::string> existingInvokeIds;
    for (const auto &[invokeid, handler] : invokeHandlers_) {
        existingInvokeIds.push_back(invokeid);
    }

    // Cancel each existing invoke
    for (const auto &invokeid : existingInvokeIds) {
        auto it = invokeHandlers_.find(invokeid);
        if (it != invokeHandlers_.end() && it->second) {
            it->second->cancelInvoke(invokeid);
            SCE_LOG_DEBUG("InvokeExecutor: Cancelled existing invoke: {}", invokeid);
        }
    }

    // Clear handlers map (handlers were already cancelled above)
    invokeHandlers_.clear();
    SCE_LOG_INFO("InvokeExecutor: Cleared all existing invoke handlers before restoration");

    SCE_LOG_INFO("InvokeExecutor: Restoring {} invocations", invokes.size());

    for (const auto &snapshot : invokes) {
        // §scxml-6.4: Extract parent state ID from invoke ID format
        std::string parentStateId = extractParentStateIdFromInvokeId(snapshot.invokeId);

        // §scxml-3.11: Create temporary InvokeNode with captured SCXML content
        // This bypasses src/srcexpr evaluation which would fail in new parent session context
        // (new session after reset has no file path context)
        auto invokeNode = std::make_shared<InvokeNode>(snapshot.invokeId);
        invokeNode->setContent(snapshot.scxmlContent);  // Use captured content directly
        invokeNode->setStateId(parentStateId);
        invokeNode->setType(snapshot.type);
        invokeNode->setFinalize(snapshot.finalizeScript);  // §scxml-6.5: Restore finalize script (Test 233)
        invokeNode->setAutoForward(snapshot.autoForward);  // §scxml-6.4: Restore autoforward flag (Test 229)
        // Leave src/srcexpr/contentexpr empty - startInvokeInternal will use content field

        // Create SCXML invoke handler
        auto handler = std::make_shared<SCXMLInvokeHandler>(scriptEngine_);
        handler->setParentStateMachine(parentSM);

        // Register handler
        invokeHandlers_[snapshot.invokeId] = handler;

        // §scxml-3.11: Restore invoke with isRestoration=true (skip completion callback and start())
        handler->startInvokeWithSessionId(invokeNode,                // Temporary InvokeNode with captured content
                                          parentSM->getSessionId(),  // parentSessionId
                                          eventDispatcher_,          // eventDispatcher
                                          snapshot.childSessionId,   // childSessionId
                                          true                       // isRestoration - skip side effects
        );

        // Restore child state if available
        if (snapshot.childState) {
            SCE_LOG_INFO("[CHILD STATE RESTORE] Starting restoration for child session: {}", snapshot.childSessionId);
            handler->restoreChildState(*snapshot.childState, snapshot.childSessionId);
            SCE_LOG_INFO("[CHILD STATE RESTORE] Completed for child session: {}", snapshot.childSessionId);
        } else {
            SCE_LOG_WARN("[CHILD STATE RESTORE] No child state snapshot available for: {}", snapshot.childSessionId);
        }

        SCE_LOG_INFO("[INVOKE RESTORED] invokeId: {}, childSession: {}", snapshot.invokeId, snapshot.childSessionId);
    }
}

}  // namespace SCE