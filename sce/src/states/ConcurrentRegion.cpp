// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "states/ConcurrentRegion.h"
#include "actions/AssignAction.h"
#include "actions/ScriptAction.h"
#include "core/LogMacros.h"
#include "events/EventDescriptor.h"
#include "model/IStateNode.h"
#include "model/ITransitionNode.h"
#include "runtime/IActionExecutor.h"
#include "runtime/IExecutionContext.h"
#include "states/StateExitExecutor.h"
#include <algorithm>
#include <format>

namespace SCE {

namespace {

/// Find a state by id within one region's subtree.
///
/// A region's active state is whatever leaf its own descent reached, which is
/// only a direct child of the region root when that child is atomic. The search
/// is bounded by the subtree so a region can never resolve an id belonging to a
/// sibling region.
std::shared_ptr<IStateNode> findStateInRegion(const std::shared_ptr<IStateNode> &root, const std::string &stateId) {
    if (!root) {
        return nullptr;
    }
    if (root->getId() == stateId) {
        return root;
    }
    for (const auto &child : root->getChildren()) {
        if (auto found = findStateInRegion(child, stateId)) {
            return found;
        }
    }
    return nullptr;
}

/// §scxml-D-isDescendant: STRICTLY below `ancestor`.
///
/// A state is not its own descendant, which is what keeps `findLCCA` from
/// collapsing a transition's domain onto a target that happens to be an
/// ancestor of the source.
bool containsProperDescendant(const IStateNode *ancestor, const std::string &stateId) {
    if (!ancestor || ancestor->getId() == stateId) {
        return false;
    }
    for (const auto &child : ancestor->getChildren()) {
        if (!child) {
            continue;
        }
        if (child->getId() == stateId || containsProperDescendant(child.get(), stateId)) {
            return true;
        }
    }
    return false;
}

}  // namespace

ConcurrentRegion::ConcurrentRegion(const std::string &id, std::shared_ptr<IStateNode> rootState,
                                   std::shared_ptr<IExecutionContext> executionContext)
    : id_(id), status_(ConcurrentRegionStatus::INACTIVE), rootState_(rootState), executionContext_(executionContext),
      isInFinalState_(false), exitHandler_(std::make_shared<StateExitExecutor>()) {
    // SCXML W3C specification section 3.4: regions must have valid identifiers
    assert(!id_.empty() && "SCXML violation: concurrent region must have non-empty ID");

    SCE_LOG_DEBUG("Creating region: {}", id_);

    if (rootState_) {
        SCE_LOG_DEBUG("Root state provided: {}", rootState_->getId());
    } else {
        SCE_LOG_DEBUG("No root state provided (will be set later)");
    }
}

ConcurrentRegion::~ConcurrentRegion() {
    SCE_LOG_DEBUG("Destroying region: {}", id_);

    // Clean deactivation if still active
    if (status_ == ConcurrentRegionStatus::ACTIVE) {
        SCE_LOG_DEBUG("Deactivating region during destruction");
        deactivate(nullptr);
    }
}

const std::string &ConcurrentRegion::getId() const {
    return id_;
}

ConcurrentOperationResult ConcurrentRegion::activate(bool enterDefaultChild) {
    if (status_ == ConcurrentRegionStatus::ACTIVE) {
        SCE_LOG_DEBUG("Region {} already active", id_);
        return ConcurrentOperationResult::success(id_);
    }

    // SCXML W3C specification section 3.4: regions must have root states
    if (!rootState_) {
        std::string error = std::format("SCXML violation: cannot activate region '{}' without root state. SCXML "
                                        "specification requires regions to have states.",
                                        id_);
        SCE_LOG_ERROR("Activate error: {}", error);
        setErrorState(error);
        return ConcurrentOperationResult::failure(id_, error);
    }

    // Validate root state before activation
    if (!validateRootState()) {
        std::string error = std::format("Root state validation failed for region: {}", id_);
        SCE_LOG_ERROR("Root state validation failed: {}", error);
        setErrorState(error);
        return ConcurrentOperationResult::failure(id_, error);
    }

    SCE_LOG_DEBUG("Activating region: {}", id_);

    // Mark region as active before entering initial state to enable final state detection
    status_ = ConcurrentRegionStatus::ACTIVE;

    // Enter initial state according to SCXML semantics
    auto result = enterInitialState(enterDefaultChild);
    if (!result.isSuccess) {
        SCE_LOG_ERROR("Failed to enter initial state: {}", result.errorMessage);
        status_ = ConcurrentRegionStatus::ERROR;  // Rollback on failure
        setErrorState(result.errorMessage);
        return result;
    }
    updateCurrentState();

    SCE_LOG_DEBUG("Successfully activated region: {}", id_);
    return ConcurrentOperationResult::success(id_);
}

ConcurrentOperationResult ConcurrentRegion::deactivate(std::shared_ptr<IExecutionContext> executionContext) {
    if (status_ == ConcurrentRegionStatus::INACTIVE) {
        SCE_LOG_DEBUG("Region {} already inactive", id_);
        return ConcurrentOperationResult::success(id_);
    }

    // §scxml-3.13: If activeStates_ is already empty, region was exited via exit set
    // Skip exitAllStates to avoid duplicate exit action execution (test 504)
    if (activeStates_.empty()) {
        SCE_LOG_DEBUG("Region {} activeStates already empty, skipping exitAllStates", id_);
        status_ = ConcurrentRegionStatus::INACTIVE;
        currentState_.clear();
        isInFinalState_ = false;
        SCE_LOG_DEBUG("Successfully deactivated region: {}", id_);
        return ConcurrentOperationResult::success(id_);
    }

    SCE_LOG_DEBUG("Deactivating region: {}", id_);

    // Exit all active states
    auto result = exitAllStates(executionContext);
    if (!result.isSuccess) {
        SCE_LOG_WARN("Warning during state exit: {}", result.errorMessage);
        // Continue with deactivation even if exit has issues
    }

    status_ = ConcurrentRegionStatus::INACTIVE;
    currentState_.clear();
    activeStates_.clear();
    isInFinalState_ = false;

    SCE_LOG_DEBUG("Successfully deactivated region: {}", id_);
    return ConcurrentOperationResult::success(id_);
}

bool ConcurrentRegion::isActive() const {
    return status_ == ConcurrentRegionStatus::ACTIVE;
}

bool ConcurrentRegion::isInFinalState() const {
    bool result = isInFinalState_ && status_ == ConcurrentRegionStatus::FINAL;
    return result;
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
        SCE_LOG_WARN("processEvent - {}", error);
        return ConcurrentOperationResult::failure(id_, error);
    }

