// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "actions/RaiseAction.h"
#include "runtime/IActionExecutor.h"
#include "runtime/IExecutionContext.h"

namespace SCE {

RaiseAction::RaiseAction(const std::string &event, const std::string &id) : BaseAction(id), event_(event) {}

bool RaiseAction::execute(IExecutionContext &context) {
    // §scxml-4.2: <raise> raises an event in the current SCXML session. The event is
    // not seen until the enclosing block of executable content has finished and every
    // event already on the internal queue has been processed.
    if (!context.isValid()) {
        return false;
    }

    try {
        return context.getActionExecutor().executeRaiseAction(*this);
    } catch (const std::exception &) {
        return false;
    }
}

std::string RaiseAction::getActionType() const {
    return "raise";
}

std::shared_ptr<IActionNode> RaiseAction::clone() const {
    auto cloned = std::make_shared<RaiseAction>(getId());
    cloned->setEvent(event_);
    cloned->setData(data_);
    return cloned;
}

const std::string &RaiseAction::getEvent() const {
    return event_;
}

void RaiseAction::setEvent(const std::string &event) {
    event_ = event;
}

const std::string &RaiseAction::getData() const {
    return data_;
}

void RaiseAction::setData(const std::string &data) {
    data_ = data;
}

std::vector<std::string> RaiseAction::validateSpecific() const {
    std::vector<std::string> errors;

    if (isEmptyString(event_)) {
        errors.push_back("Raise action must have an event name");
    }

    // Data is optional, empty data is valid

    return errors;
}

std::string RaiseAction::getSpecificDescription() const {
    std::string desc = "raise event=\"" + event_ + "\"";
    if (!data_.empty()) {
        desc += " data=\"" + data_ + "\"";
    }
    return desc;
}

}  // namespace SCE