// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "events/EventDescriptor.h"
#include "states/IConcurrentRegion.h"
#include <atomic>
#include <memory>
#include <mutex>
#include <string>

namespace SCE {

/**
 * @brief Mock concurrent region for testing event broadcasting and parallel state components
 *
 * `processEvent` is called from the broadcaster's worker threads — that is the
 * whole point of `ParallelStateEventBroadcastingTest.ConcurrentBroadcasting`,
 * which fires ten broadcasts at two registered regions at once. So the mock's
 * own recording has to be thread-safe, or the test that exists to prove the
 * broadcaster is safe corrupts the heap on its own bookkeeping instead.
 *
 * It did: `lastEvent_` was a plain `std::string` assigned from every worker,
 * and a full `ctest -j 8` run aborted with `double free or corruption
 * (fasttop)` inside this test on 2026-08-14. The race is narrow — the event
 * names here are short enough to live in the small-string buffer most of the
 * time — so it surfaced only under load, and a green suite was not evidence
 * of its absence.
 */
class MockConcurrentRegion : public IConcurrentRegion {
public:
    explicit MockConcurrentRegion(const std::string &id) : id_(id), active_(false), eventCount_(0), currentState_("") {}

    const std::string &getId() const override {
        return id_;
    }

    /// This mock holds no states, so `enterDefaultChild` (§scxml-D) has nothing
    /// to select: activation here is only the active flag.
    ConcurrentOperationResult activate(bool = true) override {
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
        {
            std::lock_guard<std::mutex> lock(lastEventMutex_);
            lastEvent_ = event.eventName;
        }
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

    void setRestoringSnapshot(bool) override {
        // W3C SCXML 3.13: Mock implementation for restoration mode flag
        // No-op for test mock - restoration semantics handled by real ConcurrentRegion
    }

    size_t getEventCount() const {
        return eventCount_.load();
    }

    std::string getLastEvent() const {
        std::lock_guard<std::mutex> lock(lastEventMutex_);
        return lastEvent_;
    }

private:
    std::string id_;
    /// Read by `isActive`/`getStatus`/`getActiveStates` from every broadcasting
    /// thread while `activate()` may still be running on another.
    std::atomic<bool> active_;
    std::atomic<size_t> eventCount_;
    /// `currentState_` needs no lock and could not usefully have one:
    /// `getCurrentState()` returns a reference by interface, so a caller holds
    /// it after any lock would be released. Nothing writes it concurrently.
    mutable std::mutex lastEventMutex_;
    std::string lastEvent_;
    std::string currentState_;
};

}  // namespace SCE
