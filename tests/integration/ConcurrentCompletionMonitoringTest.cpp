#include "actions/ScriptAction.h"
#include "factory/NodeFactory.h"
#include "mocks/MockActionExecutor.h"
#include "model/StateNode.h"
#include "parsing/SCXMLParser.h"
#include "runtime/ExecutionContextImpl.h"
#include "scripting/JSEngine.h"
#include "states/ConcurrentCompletionMonitor.h"
#include "states/ConcurrentRegion.h"
#include "gtest/gtest.h"
#include <chrono>
#include <memory>
#include <string>
#include <thread>

namespace RSM {

class ConcurrentCompletionMonitoringTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &JSEngine::instance();
        engine_->reset();
        nodeFactory_ = std::make_shared<NodeFactory>();
        parser_ = std::make_unique<SCXMLParser>(nodeFactory_);
        monitor_ = std::make_unique<ConcurrentCompletionMonitor>("parallel_test");
        sessionId_ = "concurrent_completion_monitoring_test";
    }

    void TearDown() override {
        if (engine_) {
            engine_->reset();
        }
    }

    JSEngine *engine_;
    std::shared_ptr<NodeFactory> nodeFactory_;
    std::unique_ptr<SCXMLParser> parser_;
    std::unique_ptr<ConcurrentCompletionMonitor> monitor_;
    std::string sessionId_;
};

// Basic monitoring start/stop test
TEST_F(ConcurrentCompletionMonitoringTest, BasicMonitoringStartStop) {
    EXPECT_FALSE(monitor_->isMonitoringActive()) << "Monitoring is active at initialization";

    bool started = monitor_->startMonitoring();
    EXPECT_TRUE(started) << "Failed to start monitoring";
    EXPECT_TRUE(monitor_->isMonitoringActive()) << "Monitoring is not active";

    monitor_->stopMonitoring();
    EXPECT_FALSE(monitor_->isMonitoringActive()) << "Monitoring is not stopped";
}

// Region completion status update test
TEST_F(ConcurrentCompletionMonitoringTest, RegionCompletionUpdate) {
    monitor_->startMonitoring();

    // Update region completion status
    monitor_->updateRegionCompletion("region1", false);
    monitor_->updateRegionCompletion("region2", false);

    EXPECT_FALSE(monitor_->isCompletionCriteriaMet()) << "Completion criteria met when all regions are incomplete";

    // Complete one region
    monitor_->updateRegionCompletion("region1", true);
    EXPECT_FALSE(monitor_->isCompletionCriteriaMet()) << "Completion criteria met when only some regions are complete";

    // Complete all regions
    monitor_->updateRegionCompletion("region2", true);
    EXPECT_TRUE(monitor_->isCompletionCriteriaMet()) << "Completion criteria not met when all regions are complete";
}

// Registered regions retrieval test
TEST_F(ConcurrentCompletionMonitoringTest, RegisteredRegionsRetrieval) {
    monitor_->startMonitoring();

    // No regions should be registered initially
    auto regions = monitor_->getRegisteredRegions();
    EXPECT_TRUE(regions.empty()) << "Regions are registered in initial state";

    // Register regions
    monitor_->updateRegionCompletion("region1", false);
    monitor_->updateRegionCompletion("region2", false);
    monitor_->updateRegionCompletion("region3", false);

    regions = monitor_->getRegisteredRegions();
    EXPECT_EQ(regions.size(), 3) << "Number of registered regions differs from expected";

    // Check region names
    std::set<std::string> regionSet(regions.begin(), regions.end());
    EXPECT_TRUE(regionSet.count("region1") > 0) << "region1 is not registered";
    EXPECT_TRUE(regionSet.count("region2") > 0) << "region2 is not registered";
    EXPECT_TRUE(regionSet.count("region3") > 0) << "region3 is not registered";
}

// Update when monitoring inactive test
TEST_F(ConcurrentCompletionMonitoringTest, UpdateWhenMonitoringInactive) {
    // Attempt update when monitoring is inactive
    monitor_->updateRegionCompletion("region1", true);

    EXPECT_FALSE(monitor_->isCompletionCriteriaMet()) << "Completion criteria met when monitoring is inactive";

    auto regions = monitor_->getRegisteredRegions();
    EXPECT_TRUE(regions.empty()) << "Regions registered when monitoring is inactive";
}

