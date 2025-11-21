#include "runtime/StateMachine.h"
#include "common/BindingHelper.h"
#include "common/ConflictResolutionHelper.h"
#include "states/ConcurrentStateTypes.h"

using SCE::Common::ConflictResolutionHelperString;
#include "common/DataModelInitHelper.h"
#include "common/DoneDataHelper.h"
#include "common/EntryExitHelper.h"
#include "common/FileLoadingHelper.h"
#include "common/Logger.h"
#ifdef SCE_USE_SPDLOG
#include <spdlog/spdlog.h>
#endif
#include "common/ParallelTransitionHelper.h"
#include "common/StringUtils.h"
#include "common/SystemVariableHelper.h"
#include "common/TransitionHelper.h"
#include "core/EventProcessingAlgorithms.h"
#include "core/EventQueueAdapters.h"
#include "events/EventRaiserService.h"

#include "factory/NodeFactory.h"
#include "model/SCXMLModel.h"
#include "parsing/ActionParser.h"
#include "parsing/IXMLParser.h"
#include "parsing/SCXMLParser.h"
#include "parsing/XIncludeProcessor.h"
#include "runtime/ActionExecutorImpl.h"
#include "runtime/DataContentHelpers.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/ExecutionContextImpl.h"
#include "runtime/HistoryManager.h"
#include "runtime/HistoryStateAutoRegistrar.h"
#include "runtime/HistoryValidator.h"
#include "runtime/ImmediateModeGuard.h"
#include "scripting/JSEngine.h"
#include "states/ConcurrentRegion.h"
#include "states/ConcurrentStateNode.h"
#include <algorithm>
#include <fstream>
#include <random>
#include <regex>
#include <set>
#include <sstream>
#include <unordered_set>

namespace SCE {

// Thread-local depth tracking for nested processEvent calls (W3C SCXML compliance)
// Prevents deadlock by allowing same-thread recursion without re-acquiring mutex
thread_local int processEventDepth = 0;

// RAII guard for exception-safe initial configuration flag management
namespace {
class InitialConfigurationGuard {
public:
    explicit InitialConfigurationGuard(bool &flag) : flag_(flag) {
        flag_ = true;
    }

    ~InitialConfigurationGuard() {
        flag_ = false;
    }

    // Non-copyable and non-movable
    InitialConfigurationGuard(const InitialConfigurationGuard &) = delete;
    InitialConfigurationGuard &operator=(const InitialConfigurationGuard &) = delete;
    InitialConfigurationGuard(InitialConfigurationGuard &&) = delete;
    InitialConfigurationGuard &operator=(InitialConfigurationGuard &&) = delete;

private:
    bool &flag_;
};
}  // anonymous namespace

StateMachine::StateMachine() : jsEnvironmentReady_(false) {
    sessionId_ = JSEngine::instance().generateSessionIdString("sm_");
    // JS environment changed to lazy initialization
    // ActionExecutor and ExecutionContext initialized in setupJSEnvironment

    // Initialize History Manager with SOLID architecture (Dependency Injection)
    initializeHistoryManager();

    // Initialize InvokeExecutor with SOLID architecture (W3C SCXML invoke support)
    invokeExecutor_ = std::make_unique<InvokeExecutor>(nullptr);  // EventDispatcher will be set later if needed
}

// Constructor with session ID injection for invoke scenarios
StateMachine::StateMachine(const std::string &sessionId) : jsEnvironmentReady_(false) {
    if (sessionId.empty()) {
        throw std::invalid_argument("StateMachine: Session ID cannot be empty when using injection constructor");
    }

    sessionId_ = sessionId;
    LOG_DEBUG("StateMachine: Created with injected session ID: {}", sessionId_);

    // JS environment uses lazy initialization - use existing session
    // ActionExecutor and ExecutionContext are initialized in setupJSEnvironment

    // Initialize History Manager with SOLID architecture (Dependency Injection)
    initializeHistoryManager();

    // Initialize InvokeExecutor with SOLID architecture (W3C SCXML invoke support)
    invokeExecutor_ = std::make_unique<InvokeExecutor>(nullptr);  // EventDispatcher will be set later if needed
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
            LOG_DEBUG("StateMachine: Clearing EventRaiser callback before destruction");
            eventRaiserImpl->clearEventCallback();
        }
    }

    // CRITICAL: Wait for any in-progress processEvent calls to complete (ASAN heap-use-after-free fix)
    // Lock mutex to ensure no processEvent is running when we proceed with destruction
    // This prevents ProcessingEventGuard from accessing freed isProcessingEvent_ member
    // Thread-local depth tracking ensures nested calls don't cause deadlock
    {
        std::lock_guard<std::mutex> processEventLock(processEventMutex_);
        LOG_DEBUG("StateMachine: All processEvent calls completed, proceeding with destruction");
    }

    // W3C SCXML 3.13: Always call stop() to ensure session cleanup
    // Final state sets isRunning_=false but session must still be destroyed
    // stop() is idempotent and handles cleanup even when isRunning_=false
    stop();

    // FUNDAMENTAL FIX: Two-Phase Destruction Pattern
    // LIFECYCLE: RAII Destruction Stage
    // Destructor handles only internal resource cleanup (no external dependencies)
    // JSEngine session already destroyed in stop() to prevent deadlock
    // See stop() method for explicit cleanup of external dependencies
    LOG_DEBUG("StateMachine: Destruction complete (JSEngine session cleaned up in stop())");
}

bool StateMachine::loadSCXML(const std::string &filename) {
    try {
        auto nodeFactory = std::make_shared<NodeFactory>();
        auto xincludeProcessor = std::make_shared<XIncludeProcessor>();
        SCXMLParser parser(nodeFactory, xincludeProcessor);

        model_ = parser.parseFile(filename);
        if (!model_) {
            LOG_ERROR("Failed to parse SCXML file: {}", filename);
            return false;
        }

        // Register file path for this session to enable relative path resolution
        SCE::JSEngine::instance().registerSessionFilePath(sessionId_, filename);
        LOG_DEBUG("StateMachine: Registered file path '{}' for session '{}'", filename, sessionId_);

        return initializeFromModel();
    } catch (const std::exception &e) {
        LOG_ERROR("Exception loading SCXML: {}", e.what());
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
            LOG_ERROR("StateMachine: Failed to parse SCXML content");
            return false;
        }

        return initializeFromModel();
    } catch (const std::exception &e) {
        LOG_ERROR("Exception parsing SCXML content: {}", e.what());
        return false;
    }
}

bool StateMachine::loadModel(std::shared_ptr<SCXMLModel> model) {
    if (!model) {
        LOG_ERROR("StateMachine: Cannot load null model");
        return false;
    }

    model_ = model;
    return initializeFromModel();
}

bool StateMachine::start(bool autoProcessQueuedEvents) {
    // Interactive mode: Store flag for processEvent() to skip auto-batch processing
    autoProcessQueuedEvents_ = autoProcessQueuedEvents;

    if (initialState_.empty()) {
        LOG_ERROR("StateMachine: Cannot start - no initial state defined");
        return false;
    }

    // Ensure JS environment initialization
    if (!ensureJSEnvironment()) {
        LOG_ERROR("StateMachine: Cannot start - JavaScript environment initialization failed");
        return false;
    }

    LOG_DEBUG("Starting with initial state: {}", initialState_);

    // Check EventRaiser status at StateMachine start
    if (eventRaiser_) {
        LOG_DEBUG("StateMachine: EventRaiser status check - EventRaiser: {}, sessionId: {}", (void *)eventRaiser_.get(),
                  sessionId_);
    } else {
        LOG_WARN("StateMachine: EventRaiser is null - sessionId: {}", sessionId_);
    }

    // Set running state before entering initial state to handle immediate done.state events
    isRunning_ = true;

    // W3C SCXML 3.3: Support multiple initial states for parallel regions
    // W3C SCXML 3.2: If no initial attribute specified, use first state in document order
    const auto &modelInitialStates = model_->getInitialStates();
    std::vector<std::string> initialStates;

    if (modelInitialStates.empty()) {
        // W3C SCXML 3.2: No initial attribute - auto-select first state in document order
        const auto &allStates = model_->getAllStates();
        if (allStates.empty()) {
            LOG_ERROR("StateMachine: No states found in SCXML model");
            isRunning_ = false;
            return false;
        }

        initialStates.push_back(allStates[0]->getId());
        LOG_DEBUG("W3C SCXML 3.2: No initial attribute, auto-selected first state: '{}'", initialStates[0]);
    } else {
        // W3C SCXML 3.3: Use explicitly specified initial states
        initialStates = modelInitialStates;
    }

    // W3C SCXML: For initial state entry, add ancestor states to configuration first
    // This ensures ancestor onentry actions are executed (e.g., test 388 requires s0 onentry)
    if (model_ && hierarchyManager_) {
        // Collect all unique ancestors from all initial states
        std::vector<std::string> ancestorChain;
        std::set<std::string> seenAncestors;

        for (const auto &initialStateId : initialStates) {
            auto stateNode = model_->findStateById(initialStateId);
            IStateNode *current = stateNode ? stateNode->getParent() : nullptr;

            std::vector<std::string> currentAncestors;
            while (current) {
                const std::string &ancestorId = current->getId();
                if (!ancestorId.empty() && seenAncestors.find(ancestorId) == seenAncestors.end()) {
                    currentAncestors.push_back(ancestorId);
                    seenAncestors.insert(ancestorId);
                }
                current = current->getParent();
            }

            // Reverse to get parent->child order
            std::reverse(currentAncestors.begin(), currentAncestors.end());

            // Merge into main ancestor chain
            for (const auto &ancestorId : currentAncestors) {
                if (std::find(ancestorChain.begin(), ancestorChain.end(), ancestorId) == ancestorChain.end()) {
                    ancestorChain.push_back(ancestorId);
                }
            }
        }

        // Add ancestors to configuration (without onentry yet)
        for (const auto &ancestorId : ancestorChain) {
            hierarchyManager_->addStateToConfigurationWithoutOnEntry(ancestorId);
            LOG_DEBUG("Added ancestor state to configuration: {}", ancestorId);

            // W3C SCXML 3.3 test 576: Setup and activate parallel state regions for deep initial targets
            // When entering via deep initial targets (e.g., initial="s11p112 s11p122"),
            // parallel ancestor states must have their regions properly configured and activated
            // for event processing, invoke deferral, and action execution
            auto ancestorState = model_->findStateById(ancestorId);
            if (ancestorState && ancestorState->getType() == Type::PARALLEL) {
                auto parallelState = dynamic_cast<ConcurrentStateNode *>(ancestorState);
                if (parallelState) {
                    if (!setupAndActivateParallelState(parallelState, ancestorId)) {
                        isRunning_ = false;
                        return false;
                    }
                }
            }
        }

        // Execute onentry for ancestors in order (parent to child)
        for (const auto &ancestorId : ancestorChain) {
            executeOnEntryActions(ancestorId);
            LOG_DEBUG("Executed onentry for ancestor state: {}", ancestorId);
        }
    }

    // W3C SCXML 3.3: Enter all initial states (supports parallel initial configuration)
    // RAII guard ensures flag is reset even on exception
    InitialConfigurationGuard guard(isEnteringInitialConfiguration_);

    for (const auto &initialStateId : initialStates) {
        if (!enterState(initialStateId)) {
            LOG_ERROR("Failed to enter initial state: {}", initialStateId);
            isRunning_ = false;
            return false;  // Guard destructor will reset isEnteringInitialConfiguration_
        }
        LOG_DEBUG("Entered initial state: {}", initialStateId);
    }

    // Guard destructor will automatically reset isEnteringInitialConfiguration_ to false

    // W3C SCXML 3.13: Macrostep execution order after initial state entry
    // Per W3C SCXML specification, invokes must only execute for states "entered and not exited":
    //
    // Execution sequence:
    // 1. Enter initial states (compound states → initial children via recursive entry)
    //    - Invokes are deferred during state entry (not executed yet)
    // 2. Check eventless transitions (states may exit before invokes execute - test 422)
    //    - Example: s11 has eventless transition to s12, s11 exits immediately
    // 3. Execute pending invokes (only for states still active after step 2)
    //    - Filter: invoke executes only if isStateActive(stateId) returns true
    // 4. Process queued events (invokes may raise internal events)
    // 5. Repeat eventless transition checks until stable configuration reached
    //
    // This order ensures W3C SCXML 3.13 compliance: "invokes execute in document order
    // in all states that have been entered (and not exited) since last macrostep"

    // W3C SCXML 3.13: Repeat eventless transitions until stable configuration reached
    // This is critical for parallel states where entering a parallel state may enable
    // new eventless transitions in its regions (e.g., test 448)
    int eventlessIterations = 0;
    const int MAX_EVENTLESS_ITERATIONS = 1000;
    while (checkEventlessTransitions()) {
        if (++eventlessIterations > MAX_EVENTLESS_ITERATIONS) {
            LOG_ERROR("StateMachine: checkEventlessTransitions exceeded max iterations ({}) - possible infinite loop",
                      MAX_EVENTLESS_ITERATIONS);
            break;
        }
        LOG_DEBUG("StateMachine: Eventless transition executed (iteration {})", eventlessIterations);
    }
    LOG_DEBUG("StateMachine: Reached stable configuration after {} eventless iterations", eventlessIterations);

    // W3C SCXML compliance: Execute deferred invokes after eventless transitions
    // Only states that remain active after eventless transitions should have invokes executed
    LOG_DEBUG("StateMachine: Executing pending invokes after eventless transitions for session: {}", sessionId_);
    executePendingInvokes();

    // W3C SCXML: Process all remaining queued events after initial state entry
    // This ensures the state machine reaches a stable state before returning,
    // eliminating the need for external callers to explicitly call processQueuedEvents()
    // Interactive mode: Skip auto-processing to allow manual step-by-step execution
    if (autoProcessQueuedEvents && eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            int iterations = 0;
            const int MAX_START_ITERATIONS = 1000;

            // W3C SCXML 3.12.1: Use shared algorithm (Single Source of Truth)
            SCE::Core::InterpreterEventQueue adapter(eventRaiserImpl);
            while (adapter.hasEvents()) {
                if (++iterations > MAX_START_ITERATIONS) {
                    LOG_ERROR("StateMachine: start() exceeded max iterations ({}) - possible infinite event loop",
                              MAX_START_ITERATIONS);
                    break;
                }

                LOG_DEBUG("StateMachine: Processing queued events after start (iteration {})", iterations);

                // W3C SCXML 3.3: RAII guard to prevent recursive auto-processing during batch event processing
                {
                    BatchProcessingGuard batchGuard(isBatchProcessing_);
                    adapter.popNext();
                }

                // Check for eventless transitions after processing events
                checkEventlessTransitions();
            }

            if (iterations > 0) {
                LOG_DEBUG("StateMachine: All queued events processed after start ({} iterations)", iterations);
            }
        }
    } else if (!autoProcessQueuedEvents) {
        LOG_DEBUG("StateMachine: Interactive mode - skipping auto-processing of queued events");
    }

    updateStatistics();

    LOG_INFO("StateMachine: Started successfully");
    return true;
}

void StateMachine::stop() {
    LOG_DEBUG("StateMachine: Stopping state machine (isRunning: {})", isRunning_.load());

    // W3C SCXML Test 250: Exit ALL active states with onexit handlers (only if still running)
    // Must exit in reverse document order (children before parents)
    if (isRunning_) {
        auto activeStates = getActiveStates();
        for (auto it = activeStates.rbegin(); it != activeStates.rend(); ++it) {
            exitState(*it);
        }

        isRunning_ = false;

        // State management delegated to StateHierarchyManager
        if (hierarchyManager_) {
            hierarchyManager_->reset();
        }
    }

    // CRITICAL: Always unregister from JSEngine, even if isRunning_ is already false
    // Race condition prevention: JSEngine worker threads may have queued tasks accessing StateMachine
    // W3C Test 415: isRunning_=false may be set in top-level final state before destructor calls stop()
    SCE::JSEngine::instance().setStateMachine(nullptr, sessionId_);
    LOG_DEBUG("StateMachine: Unregistered from JSEngine");

    // FUNDAMENTAL FIX: Two-Phase Destruction Pattern
    // LIFECYCLE: Explicit Cleanup Stage
    // W3C SCXML: Destroy JSEngine session before RAII destruction
    // Ensures JSEngine singleton is alive during cleanup (prevents deadlock)
    // Required for StaticExecutionEngine wrapper lifecycle management
    if (jsEnvironmentReady_) {
        SCE::JSEngine::instance().destroySession(sessionId_);
        jsEnvironmentReady_ = false;
        LOG_DEBUG("StateMachine: Destroyed JSEngine session in stop(): {}", sessionId_);
    }

    updateStatistics();
    LOG_INFO("StateMachine: Stopped");
}

StateMachine::TransitionResult StateMachine::processEvent(const std::string &eventName, const std::string &eventData) {
    // W3C SCXML 6.4: Check if there's an origin session ID from EventRaiser thread-local storage
    std::string originSessionId = EventRaiserImpl::getCurrentOriginSessionId();

    // W3C SCXML 5.10: Check if there's a send ID from EventRaiser thread-local storage (for error events)
    std::string sendId = EventRaiserImpl::getCurrentSendId();

    // W3C SCXML 5.10: Check if there's an invoke ID from EventRaiser thread-local storage (test 338)
    std::string invokeId = EventRaiserImpl::getCurrentInvokeId();

    // W3C SCXML 5.10: Check if there's an origin type from EventRaiser thread-local storage (test 253, 331, 352, 372)
    std::string originType = EventRaiserImpl::getCurrentOriginType();

    // Delegate to overload with originSessionId (may be empty for non-invoke events)
    return processEvent(eventName, eventData, originSessionId, sendId, invokeId, originType);
}

