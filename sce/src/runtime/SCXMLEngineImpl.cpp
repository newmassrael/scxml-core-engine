// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#define SCXML_ENGINE_EXPORTS
#include "SCXMLEngineImpl.h"
#include "common/UniqueIdGenerator.h"
#include "core/LogMacros.h"
#include "runtime/ExecutionContextImpl.h"
#include "runtime/StateMachineFactory.h"
#include "scripting/ScriptEngineProvider.h"  // For default constructor engine resolution
#include "scripting/ScriptResultUtils.h"
// W3C SCXML event infrastructure for <send>, <invoke>, delayed events
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <random>
#include <sstream>

namespace SCE {

// === ExecutionResult Implementation ===

::std::string ExecutionResult::getValueAsString() const {
    return ::std::visit(
        [](const auto &v) -> ::std::string {
            using T = ::std::decay_t<decltype(v)>;
            if constexpr (::std::is_same_v<T, ::std::string>) {
                return v;
            } else if constexpr (::std::is_same_v<T, bool>) {
                return v ? "true" : "false";
            } else if constexpr (::std::is_same_v<T, int64_t>) {
                return ::std::to_string(v);
            } else if constexpr (::std::is_same_v<T, double>) {
                return ::std::to_string(v);
            } else {
                return "undefined";
            }
        },
        value);
}

// === Event Implementation ===

Event::Event(const ::std::string &name, const ::std::string &type) : name_(name), type_(type) {}

// === SCXMLEngineImpl Implementation ===

SCXMLEngineImpl::SCXMLEngineImpl()
    : scriptEngine_(ScriptEngineProvider::getScriptEngine()),
      sessionManager_(ScriptEngineProvider::getSessionManager()) {}

SCXMLEngineImpl::SCXMLEngineImpl(IScriptEngine &scriptEngine, ISessionManager &sessionManager)
    : scriptEngine_(scriptEngine), sessionManager_(sessionManager) {}

SCXMLEngineImpl::~SCXMLEngineImpl() {
    // Shut down event infrastructure to prevent orphaned timer threads
    // from firing after the engine is destroyed
    if (eventScheduler_) {
        eventScheduler_->shutdown(false);
    }
    if (eventRaiser_) {
        eventRaiser_->shutdown();
    }
    // Do NOT call shutdown() - script engine may be shared by all SCXMLEngine instances
    // Engine lifecycle is managed at process level, not per-instance
    initialized_ = false;
}

bool SCXMLEngineImpl::initialize() {
    SCE_LOG_DEBUG("SCXMLEngineImpl: Starting initialization...");
    if (initialized_) {
        SCE_LOG_DEBUG("SCXMLEngineImpl: Already initialized");
        return true;
    }

    // Script engine automatically initialized (RAII)
    SCE_LOG_DEBUG("SCXMLEngineImpl: Script engine ready");
    initialized_ = true;
    return true;
}

void SCXMLEngineImpl::shutdown() {
    if (initialized_) {
        scriptEngine_.shutdown();
        initialized_ = false;
    }
}

::std::string SCXMLEngineImpl::getEngineInfo() const {
    return scriptEngine_.getEngineInfo() + " (SCXML C++ API v1.0)";
}

bool SCXMLEngineImpl::createSession(const ::std::string &sessionId, const ::std::string &parentSessionId) {
    return scriptEngine_.createSession(sessionId, parentSessionId);
}

bool SCXMLEngineImpl::destroySession(const ::std::string &sessionId) {
    return scriptEngine_.destroySession(sessionId);
}

bool SCXMLEngineImpl::hasSession(const ::std::string &sessionId) const {
    return scriptEngine_.hasSession(sessionId);
}

::std::vector<SessionInfo> SCXMLEngineImpl::getActiveSessions() const {
    auto sessionIds = sessionManager_.getActiveSessions();
    ::std::vector<SessionInfo> result;
    result.reserve(sessionIds.size());

    for (const auto &id : sessionIds) {
        SessionInfo info;
        info.sessionId = id;
        info.isActive = true;
        result.push_back(::std::move(info));
    }

    return result;
}

::std::future<ExecutionResult> SCXMLEngineImpl::executeScript(const ::std::string &sessionId,
                                                              const ::std::string &script) {
    auto jsFuture = scriptEngine_.executeScript(sessionId, script);
    return ::std::async(::std::launch::deferred, [jsFuture = ::std::move(jsFuture), this]() mutable {
        auto jsResult = jsFuture.get();
        return convertResult(jsResult);
    });
}

::std::future<ExecutionResult> SCXMLEngineImpl::evaluateExpression(const ::std::string &sessionId,
                                                                   const ::std::string &expression) {
    auto jsFuture = scriptEngine_.evaluateExpression(sessionId, expression);
    return ::std::async(::std::launch::deferred, [jsFuture = ::std::move(jsFuture), this]() mutable {
        auto jsResult = jsFuture.get();
        return convertResult(jsResult);
    });
}

::std::future<ExecutionResult> SCXMLEngineImpl::setVariable(const ::std::string &sessionId, const ::std::string &name,
                                                            const ScriptValue &value) {
    // Convert public ScriptValue to internal ScriptValue (same type)
    auto jsFuture = scriptEngine_.setVariable(sessionId, name, value);
    return ::std::async(::std::launch::deferred, [jsFuture = ::std::move(jsFuture), this]() mutable {
        auto jsResult = jsFuture.get();
        return convertResult(jsResult);
    });
}

::std::future<ExecutionResult> SCXMLEngineImpl::getVariable(const ::std::string &sessionId, const ::std::string &name) {
    auto jsFuture = scriptEngine_.getVariable(sessionId, name);
    return ::std::async(::std::launch::deferred, [jsFuture = ::std::move(jsFuture), this]() mutable {
        auto jsResult = jsFuture.get();
        return convertResult(jsResult);
    });
}

::std::future<ExecutionResult> SCXMLEngineImpl::setCurrentEvent(const ::std::string &sessionId,
                                                                ::std::shared_ptr<Event> event) {
    return ::std::async(::std::launch::deferred, [this, sessionId, event]() -> ExecutionResult {
        auto jsResult = scriptEngine_.setCurrentEvent(sessionId, event).get();
        ExecutionResult result;
        result.success = jsResult.isSuccess();
        result.errorMessage = jsResult.isSuccess() ? "" : "Failed to set current event";
        return result;
    });
}

::std::future<ExecutionResult>
SCXMLEngineImpl::setupSystemVariables(const ::std::string &sessionId, const ::std::string &sessionName,
                                      const ::std::vector<IOProcessorDescriptor> &ioProcessors) {
    auto jsFuture = scriptEngine_.setupSystemVariables(sessionId, sessionName, ioProcessors);
    return ::std::async(::std::launch::deferred, [jsFuture = ::std::move(jsFuture), this]() mutable {
        auto jsResult = jsFuture.get();
        return convertResult(jsResult);
    });
}

size_t SCXMLEngineImpl::getMemoryUsage() const {
    return scriptEngine_.getMemoryUsage();
}

void SCXMLEngineImpl::collectGarbage() {
    scriptEngine_.collectGarbage();
}

ExecutionResult SCXMLEngineImpl::convertResult(const ScriptResult &jsResult) const {
    ExecutionResult result;
    result.success = jsResult.isSuccess();
    // Direct access to value through internal member (friend class access)
    result.value = jsResult.getInternalValue();
    result.errorMessage = jsResult.isSuccess() ? "" : "Execution failed";
    return result;
}

::std::shared_ptr<Event> SCXMLEngineImpl::convertEvent(::std::shared_ptr<Event> publicEvent) const {
    if (!publicEvent) {
        return nullptr;
    }

    auto internalEvent = ::std::make_shared<Event>(publicEvent->getName(), publicEvent->getType());
    internalEvent->setSendId(publicEvent->getSendId());
    internalEvent->setOrigin(publicEvent->getOrigin());
    internalEvent->setOriginType(publicEvent->getOriginType());
    internalEvent->setInvokeId(publicEvent->getInvokeId());
    if (publicEvent->hasData()) {
        internalEvent->setRawJsonData(publicEvent->getDataAsString());
    }

    return internalEvent;
}

// === Factory Functions ===

::std::unique_ptr<SCXMLEngine> createSCXMLEngine() {
    return ::std::make_unique<SCXMLEngineImpl>();
}

::std::string getSCXMLVersion() {
    return "1.0.0";
}

// === High-Level SCXML State Machine API Implementation ===

std::string SCXMLEngineImpl::generateSessionId() const {
    // REFACTOR: Use centralized UniqueIdGenerator instead of duplicate logic
    return UniqueIdGenerator::generateSessionId("scxml");
}

bool SCXMLEngineImpl::loadSCXMLFromString(const std::string &scxmlContent, const std::string &sessionId) {
    try {
        // Determine session ID
        std::string actualSessionId = sessionId.empty() ? generateSessionId() : sessionId;

        // Store as default session if none specified
        if (sessionId.empty()) {
            defaultSessionId_ = actualSessionId;
        }

        // Tear down previous event infrastructure if re-loading
        if (eventScheduler_) {
            eventScheduler_->shutdown(false);
            eventScheduler_.reset();
        }
        if (eventRaiser_) {
            eventRaiser_->shutdown();
            eventRaiser_.reset();
        }
        eventDispatcher_.reset();
        stateMachine_.reset();

        // W3C SCXML event infrastructure: EventRaiser, EventScheduler, EventDispatcher
        // Required for <send>, <invoke>, delayed events, and event routing
        eventRaiser_ = std::make_shared<EventRaiserImpl>();

        eventScheduler_ = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &sendId) -> bool {
                SCE_LOG_DEBUG("SCXMLEngine: Executing scheduled event '{}' (sendId: '{}')", event.eventName, sendId);
                auto future = target->send(event);
                try {
                    auto sendResult = future.get();
                    return sendResult.isSuccess;
                } catch (const std::exception &e) {
                    SCE_LOG_ERROR("SCXMLEngine: Failed to send scheduled event '{}': {}", event.eventName, e.what());
                    return false;
                }
            });