// Concurrency test - concurrent updates from multiple threads
TEST_F(ConcurrentCompletionMonitoringTest, ConcurrentUpdates) {
    monitor_->startMonitoring();

    const int numThreads = 5;
    const int numRegionsPerThread = 10;
    std::vector<std::thread> threads;

    // Update region completion status concurrently from multiple threads
    for (int t = 0; t < numThreads; ++t) {
        threads.emplace_back([this, t, numRegionsPerThread]() {
            for (int r = 0; r < numRegionsPerThread; ++r) {
                std::string regionId = "thread" + std::to_string(t) + "_region" + std::to_string(r);
                monitor_->updateRegionCompletion(regionId, (r % 2 == 0));  // Even: complete, odd: incomplete
            }
        });
    }

    // Wait for all threads to complete
    for (auto &thread : threads) {
        thread.join();
    }

    auto regions = monitor_->getRegisteredRegions();
    EXPECT_EQ(regions.size(), numThreads * numRegionsPerThread) << "Number of registered regions differs from expected";

    // Completion criteria should not be met (odd regions are incomplete)
    EXPECT_FALSE(monitor_->isCompletionCriteriaMet()) << "Completion criteria met when some regions are incomplete";
}

// Empty regions completion criteria test
TEST_F(ConcurrentCompletionMonitoringTest, EmptyRegionsCompletionCriteria) {
    monitor_->startMonitoring();

    // Check completion criteria when no regions are registered
    EXPECT_FALSE(monitor_->isCompletionCriteriaMet()) << "Completion criteria met with empty region list";
}

// Duplicate region updates test
TEST_F(ConcurrentCompletionMonitoringTest, DuplicateRegionUpdates) {
    monitor_->startMonitoring();

    // Update the same region multiple times
    monitor_->updateRegionCompletion("region1", false);
    monitor_->updateRegionCompletion("region1", true);
    monitor_->updateRegionCompletion("region1", false);
    monitor_->updateRegionCompletion("region1", true);

    auto regions = monitor_->getRegisteredRegions();
    EXPECT_EQ(regions.size(), 1) << "Region registered multiple times due to duplicate updates";

    // Final state should be true
    EXPECT_TRUE(monitor_->isCompletionCriteriaMet()) << "Final completion state not reflected";
}

// Update with final state IDs test
TEST_F(ConcurrentCompletionMonitoringTest, UpdateWithFinalStateIds) {
    monitor_->startMonitoring();

    std::vector<std::string> finalStateIds = {"final1", "final2"};
    monitor_->updateRegionCompletion("region1", true, finalStateIds);
    monitor_->updateRegionCompletion("region2", false);

    EXPECT_FALSE(monitor_->isCompletionCriteriaMet()) << "Completion criteria met when only some regions are complete";

    monitor_->updateRegionCompletion("region2", true, {"final3"});
    EXPECT_TRUE(monitor_->isCompletionCriteriaMet()) << "Completion criteria not met when all regions are complete";
}

// Large scale region handling performance test
TEST_F(ConcurrentCompletionMonitoringTest, LargeScaleRegionHandling) {
    monitor_->startMonitoring();

    const int numRegions = 1000;
    auto startTime = std::chrono::high_resolution_clock::now();

    // Register and update large number of regions
    for (int i = 0; i < numRegions; ++i) {
        std::string regionId = "large_scale_region_" + std::to_string(i);
        monitor_->updateRegionCompletion(regionId, (i % 2 == 0));
    }

    auto endTime = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime);

    auto regions = monitor_->getRegisteredRegions();
    EXPECT_EQ(regions.size(), numRegions) << "Failed to register large number of regions";
    EXPECT_LT(duration.count(), 1000) << "Large scale region processing performance is too slow (exceeds 1 second)";

    // Completion criteria should not be met (odd regions are incomplete)
    EXPECT_FALSE(monitor_->isCompletionCriteriaMet()) << "Completion criteria met when some regions are incomplete";
}
}  // namespace RSM
