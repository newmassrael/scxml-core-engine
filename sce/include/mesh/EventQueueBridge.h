// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh EventQueueBridge — bounded MPSC (Multi-Producer Single-Consumer)
// lock-free queue for cross-thread event injection.
//
// Implementation: Dmitry Vyukov bounded MPSC queue with per-cell sequence
// numbers. Lock-free for both producers (CAS on write position) and
// the single consumer (non-atomic read position).
//
// Phase 1: complete implementation. Phase 2 (shm_transport) is the first
// consumer — the receive-side codegen injects events through this bridge.

#pragma once

#include <array>
#include <atomic>
#include <cstddef>
#include <new>
#include <type_traits>
#include <utility>

namespace SCE::Mesh {

/// Bounded MPSC lock-free queue for cross-thread event delivery.
///
/// @tparam T        Event type (must be move-constructible and destructible)
/// @tparam Capacity Queue depth, must be a power of 2 (default: 256)
///
/// Thread safety:
///   try_push()  — safe to call from multiple producer threads concurrently
///   try_pop()   — must be called from exactly one consumer thread
///   empty()     — safe to call from the consumer thread only
template <typename T, std::size_t Capacity = 256>
class EventQueueBridge {
    static_assert(Capacity > 0 && (Capacity & (Capacity - 1)) == 0,
                  "Capacity must be a power of 2");
    static_assert(std::is_move_constructible<T>::value,
                  "Event type must be move-constructible");
    static_assert(std::is_destructible<T>::value,
                  "Event type must be destructible");

    static constexpr std::size_t MASK = Capacity - 1;

    // ── Per-cell layout ─────────────────────────────────────────
    // Each cell has a sequence number that coordinates producer/consumer
    // handoff without requiring a global lock.
    //
    // Sequence lifecycle:
    //   Initial:    seq == cell_index  (cell is empty, available for write)
    //   After push: seq == cell_index + 1  (cell is full, available for read)
    //   After pop:  seq == cell_index + Capacity  (cell is empty, next round)
    struct Cell {
        std::atomic<std::size_t> sequence;
        alignas(T) unsigned char storage[sizeof(T)];
    };

    // ── Cache-line aligned positions ────────────────────────────
    // Separate cache lines prevent false sharing between producer
    // and consumer hot paths.
    alignas(64) std::array<Cell, Capacity> buffer_;
    alignas(64) std::atomic<std::size_t> write_pos_{0};
    alignas(64) std::size_t read_pos_{0};  // consumer-only, no atomic needed

public:
    EventQueueBridge() noexcept {
        for (std::size_t i = 0; i < Capacity; ++i) {
            buffer_[i].sequence.store(i, std::memory_order_relaxed);
        }
    }

    ~EventQueueBridge() {
        // In-place destroy remaining elements without requiring default constructor
        for (;;) {
            Cell& cell = buffer_[read_pos_ & MASK];
            std::size_t seq = cell.sequence.load(std::memory_order_acquire);
            if (static_cast<std::ptrdiff_t>(seq - (read_pos_ + 1)) < 0) {
                break;
            }
            reinterpret_cast<T*>(cell.storage)->~T();
            cell.sequence.store(read_pos_ + Capacity, std::memory_order_relaxed);
            ++read_pos_;
        }
    }

    EventQueueBridge(const EventQueueBridge&) = delete;
    EventQueueBridge& operator=(const EventQueueBridge&) = delete;

    /// Push an event into the queue (producer side, thread-safe).
    ///
    /// @return true if the event was enqueued, false if the queue is full.
    ///         Callers should back off or drop the event on false.
    [[nodiscard]] bool try_push(T event) noexcept {
        Cell* cell;
        std::size_t pos = write_pos_.load(std::memory_order_relaxed);

        for (;;) {
            cell = &buffer_[pos & MASK];
            std::size_t seq = cell->sequence.load(std::memory_order_acquire);
            auto diff = static_cast<std::ptrdiff_t>(seq) -
                        static_cast<std::ptrdiff_t>(pos);

            if (diff == 0) {
                // Cell is available for writing — try to claim it
                if (write_pos_.compare_exchange_weak(
                        pos, pos + 1, std::memory_order_relaxed)) {
                    break;
                }
                // CAS failed — another producer claimed this cell, retry
            } else if (diff < 0) {
                // Cell is still occupied by an unread value — queue is full
                return false;
            } else {
                // Another producer advanced write_pos past us — reload
                pos = write_pos_.load(std::memory_order_relaxed);
            }
        }

        // We own the cell — move-construct the event in-place
        ::new (cell->storage) T(std::move(event));
        // Publish: advance sequence to signal consumer
        cell->sequence.store(pos + 1, std::memory_order_release);
        return true;
    }

    /// Pop an event from the queue (consumer side, single-thread only).
    ///
    /// @param[out] out Receives the popped event on success.
    /// @return true if an event was dequeued, false if the queue is empty.
    [[nodiscard]] bool try_pop(T& out) noexcept {
        Cell* cell = &buffer_[read_pos_ & MASK];
        std::size_t seq = cell->sequence.load(std::memory_order_acquire);
        auto diff = static_cast<std::ptrdiff_t>(seq) -
                    static_cast<std::ptrdiff_t>(read_pos_ + 1);

        if (diff < 0) {
            // Cell has not been written yet — queue is empty
            return false;
        }

        // Cell is ready — move the event out and destroy the in-place copy
        T* ptr = reinterpret_cast<T*>(cell->storage);
        out = std::move(*ptr);
        ptr->~T();

        // Release cell for the next producer round
        cell->sequence.store(read_pos_ + Capacity, std::memory_order_release);
        ++read_pos_;
        return true;
    }

    /// Check if the queue appears empty (consumer-thread only).
    ///
    /// This is an approximation: a concurrent push may complete between
    /// the check and the caller's next action. Use try_pop() for
    /// authoritative emptiness detection.
    [[nodiscard]] bool empty() const noexcept {
        const Cell& cell = buffer_[read_pos_ & MASK];
        std::size_t seq = cell.sequence.load(std::memory_order_acquire);
        auto diff = static_cast<std::ptrdiff_t>(seq) -
                    static_cast<std::ptrdiff_t>(read_pos_ + 1);
        return diff < 0;
    }
};

}  // namespace SCE::Mesh
