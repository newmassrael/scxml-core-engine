// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/StateMachine.h"
#include "scripting/ScriptResultUtils.h"
#include "scripting/SessionRegistry.h"

#include "common/DoneDataHelper.h"
#include "core/EntryExitHelper.h"
#include "core/LogMacros.h"
#include "core/MicrostepAlgorithms.h"
#include "core/ParallelCompletionHelper.h"
#ifdef SCE_USE_SPDLOG
#include <spdlog/spdlog.h>
#endif
#include "common/StringUtils.h"
#include "core/TransitionHelper.h"
#include "events/EventRaiserService.h"

#include "factory/NodeFactory.h"
#include "model/ITransitionNode.h"
#include "model/SCXMLModel.h"
#include "parsing/ActionParser.h"
#include "parsing/IXMLParser.h"
#include "parsing/SCXMLParser.h"
#include "parsing/XIncludeProcessor.h"
#include "runtime/ActionExecutorImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/ExecutionContextImpl.h"
#include "runtime/HistoryManager.h"
#include "runtime/HistoryStateAutoRegistrar.h"
#include "runtime/HistoryValidator.h"
#include "scripting/IScriptEngine.h"
#include "scripting/ScriptEngineProvider.h"  // For backward-compat static factory methods
#include <algorithm>
#include <fstream>
#include <set>
#include <sstream>

namespace SCE {

// ── W3C SCXML Appendix D over the parsed model ───────────────────────────
//
// What Appendix D does with a document is `SCE::Core::MicrostepAlgorithms`,
// the procedure the AOT engine runs too. This is the machine it is handed:
// the parsed document as the appendix reads it (`InterpreterDocument`), what
// each `<history>` recorded (`HistoryManager`), the configuration
// (`StateHierarchyManager`), and what exiting a state, entering one and taking
// a transition do in this engine.
struct StateMachine::MicrostepHost {
    using State = std::string;
    using History = std::string;

    StateMachine &machine;

    std::optional<State> parentOf(const State &s) const {
        return machine.document_->parentOf(s);
    }

    bool isCompound(const State &s) const {
        return machine.document_->isCompound(s);
    }

    bool isParallel(const State &s) const {
        return machine.document_->isParallel(s);
    }

    std::vector<State> childStates(const State &s) const {
        return machine.document_->childStates(s);
    }

    std::vector<Core::EntryTarget<State, History>> initialTargets(const State &s) const {
        return machine.document_->initialTargets(s);
    }

    State historyParent(const History &h) const {
        return machine.document_->historyParent(h);
    }

    std::optional<std::vector<State>> historyValue(const History &h) const {
        return machine.historyManager_ ? machine.historyManager_->recordedValue(h) : std::nullopt;
    }

    std::vector<Core::EntryTarget<State, History>> historyDefaultTargets(const History &h) const {
        return machine.document_->historyDefaultTargets(h);
    }

    int documentOrder(const State &s) const {
        return machine.document_->documentOrder(s);
    }

    std::vector<State> configuration() const {
        return machine.hierarchyManager_->getActiveStates();
    }

    std::optional<Transition> firstEnabledTransition(const State &s, const std::string &eventName) {
        return machine.firstEnabledTransition(s, eventName);
    }

    void exitState(const State &s, const std::vector<State> &configurationBeforeExit) {
        machine.exitStateInMicrostep(s, configurationBeforeExit);
    }

    void executeTransitionContent(const Transition &t) {
        machine.executeTransitionContent(t);
    }

    void enterState(const State &s, bool isDefaultEntry) {
        machine.enterStateInMicrostep(s, isDefaultEntry);
    }

    void executeHistoryDefaultContent(const History &h) {
        machine.executeHistoryDefaultContent(h);
    }
};

StateMachine::MacrostepScope::MacrostepScope(StateMachine &machine) : machine_(machine) {
    machine_.macrostepOwner_.store(std::this_thread::get_id());
    if (machine_.eventRaiser_) {
        machine_.eventRaiser_->setImmediateMode(false);
    }
}

StateMachine::MacrostepScope::~MacrostepScope() {
    machine_.macrostepOwner_.store(std::thread::id{});
    if (machine_.eventRaiser_) {
        machine_.eventRaiser_->setImmediateMode(machine_.autoProcessQueuedEvents_);
    }
}

bool StateMachine::macrostepInProgressOnThisThread() const {
    return macrostepOwner_.load() == std::this_thread::get_id();
}

std::shared_ptr<StateMachine> StateMachine::createFromSCXMLString(const std::string &scxmlContent,
                                                                  const std::string &sessionId) {
    return createFromSCXMLString(ScriptEngineProvider::getScriptEngine(), scxmlContent, sessionId);
}

std::shared_ptr<StateMachine> StateMachine::createFromSCXMLString(IScriptEngine &scriptEngine,
                                                                  const std::string &scxmlContent,
                                                                  const std::string &sessionId) {
    auto sm = std::make_shared<StateMachine>(scriptEngine, sessionId);
    if (!sm->loadSCXMLFromString(scxmlContent)) {
        SCE_LOG_ERROR("StateMachine::createFromSCXMLString: Failed to load SCXML from string");
        return nullptr;
    }
    return sm;
}

StateMachine::StateMachine(IScriptEngine &scriptEngine, const std::string &sessionId)
    : scriptEngine_(scriptEngine), jsEnvironmentReady_(false) {
    if (sessionId.empty()) {
        sessionId_ = SessionRegistry::instance().generateSessionIdString("sm_");
    } else {
        sessionId_ = sessionId;
        SCE_LOG_DEBUG("StateMachine: Created with injected session ID: {}", sessionId_);
    }

    // Script environment uses lazy initialization
    // ActionExecutor and ExecutionContext initialized in setupJSEnvironment

    // Initialize History Manager with SOLID architecture (Dependency Injection)
    initializeHistoryManager();

    // Initialize InvokeExecutor with SOLID architecture (W3C SCXML invoke support)
    invokeExecutor_ = std::make_unique<InvokeExecutor>(scriptEngine_, nullptr);
}

StateMachine::~StateMachine() {
    // Clear callbacks first to prevent execution during destruction
    completionCallback_ = nullptr;

    // CRITICAL: Clear EventRaiser callback to prevent heap-use-after-free
    // EventScheduler threads may still be running and executing callbacks
    // Clearing the callback ensures they won't access destroyed StateMachine
    // DO NOT shutdown EventDispatcher here - it would cancel delayed events needed by W3C tests
    if (eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            SCE_LOG_DEBUG("StateMachine: Clearing EventRaiser callback before destruction");
            eventRaiserImpl->clearEventCallback();
        }
    }

    // CRITICAL: Wait for any in-progress macrostep to complete (ASAN heap-use-after-free fix)
    // Lock mutex to ensure no processEvent is running when we proceed with destruction
    {
        std::lock_guard<std::mutex> processEventLock(processEventMutex_);
        SCE_LOG_DEBUG("StateMachine: All processEvent calls completed, proceeding with destruction");
    }

    // Always call stop() to ensure session cleanup
    // Final state sets isRunning_=false but session must still be destroyed
    // stop() is idempotent and handles cleanup even when isRunning_=false
    stop();

    // FUNDAMENTAL FIX: Two-Phase Destruction Pattern
    // LIFECYCLE: RAII Destruction Stage
    // Destructor handles only internal resource cleanup (no external dependencies)
    // JSEngine session already destroyed in stop() to prevent deadlock
    // See stop() method for explicit cleanup of external dependencies
    SCE_LOG_DEBUG("StateMachine: Destruction complete (JSEngine session cleaned up in stop())");
}

bool StateMachine::loadSCXML(const std::string &filename) {
    try {
        auto nodeFactory = std::make_shared<NodeFactory>();
        auto xincludeProcessor = std::make_shared<XIncludeProcessor>();
        SCXMLParser parser(nodeFactory, xincludeProcessor);

        model_ = parser.parseFile(filename);
        if (!model_) {
            SCE_LOG_ERROR("Failed to parse SCXML file: {}", filename);
            return false;
        }

        // Register file path for this session to enable relative path resolution
        SCE::SessionRegistry::instance().registerSessionFilePath(sessionId_, filename);
        SCE_LOG_DEBUG("StateMachine: Registered file path '{}' for session '{}'", filename, sessionId_);

        return initializeFromModel();
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Exception loading SCXML: {}", e.what());
        return false;
    }
}

bool StateMachine::loadSCXMLFromString(const std::string &scxmlContent) {
    try {
        auto nodeFactory = std::make_shared<NodeFactory>();
        auto xincludeProcessor = std::make_shared<XIncludeProcessor>();
        SCXMLParser parser(nodeFactory, xincludeProcessor);

        // Use parseContent method which exists in SCXMLParser
        model_ = parser.parseContent(scxmlContent);
        if (!model_) {
            const auto &errors = parser.getErrorMessages();
            lastLoadError_ = errors.empty() ? "Failed to parse SCXML content" : errors.back();
            SCE_LOG_ERROR("StateMachine: {}", lastLoadError_);
            return false;
        }

        return initializeFromModel();
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Exception parsing SCXML content: {}", e.what());
        return false;
    }
}

bool StateMachine::loadModel(std::shared_ptr<SCXMLModel> model) {
    if (!model) {
        SCE_LOG_ERROR("StateMachine: Cannot load null model");
        return false;
    }

    model_ = model;
    return initializeFromModel();
}

bool StateMachine::start(bool autoProcessQueuedEvents) {
    // Interactive mode: the host steps queued events itself
    autoProcessQueuedEvents_ = autoProcessQueuedEvents;

    if (!document_ || document_->documentInitialTargets().empty()) {
        SCE_LOG_ERROR("StateMachine: Cannot start - no initial state defined");
        return false;
    }

    // Ensure JS environment initialization
    if (!ensureJSEnvironment()) {
        SCE_LOG_ERROR("StateMachine: Cannot start - JavaScript environment initialization failed");
        return false;
    }

    // Check EventRaiser status at StateMachine start
    if (eventRaiser_) {
        SCE_LOG_DEBUG("StateMachine: EventRaiser status check - EventRaiser: {}, sessionId: {}",
                      (void *)eventRaiser_.get(), sessionId_);
    } else {
        SCE_LOG_WARN("StateMachine: EventRaiser is null - sessionId: {}", sessionId_);
    }

    std::lock_guard<std::mutex> processEventLock(processEventMutex_);
    MacrostepScope macrostep(*this);

    // §scxml-D-interpret: initialise the global data structures and the data model,
    // run the global <script>, then enter the initial configuration and set running.
    isRunning_ = true;
    topLevelFinalReached_ = false;
    macrostepTruncated_ = false;
    macrostepMicrostepsTaken_ = 0;

    enterInitialConfiguration();
    runMainEventLoop();
    if (topLevelFinalReached_) {
        finishAtTopLevelFinal();
    }

    updateStatistics();

    SCE_LOG_INFO("StateMachine: Started successfully");
    return true;
}

