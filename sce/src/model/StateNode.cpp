// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "model/StateNode.h"
#include "core/LogMacros.h"

SCE::StateNode::StateNode(const std::string &id, Type type) : id_(id), type_(type), parent_(nullptr) {
    SCE_LOG_DEBUG("Creating state node: {}, type: {}", id, static_cast<int>(type));
}

SCE::StateNode::~StateNode() {
    SCE_LOG_DEBUG("Destroying state node: {}", id_);
}

const std::string &SCE::StateNode::getId() const {
    return id_;
}

SCE::Type SCE::StateNode::getType() const {
    return type_;
}

void SCE::StateNode::setParent(SCE::IStateNode *parent) {
    SCE_LOG_DEBUG("Setting parent for {}: {}", id_, (parent ? parent->getId() : "null"));
    parent_ = parent;
}

SCE::IStateNode *SCE::StateNode::getParent() const {
    return parent_;
}

void SCE::StateNode::addChild(std::shared_ptr<SCE::IStateNode> child) {
    if (child) {
        SCE_LOG_DEBUG("Adding child to {}: {}", id_, child->getId());
        children_.push_back(child);
    } else {
        SCE_LOG_WARN("Attempt to add null child to {}", id_);
    }
}

const std::vector<std::shared_ptr<SCE::IStateNode>> &SCE::StateNode::getChildren() const {
    return children_;
}

void SCE::StateNode::addTransition(std::shared_ptr<SCE::ITransitionNode> transition) {
    if (transition) {
        const auto targets = transition->getTargets();
        std::string targetStr = targets.empty() ? "" : (targets.size() == 1 ? targets[0] : "[multiple targets]");

        SCE_LOG_DEBUG("Adding transition to {}: event={}, target={}", id_, transition->getEvent(), targetStr);
        transitions_.push_back(transition);
    } else {
        SCE_LOG_WARN("Attempt to add null transition to {}", id_);
    }
}

const std::vector<std::shared_ptr<SCE::ITransitionNode>> &SCE::StateNode::getTransitions() const {
    return transitions_;
}

// Implementation of newly added data model related methods
void SCE::StateNode::addDataItem(std::shared_ptr<SCE::IDataModelItem> dataItem) {
    if (dataItem) {
        SCE_LOG_DEBUG("Adding data item to {}: {}", id_, dataItem->getId());
        dataItems_.push_back(dataItem);
    } else {
        SCE_LOG_WARN("Attempt to add null data item to {}", id_);
    }
}

const std::vector<std::shared_ptr<SCE::IDataModelItem>> &SCE::StateNode::getDataItems() const {
    return dataItems_;
}

void SCE::StateNode::setInitialState(const std::string &initialState) {
    SCE_LOG_DEBUG("Setting initial state for {}: {}", id_, initialState);
    initialState_ = initialState;
}

const std::string &SCE::StateNode::getInitialState() const {
    return initialState_;
}

void SCE::StateNode::setOnEntry(const std::string &callback) {
    SCE_LOG_DEBUG("Setting onEntry callback for {}: {}", id_, callback);
    onEntry_ = callback;
}

const std::string &SCE::StateNode::getOnEntry() const {
    return onEntry_;
}

void SCE::StateNode::setOnExit(const std::string &callback) {
    SCE_LOG_DEBUG("Setting onExit callback for {}: {}", id_, callback);
    onExit_ = callback;
}

const std::string &SCE::StateNode::getOnExit() const {
    return onExit_;
}

void SCE::StateNode::addInvoke(std::shared_ptr<SCE::IInvokeNode> invoke) {
    if (invoke) {
        SCE_LOG_DEBUG("Adding invoke to {}: {}", id_, invoke->getId());
        invokes_.push_back(invoke);
    } else {
        SCE_LOG_WARN("Attempt to add null invoke to {}", id_);
    }
}

const std::vector<std::shared_ptr<SCE::IInvokeNode>> &SCE::StateNode::getInvoke() const {
    return invokes_;
}

bool SCE::StateNode::isFinalState() const {
    return type_ == Type::FINAL;
}

const SCE::DoneData &SCE::StateNode::getDoneData() const {
    return doneData_;
}

SCE::DoneData &SCE::StateNode::getDoneData() {
    return doneData_;
}

void SCE::StateNode::setDoneDataContent(const std::string &content) {
    SCE_LOG_DEBUG("Setting donedata content for {}", id_);
    doneData_.setContent(content);
}

void SCE::StateNode::addDoneDataParam(const std::string &name, const std::string &location) {
    SCE_LOG_DEBUG("Adding param to donedata for {}: {} -> {}", id_, name, location);
    doneData_.addParam(name, location);
}

void SCE::StateNode::clearDoneDataParams() {
    doneData_.clearParams();
}

std::shared_ptr<SCE::ITransitionNode> SCE::StateNode::getInitialTransition() const {
    return initialTransition_;
}

void SCE::StateNode::setInitialTransition(std::shared_ptr<SCE::ITransitionNode> transition) {
    SCE_LOG_DEBUG("Setting initial transition for {}", id_);
    initialTransition_ = transition;
}

// W3C SCXML 3.8/3.9: Block-based action methods
void SCE::StateNode::addEntryActionBlock(std::vector<std::shared_ptr<SCE::IActionNode>> block) {
    if (!block.empty()) {
        SCE_LOG_DEBUG("W3C SCXML 3.8: Adding entry action block to {} with {} actions", id_, block.size());
        entryActionBlocks_.push_back(std::move(block));
    } else {
        SCE_LOG_WARN("Attempted to add empty entry action block to {}", id_);
    }
}

const std::vector<std::vector<std::shared_ptr<SCE::IActionNode>>> &SCE::StateNode::getEntryActionBlocks() const {
    return entryActionBlocks_;
}

void SCE::StateNode::addExitActionBlock(std::vector<std::shared_ptr<SCE::IActionNode>> block) {
    if (!block.empty()) {
        SCE_LOG_DEBUG("W3C SCXML 3.9: Adding exit action block to {} with {} actions", id_, block.size());
        exitActionBlocks_.push_back(std::move(block));
    } else {
        SCE_LOG_WARN("Attempted to add empty exit action block to {}", id_);
    }
}

const std::vector<std::vector<std::shared_ptr<SCE::IActionNode>>> &SCE::StateNode::getExitActionBlocks() const {
    return exitActionBlocks_;
}