        // §scxml-6.2: Connect EventRaiser to EventScheduler for delayed event polling
        auto eventRaiserImpl = std::static_pointer_cast<EventRaiserImpl>(eventRaiser_);
        eventRaiserImpl->setScheduler(eventScheduler_);

        auto targetFactory = std::make_shared<EventTargetFactoryImpl>(eventRaiser_, eventScheduler_);
        eventDispatcher_ = std::make_shared<EventDispatcherImpl>(eventScheduler_, targetFactory);

        // Create StateMachine with full W3C event infrastructure
        auto result = StateMachineFactory::builder()
                          .withSCXML(scxmlContent)
                          .withAutoInitialize(false)
                          .withEventDispatcher(eventDispatcher_)
                          .withEventRaiser(eventRaiser_)
                          .build();
        if (!result.has_value()) {
            sessionErrors_[actualSessionId] = "Failed to create state machine: " + result.error;
            SCE_LOG_ERROR("SCXMLEngine: Failed to load SCXML content: {}", result.error);
            return false;
        }

        stateMachine_ = std::move(result.value);
        if (!stateMachine_) {
            sessionErrors_[actualSessionId] = "State machine creation returned null";
            SCE_LOG_ERROR("SCXMLEngine: State machine creation returned null");
            return false;
        }