void StateMachine::stop() {
    SCE_LOG_DEBUG("StateMachine: Stopping state machine (isRunning: {})", isRunning_.load());

    // W3C SCXML Test 250: Exit ALL active states with onexit handlers (only if still running)
    // §scxml-D-exitInterpreter: exit every active state in exit order, running each
    // state's <onexit> content, and clear the configuration.
    if (isRunning_) {
        {
            // The onexit content is executable content like any other: what it
            // raises is queued, not handed straight back to a stopping machine.
            MacrostepScope macrostep(*this);
            for (const auto &state : configurationInExitOrder()) {
                exitState(state);
            }
        }

        isRunning_ = false;

        // State management delegated to StateHierarchyManager
        if (hierarchyManager_) {
            hierarchyManager_->reset();
        }
    }

    // CRITICAL: Always unregister state query callback, even if isRunning_ is already false
    // Race condition prevention: JSEngine worker threads may have queued tasks accessing StateMachine
    // W3C Test 415: isRunning_=false may be set in top-level final state before destructor calls stop()
    scriptEngine_.setStateQueryCallback(nullptr, sessionId_);
    SCE_LOG_DEBUG("StateMachine: Unregistered state query callback from script engine");

    // FUNDAMENTAL FIX: Two-Phase Destruction Pattern
    // LIFECYCLE: Explicit Cleanup Stage
    // W3C SCXML: Destroy JSEngine session before RAII destruction
    // Ensures JSEngine singleton is alive during cleanup (prevents deadlock)
    // Required for StaticExecutionEngine wrapper lifecycle management
    if (jsEnvironmentReady_) {
        scriptEngine_.destroySession(sessionId_);
        jsEnvironmentReady_ = false;
        SCE_LOG_DEBUG("StateMachine: Destroyed JSEngine session in stop(): {}", sessionId_);
    }

    updateStatistics();
    SCE_LOG_INFO("StateMachine: Stopped");
}

StateMachine::TransitionResult StateMachine::processEvent(const std::string &eventName, const std::string &eventData) {
    // §scxml-3.12 classifies an event a host hands to a running machine as
    // an external one, and W3C SCXML Appendix D's mainEventLoop owes every
    // external event the preliminary step — `<finalize>` and the autoforward
    // copy — before it selects transitions for it.
    //
    // That step keys on `isCurrentEventFromExternalQueue()`, which is a
    // thread-local the EventRaiser establishes around each dispatch. A host
    // calling this method goes through no raiser, so the flag it read was
    // whatever the last dispatch left behind: cleared, hence false. Measured
    // 2026-08-21 — an `autoforward` child saw nothing a host delivered this
    // way, while the identical document forwarded the same event when the
    // raiser carried it. The C++ AOT engine had the same defect at its own
    // host-facing door, for the same reason: the step was bound to the path
    // instead of to the event.
    //
    // Establishing the context here rather than forcing the flag deeper keeps
    // the queue category one fact with one owner. The raiser's own callbacks
    // reach the machine through `processDispatchedEvent` below, so an internal
    // event still arrives with `isExternalQueue` false and stays out of the
    // forwarded set — which is what `autoforward_internal_queue` pins.
    EventRaiserImpl::EventContext hostContext;
    hostContext.isExternalQueue = true;
    EventRaiserImpl::EventContextGuard hostGuard(std::move(hostContext));

    return processDispatchedEvent(eventName, eventData);
}

StateMachine::TransitionResult StateMachine::processDispatchedEvent(const std::string &eventName,
                                                                    const std::string &eventData) {
    // §scxml-5.10: Check if there's an origin session ID from EventRaiser thread-local storage
    std::string originSessionId = EventRaiserImpl::getCurrentOriginSessionId();

    // §scxml-5.10: Check if there's a send ID from EventRaiser thread-local storage (for error events)
    std::string sendId = EventRaiserImpl::getCurrentSendId();

    // §scxml-5.10: Check if there's an invoke ID from EventRaiser thread-local storage (test 338)
    std::string invokeId = EventRaiserImpl::getCurrentInvokeId();

    // §scxml-5.10: Check if there's an origin type from EventRaiser thread-local storage (test 253, 331, 352, 372)
    std::string originType = EventRaiserImpl::getCurrentOriginType();

    // Delegate to overload with originSessionId (may be empty for non-invoke events)
    return processEvent(eventName, eventData, originSessionId, sendId, invokeId, originType);
}

StateMachine::TransitionResult StateMachine::processEvent(const std::string &eventName, const std::string &eventData,
                                                          const std::string &originSessionId, const std::string &sendId,
                                                          const std::string &invokeId, const std::string &originType) {
    // §scxml-5.10: Get event type from EventRaiser thread-local storage (test 331)
    const std::string eventType = EventRaiserImpl::getCurrentEventType();
    const bool fromExternalQueue = EventRaiserImpl::isCurrentEventFromExternalQueue();

    if (!isRunning_) {
        return refuseUnseen(eventName);
    }

    // Check JS environment
    if (!jsEnvironmentReady_) {
        SCE_LOG_ERROR("StateMachine: Cannot process event - JavaScript environment not ready");
        TransitionResult result;
        result.success = false;
        result.errorMessage = "JavaScript environment not ready";
        return result;
    }

    EventMetadata event(eventName, eventData, eventType, sendId, invokeId, originType, originSessionId);
    // §scxml-5.10: Carry typed event data from EventRaiser thread-local (avoids JSON round-trip)
    event.typedData = EventRaiserImpl::getCurrentTypedData();

    if (macrostepInProgressOnThisThread()) {
        // §scxml-D-mainEventLoop: an event is processed only where the main
        // event loop takes it. One handed to this machine from inside its own
        // running macrostep — by a raiser that delivers synchronously, a host
        // callback bound into the datamodel, an autoforward from a parent this
        // macrostep called into — goes onto the queue it belongs to.
        return holdDelivery(event, fromExternalQueue);
    }

    // Serialize against a macrostep another thread is running, and against
    // destruction.
    std::lock_guard<std::mutex> processEventLock(processEventMutex_);
    if (!isRunning_) {
        // It stopped while this call waited for the other thread's macrostep.
        return refuseUnseen(eventName);
    }

    MacrostepScope macrostep(*this);

    // §scxml-3.12.2: a call from the host starts a new piece of work, so any
    // `error.*` chain the last one built is over.
    if (eventRaiser_) {
        eventRaiser_->resetErrorCascadeDepth();
    }
    // The same boundary bounds the macrostep's own chain: the algorithm
    // starts a macrostep at the external dequeue, and for this engine the
    // host's call is that dequeue — so a machine left inside an endless
    // chain gets a full budget for each event it is given, and each refusal is
    // counted separately. It is also the only way a chain refused for leaving
    // events queued ever gets drained: the events are still there, and this is
    // where the budget that drains them comes back.
    macrostepTruncated_ = false;
    macrostepMicrostepsTaken_ = 0;

    // §scxml-D-microstepProcedure: what the visualizer shows belongs to this event
    lastEnabledTransitions_.clear();
    lastOptimalTransitions_.clear();

    SCE_LOG_DEBUG("StateMachine: Processing event: '{}' with data: '{}' in session: '{}', originSessionId: '{}'",
                  eventName, eventData, sessionId_, originSessionId);

    const TransitionResult result = processTakenEvent(event, fromExternalQueue);
    runMainEventLoop();
    if (topLevelFinalReached_) {
        finishAtTopLevelFinal();
    }

    updateStatistics();
    return result;
}

StateMachine::TransitionResult StateMachine::refuseUnseen(const std::string &eventName) {
    SCE_LOG_WARN("StateMachine: Cannot process event - state machine not running");
    // §scxml-3.13: this engine has always told the CALLER — `success` is
    // false and the message names the reason. The count is added because
    // the six generated engines have no return value to carry it, so a
    // host that polls statistics reads the same fact on every backend.
    // See `Statistics::unseenExternalEvents`.
    noteUnseenEvent(eventName);
    TransitionResult result;
    result.success = false;
    result.errorMessage = "State machine not running";
    return result;
}

void StateMachine::noteUnseenEvent(const std::string &eventName) {
    ++unseenExternalEvents_;
    lastUnseenEventName_ = eventName;
}

StateMachine::TransitionResult StateMachine::holdDelivery(const EventMetadata &event, bool fromExternalQueue) {
    TransitionResult result;
    result.success = false;
    result.fromState = getCurrentState();
    result.eventName = event.name;

    // §scxml-D-mainEventLoop: onto the tail of the queue it belongs to, in
    // order with everything already waiting there. Holding it anywhere else
    // would be a third queue the loop takes from in some order of its own.
    // The raiser is asked to queue, never to dispatch: dispatching would hand
    // the event straight back into the macrostep it is waiting for.
    const EventQueue queue = fromExternalQueue ? EventQueue::External : EventQueue::Internal;
    if (eventRaiser_ && eventRaiser_->enqueue(event, queue)) {
        result.errorMessage = "Held: a macrostep of this machine is in progress";
        return result;
    }

    // No queue to put it on: the raiser is gone or shut down. The event is
    // refused, and counted the way any event this machine never looks at is.
    SCE_LOG_WARN("StateMachine: '{}' arrived during a macrostep and there is no event queue to hold it", event.name);
    noteUnseenEvent(event.name);
    result.errorMessage = "Refused: a macrostep of this machine is in progress and no event queue can hold the event";
    return result;
}

StateMachine::TransitionResult StateMachine::processTakenEvent(const EventMetadata &event, bool fromExternalQueue) {
    stats_.totalEvents++;
    currentOriginSessionId_ = event.originSessionId;

    // W3C SCXML Test 252: an invoked session that was cancelled generates no
    // more events for its parent
    if (invokeExecutor_ && !event.originSessionId.empty() &&
        invokeExecutor_->shouldFilterCancelledInvokeEvent(event.originSessionId)) {
        SCE_LOG_DEBUG("StateMachine: Filtering event '{}' from cancelled invoke child session: {}", event.name,
                      event.originSessionId);
        return TransitionResult(false, getCurrentState(), getCurrentState(), event.name);
    }

    // §scxml-D-mainEventLoop: datamodel["_event"] = the event just taken, and
    // it stays bound until the loop takes the next one.
    bindCurrentEvent(event);

    // §scxml-D-mainEventLoop: an external event's preliminary step — the
    // `<finalize>` of the invoke it came from, then the autoforward copy —
    // runs before transitions are selected for it.
    if (!event.originSessionId.empty()) {
        applyFinalize(event.originSessionId, event.name);
    }
    if (fromExternalQueue) {
        autoforward(event);
    }

    const std::vector<Transition> enabled = selectTransitions(event.name);
    if (enabled.empty()) {
        SCE_LOG_DEBUG("StateMachine: No transition enabled by event '{}'", event.name);
        stats_.failedTransitions++;
        TransitionResult result;
        result.success = false;
        result.fromState = getCurrentState();
        result.eventName = event.name;
        result.errorMessage = "No valid transitions found";
        return result;
    }

    // What a caller is told about the event: the first transition it
    // selected, from its source to its first target as written.
    const Transition &first = enabled.front();
    std::string firstTarget = first.source;
    if (!first.targets.empty()) {
        firstTarget =
            first.targets.front().isHistory() ? first.targets.front().history() : first.targets.front().state();
    }
    TransitionResult result(true, first.source, firstTarget, event.name);

    takeMicrostep(enabled, event.name);
    return result;
}