StateMachine::TransitionResult StateMachine::processEvent(const std::string &eventName, const std::string &eventData,
                                                          const std::string &originSessionId, const std::string &sendId,
                                                          const std::string &invokeId, const std::string &originType) {
    // W3C SCXML 5.10: Get event type from EventRaiser thread-local storage (test 331)
    std::string eventType = EventRaiserImpl::getCurrentEventType();
    if (!isRunning_) {
        LOG_WARN("StateMachine: Cannot process event - state machine not running");
        TransitionResult result;
        result.success = false;
        result.errorMessage = "State machine not running";
        return result;
    }

    // Check JS environment
    if (!jsEnvironmentReady_) {
        LOG_ERROR("StateMachine: Cannot process event - JavaScript environment not ready");
        TransitionResult result;
        result.success = false;
        result.errorMessage = "JavaScript environment not ready";
        return result;
    }

    LOG_DEBUG("StateMachine: Processing event: '{}' with data: '{}' in session: '{}', originSessionId: '{}'", eventName,
              eventData, sessionId_, originSessionId);

    // CRITICAL: Thread-local depth tracking for nested processEvent calls (ASAN heap-use-after-free fix)
    // Top-level call (depth==0): acquire mutex to synchronize with destructor
    // Nested call (depth>0): same thread, no mutex needed (prevents deadlock)
    // This pattern matches EventSchedulerImpl's thread-local approach
    bool isTopLevelCall = (processEventDepth == 0);
    std::unique_ptr<std::lock_guard<std::mutex>> processEventLock;
    if (isTopLevelCall) {
        processEventLock = std::make_unique<std::lock_guard<std::mutex>>(processEventMutex_);
    }

    // RAII-style depth tracking with exception safety
    ++processEventDepth;

    struct DepthGuard {
        ~DepthGuard() {
            --processEventDepth;
        }
    } depthGuard;

    // Set event processing flag with RAII for exception safety
    struct ProcessingEventGuard {
        bool &flag_;
        bool wasAlreadySet_;  // Public member to check if this is a nested call

        explicit ProcessingEventGuard(bool &flag) : flag_(flag), wasAlreadySet_(flag) {
            if (!wasAlreadySet_) {
                LOG_DEBUG("ProcessingEventGuard: Setting isProcessingEvent_ = true");
                flag_ = true;
            } else {
                LOG_DEBUG("ProcessingEventGuard: Already processing event (nested call)");
            }
        }

        ~ProcessingEventGuard() {
            if (!wasAlreadySet_) {
                LOG_DEBUG("ProcessingEventGuard: Setting isProcessingEvent_ = false");
                flag_ = false;
            } else {
                LOG_DEBUG("ProcessingEventGuard: Leaving isProcessingEvent_ = true (nested call)");
            }
        }

        // Delete copy constructor and assignment
        ProcessingEventGuard(const ProcessingEventGuard &) = delete;
        ProcessingEventGuard &operator=(const ProcessingEventGuard &) = delete;
    };

    // W3C SCXML 5.10: RAII guard to protect _event during nested event processing (Test 230)
    struct EventContextGuard {
        ActionExecutorImpl *actionExecutorImpl_;  // Cached pointer to avoid dynamic_pointer_cast overhead
        EventMetadata savedEvent_;
        bool isNested_;

        explicit EventContextGuard(ActionExecutorImpl *actionExecutorImpl, const EventMetadata &newEvent)
            : actionExecutorImpl_(actionExecutorImpl), isNested_(false) {
            if (actionExecutorImpl_) {
                // Save current event (may be from parent processEvent call)
                savedEvent_ = actionExecutorImpl_->getCurrentEvent();
                isNested_ = !savedEvent_.name.empty();

                if (isNested_) {
                    LOG_DEBUG(
                        "EventContextGuard: Nested event processing - saving _event='{}', setting new _event='{}'",
                        savedEvent_.name, newEvent.name);
                }

                // Set new event for this processing level
                actionExecutorImpl_->setCurrentEvent(newEvent);
            }
        }

        ~EventContextGuard() {
            if (actionExecutorImpl_ && isNested_) {
                // Restore saved event
                actionExecutorImpl_->setCurrentEvent(savedEvent_);
                LOG_DEBUG("EventContextGuard: Restored _event='{}' after nested processing", savedEvent_.name);
            }
        }

        // Delete copy constructor and assignment
        EventContextGuard(const EventContextGuard &) = delete;
        EventContextGuard &operator=(const EventContextGuard &) = delete;
    };

    ProcessingEventGuard eventGuard(isProcessingEvent_);

    // W3C SCXML 5.10: Protect _event during nested event processing with RAII guard (Test 230)
    EventMetadata currentEventMetadata(eventName, eventData, eventType, sendId, invokeId, originType, originSessionId);
    EventContextGuard eventContextGuard(cachedExecutorImpl_, currentEventMetadata);

    // Count this event
    stats_.totalEvents++;

    // Store event data for access in guards/actions
    currentEventData_ = eventData;

    if (!sendId.empty() || !invokeId.empty() || !originType.empty() || !eventType.empty() || !originSessionId.empty()) {
        LOG_DEBUG("StateMachine: Set current event in ActionExecutor - event: '{}', data: '{}', sendid: '{}', "
                  "invokeid: '{}', origintype: '{}', type: '{}', originSessionId: '{}'",
                  eventName, eventData, sendId, invokeId, originType, eventType, originSessionId);
    } else {
        LOG_DEBUG("StateMachine: Set current event in ActionExecutor - event: '{}', data: '{}'", eventName, eventData);
    }

    // W3C SCXML Test 252: Filter events from cancelled invoke child sessions
    if (invokeExecutor_ && !originSessionId.empty()) {
        if (invokeExecutor_->shouldFilterCancelledInvokeEvent(originSessionId)) {
            LOG_DEBUG("StateMachine: Filtering event '{}' from cancelled invoke child session: {}", eventName,
                      originSessionId);
            return TransitionResult(false, getCurrentState(), getCurrentState(), eventName);
        }
    }

    // W3C SCXML 1.0 Section 6.4: Execute finalize handler before processing events from invoked children
    // According to W3C SCXML: "finalize markup runs BEFORE the event is processed"
    // The finalize handler is executed when an event arrives from an invoked child
    // and has access to _event.data to update parent variables before transition evaluation
    if (invokeExecutor_ && !originSessionId.empty()) {
        // W3C SCXML compliance: Use originSessionId to find the exact child that sent this event
        std::string finalizeScript = invokeExecutor_->getFinalizeScriptForChildSession(originSessionId);

        if (!finalizeScript.empty()) {
            LOG_DEBUG("StateMachine: Executing finalize handler BEFORE processing event '{}', script: '{}'", eventName,
                      finalizeScript);

            // W3C SCXML 6.4: Parse and execute finalize as SCXML executable content
            // Finalize contains elements like <assign>, <script>, <log>, <raise>, <if>, <foreach> etc.
            if (actionExecutor_) {
                try {
                    // Parse finalize XML content using IXMLParser
                    std::string xmlWrapper =
                        "<finalize xmlns=\"http://www.w3.org/2005/07/scxml\">" + finalizeScript + "</finalize>";

                    auto parser = IXMLParser::create();
                    auto document = parser->parseContent(xmlWrapper);

                    if (!document || !document->isValid()) {
                        LOG_ERROR("StateMachine: Failed to parse finalize XML: {}", parser->getLastError());
                    } else {
                        auto root = document->getRootElement();
                        if (!root) {
                            LOG_ERROR("StateMachine: No root element in finalize XML");
                        } else {
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
                                    LOG_DEBUG("StateMachine: Finalize action '{}' executed: {}", child->getName(),
                                              success);
                                }
                            }
                        }
                    }

                    LOG_DEBUG("StateMachine: Finalize handler executed successfully for event '{}'", eventName);
                } catch (const std::exception &e) {
                    LOG_ERROR("StateMachine: Exception during finalize handler execution: {}", e.what());
                }
            } else {
                LOG_WARN("StateMachine: No ActionExecutor available for finalize execution");
            }
        }
    }

    // W3C SCXML 1.0 Section 6.4: Auto-forward external events to child invoke sessions
    // Autoforward all events EXCEPT platform events (done.*, error.*) which are state machine internal
    // W3C Test 230: Events from child sessions ARE autoforwarded back to verify field preservation
    // Use shared_ptr to prevent use-after-free if child reaches final state during processEvent
    bool isPlatform = isPlatformEvent(eventName);
    LOG_DEBUG("W3C SCXML 6.4: Autoforward check - event='{}', invokeExecutor={}, isPlatform={}", eventName,
              (invokeExecutor_ ? "YES" : "NO"), isPlatform);
    if (invokeExecutor_ && !isPlatform) {
        auto autoForwardSessions = invokeExecutor_->getAutoForwardSessions(sessionId_);
        LOG_DEBUG("W3C SCXML 6.4: Found {} autoforward sessions for parent '{}'", autoForwardSessions.size(),
                  sessionId_);
        for (const auto &childStateMachine : autoForwardSessions) {
            if (childStateMachine && childStateMachine->isRunning()) {
                LOG_DEBUG("W3C SCXML 6.4: Auto-forwarding event '{}' to child session", eventName);
                childStateMachine->processEvent(eventName, eventData, originSessionId, sendId, invokeId, originType);
            }
        }
    }

    // Find applicable transitions from SCXML model
    if (!model_) {
        LOG_ERROR("StateMachine: No SCXML model available");
        TransitionResult result;
        result.success = false;
        result.fromState = getCurrentState();
        result.eventName = eventName;
        result.errorMessage = "No SCXML model available";
        return result;
    }

    // SCXML W3C specification section 3.4: Handle parallel state event broadcasting
    std::string currentState = getCurrentState();
    auto currentStateNode = model_->findStateById(currentState);
    if (!currentStateNode) {
        LOG_DEBUG("Current state not found in model: {}", currentState);
        TransitionResult result;
        result.success = false;
        result.fromState = getCurrentState();
        result.eventName = eventName;
        result.errorMessage = "Current state not found in model";
        return result;
    }

    // SCXML W3C specification compliance: Process parallel state events according to standard priority
    if (currentStateNode->getType() == Type::PARALLEL) {
        auto parallelState = dynamic_cast<ConcurrentStateNode *>(currentStateNode);
        assert(parallelState && "SCXML violation: PARALLEL type state must be ConcurrentStateNode");

        LOG_DEBUG("Processing event '{}' for parallel state: {}", eventName, currentState);

        // SCXML W3C specification 3.13: Check transitions on the parallel state itself
        // Internal transitions (no target) execute actions but DON'T prevent region processing
        // External transitions (with target) exit the parallel state and return immediately
        auto stateTransitionResult = processStateTransitions(currentStateNode, eventName, eventData);
        if (stateTransitionResult.success) {
            // Check if this is an external transition (toState != fromState)
            if (stateTransitionResult.toState != stateTransitionResult.fromState) {
                // External transition: exit parallel state
                LOG_DEBUG("SCXML W3C: External transition from parallel state: {} -> {}",
                          stateTransitionResult.fromState, stateTransitionResult.toState);

                // W3C SCXML 3.3: Process all internal events before returning
                // Only process if this is the top-level event (not nested/recursive call)
                // Interactive mode: Skip auto-processing to allow manual step-by-step execution
                if (!eventGuard.wasAlreadySet_ && !isBatchProcessing_ && autoProcessQueuedEvents_ && eventRaiser_) {
                    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
                    if (eventRaiserImpl) {
                        // W3C SCXML 3.12.1: Use shared algorithm (Single Source of Truth)
                        SCE::Core::InterpreterEventQueue adapter(eventRaiserImpl);
                        SCE::Core::EventProcessingAlgorithms::processInternalEventQueue(adapter, [](bool) {
                            LOG_DEBUG(
                                "W3C SCXML 3.3: Processing queued internal event after parallel external transition");
                            return true;
                        });
                    }
                }

                // W3C SCXML 6.4: Execute pending invokes after macrostep completes
                if (!eventGuard.wasAlreadySet_) {
                    executePendingInvokes();
                }

                return stateTransitionResult;
            }
            // Internal transition: actions executed, continue to region processing
            LOG_DEBUG("SCXML W3C: Internal transition on parallel state {} (actions executed, continuing to regions)",
                      currentState);
        }

        // SCXML W3C specification 3.13: Removed region root state check (lines 457-471)
        // The old approach checked region root states with processStateTransitions() and returned early.
        // This violated W3C SCXML 3.13 because:
        // 1. It prevented proper event broadcasting to ALL regions
        // 2. It didn't handle transition preemption correctly (child > parent)
        // 3. It didn't respect document order for transition priority
        // 4. It didn't distinguish cross-region vs external transitions
        // Instead, use region->processEvent() below (lines 484-537) which properly implements SCXML 3.13

        // W3C SCXML 3.13: Broadcast event to ALL regions using processEventInAllRegions()
        // This ensures proper transition preemption, blocking, and external transition handling
        LOG_DEBUG("StateMachine: No transitions on parallel state or region children, broadcasting to all regions");

        // W3C SCXML 3.13: Disable immediate mode during parallel state event processing
        // RAII guard ensures restoration even if processEventInAllRegions() throws exception
        // This prevents re-entrancy: raised events must be queued, not processed immediately
        // Otherwise, one region's <raise> action can deactivate other regions before they compute their transitions
        std::vector<ConcurrentOperationResult> results;
        {
            ImmediateModeGuard guard(eventRaiser_, false);
            LOG_DEBUG("W3C SCXML 3.13: Disabled immediate mode for parallel state event processing");

            // Create EventDescriptor for SCXML-compliant event processing
            EventDescriptor event;
            event.eventName = eventName;
            event.data = eventData;

            // Broadcast event to all active regions (SCXML W3C mandated)
            // Exception safety: guard automatically restores immediate mode on scope exit
            results = parallelState->processEventInAllRegions(event);

            LOG_DEBUG("W3C SCXML 3.13: Immediate mode will be restored on scope exit");
        }  // RAII guard restores immediate mode here

        bool anyTransitionExecuted = false;

        // W3C SCXML Appendix D.2: Collect all enabled transitions from all regions
        std::vector<TransitionDescriptorString> allEnabledTransitions;

        // Collect enabled transitions from all regions (including external transitions)
        for (const auto &result : results) {
            // Collect enabled transitions from this region
            for (const auto &transition : result.enabledTransitions) {
                LOG_DEBUG(
                    "StateMachine: Collected enabled transition from region '{}': {} -> {} (event='{}', external={})",
                    result.regionId, transition.source, transition.target, transition.event, transition.isExternal);
                allEnabledTransitions.push_back(transition);
            }
        }

        // W3C SCXML Appendix D.2: Apply conflict resolution to select optimal transition set
        if (!allEnabledTransitions.empty()) {
            LOG_DEBUG("StateMachine: Applying W3C SCXML Appendix D.2 conflict resolution to {} enabled transitions",
                      allEnabledTransitions.size());

            // Convert to ConflictResolutionHelperString::TransitionDescriptor format
            std::vector<ConflictResolutionHelperString::TransitionDescriptor> descriptors;
            for (const auto &t : allEnabledTransitions) {
                ConflictResolutionHelperString::TransitionDescriptor desc;
                desc.source = t.source;
                desc.target = t.target;
                desc.exitSet = t.exitSet;
                desc.transitionIndex = t.transitionIndex;
                desc.hasActions = t.hasActions;
                desc.isInternal = t.isInternal;
                desc.isExternal = t.isExternal;  // W3C SCXML: External transition flag for parallel state exit
                descriptors.push_back(desc);
            }

            // Apply W3C SCXML conflict resolution
            auto getParent = [this](const std::string &stateId) -> std::optional<std::string> {
                auto stateNode = model_->findStateById(stateId);
                if (stateNode && stateNode->getParent()) {
                    return stateNode->getParent()->getId();
                }
                return std::nullopt;
            };

            auto isParallelState = [this](const std::string &stateId) -> bool {
                auto stateNode = model_->findStateById(stateId);
                return stateNode && stateNode->getType() == Type::PARALLEL;
            };

            descriptors =
                ConflictResolutionHelperString::removeConflictingTransitions(descriptors, getParent, isParallelState);

            LOG_DEBUG("StateMachine: After conflict resolution: {} transitions in optimal set", descriptors.size());

            // W3C SCXML Appendix D.2: Execute optimal transition set as microstep
            if (!descriptors.empty()) {
                // Check if optimal set contains external transition
                bool hasExternalTransition = false;
                std::string externalTransitionTarget;
                std::string externalTransitionSource;

                for (const auto &desc : descriptors) {
                    if (desc.isExternal) {
                        hasExternalTransition = true;
                        externalTransitionTarget = desc.target;
                        externalTransitionSource = desc.source;
                        LOG_INFO(
                            "StateMachine: Optimal set contains external transition: {} -> {} (W3C SCXML Appendix D.2)",
                            desc.source, desc.target);
                        break;
                    }
                }

                // W3C SCXML Appendix D.2: Execute ALL transitions' actions first (including those with external
                // transitions)
                LOG_INFO("StateMachine: Executing {} transitions in optimal set as microstep (W3C SCXML Appendix D.2)",
                         descriptors.size());

                // Step 1: Exit all states in exit sets (W3C SCXML Appendix D Step 2)
                std::unordered_set<std::string> statesToExit;
                for (const auto &desc : descriptors) {
                    for (const auto &exitState : desc.exitSet) {
                        statesToExit.insert(exitState);
                    }
                }

                // Exit states in document order
                for (const auto &stateId : statesToExit) {
                    LOG_DEBUG("StateMachine: Microstep exit state: {}", stateId);
                    // Find region containing this state and exit it
                    auto stateNode = model_->findStateById(stateId);
                    if (stateNode && stateNode->getParent()) {
                        const auto &regions = parallelState->getRegions();
                        for (const auto &region : regions) {
                            auto regionStates = region->getActiveStates();
                            if (std::find(regionStates.begin(), regionStates.end(), stateId) != regionStates.end()) {
                                LOG_DEBUG("StateMachine: Region {} exits state {}", region->getId(), stateId);
                                break;
                            }
                        }
                    }
                }

                // Step 2: Execute ALL transition actions (W3C SCXML Appendix D Step 3)
                // W3C SCXML 3.13: Disable immediate mode during action execution to prevent nested event processing
                // Raised events should be queued until after all actions in the optimal set complete
                ImmediateModeGuard immediateModeGuard(eventRaiser_, false);

                for (const auto &desc : descriptors) {
                    LOG_INFO("StateMachine: Microstep execute transition: {} -> {}", desc.source, desc.target);

                    // Find the transition node and execute its actions
                    auto sourceStateNode = model_->findStateById(desc.source);
                    if (sourceStateNode) {
                        const auto &transitions = sourceStateNode->getTransitions();
                        for (const auto &transition : transitions) {
                            bool isMatch = false;
                            const auto &targets = transition->getTargets();

                            // W3C SCXML: Targetless internal transitions (source == target)
                            if (desc.source == desc.target && targets.empty()) {
                                const auto &events = transition->getEvents();
                                for (const auto &event : events) {
                                    if (event == eventName || event == "*") {
                                        isMatch = true;
                                        break;
                                    }
                                }
                            }
                            // Normal transition - match by target
                            else if (!targets.empty() && targets[0] == desc.target) {
                                isMatch = true;
                            }

                            if (isMatch) {
                                // Execute transition actions
                                const auto &actionNodes = transition->getActionNodes();
                                if (!actionNodes.empty() && executionContext_) {
                                    LOG_DEBUG("StateMachine: Executing {} transition actions", actionNodes.size());
                                    for (const auto &actionNode : actionNodes) {
                                        if (actionNode) {
                                            try {
                                                actionNode->execute(*executionContext_);
                                            } catch (const std::exception &e) {
                                                LOG_WARN("StateMachine: Transition action failed: {}", e.what());
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }

                // Step 3: Handle external transition or enter target states
                if (hasExternalTransition) {
                    // W3C SCXML: External transition exits parallel state
                    LOG_INFO("StateMachine: External transition {} -> {}, exiting parallel state {}",
                             externalTransitionSource, externalTransitionTarget, currentState);

                    // Exit parallel state and all its regions
                    exitState(currentState);

                    // Enter external target state
                    enterState(externalTransitionTarget);

                    anyTransitionExecuted = true;
                    stats_.totalTransitions++;
                } else {
                    // No external transition - enter target states for internal transitions
                    LOG_INFO(
                        "StateMachine: Internal transitions only - entering target states (W3C SCXML Appendix D.2)");

                    // Enter all target states
                    for (const auto &desc : descriptors) {
                        LOG_DEBUG("StateMachine: Microstep enter target state: {}", desc.target);

                        // Find which region this target belongs to and update its state
                        auto targetStateNode = model_->findStateById(desc.target);
                        if (targetStateNode && targetStateNode->getParent()) {
                            const auto &regions = parallelState->getRegions();
                            for (const auto &region : regions) {
                                // Check if target is descendant of this region's root
                                auto regionRoot = region->getRootState();
                                if (regionRoot) {
                                    std::function<bool(const std::shared_ptr<IStateNode> &, const std::string &)>
                                        isDescendant;
                                    isDescendant = [&isDescendant](const std::shared_ptr<IStateNode> &root,
                                                                   const std::string &targetId) -> bool {
                                        if (root->getId() == targetId) {
                                            return true;
                                        }
                                        for (const auto &child : root->getChildren()) {
                                            if (isDescendant(child, targetId)) {
                                                return true;
                                            }
                                        }
                                        return false;
                                    };

                                    if (isDescendant(regionRoot, desc.target)) {
                                        // This region contains the target - update its current state
                                        auto concreteRegion = std::dynamic_pointer_cast<ConcurrentRegion>(region);
                                        if (concreteRegion) {
                                            concreteRegion->setCurrentState(desc.target);
                                            LOG_DEBUG("StateMachine: Region {} entered state {}", region->getId(),
                                                      desc.target);

                                            // Execute entry actions
                                            const auto &entryBlocks = targetStateNode->getEntryActionBlocks();
                                            if (!entryBlocks.empty() && executionContext_) {
                                                for (const auto &actionBlock : entryBlocks) {
                                                    for (const auto &actionNode : actionBlock) {
                                                        if (actionNode) {
                                                            try {
                                                                actionNode->execute(*executionContext_);
                                                            } catch (const std::exception &e) {
                                                                LOG_WARN("StateMachine: Entry action failed: {}",
                                                                         e.what());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    anyTransitionExecuted = true;
                    stats_.totalTransitions += descriptors.size();
                }  // end else (no external transition)
            }  // end if (!descriptors.empty())
        }  // end if (!allEnabledTransitions.empty())

        if (anyTransitionExecuted) {
            LOG_INFO("SCXML compliant parallel region processing succeeded. Regions processed: {}", results.size());

            // W3C SCXML 3.4: Check if all regions completed (reached final states)
            // This triggers done.state.{id} event generation
            bool allRegionsComplete = parallelState->areAllRegionsComplete();
            if (allRegionsComplete) {
                LOG_DEBUG("SCXML W3C: All parallel regions completed for state: {}", currentState);

                // W3C SCXML 3.4: Process done.state events when all regions complete
                // Only process if this is the top-level event (not nested/recursive call)
                // Interactive mode: Skip auto-processing to allow manual step-by-step execution
                if (!eventGuard.wasAlreadySet_ && !isBatchProcessing_ && autoProcessQueuedEvents_ && eventRaiser_) {
                    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
                    if (eventRaiserImpl) {
                        // W3C SCXML 3.12.1: Use shared algorithm (Single Source of Truth)
                        SCE::Core::InterpreterEventQueue adapter(eventRaiserImpl);
                        SCE::Core::EventProcessingAlgorithms::processInternalEventQueue(adapter, [](bool) {
                            LOG_DEBUG("W3C SCXML 3.4: Processing done.state event after parallel completion");
                            return true;
                        });
                    }
                }
            }

            // Invoke execution consolidated to key lifecycle points            // Return success with parallel state as
            // context
            TransitionResult finalResult;
            finalResult.success = true;
            finalResult.fromState = currentState;
            finalResult.toState = currentState;  // Parallel state remains active
            finalResult.eventName = eventName;

            // W3C SCXML 3.3: Internal events will be processed at hierarchical transition completion
            // Removed auto-processing here to prevent out-of-order execution during eventless transitions
            return finalResult;
        } else {
            LOG_DEBUG("No transitions executed in any region for event: {}", eventName);
            stats_.failedTransitions++;
            TransitionResult result;
            result.success = false;
            result.fromState = getCurrentState();
            result.eventName = eventName;
            result.errorMessage = "No valid transitions found";
            return result;
        }
    }

    // Non-parallel state: SCXML W3C compliant hierarchical event processing
    // Process transitions in active state hierarchy (innermost to outermost)
    auto activeStates = hierarchyManager_->getActiveStates();

    LOG_DEBUG("SCXML hierarchical processing: Checking {} active states for event '{}'", activeStates.size(),
              eventName);

    // W3C SCXML: Process states from most specific (innermost) to least specific (outermost)
    // Optimization: Track checked states to avoid duplicate ancestor traversal
    std::unordered_set<std::string> checkedStates;

    for (auto it = activeStates.rbegin(); it != activeStates.rend(); ++it) {
        const std::string &stateId = *it;
        auto stateNode = model_->findStateById(stateId);
        if (!stateNode) {
            LOG_WARN("SCXML hierarchical processing: State node not found: {}", stateId);
            continue;
        }

        // W3C SCXML: Check transitions from innermost state to root
        // Skip already-checked ancestors to avoid duplicate processing
        IStateNode *currentNode = stateNode;
        while (currentNode) {
            const std::string &nodeId = currentNode->getId();

            // Skip if already checked (optimization for duplicate ancestor traversal)
            if (checkedStates.count(nodeId)) {
                break;
            }
            checkedStates.insert(nodeId);

            LOG_DEBUG("SCXML hierarchical processing: Checking state '{}' for transitions", nodeId);
            auto transitionResult = processStateTransitions(currentNode, eventName, eventData);
            if (transitionResult.success) {
                LOG_DEBUG("SCXML hierarchical processing: Transition found in state '{}': {} -> {}", nodeId,
                          transitionResult.fromState, transitionResult.toState);

                // W3C SCXML 3.3: Process all internal events before returning
                // Only process if this is the top-level event (not nested/recursive call)
                // Interactive mode: Skip auto-processing to allow manual step-by-step execution
                if (!eventGuard.wasAlreadySet_ && !isBatchProcessing_ && autoProcessQueuedEvents_ && eventRaiser_) {
                    auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
                    if (eventRaiserImpl) {
                        // W3C SCXML 3.12.1: Use shared algorithm (Single Source of Truth)
                        SCE::Core::InterpreterEventQueue adapter(eventRaiserImpl);
                        SCE::Core::EventProcessingAlgorithms::processInternalEventQueue(adapter, [](bool) {
                            LOG_DEBUG("W3C SCXML 3.3: Processing queued internal event after successful transition");
                            return true;
                        });
                    }
                }

                // W3C SCXML 6.4: Execute pending invokes after macrostep completes
                if (!eventGuard.wasAlreadySet_) {
                    executePendingInvokes();
                }

                return transitionResult;
            }

            // Move to parent state
            currentNode = currentNode->getParent();
        }
    }

    // No transitions found in any active state
    LOG_DEBUG("SCXML hierarchical processing: No transitions found in any active state for event '{}'", eventName);
    stats_.failedTransitions++;

    TransitionResult result;
    result.success = false;
    result.fromState = getCurrentState();
    result.eventName = eventName;
    result.errorMessage = "No valid transitions found in active state hierarchy";

    // W3C SCXML 3.3: Process all internal events before returning
    // After processing an external event, the system MUST process all queued internal events
    // This ensures done.state events are automatically processed (test: W3C_Parallel_CompletionCriteria)
    // Only process if this is the top-level event (not nested/recursive call)
    // Interactive mode: Skip auto-processing to allow manual step-by-step execution
    if (!eventGuard.wasAlreadySet_ && !isBatchProcessing_ && autoProcessQueuedEvents_ && eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            // W3C SCXML 3.12.1: Use shared algorithm (Single Source of Truth)
            SCE::Core::InterpreterEventQueue adapter(eventRaiserImpl);
            SCE::Core::EventProcessingAlgorithms::processInternalEventQueue(adapter, [](bool) {
                LOG_DEBUG("W3C SCXML 3.3: Processing queued internal event");
                return true;
            });
        }
    }

    // W3C SCXML 6.4: Execute pending invokes after macrostep completes
    if (!eventGuard.wasAlreadySet_) {
        executePendingInvokes();
    }

    return result;
}

StateMachine::TransitionResult StateMachine::processStateTransitions(IStateNode *stateNode,
                                                                     const std::string &eventName,
                                                                     const std::string &eventData) {
    LOG_DEBUG("[PROCESS STATE TRANSITIONS CALLED] stateNode: {}, event: '{}', isRunning: {}",
              (stateNode ? stateNode->getId() : "null"), eventName, isRunning_.load());

    // eventData available for future SCXML features (e.g., event.data access in guards/actions)
    (void)eventData;

    if (!stateNode) {
        TransitionResult result;
        result.success = false;
        result.fromState = getCurrentState();
        result.eventName = eventName;
        result.errorMessage = "Invalid state node";
        return result;
    }

    // SCXML W3C specification: Process transitions in document order
    const auto &transitions = stateNode->getTransitions();

    LOG_DEBUG("Checking {} transitions for event '{}' on state: {}", transitions.size(), eventName, stateNode->getId());

    // Execute first valid transition (SCXML W3C specification)
    for (const auto &transitionNode : transitions) {
        // W3C SCXML 3.12: A transition can have multiple event descriptors
        // The transition matches if at least one descriptor matches the event name
        const std::vector<std::string> &eventDescriptors = transitionNode->getEvents();

        // Check if this transition matches the event
        bool eventMatches = false;

        if (eventName.empty()) {
            // For eventless transitions, only consider transitions without event descriptors
            eventMatches = eventDescriptors.empty();
        } else {
            // W3C SCXML 3.12: Check if ANY descriptor matches the event
            // Use TransitionHelper for Single Source of Truth (Zero Duplication with AOT engine)
            eventMatches = SCE::TransitionHelper::matchesAnyEventDescriptor(eventDescriptors, eventName);
        }

        if (!eventMatches) {
            continue;
        }

        const auto &targets = transitionNode->getTargets();

        // W3C SCXML: Internal transitions have no targets but should still execute
        bool isInternal = transitionNode->isInternal();
        if (targets.empty() && !isInternal) {
            LOG_DEBUG("StateMachine: Skipping transition with no targets (not internal)");
            continue;
        }

        std::string targetState = targets.empty() ? "" : targets[0];
        std::string condition = transitionNode->getGuard();

        // Performance optimization: Only build debug string when DEBUG logging is enabled
#ifdef SCE_USE_SPDLOG
        if constexpr (SPDLOG_ACTIVE_LEVEL <= SPDLOG_LEVEL_DEBUG) {
#else
        // DefaultBackend: always build debug string (no compile-time level check)
        {
#endif
            std::string eventDescStr;
            for (size_t i = 0; i < eventDescriptors.size(); ++i) {
                if (i > 0) {
                    eventDescStr += " ";
                }
                eventDescStr += eventDescriptors[i];
            }
            LOG_DEBUG("Checking transition: {} -> {} with condition: '{}' (events: '{}')", stateNode->getId(),
                      targetState, condition, eventDescStr);
        }

        bool conditionResult = condition.empty() || evaluateCondition(condition);
        LOG_DEBUG("Condition result: {}", conditionResult ? "true" : "false");

        if (conditionResult) {
            // W3C SCXML: The source state of the transition is the state that contains it
            // NOT getCurrentState() which may return a parallel state
            std::string fromState = stateNode->getId();

            // W3C SCXML 3.13: Internal transitions (test 505)
            if (isInternal) {
                // Case 1: Internal transition with no target (targetless)
                if (targets.empty()) {
                    LOG_DEBUG("StateMachine: Executing internal transition actions (no state change)");
                    const auto &actionNodes = transitionNode->getActionNodes();
                    if (!actionNodes.empty()) {
                        executeActionNodes(actionNodes, false);
                    }

                    TransitionResult result;
                    result.success = true;
                    result.fromState = fromState;
                    result.toState = fromState;  // Same state (internal transition)
                    result.eventName = eventName;
                    return result;
                }

                // Case 2: Internal transition with target (test 505, 533)
                // W3C SCXML 3.13: "if the transition has 'type' "internal", its source state is a compound state
                // and all its target states are proper descendents of its source state"

                // W3C SCXML 3.13 (test 533): Check if source state is compound
                // If source is not compound (e.g., parallel, atomic), treat as external
                auto sourceNode = model_->findStateById(fromState);
                if (sourceNode && sourceNode->getType() != Type::COMPOUND) {
                    LOG_WARN("StateMachine: Internal transition source '{}' is not a compound state (type: {}) - "
                             "treating as external per W3C SCXML 3.13",
                             fromState, static_cast<int>(sourceNode->getType()));
                    isInternal = false;
                }

                // VALIDATION: Check all targets before making any state changes
                // This ensures atomic transition semantics - either all succeed or none
                for (const auto &target : targets) {
                    // Check 1: Target state node must exist
                    auto targetNode = model_->findStateById(target);
                    if (!targetNode) {
                        LOG_ERROR("Internal transition target state not found: {}", target);
                        TransitionResult result;
                        result.success = false;
                        result.fromState = fromState;
                        result.eventName = eventName;
                        result.errorMessage = "Internal transition target state not found: " + target;
                        return result;
                    }

                    // Check 2: Target must be a proper descendant of source
                    if (!isDescendant(target, fromState)) {
                        LOG_WARN("StateMachine: Internal transition target '{}' is not a descendant of source '{}' - "
                                 "treating as external",
                                 target, fromState);
                        isInternal = false;
                        break;
                    }
                }

                // If validation passed, proceed with internal transition
                if (isInternal) {
                    // Valid internal transition with target
                    // Exit only the descendants, not the source state itself
                    LOG_DEBUG("StateMachine: Executing internal transition with target: {} -> {}", fromState,
                              targetState);

                    // W3C SCXML 3.13: Exit active descendants of source that need to be exited
                    // For test 505: s11 is active and must be exited before entering again
                    // Use helper method to build exit set (reduces code duplication)
                    std::vector<std::string> exitSet = buildExitSetForDescendants(fromState, false);

                    // Exit descendant states
                    for (const auto &stateToExit : exitSet) {
                        if (!exitState(stateToExit)) {
                            LOG_ERROR("Failed to exit state: {}", stateToExit);
                            inTransition_ = false;  // Clear flag on error
                            TransitionResult result;
                            result.success = false;
                            result.fromState = fromState;
                            result.eventName = eventName;
                            result.errorMessage = "Failed to exit state: " + stateToExit;
                            return result;
                        }
                    }

                    // Execute transition actions
                    const auto &actionNodes = transitionNode->getActionNodes();
                    if (!actionNodes.empty()) {
                        LOG_DEBUG("StateMachine: Executing internal transition actions");
                        executeActionNodes(actionNodes, false);
                    }

                    // W3C SCXML 3.13: Enter target state(s) without re-entering source state
                    // For internal transitions, use enterStateWithAncestors to prevent source re-entry
                    LOG_DEBUG("StateMachine: Before entering target states, active states: {}", [this]() {
                        auto states = hierarchyManager_->getActiveStates();
                        std::string result;
                        for (const auto &s : states) {
                            if (!result.empty()) {
                                result += ", ";
                            }
                            result += s;
                        }
                        return result;
                    }());

                    auto sourceNode = model_->findStateById(fromState);
                    if (!sourceNode) {
                        LOG_ERROR("Source state node not found: {}", fromState);
                        TransitionResult result;
                        result.success = false;
                        result.fromState = fromState;
                        result.eventName = eventName;
                        result.errorMessage = "Source state node not found: " + fromState;
                        return result;
                    }

                    for (const auto &target : targets) {
                        LOG_DEBUG("StateMachine: Entering target state '{}' with stopAtParent='{}'", target, fromState);
                        // Use enterStateWithAncestors with stopAtParent=source to prevent source re-entry
                        if (!hierarchyManager_->enterStateWithAncestors(target, sourceNode, nullptr)) {
                            LOG_ERROR("Failed to enter target state: {}", target);
                            TransitionResult result;
                            result.success = false;
                            result.fromState = fromState;
                            result.eventName = eventName;
                            result.errorMessage = "Failed to enter target state: " + target;
                            return result;
                        }
                    }

                    // Check for eventless transitions after entering target
                    checkEventlessTransitions();

                    LOG_DEBUG("StateMachine: After internal transition, active states: {}", [this]() {
                        auto states = hierarchyManager_->getActiveStates();
                        std::string result;
                        for (const auto &s : states) {
                            if (!result.empty()) {
                                result += ", ";
                            }
                            result += s;
                        }
                        return result;
                    }());

                    TransitionResult result;
                    result.success = true;
                    result.fromState = fromState;
                    result.toState = targetState;  // Target state entered
                    result.eventName = eventName;
                    return result;
                }
            }

            LOG_DEBUG("Executing SCXML compliant transition from {} to {}", fromState, targetState);

            // Set transition context flag (for history recording in exitState)
            // RAII guard ensures flag is cleared on all exit paths (normal return, error, exception)
            TransitionGuard transitionGuard(inTransition_);

            // W3C SCXML 3.13: Compute exit set and LCA in one call (optimization: avoid duplicate LCA calculation)
            ExitSetResult exitSetResult = computeExitSet(fromState, targetState);
            LOG_DEBUG("W3C SCXML: Exiting {} states for transition {} -> {}", exitSetResult.states.size(), fromState,
                      targetState);

            // W3C SCXML 3.6: Record history BEFORE exiting states (test 388)
            // History must be recorded while all descendants are still active
            // Optimization: Only record for states that actually have history children
            if (historyManager_ && hierarchyManager_) {
                auto currentActiveStates = hierarchyManager_->getActiveStates();
                for (const std::string &stateToExit : exitSetResult.states) {
                    auto stateNode = model_->findStateById(stateToExit);
                    if (stateNode &&
                        (stateNode->getType() == Type::COMPOUND || stateNode->getType() == Type::PARALLEL)) {
                        // Check if this state has history children
                        bool hasHistoryChildren = false;
                        for (const auto &child : stateNode->getChildren()) {
                            if (child->getType() == Type::HISTORY) {
                                hasHistoryChildren = true;
                                break;
                            }
                        }

                        // Only record history if this state has history children
                        if (hasHistoryChildren) {
                            bool recorded = historyManager_->recordHistory(stateToExit, currentActiveStates);
                            if (recorded) {
                                LOG_DEBUG("Pre-recorded history for state '{}' before exit", stateToExit);
                            }
                        }
                    }
                }
            }

            // Exit states in the exit set (already in correct order: deepest first)
            for (const std::string &stateToExit : exitSetResult.states) {
                if (!exitState(stateToExit)) {
                    LOG_ERROR("Failed to exit state: {}", stateToExit);
                    // TransitionGuard will automatically clear inTransition_ flag on return
                    TransitionResult result;
                    result.success = false;
                    result.fromState = fromState;
                    result.eventName = eventName;
                    result.errorMessage = "Failed to exit state: " + stateToExit;
                    return result;
                }
            }

            // Execute transition actions (SCXML W3C specification)
            // W3C compliance: Events raised in transition actions must be queued, not processed immediately
            const auto &actionNodes = transitionNode->getActionNodes();
            if (!actionNodes.empty()) {
                // W3C SCXML 5.10: Protect _event during transition action execution (Test 230)
                // Save current event context before executing actions to prevent corruption by nested events
                EventMetadata savedEvent;
                if (actionExecutor_) {
                    auto actionExecutorImpl = std::dynamic_pointer_cast<ActionExecutorImpl>(actionExecutor_);
                    if (actionExecutorImpl) {
                        savedEvent = actionExecutorImpl->getCurrentEvent();
                    }
                }

                LOG_DEBUG("StateMachine: Executing transition actions (events will be queued)");
                // processEventsAfter=false: Don't process events yet, they will be handled in macrostep loop
                executeActionNodes(actionNodes, false);

                // W3C SCXML 5.10: Restore _event after transition action execution
                if (actionExecutor_) {
                    auto actionExecutorImpl = std::dynamic_pointer_cast<ActionExecutorImpl>(actionExecutor_);
                    if (actionExecutorImpl) {
                        actionExecutorImpl->setCurrentEvent(savedEvent);
                        LOG_DEBUG("StateMachine: Restored _event after transition actions (name='{}', data='{}')",
                                  savedEvent.name, savedEvent.data);
                    }
                }
            } else {
                LOG_DEBUG("StateMachine: No transition actions for this transition");
            }

            // W3C SCXML 3.13: Compute enter set - all states from LCA (exclusive) to target (inclusive)
            // Special case: history states use enterStateWithAncestors(), so skip enter set
            std::vector<std::string> enterSet;
            bool isHistoryTarget = historyManager_ && historyManager_->isHistoryState(targetState);

            if (!targetState.empty() && model_ && !isHistoryTarget) {
                auto targetNode = model_->findStateById(targetState);
                if (targetNode) {
                    // W3C SCXML: Compute enter set from target up to (not including) LCA
                    // Special case (test 579): if target == LCA (ancestor transition),
                    // include target in enter set to ensure onentry is executed
                    std::vector<std::string> statesToEnter;
                    IStateNode *current = targetNode;

                    while (current != nullptr) {
                        std::string currentId = current->getId();

                        // W3C SCXML: Don't include LCA unless it's the target (ancestor transition)
                        if (currentId == exitSetResult.lca && currentId != targetState) {
                            break;  // Reached LCA for normal transition, stop without adding it
                        }

                        // Add state to enter set
                        statesToEnter.push_back(currentId);

                        // If we just added target==LCA (ancestor transition), stop here
                        if (currentId == exitSetResult.lca) {
                            break;
                        }

                        current = current->getParent();
                    }
                    // Reverse to get shallowest first (parent before children)
                    enterSet.assign(statesToEnter.rbegin(), statesToEnter.rend());
                }
            }

            LOG_DEBUG("W3C SCXML: Entering {} states for transition {} -> {}", enterSet.size(), fromState, targetState);

            updateStatistics();
            stats_.totalTransitions++;

            // W3C SCXML 3.13: Track last executed transition for interactive visualizer
            // IMPORTANT: Set BEFORE enterState() because enterState() may trigger eventless transitions
            // that will overwrite this value with the correct final transition
            lastTransitionSource_ = fromState;
            lastTransitionTarget_ = targetState;
            LOG_DEBUG("W3C SCXML 3.13: Event transition executed: {} -> {}", fromState, targetState);

            // Enter all states in enter set (shallowest first)
            for (const std::string &stateToEnter : enterSet) {
                if (!enterState(stateToEnter)) {
                    LOG_ERROR("Failed to enter state: {}", stateToEnter);
                    // TransitionGuard will automatically clear inTransition_ flag on return
                    TransitionResult result;
                    result.success = false;
                    result.fromState = fromState;
                    result.toState = targetState;
                    result.eventName = eventName;
                    result.errorMessage = "Failed to enter state: " + stateToEnter;
                    return result;
                }
            }

            // W3C SCXML 3.10: History states handle ancestors automatically via enterStateWithAncestors()
            if (isHistoryTarget) {
                if (!enterState(targetState)) {
                    LOG_ERROR("Failed to enter history state: {}", targetState);
                    // TransitionGuard will automatically clear inTransition_ flag on return
                    TransitionResult result;
                    result.success = false;
                    result.fromState = fromState;
                    result.toState = targetState;
                    result.eventName = eventName;
                    result.errorMessage = "Failed to enter history state: " + targetState;
                    return result;
                }
            }

            LOG_INFO("Successfully transitioned from {} to {}", fromState, targetState);

            // W3C SCXML compliance: Macrostep loop - check for eventless transitions
            // After a transition completes, we must check for eventless transitions
            // that may have been enabled by the state change. Repeat until no
            // eventless transitions are found. Queued events are processed by
            // processQueuedEvents() in FIFO order to maintain event ordering guarantees.
            if (eventRaiser_) {
                auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
                if (eventRaiserImpl) {
                    LOG_DEBUG("W3C SCXML: Starting macrostep loop after transition");

                    // W3C SCXML: Safety guard against infinite loops in malformed SCXML
                    // Typical SCXML should complete in far fewer iterations
                    const int MAX_MACROSTEP_ITERATIONS = 1000;
                    int iterations = 0;

                    while (true) {
                        if (++iterations > MAX_MACROSTEP_ITERATIONS) {
                            LOG_ERROR(
                                "W3C SCXML: Macrostep limit exceeded ({} iterations) - possible infinite loop in SCXML",
                                MAX_MACROSTEP_ITERATIONS);
                            LOG_ERROR("W3C SCXML: Check for circular eventless transitions in your SCXML document");
                            break;  // Safety exit
                        }

                        // W3C SCXML: Check for eventless transitions on all active states
                        bool eventlessTransitionExecuted = checkEventlessTransitions();

                        if (eventlessTransitionExecuted) {
                            LOG_DEBUG("W3C SCXML: Eventless transition executed, continuing macrostep");
                            continue;  // Loop back to check for more eventless transitions
                        }

                        // W3C SCXML: No eventless transitions found, exit macrostep
                        // Queued events will be processed by processQueuedEvents() in FIFO order
                        LOG_DEBUG("W3C SCXML: No eventless transitions, macrostep complete");
                        break;
                    }

                    LOG_DEBUG("W3C SCXML: Macrostep loop complete");
                }
            }

            // TransitionGuard will automatically clear inTransition_ flag on return
            // Note: executePendingInvokes() is NOT called here to prevent recursive deadlock
            // when child invokes send events to parent during initialization (W3C SCXML 6.4)
            // Invokes are executed only at top-level macrostep boundaries in start()
            return TransitionResult(true, fromState, targetState, eventName);
        }
    }

    // No valid transitions found
    LOG_DEBUG("No valid transitions found for event: {} from state: {}", eventName, stateNode->getId());

    // Note: Failed transition counter is managed at processEvent() level to avoid double counting

    TransitionResult result;
    result.success = false;
    result.fromState = getCurrentState();
    result.eventName = eventName;
    result.errorMessage = "No valid transitions found";
    return result;
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
        LOG_WARN("StateMachine::getActiveStates: hierarchyManager is null!");
        return {};
    }

    auto states = hierarchyManager_->getActiveStates();
    std::string statesStr;
    for (const auto &s : states) {
        if (!statesStr.empty()) {
            statesStr += ", ";
        }
        statesStr += s;
    }
    LOG_DEBUG("[GET ACTIVE STATES] Returning {} states from hierarchyManager: [{}]", states.size(), statesStr);
    return states;
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
        LOG_DEBUG("StateMachine::isStateInFinalState: No model available");
        return false;
    }

    if (stateId.empty()) {
        LOG_DEBUG("StateMachine::isStateInFinalState: State ID is empty");
        return false;
    }

    auto state = model_->findStateById(stateId);
    bool isFinal = state && state->isFinalState();
    LOG_DEBUG("StateMachine::isStateInFinalState: stateId='{}', state found: {}, isFinalState: {}", stateId,
              (void *)state, isFinal);
    return isFinal;
}

bool StateMachine::isInFinalState() const {
    if (!isRunning_) {
        LOG_DEBUG("StateMachine::isInFinalState: State machine is not running");
        return false;
    }

    // W3C SCXML: Check only top-level final states, not child final states of compound states
    // Bug fix: Compound state children (e.g., s02 in test294) should NOT make the machine final
    auto activeStates = hierarchyManager_->getActiveStates();
    for (const auto &stateId : activeStates) {
        auto state = model_->findStateById(stateId);
        if (!state) {
            continue;
        }

        // Only consider top-level final states (states without parent)
        if (state->isFinalState() && !state->getParent()) {
            LOG_DEBUG("StateMachine::isInFinalState: Found top-level final state '{}'", stateId);
            return true;
        }
    }

    LOG_DEBUG("StateMachine::isInFinalState: No top-level final states active");
    return false;
}

std::string StateMachine::getLastTransitionSource() const {
    return lastTransitionSource_;
}

std::string StateMachine::getLastTransitionTarget() const {
    return lastTransitionTarget_;
}

bool StateMachine::restoreFromSnapshot(const std::set<std::string> &states) {
    // W3C SCXML 3.13: Complete state machine restoration for time-travel debugging
    // ARCHITECTURE.md: Template Method pattern - encapsulates restoration lifecycle
    // to prevent temporal coupling and maintain Single Source of Truth

    std::string statesStr;
    for (const auto &s : states) {
        if (!statesStr.empty()) {
            statesStr += ", ";
        }
        statesStr += s;
    }
    LOG_DEBUG("StateMachine::restoreFromSnapshot: Starting - states: [{}], session: {}", statesStr, sessionId_);

    // Step 1: Ensure JavaScript environment is initialized
    // CRITICAL: JS environment must exist BEFORE state restoration for event processing (Test 192)
    if (!ensureJSEnvironment()) {
        LOG_ERROR("StateMachine::restoreFromSnapshot: Failed to initialize JS environment for session {}", sessionId_);
        return false;
    }

    // Step 2: Restore state configuration (delegates to internal method)
    // This sets isRunning_ = true internally
    restoreActiveStatesDirectly(states);

    // Verify restoration
    auto restoredStates = getActiveStates();
    std::string restoredStr;
    for (const auto &s : restoredStates) {
        if (!restoredStr.empty()) {
            restoredStr += ", ";
        }
        restoredStr += s;
    }
    LOG_DEBUG("StateMachine::restoreFromSnapshot: Complete - restored: [{}], running: {}", restoredStr,
              isRunning_.load());

    return true;
}

void StateMachine::restoreActiveStatesDirectly(const std::set<std::string> &states) {
    // W3C SCXML 3.13: Time-travel debugging - restore configuration without side effects
    // ARCHITECTURE.md Zero Duplication: Uses StateHierarchyManager's addStateToConfigurationWithoutOnEntry
    // INTERNAL USE ONLY: Called by restoreFromSnapshot() after JS environment initialization

    LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: Called with {} states", states.size());

    // Mutex scope: Release before getActiveStates() verification to prevent deadlock
    {
        // Thread safety: Lock mutex for hierarchyManager access
        std::lock_guard<std::mutex> lock(hierarchyManagerMutex_);

        if (!hierarchyManager_) {
            LOG_ERROR("StateMachine::restoreActiveStatesDirectly: hierarchyManager_ is null");
            return;
        }

        // Clear current configuration first
        LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: Calling hierarchyManager_->reset()");
        hierarchyManager_->reset();

        // Restore each state without triggering onentry actions
        // States must be added in hierarchical order (parent -> child)
        std::vector<std::string> orderedStates(states.begin(), states.end());
        std::sort(orderedStates.begin(), orderedStates.end(), [this](const std::string &a, const std::string &b) {
            // Parent states come before children (shorter path = higher in hierarchy)
            auto aNode = model_->findStateById(a);
            auto bNode = model_->findStateById(b);
            if (!aNode || !bNode) {
                return a < b;  // Fallback to lexicographic
            }

            // Count ancestors to determine depth
            int depthA = 0, depthB = 0;
            for (auto *p = aNode->getParent(); p; p = p->getParent()) {
                depthA++;
            }
            for (auto *p = bNode->getParent(); p; p = p->getParent()) {
                depthB++;
            }

            return depthA < depthB;  // Lower depth (closer to root) comes first
        });

        for (const auto &stateId : orderedStates) {
            hierarchyManager_->addStateToConfigurationWithoutOnEntry(stateId);
            LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: Added state '{}' to configuration", stateId);
        }

        // W3C SCXML 3.13: Set running state after restoration to enable event processing
        // CRITICAL FIX: Child state machines must be running to receive events from parent
        // Without this, stepBackward() restoration leaves child in stopped state (Test 192)
        LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: [BEFORE isRunning_=true] About to set isRunning_ = true");
        isRunning_ = true;
        LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: [AFTER isRunning_=true] Set to true, mutex will release");
    }  // Mutex released here

    // Verify final state (outside mutex scope to prevent deadlock with getActiveStates())
    LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: [BEFORE getActiveStates()] Calling getActiveStates()");
    auto finalStates = getActiveStates();
    LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: [AFTER getActiveStates()] Returned {} states",
              finalStates.size());
    std::string finalStr;
    for (const auto &s : finalStates) {
        if (!finalStr.empty()) {
            finalStr += ", ";
        }
        finalStr += s;
    }
    LOG_DEBUG("StateMachine::restoreActiveStatesDirectly: Complete - final states: [{}], running: true", finalStr);
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
    return stats_;
}

// W3C SCXML 5.3: Collect all data items from document for global scope initialization
std::vector<StateMachine::DataItemInfo> StateMachine::collectAllDataItems() const {
    std::vector<DataItemInfo> allDataItems;

    if (!model_) {
        return allDataItems;
    }

    // Collect top-level datamodel items
    const auto &topLevelItems = model_->getDataModelItems();
    for (const auto &item : topLevelItems) {
        allDataItems.push_back(DataItemInfo{"", item});  // Empty stateId for top-level
    }
    LOG_DEBUG("StateMachine: Collected {} top-level data items", topLevelItems.size());

    // Collect state-level data items from all states
    const auto &allStates = model_->getAllStates();
    for (const auto &state : allStates) {
        if (!state) {
            continue;
        }

        const auto &stateDataItems = state->getDataItems();
        if (!stateDataItems.empty()) {
            for (const auto &item : stateDataItems) {
                allDataItems.push_back(DataItemInfo{state->getId(), item});
            }
            LOG_DEBUG("StateMachine: Collected {} data items from state '{}'", stateDataItems.size(), state->getId());
        }
    }

    LOG_INFO("StateMachine: Total data items collected: {} (for global scope initialization)", allDataItems.size());
    return allDataItems;
}

// W3C SCXML 5.3: Initialize a single data item with binding mode support
void StateMachine::initializeDataItem(const std::shared_ptr<IDataModelItem> &item, bool assignValue) {
    if (!item) {
        return;
    }

    std::string id = item->getId();
    std::string expr = item->getExpr();
    std::string src = item->getSrc();
    std::string content = item->getContent();

    // W3C SCXML 6.4: Check if variable was pre-initialized (e.g., by invoke namelist/param)
    // Skip this check for late binding value assignment (assignValue=true with late binding)
    // because late binding creates variables as undefined first, then assigns values on state entry
    bool isLateBindingAssignment = assignValue && model_ && (model_->getBinding() == "late");

    if (!isLateBindingAssignment && SCE::JSEngine::instance().isVariablePreInitialized(sessionId_, id)) {
        LOG_INFO("StateMachine: Skipping initialization for '{}' - pre-initialized by invoke data", id);
        return;
    }

    // W3C SCXML B.2.2: Late binding creates variables with undefined at init, assigns values on state entry
    if (!assignValue) {
        // Create variable with undefined value (both early and late binding)
        auto setVarFuture = SCE::JSEngine::instance().setVariable(sessionId_, id, ScriptValue{});
        auto setResult = setVarFuture.get();

        if (!SCE::JSEngine::isSuccess(setResult)) {
            LOG_ERROR("StateMachine: Failed to create unbound variable '{}': {}", id, setResult.getErrorMessage());
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution",
                                         "Failed to create variable '" + id + "': " + setResult.getErrorMessage());
            }
            return;
        }

        LOG_DEBUG("StateMachine: Created unbound variable '{}' (value assignment deferred for late binding)", id);
        return;
    }

    // Early binding or late binding value assignment: Evaluate and assign
    if (!expr.empty()) {
        // ARCHITECTURE.MD: Zero Duplication - Use DataModelInitHelper (shared with AOT engine)
        // W3C SCXML B.2: For function expressions, use direct JavaScript assignment to preserve function type
        // Test 453: ECMAScript function literals must be stored as functions, not converted to C++
        bool isFunctionExpression = DataModelInitHelper::isFunctionExpression(expr);

        if (isFunctionExpression) {
            // Use direct JavaScript assignment to avoid function → C++ → function conversion loss
            std::string assignmentScript = id + " = " + expr;
            auto scriptFuture = SCE::JSEngine::instance().executeScript(sessionId_, assignmentScript);
            auto scriptResult = scriptFuture.get();

            if (!SCE::JSEngine::isSuccess(scriptResult)) {
                LOG_ERROR("StateMachine: Failed to assign function expression '{}' to variable '{}': {}", expr, id,
                          scriptResult.getErrorMessage());
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", "Failed to assign function expression for '" + id +
                                                                    "': " + scriptResult.getErrorMessage());
                }
                return;
            }
            LOG_DEBUG("StateMachine: Initialized function variable '{}' from expression '{}'", id, expr);
        } else {
            // ARCHITECTURE.MD: Zero Duplication - Use DataModelInitHelper (shared with AOT engine)
            // W3C SCXML 5.2/5.3: Use initializeVariableFromExpr for expr attribute
            // Test 277: expr evaluation failure must raise error.execution (no fallback)
            bool success = DataModelInitHelper::initializeVariableFromExpr(
                SCE::JSEngine::instance(), sessionId_, id, expr, [this](const std::string &msg) {
                    // W3C SCXML 5.3: Raise error.execution on initialization failure
                    if (eventRaiser_) {
                        eventRaiser_->raiseEvent("error.execution", msg);
                    }
                    LOG_ERROR("StateMachine: {}", msg);
                });

            if (success) {
                LOG_DEBUG("StateMachine: Initialized variable '{}' from expression '{}'", id, expr);
            } else {
                // Leave variable unbound (don't create it) so it can be assigned later
                return;
            }
        }
    } else if (!src.empty()) {
        // W3C SCXML 5.3: Load data from external source (test 446)
        // ARCHITECTURE.MD: Zero Duplication - Use FileLoadingHelper (Single Source of Truth)

        std::string filePath = FileLoadingHelper::normalizePath(src);

        // Resolve relative path based on SCXML file location
        if (filePath[0] != '/') {  // Relative path
            std::string scxmlFilePath = SCE::JSEngine::instance().getSessionFilePath(sessionId_);
            if (!scxmlFilePath.empty()) {
                // Extract directory from SCXML file path
                size_t lastSlash = scxmlFilePath.find_last_of("/");
                if (lastSlash != std::string::npos) {
                    std::string directory = scxmlFilePath.substr(0, lastSlash + 1);
                    filePath = directory + filePath;
                }
            }
        }

        // Load file content using FileLoadingHelper
        std::string fileContent;
        bool success = FileLoadingHelper::loadFileContent(filePath, fileContent);

        if (!success) {
            LOG_ERROR("StateMachine: Failed to load file '{}' for variable '{}'", filePath, id);
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution",
                                         "Failed to load file '" + filePath + "' for variable '" + id + "'");
            }
            return;
        }

        // W3C SCXML B.2: Check content type (XML/JSON/text) and handle appropriately
        if (isXMLContent(fileContent)) {
            // W3C SCXML B.2 test 557: Parse XML content as DOM object
            LOG_DEBUG("StateMachine: Parsing XML content from file '{}' as DOM for variable '{}'", filePath, id);

            auto setVarFuture = SCE::JSEngine::instance().setVariableAsDOM(sessionId_, id, fileContent);
            auto setResult = setVarFuture.get();

            if (!SCE::JSEngine::isSuccess(setResult)) {
                LOG_ERROR("StateMachine: Failed to set XML content from file '{}' for variable '{}': {}", filePath, id,
                          setResult.getErrorMessage());
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", "Failed to set XML content from file '" + filePath +
                                                                    "' for '" + id +
                                                                    "': " + setResult.getErrorMessage());
                }
                return;
            }

            LOG_DEBUG("StateMachine: Set variable '{}' as XML DOM object from file '{}'", id, filePath);
        } else {
            // W3C SCXML B.2: Try evaluating as JSON/JS first (test 446), fall back to text (test 558)
            auto future = SCE::JSEngine::instance().evaluateExpression(sessionId_, fileContent);
            auto result = future.get();

            if (SCE::JSEngine::isSuccess(result)) {
                // Successfully evaluated as JSON/JS expression
                auto setVarFuture = SCE::JSEngine::instance().setVariable(sessionId_, id, result.getInternalValue());
                auto setResult = setVarFuture.get();

                if (!SCE::JSEngine::isSuccess(setResult)) {
                    LOG_ERROR("StateMachine: Failed to set variable '{}' from file '{}': {}", id, filePath,
                              setResult.getErrorMessage());
                    if (eventRaiser_) {
                        eventRaiser_->raiseEvent("error.execution", "Failed to set variable '" + id + "' from file '" +
                                                                        filePath + "': " + setResult.getErrorMessage());
                    }
                    return;
                }

                LOG_DEBUG("StateMachine: Initialized variable '{}' from file '{}'", id, filePath);
            } else {
                // W3C SCXML B.2 test 558: Non-JSON content - normalize whitespace and store as string
                std::string normalized = normalizeWhitespace(fileContent);

                auto setVarFuture = SCE::JSEngine::instance().setVariable(sessionId_, id, ScriptValue{normalized});
                auto setResult = setVarFuture.get();

                if (!SCE::JSEngine::isSuccess(setResult)) {
                    LOG_ERROR("StateMachine: Failed to set normalized text from file '{}' for variable '{}': {}",
                              filePath, id, setResult.getErrorMessage());
                    if (eventRaiser_) {
                        eventRaiser_->raiseEvent("error.execution", "Failed to set text content from file '" +
                                                                        filePath + "' for '" + id +
                                                                        "': " + setResult.getErrorMessage());
                    }
                    return;
                }

                LOG_DEBUG("StateMachine: Set variable '{}' with normalized text from file '{}': '{}'", id, filePath,
                          normalized);
            }
        }
    } else if (!content.empty()) {
        // W3C SCXML B.2: Initialize with inline content
        // ARCHITECTURE.md: Zero Duplication - Use DataModelInitHelper (shared with AOT engine)
        bool success = DataModelInitHelper::initializeVariable(SCE::JSEngine::instance(), sessionId_, id, content,
                                                               [this](const std::string &msg) {
                                                                   LOG_ERROR("StateMachine: {}", msg);
                                                                   if (eventRaiser_) {
                                                                       eventRaiser_->raiseEvent("error.execution", msg);
                                                                   }
                                                               });

        if (!success) {
            return;  // Error already handled by callback
        }

        LOG_DEBUG("StateMachine: Initialized variable '{}' from content", id);
    } else {
        // W3C SCXML 5.3: No expression or content - create variable with undefined value (test 445)
        auto setVarFuture = SCE::JSEngine::instance().setVariable(sessionId_, id, ScriptValue{});
        auto setResult = setVarFuture.get();

        if (!SCE::JSEngine::isSuccess(setResult)) {
            LOG_ERROR("StateMachine: Failed to create undefined variable '{}': {}", id, setResult.getErrorMessage());
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution",
                                         "Failed to create variable '" + id + "': " + setResult.getErrorMessage());
            }
            return;
        }

        LOG_DEBUG("StateMachine: Created variable '{}' with undefined value", id);
    }
}

bool StateMachine::initializeFromModel() {
    LOG_DEBUG("StateMachine: Initializing from SCXML model");

    // Clear existing state
    initialState_.clear();

    // Get initial state
    initialState_ = model_->getInitialState();

    // W3C SCXML 3.2: If no initial attribute, use first state in document order
    if (initialState_.empty()) {
        const auto &allStates = model_->getAllStates();
        if (allStates.empty()) {
            LOG_ERROR("StateMachine: No states found in SCXML model");
            return false;
        }

        // Auto-select first state in document order (W3C SCXML 3.2 compliance)
        initialState_ = allStates[0]->getId();
        LOG_DEBUG("StateMachine: No initial attribute found, auto-selected first state in document order: '{}'",
                  initialState_);
    }

    // Extract all states from the model
    const auto &allStates = model_->getAllStates();
    if (allStates.empty()) {
        LOG_ERROR("StateMachine: No states found in SCXML model");
        return false;
    }

    try {
        // Initialize hierarchy manager for hierarchical state support
        hierarchyManager_ = std::make_unique<StateHierarchyManager>(model_);

        // Set up onentry callback for W3C SCXML compliance
        LOG_DEBUG("StateMachine: Setting up onentry callback for StateHierarchyManager");
        hierarchyManager_->setOnEntryCallback([this](const std::string &stateId) {
            LOG_DEBUG("StateMachine: Onentry callback triggered for state: {}", stateId);
            executeOnEntryActions(stateId);
        });
        LOG_DEBUG("StateMachine: Onentry callback successfully configured");

        // W3C SCXML 6.4: Set up invoke defer callback for proper timing in parallel states
        LOG_DEBUG("StateMachine: Setting up invoke defer callback for StateHierarchyManager");
        hierarchyManager_->setInvokeDeferCallback(
            [this](const std::string &stateId, const std::vector<std::shared_ptr<IInvokeNode>> &invokes) {
                LOG_DEBUG("StateMachine: Invoke defer callback triggered for state: {} with {} invokes", stateId,
                          invokes.size());
                deferInvokeExecution(stateId, invokes);
            });
        LOG_DEBUG("StateMachine: Invoke defer callback successfully configured");

        // W3C SCXML: Set up condition evaluator callback for transition guard evaluation in parallel states
        LOG_DEBUG("StateMachine: Setting up condition evaluator callback for StateHierarchyManager");
        hierarchyManager_->setConditionEvaluator(
            [this](const std::string &condition) -> bool { return evaluateCondition(condition); });
        LOG_DEBUG("StateMachine: Condition evaluator callback successfully configured");

        // Set up completion callbacks for parallel states (SCXML W3C compliance)
        setupParallelStateCallbacks();

        // SCXML W3C Section 3.6: Auto-register history states from parsed model (SOLID architecture)
        initializeHistoryAutoRegistrar();
        if (historyAutoRegistrar_) {
            historyAutoRegistrar_->autoRegisterHistoryStates(model_, historyManager_.get());
        }

        LOG_DEBUG("Model initialized with initial state: {}", initialState_);
        LOG_INFO("Model initialized with {} states", allStates.size());
        return true;
    } catch (const std::exception &e) {
        LOG_ERROR("Failed to extract model: {}", e.what());
        return false;
    }
}

bool StateMachine::evaluateCondition(const std::string &condition) {
    if (condition.empty()) {
        LOG_DEBUG("Empty condition, returning true");
        return true;
    }

    try {
        LOG_DEBUG("Evaluating condition: '{}'", condition);

        auto future = SCE::JSEngine::instance().evaluateExpression(sessionId_, condition);
        auto result = future.get();

        if (!SCE::JSEngine::isSuccess(result)) {
            // W3C SCXML 5.9: Condition evaluation error must raise error.execution
            LOG_ERROR("W3C SCXML 5.9: Failed to evaluate condition '{}': {}", condition, result.getErrorMessage());

            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution", "Failed to evaluate condition: " + condition);
            }
            return false;
        }

        // Convert result to boolean using integrated JSEngine method
        bool conditionResult = SCE::JSEngine::resultToBool(result);
        LOG_DEBUG("Condition '{}' evaluated to: {}", condition, conditionResult ? "true" : "false");

        return conditionResult;

    } catch (const std::exception &e) {
        // W3C SCXML 5.9: Exception during condition evaluation must raise error.execution
        LOG_ERROR("W3C SCXML 5.9: Exception evaluating condition '{}': {}", condition, e.what());

        if (eventRaiser_) {
            eventRaiser_->raiseEvent("error.execution", "Exception evaluating condition: " + condition);
        }
        return false;
    }
}

bool StateMachine::enterState(const std::string &stateId) {
    LOG_DEBUG("[ENTER STATE CALLED] State: {}, isRunning: {}", stateId, isRunning_.load());
    LOG_DEBUG("Entering state: {}", stateId);

    // RAII guard against invalid reentrant calls
    // Automatically handles legitimate reentrant calls during event processing
    EnterStateGuard guard(isEnteringState_, isProcessingEvent_);

    // Early return for invalid reentrant calls (matches original behavior)
    if (guard.isInvalidCall()) {
        LOG_DEBUG("Invalid reentrant enterState call detected, ignoring: {}", stateId);
        return true;  // Return success to avoid breaking transition chain
    }

    // Check if this is a history state and handle restoration (SCXML W3C specification section 3.6)
    if (historyManager_ && historyManager_->isHistoryState(stateId)) {
        LOG_INFO("Entering history state: {}", stateId);

        // W3C SCXML 3.10: Restore history configuration and enter target states with ancestors
        auto restorationResult = historyManager_->restoreHistory(stateId);
        if (restorationResult.success && !restorationResult.targetStateIds.empty()) {
            LOG_INFO("History restoration successful, entering {} target states",
                     restorationResult.targetStateIds.size());

            // W3C SCXML 3.10 (test 579): Execute default transition actions BEFORE entering target state
            // "The processor MUST execute any executable content in the transition...
            //  However the Processor MUST execute this content only if there is no stored history"
            bool hasRecordedHistory = restorationResult.isRestoredFromRecording;
            if (!hasRecordedHistory && model_) {
                auto historyStateNode = model_->findStateById(stateId);
                if (historyStateNode) {
                    const auto &transitions = historyStateNode->getTransitions();
                    if (!transitions.empty()) {
                        // History state should have exactly one default transition
                        const auto &defaultTransition = transitions[0];
                        const auto &actions = defaultTransition->getActionNodes();
                        if (!actions.empty()) {
                            LOG_DEBUG("W3C SCXML 3.10: Executing {} default transition actions for history state {}",
                                      actions.size(), stateId);
                            executeActionNodes(actions, "history default transition");
                        }
                    }
                }
            }

            // Release guard before entering target states (allows recursive enterState calls)
            guard.release();

            // Enter all target states from history restoration
            // Use enterStateWithAncestors to ensure parent states are entered (test 387)
            bool allSucceeded = true;
            for (const auto &targetStateId : restorationResult.targetStateIds) {
                if (hierarchyManager_) {
                    // Enter target state along with all its ancestors
                    if (!hierarchyManager_->enterStateWithAncestors(targetStateId, nullptr)) {
                        LOG_ERROR("Failed to enter restored target state with ancestors: {}", targetStateId);
                        allSucceeded = false;
                    }
                } else {
                    // Fallback: use regular enterState if hierarchyManager not available
                    if (!enterState(targetStateId)) {
                        LOG_ERROR("Failed to enter restored target state: {}", targetStateId);
                        allSucceeded = false;
                    }
                }
            }

            return allSucceeded;
        } else {
            LOG_ERROR("History restoration failed: {}", restorationResult.errorMessage);
            // Guard will auto-clear on scope exit
            return false;
        }
    }

    // SCXML W3C specification: hierarchy manager is required for compliant state entry
    assert(hierarchyManager_ && "SCXML violation: hierarchy manager required for state management");

    // W3C SCXML 5.3: Late binding - assign values to state's data items when state is entered
    // W3C SCXML 5.3: Handle late binding initialization on state entry
    // Use BindingHelper (Single Source of Truth) for binding semantics
    if (model_) {
        const std::string &binding = model_->getBinding();
        bool isFirstEntry = (initializedStates_.find(stateId) == initializedStates_.end());

        if (isFirstEntry) {
            // First entry to this state - check if we need to initialize variables
            auto stateNode = model_->findStateById(stateId);
            if (stateNode) {
                const auto &stateDataItems = stateNode->getDataItems();
                if (!stateDataItems.empty()) {
                    LOG_DEBUG("StateMachine: First entry to state '{}' - checking {} data items for late binding",
                              stateId, stateDataItems.size());

                    for (const auto &item : stateDataItems) {
                        bool hasExpr = !item->getExpr().empty();

                        // Use BindingHelper to determine if value should be assigned on state entry
                        if (BindingHelper::shouldAssignValueOnStateEntry(binding, isFirstEntry, hasExpr)) {
                            // Late binding: assign value now
                            initializeDataItem(item, true);  // assignValue=true
                        }
                    }

                    initializedStates_.insert(stateId);  // Mark state as initialized
                }
            }
        }
    }

    bool hierarchyResult = hierarchyManager_->enterState(stateId);
    assert(hierarchyResult && "SCXML violation: state entry must succeed");
    (void)hierarchyResult;  // Suppress unused variable warning in release builds

    // SCXML W3C 3.4: For parallel states, activate regions AFTER parent onentry executed
    // This ensures correct entry sequence: parallel onentry -> child onentry
    if (model_) {
        auto stateNode = model_->findStateById(stateId);
        if (stateNode && stateNode->getType() == Type::PARALLEL) {
            auto parallelState = dynamic_cast<ConcurrentStateNode *>(stateNode);
            if (parallelState) {
                // Set ExecutionContext for region action execution
                if (executionContext_) {
                    parallelState->setExecutionContextForRegions(executionContext_);
                    LOG_DEBUG("SCXML compliant: Injected ExecutionContext into parallel state regions: {}", stateId);
                }

                // W3C SCXML 3.4: Activate all regions AFTER parallel state entered
                auto activationResults = parallelState->activateAllRegions();
                for (const auto &result : activationResults) {
                    if (!result.isSuccess) {
                        LOG_ERROR("Failed to activate region '{}': {}", result.regionId, result.errorMessage);
                    } else {
                        LOG_DEBUG("SCXML W3C: Activated region '{}' in parallel state '{}'", result.regionId, stateId);
                    }
                }

                // Check if all regions immediately reached final state (for done.state event)
                const auto &regions = parallelState->getRegions();
                bool allInFinalState =
                    !regions.empty() && std::all_of(regions.begin(), regions.end(), [](const auto &region) {
                        return region && region->isInFinalState();
                    });

                if (allInFinalState) {
                    LOG_DEBUG("SCXML W3C 3.4: All parallel regions in final state, triggering done.state event for {}",
                              stateId);
                    handleParallelStateCompletion(stateId);
                }
            }
        }
    }

    // SCXML W3C macrostep compliance: Check if reentrant transition occurred during state entry
    // This handles cases where onentry actions cause immediate transitions
    std::string actualCurrentState = getCurrentState();
    LOG_DEBUG("StateMachine: After entering '{}', getCurrentState() returns '{}'", stateId, actualCurrentState);
    if (actualCurrentState != stateId) {
        LOG_DEBUG("SCXML macrostep: State transition occurred during entry (expected: {}, actual: {})", stateId,
                  actualCurrentState);
        LOG_DEBUG("This indicates a valid internal transition (e.g., compound state entering initial child) - must "
                  "check eventless");

        // W3C SCXML 3.7: Check if actualCurrentState is a final state and generate done.state event
        // This handles compound states with initial attribute pointing to final child (test 372)
        if (model_) {
            auto currentStateNode = model_->findStateById(actualCurrentState);
            if (currentStateNode && currentStateNode->isFinalState()) {
                LOG_DEBUG("W3C SCXML 3.7: Current state '{}' is final, generating done.state event before early return",
                          actualCurrentState);
                handleCompoundStateFinalChild(actualCurrentState);
            }
        }

        // IMPORTANT: Release guard before checking eventless transitions
        guard.release();

        // W3C SCXML 3.3: Skip eventless transition check during initial configuration entry
        // This prevents premature transitions before all initial states are entered
        if (!isEnteringInitialConfiguration_) {
            // W3C SCXML: Check eventless transitions even on early return (initial child may have eventless
            // transitions)
            checkEventlessTransitions();
        }
        return true;
    }

    // W3C SCXML: onentry actions (including invokes) are executed via callback from StateHierarchyManager
    // This ensures proper execution order per W3C specification

    // NOTE: _state is not a W3C SCXML standard system variable (only _event, _sessionid, _name, _ioprocessors, _x
    // exist) Setting _state here causes issues with invoke lifecycle when child sessions terminate Removed to comply
    // with W3C SCXML 5.10 specification

    LOG_DEBUG("Successfully entered state using hierarchy manager: {} (current: {})", stateId, getCurrentState());

    // W3C SCXML 3.13: "If it has entered a final state that is a child of scxml, it MUST halt processing"
    // W3C SCXML 6.5: Invoke completion callback for invoked child StateMachines
    // IMPORTANT: ALL StateMachines must halt, but only invoked ones call completionCallback
    // IMPORTANT: Parallel states are NOT final states, even when all regions complete
    if (model_) {
        auto stateNode = model_->findStateById(actualCurrentState);
        if (stateNode && stateNode->isFinalState() && stateNode->getType() != SCE::Type::PARALLEL) {
            // Check if this is a top-level final state by checking parent chain
            // Top-level states have no parent or parent is the <scxml> root element
            // We need to traverse up to ensure we're not in a parallel region
            auto parent = stateNode->getParent();
            bool isTopLevel = false;

            // W3C SCXML 3.13: "a final state that is a child of scxml"
            // Top-level means parent is directly the <scxml> root element
            if (!parent) {
                // No parent means root-level final state
                isTopLevel = true;
            } else if (parent->getId() == "scxml") {
                // Parent is <scxml> root - this is top-level
                isTopLevel = true;
            }
            // All other cases (nested in compound states, parallel regions, etc.) are NOT top-level

            if (isTopLevel) {
                LOG_INFO("StateMachine: Reached top-level final state: {}, halting processing (W3C SCXML 3.13)",
                         actualCurrentState);

                // W3C SCXML 3.13: MUST halt processing when entering top-level final state
                isRunning_ = false;

                // W3C SCXML: Execute onexit actions BEFORE generating done.invoke
                // For top-level final states, onexit runs when state machine completes
                bool exitResult = executeExitActions(actualCurrentState);
                if (!exitResult) {
                    LOG_WARN("StateMachine: Failed to execute onexit for final state: {}", actualCurrentState);
                }

                // W3C SCXML 6.5: Callback is invoked AFTER onexit handlers execute (for invoked StateMachines)
                // This ensures correct event order: child events → done.invoke
                if (completionCallback_) {
                    try {
                        completionCallback_();
                    } catch (const std::exception &e) {
                        LOG_ERROR("StateMachine: Exception in completion callback: {}", e.what());
                    }
                }
            }
        }
    }

    // W3C SCXML 3.7 & 5.5: Generate done.state event for compound state completion
    if (model_) {
        auto stateNode = model_->findStateById(actualCurrentState);
        if (stateNode && stateNode->isFinalState()) {
            handleCompoundStateFinalChild(actualCurrentState);
        }
    }

    // Release guard - state entry complete
    guard.release();

    // W3C SCXML: Check for eventless transitions after state entry
    checkEventlessTransitions();

    return true;
}

bool StateMachine::executeTransitionDirect(IStateNode *sourceState, std::shared_ptr<ITransitionNode> transition) {
    if (!sourceState || !transition) {
        LOG_ERROR("StateMachine: Invalid parameters for executeTransitionDirect");
        return false;
    }

    // Execute the transition directly without re-evaluating its condition
    // This avoids side effects from conditions with mutations (e.g., ++var1 in W3C test 444)
    const auto &targets = transition->getTargets();
    bool isInternal = transition->isInternal();

    if (targets.empty() && !isInternal) {
        LOG_DEBUG("SCXML: Skipping transition with no targets (not internal)");
        return false;
    }

    std::string targetState = targets.empty() ? "" : targets[0];
    std::string fromState = sourceState->getId();

    // W3C SCXML: Internal transitions execute actions without exiting/entering states
    if (isInternal) {
        LOG_DEBUG("SCXML: Executing internal eventless transition actions (no state change)");
        const auto &actionNodes = transition->getActionNodes();
        if (!actionNodes.empty()) {
            if (!executeActionNodes(actionNodes, false)) {
                LOG_ERROR("StateMachine: Failed to execute internal transition actions");
                return false;
            }
        }
        return true;
    }

    // W3C SCXML: Compute and exit ALL states in the exit set
    ExitSetResult exitSetResult = computeExitSet(fromState, targetState);
    LOG_DEBUG("W3C SCXML: Exiting {} states for eventless transition {} -> {}", exitSetResult.states.size(), fromState,
              targetState);

    for (const std::string &stateToExit : exitSetResult.states) {
        if (!exitState(stateToExit)) {
            LOG_ERROR("Failed to exit state: {}", stateToExit);
            return false;
        }
    }

    // Execute transition actions
    const auto &actionNodes = transition->getActionNodes();
    if (!actionNodes.empty()) {
        LOG_DEBUG("SCXML: Executing eventless transition actions");
        if (!executeActionNodes(actionNodes, false)) {
            LOG_ERROR("StateMachine: Failed to execute transition actions");
            return false;
        }
    }

    // W3C SCXML 3.13: Enter states from LCA to target
    // Special case: history states use enterStateWithAncestors(), so skip enter set
    std::vector<std::string> enterSet;
    bool isHistoryTarget = historyManager_ && historyManager_->isHistoryState(targetState);

    if (!targetState.empty() && model_ && !isHistoryTarget) {
        auto targetNode = model_->findStateById(targetState);
        if (targetNode) {
            std::vector<std::string> statesToEnter;
            IStateNode *current = targetNode;
            while (current != nullptr) {
                std::string currentId = current->getId();
                if (currentId == exitSetResult.lca) {
                    break;
                }
                statesToEnter.push_back(currentId);
                current = current->getParent();
            }
            enterSet.assign(statesToEnter.rbegin(), statesToEnter.rend());
        }
    }

    // Enter all states in enter set
    for (const std::string &stateToEnter : enterSet) {
        if (!enterState(stateToEnter)) {
            LOG_ERROR("Failed to enter state: {}", stateToEnter);
            return false;
        }
    }

    // W3C SCXML 3.10: History states handle ancestors automatically
    if (isHistoryTarget) {
        if (!enterState(targetState)) {
            LOG_ERROR("Failed to enter history state: {}", targetState);
            return false;
        }
    }

    updateStatistics();
    stats_.totalTransitions++;

    // W3C SCXML 3.13: Track last executed transition for interactive visualizer
    // Only update if at deeper recursion level (preserves actual last transition in eventless chains)
    if (eventlessRecursionDepth_ == 0 || eventlessRecursionDepth_ > lastTransitionDepth_) {
        lastTransitionSource_ = fromState;
        lastTransitionTarget_ = targetState;
        lastTransitionDepth_ = eventlessRecursionDepth_;
        LOG_DEBUG("W3C SCXML 3.13: Eventless transition executed (depth {}): {} -> {}", eventlessRecursionDepth_,
                  fromState, targetState);
    } else {
        LOG_DEBUG("W3C SCXML 3.13: Eventless transition at depth {} skipped (preserving depth {}): {} -> {}",
                  eventlessRecursionDepth_, lastTransitionDepth_, fromState, targetState);
    }
    return true;
}

bool StateMachine::checkEventlessTransitions() {
    // Track recursion depth for visualizer transition tracking
    ++eventlessRecursionDepth_;

    // DEBUG: Log WHO is calling this function
    LOG_DEBUG("[CHECK EVENTLESS] checkEventlessTransitions() called, recursionDepth={}, isRunning_={}",
              eventlessRecursionDepth_, isRunning_.load());

    // W3C SCXML 3.13: Eventless Transition Selection Algorithm
    //
    // 1. For each active state (reverse document order):
    //    a. Find first enabled eventless transition (document order)
    //    b. Check if state is within a parallel state
    //    c. If parallel: collect transitions from ALL parallel regions (microstep)
    //    d. If not: execute single transition immediately
    // 2. Execute collected transitions atomically (exit all → execute all → enter all)
    //
    // Key Rule: Only the FIRST enabled transition per state is selected
    // Internal transitions count as "first" and prevent further checking

    if (!model_) {
        --eventlessRecursionDepth_;
        if (eventlessRecursionDepth_ == 0) {
            lastTransitionDepth_ = 0;
        }
        return false;
    }

    auto activeStates = hierarchyManager_->getActiveStates();
    LOG_DEBUG("SCXML: Checking eventless transitions on {} active state(s)", activeStates.size());

    // Performance: Cache state lookups to avoid repeated O(n) searches
    std::unordered_map<std::string, IStateNode *> stateCache;
    for (const auto &stateId : activeStates) {
        stateCache[stateId] = model_->findStateById(stateId);
    }

    IStateNode *firstEnabledState = nullptr;
    std::shared_ptr<ITransitionNode> firstTransition = nullptr;
    IStateNode *parallelAncestor = nullptr;

    // Find first enabled eventless transition
    for (auto it = activeStates.rbegin(); it != activeStates.rend(); ++it) {
        const std::string &activeStateId = *it;
        auto stateNode = stateCache[activeStateId];

        if (!stateNode) {
            continue;
        }

        const auto &transitions = stateNode->getTransitions();
        for (const auto &transitionNode : transitions) {
            const std::vector<std::string> &eventDescriptors = transitionNode->getEvents();
            if (!eventDescriptors.empty()) {
                continue;  // Not eventless
            }

            std::string condition = transitionNode->getGuard();
            bool conditionResult = condition.empty() || evaluateCondition(condition);

            if (conditionResult) {
                firstEnabledState = stateNode;
                firstTransition = transitionNode;

                // Check if this state is within a parallel state
                IStateNode *current = stateNode->getParent();
                while (current) {
                    if (current->getType() == Type::PARALLEL) {
                        parallelAncestor = current;
                        break;
                    }
                    current = current->getParent();
                }

                break;
            }
        }

        if (firstEnabledState) {
            break;
        }
    }

    if (!firstEnabledState) {
        LOG_DEBUG("SCXML: No eventless transitions found");
        --eventlessRecursionDepth_;
        if (eventlessRecursionDepth_ == 0) {
            lastTransitionDepth_ = 0;
        }
        return false;
    }

    // W3C SCXML 3.13: If not in parallel state, execute the already-selected transition
    // IMPORTANT: We already evaluated the condition, so we must not re-evaluate it
    // to avoid side effects (e.g., ++var1 would increment twice - W3C test 444)
    if (!parallelAncestor) {
        LOG_DEBUG("SCXML: Single eventless transition (non-parallel)");
        bool result = executeTransitionDirect(firstEnabledState, firstTransition);
        --eventlessRecursionDepth_;
        if (eventlessRecursionDepth_ == 0) {
            lastTransitionDepth_ = 0;
        }
        return result;
    }

    // W3C SCXML 3.13: Parallel state - collect ALL eventless transitions from all regions
    // Algorithm: For each active state in parallel, select first enabled transition (document order)
    LOG_DEBUG("W3C SCXML 3.13: Parallel state detected - collecting all region transitions");
    std::vector<TransitionInfo> enabledTransitions;
    enabledTransitions.reserve(activeStates.size());  // Optimize: pre-allocate for typical case

    for (auto it = activeStates.rbegin(); it != activeStates.rend(); ++it) {
        const std::string &activeStateId = *it;
        auto stateNode = stateCache[activeStateId];

        if (!stateNode) {
            continue;
        }

        // Check if this state is descendant of the same parallel ancestor
        bool isInParallel = false;
        IStateNode *current = stateNode;
        while (current) {
            if (current == parallelAncestor) {
                isInParallel = true;
                break;
            }
            current = current->getParent();
        }

        if (!isInParallel) {
            continue;
        }

        // W3C SCXML Appendix D.2: Collect all enabled transitions first
        // Conflict resolution will be applied after collection
        const auto &transitions = stateNode->getTransitions();
        for (const auto &transitionNode : transitions) {
            const std::vector<std::string> &eventDescriptors = transitionNode->getEvents();
            if (!eventDescriptors.empty()) {
                continue;
            }

            std::string condition = transitionNode->getGuard();
            bool conditionResult = condition.empty() || evaluateCondition(condition);

            if (!conditionResult) {
                continue;
            }

            const auto &targets = transitionNode->getTargets();
            if (targets.empty()) {
                // W3C SCXML: Internal transition - execute inline and stop checking this state
                // This is still the "first enabled transition" for this state
                const auto &actionNodes = transitionNode->getActionNodes();
                if (!actionNodes.empty()) {
                    executeActionNodes(actionNodes, false);
                }
                break;  // First enabled transition rule applies to internal transitions too
            }

            std::string targetState = targets[0];
            ExitSetResult exitSetResult = computeExitSet(activeStateId, targetState);

            enabledTransitions.emplace_back(stateNode, transitionNode, targetState, exitSetResult.states);
            LOG_DEBUG("W3C SCXML 3.13: Collected parallel transition: {} -> {}", activeStateId, targetState);

            // W3C SCXML: Only select first enabled transition per state (document order)
            break;
        }
    }

    if (enabledTransitions.empty()) {
        LOG_DEBUG("W3C SCXML 3.13: No transitions collected from parallel regions");
        return false;
    }

    // W3C SCXML Appendix D.2: Apply conflict resolution using shared Helper
    // ARCHITECTURE.MD: Zero Duplication - use ConflictResolutionHelper (Single Source of Truth)
    {
        using Helper = SCE::Common::ConflictResolutionHelperString;
        std::vector<Helper::TransitionDescriptor> descriptors;
        descriptors.reserve(enabledTransitions.size());

        // Convert to Helper format with exit sets
        for (const auto &trans : enabledTransitions) {
            Helper::TransitionDescriptor desc;
            desc.source = trans.sourceState->getId();
            desc.target = trans.targetState;
            desc.transitionIndex = static_cast<int>(descriptors.size());

            // Exit set already computed in computeExitSet()
            desc.exitSet = trans.exitSet;

            descriptors.push_back(desc);
        }

        // Apply W3C SCXML Appendix D.2 conflict resolution
        auto getParentFunc = [&stateCache](const std::string &stateId) -> std::optional<std::string> {
            auto stateNode = stateCache[stateId];
            if (!stateNode || !stateNode->getParent()) {
                return std::nullopt;
            }
            return stateNode->getParent()->getId();
        };

        auto isParallelStateFunc = [&stateCache](const std::string &stateId) -> bool {
            auto stateNode = stateCache[stateId];
            return stateNode && stateNode->getType() == Type::PARALLEL;
        };

        auto filtered = Helper::removeConflictingTransitions(descriptors, getParentFunc, isParallelStateFunc);

        // Rebuild enabledTransitions with filtered set
        std::vector<TransitionInfo> filteredTransitions;
        filteredTransitions.reserve(filtered.size());

        for (const auto &desc : filtered) {
            // Find original transition by matching source and target
            for (const auto &trans : enabledTransitions) {
                if (trans.sourceState->getId() == desc.source && trans.targetState == desc.target) {
                    filteredTransitions.push_back(trans);
                    break;
                }
            }
        }

        enabledTransitions = std::move(filteredTransitions);
        LOG_DEBUG("W3C SCXML Appendix D.2: After conflict resolution: {} transitions", enabledTransitions.size());
    }

    if (enabledTransitions.empty()) {
        LOG_DEBUG("W3C SCXML Appendix D.2: All transitions preempted by conflict resolution");
        --eventlessRecursionDepth_;
        if (eventlessRecursionDepth_ == 0) {
            lastTransitionDepth_ = 0;
        }
        return false;
    }

    // W3C SCXML 3.13: Sort by document order
    // Performance: Cache document positions to avoid O(n) tree traversal per comparison
    std::unordered_map<std::string, int> positionCache;
    for (const auto &trans : enabledTransitions) {
        const std::string &stateId = trans.sourceState->getId();
        if (positionCache.find(stateId) == positionCache.end()) {
            positionCache[stateId] = getStateDocumentPosition(stateId);
        }
    }

    std::sort(enabledTransitions.begin(), enabledTransitions.end(),
              [&positionCache](const TransitionInfo &a, const TransitionInfo &b) {
                  int posA = positionCache.at(a.sourceState->getId());
                  int posB = positionCache.at(b.sourceState->getId());
                  return posA < posB;
              });

    LOG_DEBUG("W3C SCXML 3.13: Executing {} parallel transitions as microstep", enabledTransitions.size());

    bool success = executeTransitionMicrostep(enabledTransitions);

    LOG_DEBUG("W3C SCXML 3.13: Microstep success = {}, parallelAncestor = {}", success,
              parallelAncestor ? parallelAncestor->getId() : "nullptr");

    if (success) {
        updateStatistics();
        stats_.totalTransitions += static_cast<int>(enabledTransitions.size());

        // W3C SCXML 3.4: Check if parallel state completed after eventless transitions
        // If all regions reached final states, generate done.state.{parallelId} event
        if (parallelAncestor) {
            LOG_DEBUG("W3C SCXML 3.4: Checking parallel ancestor '{}', type = {}", parallelAncestor->getId(),
                      static_cast<int>(parallelAncestor->getType()));

            if (parallelAncestor->getType() == Type::PARALLEL) {
                auto concurrentState = dynamic_cast<ConcurrentStateNode *>(parallelAncestor);
                LOG_DEBUG("W3C SCXML 3.4: dynamic_cast result = {}", concurrentState ? "success" : "failed");

                if (concurrentState) {
                    bool allComplete = concurrentState->areAllRegionsComplete();
                    LOG_DEBUG("W3C SCXML 3.4: areAllRegionsComplete() = {}", allComplete);

                    if (allComplete) {
                        std::string parallelId = parallelAncestor->getId();
                        std::string doneEventName = "done.state." + parallelId;

                        LOG_DEBUG("W3C SCXML 3.4: All parallel regions completed for state '{}' after eventless "
                                  "transitions, generating done.state event: {}",
                                  parallelId, doneEventName);

                        // W3C SCXML: Queue the done.state event (not immediate processing)
                        if (isRunning_ && eventRaiser_) {
                            eventRaiser_->raiseEvent(doneEventName, "");
                            LOG_DEBUG("W3C SCXML: Queued done.state event: {}", doneEventName);
                        }
                    }
                }
            }
        }
    }

    --eventlessRecursionDepth_;
    if (eventlessRecursionDepth_ == 0) {
        lastTransitionDepth_ = 0;
    }
    return success;
}

bool StateMachine::executeTransitionMicrostep(const std::vector<TransitionInfo> &transitions) {
    // ARCHITECTURE.MD: W3C SCXML Appendix D.2 Microstep Execution
    // Note: Interpreter engine uses dynamic node-based approach (runtime state IDs)
    // AOT engine uses ParallelTransitionHelper with static enum-based approach
    // Zero Duplication applies to algorithm structure, not implementation (different representations)

    if (transitions.empty()) {
        return false;
    }

    LOG_DEBUG("W3C SCXML 3.13: Executing microstep with {} transition(s)", transitions.size());

    // Set transition context flag (for history recording in exitState)
    // RAII guard ensures flag is cleared on all exit paths (normal return, error, exception)
    TransitionGuard transitionGuard(inTransition_);

    // W3C SCXML Appendix D.2 Step 1 & 2: Exit all source states (executing onexit actions)
    // ARCHITECTURE.MD: Algorithm structure shared with AOT engine (via ParallelTransitionHelper)
    // Compute unique exit set from all transitions, exit in W3C SCXML 3.13 order
    std::set<std::string> exitSetUnique;
    for (const auto &transInfo : transitions) {
        for (const auto &stateId : transInfo.exitSet) {
            exitSetUnique.insert(stateId);
        }
    }

    // Convert to vector for ordered exit (deepest first)
    std::vector<std::string> allStatesToExit(exitSetUnique.begin(), exitSetUnique.end());

    // Performance: Cache state lookups and depths to avoid repeated parent chain traversal
    std::unordered_map<std::string, IStateNode *> exitStateCache;
    std::unordered_map<std::string, int> depthCache;

    for (const auto &stateId : allStatesToExit) {
        auto node = model_->findStateById(stateId);
        exitStateCache[stateId] = node;

        // Pre-calculate depth once for O(1) lookup during sort
        int depth = 0;
        if (node) {
            auto parent = node->getParent();
            while (parent) {
                depth++;
                parent = parent->getParent();
            }
        }
        depthCache[stateId] = depth;
    }

    // W3C SCXML 3.13: Sort by depth (deepest first), then by reverse document order
    // ARCHITECTURE.MD: Zero Duplication - Use ParallelTransitionHelper (shared with AOT engine)
    // Performance: Cache document positions for O(1) lookup during sort
    std::unordered_map<std::string, int> positionCache;
    for (const auto &stateId : allStatesToExit) {
        positionCache[stateId] = getStateDocumentPosition(stateId);
    }

    allStatesToExit = ParallelTransitionHelper::sortStatesForExit<std::string>(
        allStatesToExit, [&depthCache](const std::string &stateId) { return depthCache.at(stateId); },
        [&positionCache](const std::string &stateId) { return positionCache.at(stateId); });

    // W3C SCXML 3.6 (test 580): Record history BEFORE exiting states
    // History must be recorded while all descendants are still active
    // Only record for states that actually have history children
    if (historyManager_ && hierarchyManager_) {
        auto currentActiveStates = hierarchyManager_->getActiveStates();
        for (const auto &stateToExit : allStatesToExit) {
            auto stateNode = exitStateCache[stateToExit];
            if (stateNode && (stateNode->getType() == Type::COMPOUND || stateNode->getType() == Type::PARALLEL)) {
                // Check if this state has history children
                bool hasHistoryChildren = false;
                for (const auto &child : stateNode->getChildren()) {
                    if (child->getType() == Type::HISTORY) {
                        hasHistoryChildren = true;
                        break;
                    }
                }

                // Only record history if this state has history children
                if (hasHistoryChildren) {
                    bool recorded = historyManager_->recordHistory(stateToExit, currentActiveStates);
                    if (recorded) {
                        LOG_DEBUG("Pre-recorded history for state '{}' before microstep exit (W3C SCXML 3.6, test 580)",
                                  stateToExit);
                    }
                }
            }
        }
    }

    LOG_DEBUG("W3C SCXML 3.13: Exiting {} state(s)", allStatesToExit.size());
    for (const auto &stateId : allStatesToExit) {
        if (!exitState(stateId)) {
            LOG_ERROR("W3C SCXML 3.13: Failed to exit state '{}' during microstep", stateId);
            return false;
        }
    }

    // W3C SCXML Appendix D.2 Step 3: Execute all transition actions in document order
    // ARCHITECTURE.MD: Algorithm structure same as AOT engine (different execution method)
    LOG_DEBUG("W3C SCXML 3.13: Executing transition actions for {} transition(s)", transitions.size());
    for (const auto &transInfo : transitions) {
        const auto &actionNodes = transInfo.transition->getActionNodes();
        if (!actionNodes.empty()) {
            // W3C SCXML 5.10: Protect _event during transition action execution (Test 230)
            // Save current event context before executing actions to prevent corruption by nested events
            EventMetadata savedEvent;
            if (actionExecutor_) {
                auto actionExecutorImpl = std::dynamic_pointer_cast<ActionExecutorImpl>(actionExecutor_);
                if (actionExecutorImpl) {
                    savedEvent = actionExecutorImpl->getCurrentEvent();
                }
            }

            LOG_DEBUG("W3C SCXML 3.13: Executing {} action(s) from transition", actionNodes.size());
            // processEventsAfter=false: Events raised here will be queued, not processed immediately
            executeActionNodes(actionNodes, false);

            // W3C SCXML 5.10: Restore _event after transition action execution
            if (actionExecutor_) {
                auto actionExecutorImpl = std::dynamic_pointer_cast<ActionExecutorImpl>(actionExecutor_);
                if (actionExecutorImpl) {
                    actionExecutorImpl->setCurrentEvent(savedEvent);
                }
            }
        }
    }

    // W3C SCXML Appendix D.2 Step 4-5: Enter all target states (executing onentry actions)
    // ARCHITECTURE.MD: Algorithm structure same as AOT engine (different execution method)
    LOG_DEBUG("W3C SCXML 3.13: Entering {} target state(s)", transitions.size());
    for (const auto &transInfo : transitions) {
        if (!transInfo.targetState.empty()) {
            if (!enterState(transInfo.targetState)) {
                LOG_ERROR("W3C SCXML 3.13: Failed to enter target state '{}' during microstep", transInfo.targetState);
                return false;
            }
        }
    }

    LOG_DEBUG("W3C SCXML 3.13: Microstep execution complete");
    return true;
}

bool StateMachine::exitState(const std::string &stateId) {
    LOG_DEBUG("Exiting state: {}", stateId);

    // W3C SCXML 3.13: Parallel states exit actions are handled by StateHierarchyManager (test 404)
    // Regions exit first, then parallel state's onexit is executed
    // Non-parallel states execute exit actions here
    auto stateNode = model_->findStateById(stateId);
    if (stateNode && stateNode->getType() != Type::PARALLEL) {
        // Execute IActionNode-based exit actions for non-parallel states only
        bool exitResult = executeExitActions(stateId);
        if (!exitResult && isRunning_) {
            // Only log error if machine is still running - during shutdown, raise failures are expected
            LOG_ERROR("StateMachine: Failed to execute exit actions for state: {}", stateId);
        }
        (void)exitResult;  // Suppress unused variable warning in release builds
    }

    // Get state node for invoke cancellation and history recording
    auto stateNodeForCleanup = model_->findStateById(stateId);

    // W3C SCXML specification section 3.13: Cancel invokes BEFORE removing from active states
    // "Then it MUST cancel any ongoing invocations that were triggered by that state"
    // This must happen AFTER onexit handlers but BEFORE state removal
    if (stateNodeForCleanup && invokeExecutor_) {
        const auto &invokes = stateNodeForCleanup->getInvoke();
        LOG_DEBUG("StateMachine::exitState - State '{}' has {} invoke(s) to check", stateId, invokes.size());

        for (const auto &invoke : invokes) {
            const std::string &invokeid = invoke->getId();
            if (!invokeid.empty()) {
                bool isActive = invokeExecutor_->isInvokeActive(invokeid);
                LOG_DEBUG("StateMachine::exitState - Invoke '{}' isActive: {}", invokeid, isActive);

                if (isActive) {
                    LOG_DEBUG("StateMachine: Cancelling active invoke '{}' due to state exit: {}", invokeid, stateId);
                    bool cancelled = invokeExecutor_->cancelInvoke(invokeid);
                    LOG_DEBUG("StateMachine: Cancel result for invoke '{}': {}", invokeid, cancelled);
                } else {
                    LOG_DEBUG("StateMachine: NOT cancelling inactive invoke '{}' (may be completing naturally)",
                              invokeid);
                }
            } else {
                LOG_WARN("StateMachine::exitState - Found invoke with empty ID in state '{}'", stateId);
            }
        }
    } else {
        if (!stateNodeForCleanup) {
            LOG_DEBUG("StateMachine::exitState - stateNodeForCleanup is null for state '{}'", stateId);
        }
        if (!invokeExecutor_) {
            LOG_DEBUG("StateMachine::exitState - invokeExecutor_ is null");
        }
    }

    // W3C SCXML 3.6: History recording (test 388)
    // In transition context: History is pre-recorded before exit set execution
    // Outside transition context (direct exitState call): Record history now as fallback
    if (!inTransition_ && historyManager_ && hierarchyManager_) {
        auto currentActiveStates = hierarchyManager_->getActiveStates();
        if (stateNode && (stateNode->getType() == Type::COMPOUND || stateNode->getType() == Type::PARALLEL)) {
            bool recorded = historyManager_->recordHistory(stateId, currentActiveStates);
            if (recorded) {
                LOG_DEBUG("Fallback: Recorded history for state '{}' (direct exitState call)", stateId);
            }
        }
    }

    // W3C SCXML section 3.13: Finally remove the state from active states list
    // Use hierarchy manager for SCXML-compliant state exit
    assert(hierarchyManager_ && "SCXML violation: hierarchy manager required for state management");
    LOG_DEBUG("StateMachine::exitState - executionContext_ is {}", executionContext_ ? "valid" : "NULL");
    hierarchyManager_->exitState(stateId, executionContext_);

    // State management fully delegated to StateHierarchyManager

    LOG_DEBUG("Successfully exited state: {}", stateId);
    return true;
}

bool StateMachine::ensureJSEnvironment() {
    if (jsEnvironmentReady_) {
        return true;
    }

    return setupJSEnvironment();
}

bool StateMachine::setupJSEnvironment() {
    // JSEngine automatically initialized in constructor (RAII)
    auto &jsEngine = SCE::JSEngine::instance();  // RAII guaranteed
    LOG_DEBUG("StateMachine: JSEngine automatically initialized via RAII at address: {}",
              static_cast<void *>(&jsEngine));

    // Create JavaScript session only if it doesn't exist (for invoke scenarios)
    // Check if session already exists (created by InvokeExecutor for child sessions)
    bool sessionExists = SCE::JSEngine::instance().hasSession(sessionId_);

    if (!sessionExists) {
        // Create new session for standalone StateMachine
        if (!SCE::JSEngine::instance().createSession(sessionId_)) {
            LOG_ERROR("StateMachine: Failed to create JavaScript session");
            return false;
        }
        LOG_DEBUG("StateMachine: Created new JavaScript session: {}", sessionId_);
    } else {
        LOG_DEBUG("StateMachine: Using existing JavaScript session (injected): {}", sessionId_);
    }

    // W3C SCXML 5.10: Set up read-only system variables (_sessionid, _name, _ioprocessors)
    // ARCHITECTURE.md Zero Duplication: SystemVariableHelper provides Single Source of Truth
    std::string sessionName = model_ && !model_->getName().empty() ? model_->getName() : "StateMachine";
    std::vector<std::string> ioProcessors = {"scxml"};  // W3C SCXML I/O Processors
    auto setupResult = SCE::SystemVariableHelper::setupSystemVariables(sessionId_, sessionName, ioProcessors).get();
    if (!setupResult.isSuccess()) {
        LOG_ERROR("StateMachine: Failed to setup system variables: {}", setupResult.getErrorMessage());
        return false;
    }

    // Register this StateMachine instance with JSEngine for In() function support
    // RACE CONDITION FIX: Use shared_from_this() to enable weak_ptr safety
    // W3C Test 530: Prevents heap-use-after-free during invoke child destruction
    SCE::JSEngine::instance().setStateMachine(shared_from_this(), sessionId_);
    LOG_DEBUG("StateMachine: Registered with JSEngine for In() function support");

    // W3C SCXML 5.3: Initialize data model with binding mode support (early/late binding)
    // Use BindingHelper (Single Source of Truth) for binding semantics
    if (model_) {
        // Collect all data items (top-level + state-level) for global scope
        const auto allDataItems = collectAllDataItems();
        const std::string &binding = model_->getBinding();
        LOG_INFO("StateMachine: Initializing {} total data items (global scope with {} binding)", allDataItems.size(),
                 binding.empty() ? "early (default)" : binding);

        // Use BindingHelper to determine initialization strategy
        // This ensures W3C SCXML 5.3 compliance through shared logic with AOT engine
        bool shouldAssignValue = BindingHelper::shouldAssignValueAtDocumentLoad(binding);

        for (const auto &dataInfo : allDataItems) {
            // Always call initializeDataItem (handles expr/src/content/undefined)
            // The assignValue flag controls whether to evaluate expr/src/content or use undefined
            initializeDataItem(dataInfo.dataItem, shouldAssignValue);
        }

        if (BindingHelper::isLateBinding(binding)) {
            LOG_DEBUG("StateMachine: Late binding mode - values will be assigned on state entry");
        } else {
            LOG_DEBUG("StateMachine: Early binding mode - all values assigned at init");
        }
    } else {
        LOG_DEBUG("StateMachine: No model available for data model initialization");
    }

    // Initialize ActionExecutor and ExecutionContext (needed for script execution)
    if (!initializeActionExecutor()) {
        LOG_ERROR("StateMachine: Failed to initialize action executor");
        return false;
    }

    // W3C SCXML 403c: Set execution context for concurrent region action execution
    // This must happen AFTER executionContext_ is created in initializeActionExecutor()
    if (hierarchyManager_ && executionContext_) {
        hierarchyManager_->setExecutionContext(executionContext_);
        LOG_DEBUG("StateMachine: ExecutionContext successfully configured for StateHierarchyManager (403c compliance)");

        // W3C SCXML 3.13: Set initial transition callback for proper event queuing
        hierarchyManager_->setInitialTransitionCallback(
            [this](const std::vector<std::shared_ptr<IActionNode>> &actions) {
                // Execute actions with immediate mode control to ensure proper event queuing
                executeActionNodes(actions, false);
            });
        LOG_DEBUG(
            "StateMachine: Initial transition callback configured for StateHierarchyManager (test 412 compliance)");

        // W3C SCXML 3.10: Set history manager for direct restoration (test 579)
        // This avoids EnterStateGuard issues from reentrant enterState calls
        hierarchyManager_->setHistoryManager(historyManager_.get());
        LOG_DEBUG("StateMachine: History manager configured for StateHierarchyManager (test 579 compliance)");
    }

    // W3C SCXML 5.8: Execute top-level scripts AFTER datamodel init, BEFORE start()
    if (model_) {
        const auto &topLevelScripts = model_->getTopLevelScripts();
        if (!topLevelScripts.empty()) {
            LOG_INFO("StateMachine: Executing {} top-level script(s) at document load time (W3C SCXML 5.8)",
                     topLevelScripts.size());

            for (size_t i = 0; i < topLevelScripts.size(); ++i) {
                const auto &script = topLevelScripts[i];

                if (!script) {
                    LOG_WARN("StateMachine: Null script at index {} - skipping (W3C SCXML 5.8)", i);
                    continue;
                }

                if (!executionContext_) {
                    LOG_ERROR("StateMachine: ExecutionContext is null - cannot execute scripts (W3C SCXML 5.8)");
                    return false;
                }

                LOG_DEBUG("StateMachine: Executing top-level script #{} (W3C SCXML 5.8)", i + 1);
                bool success = script->execute(*executionContext_);
                if (!success) {
                    LOG_ERROR("StateMachine: Top-level script #{} execution failed (W3C SCXML 5.8) - document rejected",
                              i + 1);
                    return false;  // W3C SCXML 5.8: Script failure rejects document
                }
            }
            LOG_DEBUG("StateMachine: All {} top-level script(s) executed successfully (W3C SCXML 5.8)",
                      topLevelScripts.size());
        }
    }

    // Pass EventDispatcher to ActionExecutor if it was set before initialization
    if (eventDispatcher_ && actionExecutor_) {
        auto actionExecutorImpl = std::dynamic_pointer_cast<ActionExecutorImpl>(actionExecutor_);
        if (actionExecutorImpl) {
            actionExecutorImpl->setEventDispatcher(eventDispatcher_);
            LOG_DEBUG(
                "StateMachine: EventDispatcher passed to ActionExecutor during JS environment setup for session: {}",
                sessionId_);
        }
    }

    // Pass EventRaiser to ActionExecutor if available
    if (eventRaiser_ && actionExecutor_) {
        actionExecutor_->setEventRaiser(eventRaiser_);
        LOG_DEBUG("StateMachine: EventRaiser passed to ActionExecutor for session: {}", sessionId_);
    }

    // W3C SCXML: Auto-initialize EventRaiser if not already set (for standalone StateMachine)
    // This ensures done.state events can be queued during start() when parallel regions complete
    if (!eventRaiser_) {
        auto eventRaiser = std::make_shared<EventRaiserImpl>();
        setEventRaiser(eventRaiser);
        EventRaiserService::getInstance().registerEventRaiser(sessionId_, eventRaiser);
        LOG_DEBUG("StateMachine: Auto-initialized EventRaiser for session: {}", sessionId_);
    }

    // Register EventRaiser with JSEngine after session creation
    // This handles both cases: EventRaiser set before session creation (deferred) and after
    if (eventRaiser_) {
        // Use EventRaiserService for centralized registration
        if (EventRaiserService::getInstance().registerEventRaiser(sessionId_, eventRaiser_)) {
            LOG_DEBUG("StateMachine: EventRaiser registered via Service after session creation for session: {}",
                      sessionId_);
        } else {
            LOG_DEBUG("StateMachine: EventRaiser already registered for session: {}", sessionId_);
        }
    }

    jsEnvironmentReady_ = true;
    LOG_DEBUG("StateMachine: JavaScript environment setup completed");
    return true;
}

void StateMachine::updateStatistics() {
    stats_.currentState = getCurrentState();
    stats_.isRunning = isRunning_.load();
}

bool StateMachine::initializeActionExecutor() {
    try {
        // Create ActionExecutor using the same session as StateMachine
        actionExecutor_ = std::make_shared<ActionExecutorImpl>(sessionId_);

        // Cache the pointer to avoid dynamic_pointer_cast overhead in hot path
        cachedExecutorImpl_ = dynamic_cast<ActionExecutorImpl *>(actionExecutor_.get());

        // Inject EventRaiser if already set via builder pattern
        if (eventRaiser_) {
            actionExecutor_->setEventRaiser(eventRaiser_);
            LOG_DEBUG("StateMachine: EventRaiser injected to ActionExecutor during initialization for session: {}",
                      sessionId_);
        }

        // Create ExecutionContext with shared_ptr and sessionId
        executionContext_ = std::make_shared<ExecutionContextImpl>(actionExecutor_, sessionId_);

        LOG_DEBUG("ActionExecutor and ExecutionContext initialized for session: {}", sessionId_);
        return true;
    } catch (const std::exception &e) {
        LOG_ERROR("Failed to initialize ActionExecutor: {}", e.what());
        return false;
    }
}

bool StateMachine::executeActionNodes(const std::vector<std::shared_ptr<SCE::IActionNode>> &actions,
                                      bool processEventsAfter) {
    if (!executionContext_) {
        LOG_WARN("StateMachine: ExecutionContext not initialized, skipping action node execution");
        return true;  // Not a failure, just no actions to execute
    }

    bool allSucceeded = true;

    // W3C SCXML compliance: Set immediate mode to false during executable content execution
    // This ensures events raised during execution are queued and processed after completion
    if (eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            eventRaiserImpl->setImmediateMode(false);
            LOG_DEBUG("SCXML compliance: Set immediate mode to false for executable content execution");
        }
    }

    for (const auto &action : actions) {
        if (!action) {
            LOG_WARN("StateMachine: Null action node encountered, skipping");
            continue;
        }

        try {
            LOG_DEBUG("Executing action: {}", action->getActionType());
            if (action->execute(*executionContext_)) {
                LOG_DEBUG("Successfully executed action: {}", action->getActionType());
            } else {
                LOG_WARN("Failed to execute action: {} - W3C compliance: stopping remaining actions",
                         action->getActionType());
                allSucceeded = false;
                // W3C SCXML specification: If error occurs in executable content,
                // processor MUST NOT process remaining elements in the block
                break;
            }
        } catch (const std::exception &e) {
            LOG_WARN("Exception executing action {}: {} - W3C compliance: stopping remaining actions",
                     action->getActionType(), e.what());
            allSucceeded = false;
            // W3C SCXML specification: If error occurs in executable content,
            // processor MUST NOT process remaining elements in the block
            break;
        }
    }

    // W3C SCXML compliance: Restore immediate mode and optionally process queued events
    if (eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            eventRaiserImpl->setImmediateMode(true);
            // Process events only if requested (e.g., for entry actions, not exit/transition actions)
            if (processEventsAfter) {
                eventRaiserImpl->processQueuedEvents();
                LOG_DEBUG("SCXML compliance: Restored immediate mode and processed queued events");
            } else {
                LOG_DEBUG("SCXML compliance: Restored immediate mode (events will be processed later)");
            }
        }
    }

    // W3C SCXML compliance: Return true only if all actions succeeded or no actions to execute
    // If any action failed, we stopped execution per W3C spec, so return false to indicate failure
    return actions.empty() || allSucceeded;
}

bool StateMachine::executeEntryActions(const std::string &stateId) {
    if (!model_) {
        assert(false && "SCXML violation: StateMachine must have a model for entry action execution");
        return false;
    }

    // Find the StateNode in the SCXML model
    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        // SCXML W3C compliance: All states in active configuration must exist in model
        assert(false && "SCXML violation: Active state not found in model");
        return false;
    }

    LOG_DEBUG("Executing entry actions for state: {}", stateId);

    // SCXML W3C specification section 3.4: Parallel states require special handling
    if (stateNode->getType() == Type::PARALLEL) {
        auto parallelState = dynamic_cast<ConcurrentStateNode *>(stateNode);
        assert(parallelState && "SCXML violation: PARALLEL type state must be ConcurrentStateNode");

        // W3C SCXML 3.8: Execute parallel state's own onentry action blocks FIRST
        const auto &parallelEntryBlocks = parallelState->getEntryActionBlocks();
        if (!parallelEntryBlocks.empty()) {
            LOG_DEBUG("W3C SCXML 3.8: executing {} entry action blocks for parallel state itself: {}",
                      parallelEntryBlocks.size(), stateId);
            for (size_t i = 0; i < parallelEntryBlocks.size(); ++i) {
                if (!executeActionNodes(parallelEntryBlocks[i])) {
                    LOG_WARN("W3C SCXML 3.8: Parallel entry block {}/{} failed, continuing", i + 1,
                             parallelEntryBlocks.size());
                }
            }
        }

        // provide ExecutionContext to all regions for action execution
        if (executionContext_) {
            parallelState->setExecutionContextForRegions(executionContext_);
            LOG_DEBUG("Injected ExecutionContext into all regions of parallel state: {}", stateId);
        }

        // SCXML W3C specification: ALL child regions MUST have their entry actions executed AFTER parallel state
        const auto &regions = parallelState->getRegions();
        assert(!regions.empty() && "SCXML violation: parallel state must have at least one region");

        LOG_DEBUG("SCXML W3C compliant - executing entry actions for {} child regions in parallel state: {}",
                  regions.size(), stateId);

        // Execute entry actions for each region's root state
        for (const auto &region : regions) {
            assert(region && "SCXML violation: parallel state cannot have null regions");

            auto rootState = region->getRootState();
            assert(rootState && "SCXML violation: region must have root state");

            // W3C SCXML 3.8: Execute entry action blocks for the region's root state
            const auto &regionEntryBlocks = rootState->getEntryActionBlocks();
            if (!regionEntryBlocks.empty()) {
                LOG_DEBUG("W3C SCXML 3.8: executing {} entry action blocks for region: {}", regionEntryBlocks.size(),
                          region->getId());
                for (size_t i = 0; i < regionEntryBlocks.size(); ++i) {
                    if (!executeActionNodes(regionEntryBlocks[i])) {
                        LOG_WARN("W3C SCXML 3.8: Region entry block {}/{} failed, continuing", i + 1,
                                 regionEntryBlocks.size());
                    }
                }
            }

            // SCXML W3C specification: Enter initial child states of each region ONLY if not already active
            const auto &children = rootState->getChildren();
            if (!children.empty()) {
                // SCXML W3C compliance: Do not re-enter initial state if parallel region already active
                if (!region->isActive()) {
                    std::string initialChild = rootState->getInitialState();
                    if (initialChild.empty()) {
                        // SCXML W3C: Use first child as default initial state
                        initialChild = children[0]->getId();
                    }

                    LOG_DEBUG("Entering initial child state for INACTIVE region {}: {}", region->getId(), initialChild);

                    // W3C SCXML 3.8: Execute entry action blocks for the initial child state
                    auto childState = model_->findStateById(initialChild);
                    if (childState) {
                        const auto &childEntryBlocks = childState->getEntryActionBlocks();
                        if (!childEntryBlocks.empty()) {
                            LOG_DEBUG("W3C SCXML 3.8: executing {} entry action blocks for initial child state: {}",
                                      childEntryBlocks.size(), initialChild);
                            for (size_t i = 0; i < childEntryBlocks.size(); ++i) {
                                if (!executeActionNodes(childEntryBlocks[i])) {
                                    LOG_WARN("W3C SCXML 3.8: Child entry block {}/{} failed, continuing", i + 1,
                                             childEntryBlocks.size());
                                }
                            }
                        }
                    }
                } else {
                    // SCXML W3C compliance: Already active region does not re-enter initial state
                    auto concreteRegion = std::dynamic_pointer_cast<ConcurrentRegion>(region);
                    std::string currentState = concreteRegion ? concreteRegion->getCurrentState() : "unknown";

                    LOG_DEBUG("SCXML W3C compliance - skipping initial state entry for already ACTIVE region: {} "
                              "(current state: {})",
                              region->getId(), currentState);

                    // Prevent SCXML W3C violation: Maintain current state of already active region
                    assert(concreteRegion && !concreteRegion->getCurrentState().empty() &&
                           "SCXML violation: active region must have current state");

                    // Verify SCXML W3C compliance: Ensure active region not reset to initial state
                    assert(region->isActive() &&
                           "SCXML violation: region marked as active but isActive() returns false");

                    // Detect SCXML W3C violation: Verify state consistency on parallel state re-entry
                    const auto &currentActiveStates = region->getActiveStates();
                    assert(!currentActiveStates.empty() && "SCXML violation: active region must have active states");
                }
            }
        }

        return true;
    }

    // W3C SCXML 3.8: Execute block-based entry actions for non-parallel states
    const auto &entryBlocks = stateNode->getEntryActionBlocks();
    if (!entryBlocks.empty()) {
        LOG_DEBUG("W3C SCXML 3.8: Executing {} entry action blocks for state: {}", entryBlocks.size(), stateId);

        for (size_t i = 0; i < entryBlocks.size(); ++i) {
            LOG_DEBUG("W3C SCXML 3.8: Executing entry action block {}/{} for state: {}", i + 1, entryBlocks.size(),
                      stateId);

            // W3C SCXML 3.8: Each onentry handler is a separate block
            // If one block fails, continue with remaining blocks
            if (!executeActionNodes(entryBlocks[i])) {
                LOG_WARN("W3C SCXML 3.8: Entry action block {}/{} failed, continuing with remaining blocks", i + 1,
                         entryBlocks.size());
                // Don't break - continue with next block per W3C spec
            }
        }

        // W3C SCXML: State entry succeeds even if some action blocks fail
        return true;
    }

    return true;
}

