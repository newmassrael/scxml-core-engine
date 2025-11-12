// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "InteractiveTestRunner.h"

#include "common/Logger.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "factory/NodeFactory.h"
#include "model/InvokeNode.h"
#include "model/SCXMLModel.h"
#include "model/StateNode.h"
#include "parsing/SCXMLParser.h"
#include "runtime/EventRaiserImpl.h"
#include "scripting/JSEngine.h"

// W3C SCXML 3.7: Action types for executable content visualization
#include "actions/AssignAction.h"
#include "actions/CancelAction.h"
#include "actions/ForeachAction.h"
#include "actions/IfAction.h"
#include "actions/LogAction.h"
#include "actions/RaiseAction.h"
#include "actions/ScriptAction.h"
#include "actions/SendAction.h"

#include <set>

namespace SCE::W3C {

InteractiveTestRunner::InteractiveTestRunner()
    : stateMachine_(std::make_shared<StateMachine>()), snapshotManager_(1000)  // 1000 step history
      ,
      currentStep_(0) {
    // W3C SCXML 6.2: Create event infrastructure for send/invoke support
    // Event callback: For visualization, we don't auto-process scheduled events
    // User manually steps forward via stepForward()
    auto eventCallback = [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target,
                            [[maybe_unused]] const std::string &sendId) -> bool {
        // Execute event through target (InternalEventTarget will call EventRaiser)
        try {
            auto future = target->send(event);
            auto result = future.get();
            return result.isSuccess;
        } catch (...) {
            return false;
        }
    };

    // Create event scheduler
    scheduler_ = std::make_shared<EventSchedulerImpl>(eventCallback);

    // Create event raiser (concrete type for setScheduler access)
    auto eventRaiser = std::make_shared<EventRaiserImpl>();
    eventRaiser_ = eventRaiser;

    // Connect EventRaiser to EventScheduler for delayed event polling
    eventRaiser->setScheduler(scheduler_);

    // W3C SCXML: Inject EventRaiser to StateMachine BEFORE setupJSEnvironment()
    // This prevents StateMachine from auto-creating a duplicate EventRaiser
    // Single Source of Truth: One EventRaiser per state machine (ARCHITECTURE.md Zero Duplication)
    stateMachine_->setEventRaiser(eventRaiser_);

    // Create event target factory and dispatcher
    auto targetFactory = std::make_shared<EventTargetFactoryImpl>(eventRaiser, scheduler_);
    eventDispatcher_ = std::make_shared<EventDispatcherImpl>(scheduler_, targetFactory);

    // Set EventDispatcher on StateMachine
    stateMachine_->setEventDispatcher(eventDispatcher_);

    LOG_DEBUG("InteractiveTestRunner: Event infrastructure initialized (scheduler, dispatcher, targets)");
}

InteractiveTestRunner::~InteractiveTestRunner() {
    // W3C SCXML 6.2: Shutdown event infrastructure
    if (scheduler_) {
        scheduler_->shutdown(true);  // Wait for pending events to complete
    }

    if (stateMachine_) {
        stateMachine_->stop();
    }
}

bool InteractiveTestRunner::loadSCXML(const std::string &scxmlSource, bool isFilePath) {
    bool success = false;

    if (isFilePath) {
        success = stateMachine_->loadSCXML(scxmlSource);
    } else {
        success = stateMachine_->loadSCXMLFromString(scxmlSource);
    }

    if (!success) {
        LOG_ERROR("InteractiveTestRunner: Failed to load SCXML from {}", isFilePath ? "file" : "string");
        return false;
    }

    LOG_DEBUG("InteractiveTestRunner: Successfully loaded SCXML");

    // W3C SCXML 6.3: Static analysis to detect sub-SCXML files for visualization
    subScxmlStructures_.clear();

    auto model = stateMachine_->getModel();
    if (model) {
        analyzeSubSCXML(model);
    }

    return true;
}

bool InteractiveTestRunner::initialize() {
    // Interactive mode: Enter initial state only, skip auto-processing of queued events
    // W3C SCXML 3.13: Allow manual step-by-step execution of raise/send events
    if (!stateMachine_->start(/*autoProcessQueuedEvents=*/false)) {
        LOG_ERROR("InteractiveTestRunner: Failed to start state machine");
        return false;
    }

    // Initialize tracking variables
    currentStep_ = 0;
    previousActiveStates_.clear();
    lastTransitionSource_.clear();
    lastTransitionTarget_.clear();
    lastEventName_.clear();

    // Capture initial snapshot for true reset functionality
    captureSnapshot();

    // Save initial snapshot (before any raiseEvent() calls)
    auto initialSnapshotOpt = snapshotManager_.getSnapshot(0);
    if (initialSnapshotOpt) {
        initialSnapshot_ = *initialSnapshotOpt;
    }

    LOG_DEBUG("InteractiveTestRunner: Initialized to step 0 (initial configuration)");
    return true;
}

bool InteractiveTestRunner::stepForward() {
    if (isInFinalState()) {
        LOG_DEBUG("InteractiveTestRunner: Already in final state, cannot step forward");
        return false;
    }

#ifdef __EMSCRIPTEN__
    // W3C SCXML 6.2: Poll scheduled events (for delayed <send> operations)
    // WASM only: Manual polling required (no timer thread)
    // Native: Timer thread handles scheduled events automatically
    if (scheduler_) {
        // Downcast to EventSchedulerImpl to access poll() method
        auto schedulerImpl = std::dynamic_pointer_cast<EventSchedulerImpl>(scheduler_);
        if (schedulerImpl) {
            size_t polledCount = schedulerImpl->poll();
            if (polledCount > 0) {
                LOG_DEBUG("InteractiveTestRunner: Polled {} scheduled events into queue", polledCount);
            }
        }
    }
#endif

    // Capture current active states BEFORE transition (for source detection)
    auto preTransitionStates = stateMachine_->getActiveStates();
    previousActiveStates_.clear();
    for (const auto &state : preTransitionStates) {
        previousActiveStates_.insert(state);
    }

    // W3C SCXML 3.13: EventRaiser automatically handles priority (INTERNAL → EXTERNAL → Eventless)
    // Zero Duplication: Single priority queue with QueuedEventComparator
    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
    if (eventRaiserImpl && eventRaiserImpl->processNextQueuedEvent()) {
        // EventRaiser processed an event (internal or external)
        // W3C SCXML 3.13: Record event to history for time-travel debugging
        std::string eventName, eventData;
        if (eventRaiserImpl->getLastProcessedEvent(eventName, eventData)) {
            executedEvents_.push_back(EventSnapshot(eventName, eventData));
            lastEventName_ = eventName;
        } else {
            lastEventName_ = "[internal]";
        }

        // Get active states AFTER transition
        auto postTransitionStates = stateMachine_->getActiveStates();
        std::set<std::string> currentStates;
        for (const auto &state : postTransitionStates) {
            currentStates.insert(state);
        }

        // W3C SCXML 3.13: Get last executed transition from StateMachine
        // ARCHITECTURE.md Zero Duplication: StateMachine tracks all transitions (including eventless)
        lastTransitionSource_ = stateMachine_->getLastTransitionSource();
        lastTransitionTarget_ = stateMachine_->getLastTransitionTarget();
        LOG_DEBUG("W3C SCXML 3.13: Interactive visualizer read transition: {} -> {}", lastTransitionSource_,
                  lastTransitionTarget_);

        currentStep_++;
        captureSnapshot();

        LOG_DEBUG("InteractiveTestRunner: Step {} - event '{}' processed (transition: {} -> {})", currentStep_,
                  lastEventName_, lastTransitionSource_, lastTransitionTarget_);
        return true;
    }

    // W3C SCXML 3.13: Check for eventless transitions (only if all queues are empty)
    auto result = stateMachine_->processEvent("");  // Null event triggers eventless transitions
    lastEventName_ = "";

    if (result.success) {
        // W3C SCXML 3.13: Record eventless transition to history
        executedEvents_.push_back(EventSnapshot("", ""));

        lastTransitionSource_ = result.fromState;
        lastTransitionTarget_ = result.toState;
        currentStep_++;
        captureSnapshot();

        LOG_DEBUG("InteractiveTestRunner: Step {} - eventless transition: {} -> {}", currentStep_,
                  lastTransitionSource_, lastTransitionTarget_);
        return true;
    }

    LOG_DEBUG("InteractiveTestRunner: No event in queue and no eventless transitions available");
    return false;
}

bool InteractiveTestRunner::stepBackward() {
    if (currentStep_ <= 0) {
        LOG_DEBUG("InteractiveTestRunner: Already at initial state, cannot step backward");
        return false;
    }

    // Get previous snapshot
    auto prevSnapshot = snapshotManager_.getSnapshot(currentStep_ - 1);
    if (!prevSnapshot) {
        LOG_ERROR("InteractiveTestRunner: Failed to find snapshot for step {}", currentStep_ - 1);
        return false;
    }

    // Restore snapshot
    if (!restoreSnapshot(*prevSnapshot)) {
        LOG_ERROR("InteractiveTestRunner: Failed to restore snapshot for step {}", currentStep_ - 1);
        return false;
    }

    currentStep_--;
    LOG_DEBUG("InteractiveTestRunner: Restored to step {}", currentStep_);
    return true;
}

void InteractiveTestRunner::reset() {
    // Restore to true initial configuration (before any raiseEvent() calls)
    if (!initialSnapshot_) {
        LOG_ERROR("InteractiveTestRunner: No initial snapshot available, cannot reset");
        return;
    }

    if (restoreSnapshot(*initialSnapshot_)) {
        // W3C SCXML 3.13: Complete history reset for time-travel debugging
        // Zero Duplication: Single Source of Truth - clear all execution history
        currentStep_ = 0;
        previousActiveStates_.clear();
        executedEvents_.clear();

        // CRITICAL: Clear snapshot history to prevent stepBackward() from accessing old sessions
        // Without this, stepBackward() would restore stale snapshots from previous execution
        snapshotManager_.clear();

        // Re-capture initial snapshot as step 0 (fresh start)
        captureSnapshot();

        // Update initialSnapshot_ reference to new step 0
        auto initialSnapshotOpt = snapshotManager_.getSnapshot(0);
        if (initialSnapshotOpt) {
            initialSnapshot_ = *initialSnapshotOpt;
            LOG_DEBUG("InteractiveTestRunner: Reset complete - history cleared, new step 0 captured");
        } else {
            LOG_ERROR("InteractiveTestRunner: Failed to capture new initial snapshot after reset");
        }
    } else {
        LOG_ERROR("InteractiveTestRunner: Failed to restore initial snapshot");
    }
}

void InteractiveTestRunner::raiseEvent(const std::string &eventName, const std::string &eventData) {
    // W3C SCXML 3.13: Delegate to EventRaiser's external queue (Single Source of Truth)
    // Zero Duplication: EventRaiser owns all event queue management
    eventRaiser_->raiseExternalEvent(eventName, eventData);

    // W3C SCXML 3.13: Event queuing is NOT a microstep
    // Step only increments when event is actually processed in stepForward()
    // However, we must capture snapshot to preserve queue state for time-travel debugging
    captureSnapshot();

    LOG_DEBUG("InteractiveTestRunner: Queued external event '{}' via EventRaiser (current step: {})", eventName,
              currentStep_);
}

bool InteractiveTestRunner::removeInternalEvent(int index) {
    // W3C SCXML 3.13: Remove event from internal queue at specified index
    std::vector<EventSnapshot> internalQueue, externalQueue;
    extractEventQueues(internalQueue, externalQueue);

    // Validate index
    if (index < 0 || index >= static_cast<int>(internalQueue.size())) {
        LOG_WARN("InteractiveTestRunner: Invalid internal queue index {} (queue size: {})", index,
                 internalQueue.size());
        return false;
    }

    // Remove event at index
    internalQueue.erase(internalQueue.begin() + index);

    // Restore modified queues
    restoreEventQueues(internalQueue, externalQueue);

    // Capture snapshot to reflect queue modification
    // History branching: This creates a new execution path from current step
    captureSnapshot();

    LOG_DEBUG("InteractiveTestRunner: Removed internal event at index {} (current step: {})", index, currentStep_);
    return true;
}

bool InteractiveTestRunner::removeExternalEvent(int index) {
    // W3C SCXML 3.13: Remove event from external queue at specified index
    std::vector<EventSnapshot> internalQueue, externalQueue;
    extractEventQueues(internalQueue, externalQueue);

    // Validate index
    if (index < 0 || index >= static_cast<int>(externalQueue.size())) {
        LOG_WARN("InteractiveTestRunner: Invalid external queue index {} (queue size: {})", index,
                 externalQueue.size());
        return false;
    }

    // Remove event at index
    externalQueue.erase(externalQueue.begin() + index);

    // Restore modified queues
    restoreEventQueues(internalQueue, externalQueue);

    // Capture snapshot to reflect queue modification
    // History branching: This creates a new execution path from current step
    captureSnapshot();

    LOG_DEBUG("InteractiveTestRunner: Removed external event at index {} (current step: {})", index, currentStep_);
    return true;
}

size_t InteractiveTestRunner::pollScheduler() {
    // W3C SCXML 6.2: Poll event scheduler to move ready delayed send events to queue
#ifdef __EMSCRIPTEN__
    if (!scheduler_) {
        return 0;
    }

    auto schedulerImpl = std::dynamic_pointer_cast<EventSchedulerImpl>(scheduler_);
    if (!schedulerImpl) {
        return 0;
    }

    size_t polledCount = schedulerImpl->poll();
    if (polledCount > 0) {
        LOG_DEBUG("InteractiveTestRunner::pollScheduler: Moved {} scheduled events to queue", polledCount);
    }

    return polledCount;
#else
    // Native builds use automatic timer thread for scheduled event processing
    LOG_WARN("InteractiveTestRunner::pollScheduler: Not supported in Native builds (use timer thread)");
    return 0;
#endif
}

std::vector<std::string> InteractiveTestRunner::getActiveStates() const {
    return stateMachine_->getActiveStates();
}

bool InteractiveTestRunner::isInFinalState() const {
    return stateMachine_->isInFinalState();
}

void InteractiveTestRunner::captureSnapshot() {
    auto activeStates = stateMachine_->getActiveStates();
    auto dataModel = extractDataModel();

    // W3C SCXML 3.13: Extract event queues from EventRaiser (Single Source of Truth)
    std::vector<EventSnapshot> internalQueue;
    std::vector<EventSnapshot> externalQueue;
    extractEventQueues(internalQueue, externalQueue);

    // W3C SCXML 3.13: UI events are now part of EventRaiser's external queue
    // Zero Duplication: No separate pendingEvents_ tracking needed
    std::vector<EventSnapshot> pendingUIEvents = externalQueue;  // Alias for clarity in StateSnapshot

    // W3C SCXML 6.2: Extract scheduled events for step backward restoration
    std::vector<ScheduledEventSnapshot> scheduledEventsSnapshots;
    if (scheduler_) {
        auto scheduledEvents = scheduler_->getScheduledEvents();
        scheduledEventsSnapshots.reserve(scheduledEvents.size());
        for (const auto &event : scheduledEvents) {
            scheduledEventsSnapshots.emplace_back(event.eventName, event.sendId, event.originalDelay.count(),
                                                  event.sessionId, event.targetUri, event.eventType, event.eventData,
                                                  event.content);
        }
    }

    // Convert std::vector<std::string> to std::set<std::string> for StateSnapshot
    std::set<std::string> activeStatesSet(activeStates.begin(), activeStates.end());

    snapshotManager_.captureSnapshot(activeStatesSet, dataModel, internalQueue, externalQueue, pendingUIEvents,
                                     scheduledEventsSnapshots, executedEvents_, currentStep_, lastEventName_,
                                     lastTransitionSource_, lastTransitionTarget_);
}

bool InteractiveTestRunner::restoreSnapshot(const StateSnapshot &snapshot) {
    // Stop current state machine
    stateMachine_->stop();

    // Reload SCXML model
    auto model = stateMachine_->getModel();
    if (!model) {
        LOG_ERROR("InteractiveTestRunner: No model available for restore");
        return false;
    }

    // Create new state machine instance
    stateMachine_ = std::make_shared<StateMachine>();
    if (!stateMachine_->loadModel(model)) {
        LOG_ERROR("InteractiveTestRunner: Failed to reload model");
        return false;
    }

    // W3C SCXML: Re-inject EventRaiser and EventDispatcher after StateMachine recreation
    // This prevents duplicate EventRaiser creation (ARCHITECTURE.md Zero Duplication)
    stateMachine_->setEventRaiser(eventRaiser_);
    stateMachine_->setEventDispatcher(eventDispatcher_);

    // Interactive mode: Enter initial state only, skip auto-processing
    if (!stateMachine_->start(/*autoProcessQueuedEvents=*/false)) {
        LOG_ERROR("InteractiveTestRunner: Failed to restart state machine");
        return false;
    }

    // Restore data model
    restoreDataModel(snapshot.dataModel);

    // W3C SCXML 3.13: Restore active states directly without side effects
    // ARCHITECTURE.md Zero Duplication: Uses StateHierarchyManager infrastructure
    // Direct restoration prevents onentry re-execution (fixes event duplication bug)
    stateMachine_->restoreActiveStatesDirectly(snapshot.activeStates);
    LOG_DEBUG("InteractiveTestRunner: Restored {} active states directly", snapshot.activeStates.size());

    // W3C SCXML 3.13: Restore event queues to EventRaiser (Single Source of Truth)
    // Zero Duplication: UI events are included in externalQueue
    restoreEventQueues(snapshot.internalQueue, snapshot.externalQueue);

    // Note: snapshot.pendingUIEvents is now redundant (same as externalQueue)
    // EventRaiser's external queue already contains all UI events

    // W3C SCXML 6.2: Restore scheduled events state
    // Cancel all current scheduled events, then recreate snapshot events
    if (scheduler_ && eventDispatcher_) {
        // Cancel all current scheduled events
        auto currentScheduledEvents = scheduler_->getScheduledEvents();
        for (const auto &event : currentScheduledEvents) {
            scheduler_->cancelEvent(event.sendId);
            LOG_DEBUG("InteractiveTestRunner: Canceled scheduled event '{}' (sendId: {}) before restore",
                      event.eventName, event.sendId);
        }

        // Create target factory for event target recreation
        auto targetFactory = std::make_shared<EventTargetFactoryImpl>(eventRaiser_, scheduler_);

        // Recreate scheduled events from snapshot
        for (const auto &scheduledEvent : snapshot.scheduledEvents) {
            // Create complete EventDescriptor with all fields
            EventDescriptor eventDesc;
            eventDesc.eventName = scheduledEvent.eventName;
            eventDesc.target = scheduledEvent.targetUri;
            eventDesc.type = scheduledEvent.eventType;
            eventDesc.data = scheduledEvent.eventData;
            eventDesc.content = scheduledEvent.content;

            // Create target with correct URI (internal/external/http)
            auto target = targetFactory->createTarget(scheduledEvent.targetUri, scheduledEvent.sessionId);

            // Schedule event with original delay and sendId
            auto delay = std::chrono::milliseconds(scheduledEvent.originalDelayMs);
            scheduler_->scheduleEvent(eventDesc, delay, target, scheduledEvent.sendId, scheduledEvent.sessionId);

            LOG_DEBUG(
                "InteractiveTestRunner: Recreated scheduled event '{}' (sendId: {}, delay: {}ms, target: '{}') for "
                "step {}",
                scheduledEvent.eventName, scheduledEvent.sendId, scheduledEvent.originalDelayMs,
                scheduledEvent.targetUri, snapshot.stepNumber);
        }
    }

    // Restore metadata
    lastEventName_ = snapshot.lastEventName;
    lastTransitionSource_ = snapshot.lastTransitionSource;
    lastTransitionTarget_ = snapshot.lastTransitionTarget;

    // Restore execution history (for UI display, NOT for replay)
    executedEvents_ = snapshot.executedEvents;

    LOG_INFO("InteractiveTestRunner: State restored to step {} via direct restoration (no side effects)",
             snapshot.stepNumber);

    return true;
}

std::string InteractiveTestRunner::typeToString(SCE::Type type) {
    // W3C SCXML state types: atomic, compound, parallel, final, history, initial
    // Zero Duplication: Single implementation replaces 3 duplicate lambdas
    switch (type) {
    case SCE::Type::ATOMIC:
        return "atomic";
    case SCE::Type::COMPOUND:
        return "compound";
    case SCE::Type::PARALLEL:
        return "parallel";
    case SCE::Type::FINAL:
        return "final";
    case SCE::Type::HISTORY:
        return "history";
    case SCE::Type::INITIAL:
        return "initial";
    default:
        return "atomic";
    }
}

std::map<std::string, std::string> InteractiveTestRunner::extractDataModel() const {
    std::map<std::string, std::string> dataModel;

    // W3C SCXML 5.0: Extract data model variables from SCXMLModel
    // Single Source of Truth: Use model's dataModelItems_ (no memory duplication)
    if (!stateMachine_) {
        LOG_DEBUG("InteractiveTestRunner: No state machine available");
        return dataModel;
    }

    auto model = stateMachine_->getModel();
    if (!model) {
        LOG_DEBUG("InteractiveTestRunner: No SCXML model available");
        return dataModel;
    }

    // Get variable names from model (populated by SCXMLParser)
    auto variableNames = model->getDataModelVariableNames();

    if (variableNames.empty()) {
        LOG_DEBUG("InteractiveTestRunner: No data model variables defined");
        return dataModel;
    }

    // Access JSEngine singleton and session
    auto &jsEngine = JSEngine::instance();
    const std::string &sessionId = stateMachine_->getSessionId();

    // Extract each variable value from JSEngine
    for (const auto &varName : variableNames) {
        try {
            auto future = jsEngine.getVariable(sessionId, varName);
            auto result = future.get();

            if (result.isSuccess()) {
                dataModel[varName] = result.getValueAsString();
                LOG_DEBUG("InteractiveTestRunner: Extracted variable '{}' = '{}'", varName, dataModel[varName]);
            } else {
                LOG_WARN("InteractiveTestRunner: Failed to extract variable '{}': {}", varName,
                         result.getErrorMessage());
            }
        } catch (const std::exception &e) {
            LOG_ERROR("InteractiveTestRunner: Exception extracting variable '{}': {}", varName, e.what());
        }
    }

    LOG_DEBUG("InteractiveTestRunner: Extracted {} data model variables", dataModel.size());
    return dataModel;
}

void InteractiveTestRunner::restoreDataModel(const std::map<std::string, std::string> &dataModel) {
    auto &jsEngine = JSEngine::instance();
    const std::string &sessionId = stateMachine_->getSessionId();

    for (const auto &[varName, value] : dataModel) {
        // Assign variable value
        std::string assignment = varName + " = " + value + ";";
        jsEngine.evaluateExpression(sessionId, assignment);
    }
}

void InteractiveTestRunner::extractEventQueues(std::vector<EventSnapshot> &outInternal,
                                               std::vector<EventSnapshot> &outExternal) const {
    // W3C SCXML 3.13: Extract internal and external event queues from StateMachine
    auto eventRaiser = stateMachine_->getEventRaiser();
    if (!eventRaiser) {
        LOG_WARN("InteractiveTestRunner: No EventRaiser available for queue extraction");
        outInternal.clear();
        outExternal.clear();
        return;
    }

    eventRaiser->getEventQueues(outInternal, outExternal);
    LOG_DEBUG("InteractiveTestRunner: Extracted queues - internal: {}, external: {}", outInternal.size(),
              outExternal.size());
}

void InteractiveTestRunner::restoreEventQueues(const std::vector<EventSnapshot> &internal,
                                               const std::vector<EventSnapshot> &external) {
    // W3C SCXML 3.13: Restore internal and external event queues to StateMachine
    auto eventRaiser = stateMachine_->getEventRaiser();
    if (!eventRaiser) {
        LOG_WARN("InteractiveTestRunner: No EventRaiser available for queue restoration");
        return;
    }

    // Time-travel debugging: Clear existing queue before restoration
    // This prevents duplicate events from start()'s onentry execution
    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser);
    if (eventRaiserImpl) {
        eventRaiserImpl->clearQueue();
        LOG_DEBUG("InteractiveTestRunner: Cleared existing queue for clean restoration");
    }