void StateMachine::bindCurrentEvent(const EventMetadata &event) {
    currentEventData_ = event.data;
    if (!cachedExecutorImpl_) {
        return;
    }
    cachedExecutorImpl_->setCurrentEvent(event);
    // §scxml-B-2-8-1: the binding just chose a rung, and this is the only
    // moment that knows which event it belonged to.
    if (cachedExecutorImpl_->lastPayloadReading() == PayloadReading::Undecodable) {
        ++undecodablePayloads_;
        lastUndecodablePayloadEvent_ = event.name;
    }
}

void StateMachine::applyFinalize(const std::string &originSessionId, const std::string &eventName) {
    // W3C SCXML 1.0 Section 6.4: Execute finalize handler before processing events from invoked children
    // According to W3C SCXML: "finalize markup runs BEFORE the event is processed"
    // The finalize handler is executed when an event arrives from an invoked child
    // and has access to _event.data to update parent variables before transition evaluation
    if (!invokeExecutor_) {
        return;
    }

    // W3C SCXML compliance: Use originSessionId to find the exact child that sent this event
    std::string finalizeScript = invokeExecutor_->getFinalizeScriptForChildSession(originSessionId);
    if (finalizeScript.empty()) {
        return;
    }

    SCE_LOG_DEBUG("StateMachine: Executing finalize handler BEFORE processing event '{}', script: '{}'", eventName,
                  finalizeScript);

    // §scxml-6.5.2: Parse and execute finalize as SCXML executable content
    // Finalize contains elements like <assign>, <script>, <log>, <raise>, <if>, <foreach> etc.
    if (!actionExecutor_) {
        SCE_LOG_WARN("StateMachine: No ActionExecutor available for finalize execution");
        return;
    }
    try {
        // Parse finalize XML content using IXMLParser
        std::string xmlWrapper =
            "<finalize xmlns=\"http://www.w3.org/2005/07/scxml\">" + finalizeScript + "</finalize>";

        // §wire-W4 D1-C: parseContent throws
        // `SCE::parsing::ParseXmlFailed` on malformed input;
        // the outer `catch (std::exception&)` arm catches
        // the typed leaf via base-class slicing.
        auto parser = IXMLParser::create();
        auto document = parser->parseContent(xmlWrapper);

        auto root = document->getRootElement();
        if (!root) {
            SCE_LOG_ERROR("StateMachine: No root element in finalize XML");
            return;
        }

        // Use ActionParser to parse and execute each action in finalize
        ActionParser actionParser(nullptr);
        auto children = root->getChildren();

        // Create execution context
        auto sharedExecutor = std::static_pointer_cast<IActionExecutor>(actionExecutor_);
        ExecutionContextImpl context(sharedExecutor, sessionId_);

        // Execute each action in finalize
        for (const auto &child : children) {
            auto action = actionParser.parseActionNode(child);
            if (action) {
                bool success = action->execute(context);
                SCE_LOG_DEBUG("StateMachine: Finalize action '{}' executed: {}", child->getName(), success);
            }
        }

        SCE_LOG_DEBUG("StateMachine: Finalize handler executed successfully for event '{}'", eventName);
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("StateMachine: Exception during finalize handler execution: {}", e.what());
    }
}

void StateMachine::autoforward(const EventMetadata &event) {
    // §scxml-D-mainEventLoop: auto-forward to child invoke sessions at the
    // one point the algorithm names — immediately after an event is removed
    // from the *external* queue, before transition selection:
    //
    //     externalEvent = externalQueue.dequeue()
    //     if isCancelEvent(externalEvent): running = false; continue
    //     for state in configuration: for inv in state.invoke:
    //         if inv.autoforward: send(inv.id, externalEvent)
    //
    // The loop consults no event name, and 6.4.2 requires an exact copy of
    // "every external event it receives" — including `done.invoke.<id>`,
    // which the same section returns "to the external event queue of the
    // invoking process", so a sibling invoke still
    // running must see it. `error.*` and `done.state.*` stay out of the
    // forwarded set because they are raised onto the internal queue and
    // never reach this point, not because of how they are spelled: this
    // function serves both queues, so the queue category is carried on the
    // dispatch context rather than re-derived from the name.
    //
    // W3C Test 230: events from child sessions ARE autoforwarded back, to
    // verify field preservation.
    // Use shared_ptr to prevent use-after-free if child reaches final state during processEvent
    if (!invokeExecutor_) {
        return;
    }
    auto autoForwardSessions = invokeExecutor_->getAutoForwardSessions(sessionId_);
    if (autoForwardSessions.empty()) {
        return;
    }

    // The copy a child receives is an external event of its own, with this
    // event's metadata — the context a raiser would have established had it
    // delivered the copy.
    EventRaiserImpl::EventContext forwarded;
    forwarded.originSessionId = event.originSessionId;
    forwarded.sendId = event.sendId;
    forwarded.invokeId = event.invokeId;
    forwarded.originType = event.originType;
    forwarded.eventType = event.type;
    forwarded.typedData = event.typedData;
    forwarded.isExternalQueue = true;
    EventRaiserImpl::EventContextGuard forwardedGuard(std::move(forwarded));

    for (const auto &childStateMachine : autoForwardSessions) {
        if (childStateMachine && childStateMachine->isRunning()) {
            SCE_LOG_DEBUG("W3C SCXML 6.4: Auto-forwarding event '{}' to child session", event.name);
            childStateMachine->processEvent(event.name, event.data, event.originSessionId, event.sendId, event.invokeId,
                                            event.originType);
        }
    }
}

void StateMachine::enterInitialConfiguration() {
    // §scxml-D-interpret: enterStates([doc.initialTransition]) — the
    // document's own initial transition, whose source is the <scxml> element
    // and whose domain is therefore the whole document. A target set, a deep
    // initial, an initial naming a <history>: the entry procedures answer all
    // of them, so nothing here resolves a leaf first.
    MicrostepHost host{*this};
    Core::MicrostepAlgorithms::enterStates(host, {Core::EntryTransition<std::string, std::string>{
                                                     std::nullopt, document_->documentInitialTargets(), false}});
}

void StateMachine::runMainEventLoop() {
    // §scxml-D-mainEventLoop: one macrostep after another. Each completes on
    // eventless transitions and internal events alone — an eventless
    // selection first, and only when it finds nothing the next internal
    // event. Then the invokes for the states the macrostep entered run, and
    // invoking may raise internal events, which the macrostep takes before it
    // is done. Only then is the next external event taken, and it opens the
    // next macrostep: a state one event enters has its invokes started before
    // the next event comes off the queue.
    while (isRunning_) {
        // A refusal ends the macrostep outright, whatever is left queued: the
        // refused chain is what would be taken next.
        bool refused = false;
        while (isRunning_ && !macrostepTruncated_) {
            const std::vector<Transition> enabled = selectTransitions(std::string{});
            if (!enabled.empty()) {
                if (macrostepMicrostepsTaken_ >= static_cast<uint32_t>(MAX_MACROSTEP_MICROSTEPS)) {
                    // The chain is still going one microstep past the budget.
                    // The microstep is refused rather than taken — selection
                    // moves nothing, so refusing it is exact.
                    recordTruncatedMacrostep();
                    refused = true;
                    break;
                }
                ++macrostepMicrostepsTaken_;
                takeMicrostep(enabled, std::string{});
                continue;
            }

            // Interactive mode: the host steps internal events one at a time.
            if (!autoProcessQueuedEvents_ || !eventRaiser_ || !eventRaiser_->hasQueuedInternalEvents()) {
                break;
            }
            if (!mayTakeMicrostep()) {
                // Asked before the event leaves the queue, so a refusal leaves
                // it for the next macrostep rather than swallowing it.
                refused = true;
                break;
            }
            const std::optional<EventMetadata> internalEvent = eventRaiser_->takeQueuedEvent(EventQueue::Internal);
            if (!internalEvent) {
                break;
            }
            eventRaiser_->dispatchTaken(internalEvent->name, [this, &internalEvent] {
                const TransitionResult result = processTakenEvent(*internalEvent, /*fromExternalQueue=*/false);
                if (result.success) {
                    // A turn that selects nothing takes no microstep, so it
                    // spends no budget.
                    ++macrostepMicrostepsTaken_;
                }
                return result.success;
            });
        }

        if (!isRunning_) {
            break;
        }

        // §scxml-6.4: the invokes of the states entered during this macrostep.
        // A macrostep stopped at its ceiling entered states too, and the next
        // one starts from them.
        executePendingInvokes();

        // Invoking may have raised internal events, and a child that finished
        // at once may already have reported: the macrostep takes them before
        // it is done. Not after a refusal — the queue holds what the refusal
        // left there, and going back for it is a turn that takes nothing.
        if (!refused && !macrostepTruncated_ && autoProcessQueuedEvents_ && eventRaiser_ &&
            eventRaiser_->hasQueuedInternalEvents()) {
            continue;
        }

        // Interactive mode: the host steps the external queue itself.
        if (!autoProcessQueuedEvents_) {
            break;
        }

        // The outer loop. A refused chain does not hold the external queue
        // back: the next external event opens a macrostep with a budget of its
        // own, and it is often the very event that gets the machine out.
        if (!takeNextExternalEvent()) {
            break;
        }
    }
}

bool StateMachine::takeNextExternalEvent() {
    if (!eventRaiser_) {
        return false;
    }
    const std::optional<EventMetadata> externalEvent = eventRaiser_->takeQueuedEvent(EventQueue::External);
    if (!externalEvent) {
        return false;
    }
    // §scxml-D-mainEventLoop: taking an event off the external queue is where
    // a macrostep begins, so it is where the previous one's ceiling stops
    // applying — a machine inside an endless chain gets a whole budget for
    // each event it is given, and each refusal is counted on its own.
    macrostepTruncated_ = false;
    macrostepMicrostepsTaken_ = 0;
    eventRaiser_->dispatchTaken(externalEvent->name, [this, &externalEvent] {
        return processTakenEvent(*externalEvent, /*fromExternalQueue=*/true).success;
    });
    return true;
}

std::vector<StateMachine::Transition> StateMachine::selectTransitions(const std::string &eventName) {
    selectionCandidates_.clear();
    MicrostepHost host{*this};
    return Core::MicrostepAlgorithms::selectTransitions(host, eventName);
}

