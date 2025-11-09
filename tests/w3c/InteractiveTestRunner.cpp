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
    if (!stateMachine_->start()) {
        LOG_ERROR("InteractiveTestRunner: Failed to start state machine");
        return false;
    }

    // Capture initial snapshot for true reset functionality
    currentStep_ = 0;
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

    // W3C SCXML 3.13: Process pending event or check eventless transitions
    StateMachine::TransitionResult result;

    if (!pendingEvents_.empty()) {
        // W3C SCXML 3.13: Microstep = dequeue event + process transitions
        // Event is consumed regardless of whether transition succeeds
        auto event = pendingEvents_.front();
        pendingEvents_.pop();

        result = stateMachine_->processEvent(event.name, event.data);
        lastEventName_ = event.name;

        // W3C SCXML 3.13: Record processed event to history for accurate restoration
        executedEvents_.push_back(event);

        // Update transition metadata
        if (result.success) {
            // Transition occurred: update source and target
            lastTransitionSource_ = result.fromState;
            lastTransitionTarget_ = result.toState;
        } else {
            // No transition occurred: clear transition metadata to prevent animation
            lastTransitionSource_ = "";
            lastTransitionTarget_ = "";
        }

        // Microstep occurred: event was dequeued and processed
        currentStep_++;
        captureSnapshot();

        LOG_DEBUG("InteractiveTestRunner: Step {} - event '{}' processed (transition: {}, remaining queue: {})",
                  currentStep_, event.name, result.success ? "success" : "none", pendingEvents_.size());
        return true;

    } else {
        // W3C SCXML 3.13: Check for eventless transitions (only if queue is empty)
        result = stateMachine_->processEvent("");  // Null event triggers eventless transitions
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
        currentStep_ = 0;
        LOG_DEBUG("InteractiveTestRunner: Reset to true initial configuration (queue cleared)");
    } else {
        LOG_ERROR("InteractiveTestRunner: Failed to restore initial snapshot");
    }
}