bool StateMachine::executeExitActions(const std::string &stateId) {
    if (!model_) {
        return true;  // No model, no actions to execute
    }

    // Find the StateNode in the SCXML model
    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        LOG_DEBUG("State {} not found in SCXML model, skipping exit actions", stateId);
        return true;  // Not an error if state not found in model
    }

    // SCXML W3C specification section 3.4: Parallel states require special exit sequence
    if (stateNode->getType() == Type::PARALLEL) {
        auto parallelState = dynamic_cast<ConcurrentStateNode *>(stateNode);
        assert(parallelState && "SCXML violation: PARALLEL type state must be ConcurrentStateNode");

        LOG_DEBUG("SCXML W3C compliant - executing exit sequence for parallel state: {}", stateId);

        // W3C SCXML 3.13: Skip region exit actions if regions are already in exit set (test 504)
        // Child regions will execute their own exit actions when their exitState() is called
        // Only execute parallel state's own exit actions here

        // W3C SCXML 3.9: Execute parallel state's own onexit action blocks
        const auto &parallelExitBlocks = parallelState->getExitActionBlocks();
        if (!parallelExitBlocks.empty()) {
            LOG_DEBUG("W3C SCXML 3.9: executing {} exit action blocks for parallel state itself: {}",
                      parallelExitBlocks.size(), stateId);

            // W3C SCXML 3.9: Build lambda blocks for EntryExitHelper
            // ARCHITECTURE.md Zero Duplication: Delegate to shared Helper (lines 311-373)
            std::vector<std::function<void()>> exitLambdas;
            for (const auto &exitBlock : parallelExitBlocks) {
                exitLambdas.push_back([&, exitBlock]() {
                    if (!executeActionNodes(exitBlock, false)) {
                        LOG_WARN("W3C SCXML 3.9: Parallel exit block failed");
                        // Lambda return stops THIS block only, next block continues
                    }
                });
            }

            // W3C SCXML 3.9: Delegate to EntryExitHelper (Single Source of Truth)
            EntryExitHelper<InterpreterPolicy, IEventRaiser>::executeExitBlocks(exitLambdas, *eventRaiser_, stateId);
        }

        return true;
    }

    // W3C SCXML 3.9: Execute block-based exit actions for non-parallel states
    const auto &exitBlocks = stateNode->getExitActionBlocks();
    if (!exitBlocks.empty()) {
        LOG_DEBUG("W3C SCXML 3.9: Executing {} exit action blocks for state: {}", exitBlocks.size(), stateId);

        // W3C SCXML 3.9: Build lambda blocks for EntryExitHelper
        // ARCHITECTURE.md Zero Duplication: Delegate to shared Helper (lines 311-373)
        std::vector<std::function<void()>> exitLambdas;
        for (const auto &exitBlock : exitBlocks) {
            exitLambdas.push_back([&, exitBlock]() {
                // W3C SCXML 3.9: Each onexit handler is a separate block
                if (!executeActionNodes(exitBlock, false)) {
                    LOG_WARN("W3C SCXML 3.9: Exit action block failed, continuing with remaining blocks");
                    // Lambda return stops THIS block only, next block continues per W3C spec
                }
            });
        }

        // W3C SCXML 3.9: Delegate to EntryExitHelper (Single Source of Truth)
        EntryExitHelper<InterpreterPolicy, IEventRaiser>::executeExitBlocks(exitLambdas, *eventRaiser_, stateId);

        // W3C SCXML: State exit succeeds even if some action blocks fail
        return true;
    }

    return true;
}