        SCE_LOG_INFO("SCXMLEngine: SCXML content loaded successfully with session: {}", actualSessionId);
        return true;

    } catch (const std::exception &e) {
        std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;
        sessionErrors_[actualSessionId] = std::string("Load failed: ") + e.what();
        SCE_LOG_ERROR("SCXMLEngine: Exception during SCXML load: {}", e.what());
        return false;
    }
}

bool SCXMLEngineImpl::loadSCXMLFromFile(const std::string &scxmlFile, const std::string &sessionId) {
    try {
        // Read file content
        std::ifstream file(scxmlFile);
        if (!file.is_open()) {
            std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;
            sessionErrors_[actualSessionId] = "Cannot open SCXML file: " + scxmlFile;
            SCE_LOG_ERROR("SCXMLEngine: Cannot open SCXML file: {}", scxmlFile);
            return false;
        }

        std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
        if (!loadSCXMLFromString(content, sessionId)) {
            return false;
        }

        // W3C SCXML: Register file path for invoke relative path resolution
        if (stateMachine_) {
            auto absPath = std::filesystem::absolute(scxmlFile).string();
            stateMachine_->setSessionFilePath(absPath);
            SCE_LOG_DEBUG("SCXMLEngine: Registered session file path: {}", absPath);
        }

        return true;

    } catch (const std::exception &e) {
        std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;
        sessionErrors_[actualSessionId] = std::string("File load failed: ") + e.what();
        SCE_LOG_ERROR("SCXMLEngine: Exception during file load: {}", e.what());
        return false;
    }
}

