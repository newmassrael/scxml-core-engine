// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084 D2: an engine's inbox (`core/EngineInbox.h`), apart from any
// engine. A hosted inbox is unbounded: it grows by segments of the forge LSCQ as
// posts arrive, refuses a post only when it is closed or its allocator refuses a
// segment, and makes a poster that finds every place taken wait for one rather
// than refuse it. The allocator is injected here, so the refusal the heap gives
// only under memory exhaustion can be made on demand.

#include "core/EngineInbox.h"

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <new>
#include <optional>
#include <thread>
#include <vector>

namespace SCE::Tests {

namespace {

/// What a `CountingAllocator` and the test that made it share: how many blocks it
/// may still give (none left means it refuses), and how many are out now.
struct Ledger {
    std::atomic<std::size_t> remaining{0};
    std::atomic<std::size_t> live{0};
};

/// The heap behind the segmented queue's allocator contract, giving at most what
/// its ledger allows and counting what it has out, so a case sees a refusal and a
/// leak. Copies share the ledger.
class CountingAllocator {
public:
    static constexpr ::SCE::Forge::Queue::Progress kProgress = ::SCE::Forge::Queue::Progress::Blocking;

    explicit CountingAllocator(std::shared_ptr<Ledger> ledger) noexcept : ledger_(std::move(ledger)) {}

    void *allocate(std::size_t size, std::size_t align) noexcept {
        std::size_t seen = ledger_->remaining.load(std::memory_order_acquire);
        do {
            if (seen == 0) {
                return nullptr;
            }
        } while (!ledger_->remaining.compare_exchange_weak(seen, seen - 1, std::memory_order_acq_rel));
        void *block = ::operator new(size, std::align_val_t(align), std::nothrow);
        if (block != nullptr) {
            ledger_->live.fetch_add(1, std::memory_order_acq_rel);
        }
        return block;
    }