std::optional<StateMachine::Transition> StateMachine::firstEnabledTransition(const std::string &state,
                                                                             const std::string &eventName) {
    IStateNode *node = document_->nodeOf(state);
    if (!node) {
        return std::nullopt;
    }

    const auto &transitions = node->getTransitions();
    for (size_t index = 0; index < transitions.size(); ++index) {
        const auto &transitionNode = transitions[index];
        const std::vector<std::string> &descriptors = transitionNode->getEvents();

        // §scxml-3.12: a transition matches an event when any of its
        // descriptors does; an eventless selection considers only
        // transitions that name no event at all.
        const bool eventMatches = eventName.empty()
                                      ? descriptors.empty()
                                      : SCE::Core::TransitionHelper::matchesAnyEventDescriptor(descriptors, eventName);
        if (!eventMatches) {
            continue;
        }

        const std::string &condition = transitionNode->getGuard();
        if (!condition.empty() && !evaluateCondition(condition)) {
            continue;
        }

        Transition enabled;
        enabled.source = state;
        enabled.targets = document_->targetsOf(transitionNode->getTargets());
        enabled.transitionIndex = static_cast<int>(index);
        enabled.hasActions = !transitionNode->getActionNodes().empty();
        enabled.isInternal = transitionNode->isInternal();

        const bool seen =
            std::any_of(selectionCandidates_.begin(), selectionCandidates_.end(), [&enabled](const Transition &t) {
                return t.source == enabled.source && t.transitionIndex == enabled.transitionIndex;
            });
        if (!seen) {
            selectionCandidates_.push_back(enabled);
        }
        return enabled;
    }
    return std::nullopt;
}

void StateMachine::takeMicrostep(const std::vector<Transition> &transitions, const std::string &eventName) {
    // What the interactive visualizer shows about this microstep: everything
    // the selection found and what survived preemption, read off the
    // configuration the transitions are about to exit.
    const std::vector<std::string> configuration = hierarchyManager_->getActiveStates();
    lastEnabledTransitions_.clear();
    for (const auto &candidate : selectionCandidates_) {
        lastEnabledTransitions_.push_back(describe(candidate, eventName, configuration));
    }
    lastOptimalTransitions_.clear();
    for (const auto &transition : transitions) {
        lastOptimalTransitions_.push_back(describe(transition, eventName, configuration));
    }
    const Transition &last = transitions.back();
    lastTransitionSource_ = last.source;
    lastTransitionTarget_.clear();
    if (!last.targets.empty()) {
        lastTransitionTarget_ =
            last.targets.front().isHistory() ? last.targets.front().history() : last.targets.front().state();
    }
    lastTransitionIndex_ = last.transitionIndex;
    stats_.totalTransitions += static_cast<int>(transitions.size());

    MicrostepHost host{*this};
    Core::MicrostepAlgorithms::microstep(host, transitions);
    updateStatistics();
}

TransitionDescriptorString StateMachine::describe(const Transition &transition, const std::string &eventName,
                                                  const std::vector<std::string> &configuration) {
    TransitionDescriptorString descriptor;
    descriptor.source = transition.source;
    if (!transition.targets.empty()) {
        descriptor.target = transition.targets.front().isHistory() ? transition.targets.front().history()
                                                                   : transition.targets.front().state();
    }
    descriptor.event = eventName;
    MicrostepHost host{*this};
    descriptor.exitSet = Core::ExitSetAlgorithms::computeExitSet(
        transition.source, Core::MicrostepAlgorithms::effectiveTargets(host, transition), transition.isInternal,
        transition.isTargetless(), configuration, [this](const std::string &s) { return document_->parentOf(s); },
        [this](const std::string &s) { return document_->isCompound(s); });
    descriptor.transitionIndex = transition.transitionIndex;
    descriptor.hasActions = transition.hasActions;
    descriptor.isInternal = transition.isInternal;
    descriptor.isTargetless = transition.isTargetless();
    // The transition tears a `<parallel>` down: one is among what it exits.
    descriptor.isExternal = std::any_of(descriptor.exitSet.begin(), descriptor.exitSet.end(),
                                        [this](const std::string &s) { return document_->isParallel(s); });
    return descriptor;
}

void StateMachine::exitStateInMicrostep(const std::string &state,
                                        const std::vector<std::string> &configurationBeforeExit) {
    // §scxml-D-exitStates: every `<history>` of an exited state records from
    // the configuration as it stood before the microstep's first exit, so a
    // descendant already exited in this microstep is still in what it
    // records.
    IStateNode *node = document_->nodeOf(state);
    if (historyManager_ && node) {
        const auto &children = node->getChildren();
        const bool hasHistory = std::any_of(children.begin(), children.end(), [](const auto &child) {
            return child && child->getType() == Type::HISTORY;
        });
        if (hasHistory) {
            historyManager_->recordHistory(state, configurationBeforeExit);
        }
    }
    exitState(state);
}

void StateMachine::exitState(const std::string &state) {
    // §scxml-D-exitStates: the state's onexit content, then cancel the
    // invocations it started, then it leaves the configuration.
    executeExitActions(state);
    cancelInvokesOf(state);
    hierarchyManager_->removeStateFromConfiguration(state);
}

void StateMachine::cancelInvokesOf(const std::string &state) {
    IStateNode *node = document_ ? document_->nodeOf(state) : nullptr;
    if (!node || !invokeExecutor_) {
        return;
    }
    for (const auto &invoke : node->getInvoke()) {
        const std::string &invokeid = invoke->getId();
        if (invokeid.empty()) {
            SCE_LOG_WARN("StateMachine::cancelInvokesOf - Found invoke with empty ID in state '{}'", state);
            continue;
        }
        if (invokeExecutor_->isInvokeActive(invokeid)) {
            SCE_LOG_DEBUG("StateMachine: Cancelling active invoke '{}' due to state exit: {}", invokeid, state);
            invokeExecutor_->cancelInvoke(invokeid);
        } else {
            SCE_LOG_DEBUG("StateMachine: NOT cancelling inactive invoke '{}' (may be completing naturally)", invokeid);
        }
    }
}

void StateMachine::enterStateInMicrostep(const std::string &state, bool isDefaultEntry) {
    // §scxml-D-enterStates: the state joins the configuration and the states
    // whose invokes run once the macrostep is complete; under late binding
    // its data is initialized on first entry; then its onentry content, and
    // its initial transition's content if and only if it is entered by
    // default.
    hierarchyManager_->addStateToConfiguration(state);
    IStateNode *node = document_->nodeOf(state);
    if (!node) {
        SCE_LOG_ERROR("StateMachine: entered state '{}' is not in the document", state);
        return;
    }

    deferInvokeExecution(state, node->getInvoke());

    if (dataModelInit_) {
        dataModelInit_->initializeStateDataOnEntry(state);
    }

    executeOnEntryActions(state);

    if (isDefaultEntry) {
        if (const auto initialTransition = node->getInitialTransition()) {
            executeActionNodes(initialTransition->getActionNodes());
        }
    }

    if (node->getType() == Type::FINAL) {
        enterFinalState(state);
    }
}

void StateMachine::enterFinalState(const std::string &finalState) {
    const auto parent = document_->parentOf(finalState);
    if (!parent) {
        // §scxml-D-enterStates: a `<final>` child of `<scxml>` ends the
        // interpretation. Its donedata is what a parent session's
        // done.invoke carries (§scxml-6.4.3), stashed here for the
        // completion callback that runs once the microstep is over.
        pendingDonedataAtFinal_.clear();
        pendingTypedDonedataAtFinal_.reset();
        (void)evaluateDoneData(finalState, pendingDonedataAtFinal_, pendingTypedDonedataAtFinal_);
        topLevelFinalReached_ = true;
        isRunning_ = false;
        SCE_LOG_INFO("StateMachine: Reached top-level final state: {}, halting processing", finalState);
        return;
    }

    // §scxml-D-enterStates: done.state.<parent>, carrying the `<final>`'s
    // donedata; and when the grandparent is a `<parallel>` every region of
    // which is now in a final state, done.state.<grandparent>. A structural
    // error in the donedata raises error.execution and neither event, the
    // same answer the generated engines give.
    std::string eventData;
    std::optional<ScriptValue> typedData;
    if (!evaluateDoneData(finalState, eventData, typedData)) {
        SCE_LOG_DEBUG("W3C SCXML 5.7: Donedata evaluation failed, skipping done.state event generation");
        return;
    }
    raiseInternal("done.state." + *parent, eventData, std::move(typedData));

    const auto grandparent = document_->parentOf(*parent);
    if (grandparent && document_->isParallel(*grandparent) && stateIsInFinalState(*grandparent)) {
        raiseInternal("done.state." + *grandparent, "", std::nullopt);
    }
}

void StateMachine::raiseInternal(const std::string &eventName, const std::string &eventData,
                                 std::optional<ScriptValue> typedData) {
    if (!eventRaiser_) {
        SCE_LOG_WARN("StateMachine: Cannot queue {} - no event raiser", eventName);
        return;
    }
    // §scxml-5.5: typed data rides the engine-agnostic ScriptValue pipeline
    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
    if (eventRaiserImpl && typedData.has_value()) {
        eventRaiserImpl->raiseEventWithPriority(eventName, eventData, EventRaiserImpl::EventPriority::INTERNAL, "", "",
                                                "", "", 0, std::move(typedData));
    } else {
        eventRaiser_->raiseEvent(eventName, eventData);
    }
    SCE_LOG_DEBUG("W3C SCXML: Queued {}", eventName);
}

void StateMachine::executeTransitionContent(const Transition &transition) {
    IStateNode *node = document_->nodeOf(transition.source);
    if (!node) {
        return;
    }
    const auto &transitions = node->getTransitions();
    if (transition.transitionIndex < 0 || static_cast<size_t>(transition.transitionIndex) >= transitions.size()) {
        return;
    }
    executeActionNodes(transitions[static_cast<size_t>(transition.transitionIndex)]->getActionNodes());
}

void StateMachine::executeHistoryDefaultContent(const std::string &history) {
    IStateNode *node = document_->nodeOf(history);
    if (!node || node->getTransitions().empty()) {
        return;
    }
    executeActionNodes(node->getTransitions().front()->getActionNodes());
}

bool StateMachine::stateIsInFinalState(const std::string &state) const {
    const std::vector<std::string> configuration =
        hierarchyManager_ ? hierarchyManager_->getActiveStates() : std::vector<std::string>{};
    return Core::CompletionAlgorithms::isInFinalState(
        state, configuration, [this](const std::string &s) { return document_->parentOf(s); },
        [this](const std::string &s) { return document_->isParallel(s); },
        [this](const std::string &s) { return document_->childStates(s); },
        [this](const std::string &s) { return document_->isFinal(s); });
}

std::vector<std::string> StateMachine::configurationInExitOrder() const {
    std::vector<std::string> states =
        hierarchyManager_ ? hierarchyManager_->getActiveStates() : std::vector<std::string>{};
    if (!document_) {
        return states;
    }
    // Exit order: descendants before their ancestors, reverse document order
    // among the rest — together, exactly reverse document order.
    std::sort(states.begin(), states.end(), [this](const std::string &a, const std::string &b) {
        return document_->documentOrder(a) > document_->documentOrder(b);
    });
    return states;
}

void StateMachine::finishAtTopLevelFinal() {
    topLevelFinalReached_ = false;
    // §scxml-D-exitInterpreter: the machine has entered a top-level `<final>`,
    // so the interpretation is over — every active state's onexit runs and
    // its invocations are cancelled, then the parent session is told
    // (§scxml-6.4.3: done.invoke only after the onexit handlers). The
    // configuration itself is kept: it is what a host reads to learn where
    // the machine stopped.
    for (const auto &state : configurationInExitOrder()) {
        if (!executeExitActions(state)) {
            SCE_LOG_WARN("StateMachine: Failed to execute onexit for final state: {}", state);
        }
        cancelInvokesOf(state);
    }
    recordUnseenQueuedEvents();

    if (completionCallback_) {
        try {
            completionCallback_();
        } catch (const std::exception &e) {
            SCE_LOG_ERROR("StateMachine: Exception in completion callback: {}", e.what());
        }
    }
}

