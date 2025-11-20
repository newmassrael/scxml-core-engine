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
#include <unordered_set>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

namespace SCE::W3C {

InteractiveTestRunner::InteractiveTestRunner()
    : stateMachine_(std::make_shared<StateMachine>()), snapshotManager_(1000)  // 1000 step history
      ,
      currentStep_(0) {
    // Initialize logger with Debug level by default (can be changed via setSpdlogLevel)
    Logger::setLevel(LogLevel::Debug);

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

    // W3C SCXML 3.13: Scheduler always polls automatically (timeout → queue)
    // Queue processing is controlled by EventRaiser immediate mode (disabled below)

    // Create event raiser (concrete type for setScheduler access)
    auto eventRaiser = std::make_shared<EventRaiserImpl>();
    eventRaiser_ = eventRaiser;

    // Connect EventRaiser to EventScheduler for delayed event polling
    eventRaiser->setScheduler(scheduler_);

    // W3C SCXML 3.13: Disable immediate mode for interactive debugging
    // Events must be queued and only processed when stepForward() is called
    // This prevents automatic event processing when scheduler moves events to queue
    eventRaiser->setImmediateMode(false);

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
    // Preserve initial transition executed by start() for UI display
    // start() already ran eventless transitions - don't clear(), read from StateMachine
    lastTransitionSource_ = stateMachine_->getLastTransitionSource();
    lastTransitionTarget_ = stateMachine_->getLastTransitionTarget();
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

StepResult InteractiveTestRunner::stepForward() {
    // W3C SCXML 3.3.1: Check if state machine reached final state and stopped
    // Final states are defined by W3C SCXML, stopping behavior is implementation-specific
    if (isInFinalState() || !stateMachine_->isRunning()) {
        LOG_DEBUG("InteractiveTestRunner: State machine in final state or stopped - cannot step forward");
        return StepResult::FINAL_STATE;
    }

    // REPLAY MODE: Cache-first strategy for deterministic stepping
    auto replayResult = attemptReplayFromCache();
    if (replayResult != StepResult::NO_EVENTS_AVAILABLE) {
        return replayResult;
    }

    // NEW EXECUTION MODE: Execute transition and capture new snapshot
    LOG_DEBUG("InteractiveTestRunner: NEW EXECUTION MODE - No cached snapshot, executing transition");

    pollSchedulerIfNeeded();
    capturePreTransitionStates();

    // W3C SCXML 3.13: Try event processing first (queued → eventless → scheduled)
    auto queuedResult = processQueuedEventStep();
    if (queuedResult != StepResult::NO_EVENTS_AVAILABLE) {
        return queuedResult;
    }

    auto eventlessResult = processEventlessTransitionStep();
    if (eventlessResult != StepResult::NO_EVENTS_AVAILABLE) {
        return eventlessResult;
    }

    return checkScheduledEventsStatus();
}

StepResult InteractiveTestRunner::attemptReplayFromCache() {
    // REPLAY MODE: If snapshot exists for next step, restore it instead of re-executing
    // This ensures step back → step forward produces identical results
    auto nextSnapshot = snapshotManager_.getSnapshot(currentStep_ + 1);
    if (!nextSnapshot.has_value()) {
        return StepResult::NO_EVENTS_AVAILABLE;  // No cache, proceed to NEW EXECUTION
    }

    LOG_DEBUG("InteractiveTestRunner: REPLAY MODE - Found existing snapshot for step {}", currentStep_ + 1);

    if (restoreSnapshot(*nextSnapshot)) {
        currentStep_++;
        LOG_INFO("InteractiveTestRunner: REPLAY MODE - Restored to step {} from cache (no side effects)", currentStep_);
        return StepResult::SUCCESS;
    } else {
        LOG_ERROR("InteractiveTestRunner: REPLAY MODE - Failed to restore snapshot for step {}", currentStep_ + 1);
        return StepResult::NO_EVENTS_AVAILABLE;  // Fall through to NEW EXECUTION as recovery
    }
}

void InteractiveTestRunner::pollSchedulerIfNeeded() {
#ifdef __EMSCRIPTEN__
    // W3C SCXML 6.2: Poll scheduled events from delayed <send> operations
    // W3C SCXML 3.13: Scheduler always polls (timeout → queue)
    // WASM only: Manual polling required (no timer thread)
    // Native: Timer thread handles scheduled events automatically
    if (scheduler_) {
        auto schedulerImpl = std::dynamic_pointer_cast<EventSchedulerImpl>(scheduler_);
        if (schedulerImpl) {
            size_t polledCount = schedulerImpl->poll();
            if (polledCount > 0) {
                LOG_DEBUG("InteractiveTestRunner: Polled {} scheduled events into queue", polledCount);
            }
        }
    }
#endif
}

void InteractiveTestRunner::capturePreTransitionStates() {
    // Capture current active states BEFORE transition (for source detection)
    auto preTransitionStates = stateMachine_->getActiveStates();
    previousActiveStates_.clear();
    for (const auto &state : preTransitionStates) {
        previousActiveStates_.insert(state);
    }
}

StepResult InteractiveTestRunner::processQueuedEventStep() {
    // W3C SCXML 3.13: Increment step FIRST, then capture snapshot (pre-transition state)
    // This ensures snapshot is indexed with the correct step number for stepBackward() retrieval
    currentStep_++;
    captureSnapshot();

    // Process next queued event with priority: INTERNAL → EXTERNAL (W3C SCXML Appendix D)
    // Zero Duplication: Single priority queue with QueuedEventComparator
    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
    if (!eventRaiserImpl || !eventRaiserImpl->processNextQueuedEvent()) {
        // Rollback snapshot if no event was processed
        currentStep_--;
        return StepResult::NO_EVENTS_AVAILABLE;
    }

    // W3C SCXML 3.13: Event processed, record to execution history for snapshot restoration
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

    // Get last executed transition from StateMachine
    // ARCHITECTURE.md Zero Duplication: StateMachine tracks all transitions (including eventless)
    lastTransitionSource_ = stateMachine_->getLastTransitionSource();
    lastTransitionTarget_ = stateMachine_->getLastTransitionTarget();
    LOG_DEBUG("Interactive visualizer read transition: {} -> {}", lastTransitionSource_, lastTransitionTarget_);

    LOG_DEBUG("InteractiveTestRunner: Step {} - event '{}' processed (transition: {} -> {})", currentStep_,
              lastEventName_, lastTransitionSource_, lastTransitionTarget_);
    return StepResult::SUCCESS;
}

StepResult InteractiveTestRunner::processEventlessTransitionStep() {
    // W3C SCXML 3.13: Increment step FIRST, then capture snapshot (pre-transition state)
    currentStep_++;
    captureSnapshot();

    // Check for eventless transitions (W3C SCXML 3.13)
    auto result = stateMachine_->processEvent("");  // Null event triggers eventless transitions
    lastEventName_ = "";

    if (!result.success) {
        // Rollback snapshot if no transition occurred
        currentStep_--;
        return StepResult::NO_EVENTS_AVAILABLE;
    }

    // W3C SCXML 3.13: Record eventless transition to execution history
    executedEvents_.push_back(EventSnapshot("", ""));

    lastTransitionSource_ = result.fromState;
    lastTransitionTarget_ = result.toState;

    LOG_DEBUG("InteractiveTestRunner: Step {} - eventless transition: {} -> {}", currentStep_, lastTransitionSource_,
              lastTransitionTarget_);
    return StepResult::SUCCESS;
}

StepResult InteractiveTestRunner::checkScheduledEventsStatus() {
    // Check if scheduled events from <send delay="..."> are waiting (W3C SCXML 6.2.4)
    // If yes, return NO_EVENTS_READY so UI can disable button and wait
    if (hasScheduledEvents()) {
        int nextEventTime = getNextScheduledEventTime();
        LOG_DEBUG("InteractiveTestRunner: No events ready - waiting for scheduled event in {}ms", nextEventTime);
        return StepResult::NO_EVENTS_READY;
    }

    LOG_DEBUG("InteractiveTestRunner: No events in queue, no scheduled events - stuck state");
    return StepResult::NO_EVENTS_AVAILABLE;
}

bool InteractiveTestRunner::stepBackward() {
    if (currentStep_ <= 0) {
        LOG_DEBUG("InteractiveTestRunner: Already at initial state, cannot step backward");
        return false;
    }

    // Get previous snapshot
    // W3C SCXML 3.13: Snapshots are captured BEFORE transitions
    // Snapshot N = state BEFORE step N's transition
    // If currentStep=2 (just executed step 2), go back to state before step 2 = snapshot[2]
    int requestedStep = currentStep_;
    LOG_DEBUG("[SNAPSHOT RETRIEVAL] Requesting snapshot at index {} (currentStep={})", requestedStep, currentStep_);

    auto prevSnapshot = snapshotManager_.getSnapshot(requestedStep);
    if (!prevSnapshot) {
        LOG_ERROR("[SNAPSHOT RETRIEVAL] Failed to find snapshot for step {}", requestedStep);
        return false;
    }

    // Log retrieved snapshot info
    std::string retrievedStatesStr;
    for (const auto &state : prevSnapshot->activeStates) {
        if (!retrievedStatesStr.empty()) {
            retrievedStatesStr += ", ";
        }
        retrievedStatesStr += state;
    }
    LOG_DEBUG("[SNAPSHOT RETRIEVAL] Retrieved snapshot {} with states: [{}]", prevSnapshot->stepNumber,
              retrievedStatesStr);

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

    // Log scheduler state before reset
    if (scheduler_) {
        auto currentScheduledEvents = scheduler_->getScheduledEvents();
        LOG_DEBUG("[RESET] Current scheduler has {} scheduled events before restore", currentScheduledEvents.size());
        for (const auto &event : currentScheduledEvents) {
            LOG_DEBUG("[RESET] Current scheduled event: '{}' (sendId: {}, remainingTime: {}ms)", event.eventName,
                      event.sendId, event.remainingTime.count());
        }
    }

    if (restoreSnapshot(*initialSnapshot_)) {
        // W3C SCXML 3.13: Reset to initial configuration for time-travel debugging
        currentStep_ = 0;
        previousActiveStates_.clear();
        executedEvents_.clear();

        // Clear snapshot history to force re-execution on step forward
        // Reset means starting fresh - step forward should re-execute, not replay cache
        snapshotManager_.clear();

        LOG_INFO("InteractiveTestRunner: Reset complete - returned to step 0 (snapshot history cleared)");

        // Note: No need to re-capture or update initialSnapshot_
        // Step 0 snapshot already exists and will be reused by stepForward() REPLAY MODE
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
    std::string removedEventName = internalQueue[index].name;
    internalQueue.erase(internalQueue.begin() + index);

    // Restore modified queues
    restoreEventQueues(internalQueue, externalQueue);

    // History branching: Invalidate all future snapshots (NEW TIMELINE)
    // Execution path has diverged from cached history
    snapshotManager_.removeSnapshotsAfter(currentStep_);
    LOG_INFO(
        "InteractiveTestRunner: HISTORY BRANCHING - Removed {} future snapshot(s) after step {} (event '{}' removed)",
        snapshotManager_.size(), currentStep_, removedEventName);

    // Capture snapshot to reflect queue modification
    captureSnapshot();

    LOG_DEBUG("InteractiveTestRunner: Removed internal event '{}' at index {} (current step: {})", removedEventName,
              index, currentStep_);
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
    std::string removedEventName = externalQueue[index].name;
    externalQueue.erase(externalQueue.begin() + index);

    // Restore modified queues
    restoreEventQueues(internalQueue, externalQueue);

    // History branching: Invalidate all future snapshots (NEW TIMELINE)
    // Execution path has diverged from cached history
    snapshotManager_.removeSnapshotsAfter(currentStep_);
    LOG_INFO(
        "InteractiveTestRunner: HISTORY BRANCHING - Removed {} future snapshot(s) after step {} (event '{}' removed)",
        snapshotManager_.size(), currentStep_, removedEventName);

    // Capture snapshot to reflect queue modification
    captureSnapshot();

    LOG_DEBUG("InteractiveTestRunner: Removed external event '{}' at index {} (current step: {})", removedEventName,
              index, currentStep_);
    return true;
}

size_t InteractiveTestRunner::pollScheduler() {
    // W3C SCXML 6.2: Poll event scheduler to move ready delayed send events to queue
    // W3C SCXML 3.13: Scheduler always polls automatically (timeout → queue)
    //                 Queue processing is controlled by immediate mode (queue → state machine)
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

bool InteractiveTestRunner::hasQueuedEvents() const {
    if (!eventRaiser_) {
        return false;
    }

    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
    if (!eventRaiserImpl) {
        return false;
    }

    return eventRaiserImpl->hasQueuedEvents();
}

bool InteractiveTestRunner::hasScheduledEvents() const {
    // W3C SCXML 6.2.4: Check scheduler for delayed send operations
    if (!scheduler_) {
        return false;
    }

    return scheduler_->getScheduledEventCount() > 0;
}

int InteractiveTestRunner::getNextScheduledEventTime() const {
    int minTime = std::numeric_limits<int>::max();

    // W3C SCXML 6.2.4: Find next scheduled send operation delay
    if (scheduler_) {
        auto scheduledEvents = scheduler_->getScheduledEvents();
        for (const auto &event : scheduledEvents) {
            int remainingMs = static_cast<int>(event.remainingTime.count());
            if (remainingMs < minTime) {
                minTime = remainingMs;
            }
        }
    }

    if (minTime == std::numeric_limits<int>::max()) {
        return -1;  // No scheduled events
    }

    return minTime >= 0 ? minTime : 0;
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

        LOG_DEBUG("[SNAPSHOT CAPTURE] Found {} scheduled events in scheduler", scheduledEvents.size());

        for (const auto &event : scheduledEvents) {
            // W3C SCXML 6.2: Extract params (convert vector<string> to single string - first value only)
            std::map<std::string, std::string> paramsMap;
            for (const auto &[paramName, paramValues] : event.params) {
                if (!paramValues.empty()) {
                    paramsMap[paramName] = paramValues[0];  // Take first value (W3C allows duplicates)
                }
            }

            scheduledEventsSnapshots.emplace_back(event.eventName, event.sendId, event.originalDelay.count(),
                                                  event.remainingTime.count(), event.sessionId, event.targetUri,
                                                  event.eventType, event.eventData, event.content, paramsMap);

            LOG_DEBUG("[SNAPSHOT CAPTURE] Scheduled event: '{}' (sendId: {}, originalDelay: {}ms, remainingTime: {}ms)",
                      event.eventName, event.sendId, event.originalDelay.count(), event.remainingTime.count());
        }
    }

    // W3C SCXML 3.11: Capture active invocations (part of configuration)
    std::vector<InvokeSnapshot> activeInvokes;
    auto invokeExecutor = stateMachine_->getInvokeExecutor();
    if (invokeExecutor) {
        invokeExecutor->captureInvokeState(activeInvokes);
        LOG_DEBUG("InteractiveTestRunner: Captured {} active invocations", activeInvokes.size());
    }

    // Convert std::vector<std::string> to std::set<std::string> for StateSnapshot
    std::set<std::string> activeStatesSet(activeStates.begin(), activeStates.end());

    // Log snapshot capture for verification
    std::string statesStr;
    for (const auto &state : activeStatesSet) {
        if (!statesStr.empty()) {
            statesStr += ", ";
        }
        statesStr += state;
    }
    LOG_DEBUG("[SNAPSHOT CAPTURE] Storing snapshot at index {} with states: [{}]", currentStep_, statesStr);

    snapshotManager_.captureSnapshot(activeStatesSet, dataModel, internalQueue, externalQueue, pendingUIEvents,
                                     scheduledEventsSnapshots, activeInvokes, executedEvents_, currentStep_,
                                     lastEventName_, lastTransitionSource_, lastTransitionTarget_);
}

bool InteractiveTestRunner::restoreSnapshot(const StateSnapshot &snapshot) {
    // W3C SCXML 3.13: Complete reset-restore using Option B (long-term solution)
    // ARCHITECTURE.md: Maintains instance identity, proper reset-restore lifecycle
    // No instance recreation, no start() side effects, no temporal coupling

    // Clear EventRaiser queues BEFORE reset to ensure clean state
    // EventRaiser is reused across restoreSnapshot() calls, so old events may persist
    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
    if (eventRaiserImpl) {
        eventRaiserImpl->clearQueue();
        LOG_DEBUG("InteractiveTestRunner: Cleared EventRaiser queues before reset");
    }

    // Debug: Log snapshot states before restoration
    std::string snapshotStatesStr;
    for (const auto &s : snapshot.activeStates) {
        if (!snapshotStatesStr.empty()) {
            snapshotStatesStr += ", ";
        }
        snapshotStatesStr += s;
    }
    LOG_INFO("InteractiveTestRunner: Restoring snapshot with states: [{}]", snapshotStatesStr);

    // Restore data model BEFORE state restoration
    // This ensures variables are available when states are entered
    restoreDataModel(snapshot.dataModel);

    // W3C SCXML 3.13: Restore state configuration using existing method
    // No instance recreation, no start() call - direct restoration only
    if (!stateMachine_->restoreFromSnapshot(snapshot.activeStates)) {
        LOG_ERROR("InteractiveTestRunner: Failed to restore snapshot states");
        return false;
    }
    LOG_DEBUG("InteractiveTestRunner: Restored {} active states from snapshot", snapshot.activeStates.size());

    // W3C SCXML 3.13: Restore event queues to EventRaiser (Single Source of Truth)
    // Zero Duplication: UI events are included in externalQueue
    restoreEventQueues(snapshot.internalQueue, snapshot.externalQueue);

    // Note: snapshot.pendingUIEvents is now redundant (same as externalQueue)
    // EventRaiser's external queue already contains all UI events

    // W3C SCXML 6.2: Restore scheduled events state for time-travel debugging
    // Time-travel principle: Restored state must be IDENTICAL to original state
    // Events must be re-scheduled to scheduler (not suspended) to maintain behavior consistency
    if (scheduler_ && eventDispatcher_) {
        // Cancel all current scheduled events
        auto currentScheduledEvents = scheduler_->getScheduledEvents();
        for (const auto &event : currentScheduledEvents) {
            scheduler_->cancelEvent(event.sendId);
            LOG_DEBUG("InteractiveTestRunner: Canceled scheduled event '{}' (sendId: {}) before restore",
                      event.eventName, event.sendId);
        }

        // Clear suspended events (we're restoring exact state to scheduler)
        suspendedScheduledEvents_.clear();

        // Re-schedule ALL parent events to scheduler with exact remainingTime
        // Child events will be restored by restoreChildState() later
        // We identify parent events by checking if they exist in child snapshots
        std::unordered_set<std::string> childEventSendIds;
        for (const auto &invoke : snapshot.activeInvokes) {
            if (invoke.childState) {
                for (const auto &childEvent : invoke.childState->scheduledEvents) {
                    childEventSendIds.insert(childEvent.sendId);
                }
            }
        }

        const std::string &currentSessionId = stateMachine_->getSessionId();

        LOG_DEBUG("[SNAPSHOT RESTORE] Restoring {} scheduled events from snapshot (step: {})",
                  snapshot.scheduledEvents.size(), snapshot.stepNumber);

        for (const auto &eventSnapshot : snapshot.scheduledEvents) {
            // Skip events that belong to child (child will restore them)
            if (childEventSendIds.count(eventSnapshot.sendId) > 0) {
                LOG_DEBUG("InteractiveTestRunner: Skipping child event '{}' (sendId: {}) - will be restored by child",
                          eventSnapshot.eventName, eventSnapshot.sendId);
                continue;
            }

            LOG_DEBUG("[SNAPSHOT RESTORE] Restoring event '{}' (sendId: {}, originalDelay: {}ms, remainingTime: {}ms, "
                      "step: {})",
                      eventSnapshot.eventName, eventSnapshot.sendId, eventSnapshot.originalDelayMs,
                      eventSnapshot.remainingTimeMs, snapshot.stepNumber);

            // W3C SCXML 6.2.4: Recreate send operation with remaining time
            EventDescriptor event;
            event.eventName = eventSnapshot.eventName;
            event.data = eventSnapshot.eventData;
            event.type = eventSnapshot.eventType;
            event.target = eventSnapshot.targetUri;
            event.sendId = eventSnapshot.sendId;
            event.sessionId = currentSessionId;  // Update to new session ID

            // W3C SCXML 6.2: Restore params for _event.data construction
            for (const auto &[paramName, paramValue] : eventSnapshot.params) {
                event.params[paramName] = {paramValue};
            }

            // W3C SCXML 3.13: Time-travel debugging - use different delay based on snapshot type
            // Step 0 (initial snapshot): Use originalDelay for reset (fresh start)
            // Other steps: Use remainingTime for accurate time-travel restoration
            auto delay = (snapshot.stepNumber == 0) ? std::chrono::milliseconds(eventSnapshot.originalDelayMs)
                                                    : std::chrono::milliseconds(eventSnapshot.remainingTimeMs);

            auto future = eventDispatcher_->sendEventDelayed(event, delay);

            LOG_DEBUG("InteractiveTestRunner: Restored parent scheduled event '{}' (sendId: {}, delay: {}ms, "
                      "remainingTime: {}ms, originalDelay: {}ms, step: {})",
                      eventSnapshot.eventName, eventSnapshot.sendId, delay.count(), eventSnapshot.remainingTimeMs,
                      eventSnapshot.originalDelayMs, snapshot.stepNumber);
        }

        LOG_DEBUG("InteractiveTestRunner: Restored {} scheduled events to scheduler with exact remainingTime",
                  snapshot.scheduledEvents.size());
    }

    // W3C SCXML 3.11: Restore active invocations (part of configuration)
    // Child state machines will restore their scheduled events to scheduler automatically
    // This maintains time-travel consistency - child events poll and fire just like initial load
    auto invokeExecutor = stateMachine_->getInvokeExecutor();
    if (invokeExecutor && !snapshot.activeInvokes.empty()) {
        invokeExecutor->restoreInvokeState(snapshot.activeInvokes, stateMachine_);
        LOG_DEBUG("InteractiveTestRunner: Restored {} active invocations", snapshot.activeInvokes.size());
    }

    // Restore metadata
    lastEventName_ = snapshot.lastEventName;
    lastTransitionSource_ = snapshot.lastTransitionSource;
    lastTransitionTarget_ = snapshot.lastTransitionTarget;

    // Restore execution history (for UI display, NOT for replay)
    executedEvents_ = snapshot.executedEvents;

    LOG_INFO("InteractiveTestRunner: State restored to step {} via direct restoration (no side effects)",
             snapshot.stepNumber);

    // Verify restoration by checking actual active states
    LOG_DEBUG("[SNAPSHOT RESTORE DEBUG] About to call getActiveStates() for verification...");
    auto actualStates = stateMachine_->getActiveStates();
    LOG_DEBUG("[SNAPSHOT RESTORE DEBUG] getActiveStates() returned {} states", actualStates.size());
    std::string actualStatesStr;
    for (const auto &s : actualStates) {
        if (!actualStatesStr.empty()) {
            actualStatesStr += ", ";
        }
        actualStatesStr += s;
    }
    LOG_DEBUG("[SNAPSHOT RESTORE] Verification - actual active states after restore: [{}]", actualStatesStr);

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
    // W3C SCXML 5.10.1: Restore events with complete metadata for _event object
    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser);
    if (!eventRaiserImpl) {
        LOG_ERROR("InteractiveTestRunner: EventRaiser is not EventRaiserImpl, cannot restore metadata");
        return;
    }

    eventRaiserImpl->clearQueue();
    LOG_DEBUG("InteractiveTestRunner: Cleared existing queue for clean restoration");

    // W3C SCXML 3.13: Disable immediate mode during queue restoration
    // Interactive debugging requires events to stay in queue until stepForward()
    // Without this, events are processed immediately instead of being queued
    eventRaiserImpl->setImmediateMode(false);
    LOG_DEBUG("InteractiveTestRunner: Disabled immediate mode for queue restoration (will remain disabled)");

    // Restore internal queue (higher priority)
    // W3C SCXML 3.13: Preserve original timestamps for FIFO ordering
    for (const auto &event : internal) {
        eventRaiserImpl->raiseEventWithPriority(event.name, event.data, EventRaiserImpl::EventPriority::INTERNAL,
                                                event.origin, event.sendid, event.invokeid, event.origintype,
                                                event.timestampNs);
    }

    // Restore external queue (lower priority)
    // W3C SCXML 3.13: Preserve original timestamps for FIFO ordering
    for (const auto &event : external) {
        eventRaiserImpl->raiseEventWithPriority(event.name, event.data, EventRaiserImpl::EventPriority::EXTERNAL,
                                                event.origin, event.sendid, event.invokeid, event.origintype,
                                                event.timestampNs);
    }

    // W3C SCXML 3.13: Keep immediate mode disabled for interactive debugging
    // Events must be processed only via explicit stepForward() calls
    LOG_DEBUG("InteractiveTestRunner: Restored queues - internal: {}, external: {} (immediate mode remains disabled)",
              internal.size(), external.size());
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

    // W3C SCXML 6.2.4: Return all scheduled send operations with remaining delays
    // Scheduler state reflects snapshot restoration for deterministic replay
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

std::string InteractiveTestRunner::evaluateExpression(const std::string &expression) const {
    if (!stateMachine_) {
        LOG_ERROR("evaluateExpression: No state machine available");
        return "";
    }

    // Zero Duplication: Use JSEngine as Single Source of Truth for expression evaluation
    auto &jsEngine = JSEngine::instance();
    const std::string &sessionId = stateMachine_->getSessionId();

    try {
        // W3C SCXML 5.9: Evaluate expression in current session context
        auto future = jsEngine.evaluateExpression(sessionId, expression);
        auto result = future.get();

        if (result.isSuccess()) {
            return result.getValueAsString();
        } else {
            LOG_ERROR("evaluateExpression failed: {}", result.getErrorMessage());
            return "";
        }
    } catch (const std::exception &e) {
        LOG_ERROR("evaluateExpression exception: {}", e.what());
        return "";
    }
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
            const std::string &content = invoke->getContent();
            const std::string &contentExpr = invoke->getContentExpr();

            // Skip dynamic invoke (srcexpr or contentExpr)
            if (!srcExpr.empty() || !contentExpr.empty()) {
                LOG_DEBUG("  Skipping dynamic invoke (srcexpr or contentExpr) in state '{}'", state->getId());
                continue;
            }

            std::shared_ptr<SCXMLModel> childModel;
            std::string srcPath;

            // W3C SCXML 6.4: Process file-based invoke (src attribute)
            if (!src.empty()) {
                // Resolve file path
                std::string fullPath = src;
                if (src.find("file:") == 0) {
                    fullPath = src.substr(5);  // Remove "file:" prefix
                }

                // Make absolute if relative
                if (!fullPath.empty() && fullPath[0] != '/') {
                    fullPath = basePath_ + fullPath;
                }

                LOG_DEBUG("  Attempting to load sub-SCXML from file: {}", fullPath);

                // Parse child SCXML file
                childModel = parser->parseFile(fullPath);

                if (!childModel) {
                    LOG_WARN("  Failed to parse sub-SCXML file '{}' - skipping visualization", fullPath);
                    continue;
                }

                srcPath = fullPath;
            }
            // W3C SCXML 6.4: Process content-based invoke (inline <content> element)
            else if (!content.empty()) {
                LOG_DEBUG("  Attempting to load sub-SCXML from inline content in state '{}'", state->getId());

                // Parse child SCXML from content string
                childModel = parser->parseContent(content);

                if (!childModel) {
                    LOG_WARN("  Failed to parse sub-SCXML inline content in state '{}' - skipping visualization",
                             state->getId());
                    continue;
                }

                // Use special marker for inline content (no actual file path)
                srcPath = "inline-content:" + state->getId();
            } else {
                LOG_DEBUG("  No src or content in invoke - skipping");
                continue;
            }

            // Build structure object for JavaScript
            SubSCXMLInfo info;
            info.parentStateId = state->getId();
            info.invokeId =
                invoke->getId().empty() ? ("invoke_" + std::to_string(subScxmlStructures_.size())) : invoke->getId();
            info.srcPath = srcPath;
#ifdef __EMSCRIPTEN__
            info.structure = buildStructureFromModel(childModel);
#endif

            subScxmlStructures_.push_back(info);
            LOG_DEBUG("  Successfully loaded sub-SCXML: {} (from state '{}')", srcPath, state->getId());
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

        // W3C SCXML 3.6: Extract initial attribute for compound/parallel states
        // Used for auto-expanding ancestor states to show initial configuration
        const auto &initialState = state->getInitialState();
        if (!initialState.empty()) {
            stateObj.set("initial", initialState);
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

        // W3C SCXML 6.3: Extract invoke metadata for child SCXML navigation
        const auto &invokes = state->getInvoke();
        if (!invokes.empty()) {
            stateObj.set("hasInvoke", true);

            // W3C SCXML 6.4: Serialize all invokes (states can have multiple invoke elements)
            auto invokesArray = emscripten::val::array();

            for (const auto &invoke : invokes) {
                if (!invoke) {
                    continue;
                }

                auto invokeObj = emscripten::val::object();

                // W3C SCXML 6.4.1: Type (static or dynamic)
                const std::string &type = invoke->getType();
                const std::string &typeExpr = invoke->getTypeExpr();
                if (!type.empty()) {
                    invokeObj.set("invokeType", type);
                } else if (!typeExpr.empty()) {
                    invokeObj.set("invokeTypeExpr", typeExpr);
                }

                // W3C SCXML 6.4.2: ID or idlocation
                const std::string &id = invoke->getId();
                const std::string &idLocation = invoke->getIdLocation();
                if (!id.empty()) {
                    invokeObj.set("invokeId", id);
                }
                if (!idLocation.empty()) {
                    invokeObj.set("invokeIdLocation", idLocation);
                }

                // W3C SCXML 6.4.3: Source (static or dynamic)
                const std::string &src = invoke->getSrc();
                const std::string &srcExpr = invoke->getSrcExpr();
                if (!src.empty()) {
                    invokeObj.set("invokeSrc", src);
                } else if (!srcExpr.empty()) {
                    invokeObj.set("invokeSrcExpr", srcExpr);
                }

                // W3C SCXML 6.4.4: Content (inline SCXML or dynamic expression)
                const std::string &content = invoke->getContent();
                const std::string &contentExpr = invoke->getContentExpr();
                if (!content.empty()) {
                    invokeObj.set("invokeContent", content);
                } else if (!contentExpr.empty()) {
                    invokeObj.set("invokeContentExpr", contentExpr);
                }

                // W3C SCXML 6.4.5: Params (name-value pairs)
                const auto &params = invoke->getParams();
                if (!params.empty()) {
                    auto paramsArray = emscripten::val::array();
                    for (const auto &param : params) {
                        auto paramObj = emscripten::val::object();
                        paramObj.set("name", std::get<0>(param));
                        paramObj.set("expr", std::get<1>(param));
                        const std::string &location = std::get<2>(param);
                        if (!location.empty()) {
                            paramObj.set("location", location);
                        }
                        paramsArray.call<void>("push", paramObj);
                    }
                    invokeObj.set("invokeParams", paramsArray);
                }

                // W3C SCXML 6.4.6: Namelist (variable names to pass)
                const std::string &namelist = invoke->getNamelist();
                if (!namelist.empty()) {
                    invokeObj.set("invokeNamelist", namelist);
                }

                // W3C SCXML 6.4.7: AutoForward (automatic event forwarding)
                bool autoForward = invoke->isAutoForward();
                if (autoForward) {
                    invokeObj.set("invokeAutoForward", true);
                }

                // W3C SCXML 6.5: Finalize (script to execute when child sends events)
                const std::string &finalize = invoke->getFinalize();
                if (!finalize.empty()) {
                    invokeObj.set("invokeFinalize", finalize);
                }

                invokesArray.call<void>("push", invokeObj);

                LOG_DEBUG("buildStructureFromModel: State '{}' has invoke (type='{}', src='{}', id='{}')", stateId,
                          type.empty() ? typeExpr : type, src.empty() ? srcExpr : src, id);
            }

            stateObj.set("invokes", invokesArray);
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

// Standalone function to set log level from JavaScript
EMSCRIPTEN_KEEPALIVE
extern "C" void setSpdlogLevel(const char *level) {
    if (!level) {
        return;
    }

    LogLevel logLevel = LogLevel::Debug;  // Default: Debug level for development

    std::string levelStr(level);
    if (levelStr == "trace") {
        logLevel = LogLevel::Trace;
    } else if (levelStr == "debug") {
        logLevel = LogLevel::Debug;
    } else if (levelStr == "info") {
        logLevel = LogLevel::Info;
    } else if (levelStr == "warn") {
        logLevel = LogLevel::Warn;
    } else if (levelStr == "error") {
        logLevel = LogLevel::Error;
    } else if (levelStr == "critical") {
        logLevel = LogLevel::Critical;
    } else if (levelStr == "off") {
        logLevel = LogLevel::Off;
    }

    Logger::setLevel(logLevel);
    LOG_INFO("Log level set to: {}", levelStr);
}
#endif

}  // namespace SCE::W3C
