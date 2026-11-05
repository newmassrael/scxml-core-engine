// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The inbox of an engine: the one part of it a thread that does not own it may
// use (docs/adr/0084, ARCHITECTURE.md "Engine Threading").
//
// A multi-producer, single-consumer queue of what other threads have for an
// engine, which the engine's owner applies at its next macrostep boundary, each
// input by the owner-thread operation of the same name. What an input is belongs
// to the engine: a closed set, each carrying what the operation that applies it
// takes.
//
// The queue is the forge LSCQ (`sce/forge/queue_segmented.h`): Nikolaev's list
// of SCQ rings, lock-free on both sides and unbounded but for the memory it is
// given. A hosted inbox is unbounded on every runtime (ADR 0084, D2), so a post
// is refused only when the inbox is closed or the heap refuses a segment, never
// because the inbox is full. A stalled poster hides nothing another thread
// posted, and a post never waits for a macrostep.
//
// A header of the shared layer because the rule is both C++ engines': the
// generated engine and the Interpreter (`runtime/StateMachine.h`) each have
// their inbox here.

#pragma once

#include "sce/forge/queue.h"
#include "sce/forge/queue_segmented.h"

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <new>
#include <optional>
#include <thread>
#include <utility>

namespace SCE::Core {

/**
 * @brief The allocator a hosted engine's inbox takes its segments from: the C++
 *        heap, aligned as each segment asks
 *
 * The contract is the one `sce/forge/queue_segmented.h` states for a segmented
 * queue's allocator. It lives here, beside the engine, and not in the forge
 * runtime: SCE_FORGE.md §2.1 (C2) keeps allocators out of that runtime, and a
 * segmented queue borrows the allocator its user injects. A hosted engine is that
 * user, and the heap is what a hosted program has. The heap can block, which is
 * what `kProgress` says; it refuses by returning null, which the queue reports as
 * `OutOfMemory`. Stateless, so any number of threads may call it at once.
 */
struct HostedSegmentAllocator {
    static constexpr ::SCE::Forge::Queue::Progress kProgress = ::SCE::Forge::Queue::Progress::Blocking;

    void *allocate(std::size_t size, std::size_t align) noexcept {
        return ::operator new(size, std::align_val_t(align), std::nothrow);
    }

    void deallocate(void *block, std::size_t, std::size_t align) noexcept {
        ::operator delete(block, std::align_val_t(align));
    }
};

/// How many inputs one segment of an inbox's queue holds. A post that finds the
/// newest segment full takes a new one from the allocator, so this is the grain the
/// inbox grows and shrinks by, not a bound on what it holds.
inline constexpr std::size_t ENGINE_INBOX_SEGMENT = 64;

/// How many threads may be inside a post or a drain of one inbox at the same
/// instant: the queue's producer places and its hazard-pointer slots, both sides
/// together (the LSCQ's `R` and `H`, which are compile-time). A post that finds
/// every place taken waits for one, yielding, and is never refused for it, so
/// this bounds how many posts proceed at once and not how many are taken. Sixty-
/// four is far above the threads a host posts from — a transport's receive
/// threads, a deadline scheduler, the host's own workers — so the wait is reached
/// only under contention a host does not have, and the domain it sizes costs one
/// cache line a slot, made with the inbox's first post.
inline constexpr std::size_t ENGINE_INBOX_PARTICIPANTS = 64;

/**
 * @brief The waker a post calls: the one a host installed, shared by the inbox of
 *        every session of one tree
 *
 * A tree has one owner, so it has one waker (docs/adr/0084, D2 as amended at step
 * 10): a post to a child session's inbox wakes the thread that drives the root,
 * through what the host installed on the root. Each inbox is made with a waker of
 * its own, and a session started in a tree is handed the tree's
 * (`EngineInboxState::wakeThrough`). Installing and calling take a short lock of
 * its own, so both are safe from any thread.
 */
class InboxWaker {
public:
    /// Install @p waker; an empty one removes what was installed.
    void install(std::function<void()> waker) {
        std::shared_ptr<const std::function<void()>> installed;
        if (waker) {
            installed = std::make_shared<const std::function<void()>>(std::move(waker));
        }
        std::lock_guard<std::mutex> lock(mutex_);
        waker_ = std::move(installed);
    }

    /// Call what is installed, on the calling thread, outside the lock.
    void wake() const {
        std::shared_ptr<const std::function<void()>> waker;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            waker = waker_;
        }
        if (waker) {
            (*waker)();
        }
    }

private:
    mutable std::mutex mutex_;
    std::shared_ptr<const std::function<void()>> waker_;
};