void StateMachine::recordUnseenQueuedEvents() {
    // §scxml-D-exitInterpreter: the loop that would have taken these has
    // ended. An external event still queued was never looked at, which is
    // what `Statistics::unseenExternalEvents` counts; an internal one has both
    // ends inside this document, and ends with it. Emptied rather than left:
    // a host that went on pumping this raiser would hand each one to a machine
    // that refuses it, and it would be counted again — as an external event,
    // whichever queue it was on.
    if (!eventRaiser_) {
        return;
    }
    while (const std::optional<EventMetadata> unseen = eventRaiser_->takeQueuedEvent(EventQueue::External)) {
        noteUnseenEvent(unseen->name);
    }
    while (eventRaiser_->takeQueuedEvent(EventQueue::Internal)) {
        // Nothing to report: the document raised it, and the document is done.
    }
}

std::string StateMachine::getCurrentState() const {
    // W3C SCXML: Thread safety for JSEngine worker thread access
    std::lock_guard<std::mutex> lock(hierarchyManagerMutex_);

    if (!hierarchyManager_) {
        return "";
    }

    return hierarchyManager_->getCurrentState();
}

std::vector<std::string> StateMachine::getActiveStates() const {
    // W3C SCXML: Thread safety for JSEngine worker thread access
    std::lock_guard<std::mutex> lock(hierarchyManagerMutex_);

    if (!hierarchyManager_) {
        SCE_LOG_WARN("StateMachine::getActiveStates: hierarchyManager is null!");
        return {};
    }

    return hierarchyManager_->getActiveStates();
}

bool StateMachine::isRunning() const {
    return isRunning_;
}

bool StateMachine::isStateActive(const std::string &stateId) const {
    // W3C SCXML: Thread safety for JSEngine worker thread access
    std::lock_guard<std::mutex> lock(hierarchyManagerMutex_);

    if (!hierarchyManager_) {
        return false;
    }
    return hierarchyManager_->isStateActive(stateId);
}

bool StateMachine::isStateInFinalState(const std::string &stateId) const {
    if (!model_) {
        SCE_LOG_DEBUG("StateMachine::isStateInFinalState: No model available");
        return false;
    }

    if (stateId.empty()) {
        SCE_LOG_DEBUG("StateMachine::isStateInFinalState: State ID is empty");
        return false;
    }

    auto state = model_->findStateById(stateId);
    bool isFinal = state && state->isFinalState();
    SCE_LOG_DEBUG("StateMachine::isStateInFinalState: stateId='{}', state found: {}, isFinalState: {}", stateId,
                  (void *)state, isFinal);
    return isFinal;
}

bool StateMachine::isInFinalState() const {
    if (!isRunning_) {
        SCE_LOG_DEBUG("StateMachine::isInFinalState: State machine is not running");
        return false;
    }

    // §scxml-3.7: Check only top-level final states, not child final states of compound states
    // Bug fix: Compound state children (e.g., s02 in test294) should NOT make the machine final
    // §scxml-3.4 & test570: Complete parallel states should NOT halt execution - only <final/> elements
    auto activeStates = hierarchyManager_->getActiveStates();
    for (const auto &stateId : activeStates) {
        auto state = model_->findStateById(stateId);
        if (!state) {
            continue;
        }

        // §scxml-3.4 & 3.7: only a top-level <final> element halts execution. A
        // <parallel> whose regions have all completed has raised done.state, and
        // that event still has to be processed (test 570).
        if (state->getType() == Type::FINAL && !state->getParent()) {
            SCE_LOG_DEBUG("StateMachine::isInFinalState: Found top-level final state '{}'", stateId);
            return true;
        }
    }

    SCE_LOG_DEBUG("StateMachine::isInFinalState: No top-level final states active");
    return false;
}

int StateMachine::getLastTransitionIndex() const {
    return lastTransitionIndex_;
}

std::string StateMachine::getLastTransitionSource() const {
    return lastTransitionSource_;
}

std::string StateMachine::getLastTransitionTarget() const {
    return lastTransitionTarget_;
}

std::vector<TransitionDescriptorString> StateMachine::getLastEnabledTransitions() const {
    return lastEnabledTransitions_;
}

std::vector<TransitionDescriptorString> StateMachine::getLastOptimalTransitions() const {
    return lastOptimalTransitions_;
}

bool StateMachine::restoreFromSnapshot(const std::vector<std::string> &states, bool running) {
    // Complete state machine restoration for time-travel debugging
    // ARCHITECTURE.md: Template Method pattern - encapsulates restoration lifecycle
    // to prevent temporal coupling and maintain Single Source of Truth

    std::string statesStr;
    for (const auto &s : states) {
        if (!statesStr.empty()) {
            statesStr += ", ";
        }
        statesStr += s;
    }
    SCE_LOG_DEBUG("StateMachine::restoreFromSnapshot: Starting - states: [{}], session: {}", statesStr, sessionId_);

    // Step 1: Ensure JavaScript environment is initialized
    // CRITICAL: JS environment must exist BEFORE state restoration for event processing (Test 192)
    if (!ensureJSEnvironment()) {
        SCE_LOG_ERROR("StateMachine::restoreFromSnapshot: Failed to initialize JS environment for session {}",
                      sessionId_);
        return false;
    }

    // Step 2: Restore state configuration (delegates to internal method)
    // This sets isRunning_ to the recorded flag internally
    restoreActiveStatesDirectly(states, running);

    // Verify restoration
    auto restoredStates = getActiveStates();
    std::string restoredStr;
    for (const auto &s : restoredStates) {
        if (!restoredStr.empty()) {
            restoredStr += ", ";
        }
        restoredStr += s;
    }
    SCE_LOG_DEBUG("StateMachine::restoreFromSnapshot: Complete - restored: [{}], running: {}", restoredStr,
                  isRunning_.load());

    return true;
}

void StateMachine::restoreActiveStatesDirectly(const std::vector<std::string> &states, bool running) {
    // Time-travel debugging - restore configuration without side effects: the
    // states are written into the configuration and no <onentry> runs.
    // INTERNAL USE ONLY: Called by restoreFromSnapshot() after JS environment initialization

    SCE_LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: Called with {} states", states.size());

    // Mutex scope: Release before getActiveStates() verification to prevent deadlock
    {
        // Thread safety: Lock mutex for hierarchyManager access
        std::lock_guard<std::mutex> lock(hierarchyManagerMutex_);

        if (!hierarchyManager_) {
            SCE_LOG_ERROR("StateMachine::restoreActiveStatesDirectly: hierarchyManager_ is null");
            return;
        }

        // Clear current configuration first
        SCE_LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: Calling hierarchyManager_->reset()");
        hierarchyManager_->reset();

        // Restore states in document order (already provided by vector)
        // No sorting needed - vector from snapshot preserves correct document order (Test 570 fix)
        for (const auto &stateId : states) {
            hierarchyManager_->addStateToConfiguration(stateId);
            SCE_LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: Added state '{}' to configuration", stateId);
        }

        // The running flag is the one the step recorded. A child that was
        // running then must come back running to receive its parent's events
        // (Test 192). One whose session had ended must come back ended: when
        // its parent leaves the invoking state, the cancel that follows
        // (§scxml-6.4) purges from the parent's queue what a running session
        // sent (W3C test 252) and keeps what an ended one sent, its
        // done.invoke among them (W3C test 236). Revived as running, an ended
        // child lost its done.invoke to that cancel in a branch taken from
        // the restored step.
        isRunning_ = running;
        SCE_LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: running restored as {}, mutex will release", running);

        // The configuration is the whole of the run state: a `<parallel>`'s
        // regions are its child states in that configuration, read by the
        // shared microstep like any other, so there is nothing further to
        // bring into line with the restored set.
    }  // Mutex released here

    // Verify final state (outside mutex scope to prevent deadlock with getActiveStates())
    SCE_LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: [BEFORE getActiveStates()] Calling getActiveStates()");
    auto finalStates = getActiveStates();
    SCE_LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: [AFTER getActiveStates()] Returned {} states",
                  finalStates.size());
    std::string finalStr;
    for (const auto &s : finalStates) {
        if (!finalStr.empty()) {
            finalStr += ", ";
        }
        finalStr += s;
    }
    SCE_LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: Complete - final states: [{}], running: {}", finalStr,
                  running);
}

bool StateMachine::isInitialStateFinal() const {
    return isStateInFinalState(model_ ? model_->getInitialState() : "");
}

std::string StateMachine::getCurrentEventData() const {
    return currentEventData_;
}

const std::string &StateMachine::getSessionId() const {
    return sessionId_;
}

std::shared_ptr<SCXMLModel> StateMachine::getModel() const {
    return model_;
}

StateMachine::Statistics StateMachine::getStatistics() const {
    Statistics stats = stats_;
    // §scxml-3.12.2: the raiser owns the queue an error chain feeds, so it is
    // where the chain is cut and counted; this is where a host already looks.
    if (eventRaiser_) {
        stats.errorCascadeEvents = eventRaiser_->getErrorCascadeEvents();
        stats.lastErrorCascadeEvent = eventRaiser_->getLastErrorCascadeEvent();
    }
    // The eventless chain is this class's own loop, so unlike the
    // error chain above there is no raiser to ask — the count lives here and
    // is the only reading that separates a machine resting in a stable
    // configuration from one this engine stopped walking.
    stats.truncatedMacrosteps = truncatedMacrosteps_;
    stats.lastTruncatedMacrostepState = lastTruncatedMacrostepState_;
    // §scxml-B-2-8-1: counted here rather than in the executor because the
    // executor binds one event at a time and has nowhere to keep a tally,
    // while this is the object a host holds. The executor answers which rung
    // the LAST binding took; `bindCurrentEvent` turns that into a count.
    stats.undecodablePayloads = undecodablePayloads_;
    stats.lastUndecodablePayloadEvent = lastUndecodablePayloadEvent_;
    // §scxml-3.13: events handed to a machine that had already stopped. This
    // engine also refuses them through `TransitionResult::success`; the count
    // is what the generated engines can offer, and parity is what makes a
    // document portable between them.
    stats.unseenExternalEvents = unseenExternalEvents_;
    stats.lastUnseenEventName = lastUnseenEventName_;
    return stats;
}

bool StateMachine::initializeFromModel() {
    SCE_LOG_DEBUG("StateMachine: Initializing from SCXML model");

    // Extract all states from the model
    const auto &allStates = model_->getAllStates();
    if (allStates.empty()) {
        SCE_LOG_ERROR("StateMachine: No states found in SCXML model");
        return false;
    }

    try {
        // The document as Appendix D reads it: document order, child states,
        // initial targets and history defaults, answered once.
        document_ = std::make_unique<InterpreterDocument>(*model_);
        if (document_->documentInitialTargets().empty()) {
            SCE_LOG_ERROR("StateMachine: The document names no initial state and holds no state to default to");
            return false;
        }

        // The configuration
        hierarchyManager_ = std::make_unique<StateHierarchyManager>(model_);

        // SCXML W3C Section 3.6: Auto-register history states from parsed model (SOLID architecture)
        initializeHistoryAutoRegistrar();
        if (historyAutoRegistrar_) {
            historyAutoRegistrar_->autoRegisterHistoryStates(model_, historyManager_.get());
        }

        SCE_LOG_INFO("Model initialized with {} states", allStates.size());
        return true;
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Failed to extract model: {}", e.what());
        return false;
    }
}