void StateMachine::generateDoneStateEvent(const std::string &stateId) {
    std::string doneEventName = "done.state." + stateId;
    LOG_INFO("Generating done.state event: {}", doneEventName);

    if (isRunning_ && eventRaiser_) {
        bool queued = eventRaiser_->raiseEvent(doneEventName, "", "", false);
        if (queued) {
            LOG_DEBUG("Queued done.state event: {}", doneEventName);
        } else {
            LOG_WARN("Failed to queue done.state event: {}", doneEventName);
        }
    } else {
        LOG_WARN("Cannot queue done.state event {} - state machine not running or no event raiser", doneEventName);
    }
}

void StateMachine::handleParallelStateCompletion(const std::string &stateId) {
    LOG_DEBUG("Handling parallel state completion for: {}", stateId);
    generateDoneStateEvent(stateId);
}

bool StateMachine::setupAndActivateParallelState(ConcurrentStateNode *parallelState, const std::string &stateId) {
    assert(parallelState && "Parallel state pointer must not be null");

    const auto &regions = parallelState->getRegions();
    if (regions.empty()) {
        LOG_ERROR("W3C SCXML violation: Parallel state '{}' has no regions", stateId);
        return false;
    }

    // W3C SCXML 6.4: Set invoke callback for proper invoke defer timing
    // Regions must be able to delegate invoke execution to StateMachine
    // Uses same defer pattern as AOT engine (ARCHITECTURE.md Zero Duplication)
    auto invokeCallback = [this](const std::string &stateId, const std::vector<std::shared_ptr<IInvokeNode>> &invokes) {
        if (invokes.empty()) {
            return;
        }
        LOG_DEBUG("StateMachine: Deferring {} invokes for state: {}", invokes.size(), stateId);

        // Thread-safe access to pendingInvokes_ - defer each invoke individually (matches AOT)
        std::lock_guard<std::recursive_mutex> lock(pendingInvokesMutex_);
        for (const auto &invoke : invokes) {
            std::string invokeId = invoke ? (invoke->getId().empty() ? "(auto-generated)" : invoke->getId()) : "null";
            PendingInvoke pending{invokeId, stateId, invoke};
            pendingInvokes_.push_back(pending);
        }
    };

    for (const auto &region : regions) {
        if (region) {
            region->setInvokeCallback(invokeCallback);
            LOG_DEBUG("Set invoke callback for region: {}", region->getId());
        }
    }

    // W3C SCXML B.1: Set condition evaluator for transition guard evaluation
    // Regions must be able to evaluate guard conditions via JavaScript engine
    auto conditionEvaluator = [this](const std::string &condition) -> bool { return evaluateCondition(condition); };

    for (const auto &region : regions) {
        if (region) {
            region->setConditionEvaluator(conditionEvaluator);
        }
    }

    // W3C SCXML 3.8: Set execution context for action execution
    // Regions need access to JavaScript engine for script evaluation
    if (executionContext_) {
        for (const auto &region : regions) {
            if (region) {
                region->setExecutionContext(executionContext_);
            }
        }
        LOG_DEBUG("Set execution context for parallel state regions: {}", stateId);
    } else {
        LOG_WARN("Execution context not available for parallel state: {}", stateId);
    }

    // W3C SCXML 3.4: Activate all regions simultaneously
    auto result = parallelState->enterParallelState();
    if (!result.isSuccess) {
        LOG_ERROR("Failed to activate parallel state regions for '{}': {}", stateId, result.errorMessage);
        return false;
    }

    LOG_DEBUG("Successfully setup and activated parallel state: {}", stateId);
    return true;
}