    if (!rootState_) {
        std::string error = std::format("SCXML violation: cannot process event without root state in region: {}", id_);
        SCE_LOG_ERROR("Error: {}", error);
        setErrorState(error);
        return ConcurrentOperationResult::failure(id_, error);
    }

    SCE_LOG_DEBUG("Processing event '{}' in region: {}", event.eventName, id_);

    // §scxml-D-removeConflictingTransitions: Collect enabled transitions instead of executing immediately
    // This allows StateMachine to apply conflict resolution across all regions
    ConcurrentOperationResult result = ConcurrentOperationResult::success(id_);

    // §scxml-3.13: Hierarchical event bubbling - check from current state up through parent hierarchy
    if (!currentState_.empty()) {
        // Find the region's active state anywhere below the region root, not
        // only among its direct children. A region whose child is
        // itself compound comes to rest deeper than one level, and searching
        // one level down left such a region unable to locate its own active
        // state at all — so no transition in it was ever enabled and the region
        // sat still while its siblings moved.
        //
        // The search stays inside the region's own subtree, which is the same
        // boundary the bubbling loop below stops at.
        std::shared_ptr<IStateNode> stateNode = findStateInRegion(rootState_, currentState_);

        if (stateNode) {
            // §scxml-3.13: Hierarchical event bubbling (innermost to outermost)
            // Check from current active state up through parent hierarchy
            IStateNode *checkStatePtr = stateNode.get();  // Use raw pointer for hierarchy traversal
            int transitionIndex = 0;

            while (checkStatePtr) {
                const auto &transitions = checkStatePtr->getTransitions();

                // §scxml-3.13: Find first enabled transition in document order
                for (const auto &transition : transitions) {
                    // §scxml-3.12.1: Wildcard event matching - "*" matches any event
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
                            SCE_LOG_DEBUG(
                                "ConcurrentRegion: Evaluated guard condition '{}' for transition: {} -> result: {}",
                                guard, event.eventName, conditionResult ? "true" : "false");
                        } else {
                            SCE_LOG_WARN(
                                "ConcurrentRegion: Guard condition '{}' present but no evaluator set, defaulting "
                                "to true",
                                guard);
                        }
                    }

                    // Skip this transition if condition is false
                    if (!conditionResult) {
                        SCE_LOG_DEBUG("ConcurrentRegion: Skipping transition due to false guard condition: {}", guard);
                        transitionIndex++;
                        continue;
                    }

                    // Found enabled transition - collect it instead of executing
                    const auto &targets = transition->getTargets();
                    std::string targetState = targets.empty() ? checkStatePtr->getId() : targets[0];
                    bool isInternal = transition->isInternal();
                    bool hasActions = !transition->getActionNodes().empty();

                    SCE_LOG_DEBUG("ConcurrentRegion: Found enabled transition in state {}: {} -> {} (event='{}', "
                                  "internal={}, hasActions={})",
                                  checkStatePtr->getId(), checkStatePtr->getId(), targetState, transitionEvent,
                                  isInternal, hasActions);

                    // §scxml-D-computeExitSet: which active states this transition
                    // exits follows from its DOMAIN, and nothing else.
                    //
                    // What stood here asked a different question -- "is the target
                    // inside this <parallel>?" -- and answered "internal" whenever
                    // it was. That reading has no domain in it: an external
                    // transition written on a REGION ROOT keeps its target inside
                    // the parallel and still has the DOCUMENT ROOT as its domain,
                    // because §scxml-D-findLCCA filters the candidate ancestors
                    // with `isCompoundStateOrScxmlElement` and a <parallel> is
                    // neither. Measured 2026-08-25: the region kept its old leaf
                    // active alongside the new one, and the sibling region's
                    // transition on the same event was never preempted.
                    IStateNode *parallelStatePtr = rootState_->getParent();

                    // §scxml-D-computeExitSet: only a transition WITH a target
                    // contributes to the exit set -- the appendix guards the whole
                    // computation with `if t.target`. A targetless transition runs
                    // its content and exits nothing at all.
                    //
                    // This engine spells a targetless transition as source ->
                    // source, so the domain rule below would otherwise read it as
                    // a real self-transition and exit everything down to the
                    // domain. The old exit-set code hid that by accident: an LCA
                    // of a state with itself is the state, leaving the set empty.
                    auto exitSet = targets.empty() ? std::vector<std::string>{}
                                                   : computeExitSet(checkStatePtr, targetState, isInternal);

                    // "External" here means what the microstep below needs to know:
                    // the <parallel> itself is in the exit set, so it exits and the
                    // target is re-entered through a fresh entry path. That is
                    // exactly the case the domain lies above the <parallel>.
                    const bool isExternalTransition =
                        parallelStatePtr &&
                        std::find(exitSet.begin(), exitSet.end(), parallelStatePtr->getId()) != exitSet.end();

                    SCE_LOG_DEBUG("ConcurrentRegion: Transition {} -> {} exits {} states, parallel '{}' exits: {}",
                                  checkStatePtr->getId(), targetState, exitSet.size(),
                                  parallelStatePtr ? parallelStatePtr->getId() : std::string{}, isExternalTransition);

                    // Create transition descriptor for conflict resolution
                    TransitionDescriptorString descriptor;
                    descriptor.source = checkStatePtr->getId();
                    descriptor.target = targetState;
                    descriptor.event = event.eventName;
                    descriptor.transitionIndex = transitionIndex;
                    descriptor.hasActions = hasActions;
                    descriptor.isInternal = isInternal;
                    descriptor.isExternal = isExternalTransition;
                    descriptor.exitSet = std::move(exitSet);

                    SCE_LOG_DEBUG(
                        "ConcurrentRegion: Transition descriptor: {} -> {} (exitSet size: {}, transitionIndex: "
                        "{}, external: {})",
                        descriptor.source, descriptor.target, descriptor.exitSet.size(), descriptor.transitionIndex,
                        descriptor.isExternal);

                    result.enabledTransitions.push_back(descriptor);
                    return result;  // §scxml-3.13: First enabled transition wins in hierarchy
                }

                // §scxml-3.13: Move to parent state for hierarchical event bubbling
                // But STOP at region boundary - don't bubble beyond the region's root state
                // This prevents regions from collecting transitions from the parallel state's ancestors
                if (checkStatePtr == rootState_.get()) {
                    SCE_LOG_DEBUG("ConcurrentRegion: Reached region boundary at {}, stopping hierarchy bubbling",
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
    SCE_LOG_DEBUG("ConcurrentRegion: No enabled transitions found in region: {}", id_);
    return result;
}

std::shared_ptr<IStateNode> ConcurrentRegion::getRootState() const {
    return rootState_;
}

void ConcurrentRegion::setRootState(std::shared_ptr<IStateNode> rootState) {
    // SCXML W3C specification section 3.4: regions must have states
    assert(rootState && "SCXML violation: concurrent region cannot have null root state");

    if (status_ == ConcurrentRegionStatus::ACTIVE) {
        SCE_LOG_WARN(
            "ConcurrentRegion::setRootState - Setting root state on active region {} (consider deactivating first)",
            id_);
    }

    SCE_LOG_DEBUG("Setting root state for region {} to: {}", id_, (rootState ? rootState->getId() : "null"));

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
    SCE_LOG_DEBUG("Resetting region: {}", id_);

    // Deactivate if currently active
    if (status_ == ConcurrentRegionStatus::ACTIVE) {
        auto result = deactivate();
        if (!result.isSuccess) {
            SCE_LOG_ERROR("Failed to deactivate during reset: {}", result.errorMessage);
            return result;
        }
    }

    // Reset all state
    status_ = ConcurrentRegionStatus::INACTIVE;
    currentState_.clear();
    activeStates_.clear();
    isInFinalState_ = false;
    errorMessage_.clear();

    SCE_LOG_DEBUG("Successfully reset region: {}", id_);
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
    // §scxml-3.3: Validate that state belongs to this region
    // This is called during deep initial target synchronization
    if (!stateId.empty() && rootState_) {
        // Validate the state is within this region's scope
        bool isValidState = isDescendantOf(rootState_, stateId);
        if (!isValidState) {
            SCE_LOG_WARN(
                "ConcurrentRegion: Attempting to set currentState to '{}' which is not within region '{}' scope",
                stateId, id_);
            // Continue anyway - StateHierarchyManager knows best in deep target scenarios
        }
    }

    SCE_LOG_DEBUG("ConcurrentRegion: Setting currentState for region {} to: {}", id_, stateId);

    // Detect if this is a state change or just a refresh (e.g., during snapshot restore)
    // Only trigger callbacks on actual state transitions, not on redundant setCurrentState calls
    bool isStateChange = (currentState_ != stateId);

    currentState_ = stateId;

    // §scxml-3.4: Update isInFinalState_ flag when currentState changes
    // This is crucial for parallel state completion detection
    isInFinalState_ = determineIfInFinalState();

    // Update region status to FINAL if we entered a final state
    // Only trigger callback on actual state transitions (not during snapshot restore)
    // Skip callback during restoration to prevent spurious event generation
    if (isInFinalState_ && status_ != ConcurrentRegionStatus::FINAL && isStateChange && !isRestoringSnapshot_) {
        status_ = ConcurrentRegionStatus::FINAL;
        SCE_LOG_DEBUG("ConcurrentRegion: Region {} entered final state '{}', updating status to FINAL", id_, stateId);

        // §scxml-3.13: Generate done.state.{regionId} event when compound state enters final
        // test570: When p0s1 (compound region) reaches p0s1final, generate done.state.p0s1
        if (doneStateCallback_) {
            SCE_LOG_DEBUG("ConcurrentRegion: Calling doneStateCallback for region {}", id_);
            doneStateCallback_(id_);
        }
    }
}

void ConcurrentRegion::setActiveForRestore() {
    // Set region to ACTIVE status without executing entry actions
    // This is used during time-travel debugging snapshot restoration
    status_ = ConcurrentRegionStatus::ACTIVE;

    // Synchronize activeStates_ with currentState_
    // activeStates_ tracks which states within the region are active
    // Without this, the region thinks it has no active states, causing incorrect exit behavior
    activeStates_.clear();
    if (!currentState_.empty()) {
        activeStates_.push_back(currentState_);
    }

    SCE_LOG_DEBUG("Region '{}' marked as ACTIVE for restoration (current state: {}, activeStates: {})", id_,
                  currentState_, activeStates_.size());
}

void ConcurrentRegion::setRestoringSnapshot(bool restoring) {
    // Enable/disable restoration mode for time-travel debugging
    // When enabled, prevents side effects (callbacks, event generation) during snapshot restoration
    isRestoringSnapshot_ = restoring;
    SCE_LOG_DEBUG("Region '{}' restoration mode: {} [isRestoringSnapshot_={}]", id_, restoring ? "ENABLED" : "DISABLED",
                  isRestoringSnapshot_.load());
}

bool ConcurrentRegion::isInErrorState() const {
    return status_ == ConcurrentRegionStatus::ERROR;
}

void ConcurrentRegion::setErrorState(const std::string &errorMessage) {
    SCE_LOG_ERROR("Region {} entering error state: {}", id_, errorMessage);
    status_ = ConcurrentRegionStatus::ERROR;
    errorMessage_ = errorMessage;

    // Clear other state information when in error
    currentState_.clear();
    activeStates_.clear();
    isInFinalState_ = false;
}

void ConcurrentRegion::clearErrorState() {
    if (status_ == ConcurrentRegionStatus::ERROR) {
        SCE_LOG_DEBUG("Clearing error state for region: {}", id_);
        status_ = ConcurrentRegionStatus::INACTIVE;
        errorMessage_.clear();
    }
}

void ConcurrentRegion::setExecutionContext(std::shared_ptr<IExecutionContext> executionContext) {
    SCE_LOG_DEBUG("Setting ExecutionContext for region: {} - new context is {}", id_,
                  executionContext ? "valid" : "null");
    executionContext_ = executionContext;
    SCE_LOG_DEBUG("ExecutionContext set successfully for region: {} - stored context is {}", id_,
                  executionContext_ ? "valid" : "null");
}

void ConcurrentRegion::setInvokeCallback(
    std::function<void(const std::string &, const std::vector<std::shared_ptr<IInvokeNode>> &)> callback) {
    invokeCallback_ = callback;
    SCE_LOG_DEBUG("ConcurrentRegion: Invoke callback set for region: {} (W3C SCXML 6.4 compliance)", id_);
}

void ConcurrentRegion::setConditionEvaluator(std::function<bool(const std::string &)> evaluator) {
    conditionEvaluator_ = evaluator;
    SCE_LOG_DEBUG(
        "ConcurrentRegion: Condition evaluator callback set for region: {} (W3C SCXML transition guard compliance)",
        id_);
}

void ConcurrentRegion::setDoneStateCallback(std::function<void(const std::string &)> callback) {
    doneStateCallback_ = callback;
    SCE_LOG_DEBUG("ConcurrentRegion: Done state callback set for region: {} (W3C SCXML 3.4 compliance)", id_);
}

void ConcurrentRegion::setDesiredInitialChild(const std::string &childStateId) {
    desiredInitialChild_ = childStateId;
    SCE_LOG_DEBUG("ConcurrentRegion: Region '{}' desiredInitialChild set to '{}'", id_, childStateId);
}

// Private methods

bool ConcurrentRegion::validateRootState() const {
    if (!rootState_) {
        return false;
    }

    // Basic validation - can be extended with more sophisticated checks
    if (rootState_->getId().empty()) {
        SCE_LOG_ERROR("Root state has empty ID in region: {}", id_);
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

    SCE_LOG_DEBUG("Region {} current state: {}", id_, currentState_);
}

IStateNode *ConcurrentRegion::computeTransitionDomain(IStateNode *sourceNode, const std::string &target,
                                                      bool isInternal) const {
    if (!sourceNode) {
        return nullptr;
    }

    // §scxml-D-getTransitionDomain: an internal transition whose source is a
    // compound state and whose targets are all proper descendants of it has the
    // SOURCE as its domain -- the source itself never exits, so the other
    // regions of an enclosing <parallel> are untouched.
    if (isInternal && sourceNode->getType() == Type::COMPOUND && containsProperDescendant(sourceNode, target)) {
        return sourceNode;
    }

    // §scxml-D-findLCCA: the proper ancestors of the source, filtered by
    // `isCompoundStateOrScxmlElement`, first one containing every target.
    //
    // A <parallel> answers neither predicate, so it is skipped -- which is the
    // entire difference between this and a plain lowest-common-ancestor, and it
    // only ever shows up when a <parallel> sits between the source and the
    // first compound <state> above it. That is precisely a transition written
    // on a REGION ROOT, and a transition crossing from one region to another.
    for (IStateNode *ancestor = sourceNode->getParent(); ancestor != nullptr; ancestor = ancestor->getParent()) {
        if (ancestor->getType() != Type::COMPOUND) {
            continue;
        }
        if (containsProperDescendant(ancestor, target)) {
            return ancestor;
        }
    }

    // Out of ancestors: the domain is the <scxml> element, of which every
    // active state is a descendant.
    return nullptr;
}

std::vector<std::string> ConcurrentRegion::computeExitSet(IStateNode *sourceNode, const std::string &target,
                                                          bool isInternal) const {
    std::vector<std::string> exitSet;

    if (!rootState_) {
        return exitSet;
    }

    IStateNode *domain = computeTransitionDomain(sourceNode, target, isInternal);

    // §scxml-D-computeExitSet: the ACTIVE proper descendants of the domain.
    //
    // The walk starts at the region's active leaf, not at the transition's
    // source: a transition written on a region root has active descendants
    // BELOW its source, and reading the source's own chain -- which is what
    // stood here -- left them active beside the newly entered state, a
    // configuration in which two children of one compound state are active at
    // once.
    IStateNode *leaf = findStateInRegion(rootState_, currentState_).get();
    if (!leaf) {
        leaf = rootState_.get();
    }

    for (IStateNode *node = leaf; node != nullptr; node = node->getParent()) {
        if (node == domain) {
            break;  // The domain itself is not exited.
        }
        exitSet.push_back(node->getId());

        if (node == rootState_.get()) {
            // The walk left the region, so the domain is above the enclosing
            // <parallel> -- a <parallel> is never a domain itself -- and the
            // <parallel> is a proper descendant of the domain too.
            if (IStateNode *parallelStatePtr = rootState_->getParent()) {
                exitSet.push_back(parallelStatePtr->getId());
            }
            break;
        }
    }

    SCE_LOG_DEBUG("ConcurrentRegion::computeExitSet: {} -> {} (internal: {}, domain: {}, exitSet size: {})",
                  sourceNode ? sourceNode->getId() : std::string{}, target, isInternal,
                  domain ? domain->getId() : std::string{"<scxml>"}, exitSet.size());
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
    SCE_LOG_DEBUG(
        "ConcurrentRegion::determineIfInFinalState - Region {} checking final state. Status: {}, currentState: '{}'",
        id_, static_cast<int>(status_), currentState_);

    // §scxml-3.4: Check final state even if status is already FINAL
    // This handles cases where setCurrentState() is called multiple times (e.g., during hierarchyManager sync)
    // §scxml-D-isInFinalState: the compound-state branch — a region is in a final
    // state when one of its child states is a <final> and is currently active.
    if (!rootState_) {
        SCE_LOG_DEBUG("Region {} has no root state", id_);
        return false;
    }

    // Allow checking final state even when status is FINAL (status check removed)
    // Previously: if (status_ != ACTIVE) return false; ← This was causing the bug!

    // SCXML W3C specification section 3.4: Check if current state is a final state
    if (currentState_.empty()) {
        return false;
    }

    // §scxml-D-isInFinalState: a compound state is in a final state exactly when
    // the child of it that is in the configuration is a `<final>`. Direct
    // children are the rule here, not a shortcut past the hierarchy: a
    // `<final>` has no children, so a final child is always itself the active
    // leaf, and a region resting deeper is one whose child on that path is a
    // compound state — not final, and correctly reported so.
    const auto &children = rootState_->getChildren();
    for (const auto &child : children) {
        if (child && child->getId() == currentState_) {
            bool isFinal = child->isFinalState();
            SCE_LOG_DEBUG("Region {} current state '{}' is {}", id_, currentState_, (isFinal ? "FINAL" : "NOT FINAL"));
            return isFinal;
        }
    }

    // If current state is the root state itself, check if root is final
    if (currentState_ == rootState_->getId()) {
        bool isFinal = rootState_->isFinalState();
        SCE_LOG_DEBUG("Region {} root state '{}' is {}", id_, currentState_, (isFinal ? "FINAL" : "NOT FINAL"));
        return isFinal;
    }

    // Resting below a direct child is ordinary — a region whose child is itself
    // compound is there whenever it is not in one of its own `<final>`s. It was
    // logged as a hierarchy error, which is what made an every-run condition
    // read as a defect.
    SCE_LOG_DEBUG("Region {} rests below a direct child at '{}' — not final", id_, currentState_);
    return false;
}

ConcurrentOperationResult ConcurrentRegion::enterInitialState(bool enterDefaultChild) {
    if (!rootState_) {
        std::string error = std::format("Cannot enter initial state: no root state in region {}", id_);
        return ConcurrentOperationResult::failure(id_, error);
    }

    SCE_LOG_DEBUG("Entering initial state for region: {}", id_);

    // §scxml-3.8: this region executes the `<onentry>` of every state it
    // enters, and it is the ONLY executor of them.
    //
    // `StateHierarchyManager` reflects this region's `getActiveStates()` into
    // the machine's configuration afterwards, and it does so through
    // `addStateToConfigurationWithoutOnEntry` precisely because these have
    // already run here. Routing them through the onentry callback as well ran
    // every one of them twice — measured 2026-08-15, a counter incremented by
    // 2 for one entry, which neither W3C IRP nor any earlier integration
    // fixture could see because none of them counts entries inside a region.
    //
    // This site is the survivor rather than the callback because it is this
    // class's own contract: `ConcurrentRegionTest` constructs a region with an
    // execution context and no manager at all, and asserts that activation
    // runs the root state's entry actions.
    if (executionContext_) {
        SCE_LOG_DEBUG("Executing entry actions for: {}", rootState_->getId());

        // §scxml-3.8: Execute entry action blocks
        const auto &entryBlocks = rootState_->getEntryActionBlocks();
        if (!entryBlocks.empty()) {
            for (const auto &actionBlock : entryBlocks) {
                for (const auto &actionNode : actionBlock) {
                    if (actionNode) {
                        try {
                            SCE_LOG_DEBUG("Executing entry action: {}", actionNode->getId());
                            executeActionNode(actionNode, "enterInitialState");
                        } catch (const std::exception &e) {
                            SCE_LOG_WARN("Entry action failed: {}", e.what());
                        }
                    }
                }
            }
        }
    } else {
        SCE_LOG_DEBUG("No execution context available, skipping entry actions");
    }

    // Set up initial configuration
    currentState_ = rootState_->getId();
    activeStates_.clear();
    activeStates_.push_back(currentState_);

    // §scxml-6.4: Check and defer invoke elements for root state itself
    const auto &rootInvokes = rootState_->getInvoke();
    SCE_LOG_INFO("ConcurrentRegion: Root state {} has {} invokes, callback is {}", rootState_->getId(),
                 rootInvokes.size(), invokeCallback_ ? "set" : "null");
    if (!rootInvokes.empty() && invokeCallback_) {
        SCE_LOG_INFO("ConcurrentRegion: Delegating {} invokes for root state: {} to callback", rootInvokes.size(),
                     currentState_);
        invokeCallback_(currentState_, rootInvokes);
    }

    // §scxml-D-addAncestorStatesToEnter: the caller is descending into this
    // region toward a state it named, so the root's default child is not on that
    // path and entering it here would leave two children of the root active at
    // once. The region is ACTIVE and holds its root; the caller supplies the
    // rest, and `StateHierarchyManager::updateParallelRegionCurrentStates` syncs
    // this bookkeeping from the configuration afterwards.
    if (!enterDefaultChild) {
        SCE_LOG_DEBUG("ConcurrentRegion: Region '{}' entered as an ancestor — root only, no default child", id_);
        isInFinalState_ = determineIfInFinalState();
        return ConcurrentOperationResult::success(id_);
    }

    // Check if we need to enter child states
    const auto &children = rootState_->getChildren();
    if (!children.empty()) {
        // §scxml-3.3: Priority order for initial state selection
        std::string initialChild;

        // Priority 1: Parent state's deep initial target (e.g., s1 initial="s11p112 s11p122")
        if (!desiredInitialChild_.empty()) {
            initialChild = desiredInitialChild_;
            SCE_LOG_DEBUG("ConcurrentRegion: Region '{}' using desiredInitialChild: '{}'", id_, initialChild);
        }
        // Priority 2: Region's <initial> element with transition target
        else if (const auto &initialTransition = rootState_->getInitialTransition();
                 initialTransition && !initialTransition->getTargets().empty()) {
            initialChild = initialTransition->getTargets()[0];
            SCE_LOG_DEBUG("Found initial transition targeting: {} in region: {}", initialChild, id_);
        }
        // Priority 3: Region's initial attribute
        else if (std::string initialFromAttr = rootState_->getInitialState(); !initialFromAttr.empty()) {
            initialChild = initialFromAttr;
            SCE_LOG_DEBUG("ConcurrentRegion: Region '{}' rootState '{}' has initialState='{}'", id_,
                          rootState_->getId(), initialChild);
        }
        // Priority 4: First child in document order (W3C default)
        else if (!children.empty()) {
            initialChild = children[0]->getId();
            SCE_LOG_DEBUG("ConcurrentRegion: Region '{}' using first child as fallback: '{}'", id_, initialChild);
        }

        if (!initialChild.empty()) {
            SCE_LOG_DEBUG("ConcurrentRegion: Region '{}' entering initial child state: '{}'", id_, initialChild);

            // Find the child state node once for efficiency
            auto childState = std::find_if(children.begin(), children.end(),
                                           [&initialChild](const std::shared_ptr<IStateNode> &child) {
                                               return child && child->getId() == initialChild;
                                           });

            if (childState != children.end() && *childState) {
                // §scxml-3.10: History states never end up part of the configuration
                // If initial child is a history state, it will be handled by StateHierarchyManager
                // Do NOT add history state to activeStates_ - it must remain transparent
                if ((*childState)->getType() == Type::HISTORY) {
                    SCE_LOG_DEBUG(
                        "ConcurrentRegion: Initial child '{}' is HISTORY state, skipping activeStates addition "
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
                    // §scxml-3.8: Execute child state's entry action blocks
                    const auto &childEntryBlocks = (*childState)->getEntryActionBlocks();
                    for (const auto &actionBlock : childEntryBlocks) {
                        for (const auto &actionNode : actionBlock) {
                            if (actionNode) {
                                SCE_LOG_DEBUG("Executing child entry action: {}", actionNode->getId());
                                if (!executeActionNode(actionNode, "enterInitialState")) {
                                    SCE_LOG_WARN(
                                        "W3C SCXML 3.8: Child entry action failed, stopping remaining actions in "
                                        "THIS block only");
                                    break;  // §scxml-3.8: stop remaining actions in this block
                                }
                            }
                        }
                    }

                    // §scxml-6.4: Invoke elements must be processed after state entry
                    // Delegate to StateHierarchyManager via callback pattern for proper timing
                    const auto &childInvokes = (*childState)->getInvoke();
                    if (!childInvokes.empty() && invokeCallback_) {
                        SCE_LOG_INFO("ConcurrentRegion: Delegating {} invokes for child state: {} to callback",
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
                            SCE_LOG_DEBUG("Child state is compound, entering "
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
                                // §scxml-3.8: Execute grandchild entry action blocks
                                const auto &grandchildEntryBlocks = (*grandchildState)->getEntryActionBlocks();
                                for (const auto &actionBlock : grandchildEntryBlocks) {
                                    for (const auto &actionNode : actionBlock) {
                                        if (actionNode) {
                                            SCE_LOG_DEBUG("Executing grandchild entry "
                                                          "action: {}",
                                                          actionNode->getId());
                                            if (!executeActionNode(actionNode, "enterInitialState")) {
                                                SCE_LOG_WARN("W3C SCXML 3.8: Grandchild entry action failed, stopping "
                                                             "remaining actions in THIS block only");
                                                break;  // §scxml-3.8: stop remaining actions in this block
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
        SCE_LOG_DEBUG(
            "ConcurrentRegion::enterInitialState - Region {} immediately entered final state, updating status to FINAL",
            id_);
    }

    SCE_LOG_DEBUG("Successfully entered initial state: {}", currentState_);
    return ConcurrentOperationResult::success(id_);
}

ConcurrentOperationResult ConcurrentRegion::exitAllStates(std::shared_ptr<IExecutionContext> executionContext) {
    SCE_LOG_DEBUG("Exiting all states in region: {}", id_);

    try {
        // SCXML W3C Specification compliance: Exit sequence for parallel states

        bool exitActionsSuccess = true;

        if (exitHandler_ && !activeStates_.empty()) {
            // §scxml-3.13: Execute exit actions for all active states in document order
            // Note: activeStates_ already includes rootState_, so no need to execute it separately (test 504)
            SCE_LOG_DEBUG("Executing exit actions for active states");

            exitActionsSuccess = exitHandler_->executeMultipleStateExits(activeStates_, rootState_, executionContext);

            if (!exitActionsSuccess) {
                SCE_LOG_WARN("Some exit actions failed, continuing with cleanup");
            }
        } else {
            SCE_LOG_DEBUG("No exit handler or active states, skipping exit actions");
        }

        // Step 3: Clear the active configuration (always perform cleanup)
        // SCXML spec: Maintain legal state configuration after transition
        SCE_LOG_DEBUG("Clearing active configuration");
        activeStates_.clear();
        currentState_.clear();
        isInFinalState_ = false;

        // Step 4: Parent state notification would be handled by orchestrator
        // SOLID: Single Responsibility - ConcurrentRegion only manages its own state

        std::string resultMsg = std::format("Successfully exited all states in region: {}", id_);
        if (!exitActionsSuccess) {
            resultMsg += " (with exit action warnings)";
        }

        SCE_LOG_DEBUG("{}", resultMsg);
        return ConcurrentOperationResult::success(id_);

    } catch (const std::exception &e) {
        std::string errorMsg = std::format("Failed to exit states in region {}: {}", id_, e.what());
        SCE_LOG_ERROR("Error: {}", errorMsg);

        // Ensure cleanup even on failure
        activeStates_.clear();
        currentState_.clear();
        isInFinalState_ = false;

        return ConcurrentOperationResult::failure(id_, errorMsg);
    }
}

bool ConcurrentRegion::executeActionNode(const std::shared_ptr<IActionNode> &actionNode, const std::string &context) {
    if (!actionNode) {
        SCE_LOG_WARN("{} - Null ActionNode encountered, skipping", context);
        return false;
    }

    try {
        SCE_LOG_DEBUG("{} - Executing ActionNode: {} (ID: {})", context, actionNode->getActionType(),
                      actionNode->getId());

        if (actionNode->execute(*executionContext_)) {
            SCE_LOG_DEBUG("{} - Successfully executed ActionNode: {}", context, actionNode->getActionType());
            return true;
        } else {
            SCE_LOG_WARN("{} - ActionNode failed: {}", context, actionNode->getActionType());
            return false;
        }
    } catch (const std::exception &e) {
        SCE_LOG_WARN("{} - ActionNode exception: {} Error: {}", context, actionNode->getActionType(), e.what());
        return false;
    }
}

void ConcurrentRegion::executeActionNodes(const std::vector<std::shared_ptr<IActionNode>> &actionNodes,
                                          const std::string &context) {
    // P1 refactoring: DRY principle - centralized action execution
    if (!executionContext_) {
        SCE_LOG_ERROR(
            "ConcurrentRegion::executeActionNodes - Cannot execute actions for '{}': executionContext is null in "
            "region '{}'",
            context, id_);
        return;
    }

    if (actionNodes.empty()) {
        return;  // Nothing to execute
    }

    for (const auto &actionNode : actionNodes) {
        if (!actionNode) {
            SCE_LOG_WARN("ConcurrentRegion::executeActionNodes - Null ActionNode in '{}', skipping", context);
            continue;
        }

        try {
            if (!actionNode->execute(*executionContext_)) {
                SCE_LOG_WARN("ConcurrentRegion::executeActionNodes - ActionNode '{}' failed in '{}'",
                             actionNode->getActionType(), context);
            }
        } catch (const std::exception &e) {
            SCE_LOG_WARN("ConcurrentRegion::executeActionNodes - ActionNode '{}' exception in '{}': {}",
                         actionNode->getActionType(), context, e.what());
        }
    }
}

}  // namespace SCE
