#include "events/EventRaiserService.h"
#include "common/LogMacros.h"
#include "events/EventRaiserRegistry.h"
#include "runtime/IEventRaiser.h"
#include <stdexcept>

namespace SCE {

std::unique_ptr<EventRaiserService> EventRaiserService::instance_;
std::mutex EventRaiserService::initMutex_;

void EventRaiserService::initialize(std::shared_ptr<IEventRaiserRegistry> registry,
                                    std::shared_ptr<ISessionManager> sessionManager) {
    std::lock_guard<std::mutex> lock(initMutex_);

    if (!registry) {
        throw std::invalid_argument("EventRaiserService: registry cannot be null");
    }

    if (!sessionManager) {
        throw std::invalid_argument("EventRaiserService: sessionManager cannot be null");
    }

    instance_ = std::unique_ptr<EventRaiserService>(new EventRaiserService(registry, sessionManager));
    SCE_LOG_DEBUG("EventRaiserService: Initialized with dependency injection");
}

EventRaiserService &EventRaiserService::getInstance() {
    std::lock_guard<std::mutex> lock(initMutex_);

    if (!instance_) {
        throw std::runtime_error("EventRaiserService: Not initialized. Call initialize() first.");
    }

    return *instance_;
}

void EventRaiserService::reset() {
    std::lock_guard<std::mutex> lock(initMutex_);
    instance_.reset();
    SCE_LOG_DEBUG("EventRaiserService: Reset for testing");
}

bool EventRaiserService::isInitialized() {
    std::lock_guard<std::mutex> lock(initMutex_);
    return instance_ != nullptr;
}

EventRaiserService::EventRaiserService(std::shared_ptr<IEventRaiserRegistry> registry,
                                       std::shared_ptr<ISessionManager> sessionManager)
    : registry_(std::move(registry)), sessionManager_(std::move(sessionManager)) {
    SCE_LOG_DEBUG("EventRaiserService: Created with injected dependencies");
}

bool EventRaiserService::registerEventRaiser(const std::string &sessionId, std::shared_ptr<IEventRaiser> eventRaiser) {
    SCE_LOG_DEBUG("EventRaiserService: Registering EventRaiser for sessionId='{}', eventRaiser={}", sessionId,
              (eventRaiser ? "valid" : "null"));

    if (sessionId.empty()) {
        SCE_LOG_ERROR("EventRaiserService: Cannot register EventRaiser - session ID is empty");
        return false;
    }

    if (!eventRaiser) {
        SCE_LOG_ERROR("EventRaiserService: Cannot register EventRaiser - eventRaiser is null for session: {}", sessionId);
        return false;
    }

    // Check if session exists before registration
    bool sessionExists = sessionManager_->hasSession(sessionId);
    SCE_LOG_DEBUG("EventRaiserService: Session '{}' exists: {}", sessionId, sessionExists);

    if (!sessionExists) {
        SCE_LOG_DEBUG("EventRaiserService: Session '{}' does not exist yet, deferring EventRaiser registration", sessionId);
        return false;  // Not an error, just deferred
    }

    // Check if already registered to avoid duplicates
    bool alreadyRegistered = registry_->hasEventRaiser(sessionId);
    SCE_LOG_DEBUG("EventRaiserService: EventRaiser already registered for session '{}': {}", sessionId, alreadyRegistered);

    if (alreadyRegistered) {
        SCE_LOG_DEBUG("EventRaiserService: EventRaiser already registered for session: {}", sessionId);
        return true;  // Already registered, success
    }

    // Perform the registration
    bool success = registry_->registerEventRaiser(sessionId, eventRaiser);
    if (success) {
        SCE_LOG_DEBUG("EventRaiserService: Successfully registered EventRaiser for session: '{}', ready={}", sessionId,
                  eventRaiser->isReady());
    } else {
        SCE_LOG_ERROR("EventRaiserService: Failed to register EventRaiser for session: {}", sessionId);
    }

    return success;
}

std::shared_ptr<IEventRaiser> EventRaiserService::getEventRaiser(const std::string &sessionId) const {
    SCE_LOG_DEBUG("EventRaiserService: Looking for EventRaiser with sessionId='{}'", sessionId);

    if (!registry_) {
        SCE_LOG_ERROR("EventRaiserService: Cannot get EventRaiser - registry is null");
        return nullptr;
    }

    auto result = registry_->getEventRaiser(sessionId);
    SCE_LOG_DEBUG("EventRaiserService: EventRaiser lookup result - sessionId='{}', found={}, ready={}", sessionId,
              (result != nullptr), (result ? result->isReady() : false));

    return result;
}

bool EventRaiserService::unregisterEventRaiser(const std::string &sessionId) {
    if (!registry_) {
        SCE_LOG_ERROR("EventRaiserService: Cannot unregister EventRaiser - registry is null");
        return false;
    }

    bool success = registry_->unregisterEventRaiser(sessionId);
    if (success) {
        SCE_LOG_DEBUG("EventRaiserService: Successfully unregistered EventRaiser for session: {}", sessionId);
    } else {
        SCE_LOG_DEBUG("EventRaiserService: EventRaiser not found for unregistration - session: {}", sessionId);
    }

    return success;
}

std::shared_ptr<IEventRaiserRegistry> EventRaiserService::getRegistry() const {
    return registry_;
}

void EventRaiserService::clearAll() {
    if (!registry_) {
        SCE_LOG_WARN("EventRaiserService: Cannot clear - registry is null");
        return;
    }

    // Safe clearing that works with any IEventRaiserRegistry implementation
    // This method should be added to IEventRaiserRegistry interface
    auto concreteRegistry = std::dynamic_pointer_cast<EventRaiserRegistry>(registry_);
    if (concreteRegistry) {
        concreteRegistry->clear();
        SCE_LOG_DEBUG("EventRaiserService: Cleared all EventRaiser registrations");
    } else {
        SCE_LOG_WARN("EventRaiserService: Registry does not support clearing");
    }
}

}  // namespace SCE