void StateMachine::setupParallelStateCallbacks() {
    if (!model_) {
        LOG_WARN("StateMachine: Cannot setup parallel state callbacks - no model available");
        return;
    }

    LOG_DEBUG("StateMachine: Setting up completion callbacks for parallel states");

    const auto &allStates = model_->getAllStates();
    int parallelStateCount = 0;
    int regionCallbackCount = 0;

    for (const auto &state : allStates) {
        if (state && state->getType() == Type::PARALLEL) {
            // Cast to ConcurrentStateNode to access the callback method
            auto parallelState = std::dynamic_pointer_cast<ConcurrentStateNode>(state);
            if (parallelState) {
                // Set up the completion callback using a lambda that captures this StateMachine
                parallelState->setCompletionCallback([this](const std::string &completedStateId) {
                    this->handleParallelStateCompletion(completedStateId);
                });

                // W3C SCXML 3.4 test 570: Set up done.state callback for each region
                // When a region reaches its final state, generate done.state.{regionId} event
                const auto &regions = parallelState->getRegions();
                for (const auto &region : regions) {
                    if (region) {
                        region->setDoneStateCallback(
                            [this](const std::string &regionId) { generateDoneStateEvent(regionId); });
                        regionCallbackCount++;
                    }
                }

                parallelStateCount++;
                LOG_DEBUG("Set up completion callback for parallel state: {}", state->getId());
            } else {
                LOG_WARN("Found parallel state that is not a ConcurrentStateNode: {}", state->getId());
            }
        }
    }

    LOG_INFO("Set up completion callbacks for {} parallel states ({} regions)", parallelStateCount,
             regionCallbackCount);
}