bool StateMachine::evaluateCondition(const std::string &condition) {
    if (condition.empty()) {
        SCE_LOG_DEBUG("Empty condition, returning true");
        return true;
    }

    try {
        SCE_LOG_DEBUG("Evaluating condition: '{}'", condition);

        auto future = scriptEngine_.evaluateExpression(sessionId_, condition);
        auto result = future.get();

        if (!SCE::ScriptResultUtils::isSuccess(result)) {
            // §scxml-5.9: Condition evaluation error must raise error.execution
            SCE_LOG_ERROR("W3C SCXML 5.9: Failed to evaluate condition '{}': {}", condition, result.getErrorMessage());

            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution", "Failed to evaluate condition: " + condition);
            }
            return false;
        }

        // Convert result to boolean using integrated JSEngine method
        bool conditionResult = SCE::ScriptResultUtils::resultToBool(result);
        SCE_LOG_DEBUG("Condition '{}' evaluated to: {}", condition, conditionResult ? "true" : "false");

        return conditionResult;

    } catch (const std::exception &e) {
        // §scxml-5.9: Exception during condition evaluation must raise error.execution
        SCE_LOG_ERROR("W3C SCXML 5.9: Exception evaluating condition '{}': {}", condition, e.what());

        if (eventRaiser_) {
            eventRaiser_->raiseEvent("error.execution", "Exception evaluating condition: " + condition);
        }
        return false;
    }
}

void StateMachine::recordTruncatedMacrostep() {
    // The chain was still going one microstep past the budget,
    // which is the case the specification's Principles and Constraints call a
    // macrostep that does not terminate. Publish the refusal: the
    // configuration left behind is not the stable one the clause promises, and
    // every other reading a host has — the current state, `isRunning()`, the
    // call returning — says the opposite.
    ++truncatedMacrosteps_;
    lastTruncatedMacrostepState_ = getCurrentState();
    macrostepTruncated_ = true;
    SCE_LOG_ERROR("W3C SCXML: macrostep still going after {} microsteps in state '{}'; stopped taking them",
                  MAX_MACROSTEP_MICROSTEPS, lastTruncatedMacrostepState_);
    SCE_LOG_ERROR("W3C SCXML: Check for a circular eventless transition or a <raise> that answers itself");
}

bool StateMachine::mayTakeMicrostep() {
    if (macrostepTruncated_) {
        // Already refused this macrostep; the same chain does not get a second
        // budget for asking again.
        return false;
    }
    if (macrostepMicrostepsTaken_ < static_cast<uint32_t>(MAX_MACROSTEP_MICROSTEPS)) {
        return true;
    }
    // Work is still queued one microstep past the budget, which is the case
    // the specification calls a macrostep that does not terminate. The event
    // stays where it is; this publishes the refusal.
    recordTruncatedMacrostep();
    return false;
}

bool StateMachine::ensureJSEnvironment() {
    if (jsEnvironmentReady_) {
        return true;
    }

    return setupJSEnvironment();
}

bool StateMachine::setupJSEnvironment() {
    SCE_LOG_DEBUG("StateMachine: Script engine available for session setup");

    // Create JavaScript session only if it doesn't exist (for invoke scenarios)
    // Check if session already exists (created by InvokeExecutor for child sessions)
    bool sessionExists = scriptEngine_.hasSession(sessionId_);

    if (!sessionExists) {
        // Create new session for standalone StateMachine
        if (!scriptEngine_.createSession(sessionId_)) {
            SCE_LOG_ERROR("StateMachine: Failed to create JavaScript session");
            return false;
        }
        SCE_LOG_DEBUG("StateMachine: Created new JavaScript session: {}", sessionId_);
    } else {
        SCE_LOG_DEBUG("StateMachine: Using existing JavaScript session (injected): {}", sessionId_);
    }

    // §scxml-5.10: Set up read-only system variables (_sessionid, _name, _ioprocessors)
    std::string sessionName = model_ && !model_->getName().empty() ? model_->getName() : "StateMachine";
    auto ioProcessors = IOProcessorHelper::build(sessionId_, basicHttpAccessUri_);
    auto setupResult = scriptEngine_.setupSystemVariables(sessionId_, sessionName, ioProcessors).get();
    if (!setupResult.isSuccess()) {
        SCE_LOG_ERROR("StateMachine: Failed to setup system variables: {}", setupResult.getErrorMessage());
        return false;
    }

    // Register state query callback for In() function support (§scxml-5.9.1)
    // Uses IScriptEngine interface method instead of JSEngine-specific setStateMachine()
    // RACE CONDITION FIX: Capture weak_ptr to prevent heap-use-after-free (W3C Test 530)
    std::weak_ptr<StateMachine> weakSelf = shared_from_this();
    scriptEngine_.setStateQueryCallback(
        [weakSelf](const std::string &stateId) -> bool {
            if (auto sm = weakSelf.lock()) {
                return sm->isStateActive(stateId);
            }
            return false;
        },
        sessionId_);
    SCE_LOG_DEBUG("StateMachine: Registered state query callback for In() function support");

    // §scxml-5.3: Initialize data model (delegated to DataModelInitializer)
    dataModelInit_ = std::make_unique<DataModelInitializer>(model_, sessionId_, scriptEngine_);
    dataModelInit_->setEventRaiser(eventRaiser_);
    if (model_) {
        dataModelInit_->initializeAllDataItems(model_->getBinding());
    }

    // Initialize ActionExecutor and ExecutionContext (needed for script execution)
    if (!initializeActionExecutor()) {
        SCE_LOG_ERROR("StateMachine: Failed to initialize action executor");
        return false;
    }

    // §scxml-5.8: Execute top-level scripts AFTER datamodel init, BEFORE start()
    // §scxml-5.8.2: a <script> that is a child of <scxml> is evaluated at document
    // load time; every other <script> runs as ordinary executable content.
    if (model_) {
        const auto &topLevelScripts = model_->getTopLevelScripts();
        if (!topLevelScripts.empty()) {
            SCE_LOG_INFO("StateMachine: Executing {} top-level script(s) at document load time (W3C SCXML 5.8)",
                         topLevelScripts.size());

            for (size_t i = 0; i < topLevelScripts.size(); ++i) {
                const auto &script = topLevelScripts[i];

                if (!script) {
                    SCE_LOG_WARN("StateMachine: Null script at index {} - skipping (W3C SCXML 5.8)", i);
                    continue;
                }

                if (!executionContext_) {
                    SCE_LOG_ERROR("StateMachine: ExecutionContext is null - cannot execute scripts (W3C SCXML 5.8)");
                    return false;
                }

                SCE_LOG_DEBUG("StateMachine: Executing top-level script #{} (W3C SCXML 5.8)", i + 1);
                bool success = script->execute(*executionContext_);
                if (!success) {
                    SCE_LOG_ERROR(
                        "StateMachine: Top-level script #{} execution failed (W3C SCXML 5.8) - document rejected",
                        i + 1);
                    return false;  // §scxml-5.8: Script failure rejects document
                }
            }
            SCE_LOG_DEBUG("StateMachine: All {} top-level script(s) executed successfully (W3C SCXML 5.8)",
                          topLevelScripts.size());
        }
    }

    // Pass EventDispatcher to ActionExecutor if it was set before initialization
    if (eventDispatcher_ && actionExecutor_) {
        auto actionExecutorImpl = std::dynamic_pointer_cast<ActionExecutorImpl>(actionExecutor_);
        if (actionExecutorImpl) {
            actionExecutorImpl->setEventDispatcher(eventDispatcher_);
            SCE_LOG_DEBUG(
                "StateMachine: EventDispatcher passed to ActionExecutor during JS environment setup for session: {}",
                sessionId_);
        }
    }

    // Pass EventRaiser to ActionExecutor if available
    if (eventRaiser_ && actionExecutor_) {
        actionExecutor_->setEventRaiser(eventRaiser_);
        SCE_LOG_DEBUG("StateMachine: EventRaiser passed to ActionExecutor for session: {}", sessionId_);
    }

    // W3C SCXML: Auto-initialize EventRaiser if not already set (for standalone StateMachine)
    // This ensures done.state events can be queued during start() when parallel regions complete
    if (!eventRaiser_) {
        auto eventRaiser = std::make_shared<EventRaiserImpl>();
        setEventRaiser(eventRaiser);
        EventRaiserService::getInstance().registerEventRaiser(sessionId_, eventRaiser);
        SCE_LOG_DEBUG("StateMachine: Auto-initialized EventRaiser for session: {}", sessionId_);
    }

    // Register EventRaiser with JSEngine after session creation
    // This handles both cases: EventRaiser set before session creation (deferred) and after
    if (eventRaiser_) {
        // Use EventRaiserService for centralized registration
        if (EventRaiserService::getInstance().registerEventRaiser(sessionId_, eventRaiser_)) {
            SCE_LOG_DEBUG("StateMachine: EventRaiser registered via Service after session creation for session: {}",
                          sessionId_);
        } else {
            SCE_LOG_DEBUG("StateMachine: EventRaiser already registered for session: {}", sessionId_);
        }
    }

    jsEnvironmentReady_ = true;
    SCE_LOG_DEBUG("StateMachine: JavaScript environment setup completed");
    return true;
}

void StateMachine::updateStatistics() {
    stats_.currentState = getCurrentState();
    stats_.isRunning = isRunning_.load();
}

bool StateMachine::initializeActionExecutor() {
    try {
        // Create ActionExecutor using the same session as StateMachine
        actionExecutor_ = std::make_shared<ActionExecutorImpl>(sessionId_, scriptEngine_);

        // Cache the pointer to avoid dynamic_pointer_cast overhead in hot path
        cachedExecutorImpl_ = dynamic_cast<ActionExecutorImpl *>(actionExecutor_.get());

        // Inject EventRaiser if already set via builder pattern
        if (eventRaiser_) {
            actionExecutor_->setEventRaiser(eventRaiser_);
            SCE_LOG_DEBUG("StateMachine: EventRaiser injected to ActionExecutor during initialization for session: {}",
                          sessionId_);
        }

        // Create ExecutionContext with shared_ptr and sessionId
        executionContext_ = std::make_shared<ExecutionContextImpl>(actionExecutor_, sessionId_);

        SCE_LOG_DEBUG("ActionExecutor and ExecutionContext initialized for session: {}", sessionId_);
        return true;
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("Failed to initialize ActionExecutor: {}", e.what());
        return false;
    }
}

