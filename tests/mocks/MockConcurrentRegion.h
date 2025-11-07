#pragma once

#include "events/EventDescriptor.h"
#include "states/IConcurrentRegion.h"
#include <atomic>
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Mock concurrent region for testing event broadcasting and parallel state components
 */
class MockConcurrentRegion : public IConcurrentRegion {
public:
    explicit MockConcurrentRegion(const std::string &id) : id_(id), active_(false), eventCount_(0), currentState_("") {}

    const std::string &getId() const override {
        return id_;
    }

    ConcurrentOperationResult activate() override {
        active_ = true;
        return ConcurrentOperationResult::success(id_);
    }

    ConcurrentOperationResult deactivate(std::shared_ptr<IExecutionContext> = nullptr) override {
        active_ = false;
        return ConcurrentOperationResult::success(id_);
    }

    bool isActive() const override {
        return active_;
    }

    bool isInFinalState() const override {
        return false;
    }

    ConcurrentRegionStatus getStatus() const override {
        return active_ ? ConcurrentRegionStatus::ACTIVE : ConcurrentRegionStatus::INACTIVE;
    }

    ConcurrentRegionInfo getInfo() const override {
        ConcurrentRegionInfo info;
        info.id = id_;
        info.status = getStatus();
        info.currentState = currentState_;
        info.isInFinalState = false;
        info.activeStates = getActiveStates();
        return info;
    }

    ConcurrentOperationResult processEvent(const EventDescriptor &event) override {
        lastEvent_ = event.eventName;
        eventCount_.fetch_add(1);
        return ConcurrentOperationResult::success(id_);
    }

    std::shared_ptr<IStateNode> getRootState() const override {
        return nullptr;
    }

    void setRootState(std::shared_ptr<IStateNode>) override {}

    std::vector<std::string> getActiveStates() const override {
        return active_ ? std::vector<std::string>{id_ + "_state"} : std::vector<std::string>{};
    }

    ConcurrentOperationResult reset() override {
        return ConcurrentOperationResult::success(id_);
    }

    std::vector<std::string> validate() const override {
        return {};
    }

    void setInvokeCallback(
        std::function<void(const std::string &, const std::vector<std::shared_ptr<IInvokeNode>> &)>) override {}

    void setConditionEvaluator(std::function<bool(const std::string &)>) override {}

    void setDoneStateCallback(std::function<void(const std::string &)>) override {}

    void setExecutionContext(std::shared_ptr<IExecutionContext>) override {}

    void setDesiredInitialChild(const std::string &) override {}

    const std::string &getCurrentState() const override {
        return currentState_;
    }

    void setCurrentState(const std::string &stateId) override {
        currentState_ = stateId;
    }

    size_t getEventCount() const {
        return eventCount_.load();
    }

    std::string getLastEvent() const {
        return lastEvent_;
    }

private:
    std::string id_;
    bool active_;
    std::atomic<size_t> eventCount_;
    std::string lastEvent_;
    std::string currentState_;
};

}  // namespace SCE
