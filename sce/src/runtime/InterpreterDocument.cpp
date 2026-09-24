// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "runtime/InterpreterDocument.h"

#include "core/LogMacros.h"
#include "model/IStateNode.h"
#include "model/ITransitionNode.h"
#include "model/SCXMLModel.h"

#include <sstream>

namespace SCE {

namespace {

std::vector<std::string> tokensOf(const std::string &text) {
    std::vector<std::string> tokens;
    std::istringstream in(text);
    std::string token;
    while (in >> token) {
        tokens.push_back(token);
    }
    return tokens;
}

bool isStateElement(Type type) {
    return type != Type::HISTORY && type != Type::INITIAL;
}

}  // namespace

InterpreterDocument::InterpreterDocument(const SCXMLModel &model) {
    // Document order is a pre-order walk of the whole document: every
    // top-level state in the order `<scxml>` holds them, each followed by its
    // subtree.
    int order = 0;
    for (const auto &topLevel : model.getTopLevelStates()) {
        indexSubtree(topLevel.get(), std::nullopt, order);
    }

    // Initial targets and history defaults may name any state or `<history>`
    // in the document, so they are read only once every id is known.
    for (auto &[id, entry] : states_) {
        if (entry.type == Type::COMPOUND) {
            entry.initial = defaultInitialOf(entry);
            if (entry.initial.empty()) {
                SCE_LOG_ERROR("InterpreterDocument: compound state '{}' names no initial state and has no child state",
                              id);
            }
        }
    }
    for (auto &[id, history] : histories_) {
        const auto &transitions = history.node->getTransitions();
        if (!transitions.empty() && !transitions.front()->getTargets().empty()) {
            history.defaults = targetsOf(transitions.front()->getTargets());
            continue;
        }
        // A `<history>` without a default transition has nothing to hand the
        // appendix when nothing was recorded; its parent's first child state
        // is the entry a document with no history would have made.
        SCE_LOG_ERROR("InterpreterDocument: <history> '{}' has no default transition", id);
        const auto parent = states_.find(history.parent);
        if (parent != states_.end() && !parent->second.children.empty()) {
            history.defaults = {Target::onState(parent->second.children.front())};
        }
    }

    const auto &written = model.getInitialStates();
    if (!written.empty()) {
        documentInitial_ = targetsOf(written);
    } else if (!model.getTopLevelStates().empty()) {
        documentInitial_ = {Target::onState(model.getTopLevelStates().front()->getId())};
    }
}

void InterpreterDocument::indexSubtree(IStateNode *node, const std::optional<State> &parent, int &order) {
    if (node == nullptr) {
        return;
    }
    const std::string &id = node->getId();
    if (node->getType() == Type::HISTORY) {
        HistoryEntry history;
        history.node = node;
        history.parent = parent.value_or(std::string{});
        histories_[id] = std::move(history);
        return;
    }
    if (!isStateElement(node->getType())) {
        return;
    }

    StateEntry entry;
    entry.node = node;
    entry.parent = parent;
    entry.type = node->getType();
    entry.order = order++;
    for (const auto &child : node->getChildren()) {
        if (child && isStateElement(child->getType())) {
            entry.children.push_back(child->getId());
        }
    }
    states_[id] = std::move(entry);

    for (const auto &child : node->getChildren()) {
        indexSubtree(child.get(), id, order);
    }
}

std::vector<InterpreterDocument::Target> InterpreterDocument::defaultInitialOf(const StateEntry &entry) const {
    // §scxml-3.3: the `<initial>` element's transition, else the `initial`
    // attribute, else the first child state in document order — a state, so
    // never a `<history>` that happens to come first.
    const auto initialTransition = entry.node->getInitialTransition();
    if (initialTransition && !initialTransition->getTargets().empty()) {
        return targetsOf(initialTransition->getTargets());
    }
    const auto written = tokensOf(entry.node->getInitialState());
    if (!written.empty()) {
        return targetsOf(written);
    }
    if (!entry.children.empty()) {
        return {Target::onState(entry.children.front())};
    }
    return {};
}

std::optional<InterpreterDocument::State> InterpreterDocument::parentOf(const State &state) const {
    const auto it = states_.find(state);
    return it == states_.end() ? std::nullopt : it->second.parent;
}

bool InterpreterDocument::isCompound(const State &state) const {
    const auto it = states_.find(state);
    return it != states_.end() && it->second.type == Type::COMPOUND;
}

bool InterpreterDocument::isParallel(const State &state) const {
    const auto it = states_.find(state);
    return it != states_.end() && it->second.type == Type::PARALLEL;
}

bool InterpreterDocument::isFinal(const State &state) const {
    const auto it = states_.find(state);
    return it != states_.end() && it->second.type == Type::FINAL;
}

std::vector<InterpreterDocument::State> InterpreterDocument::childStates(const State &state) const {
    const auto it = states_.find(state);
    return it == states_.end() ? std::vector<State>{} : it->second.children;
}

std::vector<InterpreterDocument::Target> InterpreterDocument::initialTargets(const State &state) const {
    const auto it = states_.find(state);
    return it == states_.end() ? std::vector<Target>{} : it->second.initial;
}

InterpreterDocument::State InterpreterDocument::historyParent(const History &history) const {
    const auto it = histories_.find(history);
    return it == histories_.end() ? State{} : it->second.parent;
}

std::vector<InterpreterDocument::Target> InterpreterDocument::historyDefaultTargets(const History &history) const {
    const auto it = histories_.find(history);
    return it == histories_.end() ? std::vector<Target>{} : it->second.defaults;
}

int InterpreterDocument::documentOrder(const State &state) const {
    const auto it = states_.find(state);
    return it == states_.end() ? -1 : it->second.order;
}

bool InterpreterDocument::isHistory(const std::string &id) const {
    return histories_.find(id) != histories_.end();
}

std::vector<InterpreterDocument::Target> InterpreterDocument::targetsOf(const std::vector<std::string> &ids) const {
    std::vector<Target> targets;
    targets.reserve(ids.size());
    for (const auto &id : ids) {
        targets.push_back(isHistory(id) ? Target::onHistory(id) : Target::onState(id));
    }
    return targets;
}

const std::vector<InterpreterDocument::Target> &InterpreterDocument::documentInitialTargets() const {
    return documentInitial_;
}

IStateNode *InterpreterDocument::nodeOf(const std::string &id) const {
    if (const auto state = states_.find(id); state != states_.end()) {
        return state->second.node;
    }
    if (const auto history = histories_.find(id); history != histories_.end()) {
        return history->second.node;
    }
    return nullptr;
}

}  // namespace SCE