/**
 * @brief What an engine and every handle to its inbox share
 *
 * The queue, the gate that opens and closes it, the waker the host installed and
 * the engine's owner thread. Held by a `std::shared_ptr`, so a handle a host keeps
 * on another thread stays valid after the engine is gone: its posts are refused
 * from then on (`EngineInboxOwner` closes the inbox), never written into freed
 * memory.
 *
 * The posting half (`post`, `wakeWith`) is safe from any thread. The rest is the
 * owner's.
 *
 * @tparam Input The engine's closed set of inputs. Moved into a node of its own
 *         on the posting thread, so the queue's slots hold one pointer each
 *         whatever the input weighs.
 * @tparam Allocator Where the queue's segments come from (the contract of
 *         `sce/forge/queue_segmented.h`). The heap for an engine; a test injects
 *         one that refuses.
 */
template <typename Input, typename Allocator = HostedSegmentAllocator> class EngineInboxState {
public:
    explicit EngineInboxState(Allocator allocator = Allocator()) : allocator_(std::move(allocator)) {}

    ~EngineInboxState() {
        // Every handle is gone, so nobody posts; what is still queued is destroyed
        // with the queue, and the queue gives its segments back to `allocator_`,
        // which outlives it.
        delete ring_.load(std::memory_order_acquire);
    }

    EngineInboxState(const EngineInboxState &) = delete;
    EngineInboxState &operator=(const EngineInboxState &) = delete;
    EngineInboxState(EngineInboxState &&) = delete;
    EngineInboxState &operator=(EngineInboxState &&) = delete;

    /**
     * @brief Post @p input; whether the inbox took it
     *
     * Refused when the inbox is closed (the run ended, or was stopped, or the
     * engine is gone) and when memory for it is refused. A refusal is counted.
     * After a post the inbox took, the waker the host installed is called on this
     * thread.
     */
    bool post(Input input) {
        // Made before the post is counted in, so nothing below can throw while a
        // close is waiting on the count.
        std::unique_ptr<Input> node(new (std::nothrow) Input(std::move(input)));
        Ring *ring = node ? madeRing() : nullptr;
        if (ring == nullptr) {
            refused_.fetch_add(1, std::memory_order_relaxed);
            return false;
        }
        // Counted in before the gate is read, so a close waits for every post that
        // saw the inbox open, and no post that saw it open lands after the close
        // took what the inbox held.
        const std::uint64_t gate = gate_.fetch_add(ONE_POST, std::memory_order_acq_rel);
        bool taken = false;
        if ((gate & CLOSED) == 0) {
            taken = push(*ring, std::move(node));
        }
        gate_.fetch_sub(ONE_POST, std::memory_order_acq_rel);
        if (!taken) {
            refused_.fetch_add(1, std::memory_order_relaxed);
            return false;
        }
        wake();
        return true;
    }

    /**
     * @brief Install @p waker, called on the posting thread after each post the
     *        inbox took; an empty one removes it
     *
     * How a host that drives the engine itself (`tick`, `step`) learns there is
     * something to apply. It is installed in the waker this inbox calls, which is
     * the tree's once the engine's session was started in one (`wakeThrough`), so
     * a post to any session of the tree calls it. Installing and reading it takes a
     * short lock of its own; the queue takes none.
     */
    void wakeWith(std::function<void()> waker) {
        currentWaker()->install(std::move(waker));
    }

    /// The waker a post to this inbox calls, to hand to a session started in the
    /// same tree.
    std::shared_ptr<InboxWaker> waker() const {
        return currentWaker();
    }

    /**
     * @brief Call @p waker after each post this inbox takes from now on: the tree's,
     *        handed to a session started in it (docs/adr/0084, D2)
     *
     * Made by the owner before the session is entered in the tree's directory, so
     * before any thread can post to it through the tree. An empty one is ignored.
     */
    void wakeThrough(std::shared_ptr<InboxWaker> waker) {
        if (!waker) {
            return;
        }
        std::lock_guard<std::mutex> lock(wakerMutex_);
        waker_ = std::move(waker);
    }

    /**
     * @brief Apply, with @p apply, what the inbox held when this was called, oldest
     *        first; how many inputs were applied
     *
     * The owner's. What other threads post while this runs waits for the next
     * boundary, so posters that never stop cannot hold the owner at this one.
     */
    template <typename Apply> std::size_t drain(Apply &&apply) {
        Ring *ring = ring_.load(std::memory_order_acquire);
        if (ring == nullptr) {
            return 0;
        }
        const std::size_t due = held_.load(std::memory_order_acquire);
        if (due == 0) {
            return 0;
        }
        for (;;) {
            // A consumer needs a slot of the domain, which posters in flight hold
            // only for the length of a push.
            if (auto consumer = ring->queue.consumer()) {
                std::size_t applied = 0;
                while (applied < due) {
                    std::optional<std::unique_ptr<Input>> taken = consumer->try_pop();
                    if (!taken) {
                        break;
                    }
                    held_.fetch_sub(1, std::memory_order_acq_rel);
                    ++applied;
                    apply(std::move(**taken));
                }
                return applied;
            }
            std::this_thread::yield();
        }
    }

    /// Open the inbox: posts are taken from now on. The owner's.
    void open() noexcept {
        gate_.fetch_and(~CLOSED, std::memory_order_acq_rel);
    }

    /**
     * @brief Close the inbox: every post from now on is refused
     *
     * Returns once every post that found the inbox open has finished, so what it
     * holds when this returns is all it will hold until it is opened again. The
     * owner's.
     */
    void close() noexcept {
        gate_.fetch_or(CLOSED, std::memory_order_acq_rel);
        while ((gate_.load(std::memory_order_acquire) & ~CLOSED) != 0) {
            // A post in flight is a push, finished in a bounded number of its own
            // steps once it holds a place.
            std::this_thread::yield();
        }
    }

    /// Whether a post was taken that the owner has not applied yet.
    bool holdsPosts() const noexcept {
        return held_.load(std::memory_order_acquire) != 0;
    }

    /// How many posts the inbox refused since it was made.
    std::uint64_t refusedPosts() const noexcept {
        return refused_.load(std::memory_order_relaxed);
    }

    /**
     * @brief Make the calling thread the engine's owner when it has none; whether
     *        the calling thread is its owner now
     */
    bool claimOwner() noexcept {
        std::thread::id expected{};
        const std::thread::id self = std::this_thread::get_id();
        if (owner_.compare_exchange_strong(expected, self, std::memory_order_acq_rel, std::memory_order_acquire)) {
            return true;
        }
        return expected == self;
    }

    /// Whether the calling thread may make an owner-thread call: it is the owner,
    /// or the engine has none.
    bool isOwnerHere() const noexcept {
        const std::thread::id owner = owner_.load(std::memory_order_acquire);
        return owner == std::thread::id() || owner == std::this_thread::get_id();
    }

    /// Whether the calling thread is the engine's owner now. Unlike `isOwnerHere`,
    /// false when the engine has no owner: for an engine whose owner is the thread
    /// running its macrosteps for as long as it runs them (the Interpreter), this
    /// is the question "is this call made from inside a macrostep of mine".
    bool ownedHere() const noexcept {
        return owner_.load(std::memory_order_acquire) == std::this_thread::get_id();
    }

    /// The engine has no owner from now on: its run ended, or was stopped, or a new
    /// one is being entered.
    void releaseOwner() noexcept {
        owner_.store(std::thread::id(), std::memory_order_release);
    }

private:
    using Queue =
        ::SCE::Forge::Queue::Lscq<std::unique_ptr<Input>, ENGINE_INBOX_SEGMENT, ENGINE_INBOX_PARTICIPANTS,
                                  ENGINE_INBOX_PARTICIPANTS, Allocator, ::SCE::Forge::Queue::Progress::Blocking>;

    /// The queue and the hazard-pointer domain it borrows, made together by the
    /// first post. The domain is declared first, so the queue, whose destruction
    /// flushes the domain, goes before it.
    struct Ring {
        typename Queue::Domain domain;
        Queue queue;

        explicit Ring(Allocator &allocator) noexcept : domain(), queue(allocator, domain) {}
    };

    /// The gate's closed bit.
    static constexpr std::uint64_t CLOSED = 1;
    /// What one post in flight adds to the gate, above the closed bit.
    static constexpr std::uint64_t ONE_POST = 2;

    /// The queue, made by the first post: an engine nobody posts to never pays for
    /// one, which matters to a host running many sessions. Null when the memory for
    /// it, or for its first segment, is refused; the next post asks again.
    Ring *madeRing() noexcept {
        Ring *existing = ring_.load(std::memory_order_acquire);
        if (existing != nullptr) {
            return existing;
        }
        Ring *made = new (std::nothrow) Ring(allocator_);
        if (made == nullptr) {
            return nullptr;
        }
        if (!made->queue.valid()) {
            delete made;
            return nullptr;
        }
        if (ring_.compare_exchange_strong(existing, made, std::memory_order_acq_rel, std::memory_order_acquire)) {
            return made;
        }
        // Another post made it first; this one is given back.
        delete made;
        return existing;
    }

    /// Push @p node; false only when the allocator refused a segment. A producer
    /// handle is taken for the push and given back after it; when every place is
    /// taken, this waits for one rather than refuse the post.
    bool push(Ring &ring, std::unique_ptr<Input> node) {
        for (;;) {
            if (auto producer = ring.queue.producer()) {
                held_.fetch_add(1, std::memory_order_acq_rel);
                if (producer->try_push(std::move(node)) == ::SCE::Forge::Queue::PushStatus::Ok) {
                    return true;
                }
                held_.fetch_sub(1, std::memory_order_acq_rel);
                return false;
            }
            std::this_thread::yield();
        }
    }

    std::shared_ptr<InboxWaker> currentWaker() const {
        std::lock_guard<std::mutex> lock(wakerMutex_);
        return waker_;
    }

    void wake() {
        currentWaker()->wake();
    }

    /// Declared before `ring_`'s queue can need it, and outlives it: the destructor
    /// deletes the ring first.
    Allocator allocator_;
    std::atomic<Ring *> ring_{nullptr};
    /// The closed bit, and `ONE_POST` for each post in flight.
    std::atomic<std::uint64_t> gate_{0};
    /// Posts taken and not yet applied.
    std::atomic<std::size_t> held_{0};
    std::atomic<std::uint64_t> refused_{0};
    /// Guards which waker `waker_` is; the waker guards what is installed in it.
    mutable std::mutex wakerMutex_;
    std::shared_ptr<InboxWaker> waker_ = std::make_shared<InboxWaker>();
    std::atomic<std::thread::id> owner_{std::thread::id()};
};