void StateMachine::initializeHistoryManager() {
    LOG_DEBUG("StateMachine: Initializing History Manager with SOLID architecture");

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

    // W3C SCXML 3.11: Create validator for history operations
    auto validator = std::make_unique<HistoryValidator>(stateProvider);

    // W3C SCXML 3.11: Create HistoryManager using shared HistoryHelper (Zero Duplication with AOT)
    historyManager_ = std::make_unique<HistoryManager>(stateProvider, std::move(validator));

    LOG_INFO("StateMachine: History Manager initialized - using shared HistoryHelper");
}

void StateMachine::initializeHistoryAutoRegistrar() {
    LOG_DEBUG("StateMachine: Initializing History Auto-Registrar with SOLID architecture");

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

    LOG_INFO("StateMachine: History Auto-Registrar initialized with SOLID dependencies");
}

bool StateMachine::registerHistoryState(const std::string &historyStateId, const std::string &parentStateId,
                                        HistoryType type, const std::string &defaultStateId) {
    if (!historyManager_) {
        LOG_ERROR("StateMachine: History Manager not initialized");
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
        LOG_ERROR("Cannot execute onentry actions: SCXML model is null");
        return;
    }

    // Find the state node
    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        LOG_ERROR("Cannot find state node for onentry execution: {}", stateId);
        return;
    }

    // W3C SCXML 3.8: Get entry action blocks from the state
    const auto &entryBlocks = stateNode->getEntryActionBlocks();
    if (entryBlocks.empty()) {
        LOG_DEBUG("No onentry actions to execute for state: {}", stateId);
        return;
    }

    LOG_DEBUG("W3C SCXML 3.8: Executing {} onentry action blocks for state: {}", entryBlocks.size(), stateId);

    // W3C SCXML compliance: Set immediate mode to false during executable content execution
    // This ensures events raised during execution are queued and processed after completion
    if (eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            eventRaiserImpl->setImmediateMode(false);
            LOG_DEBUG("SCXML compliance: Set immediate mode to false for onentry actions execution");
        }
    }

    // W3C SCXML 3.8: Build lambda blocks for EntryExitHelper
    // ARCHITECTURE.md Zero Duplication: Delegate to shared Helper (lines 311-373)
    std::vector<std::function<void()>> lambdaBlocks;
    for (const auto &actionBlock : entryBlocks) {
        lambdaBlocks.push_back([&, actionBlock]() {
            // Execute all actions in this block
            for (const auto &action : actionBlock) {
                if (!action) {
                    LOG_WARN("Null onentry action found in state: {}", stateId);
                    continue;
                }

                LOG_DEBUG("StateMachine: Executing onentry action: {} in state: {}", action->getActionType(), stateId);

                // Create execution context for the action
                if (actionExecutor_) {
                    auto sharedActionExecutor =
                        std::shared_ptr<IActionExecutor>(actionExecutor_.get(), [](IActionExecutor *) {});
                    ExecutionContextImpl context(sharedActionExecutor, sessionId_);

                    // Execute the action
                    if (!action->execute(context)) {
                        LOG_WARN("StateMachine: Failed to execute onentry action: {} - W3C SCXML 3.8: "
                                 "stopping remaining actions in THIS block only",
                                 action->getActionType());
                        // W3C SCXML 3.8: If error occurs, stop THIS block via lambda return
                        return;
                    } else {
                        LOG_DEBUG("StateMachine: Successfully executed onentry action: {} in state: {}",
                                  action->getActionType(), stateId);
                    }
                } else {
                    LOG_ERROR("Cannot execute onentry action: ActionExecutor is null");
                    return;
                }
            }
        });
    }

    // W3C SCXML 3.8: Delegate to EntryExitHelper (Single Source of Truth)
    // ARCHITECTURE.md Zero Duplication: Shared block orchestration between Interpreter and AOT
    EntryExitHelper<InterpreterPolicy, IEventRaiser>::executeEntryBlocks(lambdaBlocks, *eventRaiser_, stateId);

    // W3C SCXML compliance: Restore immediate mode (but DON'T process queued events yet)
    // Interactive mode: Keep immediate mode false to prevent auto-processing of queued events
    // Events must be processed AFTER the entire state tree entry completes, not during onentry
    // This ensures parent and child states are both active before processing raised events
    if (eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            if (autoProcessQueuedEvents_) {
                // Normal mode: Restore immediate mode for auto-processing
                eventRaiserImpl->setImmediateMode(true);
                LOG_DEBUG(
                    "SCXML compliance: Restored immediate mode (events will be processed after state entry completes)");
            } else {
                // Interactive mode: Keep immediate mode false to prevent auto-processing
                LOG_DEBUG("Interactive mode: Keeping immediate mode false (manual step-by-step execution)");
            }
        }
    }

    // W3C SCXML: Defer invoke execution until after state entry completes
    // This ensures proper timing with transition actions and pre-registration pattern
    const auto &invokes = stateNode->getInvoke();
    if (!invokes.empty()) {
        LOG_DEBUG("StateMachine: Deferring {} invokes for state: {}", invokes.size(), stateId);
        deferInvokeExecution(stateId, invokes);
    } else {
        LOG_DEBUG("StateMachine: No invokes to defer for state: {}", stateId);
    }
}

