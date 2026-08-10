// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "events/EventTargetFactoryImpl.h"
#include "common/SendHelper.h"
#include "core/LogMacros.h"
#include "events/EventRaiserService.h"
#include "events/InternalEventTarget.h"
#include "events/InvokeEventTarget.h"
#include "events/ParentEventTarget.h"
#include "runtime/IEventRaiser.h"
#include <algorithm>
#include <stdexcept>

#ifdef SCE_ENABLE_HTTP
#include "events/HttpEventTarget.h"
#endif

namespace SCE {

EventTargetFactoryImpl::EventTargetFactoryImpl(std::shared_ptr<IEventRaiser> eventRaiser,
                                               std::shared_ptr<IEventScheduler> scheduler)
    : eventRaiser_(std::move(eventRaiser)), scheduler_(std::move(scheduler)) {
    if (!eventRaiser_) {
        throw std::invalid_argument("EventTargetFactoryImpl requires a valid event raiser");
    }

    // Register internal target creator
    registerTargetType("internal",
                       [this](const std::string &targetUri) { return createInternalTarget(targetUri, ""); });

#ifdef SCE_ENABLE_HTTP
    // Register HTTP target creator (both native and WASM)
    // §scxml-C-2: BasicHTTP Event I/O Processor
    // Native: Uses cpp-httplib, WASM: Uses EmscriptenFetchClient
    registerTargetType("http", [](const std::string &targetUri) {
        SCE_LOG_DEBUG("EventTargetFactoryImpl: Creating HTTP target for URI: {}", targetUri);
        auto target = std::make_shared<HttpEventTarget>(targetUri);
        SCE_LOG_DEBUG("EventTargetFactoryImpl: HTTP target created successfully: {}", target->getDebugInfo());
        return target;
    });

    // Register HTTPS target creator (both native and WASM)
    registerTargetType("https", [](const std::string &targetUri) {
        SCE_LOG_DEBUG("EventTargetFactoryImpl: Creating HTTPS target for URI: {}", targetUri);
        auto target = std::make_shared<HttpEventTarget>(targetUri);
        SCE_LOG_DEBUG("EventTargetFactoryImpl: HTTPS target created successfully: {}", target->getDebugInfo());
        return target;
    });

#ifdef __EMSCRIPTEN__
    SCE_LOG_DEBUG("EventTargetFactoryImpl: Factory created with internal, HTTP, and HTTPS target support (WASM with "
                  "EmscriptenFetchClient)");
#else
    SCE_LOG_DEBUG("EventTargetFactoryImpl: Factory created with internal, HTTP, and HTTPS target support (Native with "
                  "cpp-httplib)");
#endif
#else
    SCE_LOG_DEBUG("EventTargetFactoryImpl: Factory created with internal target support (HTTP disabled)");
#endif
}

std::shared_ptr<IEventTarget> EventTargetFactoryImpl::createTarget(const std::string &targetUri,
                                                                   const std::string &sessionId) {
    if (targetUri.empty()) {
        // W3C SCXML compliance: Empty target means external queue (Test 189)
        SCE_LOG_DEBUG("EventTargetFactoryImpl: Empty target URI, creating external queue target");
        return createExternalTarget(sessionId);
    }

    // §scxml-C-1: Handle special internal target URI
    // ARCHITECTURE.md: Zero Duplication - use SendHelper (Single Source of Truth)
    if (SendHelper::isInternalTarget(targetUri)) {
        return createInternalTarget(targetUri, sessionId);
    }

    // Handle special parent target URI (#_parent)
    if (targetUri == "#_parent") {
        SCE_LOG_DEBUG("EventTargetFactoryImpl::createTarget() - Creating #_parent target");
        return createParentTarget(targetUri, sessionId);
    }

    // §scxml-C-1 (test 190, 350): #_scxml_<sessionid> addresses the session
    // with that id. The id is what decides the destination — routing every
    // such URI to the SENDING session made tests 190 and 350 pass (both name
    // the session they already are) while silently misdelivering every event
    // addressed to a peer, which is the case `_event.origin` exists for.
    // ARCHITECTURE.md Zero Duplication: uses SendHelper (Single Source of Truth).
    if (SendHelper::isSessionTarget(targetUri)) {
        const std::string targetSessionId = SendHelper::extractSessionId(targetUri);
        SCE_LOG_DEBUG("EventTargetFactoryImpl::createTarget() - session target '{}' → session '{}'", targetUri,
                      targetSessionId.empty() ? "<own external queue>" : targetSessionId);
        return createSessionTarget(targetSessionId, sessionId);
    }

    // §scxml-6.4 (test192): Handle child invoke target (#_<invokeid>)
    // ARCHITECTURE.md Zero Duplication: Uses SendHelper (Single Source of Truth)
    if (SendHelper::isChildInvokeTarget(targetUri)) {
        std::string invokeId = SendHelper::extractInvokeId(targetUri);
        SCE_LOG_DEBUG("EventTargetFactoryImpl::createTarget() - Creating invoke target for ID: {}", invokeId);
        return createInvokeTarget(invokeId, sessionId);
    }

    // Extract scheme and find appropriate creator
    std::string scheme = extractScheme(targetUri);

    auto creatorIt = targetCreators_.find(scheme);
    if (creatorIt != targetCreators_.end()) {
        SCE_LOG_DEBUG("EventTargetFactoryImpl: Creating '{}' target for URI: {}", scheme, targetUri);

        try {
            auto target = creatorIt->second(targetUri);
            if (!target) {
                SCE_LOG_ERROR("EventTargetFactoryImpl: Target creator returned null for URI: {}", targetUri);
                return nullptr;
            }

            // Validate the created target
            auto errors = target->validate();
            if (!errors.empty()) {
                SCE_LOG_ERROR("EventTargetFactoryImpl: Target validation failed for URI '{}': {}", targetUri,
                              errors.front());
                return nullptr;
            }

            return target;

        } catch (const std::exception &e) {
            SCE_LOG_ERROR("EventTargetFactoryImpl: Error creating target for URI '{}': {}", targetUri, e.what());
            return nullptr;
        }
    }

    SCE_LOG_WARN("EventTargetFactoryImpl: No creator found for scheme '{}' in URI: {}", scheme, targetUri);
    return nullptr;
}

std::vector<std::string> EventTargetFactoryImpl::getSupportedSchemes() const {
    std::vector<std::string> schemes;
    schemes.reserve(targetCreators_.size() + 1);  // +1 for internal

    schemes.push_back("internal");  // Always supported

    for (const auto &pair : targetCreators_) {
        if (pair.first != "internal") {  // Avoid duplicating internal
            schemes.push_back(pair.first);
        }
    }

    return schemes;
}

void EventTargetFactoryImpl::registerTargetType(
    const std::string &scheme, std::function<std::shared_ptr<IEventTarget>(const std::string &)> creator) {
    if (scheme.empty()) {
        throw std::invalid_argument("Target scheme cannot be empty");
    }

    if (!creator) {
        throw std::invalid_argument("Target creator cannot be null");
    }

    SCE_LOG_DEBUG("EventTargetFactoryImpl: Registering target type for scheme: {}", scheme);
    targetCreators_[scheme] = creator;
}

bool EventTargetFactoryImpl::isSchemeSupported(const std::string &scheme) const {
    if (scheme.empty()) {
        return false;
    }

    if (scheme == "internal") {
        return true;  // Internal scheme always supported
    }

    return targetCreators_.find(scheme) != targetCreators_.end();
}

void EventTargetFactoryImpl::unregisterTargetCreator(const std::string &scheme) {
    if (scheme == "internal") {
        SCE_LOG_WARN("EventTargetFactoryImpl: Cannot unregister internal target creator");
        return;
    }

    auto removed = targetCreators_.erase(scheme);
    if (removed > 0) {
        SCE_LOG_DEBUG("EventTargetFactoryImpl: Unregistered target creator for scheme: {}", scheme);
    } else {
        SCE_LOG_DEBUG("EventTargetFactoryImpl: No target creator found for scheme: {}", scheme);
    }
}

std::string EventTargetFactoryImpl::extractScheme(const std::string &targetUri) const {
    if (targetUri.empty()) {
        return "internal";
    }

    // Handle special internal URI
    if (targetUri == "#_internal") {
        return "internal";
    }

    // Find scheme separator
    size_t colonPos = targetUri.find(':');
    if (colonPos == std::string::npos) {
        // No scheme specified, assume internal
        return "internal";
    }

    std::string scheme = targetUri.substr(0, colonPos);

    // Convert to lowercase for case-insensitive matching
    std::transform(scheme.begin(), scheme.end(), scheme.begin(), ::tolower);

    return scheme;
}

std::shared_ptr<IEventTarget> EventTargetFactoryImpl::createInternalTarget(const std::string &targetUri,
                                                                           const std::string &sessionId) {
    try {
        // Use session-specific EventRaiser if sessionId is provided
        std::shared_ptr<IEventRaiser> targetEventRaiser = eventRaiser_;  // Default fallback

        if (!sessionId.empty()) {
            SCE_LOG_DEBUG("EventTargetFactoryImpl: Looking up EventRaiser for sessionId='{}'", sessionId);

            auto sessionEventRaiser = EventRaiserService::getInstance().getEventRaiser(sessionId);
            if (sessionEventRaiser) {
                targetEventRaiser = sessionEventRaiser;
                SCE_LOG_DEBUG("EventTargetFactoryImpl: Found session-specific EventRaiser for session: '{}', ready={}",
                              sessionId, sessionEventRaiser->isReady());
            } else {
                SCE_LOG_DEBUG("EventTargetFactoryImpl: Session EventRaiser not found for session: '{}', using default",
                              sessionId);
            }
        }

        // §scxml-5.10: Pass sessionId for _event.origin (test 336)
        auto target =
            std::make_shared<InternalEventTarget>(targetEventRaiser, false, sessionId);  // Internal queue priority

        SCE_LOG_DEBUG("EventTargetFactoryImpl: Created internal target for URI: {} with session: {}", targetUri,
                      sessionId);
        return target;

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("EventTargetFactoryImpl: Error creating internal target: {}", e.what());
        return nullptr;
    }
}

std::shared_ptr<IEventTarget> EventTargetFactoryImpl::createExternalTarget(const std::string &sessionId) {
    try {
        // Use session-specific EventRaiser if sessionId is provided
        std::shared_ptr<IEventRaiser> targetEventRaiser = eventRaiser_;  // Default fallback

        if (!sessionId.empty()) {
            auto sessionEventRaiser = EventRaiserService::getInstance().getEventRaiser(sessionId);
            if (sessionEventRaiser) {
                targetEventRaiser = sessionEventRaiser;
                SCE_LOG_DEBUG(
                    "EventTargetFactoryImpl: Using session-specific EventRaiser for EXTERNAL target, session: {}",
                    sessionId);
            } else {
                SCE_LOG_DEBUG("EventTargetFactoryImpl: Session EventRaiser not found for EXTERNAL target, session: {}, "
                              "using default",
                              sessionId);
            }
        }

        // W3C SCXML compliance: External target uses EXTERNAL priority for proper queue ordering
        // §scxml-5.10: Pass sessionId for _event.origin (test 336)
        auto target =
            std::make_shared<InternalEventTarget>(targetEventRaiser, true, sessionId);  // External queue priority

        SCE_LOG_DEBUG("EventTargetFactoryImpl: Created external target for W3C SCXML compliance with session: {}",
                      sessionId);
        return target;

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("EventTargetFactoryImpl: Error creating external target: {}", e.what());
        return nullptr;
    }
}

std::shared_ptr<SCE::IEventTarget> SCE::EventTargetFactoryImpl::createParentTarget(const std::string &targetUri,
                                                                                   const std::string &sessionId) {
    try {
        // §scxml-6.4: Use provided sessionId for parent-child relationship tracking
        // Session ID should always be provided during invoke creation
        if (sessionId.empty()) {
            SCE_LOG_ERROR("EventTargetFactoryImpl: Empty sessionId for parent target creation - cannot route events");
            return nullptr;
        }

        std::string childSessionId = sessionId;

        // Create parent target with child session ID for proper event routing
        auto target = std::make_shared<SCE::ParentEventTarget>(childSessionId, eventRaiser_, scheduler_);

        SCE_LOG_DEBUG("EventTargetFactoryImpl: Created parent target for URI: {} with child session: {}", targetUri,
                      childSessionId);
        return target;

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("EventTargetFactoryImpl: Error creating parent target: {}", e.what());
        return nullptr;
    }
}

std::shared_ptr<SCE::IEventTarget>
SCE::EventTargetFactoryImpl::createSessionTarget(const std::string &targetSessionId,
                                                 const std::string &originSessionId) {
    // §scxml-C-1: a URI naming no session ("#_scxml_") is the sending
    // session's own external queue — what W3C test 190 asserts. That case is
    // exactly createExternalTarget, where lookup and origin are the same
    // session, so it delegates rather than restating the construction.
    if (targetSessionId.empty()) {
        return createExternalTarget(originSessionId);
    }

    // The two session ids play DIFFERENT roles and must not be conflated:
    // the addressee decides which queue receives the event, the sender is
    // what `_event.origin` reports to the receiver (§scxml-5.10, test 336).
    // They coincide only when a session addresses itself, which is why one
    // parameter served both for as long as that was the only case tested.
    auto targetEventRaiser = EventRaiserService::getInstance().getEventRaiser(targetSessionId);
    if (!targetEventRaiser) {
        // No silent fallback. Routing an unreachable addressee to the
        // sender's own queue reports success at every layer above while the
        // addressee waits forever; returning null makes the dispatcher answer
        // TARGET_NOT_FOUND, which is a fact the caller can act on.
        SCE_LOG_ERROR("EventTargetFactoryImpl: No session registered for target session '{}' (from session '{}')",
                      targetSessionId, originSessionId);
        return nullptr;
    }

    // External priority: §scxml-C-1 delivers a session-addressed event to the
    // addressee's EXTERNAL queue, the same queue an empty target uses.
    auto target = std::make_shared<InternalEventTarget>(targetEventRaiser, true, originSessionId);
    SCE_LOG_DEBUG("EventTargetFactoryImpl: Created session target for '{}' with origin '{}'", targetSessionId,
                  originSessionId);
    return target;
}

std::shared_ptr<SCE::IEventTarget> SCE::EventTargetFactoryImpl::createInvokeTarget(const std::string &invokeId,
                                                                                   const std::string &sessionId) {
    try {
        // Create invoke event target for routing to child session
        auto target = std::make_shared<SCE::InvokeEventTarget>(invokeId, sessionId);

        SCE_LOG_DEBUG("EventTargetFactoryImpl: Created invoke target for ID '{}' from session '{}'", invokeId,
                      sessionId);
        return target;

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("EventTargetFactoryImpl: Error creating invoke target for ID '{}': {}", invokeId, e.what());
        return nullptr;
    }
}

}  // namespace SCE