    // Restore internal queue (higher priority)
    for (const auto &event : internal) {
        eventRaiser->raiseInternalEvent(event.name, event.data);
    }

    // Restore external queue (lower priority)
    for (const auto &event : external) {
        eventRaiser->raiseExternalEvent(event.name, event.data);
    }

    LOG_DEBUG("InteractiveTestRunner: Restored queues - internal: {}, external: {}", internal.size(), external.size());
}

#ifdef __EMSCRIPTEN__
emscripten::val InteractiveTestRunner::getLastTransition() const {
    auto obj = emscripten::val::object();

    if (!lastTransitionSource_.empty()) {
        obj.set("source", lastTransitionSource_);
        obj.set("target", lastTransitionTarget_);
        obj.set("event", lastEventName_);
        obj.set("id", lastTransitionSource_ + "_" + lastTransitionTarget_);
    }

    return obj;
}

emscripten::val InteractiveTestRunner::getEventQueue() const {
    auto obj = emscripten::val::object();

    auto internalArray = emscripten::val::array();
    auto externalArray = emscripten::val::array();

    // W3C SCXML 3.13: Get current event queues from StateMachine
    std::vector<EventSnapshot> internalQueue;
    std::vector<EventSnapshot> externalQueue;
    extractEventQueues(internalQueue, externalQueue);

    // Convert internal queue
    for (const auto &event : internalQueue) {
        auto eventObj = emscripten::val::object();
        eventObj.set("name", event.name);
        if (!event.data.empty()) {
            eventObj.set("data", event.data);
        }
        internalArray.call<void>("push", eventObj);
    }

    // W3C SCXML 3.13: All UI events and SCXML send events are in EventRaiser's external queue
    // Zero Duplication: No separate pendingEvents_ tracking needed
    for (const auto &event : externalQueue) {
        auto eventObj = emscripten::val::object();
        eventObj.set("name", event.name);
        if (!event.data.empty()) {
            eventObj.set("data", event.data);
        }
        externalArray.call<void>("push", eventObj);
    }

    obj.set("internal", internalArray);
    obj.set("external", externalArray);

    return obj;
}

