#include "events/EventDescriptor.h"
#include "mocks/MockConcurrentRegion.h"
#include "states/ConcurrentCompletionMonitor.h"
#include "states/ConcurrentEventBroadcaster.h"
#include "states/ExternalTransitionHandler.h"
#include "gtest/gtest.h"
#include <chrono>
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Integration tests for parallel state components working together
 *
 * Tests the interaction between ConcurrentEventBroadcaster, ConcurrentCompletionMonitor,
 * and ExternalTransitionHandler. Individual component tests are in separate files:
 * - ConcurrentEventBroadcaster: ParallelStateEventBroadcastingTest.cpp
 * - ConcurrentCompletionMonitor: ConcurrentCompletionMonitoringTest.cpp
 * - ExternalTransitionHandler: ExternalTransitionHandlerTest.cpp
 */
class ParallelStateComponentTest : public ::testing::Test {
protected:
    void SetUp() override {
        broadcaster_ = std::make_unique<ConcurrentEventBroadcaster>();
        monitor_ = std::make_unique<ConcurrentCompletionMonitor>("parallel_test");
        handler_ = std::make_unique<ExternalTransitionHandler>(5);
    }

    void TearDown() override {
        broadcaster_.reset();
        monitor_.reset();
        handler_.reset();
    }

    std::unique_ptr<ConcurrentEventBroadcaster> broadcaster_;
    std::unique_ptr<ConcurrentCompletionMonitor> monitor_;
    std::unique_ptr<ExternalTransitionHandler> handler_;
};

// ============================================================================
// Integrated Scenario Tests (Component Interactions)
// ============================================================================

TEST_F(ParallelStateComponentTest, IntegratedScenario_PartialCompletionWithTransition) {
    const std::string parallelStateId = "partial_parallel";
    std::vector<std::string> regionIds = {"region1", "region2", "region3"};

    // Register parallel state to handler
    handler_->registerParallelState(parallelStateId, regionIds);

    // Register regions to broadcaster
    auto region1 = std::make_shared<MockConcurrentRegion>("region1");
    auto region2 = std::make_shared<MockConcurrentRegion>("region2");
    auto region3 = std::make_shared<MockConcurrentRegion>("region3");

    broadcaster_->registerRegion(region1);
    broadcaster_->registerRegion(region2);
    broadcaster_->registerRegion(region3);

    region1->activate();
    region2->activate();
    region3->activate();

    // Start completion monitoring
    monitor_->startMonitoring();

    // Complete only some regions
    monitor_->updateRegionCompletion("region1", true);
    monitor_->updateRegionCompletion("region2", false);
    monitor_->updateRegionCompletion("region3", false);

    EXPECT_FALSE(monitor_->isCompletionCriteriaMet()) << "Completion criteria met when only some regions are complete";

    // Force external transition (from incomplete state)
    bool transitionResult = handler_->handleExternalTransition(parallelStateId, "early_exit", "force_exit");
    EXPECT_TRUE(transitionResult) << "Forced external transition failed";

    // Verify transition was processed
    EXPECT_EQ(handler_->getActiveTransitionCount(), 0) << "Active transition count should be 0 after completion";
}

// ============================================================================
// Performance and Stress Tests
// ============================================================================

TEST_F(ParallelStateComponentTest, Performance_LargeScaleComponents) {
    const int numStates = 100;
    const int numRegionsPerState = 10;

    auto startTime = std::chrono::high_resolution_clock::now();

    // Register large number of parallel states to handler
    for (int i = 0; i < numStates; ++i) {
        std::vector<std::string> regionIds;
        for (int j = 0; j < numRegionsPerState; ++j) {
            regionIds.push_back("state" + std::to_string(i) + "_region" + std::to_string(j));
        }

        handler_->registerParallelState("parallel_" + std::to_string(i), regionIds);
    }

    // Register regions to broadcaster
    std::vector<std::shared_ptr<MockConcurrentRegion>> allRegions;
    for (int i = 0; i < numStates; ++i) {
        for (int j = 0; j < numRegionsPerState; ++j) {
            auto region =
                std::make_shared<MockConcurrentRegion>("state" + std::to_string(i) + "_region" + std::to_string(j));
            allRegions.push_back(region);
            broadcaster_->registerRegion(region);
            region->activate();
        }
    }

    auto endTime = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

    EXPECT_LT(duration.count(), 1000)
        << "Large-scale component registration performance is too slow (exceeds 1 second)";

    // Large-scale event broadcasting test
    startTime = std::chrono::high_resolution_clock::now();

    for (int i = 0; i < 100; ++i) {
        EventDescriptor event;
        event.eventName = "perf_test_event_" + std::to_string(i);
        broadcaster_->broadcastEvent(event);
    }

    endTime = std::chrono::high_resolution_clock::now();
    duration = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

    // Performance threshold: 100 states × 10 regions × 100 events = 100,000 event deliveries
    // Actual performance: ~7.5s (13,333 ops/sec)
    // Threshold: 8s with margin for CI/debug builds
    EXPECT_LT(duration.count(), 8000) << "Large-scale event broadcasting performance is too slow (exceeds 8 seconds)";

    // Verify events were received
    size_t totalEvents = 0;
    for (const auto &region : allRegions) {
        totalEvents += region->getEventCount();
    }

    EXPECT_EQ(totalEvents, 100 * numStates * numRegionsPerState)
        << "Not all regions received all events: expected " << (100 * numStates * numRegionsPerState) << ", got "
        << totalEvents;
}

}  // namespace SCE
