// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "InteractiveTestRunner.h"

#include "common/Logger.h"
#include "model/SCXMLModel.h"
#include "scripting/JSEngine.h"

#include <set>

namespace SCE::W3C {

InteractiveTestRunner::InteractiveTestRunner()
    : stateMachine_(std::make_shared<StateMachine>()), snapshotManager_(1000)  // 1000 step history
      ,
      currentStep_(0) {}

InteractiveTestRunner::~InteractiveTestRunner() {
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

    // Build transitions array (deduplicate by source-target-event)
    auto transitionsArray = emscripten::val::array();
    int transitionId = 0;
    std::set<std::string> seenTransitions;

    LOG_DEBUG("getSCXMLStructure: Building transitions");

    for (const auto &state : allStates) {
        if (!state) {
            continue;
        }

        const auto &stateId = state->getId();

        // Skip already processed states (for deduplication)
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

            // Create one transition object per event-target combination
            for (const auto &event : events) {
                for (const auto &target : targets) {
                    // Create unique key for deduplication
                    std::string transKey = stateId + "|" + target + "|" + event;

                    if (seenTransitions.find(transKey) != seenTransitions.end()) {
                        LOG_DEBUG("  Skipping duplicate transition: {} -> {} [{}]", stateId, target, event);
                        continue;
                    }
                    seenTransitions.insert(transKey);

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

#endif  // __EMSCRIPTEN__

}  // namespace SCE::W3C