emscripten::val InteractiveTestRunner::getScheduledEvents() const {
    auto array = emscripten::val::array();

    // Get scheduled events from scheduler (read-only, no engine impact)
    if (scheduler_) {
        auto scheduledEvents = scheduler_->getScheduledEvents();

        for (const auto &event : scheduledEvents) {
            auto eventObj = emscripten::val::object();
            eventObj.set("eventName", event.eventName);
            eventObj.set("sendId", event.sendId);
            eventObj.set("remainingTime", event.remainingTime.count());
            eventObj.set("sessionId", event.sessionId);
            array.call<void>("push", eventObj);
        }
    }

    return array;
}

emscripten::val InteractiveTestRunner::getDataModel() const {
    auto obj = emscripten::val::object();
    auto dataModel = extractDataModel();

    for (const auto &[varName, value] : dataModel) {
        obj.set(varName, value);
    }

    return obj;
}

emscripten::val InteractiveTestRunner::getSCXMLStructure() const {
    auto model = stateMachine_->getModel();

    if (!model) {
        return emscripten::val::object();
    }

    // W3C SCXML structure (Zero Duplication: delegate to buildStructureFromModel)
    return buildStructureFromModel(model);
}

emscripten::val InteractiveTestRunner::getW3CReferences() const {
    auto obj = emscripten::val::object();

    // W3C SCXML spec references are loaded by JavaScript (main.js:loadSpecReferences)
    // and stored in window.specReferences for access by execution-controller.js
    // This method returns empty object as references are managed client-side

    return obj;
}