void InteractiveTestRunner::raiseEvent(const std::string &eventName, const std::string &eventData) {
    pendingEvents_.push(EventSnapshot(eventName, eventData));

    // W3C SCXML 3.13: Event queuing is NOT a microstep
    // Step only increments when event is actually processed in stepForward()
    // However, we must capture snapshot to preserve queue state for time-travel debugging
    captureSnapshot();

    LOG_DEBUG("InteractiveTestRunner: Queued event '{}' (queue size: {}, current step: {})", eventName,
              pendingEvents_.size(), currentStep_);
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

    std::vector<EventSnapshot> internalQueue;
    std::vector<EventSnapshot> externalQueue;
    extractEventQueues(internalQueue, externalQueue);

    // Convert InteractiveTestRunner's pending events queue to vector
    std::vector<EventSnapshot> pendingUIEvents;
    auto pendingQueueCopy = pendingEvents_;
    while (!pendingQueueCopy.empty()) {
        pendingUIEvents.push_back(pendingQueueCopy.front());
        pendingQueueCopy.pop();
    }

    // Convert std::vector<std::string> to std::set<std::string> for StateSnapshot
    std::set<std::string> activeStatesSet(activeStates.begin(), activeStates.end());

    snapshotManager_.captureSnapshot(activeStatesSet, dataModel, internalQueue, externalQueue, pendingUIEvents,
                                     executedEvents_, currentStep_, lastEventName_, lastTransitionSource_,
                                     lastTransitionTarget_);
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

    if (!stateMachine_->start()) {
        LOG_ERROR("InteractiveTestRunner: Failed to restart state machine");
        return false;
    }

    // Restore data model
    restoreDataModel(snapshot.dataModel);

    // Restore event queues (StateMachine internal queues)
    restoreEventQueues(snapshot.internalQueue, snapshot.externalQueue);

    // Restore InteractiveTestRunner's pending UI events
    std::queue<EventSnapshot> restoredQueue;
    for (const auto &event : snapshot.pendingUIEvents) {
        restoredQueue.push(event);
    }
    pendingEvents_ = std::move(restoredQueue);

    // Restore metadata
    lastEventName_ = snapshot.lastEventName;
    lastTransitionSource_ = snapshot.lastTransitionSource;
    lastTransitionTarget_ = snapshot.lastTransitionTarget;

    // W3C SCXML 3.13: Restore active states by replaying event history
    // Replay all processed events to restore exact state
    LOG_DEBUG("InteractiveTestRunner: Replaying {} events to restore state", snapshot.executedEvents.size());

    for (const auto &event : snapshot.executedEvents) {
        stateMachine_->processEvent(event.name, event.data);
    }

    // Restore execution history
    executedEvents_ = snapshot.executedEvents;

    LOG_DEBUG("InteractiveTestRunner: State restored to step {} via event replay", snapshot.stepNumber);

    return true;
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

    // Convert InteractiveTestRunner's pending events (UI-added events)
    auto pendingQueueCopy = pendingEvents_;  // Copy queue for iteration
    while (!pendingQueueCopy.empty()) {
        const auto &event = pendingQueueCopy.front();
        auto eventObj = emscripten::val::object();
        eventObj.set("name", event.name);
        if (!event.data.empty()) {
            eventObj.set("data", event.data);
        }
        externalArray.call<void>("push", eventObj);
        pendingQueueCopy.pop();
    }

    // Add StateMachine's external events (from SCXML execution)
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

emscripten::val InteractiveTestRunner::getDataModel() const {
    auto obj = emscripten::val::object();
    auto dataModel = extractDataModel();

    for (const auto &[varName, value] : dataModel) {
        obj.set(varName, value);
    }

    return obj;
}

emscripten::val InteractiveTestRunner::getSCXMLStructure() const {
    auto obj = emscripten::val::object();
    auto model = stateMachine_->getModel();

    if (!model) {
        return obj;
    }

    // Helper function to convert Type enum to string
    auto typeToString = [](SCE::Type type) -> std::string {
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
    };

    // Build states array (deduplicate by ID)
    auto statesArray = emscripten::val::array();
    const auto &allStates = model->getAllStates();
    std::set<std::string> seenStateIds;

    LOG_DEBUG("getSCXMLStructure: Processing {} states from model", allStates.size());

    for (size_t i = 0; i < allStates.size(); i++) {
        const auto &state = allStates[i];
        if (!state) {
            continue;
        }

        const auto &stateId = state->getId();

        // Skip duplicate state IDs
        if (seenStateIds.find(stateId) != seenStateIds.end()) {
            LOG_DEBUG("  Skipping duplicate state: '{}'", stateId);
            continue;
        }
        seenStateIds.insert(stateId);

        LOG_DEBUG("  Adding state: id='{}', type={}", stateId, typeToString(state->getType()));

        auto stateObj = emscripten::val::object();
        stateObj.set("id", stateId);
        stateObj.set("type", typeToString(state->getType()));

        statesArray.call<void>("push", stateObj);
    }

    // Build transitions array (all transitions, no deduplication for accurate SCXML visualization)
    auto transitionsArray = emscripten::val::array();
    int transitionId = 0;

    LOG_DEBUG("getSCXMLStructure: Building transitions");

    for (const auto &state : allStates) {
        if (!state) {
            continue;
        }

        const auto &stateId = state->getId();

        // Skip already processed states (for state deduplication)
        if (seenStateIds.find(stateId) == seenStateIds.end()) {
            continue;
        }

        const auto &transitions = state->getTransitions();
        for (const auto &transition : transitions) {
            if (!transition) {
                continue;
            }

            const auto &events = transition->getEvents();
            const auto &targets = transition->getTargets();

            // W3C SCXML: Handle eventless transitions
            if (events.empty()) {
                for (const auto &target : targets) {
                    LOG_DEBUG("  Adding eventless transition: {} -> {}", stateId, target);

                    auto transObj = emscripten::val::object();
                    transObj.set("id", std::to_string(transitionId++));
                    transObj.set("source", stateId);
                    transObj.set("target", target);
                    transObj.set("event", "");  // Empty string for eventless transitions

                    transitionsArray.call<void>("push", transObj);
                }
            } else {
                // Create one transition object per event-target combination
                for (const auto &event : events) {
                    for (const auto &target : targets) {
                        LOG_DEBUG("  Adding transition: {} -> {} [{}]", stateId, target, event);

                        auto transObj = emscripten::val::object();
                        transObj.set("id", std::to_string(transitionId++));
                        transObj.set("source", stateId);
                        transObj.set("target", target);
                        transObj.set("event", event);

                        transitionsArray.call<void>("push", transObj);
                    }
                }
            }
        }
    }

    obj.set("states", statesArray);
    obj.set("transitions", transitionsArray);
    obj.set("initial", model->getInitialState());

    return obj;
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

    // Helper function to convert Type enum to string (from getSCXMLStructure)
    auto typeToString = [](SCE::Type type) -> std::string {
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
    };

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

        // SCXML structure (reuse logic from getSCXMLStructure)
        auto model = child->getModel();
        if (model) {
            auto structure = emscripten::val::object();
            auto statesArray = emscripten::val::array();
            auto transitionsArray = emscripten::val::array();

            const auto &allStates = model->getAllStates();
            std::set<std::string> seenStateIds;

            // Build states array
            for (size_t i = 0; i < allStates.size(); i++) {
                const auto &state = allStates[i];
                if (!state) {
                    continue;
                }

                const auto &stateId = state->getId();
                if (seenStateIds.find(stateId) != seenStateIds.end()) {
                    continue;
                }
                seenStateIds.insert(stateId);

                auto stateObj = emscripten::val::object();
                stateObj.set("id", stateId);
                stateObj.set("type", typeToString(state->getType()));

                statesArray.call<void>("push", stateObj);
            }

            // Build transitions array (all transitions, no deduplication)
            int transitionId = 0;

            for (const auto &state : allStates) {
                if (!state) {
                    continue;
                }

                const auto &stateId = state->getId();
                if (seenStateIds.find(stateId) == seenStateIds.end()) {
                    continue;
                }

                const auto &transitions = state->getTransitions();
                for (const auto &transition : transitions) {
                    if (!transition) {
                        continue;
                    }

                    const auto &events = transition->getEvents();
                    const auto &targets = transition->getTargets();

                    // W3C SCXML: Handle eventless transitions
                    if (events.empty()) {
                        for (const auto &target : targets) {
                            auto transObj = emscripten::val::object();
                            transObj.set("id", std::to_string(transitionId++));
                            transObj.set("source", stateId);
                            transObj.set("target", target);
                            transObj.set("event", "");  // Empty string for eventless transitions

                            transitionsArray.call<void>("push", transObj);
                        }
                    } else {
                        for (const auto &event : events) {
                            for (const auto &target : targets) {
                                auto transObj = emscripten::val::object();
                                transObj.set("id", std::to_string(transitionId++));
                                transObj.set("source", stateId);
                                transObj.set("target", target);
                                transObj.set("event", event);

                                transitionsArray.call<void>("push", transObj);
                            }
                        }
                    }
                }
            }

            structure.set("states", statesArray);
            structure.set("transitions", transitionsArray);
            structure.set("initial", model->getInitialState());

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
emscripten::val InteractiveTestRunner::buildStructureFromModel(std::shared_ptr<SCXMLModel> model) const {
    auto obj = emscripten::val::object();

    if (!model) {
        return obj;
    }

    // Helper function to convert Type enum to string
    auto typeToString = [](SCE::Type type) -> std::string {
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
    };

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

        statesArray.call<void>("push", stateObj);
    }

    // Build transitions array (all transitions, no deduplication for accurate SCXML visualization)
    auto transitionsArray = emscripten::val::array();
    int transitionId = 0;

    for (const auto &state : allStates) {
        if (!state) {
            continue;
        }

        const auto &stateId = state->getId();

        const auto &transitions = state->getTransitions();
        for (const auto &transition : transitions) {
            if (!transition) {
                continue;
            }

            const auto &events = transition->getEvents();
            const auto &targets = transition->getTargets();

            // W3C SCXML: Handle eventless transitions
            if (events.empty()) {
                for (const auto &target : targets) {
                    auto transObj = emscripten::val::object();
                    transObj.set("id", std::to_string(transitionId++));
                    transObj.set("source", stateId);
                    transObj.set("target", target);
                    transObj.set("event", "");  // Empty string for eventless transitions

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

                        transitionsArray.call<void>("push", transObj);
                    }
                }
            }
        }
    }

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
