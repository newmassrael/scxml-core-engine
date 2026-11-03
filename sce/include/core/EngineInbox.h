// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The inbox of an engine: the one part of it a thread that does not own it may
// use (docs/adr/0083, ARCHITECTURE.md "Engine Threading").
//
// A multi-producer, single-consumer queue of what other threads have for an
// engine, which the engine's owner applies at its next macrostep boundary, each
// input by the owner-thread operation of the same name. The queue is the forge
// SCQ queue (`sce/forge/queue.h`, the one `mesh/EventQueueBridge.h` wraps), so a
// post never waits for a macrostep and a stalled poster hides nothing another
// thread posted. What an input is belongs to the engine: a closed set, each
// carrying what the operation that applies it takes.
//
// A header of the shared layer because the rule is both C++ engines': the
// generated engine has its inbox here, and the Interpreter takes the same one
// when its threading follows the rule.

#pragma once

#include "sce/forge/queue.h"

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <thread>
#include <utility>

namespace SCE::Core {

/// How many posted inputs an engine's inbox holds that its owner has not applied
/// yet. The inbox is bounded, as the SCQ queue is: a post that finds it full is
/// refused, the poster is told (`post` answers `false`) and the engine counts it
/// (`refusedPosts`), so nothing is dropped without a word. What fills it is what
/// other threads post between two macrostep boundaries of one owner; the bound is
/// far above what a host or a Mesh router delivers in that time, and a poster that
/// reaches it is one whose owner has stopped driving the engine.
inline constexpr std::size_t ENGINE_INBOX_CAPACITY = 1024;

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
 */
template <typename Input> class EngineInboxState {
public:
    EngineInboxState() = default;

    ~EngineInboxState() {
        // Every handle is gone, so nobody posts; what is still queued is destroyed
        // with the queue.
        delete queue_.load(std::memory_order_acquire);
    }

    EngineInboxState(const EngineInboxState &) = delete;
    EngineInboxState &operator=(const EngineInboxState &) = delete;
    EngineInboxState(EngineInboxState &&) = delete;
    EngineInboxState &operator=(EngineInboxState &&) = delete;

    /**
     * @brief Post @p input; whether the inbox took it
     *
     * Refused when the inbox is closed (the run ended, or was stopped, or the
     * engine is gone), when it holds `ENGINE_INBOX_CAPACITY` inputs, and when more
     * threads than that are posting at the same instant. A refusal is counted.
     * After a post the inbox took, the waker the host installed is called on this
     * thread.
     */
    bool post(Input input) {
        // Allocated before the post is counted in: an allocation that throws must
        // not leave a count a close would wait on for ever.
        auto node = std::make_unique<Input>(std::move(input));
        Queue &ring = queue();
        // Counted in before the gate is read, so a close waits for every post that
        // saw the inbox open, and no post that saw it open lands after the close
        // took what the inbox held.
        const std::uint64_t gate = gate_.fetch_add(ONE_POST, std::memory_order_acq_rel);
        bool taken = false;
        if ((gate & CLOSED) == 0) {
            taken = push(ring, std::move(node));
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
     * something to apply. Installing and reading it takes a short lock of its own;
     * the queue takes none.
     */
    void wakeWith(std::function<void()> waker) {
        std::shared_ptr<const std::function<void()>> installed;
        if (waker) {
            installed = std::make_shared<const std::function<void()>>(std::move(waker));
        }
        std::lock_guard<std::mutex> lock(wakerMutex_);
        waker_ = std::move(installed);
    }

    /**
     * @brief Apply, with @p apply, what the inbox holds, oldest first, at most
     *        @p limit inputs; how many were applied
     *
     * The owner's. Bounded so posters that never stop cannot hold the owner at one
     * boundary: what is left is applied at the next one.
     */
    template <typename Apply> std::size_t drain(std::size_t limit, Apply &&apply) {
        Queue *ring = queue_.load(std::memory_order_acquire);
        if (ring == nullptr) {
            return 0;
        }
        // Only the owner consumes, so the one consumer place is free.
        auto consumer = ring->consumer();
        if (!consumer) {
            return 0;
        }
        std::size_t applied = 0;
        while (applied < limit) {
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
            // A post in flight is a lock-free push, finished in a bounded number of
            // its own steps.
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

    /// The engine has no owner from now on: its run ended, or was stopped, or a new
    /// one is being entered.
    void releaseOwner() noexcept {
        owner_.store(std::thread::id(), std::memory_order_release);
    }

private:
    using Queue = ::SCE::Forge::Queue::Scq<std::unique_ptr<Input>, ENGINE_INBOX_CAPACITY, ENGINE_INBOX_CAPACITY>;

    /// The gate's closed bit.
    static constexpr std::uint64_t CLOSED = 1;
    /// What one post in flight adds to the gate, above the closed bit.
    static constexpr std::uint64_t ONE_POST = 2;

    /// The queue, made by the first post: an engine nobody posts to never pays for
    /// one, which matters to a host running many sessions.
    Queue &queue() {
        Queue *existing = queue_.load(std::memory_order_acquire);
        if (existing != nullptr) {
            return *existing;
        }
        auto made = std::make_unique<Queue>();
        if (queue_.compare_exchange_strong(existing, made.get(), std::memory_order_acq_rel,
                                           std::memory_order_acquire)) {
            return *made.release();
        }
        // Another post made it first; this one is given back.
        return *existing;
    }

    bool push(Queue &ring, std::unique_ptr<Input> node) {
        auto producer = ring.producer();
        if (!producer) {
            return false;
        }
        held_.fetch_add(1, std::memory_order_acq_rel);
        if (producer->try_push(std::move(node)) == ::SCE::Forge::Queue::PushStatus::Ok) {
            return true;
        }
        held_.fetch_sub(1, std::memory_order_acq_rel);
        return false;
    }

    void wake() {
        std::shared_ptr<const std::function<void()>> waker;
        {
            std::lock_guard<std::mutex> lock(wakerMutex_);
            waker = waker_;
        }
        if (waker) {
            (*waker)();
        }
    }

    std::atomic<Queue *> queue_{nullptr};
    /// The closed bit, and `ONE_POST` for each post in flight.
    std::atomic<std::uint64_t> gate_{0};
    /// Posts taken and not yet applied.
    std::atomic<std::size_t> held_{0};
    std::atomic<std::uint64_t> refused_{0};
    std::mutex wakerMutex_;
    std::shared_ptr<const std::function<void()>> waker_;
    std::atomic<std::thread::id> owner_{std::thread::id()};
};

/**
 * @brief The handle a host posts to an engine's inbox through, from any thread
 *
 * Copyable and cheap to copy; every copy names the same inbox, and stays valid
 * after the engine is gone (its posts are refused then).
 */
template <typename Input> class EngineInbox {
public:
    explicit EngineInbox(std::shared_ptr<EngineInboxState<Input>> state) noexcept : state_(std::move(state)) {}

    /// Post @p input; whether the inbox took it. See `EngineInboxState::post`.
    bool post(Input input) const {
        return state_->post(std::move(input));
    }

    /// Install the waker a post calls; see `EngineInboxState::wakeWith`.
    void wakeWith(std::function<void()> waker) const {
        state_->wakeWith(std::move(waker));
    }

private:
    std::shared_ptr<EngineInboxState<Input>> state_;
};

/**
 * @brief The engine's own hold on its inbox
 *
 * Makes the shared state with the engine, hands out handles, and closes the inbox
 * when the engine goes, so a handle kept past the engine is refused rather than
 * heard by nobody. Movable with its engine; not copyable, because a copy of an
 * engine sharing the original's inbox would be a second consumer of it.
 */
template <typename Input> class EngineInboxOwner {
public:
    EngineInboxOwner() : state_(std::make_shared<EngineInboxState<Input>>()) {}

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
    EngineInbox<Input> handle() const {
        return EngineInbox<Input>(state_);
    }

    /// The shared state, for the engine's own (owner-side) use.
    EngineInboxState<Input> &state() const noexcept {
        return *state_;
    }

private:
    std::shared_ptr<EngineInboxState<Input>> state_;
};

}  // namespace SCE::Core
