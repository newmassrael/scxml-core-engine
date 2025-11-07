#include "events/EventDescriptor.h"
#include "mocks/MockConcurrentRegion.h"
#include "states/ConcurrentEventBroadcaster.h"
#include "gtest/gtest.h"
#include <memory>
#include <string>
#include <thread>
#include <vector>

namespace SCE {

class ParallelStateEventBroadcastingTest : public ::testing::Test {
protected:
    void SetUp() override {
        broadcaster_ = std::make_unique<ConcurrentEventBroadcaster>();

        // Create mock regions
        region1_ = std::make_shared<MockConcurrentRegion>("region1");
        region2_ = std::make_shared<MockConcurrentRegion>("region2");
        region3_ = std::make_shared<MockConcurrentRegion>("region3");
    }

    void TearDown() override {
        broadcaster_.reset();
    }

    std::unique_ptr<ConcurrentEventBroadcaster> broadcaster_;
    std::shared_ptr<MockConcurrentRegion> region1_;
    std::shared_ptr<MockConcurrentRegion> region2_;
    std::shared_ptr<MockConcurrentRegion> region3_;
};

// Test 1: Selective event broadcasting to specific regions
TEST_F(ParallelStateEventBroadcastingTest, SelectiveEventBroadcasting) {
    // Register all regions
    broadcaster_->registerRegion(region1_);
    broadcaster_->registerRegion(region2_);
    broadcaster_->registerRegion(region3_);

    // Activate regions
    region1_->activate();
    region2_->activate();
    region3_->activate();

    // Broadcast to specific regions only
    EventDescriptor event;
    event.eventName = "selective_event";

    std::vector<std::string> targetRegions = {"region1", "region3"};
    auto result = broadcaster_->broadcastEventToRegions(event, targetRegions);

    EXPECT_TRUE(result.isSuccess) << "Selective event broadcasting failed";

    // Only region1 and region3 should receive the event
    EXPECT_EQ(region1_->getEventCount(), 1) << "Region1 should receive event";
    EXPECT_EQ(region2_->getEventCount(), 0) << "Region2 should not receive event";
    EXPECT_EQ(region3_->getEventCount(), 1) << "Region3 should receive event";

    EXPECT_EQ(region1_->getLastEvent(), "selective_event");
    EXPECT_EQ(region3_->getLastEvent(), "selective_event");
}

// Test 2: Concurrent broadcasting thread safety
TEST_F(ParallelStateEventBroadcastingTest, ConcurrentBroadcasting) {
    broadcaster_->registerRegion(region1_);
    broadcaster_->registerRegion(region2_);
    region1_->activate();
    region2_->activate();

    std::vector<std::thread> threads;
    std::atomic<int> successCount{0};

    // Concurrent broadcasts from multiple threads
    for (int i = 0; i < 10; ++i) {
        threads.emplace_back([this, &successCount, i]() {
            EventDescriptor event;
            event.eventName = "concurrent_event_" + std::to_string(i);

            auto result = broadcaster_->broadcastEvent(event);
            if (result.isSuccess) {
                successCount.fetch_add(1);
            }
        });
    }

    for (auto &thread : threads) {
        thread.join();
    }

    EXPECT_EQ(successCount.load(), 10) << "Some concurrent broadcasts failed";
    EXPECT_EQ(region1_->getEventCount(), 10) << "Region1 should receive all 10 events";
    EXPECT_EQ(region2_->getEventCount(), 10) << "Region2 should receive all 10 events";
}

// Test 3: Event statistics tracking
TEST_F(ParallelStateEventBroadcastingTest, EventStatistics) {
    broadcaster_->registerRegion(region1_);
    broadcaster_->registerRegion(region2_);
    region1_->activate();
    region2_->activate();

    // Broadcast multiple events
    for (int i = 0; i < 5; ++i) {
        EventDescriptor event;
        event.eventName = "stats_event_" + std::to_string(i);
        broadcaster_->broadcastEvent(event);
    }

    auto stats = broadcaster_->getStatistics();
    EXPECT_GT(stats.totalEvents, 0) << "No events were broadcast";
    // Note: ConcurrentEventBroadcaster doesn't track region count, verify via region event counts
    EXPECT_EQ(region1_->getEventCount(), 5) << "Region1 should receive 5 events";
    EXPECT_EQ(region2_->getEventCount(), 5) << "Region2 should receive 5 events";
}

// Test 4: Error handling for non-existent regions
TEST_F(ParallelStateEventBroadcastingTest, ErrorHandling) {
    broadcaster_->registerRegion(region1_);
    region1_->activate();

    // Try to broadcast to non-existent region
    EventDescriptor event;
    event.eventName = "error_test_event";

    std::vector<std::string> invalidRegions = {"region1", "nonexistent_region"};
    auto result = broadcaster_->broadcastEventToRegions(event, invalidRegions);

    // Should still succeed for valid region, skip invalid
    EXPECT_TRUE(result.isSuccess) << "Should handle non-existent regions gracefully";
    EXPECT_EQ(region1_->getEventCount(), 1) << "Valid region should still receive event";
}

// Test 5: Broadcasting to inactive regions
TEST_F(ParallelStateEventBroadcastingTest, InactiveRegionHandling) {
    broadcaster_->registerRegion(region1_);
    broadcaster_->registerRegion(region2_);

    // Only activate region1, leave region2 inactive
    region1_->activate();

    EventDescriptor event;
    event.eventName = "inactive_test";
    auto result = broadcaster_->broadcastEvent(event);

    // Should only broadcast to active regions
    EXPECT_TRUE(result.isSuccess);
    EXPECT_EQ(region1_->getEventCount(), 1) << "Active region should receive event";
    EXPECT_EQ(region2_->getEventCount(), 0) << "Inactive region should not receive event";
}

}  // namespace SCE