bool StateMachine::executeActionNodes(const std::vector<std::shared_ptr<SCE::IActionNode>> &actions) {
    if (!executionContext_) {
        SCE_LOG_WARN("StateMachine: ExecutionContext not initialized, skipping action node execution");
        return true;  // Not a failure, just no actions to execute
    }

    bool allSucceeded = true;

    // §scxml-4.9: the elements of a block run in document order, and once one of them
    // raises an error the remaining elements of that block are not processed.
    for (const auto &action : actions) {
        if (!action) {
            SCE_LOG_WARN("StateMachine: Null action node encountered, skipping");
            continue;
        }

        try {
            SCE_LOG_DEBUG("Executing action: {}", action->getActionType());
            if (action->execute(*executionContext_)) {
                SCE_LOG_DEBUG("Successfully executed action: {}", action->getActionType());
            } else {
                SCE_LOG_WARN("Failed to execute action: {} - W3C compliance: stopping remaining actions",
                             action->getActionType());
                allSucceeded = false;
                // W3C SCXML specification: If error occurs in executable content,
                // processor MUST NOT process remaining elements in the block
                break;
            }
        } catch (const std::exception &e) {
            SCE_LOG_WARN("Exception executing action {}: {} - W3C compliance: stopping remaining actions",
                         action->getActionType(), e.what());
            allSucceeded = false;
            // W3C SCXML specification: If error occurs in executable content,
            // processor MUST NOT process remaining elements in the block
            break;
        }
    }

    // W3C SCXML compliance: Return true only if all actions succeeded or no actions to execute
    // If any action failed, we stopped execution per W3C spec, so return false to indicate failure
    return actions.empty() || allSucceeded;
}

bool StateMachine::executeExitActions(const std::string &stateId) {
    if (!model_) {
        return true;  // No model, no actions to execute
    }

    // Find the StateNode in the SCXML model
    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        SCE_LOG_DEBUG("State {} not found in SCXML model, skipping exit actions", stateId);
        return true;  // Not an error if state not found in model
    }

    // §scxml-3.9: Execute block-based exit actions — a `<parallel>`'s like any
    // other state's: its regions are states of their own, exited before it
    const auto &exitBlocks = stateNode->getExitActionBlocks();
    if (!exitBlocks.empty()) {
        SCE_LOG_DEBUG("W3C SCXML 3.9: Executing {} exit action blocks for state: {}", exitBlocks.size(), stateId);

        // §scxml-3.9: Build lambda blocks for EntryExitHelper
        // ARCHITECTURE.md Zero Duplication: Delegate to shared Helper (lines 311-373)
        std::vector<std::function<void()>> exitLambdas;
        for (const auto &exitBlock : exitBlocks) {
            exitLambdas.push_back([&, exitBlock]() {
                // §scxml-3.9: Each onexit handler is a separate block
                if (!executeActionNodes(exitBlock)) {
                    SCE_LOG_WARN("W3C SCXML 3.9: Exit action block failed, continuing with remaining blocks");
                    // Lambda return stops THIS block only, next block continues per W3C spec
                }
            });
        }

        // §scxml-3.9: Delegate to EntryExitHelper (Single Source of Truth)
        SCE::Core::EntryExitHelper<InterpreterPolicy, IEventRaiser>::executeExitBlocks(exitLambdas, *eventRaiser_,
                                                                                       stateId);

        // W3C SCXML: State exit succeeds even if some action blocks fail
        return true;
    }

    return true;
}

void StateMachine::initializeHistoryManager() {
    SCE_LOG_DEBUG("StateMachine: Initializing History Manager with SOLID architecture");

    // Create state provider function for dependency injection
    auto stateProvider = [this](const std::string &stateId) -> std::shared_ptr<IStateNode> {
        if (!model_) {
            return nullptr;
        }
        // Find state by ID in the shared_ptr vector
        auto allStates = model_->getAllStates();
        for (const auto &state : allStates) {
            if (state && state->getId() == stateId) {
                return state;
            }
        }
        return nullptr;
    };

    // §scxml-3.10: Create validator for history operations
    auto validator = std::make_unique<HistoryValidator>(stateProvider);

    // §scxml-3.10: Create HistoryManager using shared HistoryHelper (Zero Duplication with AOT)
    historyManager_ = std::make_unique<HistoryManager>(stateProvider, std::move(validator));

    SCE_LOG_INFO("StateMachine: History Manager initialized - using shared HistoryHelper");
}

void StateMachine::initializeHistoryAutoRegistrar() {
    SCE_LOG_DEBUG("StateMachine: Initializing History Auto-Registrar with SOLID architecture");

    // Create state provider function for dependency injection (same as history manager)
    auto stateProvider = [this](const std::string &stateId) -> std::shared_ptr<IStateNode> {
        if (!model_) {
            return nullptr;
        }
        // Find state by ID in the model
        auto allStates = model_->getAllStates();
        for (const auto &state : allStates) {
            if (state && state->getId() == stateId) {
                return state;
            }
        }
        return nullptr;
    };

    // Create HistoryStateAutoRegistrar with dependency injection
    historyAutoRegistrar_ = std::make_unique<HistoryStateAutoRegistrar>(stateProvider);

    SCE_LOG_INFO("StateMachine: History Auto-Registrar initialized with SOLID dependencies");
}

bool StateMachine::registerHistoryState(const std::string &historyStateId, const std::string &parentStateId,
                                        HistoryType type, const std::string &defaultStateId) {
    if (!historyManager_) {
        SCE_LOG_ERROR("StateMachine: History Manager not initialized");
        return false;
    }

    return historyManager_->registerHistoryState(historyStateId, parentStateId, type, defaultStateId);
}

bool StateMachine::isHistoryState(const std::string &stateId) const {
    if (!historyManager_) {
        return false;
    }

    return historyManager_->isHistoryState(stateId);
}

void StateMachine::clearAllHistory() {
    if (historyManager_) {
        historyManager_->clearAllHistory();
    }
}

std::vector<HistoryEntry> StateMachine::getHistoryEntries() const {
    if (!historyManager_) {
        return {};
    }

    return historyManager_->getHistoryEntries();
}

void StateMachine::executeOnEntryActions(const std::string &stateId) {
    if (!model_) {
        SCE_LOG_ERROR("Cannot execute onentry actions: SCXML model is null");
        return;
    }

    // Find the state node
    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        SCE_LOG_ERROR("Cannot find state node for onentry execution: {}", stateId);
        return;
    }

    // §scxml-3.8: Get entry action blocks from the state
    const auto &entryBlocks = stateNode->getEntryActionBlocks();
    if (entryBlocks.empty()) {
        SCE_LOG_DEBUG("No onentry actions to execute for state: {}", stateId);
        return;
    }

    SCE_LOG_DEBUG("W3C SCXML 3.8: Executing {} onentry action blocks for state: {}", entryBlocks.size(), stateId);

    // §scxml-3.8: Build lambda blocks for EntryExitHelper
    // ARCHITECTURE.md Zero Duplication: Delegate to shared Helper (lines 311-373)
    std::vector<std::function<void()>> lambdaBlocks;
    for (const auto &actionBlock : entryBlocks) {
        lambdaBlocks.push_back([&, actionBlock]() {
            // Execute all actions in this block
            for (const auto &action : actionBlock) {
                if (!action) {
                    SCE_LOG_WARN("Null onentry action found in state: {}", stateId);
                    continue;
                }

                SCE_LOG_DEBUG("StateMachine: Executing onentry action: {} in state: {}", action->getActionType(),
                              stateId);

                // Create execution context for the action
                if (actionExecutor_) {
                    auto sharedActionExecutor =
                        std::shared_ptr<IActionExecutor>(actionExecutor_.get(), [](IActionExecutor *) {});
                    ExecutionContextImpl context(sharedActionExecutor, sessionId_);

                    // Execute the action
                    if (!action->execute(context)) {
                        SCE_LOG_WARN("StateMachine: Failed to execute onentry action: {} - W3C SCXML 3.8: "
                                     "stopping remaining actions in THIS block only",
                                     action->getActionType());
                        // §scxml-3.8: If error occurs, stop THIS block via lambda return
                        return;
                    } else {
                        SCE_LOG_DEBUG("StateMachine: Successfully executed onentry action: {} in state: {}",
                                      action->getActionType(), stateId);
                    }
                } else {
                    SCE_LOG_ERROR("Cannot execute onentry action: ActionExecutor is null");
                    return;
                }
            }
        });
    }

    // §scxml-3.8: Delegate to EntryExitHelper (Single Source of Truth)
    // ARCHITECTURE.md Zero Duplication: Shared block orchestration between Interpreter and AOT
    SCE::Core::EntryExitHelper<InterpreterPolicy, IEventRaiser>::executeEntryBlocks(lambdaBlocks, *eventRaiser_,
                                                                                    stateId);
}

// EventDispatcher management
void StateMachine::setEventDispatcher(std::shared_ptr<IEventDispatcher> eventDispatcher) {
    eventDispatcher_ = eventDispatcher;

    // Pass EventDispatcher to ActionExecutor for send actions
    if (actionExecutor_) {
        auto actionExecutorImpl = std::dynamic_pointer_cast<ActionExecutorImpl>(actionExecutor_);
        if (actionExecutorImpl) {
            actionExecutorImpl->setEventDispatcher(eventDispatcher);
            SCE_LOG_DEBUG("StateMachine: EventDispatcher passed to ActionExecutor for session: {}", sessionId_);
        }
    }

    // Pass EventDispatcher to InvokeExecutor for child session management
    if (invokeExecutor_) {
        invokeExecutor_->setEventDispatcher(eventDispatcher);
        SCE_LOG_DEBUG("StateMachine: EventDispatcher passed to InvokeExecutor for session: {}", sessionId_);

        // W3C SCXML Test 192: Set parent StateMachine for completion callback state checking
        // Only set if this StateMachine is managed by shared_ptr (not during construction)
        // This will be set later in executeInvoke() when actually needed
    }
}

// §scxml-6.4.3: Completion callback management
void StateMachine::setCompletionCallback(CompletionCallback callback) {
    completionCallback_ = callback;
    SCE_LOG_DEBUG("StateMachine: Completion callback {} for session: {}", callback ? "set" : "cleared", sessionId_);
}