    void deallocate(void *block, std::size_t, std::size_t align) noexcept {
        ::operator delete(block, std::align_val_t(align));
        ledger_->live.fetch_sub(1, std::memory_order_acq_rel);
    }

private:
    std::shared_ptr<Ledger> ledger_;
};

using Inbox = ::SCE::Core::EngineInboxState<int, CountingAllocator>;

/// Everything `inbox` holds, oldest first.
std::vector<int> drained(Inbox &inbox) {
    std::vector<int> taken;
    inbox.drain([&taken](int &&value) { taken.push_back(value); });
    return taken;
}

}  // namespace

// The first post makes the queue, and with it the first segment. An allocator that
// refuses that segment refuses the post, which is answered and counted; the next
// post asks again, and is taken once the allocator gives.
TEST(EngineInbox, APostWhoseSegmentIsRefusedIsAnsweredAndCountedAndTheNextAsksAgain) {
    auto ledger = std::make_shared<Ledger>();
    Inbox inbox{CountingAllocator(ledger)};

    EXPECT_FALSE(inbox.post(1)) << "a post the allocator gave no memory for was taken";
    EXPECT_EQ(inbox.refusedPosts(), 1u);

    ledger->remaining.store(1, std::memory_order_release);
    EXPECT_TRUE(inbox.post(2)) << "the inbox did not ask the allocator again";
    EXPECT_EQ(drained(inbox), std::vector<int>{2});
    EXPECT_EQ(inbox.refusedPosts(), 1u);
}

// A segment holds `ENGINE_INBOX_SEGMENT` posts. The one after them needs a second
// segment; an allocator that refuses it refuses that post alone, and what the first
// segment holds is kept and applied in order. Given memory again, the inbox grows.
TEST(EngineInbox, ThePostAFullSegmentCannotGrowForIsRefusedAndTheRestAreKept) {
    auto ledger = std::make_shared<Ledger>();
    ledger->remaining.store(1, std::memory_order_release);
    Inbox inbox{CountingAllocator(ledger)};

    std::vector<int> posted;
    for (int value = 0; value < static_cast<int>(::SCE::Core::ENGINE_INBOX_SEGMENT); ++value) {
        ASSERT_TRUE(inbox.post(value)) << "post " << value << " of the first segment was refused";
        posted.push_back(value);
    }
    EXPECT_FALSE(inbox.post(-1)) << "a post past the segment was taken with no memory for a second";
    EXPECT_EQ(inbox.refusedPosts(), 1u);

    ledger->remaining.store(1, std::memory_order_release);
    ASSERT_TRUE(inbox.post(1000)) << "the inbox did not grow once memory was given";
    posted.push_back(1000);
    EXPECT_EQ(drained(inbox), posted) << "what the inbox held was not kept, or not kept in order";
}

// Unbounded: far more posts than one segment, from more threads than there are
// places for posters at once, all taken. A post that found every place taken waited
// for one; none was refused for contention.
TEST(EngineInbox, PostsFromMoreThreadsThanItHasPlacesAreAllTaken) {
    auto ledger = std::make_shared<Ledger>();
    ledger->remaining.store(1u << 20, std::memory_order_release);
    Inbox inbox{CountingAllocator(ledger)};

    constexpr int THREADS = static_cast<int>(2 * ::SCE::Core::ENGINE_INBOX_PARTICIPANTS);
    constexpr int PER_THREAD = 16;
    std::atomic<int> refused{0};
    std::atomic<bool> released{false};
    std::vector<std::thread> posters;
    for (int thread = 0; thread < THREADS; ++thread) {
        posters.emplace_back([&inbox, &refused, &released, thread] {
            while (!released.load(std::memory_order_acquire)) {
                std::this_thread::yield();
            }
            for (int k = 0; k < PER_THREAD; ++k) {
                if (!inbox.post(thread * PER_THREAD + k)) {
                    refused.fetch_add(1, std::memory_order_acq_rel);
                }
            }
        });
    }
    released.store(true, std::memory_order_release);
    for (std::thread &poster : posters) {
        poster.join();
    }

    EXPECT_EQ(refused.load(), 0) << "an open inbox refused posts";
    EXPECT_EQ(inbox.refusedPosts(), 0u);
    const std::vector<int> taken = drained(inbox);
    ASSERT_EQ(taken.size(), static_cast<std::size_t>(THREADS * PER_THREAD));
    // Each thread's own posts keep their order.
    std::vector<int> next(THREADS, 0);
    for (int value : taken) {
        const int thread = value / PER_THREAD;
        EXPECT_EQ(value % PER_THREAD, next[thread]) << "thread " << thread << "'s posts were reordered";
        next[thread] = value % PER_THREAD + 1;
    }
}

// A drain applies what the inbox held when it began; what is posted while it runs
// waits for the next one, so a poster cannot hold the owner at one boundary.
TEST(EngineInbox, ADrainAppliesWhatWasHeldWhenItBegan) {
    auto ledger = std::make_shared<Ledger>();
    ledger->remaining.store(16, std::memory_order_release);
    Inbox inbox{CountingAllocator(ledger)};
    ASSERT_TRUE(inbox.post(1));
    ASSERT_TRUE(inbox.post(2));

    std::vector<int> first;
    bool repostsTaken = true;
    const std::size_t applied = inbox.drain([&inbox, &first, &repostsTaken](int &&value) {
        first.push_back(value);
        repostsTaken = inbox.post(value + 10) && repostsTaken;
    });
    EXPECT_TRUE(repostsTaken) << "a post made during a drain was refused";
    EXPECT_EQ(applied, 2u);
    EXPECT_EQ(first, (std::vector<int>{1, 2}));
    EXPECT_TRUE(inbox.holdsPosts());
    EXPECT_EQ(drained(inbox), (std::vector<int>{11, 12}));
    EXPECT_FALSE(inbox.holdsPosts());
}

// A closed inbox refuses every post until it is opened again, and what it held
// when it closed is still there to apply.
TEST(EngineInbox, AClosedInboxRefusesUntilItIsOpened) {
    auto ledger = std::make_shared<Ledger>();
    ledger->remaining.store(16, std::memory_order_release);
    Inbox inbox{CountingAllocator(ledger)};
    ASSERT_TRUE(inbox.post(1));

    inbox.close();
    EXPECT_FALSE(inbox.post(2)) << "a closed inbox took a post";
    EXPECT_EQ(inbox.refusedPosts(), 1u);
    EXPECT_EQ(drained(inbox), std::vector<int>{1});

    inbox.open();
    EXPECT_TRUE(inbox.post(3));
    EXPECT_EQ(drained(inbox), std::vector<int>{3});
}

// Every segment the inbox took goes back to its allocator, with the posts still in
// it, when the inbox goes.
TEST(EngineInbox, AnInboxGivesEverySegmentBackWhenItGoes) {
    auto ledger = std::make_shared<Ledger>();
    ledger->remaining.store(64, std::memory_order_release);
    {
        Inbox inbox{CountingAllocator(ledger)};
        for (int value = 0; value < static_cast<int>(3 * ::SCE::Core::ENGINE_INBOX_SEGMENT); ++value) {
            ASSERT_TRUE(inbox.post(value));
        }
        EXPECT_GE(ledger->live.load(), 3u) << "three segments' worth of posts held fewer segments";
    }
    EXPECT_EQ(ledger->live.load(), 0u) << "the inbox kept segments past its end";
}

}  // namespace SCE::Tests
