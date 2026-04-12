// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh ShmChannel — shared memory event channel.
//
// Combines ShmSegment (POSIX shm) with EventQueueBridge (lock-free MPSC)
// to provide cross-process event delivery. Events are serialized as
// fixed-size name strings (ShmEvent) for cross-SM enum compatibility.
//
// Sender side:  ShmChannel(name, Mode::Create) → send(event_name)
// Receiver side: ShmChannel(name, Mode::Open) → drain<Policy>(engine)

#pragma once

#include "mesh/EventQueueBridge.h"
#include "mesh/ShmSegment.h"

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <new>

namespace SCE::Mesh {

/// Fixed-size wire event for cross-process shared memory transport.
/// Uses event name strings instead of enum values because sender and
/// receiver state machines have independent Event enum types.
struct ShmEvent {
    static constexpr std::size_t MAX_NAME_LEN = 63;
    char name[MAX_NAME_LEN + 1];

    ShmEvent() noexcept { name[0] = '\0'; }

    explicit ShmEvent(const char* event_name) noexcept {
        std::strncpy(name, event_name, MAX_NAME_LEN);
        name[MAX_NAME_LEN] = '\0';
    }
};

static_assert(sizeof(ShmEvent) == 64, "ShmEvent must be exactly 64 bytes");

/// Default channel capacity (events). Power of 2 required by EventQueueBridge.
inline constexpr std::size_t SHM_DEFAULT_CAPACITY = 256;

/// Shared memory event channel — EventQueueBridge in a POSIX shm segment.
///
/// Thread/process safety:
///   send()     — safe from multiple producer threads/processes
///   try_pop()  — single consumer thread/process only
///   drain()    — single consumer thread/process only
///
/// Startup ordering:
///   POSIX shm_open + ftruncate zero-fills the page, but EventQueueBridge
///   requires cell sequences initialized to their cell index. A receiver
///   attaching before the sender's placement-new would therefore observe
///   a silently broken queue. We guard against this with a ready-flag
///   handshake: the creator release-stores a magic value after the
///   bridge is fully constructed; the opener acquire-loads and treats
///   the channel as invalid until the magic is present. Applications
///   may thus start sender and receiver in any order — the opener
///   simply polls valid() / retries construction until the sender
///   has finished initializing the segment.
///
/// @tparam Capacity Channel depth (power of 2, default 256)
template <std::size_t Capacity = SHM_DEFAULT_CAPACITY>
class ShmChannel {
    using Bridge = EventQueueBridge<ShmEvent, Capacity>;

    /// Shared-memory layout: ready flag + bridge. The ready flag is
    /// cache-line aligned to keep it off the bridge's producer/consumer
    /// hot paths and release/acquire-ordered against bridge construction.
    /// Bridge already cache-line-aligns its own hot fields, so no extra
    /// alignas is applied here.
    struct Layout {
        alignas(64) std::atomic<std::uint64_t> ready{0};
        Bridge bridge;
    };

    /// "SCE_MSH1" — identifies a fully-constructed ShmChannel layout.
    /// Bump the trailing digit if the wire format ever changes.
    static constexpr std::uint64_t READY_MAGIC = 0x5343455F4D534831ULL;

    ShmSegment segment_;
    Bridge* bridge_ = nullptr;

public:
    enum class Mode { Create, Open };

    /// Construct a shared memory channel.
    ///
    /// @param name   POSIX shm name (e.g., "/sce_brake_motor")
    /// @param mode   Create (sender) or Open (receiver)
    ShmChannel(const char* name, Mode mode)
        : segment_(mode == Mode::Create
                       ? ShmSegment::create(name, sizeof(Layout))
                       : ShmSegment::open(name, sizeof(Layout))) {
        if (!segment_.valid()) return;

        auto* layout = reinterpret_cast<Layout*>(segment_.data());

        if (mode == Mode::Create) {
            // Placement-new the full layout: ready=0 and a valid Bridge.
            ::new (layout) Layout{};
            // Publish: any thread/process that acquire-loads READY_MAGIC
            // is guaranteed to see the fully-constructed bridge.
            layout->ready.store(READY_MAGIC, std::memory_order_release);
            bridge_ = &layout->bridge;
        } else {
            // Receiver: verify the creator has finished initializing.
            // If not, leave bridge_ == nullptr so valid() returns false —
            // the caller can retry construction later.
            if (layout->ready.load(std::memory_order_acquire) == READY_MAGIC) {
                bridge_ = &layout->bridge;
            }
        }
    }

    ~ShmChannel() {
        // Intentionally do NOT invoke bridge_->~Bridge() here, even on the
        // creator side: EventQueueBridge's destructor drains remaining
        // events from the ring buffer, which in a cross-process channel
        // would discard events the receiver has not yet read. The bridge
        // lives inside shm memory that peer processes may still be
        // mapping; destructing it would also leave those peers reading
        // a destroyed object.
        //
        // ShmEvent is trivially destructible, so leaving un-popped events
        // in place when the shm segment is finally reclaimed is a no-op.
        // ShmSegment's destructor handles munmap (every process) and
        // shm_unlink (creator only) — the kernel frees the page once
        // the last mapping is gone.
    }

    ShmChannel(ShmChannel&& o) noexcept
        : segment_(std::move(o.segment_)), bridge_(o.bridge_) {
        o.bridge_ = nullptr;
    }

    ShmChannel& operator=(ShmChannel&& o) noexcept {
        if (this != &o) {
            // See destructor comment: we intentionally do not destroy the
            // bridge — ShmSegment's move assignment releases our mapping
            // and (if owner) unlinks the shm name.
            segment_ = std::move(o.segment_);
            bridge_ = o.bridge_;
            o.bridge_ = nullptr;
        }
        return *this;
    }

    ShmChannel(const ShmChannel&) = delete;
    ShmChannel& operator=(const ShmChannel&) = delete;

    /// Send an event by name through the shared memory channel.
    /// @return true if enqueued, false if channel is full or invalid.
    [[nodiscard]] bool send(const char* event_name) noexcept {
        return bridge_ ? bridge_->try_push(ShmEvent(event_name)) : false;
    }

    /// Pop a single event from the channel (receiver side).
    [[nodiscard]] bool try_pop(ShmEvent& out) noexcept {
        return bridge_ ? bridge_->try_pop(out) : false;
    }

    /// Check if the channel is valid (shm mapped successfully).
    [[nodiscard]] bool valid() const noexcept { return bridge_ != nullptr; }

    /// Check if the channel appears empty (receiver side).
    [[nodiscard]] bool empty() const noexcept {
        return bridge_ ? bridge_->empty() : true;
    }

    /// Drain all pending events into a state machine engine.
    ///
    /// Converts event names to the receiver's Event enum via
    /// Policy::getEventFromName, then calls engine.processEvent.
    /// Unknown event names are silently dropped.
    ///
    /// @tparam Policy  Receiver's StatePolicy (generated struct)
    /// @tparam Engine  StaticExecutionEngine<Policy>
    /// @return Number of events drained
    template <typename Policy, typename Engine>
    std::size_t drain(Engine& engine) {
        ShmEvent wire_event;
        std::size_t count = 0;
        while (try_pop(wire_event)) {
            auto event = Policy::getEventFromName(wire_event.name);
            if (event) {
                engine.processEvent(*event);
            }
            ++count;
        }
        return count;
    }
};

}  // namespace SCE::Mesh