bool InteractiveTestRunner::preloadFile(const std::string &filename, const std::string &content) {
    LOG_DEBUG("InteractiveTestRunner: Preloading file: {} ({} bytes)", filename, content.size());
    preloadedFiles_[filename] = content;
    return true;
}

void InteractiveTestRunner::setBasePath(const std::string &basePath) {
    basePath_ = basePath;

    // Register session file path for invoke resolution
    if (stateMachine_) {
        // Use a dummy filename to establish the base directory
        std::string sessionFilePath = basePath_ + "parent.scxml";
        stateMachine_->setSessionFilePath(sessionFilePath);
        LOG_DEBUG("InteractiveTestRunner: Base path set to: {} (session file: {})", basePath_, sessionFilePath);
    } else {
        LOG_DEBUG("InteractiveTestRunner: Base path set to: {} (will apply when state machine loads)", basePath_);
    }
}

emscripten::val InteractiveTestRunner::getInvokedChildren() const {
    auto obj = emscripten::val::object();
    auto childrenArray = emscripten::val::array();

    if (!stateMachine_) {
        obj.set("children", childrenArray);
        return obj;
    }

    // Get invoked child state machines
    auto children = stateMachine_->getInvokedChildren();

    LOG_DEBUG("InteractiveTestRunner: Found {} invoked children", children.size());

    // Zero Duplication: Use static typeToString method (removed duplicate lambda)

    // Build child information array
    for (const auto &child : children) {
        if (!child) {
            continue;
        }

        auto childObj = emscripten::val::object();

        // Basic child info
        childObj.set("sessionId", child->getSessionId());
        childObj.set("isInFinalState", child->isInFinalState());

        // Active states
        auto activeStatesArray = emscripten::val::array();
        auto activeStates = child->getActiveStates();
        for (const auto &state : activeStates) {
            activeStatesArray.call<void>("push", state);
        }
        childObj.set("activeStates", activeStatesArray);

        // W3C SCXML structure (Zero Duplication: delegate to buildStructureFromModel)
        auto model = child->getModel();
        if (model) {
            auto structure = buildStructureFromModel(model);
            childObj.set("structure", structure);
        }

        childrenArray.call<void>("push", childObj);
    }

    obj.set("children", childrenArray);
    return obj;
}