// EventRaiser management
void StateMachine::setEventRaiser(std::shared_ptr<IEventRaiser> eventRaiser) {
    SCE_LOG_DEBUG("StateMachine: setEventRaiser called for session: {}", sessionId_);
    eventRaiser_ = eventRaiser;

    // SCXML W3C compliance: Set EventRaiser callback to StateMachine's processEvent
    // This allows events generated by raise actions to actually trigger state transitions
    if (eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            SCE_LOG_DEBUG(
                "StateMachine: EventRaiser callback setup - EventRaiser instance: {}, StateMachine instance: {}",
                (void *)eventRaiserImpl.get(), (void *)this);
            // Set StateMachine's dispatch entry point as EventRaiser callback.
            // `processDispatchedEvent`, not `processEvent`: the raiser has
            // already established which queue this event came off, and the
            // host-facing overload would overwrite that with "external" and
            // hand an internal event to every autoforward child.
            eventRaiserImpl->setEventCallback([this](const std::string &eventName,
                                                     const std::string &eventData) -> bool {
                if (isRunning_) {
                    SCE_LOG_DEBUG("EventRaiser callback: StateMachine::processDispatchedEvent called - event: '{}', "
                                  "data: '{}', StateMachine instance: {}",
                                  eventName, eventData, (void *)this);
                    // Use 2-parameter version (no originSessionId from old callback)
                    auto result = processDispatchedEvent(eventName, eventData);
                    SCE_LOG_DEBUG("EventRaiser callback: processEvent result - success: {}, state transition: {} -> {}",
                                  result.success, result.fromState, result.toState);
                    return result.success;
                } else {
                    SCE_LOG_WARN("EventRaiser callback: StateMachine not running - ignoring event '{}'", eventName);
                    return false;
                }
            });
            SCE_LOG_DEBUG(
                "StateMachine: EventRaiser callback set to processEvent - session: {}, EventRaiser instance: {}",
                sessionId_, (void *)eventRaiserImpl.get());
        }
    }

    // Register EventRaiser with JSEngine for #_invokeid target support
    // Use EventRaiserService for centralized registration
    if (eventRaiser_) {
        if (EventRaiserService::getInstance().registerEventRaiser(sessionId_, eventRaiser_)) {
            SCE_LOG_DEBUG("StateMachine: EventRaiser registered via Service for session: {}", sessionId_);
        } else {
            SCE_LOG_DEBUG("StateMachine: EventRaiser registration deferred or already exists for session: {}",
                          sessionId_);
        }
    }

    // Pass EventRaiser to ActionExecutor if it exists (during build phase)
    if (actionExecutor_) {
        actionExecutor_->setEventRaiser(eventRaiser);
        SCE_LOG_DEBUG("StateMachine: EventRaiser passed to ActionExecutor for session: {}", sessionId_);
    }
    // Note: If ActionExecutor doesn't exist yet, it will be set during loadSCXMLFromString
}

bool StateMachine::raiseExternalEvent(const std::string &eventName, const std::string &eventData) {
    if (!eventRaiser_) {
        SCE_LOG_WARN("StateMachine: Cannot raise external event - no EventRaiser");
        return false;
    }
    return eventRaiser_->raiseExternalEvent(eventName, eventData);
}

std::shared_ptr<IEventDispatcher> StateMachine::getEventDispatcher() const {
    return eventDispatcher_;
}

InvokeExecutor *StateMachine::getInvokeExecutor() const {
    return invokeExecutor_.get();
}

std::vector<std::shared_ptr<StateMachine>> StateMachine::getInvokedChildren() {
    if (!invokeExecutor_) {
        return {};
    }

    // Return ALL invoked children for visualization (not just autoForward ones)
    return invokeExecutor_->getAllInvokedSessions(sessionId_);
}

void StateMachine::setSessionFilePath(const std::string &filePath) {
    // JSEngine is a singleton, accessed via instance()
    SessionRegistry::instance().registerSessionFilePath(sessionId_, filePath);
    SCE_LOG_DEBUG("StateMachine: Registered session file path: {} for session: {}", filePath, sessionId_);
}

void StateMachine::setBasicHttpAccessUri(const std::string &accessUri) {
    basicHttpAccessUri_ = accessUri;
    SCE_LOG_DEBUG("StateMachine: BasicHTTP access URI for session {}: {}", sessionId_, accessUri);
}

void StateMachine::deferInvokeExecution(const std::string &stateId,
                                        const std::vector<std::shared_ptr<IInvokeNode>> &invokes) {
    SCE_LOG_DEBUG("StateMachine: Deferring {} invokes for state: {} in session: {}", invokes.size(), stateId,
                  sessionId_);

    // Thread-safe access to pendingInvokes_
    std::lock_guard<std::recursive_mutex> lock(pendingInvokesMutex_);
    size_t beforeSize = pendingInvokes_.size();

    // §scxml-6.4: Defer each invoke individually (matches AOT pattern) through
    // the shared helper, so the Interpreter and the AOT engines schedule with
    // one algorithm (ARCHITECTURE.md Zero Duplication) and both inherit
    // §scxml-D-GlobalVariables' set semantics for `statesToInvoke`.
    for (size_t i = 0; i < invokes.size(); ++i) {
        const auto &invoke = invokes[i];
        // The helper's identity is (state, invokeId), so an anonymous
        // `<invoke>` needs a placeholder that still separates it from its
        // siblings — document order does that and reads better in the log than
        // a bare "(auto-generated)" repeated per invoke.
        std::string invokeId =
            invoke ? (invoke->getId().empty() ? "(auto-generated#" + std::to_string(i) + ")" : invoke->getId())
                   : "null";
        std::string invokeType = invoke ? invoke->getType() : "null";
        SCE_LOG_DEBUG("StateMachine: DETAILED DEBUG - Deferring invoke[{}]: id='{}', type='{}'", i, invokeId,
                      invokeType);

        SCE::Core::InvokeHelper::deferInvoke(pendingInvokes_, PendingInvoke{invokeId, stateId, invoke});
    }

    SCE_LOG_DEBUG("StateMachine: DETAILED DEBUG - Pending invokes count: {} -> {}", beforeSize, pendingInvokes_.size());
}

void StateMachine::executePendingInvokes() {
    // W3C SCXML Test 192: Set parent StateMachine before executing invokes (requires shared_ptr context)
    // This is safe here because executePendingInvokes() is only called when StateMachine is already in shared_ptr
    // context
    if (invokeExecutor_) {
        try {
            invokeExecutor_->setParentStateMachine(shared_from_this());
            SCE_LOG_DEBUG(
                "StateMachine: Parent StateMachine set in InvokeExecutor before executing invokes for session: {}",
                sessionId_);
        } catch (const std::bad_weak_ptr &e) {
            SCE_LOG_WARN("StateMachine: Cannot set parent StateMachine - not managed by shared_ptr yet for session: {}",
                         sessionId_);
        }
    }

    // §scxml-6.4: Execute pending invokes using InvokeHelper (ARCHITECTURE.md Zero Duplication)
    // Uses same pattern as AOT engine - copy-and-clear prevents iterator invalidation
    std::lock_guard<std::recursive_mutex> lock(pendingInvokesMutex_);

    SCE_LOG_DEBUG("StateMachine: Found {} pending invokes to execute for session: {}", pendingInvokes_.size(),
                  sessionId_);

    SCE::Core::InvokeHelper::executePendingInvokes(pendingInvokes_, [this](const PendingInvoke &pending) {
        // W3C SCXML Test 252: Only execute if state is still active (entered-and-not-exited)
        if (!isStateActive(pending.state)) {
            SCE_LOG_DEBUG("StateMachine: Skipping invoke '{}' for inactive state: {}", pending.invokeId, pending.state);
            return;
        }

        SCE_LOG_DEBUG("StateMachine: Executing invoke '{}' for state '{}'", pending.invokeId, pending.state);

        if (invokeExecutor_) {
            std::string invokeid = invokeExecutor_->executeInvoke(pending.invoke, sessionId_);
            if (invokeid.empty()) {
                SCE_LOG_ERROR("StateMachine: Failed to execute invoke '{}' for state: {}", pending.invokeId,
                              pending.state);
                // W3C SCXML: Continue execution even if invokes fail
            }
        } else {
            SCE_LOG_ERROR("StateMachine: Cannot execute invoke - InvokeExecutor is null");
        }
    });
}

// §scxml-5.5: Helper functions moved to DoneDataHelper (Zero Duplication)
// - escapeJsonString() -> DoneDataHelper::escapeJsonString()
// ScriptValue -> JSON conversion goes through EventDataHelper::scriptValueToJsonString
// (canonical JSON pipeline; see DoneDataHelper::evaluateContent/evaluateParams).

/**
 * §scxml-5.5 & 5.7: Evaluate donedata and return JSON event data
 *
 * Handles two types of param errors with different behaviors:
 *
 * 1. Structural Error (empty location=""):
 *    - Indicates malformed SCXML document
 *    - Raises error.execution event
 *    - Returns false to prevent done.state event generation
 *    - Used when param has no location/expr attribute
 *
 * 2. Runtime Error (invalid expression like "foo"):
 *    - Indicates runtime evaluation failure
 *    - Raises error.execution event
 *    - Ignores the failed param and continues with others
 *    - Returns true to generate done.state event with partial/empty data
 *    - Used when param expression evaluation fails
 *
 * This distinction ensures:
 * - Structural errors fail fast (no done.state)
 * - Runtime errors are recoverable (done.state with available data)
 *
 * @param finalStateId The ID of the final state
 * @param outEventData Output parameter for JSON event data
 * @return false if structural error (prevents done.state), true otherwise
 */
bool StateMachine::evaluateDoneData(const std::string &finalStateId, std::string &outEventData,
                                    std::optional<ScriptValue> &outTypedData) {
    // §scxml-5.5: Initialize output
    outEventData = "";

    if (!model_) {
        return true;  // No donedata to evaluate
    }

    auto finalState = model_->findStateById(finalStateId);
    if (!finalState) {
        return true;  // No donedata to evaluate
    }

    const auto &doneData = finalState->getDoneData();

    // §scxml-5.5 — route through DoneDataHelper (Zero Duplication SSoT):
    //   Expression => evaluate against the datamodel.
    //   Literal    => children used as the content value, no evaluation.
    switch (doneData.getContentKind()) {
    case DoneData::ContentKind::Expression:
        SCE_LOG_DEBUG("W3C SCXML 5.5: Evaluating donedata <content expr>: '{}'", doneData.getContent());
        return DoneDataHelper::evaluateContent(
            scriptEngine_, sessionId_, doneData.getContent(), outEventData,
            [this](const std::string &msg) {
                SCE_LOG_ERROR("W3C SCXML 5.5: Failed to evaluate donedata content: {}", msg);
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", msg);
                }
            },
            &outTypedData);
    case DoneData::ContentKind::Literal:
        SCE_LOG_DEBUG("W3C SCXML 5.5: Emitting donedata literal content: '{}'", doneData.getContent());
        DoneDataHelper::emitContentLiteral(doneData.getContent(), outEventData, &outTypedData);
        return true;
    case DoneData::ContentKind::None:
        break;
    }

    // §scxml-5.5: Evaluate params using shared DoneDataHelper (Zero Duplication)
    const auto &params = doneData.getParams();
    if (!params.empty()) {
        SCE_LOG_DEBUG("W3C SCXML 5.5: Evaluating {} donedata params", params.size());
        // The Interpreter parses SCXML at run time, so a param expression here
        // is always the author's own ECMAScript — there is no build step that
        // could have lowered it. Said once, at the boundary, rather than left
        // to an implicit conversion the vector cannot perform element-wise.
        std::vector<std::pair<std::string, ScriptSource>> taggedParams;
        taggedParams.reserve(params.size());
        for (const auto &param : params) {
            taggedParams.emplace_back(param.first, ScriptSource::ecmascript(param.second));
        }
        return DoneDataHelper::evaluateParams(
            scriptEngine_, sessionId_, taggedParams, outEventData,
            [this](const std::string &msg) {
                SCE_LOG_ERROR("W3C SCXML 5.7: {}", msg);
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", msg);
                }
            },
            &outTypedData);
    }

    // No donedata
    return true;
}

}  // namespace SCE
