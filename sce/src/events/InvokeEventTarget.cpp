// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "events/InvokeEventTarget.h"
#include "common/EventDataHelper.h"
#include "runtime/JsonUtils.h"
#include "core/LogMacros.h"
#include "common/SCXMLConstants.h"
#include "events/EventRaiserService.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/IEventRaiser.h"
#include "scripting/SessionRegistry.h"
#include <sstream>

namespace SCE {

InvokeEventTarget::InvokeEventTarget(const std::string &invokeId, const std::string &parentSessionId)
    : invokeId_(invokeId), parentSessionId_(parentSessionId) {
    if (invokeId_.empty()) {
        throw std::invalid_argument("InvokeEventTarget: Invoke ID cannot be empty");
    }

    if (parentSessionId_.empty()) {
        throw std::invalid_argument("InvokeEventTarget: Parent session ID cannot be empty");
    }

    SCE_LOG_DEBUG("InvokeEventTarget: Created for invoke ID '{}' from parent session '{}'", invokeId_, parentSessionId_);
}

std::future<SendResult> InvokeEventTarget::send(const EventDescriptor &event) {
    SCE_LOG_DEBUG("InvokeEventTarget::send() - ENTRY: event='{}', target='{}', invokeId='{}'", event.eventName,
              event.target, invokeId_);

    std::promise<SendResult> resultPromise;
    auto resultFuture = resultPromise.get_future();

    try {
        // [EVENT ROUTING] Step 1: Find child session ID using JSEngine invoke mapping
        SCE_LOG_INFO("[EVENT ROUTING] InvokeEventTarget looking up child session: parent='{}', invokeId='{}'",
                 parentSessionId_, invokeId_);
        std::string childSessionId = SessionRegistry::instance().getInvokeSessionId(parentSessionId_, invokeId_);
        if (childSessionId.empty()) {
            SCE_LOG_ERROR("[EVENT ROUTING] ❌ FAILED: No child session found for invoke ID '{}' in parent '{}'", invokeId_,
                      parentSessionId_);
            resultPromise.set_value(SendResult::error("No child session found for invoke ID: " + invokeId_,
                                                      SendResult::ErrorType::TARGET_NOT_FOUND));
            return resultFuture;
        }

        SCE_LOG_INFO("[EVENT ROUTING] ✅ Found child session '{}' for invoke ID '{}'", childSessionId, invokeId_);

        // [EVENT ROUTING] Step 2: Get EventRaiser for child session from centralized service
        SCE_LOG_INFO("[EVENT ROUTING] Looking up EventRaiser for child session '{}'", childSessionId);
        auto eventRaiser = EventRaiserService::getInstance().getEventRaiser(childSessionId);
        if (!eventRaiser) {
            SCE_LOG_ERROR("[EVENT ROUTING] ❌ FAILED: No EventRaiser found for child session '{}'", childSessionId);
            resultPromise.set_value(SendResult::error("No EventRaiser found for child session: " + childSessionId,
                                                      SendResult::ErrorType::TARGET_NOT_FOUND));
            return resultFuture;
        }

        SCE_LOG_INFO("[EVENT ROUTING] ✅ Found EventRaiser for child session '{}'", childSessionId);

        SCE_LOG_DEBUG("InvokeEventTarget: Routing event '{}' to child session '{}' via invoke ID '{}'", event.eventName,
                  childSessionId, invokeId_);

        // Prepare event data
        std::string eventName = event.eventName;
        std::string eventData = event.data;

        // W3C SCXML: Format params as JSON object to match ECMAScript data model
        // This enables _event.data.paramName access (Test 233, 178 compliance)
        if (!event.params.empty()) {
            json eventDataJson = json::object();

            // Add all params to the JSON object (W3C SCXML: Support duplicate param names - Test 178)
            for (const auto &param : event.params) {
                if (param.second.size() == 1) {
                    // Single value: store as string
                    eventDataJson[param.first] = param.second[0];
                } else if (param.second.size() > 1) {
                    // Multiple values: store as array (duplicate param names)
                    eventDataJson[param.first] = param.second;
                }
            }

            eventData = JsonUtils::toCompactString(eventDataJson);
        }

        // W3C SCXML 5.10: Raise event with origin tracking and origintype (test 253)
        // Origin is parent session, origintype is SCXML processor
        std::string originType = Constants::SCXML_EVENT_PROCESSOR_TYPE;
        SCE_LOG_DEBUG("InvokeEventTarget::send() - Calling eventRaiser->raiseEvent('{}', '{}', origin: '{}', invokeId: "
                  "'{}', originType: '{}')",
                  eventName, eventData, parentSessionId_, invokeId_, originType);
        // Build typed event data from typedParams if available (engine-agnostic pipeline)
        std::optional<ScriptValue> typedData;
        if (!event.typedParams.empty()) {
            typedData = EventDataHelper::buildScriptValueFromParams(event.typedParams);
        }

        bool raiseResult = false;
        auto *eventRaiserImpl = dynamic_cast<EventRaiserImpl *>(eventRaiser.get());
        if (eventRaiserImpl && typedData.has_value()) {
            raiseResult = eventRaiserImpl->raiseEventWithPriority(
                eventName, eventData, EventRaiserImpl::EventPriority::EXTERNAL,
                parentSessionId_, "", invokeId_, originType, 0, std::move(typedData));
        } else {
            raiseResult = eventRaiser->raiseEvent(eventName, eventData, parentSessionId_, invokeId_, originType);
        }
        SCE_LOG_DEBUG("InvokeEventTarget::send() - eventRaiser->raiseEvent() returned: {}", raiseResult);

        if (!raiseResult) {
            SCE_LOG_WARN("InvokeEventTarget: Failed to raise event '{}' in child session '{}'", eventName, childSessionId);
            resultPromise.set_value(
                SendResult::error("Failed to raise event in child session", SendResult::ErrorType::INTERNAL_ERROR));
        } else {
            SCE_LOG_DEBUG("InvokeEventTarget: Successfully routed event '{}' to child session '{}'", eventName,
                      childSessionId);
            resultPromise.set_value(SendResult::success(event.sendId));
        }

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("InvokeEventTarget: Error sending event to invoke: {}", e.what());
        resultPromise.set_value(SendResult::error("Failed to send event to invoke: " + std::string(e.what()),
                                                  SendResult::ErrorType::INTERNAL_ERROR));
    }

    return resultFuture;
}

std::string InvokeEventTarget::getTargetType() const {
    return "invoke";
}

bool InvokeEventTarget::canHandle(const std::string &targetUri) const {
    // Check if target matches #_<invokeId> pattern
    if (targetUri.length() > 2 && targetUri.substr(0, 2) == "#_") {
        std::string candidateInvokeId = targetUri.substr(2);
        return candidateInvokeId == invokeId_;
    }
    return false;
}

std::vector<std::string> InvokeEventTarget::validate() const {
    std::vector<std::string> errors;

    if (invokeId_.empty()) {
        errors.push_back("Invoke ID cannot be empty");
    }

    if (parentSessionId_.empty()) {
        errors.push_back("Parent session ID cannot be empty");
    }

    // Check if child session exists
    std::string childSessionId = SessionRegistry::instance().getInvokeSessionId(parentSessionId_, invokeId_);
    if (childSessionId.empty()) {
        errors.push_back("No child session found for invoke ID: " + invokeId_);
    } else {
        // Check if EventRaiser exists for child session
        auto eventRaiser = EventRaiserService::getInstance().getEventRaiser(childSessionId);
        if (!eventRaiser) {
            errors.push_back("No EventRaiser found for child session: " + childSessionId);
        }
    }

    return errors;
}

std::string InvokeEventTarget::getDebugInfo() const {
    std::string childSessionId = SessionRegistry::instance().getInvokeSessionId(parentSessionId_, invokeId_);
    return "invoke target (invoke: " + invokeId_ + ", parent: " + parentSessionId_ + ", child: " + childSessionId + ")";
}

}  // namespace SCE