#else  // Non-WASM builds

std::string InteractiveTestRunner::getLastTransition() const {
    // JSON string for non-WASM testing
    return "{\"source\":\"" + lastTransitionSource_ + "\",\"target\":\"" + lastTransitionTarget_ + "\",\"event\":\"" +
           lastEventName_ + "\"}";
}

std::string InteractiveTestRunner::getScheduledEvents() const {
    // JSON array string for non-WASM testing
    std::string json = "[";

    if (scheduler_) {
        auto scheduledEvents = scheduler_->getScheduledEvents();
        bool first = true;

        for (const auto &event : scheduledEvents) {
            if (!first) {
                json += ",";
            }
            first = false;

            json += "{\"eventName\":\"" + event.eventName + "\"";
            json += ",\"sendId\":\"" + event.sendId + "\"";
            json += ",\"remainingTime\":" + std::to_string(event.remainingTime.count());
            json += ",\"sessionId\":\"" + event.sessionId + "\"}";
        }
    }

    json += "]";
    return json;
}

std::string InteractiveTestRunner::getDataModel() const {
    auto dataModel = extractDataModel();

    std::string json = "{";
    bool first = true;
    for (const auto &[varName, value] : dataModel) {
        if (!first) {
            json += ",";
        }
        json += "\"" + varName + "\":\"" + value + "\"";
        first = false;
    }
    json += "}";

    return json;
}

