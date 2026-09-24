// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "factory/NodeFactory.h"
#include "actions/AssignAction.h"
#include "actions/CancelAction.h"
#include "actions/IfAction.h"
#include "actions/LogAction.h"
#include "actions/RaiseAction.h"
#include "actions/ScriptAction.h"
#include "actions/SendAction.h"
#include "core/LogMacros.h"

std::shared_ptr<SCE::IStateNode> SCE::NodeFactory::createStateNode(const std::string &id, const Type type) {
    SCE_LOG_DEBUG("Creating state node: {}", id);

    // One node type for every kind of state. §scxml-3.4's regions are a
    // `<parallel>`'s child states, which the model already holds as children;
    // what a region does when the machine runs is Appendix D's, and the shared
    // microstep reads it off the type and the children alone.
    return std::make_shared<SCE::StateNode>(id, type);
}

std::shared_ptr<SCE::ITransitionNode> SCE::NodeFactory::createTransitionNode(const std::string &event,
                                                                             const std::string &target) {
    SCE_LOG_DEBUG("Creating transition node: {} -> {}", (event.empty() ? "<no event>" : event), target);
    return std::make_shared<SCE::TransitionNode>(event, target);
}

std::shared_ptr<SCE::IGuardNode> SCE::NodeFactory::createGuardNode(const std::string &id, const std::string &target) {
    SCE_LOG_DEBUG("Creating guard node: {} -> {}", id, target);
    return std::make_shared<SCE::GuardNode>(id, target);
}

std::shared_ptr<SCE::IActionNode> SCE::NodeFactory::createActionNode(const std::string &name) {
    SCE_LOG_DEBUG("Creating action node: {}", name);

    // Create specific action types based on SCXML element names
    if (name == "script") {
        return std::make_shared<SCE::ScriptAction>("");
    } else if (name == "assign") {
        return std::make_shared<SCE::AssignAction>("", "");
    } else if (name == "log") {
        return std::make_shared<SCE::LogAction>("");
    } else if (name == "raise") {
        return std::make_shared<SCE::RaiseAction>("");
    } else if (name == "if") {
        return std::make_shared<SCE::IfAction>("");
    } else if (name == "send") {
        return std::make_shared<SCE::SendAction>("");
    } else if (name == "cancel") {
        return std::make_shared<SCE::CancelAction>("");
    } else {
        // Default to ScriptAction for unknown types
        SCE_LOG_WARN("Unknown action type: {}, defaulting to ScriptAction", name);
        return std::make_shared<SCE::ScriptAction>("");
    }
}

std::shared_ptr<SCE::IDataModelItem> SCE::NodeFactory::createDataModelItem(const std::string &id,
                                                                           const std::string &expr) {
    SCE_LOG_DEBUG("Creating data model item: {}", id);
    return std::make_shared<SCE::DataModelItem>(id, expr);
}

std::shared_ptr<SCE::IInvokeNode> SCE::NodeFactory::createInvokeNode(const std::string &id) {
    SCE_LOG_DEBUG("Creating invoke node: {}", id);
    return std::make_shared<SCE::InvokeNode>(id);
}