// EventDispatcher management
void StateMachine::setEventDispatcher(std::shared_ptr<IEventDispatcher> eventDispatcher) {
    eventDispatcher_ = eventDispatcher;

    // Pass EventDispatcher to ActionExecutor for send actions
    if (actionExecutor_) {
        auto actionExecutorImpl = std::dynamic_pointer_cast<ActionExecutorImpl>(actionExecutor_);
        if (actionExecutorImpl) {
            actionExecutorImpl->setEventDispatcher(eventDispatcher);
            LOG_DEBUG("StateMachine: EventDispatcher passed to ActionExecutor for session: {}", sessionId_);
        }
    }

    // Pass EventDispatcher to InvokeExecutor for child session management
    if (invokeExecutor_) {
        invokeExecutor_->setEventDispatcher(eventDispatcher);
        LOG_DEBUG("StateMachine: EventDispatcher passed to InvokeExecutor for session: {}", sessionId_);

        // W3C SCXML Test 192: Set parent StateMachine for completion callback state checking
        // Only set if this StateMachine is managed by shared_ptr (not during construction)
        // This will be set later in executeInvoke() when actually needed
    }
}

// W3C SCXML 6.5: Completion callback management
void StateMachine::setCompletionCallback(CompletionCallback callback) {
    completionCallback_ = callback;
    LOG_DEBUG("StateMachine: Completion callback {} for session: {}", callback ? "set" : "cleared", sessionId_);
}