std::string InteractiveTestRunner::getSCXMLStructure() const {
    return "{\"states\":[],\"transitions\":[],\"initial\":\"\"}";
}

std::string InteractiveTestRunner::getW3CReferences() const {
    return "{}";
}

std::string InteractiveTestRunner::getInvokedChildren() const {
    return "{\"children\":[]}";
}

#endif  // __EMSCRIPTEN__

void InteractiveTestRunner::analyzeSubSCXML(std::shared_ptr<SCXMLModel> parentModel) {
    if (!parentModel) {
        return;
    }

    LOG_DEBUG("Analyzing parent SCXML for static invoke elements");

    // Create parser for loading sub-SCXML files
    auto nodeFactory = std::make_shared<NodeFactory>();
    auto parser = std::make_shared<SCXMLParser>(nodeFactory);

    // Iterate through all states to find invoke elements
    const auto &allStates = parentModel->getAllStates();

    for (const auto &state : allStates) {
        if (!state) {
            continue;
        }

        const auto &invokes = state->getInvoke();
        if (invokes.empty()) {
            continue;
        }

        for (const auto &invoke : invokes) {
            if (!invoke) {
                continue;
            }

            const std::string &src = invoke->getSrc();
            const std::string &srcExpr = invoke->getSrcExpr();

            // Only process static src (skip dynamic srcexpr)
            if (src.empty() || !srcExpr.empty()) {
                continue;
            }

            // Resolve file path
            std::string fullPath = src;
            if (src.find("file:") == 0) {
                fullPath = src.substr(5);  // Remove "file:" prefix
            }

            // Make absolute if relative
            if (!fullPath.empty() && fullPath[0] != '/') {
                fullPath = basePath_ + fullPath;
            }

            LOG_DEBUG("  Attempting to load sub-SCXML: {}", fullPath);

            // Parse child SCXML file
            auto childModel = parser->parseFile(fullPath);

            if (!childModel) {
                LOG_WARN("  Failed to parse sub-SCXML '{}' - skipping visualization", fullPath);
                continue;
            }

            // Build structure object for JavaScript
            SubSCXMLInfo info;
            info.parentStateId = state->getId();
            info.invokeId =
                invoke->getId().empty() ? ("invoke_" + std::to_string(subScxmlStructures_.size())) : invoke->getId();
            info.srcPath = fullPath;
#ifdef __EMSCRIPTEN__
            info.structure = buildStructureFromModel(childModel);
#endif

            subScxmlStructures_.push_back(info);
            LOG_DEBUG("  Successfully loaded sub-SCXML: {} (from state '{}')", fullPath, state->getId());
        }
    }

    LOG_DEBUG("Static analysis complete: found {} sub-SCXML file(s)", subScxmlStructures_.size());
}

