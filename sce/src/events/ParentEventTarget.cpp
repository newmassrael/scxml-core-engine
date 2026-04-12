// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "events/ParentEventTarget.h"
#include "common/EventDataHelper.h"
#include "runtime/JsonUtils.h"
#include "core/LogMacros.h"
#include "common/SCXMLConstants.h"
#include "events/EventRaiserService.h"
#include "events/IEventDispatcher.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/IEventRaiser.h"
#include "scripting/SessionRegistry.h"
#include <sstream>
#include <thread>

namespace SCE {

ParentEventTarget::ParentEventTarget(const std::string &childSessionId, std::shared_ptr<IEventRaiser> eventRaiser,
                                     std::shared_ptr<IEventScheduler> scheduler)
    : childSessionId_(childSessionId), eventRaiser_(std::move(eventRaiser)), scheduler_(std::move(scheduler)) {
    if (childSessionId_.empty()) {
        throw std::invalid_argument("ParentEventTarget requires a valid child session ID");
    }

    if (!eventRaiser_) {
        throw std::invalid_argument("ParentEventTarget requires a valid event raiser");
    }

    SCE_LOG_DEBUG("ParentEventTarget: Created for child session: {}", childSessionId_);
}

std::future<SendResult> ParentEventTarget::send(const EventDescriptor &event) {
    SCE_LOG_DEBUG("ParentEventTarget::send() - ENTRY: event='{}', target='{}', sessionId='{}', delay={}ms", event.eventName,
              event.target, event.sessionId, event.delay.count());

    // Check if this is a delayed event and scheduler is available
    if (event.delay.count() > 0 && scheduler_) {
        SCE_LOG_DEBUG("ParentEventTarget: Scheduling delayed parent event '{}' for {}ms", event.eventName,
                  event.delay.count());

        // Create a copy of this target for delayed execution
        auto sharedThis = std::make_shared<ParentEventTarget>(childSessionId_, eventRaiser_, scheduler_);

        // Schedule the event for delayed execution
        auto sendIdFuture = scheduler_->scheduleEvent(event, event.delay, sharedThis, event.sendId, event.sessionId);

        // W3C SCXML 6.2: Convert sendId future to SendResult future synchronously (WASM memory leak prevention)
        // Process result synchronously to ensure thread cleanup
        std::promise<SendResult> resultPromise;
        try {
            std::string assignedSendId = sendIdFuture.get();
            resultPromise.set_value(SendResult::success(assignedSendId));
        } catch (const std::exception &e) {
            resultPromise.set_value(
                SendResult::error("Failed to schedule delayed parent event: " + std::string(e.what()),
                                  SendResult::ErrorType::INTERNAL_ERROR));
        }
        return resultPromise.get_future();
    } else {
        // Execute immediately (no delay or no scheduler available)
        return sendImmediately(event);
    }
}

std::future<SendResult> ParentEventTarget::sendImmediately(const EventDescriptor &event) {
    SCE_LOG_DEBUG("ParentEventTarget::sendImmediately() - ENTRY: event='{}', target='{}', sessionId='{}'", event.eventName,
              event.target, event.sessionId);

    std::promise<SendResult> resultPromise;
    auto resultFuture = resultPromise.get_future();

    try {
        // Use session ID from event descriptor as child session ID
        std::string actualChildSessionId = event.sessionId.empty() ? childSessionId_ : event.sessionId;
        SCE_LOG_DEBUG(
            "ParentEventTarget::sendImmediately() - Child session: '{}' (from event: '{}', from constructor: '{}')",
            actualChildSessionId, event.sessionId, childSessionId_);

        // Find parent session ID: use _parentSessionId param if provided (for done.invoke), otherwise lookup via
        // JSEngine
        std::string parentSessionId;
        auto it = event.params.find("_parentSessionId");
        if (it != event.params.end() && !it->second.empty()) {
            parentSessionId = it->second[0];  // W3C SCXML: Get first value from vector
            SCE_LOG_DEBUG("ParentEventTarget: Using parent session from params: '{}'", parentSessionId);
        } else {
            parentSessionId = findParentSessionId(actualChildSessionId);
            SCE_LOG_DEBUG("ParentEventTarget: Found parent session via SessionRegistry: '{}'", parentSessionId);
        }

        if (parentSessionId.empty()) {
            // W3C SCXML: This is normal during cleanup when parent relationship is already removed
            // Child's onexit handlers may try to send events after invoke is cancelled
            SCE_LOG_DEBUG("ParentEventTarget: No parent session found for child: {} (likely during cleanup)",
                      actualChildSessionId);
            resultPromise.set_value(SendResult::error("No parent session found for child: " + actualChildSessionId,
                                                      SendResult::ErrorType::TARGET_NOT_FOUND));
            return resultFuture;
        }

        SCE_LOG_DEBUG("ParentEventTarget: Routing event '{}' from child '{}' to parent '{}'", event.eventName,
                  actualChildSessionId, parentSessionId);

        // Get parent session's EventRaiser from centralized service
        SCE_LOG_DEBUG("ParentEventTarget: Looking up EventRaiser for parent session: {}", parentSessionId);
        auto parentEventRaiser = EventRaiserService::getInstance().getEventRaiser(parentSessionId);
        SCE_LOG_DEBUG("ParentEventTarget: EventRaiser lookup result: {}", parentEventRaiser ? "FOUND" : "NOT FOUND");
        if (!parentEventRaiser) {
            SCE_LOG_ERROR("ParentEventTarget: No EventRaiser found for parent session: {}", parentSessionId);
            resultPromise.set_value(SendResult::error("No EventRaiser found for parent session: " + parentSessionId,
                                                      SendResult::ErrorType::TARGET_NOT_FOUND));
            return resultFuture;
        }

        // Create event with parent session as target
        std::string eventName = event.eventName;
        std::string eventData = event.data;

        // W3C SCXML: Format params as JSON object to match ECMAScript data model
        // This enables _event.data.paramName access in finalize handlers (Test 233, 178)
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

        // W3C SCXML 5.10 test 338: Get invoke ID for this child session
        std::string invokeId = SessionRegistry::instance().getInvokeIdForChildSession(actualChildSessionId);

        // Raise event in parent session using parent's EventRaiser with origin and invoke tracking
        // W3C SCXML 6.4: Pass child session ID as originSessionId for finalize support
        // W3C SCXML 5.10: Pass invoke ID for event.invokeid field (test 338)
        // W3C SCXML 5.10: Pass origintype as SCXML processor type (test 253, 331, 352, 372)
        // ARCHITECTURE.md: Use SCXMLConstants for Single Source of Truth
        std::string originType = SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE;
        SCE_LOG_DEBUG("ParentEventTarget::sendImmediately() - Calling parent EventRaiser->raiseEvent('{}', '{}', origin: "
                  "'{}', invokeId: '{}', originType: '{}')",
                  eventName, eventData, actualChildSessionId, invokeId, originType);
        // Build typed event data from typedParams if available (engine-agnostic pipeline)
        std::optional<ScriptValue> typedData;
        if (!event.typedParams.empty()) {
            typedData = EventDataHelper::buildScriptValueFromParams(event.typedParams);
        }

        bool raiseResult = false;
        // Try to use raiseEventWithPriority for typed data support
        auto *eventRaiserImpl = dynamic_cast<EventRaiserImpl *>(parentEventRaiser.get());
        if (eventRaiserImpl && typedData.has_value()) {
            raiseResult = eventRaiserImpl->raiseEventWithPriority(
                eventName, eventData, EventRaiserImpl::EventPriority::EXTERNAL,
                actualChildSessionId, "", invokeId, originType, 0, std::move(typedData));
        } else {
            raiseResult =
                parentEventRaiser->raiseEvent(eventName, eventData, actualChildSessionId, invokeId, originType);
        }
        SCE_LOG_DEBUG("ParentEventTarget::sendImmediately() - parent EventRaiser->raiseEvent() returned: {}", raiseResult);

        SCE_LOG_DEBUG("ParentEventTarget: Successfully routed event '{}' to parent session '{}'", eventName,
                  parentSessionId);

        resultPromise.set_value(SendResult::success(event.sendId));

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("ParentEventTarget: Error sending event to parent: {}", e.what());
        resultPromise.set_value(SendResult::error("Failed to send event to parent: " + std::string(e.what()),
                                                  SendResult::ErrorType::INTERNAL_ERROR));
    }

    return resultFuture;
}

std::vector<std::string> ParentEventTarget::validate() const {
    std::vector<std::string> errors;

    if (childSessionId_.empty()) {
        errors.push_back("Child session ID cannot be empty");
    }

    if (!eventRaiser_) {
        errors.push_back("Event raiser cannot be null");
    }

    // Check if parent session exists
    std::string parentSessionId = findParentSessionId(childSessionId_);
    if (parentSessionId.empty()) {
        errors.push_back("No parent session found for child: " + childSessionId_);
    }

    return errors;
}

std::string ParentEventTarget::getTargetType() const {
    return "parent";
}

bool ParentEventTarget::canHandle(const std::string &targetUri) const {
    return targetUri == "#_parent";
}

std::string ParentEventTarget::getDebugInfo() const {
    std::string parentSessionId = findParentSessionId(childSessionId_);
    return "parent target (child: " + childSessionId_ + ", parent: " + parentSessionId + ")";
}

std::string ParentEventTarget::findParentSessionId(const std::string &childSessionId) const {
    // W3C SCXML 6.4: Use SessionRegistry for parent-child relationship lookup
    // Engine-agnostic: No dependency on specific script engine implementation
    std::string parentSessionId = SessionRegistry::instance().getParentSessionId(childSessionId);

    if (parentSessionId.empty()) {
        SCE_LOG_DEBUG("ParentEventTarget: No parent session found for child: {}", childSessionId);
    } else {
        SCE_LOG_DEBUG("ParentEventTarget: Found parent session '{}' for child '{}'", parentSessionId, childSessionId);
    }

    return parentSessionId;
}

}  // namespace SCE