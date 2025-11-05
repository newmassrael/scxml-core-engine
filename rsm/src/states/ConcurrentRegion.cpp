#include "states/ConcurrentRegion.h"
#include "actions/AssignAction.h"
#include "actions/ScriptAction.h"
#include "common/Logger.h"
#include "events/EventDescriptor.h"
#include "model/IStateNode.h"
#include "model/ITransitionNode.h"
#include "runtime/IActionExecutor.h"
#include "runtime/IExecutionContext.h"
#include "states/StateExitExecutor.h"
#include <algorithm>
#include <format>

namespace RSM {

ConcurrentRegion::ConcurrentRegion(const std::string &id, std::shared_ptr<IStateNode> rootState,
                                   std::shared_ptr<IExecutionContext> executionContext)
    : id_(id), status_(ConcurrentRegionStatus::INACTIVE), rootState_(rootState), executionContext_(executionContext),
      isInFinalState_(false), exitHandler_(std::make_shared<StateExitExecutor>()) {
    // SCXML W3C specification section 3.4: regions must have valid identifiers
    assert(!id_.empty() && "SCXML violation: concurrent region must have non-empty ID");

    LOG_DEBUG("Creating region: {}", id_);

    if (rootState_) {
        LOG_DEBUG("Root state provided: {}", rootState_->getId());
    } else {
        LOG_DEBUG("No root state provided (will be set later)");
    }
}

ConcurrentRegion::~ConcurrentRegion() {
    LOG_DEBUG("Destroying region: {}", id_);

    // Clean deactivation if still active
    if (status_ == ConcurrentRegionStatus::ACTIVE) {
        LOG_DEBUG("Deactivating region during destruction");
        deactivate(nullptr);
    }
}

const std::string &ConcurrentRegion::getId() const {
    return id_;
}

ConcurrentOperationResult ConcurrentRegion::activate() {
    if (status_ == ConcurrentRegionStatus::ACTIVE) {
        LOG_DEBUG("Region {} already active", id_);
        return ConcurrentOperationResult::success(id_);
    }

    // SCXML W3C specification section 3.4: regions must have root states
    if (!rootState_) {
        std::string error = std::format("SCXML violation: cannot activate region '{}' without root state. SCXML "
                                        "specification requires regions to have states.",
                                        id_);
        LOG_ERROR("Activate error: {}", error);
        setErrorState(error);
        return ConcurrentOperationResult::failure(id_, error);
    }

    // Validate root state before activation
    if (!validateRootState()) {
        std::string error = std::format("Root state validation failed for region: {}", id_);
        LOG_ERROR("Root state validation failed: {}", error);
        setErrorState(error);
        return ConcurrentOperationResult::failure(id_, error);
    }

    LOG_DEBUG("Activating region: {}", id_);

    // Mark region as active before entering initial state to enable final state detection
    status_ = ConcurrentRegionStatus::ACTIVE;

    // Enter initial state according to SCXML semantics
    auto result = enterInitialState();
    if (!result.isSuccess) {
        LOG_ERROR("Failed to enter initial state: {}", result.errorMessage);
        status_ = ConcurrentRegionStatus::ERROR;  // Rollback on failure
        setErrorState(result.errorMessage);
        return result;
    }
    updateCurrentState();

    LOG_DEBUG("Successfully activated region: {}", id_);
    return ConcurrentOperationResult::success(id_);
}

ConcurrentOperationResult ConcurrentRegion::deactivate(std::shared_ptr<IExecutionContext> executionContext) {
    if (status_ == ConcurrentRegionStatus::INACTIVE) {
        LOG_DEBUG("Region {} already inactive", id_);
        return ConcurrentOperationResult::success(id_);
    }

    // W3C SCXML 3.13: If activeStates_ is already empty, region was exited via exit set
    // Skip exitAllStates to avoid duplicate exit action execution (test 504)
    if (activeStates_.empty()) {
        LOG_DEBUG("Region {} activeStates already empty, skipping exitAllStates", id_);
        status_ = ConcurrentRegionStatus::INACTIVE;
        currentState_.clear();
        isInFinalState_ = false;
        LOG_DEBUG("Successfully deactivated region: {}", id_);
        return ConcurrentOperationResult::success(id_);
    }

    LOG_DEBUG("Deactivating region: {}", id_);

    // Exit all active states
    auto result = exitAllStates(executionContext);
    if (!result.isSuccess) {
        LOG_WARN("Warning during state exit: {}", result.errorMessage);
        // Continue with deactivation even if exit has issues
    }

    status_ = ConcurrentRegionStatus::INACTIVE;
    currentState_.clear();
    activeStates_.clear();
    isInFinalState_ = false;

    LOG_DEBUG("Successfully deactivated region: {}", id_);
    return ConcurrentOperationResult::success(id_);
}

bool ConcurrentRegion::isActive() const {
    return status_ == ConcurrentRegionStatus::ACTIVE;
}

bool ConcurrentRegion::isInFinalState() const {
    return isInFinalState_ && status_ == ConcurrentRegionStatus::FINAL;
}

ConcurrentRegionStatus ConcurrentRegion::getStatus() const {
    return status_;
}

ConcurrentRegionInfo ConcurrentRegion::getInfo() const {
    ConcurrentRegionInfo info;
    info.id = id_;
    info.status = status_;
    info.currentState = currentState_;
    info.isInFinalState = isInFinalState_;
    info.activeStates = activeStates_;
    return info;
}

ConcurrentOperationResult ConcurrentRegion::processEvent(const EventDescriptor &event) {
    if (status_ != ConcurrentRegionStatus::ACTIVE) {
        std::string error = std::format("Cannot process event in inactive region: {}", id_);
        LOG_WARN("processEvent - {}", error);
        return ConcurrentOperationResult::failure(id_, error);
    }

    if (!rootState_) {
        std::string error = std::format("SCXML violation: cannot process event without root state in region: {}", id_);
        LOG_ERROR("Error: {}", error);
        setErrorState(error);
        return ConcurrentOperationResult::failure(id_, error);
    }

    LOG_DEBUG("Processing event '{}' in region: {}", event.eventName, id_);

    // W3C SCXML Appendix D.2: Collect enabled transitions instead of executing immediately
    // This allows StateMachine to apply conflict resolution across all regions
    ConcurrentOperationResult result = ConcurrentOperationResult::success(id_);

    // W3C SCXML 3.13: Hierarchical event bubbling - check from current state up through parent hierarchy
    if (!currentState_.empty()) {
        // Find current state node
        std::shared_ptr<IStateNode> stateNode = nullptr;
        if (currentState_ == rootState_->getId()) {
            stateNode = rootState_;
        } else {
            // Search in child states
            const auto &children = rootState_->getChildren();
            for (const auto &child : children) {
                if (child->getId() == currentState_) {
                    stateNode = child;
                    break;
                }
            }
        }

        if (stateNode) {
            // W3C SCXML 3.12: Hierarchical event bubbling (innermost to outermost)
            // Check from current active state up through parent hierarchy
            IStateNode *checkStatePtr = stateNode.get();  // Use raw pointer for hierarchy traversal
            int transitionIndex = 0;

            while (checkStatePtr) {
                const auto &transitions = checkStatePtr->getTransitions();

                // W3C SCXML 3.13: Find first enabled transition in document order
                for (const auto &transition : transitions) {
                    // W3C SCXML 3.13: Wildcard event matching - "*" matches any event
                    std::string transitionEvent = transition->getEvent();
                    bool eventMatches = (transitionEvent == event.eventName) || (transitionEvent == "*");

                    if (!eventMatches) {
                        transitionIndex++;
                        continue;
                    }

                    // W3C SCXML: Evaluate guard condition before enabling transition
                    std::string guard = transition->getGuard();
                    bool conditionResult = true;  // Default to true if no guard condition

                    if (!guard.empty()) {
                        if (conditionEvaluator_) {
                            conditionResult = conditionEvaluator_(guard);
                            LOG_DEBUG(
                                "ConcurrentRegion: Evaluated guard condition '{}' for transition: {} -> result: {}",
                                guard, event.eventName, conditionResult ? "true" : "false");
                        } else {
                            LOG_WARN("ConcurrentRegion: Guard condition '{}' present but no evaluator set, defaulting "
                                     "to true",
                                     guard);
                        }
                    }

                    // Skip this transition if condition is false
                    if (!conditionResult) {
                        LOG_DEBUG("ConcurrentRegion: Skipping transition due to false guard condition: {}", guard);
                        transitionIndex++;
                        continue;
                    }

                    // Found enabled transition - collect it instead of executing
                    const auto &targets = transition->getTargets();
                    std::string targetState = targets.empty() ? checkStatePtr->getId() : targets[0];
                    bool isInternal = transition->isInternal();
                    bool hasActions = !transition->getActionNodes().empty();

                    LOG_DEBUG("ConcurrentRegion: Found enabled transition in state {}: {} -> {} (event='{}', "
                              "internal={}, hasActions={})",
                              checkStatePtr->getId(), checkStatePtr->getId(), targetState, transitionEvent, isInternal,
                              hasActions);

                    // W3C SCXML 3.13: Check if transition exits the parallel state
                    // External: transition target is OUTSIDE the parallel state (e.g., p0s3 -> s1)
                    // Internal: transition target is INSIDE the parallel state (e.g., p0s2 -> p0s1)
                    IStateNode *parallelStatePtr = rootState_->getParent();
                    bool isExternalTransition = true;  // Default to external for safety

                    if (parallelStatePtr) {
                        // W3C SCXML: Check if target is a descendant of ANY region in the parallel state
                        // Parallel state children = regions (p0s1, p0s2, p0s3, p0s4)
                        const auto &parallelChildren = parallelStatePtr->getChildren();
                        for (const auto &regionRoot : parallelChildren) {
                            if (regionRoot && isDescendantOf(regionRoot, targetState)) {
                                // Target is within this parallel state's regions -> internal transition
                                isExternalTransition = false;
                                LOG_DEBUG("ConcurrentRegion: Target '{}' is within parallel state '{}' -> internal "
                                          "transition",
                                          targetState, parallelStatePtr->getId());
                                break;
                            }
                        }

                        if (isExternalTransition) {
                            LOG_DEBUG(
                                "ConcurrentRegion: Target '{}' is outside parallel state '{}' -> external transition",
                                targetState, parallelStatePtr->getId());
                        }
                    }

                    if (isExternalTransition) {
                        LOG_DEBUG("ConcurrentRegion: Transition target '{}' is outside region '{}' - marking as "
                                  "external for conflict resolution",
                                  targetState, id_);
                    }

                    // Create transition descriptor for conflict resolution
                    TransitionDescriptorString descriptor;
                    descriptor.source = checkStatePtr->getId();
                    descriptor.target = targetState;
                    descriptor.event = event.eventName;
                    descriptor.transitionIndex = transitionIndex;
                    descriptor.hasActions = hasActions;
                    descriptor.isInternal = isInternal;
                    descriptor.isExternal = isExternalTransition;

                    // Compute exit set (states to be exited)
                    // W3C SCXML: Exit set is all states from source up to (but not including) LCA with target
                    descriptor.exitSet = computeExitSet(checkStatePtr->getId(), targetState);

                    LOG_DEBUG("ConcurrentRegion: Transition descriptor: {} -> {} (exitSet size: {}, transitionIndex: "
                              "{}, external: {})",
                              descriptor.source, descriptor.target, descriptor.exitSet.size(),
                              descriptor.transitionIndex, descriptor.isExternal);

                    result.enabledTransitions.push_back(descriptor);
                    return result;  // W3C SCXML 3.13: First enabled transition wins in hierarchy
                }

                // W3C SCXML 3.12: Move to parent state for hierarchical event bubbling
                // But STOP at region boundary - don't bubble beyond the region's root state
                // This prevents regions from collecting transitions from the parallel state's ancestors
                if (checkStatePtr == rootState_.get()) {
                    LOG_DEBUG("ConcurrentRegion: Reached region boundary at {}, stopping hierarchy bubbling",
                              checkStatePtr->getId());
                    break;  // Reached region boundary, stop bubbling
                }

                checkStatePtr = checkStatePtr->getParent();
                if (!checkStatePtr) {
                    break;  // Reached model root, no more parents
                }
            }
        }
    }

    // No enabled transitions found - return success with empty enabledTransitions
    LOG_DEBUG("ConcurrentRegion: No enabled transitions found in region: {}", id_);
    return result;
}

std::shared_ptr<IStateNode> ConcurrentRegion::getRootState() const {
    return rootState_;
}

void ConcurrentRegion::setRootState(std::shared_ptr<IStateNode> rootState) {
    // SCXML W3C specification section 3.4: regions must have states
    assert(rootState && "SCXML violation: concurrent region cannot have null root state");

    if (status_ == ConcurrentRegionStatus::ACTIVE) {
        LOG_WARN(
            "ConcurrentRegion::setRootState - Setting root state on active region {} (consider deactivating first)",
            id_);
    }

    LOG_DEBUG("Setting root state for region {} to: {}", id_, (rootState ? rootState->getId() : "null"));

    rootState_ = rootState;

    // Reset state information when root state changes
    currentState_.clear();
    activeStates_.clear();
    isInFinalState_ = false;

    // Clear any previous error state
    if (status_ == ConcurrentRegionStatus::ERROR) {
        clearErrorState();
    }
}

std::vector<std::string> ConcurrentRegion::getActiveStates() const {
    return activeStates_;
}

ConcurrentOperationResult ConcurrentRegion::reset() {
    LOG_DEBUG("Resetting region: {}", id_);

    // Deactivate if currently active
    if (status_ == ConcurrentRegionStatus::ACTIVE) {
        auto result = deactivate();
        if (!result.isSuccess) {
            LOG_ERROR("Failed to deactivate during reset: {}", result.errorMessage);
            return result;
        }
    }

    // Reset all state
    status_ = ConcurrentRegionStatus::INACTIVE;
    currentState_.clear();
    activeStates_.clear();
    isInFinalState_ = false;
    errorMessage_.clear();

    LOG_DEBUG("Successfully reset region: {}", id_);
    return ConcurrentOperationResult::success(id_);
}

std::vector<std::string> ConcurrentRegion::validate() const {
    std::vector<std::string> errors;

    // SCXML W3C specification section 3.4: regions must have valid IDs
    if (id_.empty()) {
        errors.push_back("SCXML violation: Region has empty ID. SCXML specification requires non-empty identifiers.");
    }

    // SCXML W3C specification section 3.4: regions must have root states
    if (!rootState_) {
        errors.push_back(std::format(
            "SCXML violation: Region '{}' has no root state. SCXML specification requires regions to contain states.",
            id_));
    } else {
        // Validate root state
        if (!validateRootState()) {
            errors.push_back(std::format("Root state validation failed for region: {}", id_));
        }
    }

    // Validate status consistency
    if (status_ == ConcurrentRegionStatus::FINAL && !isInFinalState_) {
        errors.push_back(std::format("Inconsistent final state tracking in region: {}", id_));
    }

    if (status_ == ConcurrentRegionStatus::ACTIVE && currentState_.empty()) {
        errors.push_back(std::format("Active region {} has no current state", id_));
    }

    return errors;
}

const std::string &ConcurrentRegion::getCurrentState() const {
    return currentState_;
}

void ConcurrentRegion::setCurrentState(const std::string &stateId) {
    // W3C SCXML 3.3: Validate that state belongs to this region
    // This is called during deep initial target synchronization
    if (!stateId.empty() && rootState_) {
        // Validate the state is within this region's scope
        bool isValidState = isDescendantOf(rootState_, stateId);
        if (!isValidState) {
            LOG_WARN("ConcurrentRegion: Attempting to set currentState to '{}' which is not within region '{}' scope",
                     stateId, id_);
            // Continue anyway - StateHierarchyManager knows best in deep target scenarios
        }
    }

    LOG_DEBUG("ConcurrentRegion: Setting currentState for region {} to: {}", id_, stateId);
    currentState_ = stateId;

    // W3C SCXML 3.4: Update isInFinalState_ flag when currentState changes
    // This is crucial for parallel state completion detection
    isInFinalState_ = determineIfInFinalState();

    // Update region status to FINAL if we entered a final state
    if (isInFinalState_ && status_ != ConcurrentRegionStatus::FINAL) {
        status_ = ConcurrentRegionStatus::FINAL;
        LOG_DEBUG("ConcurrentRegion: Region {} entered final state '{}', updating status to FINAL", id_, stateId);

        // W3C SCXML 3.13: Generate done.state.{regionId} event when compound state enters final
        // test570: When p0s1 (compound region) reaches p0s1final, generate done.state.p0s1
        if (doneStateCallback_) {
            LOG_DEBUG("ConcurrentRegion: Calling doneStateCallback for region {}", id_);
            doneStateCallback_(id_);
        }
    }
}

bool ConcurrentRegion::isInErrorState() const {
    return status_ == ConcurrentRegionStatus::ERROR;
}

void ConcurrentRegion::setErrorState(const std::string &errorMessage) {
    LOG_ERROR("Region {} entering error state: {}", id_, errorMessage);
    status_ = ConcurrentRegionStatus::ERROR;
    errorMessage_ = errorMessage;

    // Clear other state information when in error
    currentState_.clear();
    activeStates_.clear();
    isInFinalState_ = false;
}

void ConcurrentRegion::clearErrorState() {
    if (status_ == ConcurrentRegionStatus::ERROR) {
        LOG_DEBUG("Clearing error state for region: {}", id_);
        status_ = ConcurrentRegionStatus::INACTIVE;
        errorMessage_.clear();
    }
}

void ConcurrentRegion::setExecutionContext(std::shared_ptr<IExecutionContext> executionContext) {
    LOG_DEBUG("Setting ExecutionContext for region: {} - new context is {}", id_, executionContext ? "valid" : "null");
    executionContext_ = executionContext;
    LOG_DEBUG("ExecutionContext set successfully for region: {} - stored context is {}", id_,
              executionContext_ ? "valid" : "null");
}

void ConcurrentRegion::setInvokeCallback(
    std::function<void(const std::string &, const std::vector<std::shared_ptr<IInvokeNode>> &)> callback) {
    invokeCallback_ = callback;
    LOG_DEBUG("ConcurrentRegion: Invoke callback set for region: {} (W3C SCXML 6.4 compliance)", id_);
}

void ConcurrentRegion::setConditionEvaluator(std::function<bool(const std::string &)> evaluator) {
    conditionEvaluator_ = evaluator;
    LOG_DEBUG(
        "ConcurrentRegion: Condition evaluator callback set for region: {} (W3C SCXML transition guard compliance)",
        id_);
}

void ConcurrentRegion::setDoneStateCallback(std::function<void(const std::string &)> callback) {
    doneStateCallback_ = callback;
    LOG_DEBUG("ConcurrentRegion: Done state callback set for region: {} (W3C SCXML 3.4 compliance)", id_);
}

void ConcurrentRegion::setDesiredInitialChild(const std::string &childStateId) {
    desiredInitialChild_ = childStateId;
    LOG_DEBUG("ConcurrentRegion: Region '{}' desiredInitialChild set to '{}'", id_, childStateId);
}

// Private methods

bool ConcurrentRegion::validateRootState() const {
    if (!rootState_) {
        return false;
    }

    // Basic validation - can be extended with more sophisticated checks
    if (rootState_->getId().empty()) {
        LOG_ERROR("Root state has empty ID in region: {}", id_);
        return false;
    }

    return true;
}

void ConcurrentRegion::updateCurrentState() {
    if (!rootState_ || status_ != ConcurrentRegionStatus::ACTIVE) {
        currentState_.clear();
        activeStates_.clear();
        return;
    }

    // SCXML W3C specification section 3.4: Preserve hierarchical state tracking
    // Do not override currentState_ if it's already properly set by enterInitialState()
    // The currentState_ should reflect the actual active state in the hierarchy

    if (currentState_.empty()) {
        // Only set to root state if no current state is tracked
        currentState_ = rootState_->getId();
    }

    // Update active states list to include current state
    activeStates_.clear();
    activeStates_.push_back(currentState_);

    LOG_DEBUG("Region {} current state: {}", id_, currentState_);
}

std::vector<std::string> ConcurrentRegion::computeExitSet(const std::string &source, const std::string &target) const {
    std::vector<std::string> exitSet;

    // W3C SCXML Appendix D: Compute exit set for transition
    // Exit set = states from source up to (but not including) LCA with target

    // Helper lambda to find state node by ID within a subtree
    std::function<std::shared_ptr<IStateNode>(const std::shared_ptr<IStateNode> &, const std::string &)> findNode;
    findNode = [&findNode](const std::shared_ptr<IStateNode> &node,
                           const std::string &id) -> std::shared_ptr<IStateNode> {
        if (!node) {
            return nullptr;
        }
        if (node->getId() == id) {
            return node;
        }
        for (const auto &child : node->getChildren()) {
            if (auto found = findNode(child, id)) {
                return found;
            }
        }
        return nullptr;
    };

    // Helper lambda to search across all sibling regions (within parallel state)
    auto findInParallelState = [&](const std::string &stateId) -> std::shared_ptr<IStateNode> {
        IStateNode *parallelStatePtr = rootState_->getParent();
        if (!parallelStatePtr) {
            return nullptr;
        }

        // Search in all children (regions) of the parallel state
        const auto &parallelChildren = parallelStatePtr->getChildren();
        for (const auto &regionRoot : parallelChildren) {
            if (regionRoot) {
                if (auto found = findNode(regionRoot, stateId)) {
                    return found;
                }
            }
        }
        return nullptr;
    };

    // Try to find target in current region first
    auto targetNode = findNode(rootState_, target);
    bool isCrossRegion = false;

    // If not found in current region, check sibling regions
    if (!targetNode) {
        targetNode = findInParallelState(target);
        if (targetNode) {
            isCrossRegion = true;  // Target is in sibling region
        }
    }

    // Build path from source to root
    std::vector<std::string> sourcePath;
    auto sourceNode = findNode(rootState_, source);
    while (sourceNode) {
        sourcePath.push_back(sourceNode->getId());
        IStateNode *parent = sourceNode->getParent();
        sourceNode = parent ? findNode(rootState_, parent->getId()) : nullptr;
    }

    // Calculate exitSet based on transition type
    if (!targetNode) {
        // External transition: target is outside parallel state entirely
        // W3C SCXML: Exit all states from source to region root, include parallel state
        for (const auto &state : sourcePath) {
            exitSet.push_back(state);
            if (state == rootState_->getId()) {
                // Add parallel state to exitSet for external transitions
                IStateNode *parallelStatePtr = rootState_->getParent();
                if (parallelStatePtr) {
                    exitSet.push_back(parallelStatePtr->getId());
                }
                break;
            }
        }
    } else if (isCrossRegion) {
        // Cross-region transition: LCA is the parallel state
        // W3C SCXML: Exit all states from source up to (but not including) parallel state
        // This means: exit states in source region up to and including region root
        for (const auto &state : sourcePath) {
            exitSet.push_back(state);
            if (state == rootState_->getId()) {
                // Reached region root, stop here (don't include parallel state)
                break;
            }
        }
    } else {
        // Within-region transition: normal LCA calculation
        // Build path from target to root
        std::vector<std::string> targetPath;
        auto targetNodeCopy = targetNode;
        while (targetNodeCopy) {
            targetPath.push_back(targetNodeCopy->getId());
            IStateNode *parent = targetNodeCopy->getParent();
            targetNodeCopy = parent ? findNode(rootState_, parent->getId()) : nullptr;
        }

        // Find LCA (first common ancestor)
        std::string lca;
        for (const auto &sourceState : sourcePath) {
            for (const auto &targetState : targetPath) {
                if (sourceState == targetState) {
                    lca = sourceState;
                    break;
                }
            }
            if (!lca.empty()) {
                break;
            }
        }

        // Exit set = states from source up to (but not including) LCA
        for (const auto &state : sourcePath) {
            if (state == lca) {
                break;  // Don't include LCA
            }
            exitSet.push_back(state);
        }

        LOG_DEBUG("ConcurrentRegion::computeExitSet: {} -> {} (within-region, LCA: {}, exitSet size: {})", source,
                  target, lca, exitSet.size());
        return exitSet;
    }

    const char *transitionType = !targetNode ? "external" : (isCrossRegion ? "cross-region" : "within-region");
    LOG_DEBUG("ConcurrentRegion::computeExitSet: {} -> {} ({}, exitSet size: {})", source, target, transitionType,
              exitSet.size());
    return exitSet;
}

bool ConcurrentRegion::isDescendantOf(const std::shared_ptr<IStateNode> &root, const std::string &targetId) const {
    if (!root) {
        return false;
    }

    // Check if root itself is the target
    if (root->getId() == targetId) {
        return true;
    }

    // Recursively check all children
    const auto &children = root->getChildren();
    for (const auto &child : children) {
        if (child && isDescendantOf(child, targetId)) {
            return true;
        }
    }

    return false;
}

bool ConcurrentRegion::determineIfInFinalState() const {
    LOG_DEBUG(
        "ConcurrentRegion::determineIfInFinalState - Region {} checking final state. Status: {}, currentState: '{}'",
        id_, static_cast<int>(status_), currentState_);

    if (!rootState_ || status_ != ConcurrentRegionStatus::ACTIVE) {
        LOG_DEBUG("Region {} not active or no root state", id_);
        return false;
    }

    // SCXML W3C specification section 3.4: Check if current state is a final state
    if (currentState_.empty()) {
        return false;
    }

    // Check if the current state is a final state by searching through child states
    const auto &children = rootState_->getChildren();
    for (const auto &child : children) {
        if (child && child->getId() == currentState_) {
            bool isFinal = child->isFinalState();
            LOG_DEBUG("Region {} current state '{}' is {}", id_, currentState_, (isFinal ? "FINAL" : "NOT FINAL"));
            return isFinal;
        }
    }

    // If current state is the root state itself, check if root is final
    if (currentState_ == rootState_->getId()) {
        bool isFinal = rootState_->isFinalState();
        LOG_DEBUG("Region {} root state '{}' is {}", id_, currentState_, (isFinal ? "FINAL" : "NOT FINAL"));
        return isFinal;
    }

    LOG_WARN("Region {} current state '{}' not found in state hierarchy", id_, currentState_);
    return false;
}

ConcurrentOperationResult ConcurrentRegion::enterInitialState() {
    if (!rootState_) {
        std::string error = std::format("Cannot enter initial state: no root state in region {}", id_);
        return ConcurrentOperationResult::failure(id_, error);
    }

    LOG_DEBUG("Entering initial state for region: {}", id_);

    // SCXML W3C specification section 3.4: Execute entry actions for the region state
    if (executionContext_) {
        LOG_DEBUG("Executing entry actions for: {}", rootState_->getId());

        // W3C SCXML 3.8: Execute entry action blocks
        const auto &entryBlocks = rootState_->getEntryActionBlocks();
        if (!entryBlocks.empty()) {
            for (const auto &actionBlock : entryBlocks) {
                for (const auto &actionNode : actionBlock) {
                    if (actionNode) {
                        try {
                            LOG_DEBUG("Executing entry action: {}", actionNode->getId());
                            executeActionNode(actionNode, "enterInitialState");
                        } catch (const std::exception &e) {
                            LOG_WARN("Entry action failed: {}", e.what());
                        }
                    }
                }
            }
        }
    } else {
        LOG_DEBUG("No execution context available, skipping entry actions");
    }

    // Set up initial configuration
    currentState_ = rootState_->getId();
    activeStates_.clear();
    activeStates_.push_back(currentState_);

    // W3C SCXML 6.4: Check and defer invoke elements for root state itself
    const auto &rootInvokes = rootState_->getInvoke();
    LOG_INFO("ConcurrentRegion: Root state {} has {} invokes, callback is {}", rootState_->getId(), rootInvokes.size(),
             invokeCallback_ ? "set" : "null");
    if (!rootInvokes.empty() && invokeCallback_) {
        LOG_INFO("ConcurrentRegion: Delegating {} invokes for root state: {} to callback", rootInvokes.size(),
                 currentState_);
        invokeCallback_(currentState_, rootInvokes);
    }

    // Check if we need to enter child states
    const auto &children = rootState_->getChildren();
    if (!children.empty()) {
        // W3C SCXML 3.3: Priority order for initial state selection
        std::string initialChild;

        // Priority 1: Parent state's deep initial target (e.g., s1 initial="s11p112 s11p122")
        if (!desiredInitialChild_.empty()) {
            initialChild = desiredInitialChild_;
            LOG_DEBUG("ConcurrentRegion: Region '{}' using desiredInitialChild: '{}'", id_, initialChild);
        }
        // Priority 2: Region's <initial> element with transition target
        else if (const auto &initialTransition = rootState_->getInitialTransition();
                 initialTransition && !initialTransition->getTargets().empty()) {
            initialChild = initialTransition->getTargets()[0];
            LOG_DEBUG("Found initial transition targeting: {} in region: {}", initialChild, id_);
        }
        // Priority 3: Region's initial attribute
        else if (std::string initialFromAttr = rootState_->getInitialState(); !initialFromAttr.empty()) {
            initialChild = initialFromAttr;
            LOG_DEBUG("ConcurrentRegion: Region '{}' rootState '{}' has initialState='{}'", id_, rootState_->getId(),
                      initialChild);
        }
        // Priority 4: First child in document order (W3C default)
        else if (!children.empty()) {
            initialChild = children[0]->getId();
            LOG_DEBUG("ConcurrentRegion: Region '{}' using first child as fallback: '{}'", id_, initialChild);
        }

        if (!initialChild.empty()) {
            LOG_DEBUG("ConcurrentRegion: Region '{}' entering initial child state: '{}'", id_, initialChild);

            // Find the child state node once for efficiency
            auto childState = std::find_if(children.begin(), children.end(),
                                           [&initialChild](const std::shared_ptr<IStateNode> &child) {
                                               return child && child->getId() == initialChild;
                                           });

            if (childState != children.end() && *childState) {
                // W3C SCXML 3.10: History states never end up part of the configuration
                // If initial child is a history state, it will be handled by StateHierarchyManager
                // Do NOT add history state to activeStates_ - it must remain transparent
                if ((*childState)->getType() == Type::HISTORY) {
                    LOG_DEBUG("ConcurrentRegion: Initial child '{}' is HISTORY state, skipping activeStates addition "
                              "(W3C SCXML 3.10 compliance, test 580)",
                              initialChild);
                    // History restoration will be handled externally by StateHierarchyManager
                    // Do not set currentState_ or add to activeStates_
                    return ConcurrentOperationResult::success(id_);
                }

                // Normal state - add to active configuration
                activeStates_.push_back(initialChild);
                currentState_ = initialChild;

                // Execute entry actions for child state and handle recursive nesting
                if (executionContext_) {
                    // W3C SCXML 3.8: Execute child state's entry action blocks
                    const auto &childEntryBlocks = (*childState)->getEntryActionBlocks();
                    for (const auto &actionBlock : childEntryBlocks) {
                        for (const auto &actionNode : actionBlock) {
                            if (actionNode) {
                                LOG_DEBUG("Executing child entry action: {}", actionNode->getId());
                                if (!executeActionNode(actionNode, "enterInitialState")) {
                                    LOG_WARN("W3C SCXML 3.8: Child entry action failed, stopping remaining actions in "
                                             "THIS block only");
                                    break;  // W3C SCXML 3.8: stop remaining actions in this block
                                }
                            }
                        }
                    }

                    // W3C SCXML 6.4: Invoke elements must be processed after state entry
                    // Delegate to StateHierarchyManager via callback pattern for proper timing
                    const auto &childInvokes = (*childState)->getInvoke();
                    if (!childInvokes.empty() && invokeCallback_) {
                        LOG_INFO("ConcurrentRegion: Delegating {} invokes for child state: {} to callback",
                                 childInvokes.size(), initialChild);
                        invokeCallback_(initialChild, childInvokes);
                    }

                    // SCXML spec: If child state is compound, recursively enter its initial state
                    const auto &grandchildren = (*childState)->getChildren();
                    if (!grandchildren.empty()) {
                        std::string childInitialState = (*childState)->getInitialState();
                        if (childInitialState.empty() && !grandchildren.empty()) {
                            childInitialState = grandchildren[0]->getId();
                        }

                        if (!childInitialState.empty()) {
                            LOG_DEBUG("Child state is compound, entering "
                                      "grandchild: {}",
                                      childInitialState);
                            activeStates_.push_back(childInitialState);
                            currentState_ = childInitialState;

                            // Execute grandchild entry actions
                            auto grandchildState =
                                std::find_if(grandchildren.begin(), grandchildren.end(),
                                             [&childInitialState](const std::shared_ptr<IStateNode> &grandchild) {
                                                 return grandchild && grandchild->getId() == childInitialState;
                                             });

                            if (grandchildState != grandchildren.end() && *grandchildState) {
                                // W3C SCXML 3.8: Execute grandchild entry action blocks
                                const auto &grandchildEntryBlocks = (*grandchildState)->getEntryActionBlocks();
                                for (const auto &actionBlock : grandchildEntryBlocks) {
                                    for (const auto &actionNode : actionBlock) {
                                        if (actionNode) {
                                            LOG_DEBUG("Executing grandchild entry "
                                                      "action: {}",
                                                      actionNode->getId());
                                            if (!executeActionNode(actionNode, "enterInitialState")) {
                                                LOG_WARN("W3C SCXML 3.8: Grandchild entry action failed, stopping "
                                                         "remaining actions in THIS block only");
                                                break;  // W3C SCXML 3.8: stop remaining actions in this block
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    isInFinalState_ = determineIfInFinalState();

    // Update region status to FINAL if we entered a final state immediately
    if (isInFinalState_) {
        status_ = ConcurrentRegionStatus::FINAL;
        LOG_DEBUG(
            "ConcurrentRegion::enterInitialState - Region {} immediately entered final state, updating status to FINAL",
            id_);
    }

    LOG_DEBUG("Successfully entered initial state: {}", currentState_);
    return ConcurrentOperationResult::success(id_);
}

ConcurrentOperationResult ConcurrentRegion::exitAllStates(std::shared_ptr<IExecutionContext> executionContext) {
    LOG_DEBUG("Exiting all states in region: {}", id_);

    try {
        // SCXML W3C Specification compliance: Exit sequence for parallel states

        bool exitActionsSuccess = true;

        if (exitHandler_ && !activeStates_.empty()) {
            // W3C SCXML 3.13: Execute exit actions for all active states in document order
            // Note: activeStates_ already includes rootState_, so no need to execute it separately (test 504)
            LOG_DEBUG("Executing exit actions for active states");

            exitActionsSuccess = exitHandler_->executeMultipleStateExits(activeStates_, rootState_, executionContext);

            if (!exitActionsSuccess) {
                LOG_WARN("Some exit actions failed, continuing with cleanup");
            }
        } else {
            LOG_DEBUG("No exit handler or active states, skipping exit actions");
        }

        // Step 3: Clear the active configuration (always perform cleanup)
        // SCXML spec: Maintain legal state configuration after transition
        LOG_DEBUG("Clearing active configuration");
        activeStates_.clear();
        currentState_.clear();
        isInFinalState_ = false;

        // Step 4: Parent state notification would be handled by orchestrator
        // SOLID: Single Responsibility - ConcurrentRegion only manages its own state

        std::string resultMsg = std::format("Successfully exited all states in region: {}", id_);
        if (!exitActionsSuccess) {
            resultMsg += " (with exit action warnings)";
        }

        LOG_DEBUG("{}", resultMsg);
        return ConcurrentOperationResult::success(id_);

    } catch (const std::exception &e) {
        std::string errorMsg = std::format("Failed to exit states in region {}: {}", id_, e.what());
        LOG_ERROR("Error: {}", errorMsg);

        // Ensure cleanup even on failure
        activeStates_.clear();
        currentState_.clear();
        isInFinalState_ = false;

        return ConcurrentOperationResult::failure(id_, errorMsg);
    }
}

bool ConcurrentRegion::executeActionNode(const std::shared_ptr<IActionNode> &actionNode, const std::string &context) {
    if (!actionNode) {
        LOG_WARN("{} - Null ActionNode encountered, skipping", context);
        return false;
    }

    try {
        LOG_DEBUG("{} - Executing ActionNode: {} (ID: {})", context, actionNode->getActionType(), actionNode->getId());

        if (actionNode->execute(*executionContext_)) {
            LOG_DEBUG("{} - Successfully executed ActionNode: {}", context, actionNode->getActionType());
            return true;
        } else {
            LOG_WARN("{} - ActionNode failed: {}", context, actionNode->getActionType());
            return false;
        }
    } catch (const std::exception &e) {
        LOG_WARN("{} - ActionNode exception: {} Error: {}", context, actionNode->getActionType(), e.what());
        return false;
    }
}

void ConcurrentRegion::executeActionNodes(const std::vector<std::shared_ptr<IActionNode>> &actionNodes,
                                          const std::string &context) {
    // P1 refactoring: DRY principle - centralized action execution
    if (!executionContext_) {
        LOG_ERROR("ConcurrentRegion::executeActionNodes - Cannot execute actions for '{}': executionContext is null in "
                  "region '{}'",
                  context, id_);
        return;
    }

    if (actionNodes.empty()) {
        return;  // Nothing to execute
    }

    for (const auto &actionNode : actionNodes) {
        if (!actionNode) {
            LOG_WARN("ConcurrentRegion::executeActionNodes - Null ActionNode in '{}', skipping", context);
            continue;
        }

        try {
            if (!actionNode->execute(*executionContext_)) {
                LOG_WARN("ConcurrentRegion::executeActionNodes - ActionNode '{}' failed in '{}'",
                         actionNode->getActionType(), context);
            }
        } catch (const std::exception &e) {
            LOG_WARN("ConcurrentRegion::executeActionNodes - ActionNode '{}' exception in '{}': {}",
                     actionNode->getActionType(), context, e.what());
        }
    }
}

}  // namespace RSM