#ifdef __EMSCRIPTEN__
emscripten::val
InteractiveTestRunner::serializeActions(const std::vector<std::shared_ptr<IActionNode>> &actions) const {
    auto actionsArray = emscripten::val::array();

    LOG_DEBUG("serializeActions: Serializing {} action(s)", actions.size());

    for (const auto &action : actions) {
        if (!action) {
            continue;
        }

        auto actionObj = emscripten::val::object();
        const auto &actionType = action->getActionType();
        const auto &actionId = action->getId();

        actionObj.set("actionType", actionType);
        actionObj.set("id", actionId);

        LOG_DEBUG("serializeActions: Processing action type='{}', id='{}'", actionType, actionId);

        // W3C SCXML 3.7: Serialize action-specific properties
        if (actionType == "assign") {
            auto assign = std::dynamic_pointer_cast<AssignAction>(action);
            if (assign) {
                actionObj.set("location", assign->getLocation());
                actionObj.set("expr", assign->getExpr());
                if (!assign->getType().empty()) {
                    actionObj.set("type", assign->getType());
                }
            }
        } else if (actionType == "raise") {
            auto raise = std::dynamic_pointer_cast<RaiseAction>(action);
            if (raise) {
                actionObj.set("event", raise->getEvent());
                if (!raise->getData().empty()) {
                    actionObj.set("data", raise->getData());
                }
            }
        } else if (actionType == "foreach") {
            auto foreach = std::dynamic_pointer_cast<ForeachAction>(action);
            if (foreach) {
                actionObj.set("array", foreach->getArray());
                actionObj.set("item", foreach->getItem());
                if (!foreach->getIndex().empty()) {
                    actionObj.set("index", foreach->getIndex());
                }
                // Recursive: serialize nested iteration actions
                auto nestedActions = serializeActions(foreach->getIterationActions());
                actionObj.set("iterationActions", nestedActions);
            }
        } else if (actionType == "log") {
            auto log = std::dynamic_pointer_cast<LogAction>(action);
            if (log) {
                if (!log->getExpr().empty()) {
                    actionObj.set("expr", log->getExpr());
                }
                if (!log->getLabel().empty()) {
                    actionObj.set("label", log->getLabel());
                }
                if (!log->getLevel().empty()) {
                    actionObj.set("level", log->getLevel());
                }
            }
        } else if (actionType == "if") {
            auto ifAction = std::dynamic_pointer_cast<IfAction>(action);
            if (ifAction) {
                const auto &branches = ifAction->getBranches();
                if (!branches.empty()) {
                    // W3C SCXML 3.12.1: Serialize if condition
                    actionObj.set("cond", branches[0].condition);

                    // Serialize all branches for complete visualization
                    auto branchesArray = emscripten::val::array();
                    for (const auto &branch : branches) {
                        auto branchObj = emscripten::val::object();
                        branchObj.set("condition", branch.condition);
                        branchObj.set("isElse", branch.isElseBranch);
                        branchObj.set("actions", serializeActions(branch.actions));
                        branchesArray.call<void>("push", branchObj);
                    }
                    actionObj.set("branches", branchesArray);
                }
            }
        } else if (actionType == "send") {
            auto send = std::dynamic_pointer_cast<SendAction>(action);
            if (send) {
                // W3C SCXML 6.2: Serialize send attributes
                if (!send->getEvent().empty()) {
                    actionObj.set("event", send->getEvent());
                }
                if (!send->getEventExpr().empty()) {
                    actionObj.set("eventexpr", send->getEventExpr());
                }
                if (!send->getTarget().empty()) {
                    actionObj.set("target", send->getTarget());
                }
                if (!send->getTargetExpr().empty()) {
                    actionObj.set("targetexpr", send->getTargetExpr());
                }
                if (!send->getDelay().empty()) {
                    actionObj.set("delay", send->getDelay());
                }
                if (!send->getDelayExpr().empty()) {
                    actionObj.set("delayexpr", send->getDelayExpr());
                }
                if (!send->getData().empty()) {
                    actionObj.set("data", send->getData());
                }
                if (!send->getContent().empty()) {
                    actionObj.set("content", send->getContent());
                }
                if (!send->getContentExpr().empty()) {
                    actionObj.set("contentexpr", send->getContentExpr());
                }
                if (!send->getSendId().empty()) {
                    actionObj.set("sendid", send->getSendId());
                }
                if (!send->getIdLocation().empty()) {
                    actionObj.set("idlocation", send->getIdLocation());
                }
                if (!send->getType().empty()) {
                    actionObj.set("type", send->getType());
                }
                if (!send->getTypeExpr().empty()) {
                    actionObj.set("typeexpr", send->getTypeExpr());
                }
                if (!send->getNamelist().empty()) {
                    actionObj.set("namelist", send->getNamelist());
                }
                // W3C SCXML C.1: Serialize params
                const auto &params = send->getParamsWithExpr();
                if (!params.empty()) {
                    auto paramsArray = emscripten::val::array();
                    for (const auto &param : params) {
                        auto paramObj = emscripten::val::object();
                        paramObj.set("name", param.name);
                        paramObj.set("expr", param.expr);
                        paramsArray.call<void>("push", paramObj);
                    }
                    actionObj.set("params", paramsArray);
                }
            }
        } else if (actionType == "cancel") {
            auto cancel = std::dynamic_pointer_cast<CancelAction>(action);
            if (cancel) {
                // W3C SCXML 6.3: Serialize cancel attributes
                if (!cancel->getSendId().empty()) {
                    actionObj.set("sendid", cancel->getSendId());
                }
                if (!cancel->getSendIdExpr().empty()) {
                    actionObj.set("sendidexpr", cancel->getSendIdExpr());
                }
            }
        } else if (actionType == "script") {
            auto script = std::dynamic_pointer_cast<ScriptAction>(action);
            if (script) {
                // W3C SCXML 5.9: Serialize script content
                if (!script->getContent().empty()) {
                    actionObj.set("content", script->getContent());
                }
            }
        }

        actionsArray.call<void>("push", actionObj);
    }

    return actionsArray;
}