bool SCXMLEngineImpl::startStateMachine(const std::string &sessionId) {
    std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;

    if (!stateMachine_) {
        sessionErrors_[actualSessionId] = "No state machine loaded";
        SCE_LOG_ERROR("SCXMLEngine: Cannot start - no state machine loaded");
        return false;
    }

    try {
        bool result = stateMachine_->start();
        if (!result) {
            sessionErrors_[actualSessionId] = "Failed to start state machine";
            SCE_LOG_ERROR("SCXMLEngine: Failed to start state machine");
            return false;
        }

        SCE_LOG_INFO("SCXMLEngine: State machine started successfully for session: {}", actualSessionId);
        return true;

    } catch (const std::exception &e) {
        sessionErrors_[actualSessionId] = std::string("Start failed: ") + e.what();
        SCE_LOG_ERROR("SCXMLEngine: Exception during start: {}", e.what());
        return false;
    }
}

void SCXMLEngineImpl::stopStateMachine(const std::string &sessionId) {
    if (stateMachine_) {
        try {
            stateMachine_->stop();
            std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;
            SCE_LOG_INFO("SCXMLEngine: State machine stopped for session: {}", actualSessionId);
        } catch (const std::exception &e) {
            SCE_LOG_WARN("SCXMLEngine: Exception during stop: {}", e.what());
        }
    }
}

bool SCXMLEngineImpl::sendEventSync(const std::string &eventName, const std::string &sessionId,
                                    const std::string &eventData) {
    std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;

    if (!stateMachine_) {
        sessionErrors_[actualSessionId] = "No state machine available";
        return false;
    }

    if (!stateMachine_->isRunning()) {
        sessionErrors_[actualSessionId] = "State machine is not running";
        return false;
    }

    try {
        auto result = stateMachine_->processEvent(eventName, eventData);
        if (!result.success) {
            sessionErrors_[actualSessionId] = "Event processing failed: " + result.errorMessage;
            SCE_LOG_WARN("SCXMLEngine: Event '{}' failed: {}", eventName, result.errorMessage);
            return false;
        }
        return true;

    } catch (const std::exception &e) {
        sessionErrors_[actualSessionId] = std::string("Event processing exception: ") + e.what();
        SCE_LOG_ERROR("SCXMLEngine: Event '{}' exception: {}", eventName, e.what());
        return false;
    }
}

bool SCXMLEngineImpl::raiseExternalEvent(const std::string &eventName, const std::string &sessionId,
                                         const std::string &eventData) {
    std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;

    if (!stateMachine_) {
        sessionErrors_[actualSessionId] = "No state machine available";
        return false;
    }

    if (!stateMachine_->isRunning()) {
        sessionErrors_[actualSessionId] = "State machine is not running";
        return false;
    }

    try {
        return stateMachine_->raiseExternalEvent(eventName, eventData);
    } catch (const std::exception &e) {
        sessionErrors_[actualSessionId] = std::string("External event exception: ") + e.what();
        SCE_LOG_ERROR("SCXMLEngine: External event '{}' exception: {}", eventName, e.what());
        return false;
    }
}

bool SCXMLEngineImpl::isStateMachineRunning([[maybe_unused]] const std::string &sessionId) const {
    return stateMachine_ && stateMachine_->isRunning();
}

