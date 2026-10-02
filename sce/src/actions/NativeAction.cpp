// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "actions/NativeAction.h"
#include "runtime/IActionExecutor.h"
#include "runtime/IExecutionContext.h"

namespace SCE {

NativeAction::NativeAction(const std::string &operation, const std::string &id)
    : BaseAction(id), operation_(operation) {}

bool NativeAction::execute(IExecutionContext &context) {
    if (!context.isValid()) {
        return false;
    }

    try {
        return context.getActionExecutor().executeNativeAction(*this);
    } catch (const std::exception &) {
        return false;
    }
}

std::string NativeAction::getActionType() const {
    return "native_action";
}

std::shared_ptr<IActionNode> NativeAction::clone() const {
    auto cloned = std::make_shared<NativeAction>(operation_, getId());
    for (const auto &argument : arguments_) {
        cloned->addArgument(argument.expr, argument.name);
    }
    return cloned;
}

const std::string &NativeAction::getOperation() const {
    return operation_;
}

void NativeAction::addArgument(const std::string &expr, const std::string &name) {
    arguments_.push_back(Argument{name, expr});
}

const std::vector<NativeAction::Argument> &NativeAction::getArguments() const {
    return arguments_;
}

std::vector<std::string> NativeAction::validateSpecific() const {
    std::vector<std::string> errors;
    if (isEmptyString(operation_)) {
        errors.push_back("<sce:action> requires a name: the host operation it performs");
    }
    for (const auto &argument : arguments_) {
        if (isEmptyString(argument.expr)) {
            errors.push_back("<sce:arg> requires an expr: the value the host operation takes");
        }
    }
    return errors;
}

std::string NativeAction::getSpecificDescription() const {
    std::string desc = "native_action name=\"" + operation_ + "\"";
    for (const auto &argument : arguments_) {
        desc += " arg=\"" + argument.expr + "\"";
    }
    return desc;
}

}  // namespace SCE