emscripten::val InteractiveTestRunner::buildStructureFromModel(std::shared_ptr<SCXMLModel> model) const {
    auto obj = emscripten::val::object();

    if (!model) {
        return obj;
    }

    // Zero Duplication: Use static typeToString method

    // Build states array (deduplicate by ID)
    auto statesArray = emscripten::val::array();
    const auto &allStates = model->getAllStates();
    std::set<std::string> seenStateIds;

    for (const auto &state : allStates) {
        if (!state) {
            continue;
        }

        const auto &stateId = state->getId();

        // Skip duplicate state IDs
        if (seenStateIds.find(stateId) != seenStateIds.end()) {
            continue;
        }
        seenStateIds.insert(stateId);

        auto stateObj = emscripten::val::object();
        stateObj.set("id", stateId);
        stateObj.set("type", typeToString(state->getType()));

        // W3C SCXML 3.10: Extract history state type (shallow/deep)
        if (state->getType() == SCE::Type::HISTORY) {
            stateObj.set("historyType", state->isDeepHistory() ? "deep" : "shallow");
        }

        // W3C SCXML 3.2: Extract child state IDs for hierarchical visualization
        // Used for parent-child containment links (Option 1)
        const auto &childStates = state->getChildren();
        if (!childStates.empty()) {
            auto childrenArray = emscripten::val::array();
            for (const auto &child : childStates) {
                childrenArray.call<void>("push", emscripten::val(child->getId()));
            }
            stateObj.set("children", childrenArray);
        }

        // W3C SCXML 3.7: Extract onentry action blocks (Priority 1: assign, raise, foreach, log)
        const auto &entryBlocks = state->getEntryActionBlocks();
        std::vector<std::shared_ptr<IActionNode>> flatEntryActions;
        for (const auto &block : entryBlocks) {
            flatEntryActions.insert(flatEntryActions.end(), block.begin(), block.end());
        }
        LOG_DEBUG("buildStructureFromModel: State '{}' has {} onentry action(s)", stateId, flatEntryActions.size());
        if (!flatEntryActions.empty()) {
            stateObj.set("onentry", serializeActions(flatEntryActions));
        }

        // W3C SCXML 3.7: Extract onexit action blocks (Priority 1: assign, raise, foreach, log)
        const auto &exitBlocks = state->getExitActionBlocks();
        std::vector<std::shared_ptr<IActionNode>> flatExitActions;
        for (const auto &block : exitBlocks) {
            flatExitActions.insert(flatExitActions.end(), block.begin(), block.end());
        }
        if (!flatExitActions.empty()) {
            stateObj.set("onexit", serializeActions(flatExitActions));
        }

        statesArray.call<void>("push", stateObj);
    }

    // Build transitions array - prevent duplicate state processing
    auto transitionsArray = emscripten::val::array();
    int transitionId = 0;
    std::set<std::string> processedStateIds;  // Track processed states for transitions

    for (const auto &state : allStates) {
        if (!state) {
            continue;
        }

        const auto &stateId = state->getId();

        // Skip duplicate state IDs to prevent duplicate transitions
        if (processedStateIds.find(stateId) != processedStateIds.end()) {
            LOG_DEBUG("buildStructureFromModel: Skipping duplicate state '{}' for transition extraction", stateId);
            continue;
        }
        processedStateIds.insert(stateId);

        const auto &transitions = state->getTransitions();
        LOG_DEBUG("buildStructureFromModel: State '{}' has {} transition(s)", stateId, transitions.size());
        for (const auto &transition : transitions) {
            if (!transition) {
                continue;
            }

            const auto &events = transition->getEvents();
            const auto &targets = transition->getTargets();

            // W3C SCXML 3.12.1: Extract guard condition for visualization
            const auto &guard = transition->getGuard();

            // W3C SCXML 3.7: Extract transition actions for visualization
            const auto &actionNodes = transition->getActionNodes();
            auto actionsArray = serializeActions(actionNodes);

            // W3C SCXML 3.13: Handle eventless transitions
            if (events.empty()) {
                for (const auto &target : targets) {
                    auto transObj = emscripten::val::object();
                    transObj.set("id", std::to_string(transitionId++));
                    transObj.set("source", stateId);
                    transObj.set("target", target);
                    transObj.set("event", "");        // Empty string for eventless transitions
                    transObj.set("eventless", true);  // W3C SCXML 3.13: Flag for eventless transition
                    if (!guard.empty()) {
                        transObj.set("cond", guard);  // W3C SCXML 3.12.1: Guard condition for visualization
                    }
                    if (actionsArray["length"].as<int>() > 0) {
                        transObj.set("actions", actionsArray);  // W3C SCXML 3.7: Transition actions
                    }

                    LOG_DEBUG("  → Adding eventless transition: {} → {} (id={})", stateId, target, transitionId - 1);
                    transitionsArray.call<void>("push", transObj);
                }
            } else {
                // Create one transition object per event-target combination
                for (const auto &event : events) {
                    for (const auto &target : targets) {
                        auto transObj = emscripten::val::object();
                        transObj.set("id", std::to_string(transitionId++));
                        transObj.set("source", stateId);
                        transObj.set("target", target);
                        transObj.set("event", event);
                        transObj.set("eventless", false);  // W3C SCXML 3.13: Not eventless
                        if (!guard.empty()) {
                            transObj.set("cond", guard);  // W3C SCXML 3.12.1: Guard condition for visualization
                        }
                        if (actionsArray["length"].as<int>() > 0) {
                            transObj.set("actions", actionsArray);  // W3C SCXML 3.7: Transition actions
                        }

                        LOG_DEBUG("  → Adding event transition: {} → {} event='{}' (id={})", stateId, target, event,
                                  transitionId - 1);
                        transitionsArray.call<void>("push", transObj);
                    }
                }
            }
        }
    }

    LOG_DEBUG("buildStructureFromModel: Total {} transitions extracted", transitionId);
    obj.set("states", statesArray);
    obj.set("transitions", transitionsArray);
    obj.set("initial", model->getInitialState());

    return obj;
}

emscripten::val InteractiveTestRunner::getSubSCXMLStructures() const {
    auto result = emscripten::val::array();

    for (const auto &info : subScxmlStructures_) {
        auto obj = emscripten::val::object();
        obj.set("parentStateId", info.parentStateId);
        obj.set("invokeId", info.invokeId);
        obj.set("srcPath", info.srcPath);
        obj.set("structure", info.structure);
        result.call<void>("push", obj);
    }

    LOG_DEBUG("Returning {} sub-SCXML structures to JavaScript", subScxmlStructures_.size());
    return result;
}
#endif

}  // namespace SCE::W3C