// EventRaiser management
void StateMachine::setEventRaiser(std::shared_ptr<IEventRaiser> eventRaiser) {
    LOG_DEBUG("StateMachine: setEventRaiser called for session: {}", sessionId_);
    eventRaiser_ = eventRaiser;

    // SCXML W3C compliance: Set EventRaiser callback to StateMachine's processEvent
    // This allows events generated by raise actions to actually trigger state transitions
    if (eventRaiser_) {
        auto eventRaiserImpl = std::dynamic_pointer_cast<EventRaiserImpl>(eventRaiser_);
        if (eventRaiserImpl) {
            LOG_DEBUG("StateMachine: EventRaiser callback setup - EventRaiser instance: {}, StateMachine instance: {}",
                      (void *)eventRaiserImpl.get(), (void *)this);
            // Set StateMachine's processEvent method as EventRaiser callback
            eventRaiserImpl->setEventCallback(
                [this](const std::string &eventName, const std::string &eventData) -> bool {
                    if (isRunning_) {
                        LOG_DEBUG("EventRaiser callback: StateMachine::processEvent called - event: '{}', data: '{}', "
                                  "StateMachine instance: {}",
                                  eventName, eventData, (void *)this);
                        // Use 2-parameter version (no originSessionId from old callback)
                        auto result = processEvent(eventName, eventData);
                        LOG_DEBUG("EventRaiser callback: processEvent result - success: {}, state transition: {} -> {}",
                                  result.success, result.fromState, result.toState);
                        return result.success;
                    } else {
                        LOG_WARN("EventRaiser callback: StateMachine not running - ignoring event '{}'", eventName);
                        return false;
                    }
                });
            LOG_DEBUG("StateMachine: EventRaiser callback set to processEvent - session: {}, EventRaiser instance: {}",
                      sessionId_, (void *)eventRaiserImpl.get());
        }
    }

    // Register EventRaiser with JSEngine for #_invokeid target support
    // Use EventRaiserService for centralized registration
    if (eventRaiser_) {
        if (EventRaiserService::getInstance().registerEventRaiser(sessionId_, eventRaiser_)) {
            LOG_DEBUG("StateMachine: EventRaiser registered via Service for session: {}", sessionId_);
        } else {
            LOG_DEBUG("StateMachine: EventRaiser registration deferred or already exists for session: {}", sessionId_);
        }
    }

    // Pass EventRaiser to ActionExecutor if it exists (during build phase)
    if (actionExecutor_) {
        actionExecutor_->setEventRaiser(eventRaiser);
        LOG_DEBUG("StateMachine: EventRaiser passed to ActionExecutor for session: {}", sessionId_);
    }
    // Note: If ActionExecutor doesn't exist yet, it will be set during loadSCXMLFromString
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
    JSEngine::instance().registerSessionFilePath(sessionId_, filePath);
    LOG_DEBUG("StateMachine: Registered session file path: {} for session: {}", filePath, sessionId_);
}

void StateMachine::deferInvokeExecution(const std::string &stateId,
                                        const std::vector<std::shared_ptr<IInvokeNode>> &invokes) {
    LOG_DEBUG("StateMachine: Deferring {} invokes for state: {} in session: {}", invokes.size(), stateId, sessionId_);

    // Thread-safe access to pendingInvokes_
    std::lock_guard<std::recursive_mutex> lock(pendingInvokesMutex_);
    size_t beforeSize = pendingInvokes_.size();

    // W3C SCXML 6.4: Defer each invoke individually (matches AOT pattern)
    for (size_t i = 0; i < invokes.size(); ++i) {
        const auto &invoke = invokes[i];
        std::string invokeId = invoke ? (invoke->getId().empty() ? "(auto-generated)" : invoke->getId()) : "null";
        std::string invokeType = invoke ? invoke->getType() : "null";
        LOG_DEBUG("StateMachine: DETAILED DEBUG - Deferring invoke[{}]: id='{}', type='{}'", i, invokeId, invokeType);

        PendingInvoke pending{invokeId, stateId, invoke};
        pendingInvokes_.push_back(pending);
    }

    LOG_DEBUG("StateMachine: DETAILED DEBUG - Pending invokes count: {} -> {}", beforeSize, pendingInvokes_.size());
}

void StateMachine::executePendingInvokes() {
    // W3C SCXML Test 192: Set parent StateMachine before executing invokes (requires shared_ptr context)
    // This is safe here because executePendingInvokes() is only called when StateMachine is already in shared_ptr
    // context
    if (invokeExecutor_) {
        try {
            invokeExecutor_->setParentStateMachine(shared_from_this());
            LOG_DEBUG(
                "StateMachine: Parent StateMachine set in InvokeExecutor before executing invokes for session: {}",
                sessionId_);
        } catch (const std::bad_weak_ptr &e) {
            LOG_WARN("StateMachine: Cannot set parent StateMachine - not managed by shared_ptr yet for session: {}",
                     sessionId_);
        }
    }

    // W3C SCXML 6.4: Execute pending invokes using InvokeHelper (ARCHITECTURE.md Zero Duplication)
    // Uses same pattern as AOT engine - copy-and-clear prevents iterator invalidation
    std::lock_guard<std::recursive_mutex> lock(pendingInvokesMutex_);

    LOG_DEBUG("StateMachine: Found {} pending invokes to execute for session: {}", pendingInvokes_.size(), sessionId_);

    InvokeHelper::executePendingInvokes(pendingInvokes_, [this](const PendingInvoke &pending) {
        // W3C SCXML Test 252: Only execute if state is still active (entered-and-not-exited)
        if (!isStateActive(pending.state)) {
            LOG_DEBUG("StateMachine: Skipping invoke '{}' for inactive state: {}", pending.invokeId, pending.state);
            return;
        }

        LOG_DEBUG("StateMachine: Executing invoke '{}' for state '{}'", pending.invokeId, pending.state);

        if (invokeExecutor_) {
            std::string invokeid = invokeExecutor_->executeInvoke(pending.invoke, sessionId_);
            if (invokeid.empty()) {
                LOG_ERROR("StateMachine: Failed to execute invoke '{}' for state: {}", pending.invokeId, pending.state);
                // W3C SCXML: Continue execution even if invokes fail
            }
        } else {
            LOG_ERROR("StateMachine: Cannot execute invoke - InvokeExecutor is null");
        }
    });
}

// W3C SCXML 3.7 & 5.5: Handle compound state completion when final child is entered
void StateMachine::handleCompoundStateFinalChild(const std::string &finalStateId) {
    if (!model_) {
        return;
    }

    auto finalState = model_->findStateById(finalStateId);
    if (!finalState || !finalState->isFinalState()) {
        return;
    }

    // Get parent state
    auto parent = finalState->getParent();
    if (!parent) {
        return;  // Top-level final state, no done.state event for compound
    }

    // Only generate done.state for compound (non-parallel) parent states
    if (parent->getType() == Type::PARALLEL) {
        return;  // Parallel states handled separately
    }

    // W3C SCXML 3.7: Generate done.state.{parentId} event
    std::string parentId = parent->getId();
    std::string doneEventName = "done.state." + parentId;

    LOG_INFO("W3C SCXML 3.7: Compound state '{}' completed, generating done.state event: {}", parentId, doneEventName);

    // W3C SCXML 5.5 & 5.7: Evaluate donedata and construct event data
    // If evaluation fails (error.execution raised), do not generate done.state event
    std::string eventData;
    if (!evaluateDoneData(finalStateId, eventData)) {
        LOG_DEBUG("W3C SCXML 5.7: Donedata evaluation failed, skipping done.state event generation");
        return;
    }

    // W3C SCXML: Queue the done.state event (not immediate processing)
    // This allows error.execution events from donedata evaluation to be processed first
    if (isRunning_ && eventRaiser_) {
        eventRaiser_->raiseEvent(doneEventName, eventData);
        LOG_DEBUG("W3C SCXML: Queued done.state event: {}", doneEventName);
    }
}

// W3C SCXML 5.5: Helper functions moved to DoneDataHelper (Zero Duplication)
// - escapeJsonString() -> DoneDataHelper::escapeJsonString()
// - convertScriptValueToJson() -> DoneDataHelper::convertScriptValueToJson()

/**
 * W3C SCXML 5.5 & 5.7: Evaluate donedata and return JSON event data
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
bool StateMachine::evaluateDoneData(const std::string &finalStateId, std::string &outEventData) {
    // W3C SCXML 5.5: Initialize output
    outEventData = "";

    if (!model_) {
        return true;  // No donedata to evaluate
    }

    auto finalState = model_->findStateById(finalStateId);
    if (!finalState) {
        return true;  // No donedata to evaluate
    }

    const auto &doneData = finalState->getDoneData();

    // W3C SCXML 5.5: Evaluate content using shared DoneDataHelper (Zero Duplication)
    if (!doneData.getContent().empty()) {
        LOG_DEBUG("W3C SCXML 5.5: Evaluating donedata content: '{}'", doneData.getContent());
        return DoneDataHelper::evaluateContent(
            SCE::JSEngine::instance(), sessionId_, doneData.getContent(), outEventData, [this](const std::string &msg) {
                LOG_ERROR("W3C SCXML 5.5: Failed to evaluate donedata content: {}", msg);
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", msg);
                }
            });
    }

    // W3C SCXML 5.5: Evaluate params using shared DoneDataHelper (Zero Duplication)
    const auto &params = doneData.getParams();
    if (!params.empty()) {
        LOG_DEBUG("W3C SCXML 5.5: Evaluating {} donedata params", params.size());
        return DoneDataHelper::evaluateParams(SCE::JSEngine::instance(), sessionId_, params, outEventData,
                                              [this](const std::string &msg) {
                                                  LOG_ERROR("W3C SCXML 5.7: {}", msg);
                                                  if (eventRaiser_) {
                                                      eventRaiser_->raiseEvent("error.execution", msg);
                                                  }
                                              });
    }

    // No donedata
    return true;
}

// W3C SCXML: Get proper ancestors of a state (all ancestors excluding the state itself)
std::vector<std::string> StateMachine::getProperAncestors(const std::string &stateId) const {
    std::vector<std::string> ancestors;

    if (!model_) {
        return ancestors;
    }

    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        return ancestors;
    }

    IStateNode *current = stateNode->getParent();
    while (current != nullptr) {
        ancestors.push_back(current->getId());
        current = current->getParent();
    }

    return ancestors;
}

// W3C SCXML: Check if stateId is a descendant of ancestorId
bool StateMachine::isDescendant(const std::string &stateId, const std::string &ancestorId) const {
    if (!model_ || stateId.empty() || ancestorId.empty()) {
        return false;
    }

    if (stateId == ancestorId) {
        return false;  // A state is not its own descendant
    }

    auto stateNode = model_->findStateById(stateId);
    if (!stateNode) {
        return false;
    }

    IStateNode *current = stateNode->getParent();
    while (current != nullptr) {
        if (current->getId() == ancestorId) {
            return true;
        }
        current = current->getParent();
    }

    return false;
}

// W3C SCXML: Find Lowest Common Ancestor of source and target states
int StateMachine::getStateDocumentPosition(const std::string &stateId) const {
    // W3C SCXML 3.13: Get document order position for state
    // Uses depth-first pre-order traversal to assign positions
    if (!model_) {
        return -1;
    }

    // Helper to recursively assign positions
    int position = 0;
    std::function<int(IStateNode *, const std::string &)> findPosition = [&](IStateNode *node,
                                                                             const std::string &targetId) -> int {
        if (!node) {
            return -1;
        }

        if (node->getId() == targetId) {
            return position;
        }

        position++;

        // Depth-first pre-order: visit children
        const auto &children = node->getChildren();
        for (const auto &child : children) {
            int result = findPosition(child.get(), targetId);
            if (result >= 0) {
                return result;
            }
        }

        return -1;
    };

    // Start from root state
    auto rootState = model_->getRootState();
    if (!rootState) {
        return -1;
    }

    return findPosition(rootState.get(), stateId);
}

std::string StateMachine::findLCA(const std::string &sourceStateId, const std::string &targetStateId) const {
    if (!model_) {
        return "";
    }

    // ARCHITECTURE.md: Zero Duplication - delegate to HierarchicalStateHelper
    // W3C SCXML 3.12: Find Least Common Ancestor for hierarchical transitions
    auto getParent = [this](const std::string &stateId) -> std::optional<std::string> {
        auto node = model_->findStateById(stateId);
        if (!node || !node->getParent()) {
            return std::nullopt;
        }
        return node->getParent()->getId();
    };

    // Use shared Helper implementation (Single Source of Truth)
    return SCE::Common::HierarchicalStateHelperString::findLCA(sourceStateId, targetStateId, getParent);
}

// Helper: Build exit set for descendants of an ancestor state
// Used by both internal transitions and computeExitSet to avoid code duplication
std::vector<std::string> StateMachine::buildExitSetForDescendants(const std::string &ancestorState,
                                                                  bool excludeParallelChildren) const {
    std::vector<std::string> exitSet;

    if (!hierarchyManager_ || !model_) {
        return exitSet;
    }

    // Get all active states
    auto activeStates = hierarchyManager_->getActiveStates();

    for (const auto &activeState : activeStates) {
        // Skip if this is the ancestor itself
        if (activeState == ancestorState) {
            continue;
        }

        // Defensive: Get state node and skip if not found
        auto activeNode = model_->findStateById(activeState);
        if (!activeNode) {
            LOG_WARN("buildExitSetForDescendants: Active state '{}' not found in model - skipping", activeState);
            continue;
        }

        // Check if parent is a parallel state - skip if requested
        if (excludeParallelChildren) {
            auto parent = activeNode->getParent();
            if (parent && parent->getType() == Type::PARALLEL) {
                // Skip - parallel state's children are handled by exitParallelState
                continue;
            }
        }

        // Check if activeState is a descendant of ancestorState
        if (ancestorState.empty()) {
            // If ancestor is root (empty), all active states are descendants
            exitSet.push_back(activeState);
        } else {
            // Walk up the ancestor chain to check if we reach ancestorState
            IStateNode *current = activeNode->getParent();
            while (current) {
                if (current->getId() == ancestorState) {
                    // Found ancestor - activeState is a descendant
                    exitSet.push_back(activeState);
                    break;
                }
                current = current->getParent();
            }
        }
    }

    // Sort by depth (deepest first) for correct exit order
    std::sort(exitSet.begin(), exitSet.end(), [this](const std::string &a, const std::string &b) {
        int depthA = 0, depthB = 0;
        auto nodeA = model_->findStateById(a);
        auto nodeB = model_->findStateById(b);

        if (nodeA) {
            IStateNode *current = nodeA->getParent();
            while (current) {
                depthA++;
                current = current->getParent();
            }
        }
        if (nodeB) {
            IStateNode *current = nodeB->getParent();
            while (current) {
                depthB++;
                current = current->getParent();
            }
        }

        return depthA > depthB;  // Deeper states first
    });

    return exitSet;
}

// W3C SCXML: Compute exit set for transition from source to target
StateMachine::ExitSetResult StateMachine::computeExitSet(const std::string &sourceStateId,
                                                         const std::string &targetStateId) const {
    ExitSetResult result;
    result.states.reserve(8);  // Performance: Reserve typical exit set size to avoid reallocation

    if (!model_ || sourceStateId.empty()) {
        return result;
    }

    // If target is empty (targetless transition), exit source only
    if (targetStateId.empty()) {
        result.states.push_back(sourceStateId);
        return result;
    }

    // W3C SCXML 3.13: Find LCA (Lowest Common Ancestor) once
    result.lca = findLCA(sourceStateId, targetStateId);

    // W3C SCXML 3.13: Exit set = "all active states that are proper descendants of LCCA"
    // This must include ALL active descendants, not just the source->LCA chain (test 505)
    // Use helper method to build exit set (reduces code duplication)
    result.states = buildExitSetForDescendants(result.lca, true);

    // W3C SCXML 3.10 (test 579): Ancestor transition (target == LCA)
    // When transitioning to an ancestor state, the target must also be exited and re-entered
    // This ensures onexit/onentry are executed, allowing data changes (e.g., Var1++)
    if (targetStateId == result.lca && hierarchyManager_ && hierarchyManager_->isStateActive(targetStateId)) {
        result.states.push_back(targetStateId);
        LOG_DEBUG("W3C SCXML: Ancestor transition detected, including target '{}' in exit set", targetStateId);
    }

    // W3C SCXML 3.10 (test 580): History state transition
    // When transitioning to a history state whose parent is active, exit and re-enter the parent
    // This ensures onexit/onentry actions execute (e.g., Var1++ in onexit)
    auto targetNode = model_->findStateById(targetStateId);
    if (targetNode && targetNode->getType() == Type::HISTORY) {
        auto parentNode = targetNode->getParent();
        if (parentNode && hierarchyManager_ && hierarchyManager_->isStateActive(parentNode->getId())) {
            std::string parentId = parentNode->getId();
            // Check if parent is not already in exit set
            if (std::find(result.states.begin(), result.states.end(), parentId) == result.states.end()) {
                result.states.push_back(parentId);
                LOG_DEBUG(
                    "W3C SCXML 3.10: History state transition, including active parent '{}' in exit set (test 580)",
                    parentId);
            }
        }
    }

    // Note: buildExitSetForDescendants already:
    // - Excludes parallel children (test 404, 504)
    // - Sorts by depth (deepest first)
    // - Handles null checks defensively

    LOG_DEBUG("W3C SCXML: computeExitSet({} -> {}) = {} states, LCA = '{}'", sourceStateId, targetStateId,
              result.states.size(), result.lca);

    return result;
}

}  // namespace SCE