/**
 * @brief The handle a host posts to an engine's inbox through, from any thread
 *
 * Copyable and cheap to copy; every copy names the same inbox, and stays valid
 * after the engine is gone (its posts are refused then).
 */
template <typename Input, typename Allocator = HostedSegmentAllocator> class EngineInbox {
public:
    explicit EngineInbox(std::shared_ptr<EngineInboxState<Input, Allocator>> state) noexcept
        : state_(std::move(state)) {}

    /// Post @p input; whether the inbox took it. See `EngineInboxState::post`.
    bool post(Input input) const {
        return state_->post(std::move(input));
    }

    /// Install the waker a post calls; see `EngineInboxState::wakeWith`.
    void wakeWith(std::function<void()> waker) const {
        state_->wakeWith(std::move(waker));
    }

private:
    std::shared_ptr<EngineInboxState<Input, Allocator>> state_;
};

/**
 * @brief The engine's own hold on its inbox
 *
 * Makes the shared state with the engine, hands out handles, and closes the inbox
 * when the engine goes, so a handle kept past the engine is refused rather than
 * heard by nobody. Movable with its engine; not copyable, because a copy of an
 * engine sharing the original's inbox would be a second consumer of it.
 */
template <typename Input, typename Allocator = HostedSegmentAllocator> class EngineInboxOwner {
public:
    explicit EngineInboxOwner(Allocator allocator = Allocator())
        : state_(std::make_shared<EngineInboxState<Input, Allocator>>(std::move(allocator))) {}

    ~EngineInboxOwner() {
        if (state_) {
            state_->close();
        }
    }

    EngineInboxOwner(EngineInboxOwner &&) noexcept = default;

    EngineInboxOwner &operator=(EngineInboxOwner &&other) noexcept {
        if (this != &other) {
            if (state_) {
                state_->close();
            }
            state_ = std::move(other.state_);
        }
        return *this;
    }

    EngineInboxOwner(const EngineInboxOwner &) = delete;
    EngineInboxOwner &operator=(const EngineInboxOwner &) = delete;

    /// A handle to this inbox, for any thread.
    EngineInbox<Input, Allocator> handle() const {
        return EngineInbox<Input, Allocator>(state_);
    }

    /// The shared state, for the engine's own (owner-side) use.
    EngineInboxState<Input, Allocator> &state() const noexcept {
        return *state_;
    }

private:
    std::shared_ptr<EngineInboxState<Input, Allocator>> state_;
};

}  // namespace SCE::Core