std::string SCXMLEngineImpl::getCurrentStateSync([[maybe_unused]] const std::string &sessionId) const {
    if (!stateMachine_) {
        return "";
    }
    return stateMachine_->getCurrentState();
}

bool SCXMLEngineImpl::isInStateSync(const std::string &stateId, [[maybe_unused]] const std::string &sessionId) const {
    if (!stateMachine_) {
        return false;
    }
    return stateMachine_->isStateActive(stateId);
}

std::vector<std::string> SCXMLEngineImpl::getActiveStatesSync([[maybe_unused]] const std::string &sessionId) const {
    if (!stateMachine_) {
        return {};
    }
    return stateMachine_->getActiveStates();
}

// Private helper for all setVariableSync overloads (Zero Duplication)
bool SCXMLEngineImpl::setVariableSyncImpl(const std::string &name, const ScriptValue &value,
                                          const std::string &sessionId) {
    std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;

    if (!stateMachine_) {
        sessionErrors_[actualSessionId] = "No state machine available";
        return false;
    }

    try {
        const std::string &smSessionId = stateMachine_->getSessionId();
        auto future = scriptEngine_.setVariable(smSessionId, name, value);
        auto result = future.get();

        if (!ScriptResultUtils::isSuccess(result)) {
            sessionErrors_[actualSessionId] = "Failed to set variable: " + result.getErrorMessage();
            SCE_LOG_WARN("SCXMLEngine: Failed to set variable '{}': {}", name, result.getErrorMessage());
            return false;
        }
        return true;

    } catch (const std::exception &e) {
        sessionErrors_[actualSessionId] = std::string("Variable setting exception: ") + e.what();
        SCE_LOG_ERROR("SCXMLEngine: Variable '{}' exception: {}", name, e.what());
        return false;
    }
}

bool SCXMLEngineImpl::setVariableSync(const std::string &name, const std::string &value, const std::string &sessionId) {
    return setVariableSyncImpl(name, ScriptValue(value), sessionId);
}

bool SCXMLEngineImpl::setVariableSync(const std::string &name, bool value, const std::string &sessionId) {
    return setVariableSyncImpl(name, ScriptValue(value), sessionId);
}

bool SCXMLEngineImpl::setVariableSync(const std::string &name, double value, const std::string &sessionId) {
    return setVariableSyncImpl(name, ScriptValue(value), sessionId);
}

bool SCXMLEngineImpl::setVariableSync(const std::string &name, int64_t value, const std::string &sessionId) {
    return setVariableSyncImpl(name, ScriptValue(value), sessionId);
}

std::string SCXMLEngineImpl::getVariableSync(const std::string &name,
                                             [[maybe_unused]] const std::string &sessionId) const {
    if (!stateMachine_) {
        return "";
    }

    try {
        // Use StateMachine's session ID
        const std::string &smSessionId = stateMachine_->getSessionId();
        auto future = scriptEngine_.getVariable(smSessionId, name);
        auto result = future.get();

        if (!ScriptResultUtils::isSuccess(result)) {
            return "";
        }
        return ScriptResultUtils::resultToString(result);

    } catch (const std::exception &e) {
        SCE_LOG_WARN("SCXMLEngine: Failed to get variable '{}': {}", name, e.what());
        return "";
    }
}

std::string SCXMLEngineImpl::getLastStateMachineError(const std::string &sessionId) const {
    std::string actualSessionId = sessionId.empty() ? defaultSessionId_ : sessionId;
    auto it = sessionErrors_.find(actualSessionId);
    return it != sessionErrors_.end() ? it->second : "";
}

SCXMLEngine::Statistics SCXMLEngineImpl::getStatisticsSync([[maybe_unused]] const std::string &sessionId) const {
    Statistics stats;

    if (!stateMachine_) {
        return stats;
    }

    // Get statistics from StateMachine
    auto smStats = stateMachine_->getStatistics();
    stats.totalEvents = smStats.totalEvents;
    stats.totalTransitions = smStats.totalTransitions;
    stats.failedTransitions = smStats.failedTransitions;
    stats.currentState = smStats.currentState;
    stats.isRunning = smStats.isRunning;

    return stats;
}

}  // namespace SCE
