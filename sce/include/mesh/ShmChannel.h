// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh ShmChannel — shared memory event channel.
//
// Layout (SCE_MESH.md Section 7.5):
//   [ready_magic] [head_vc] [tail_vc] [control ring] [payload arena]
//
//   ready_magic  — startup handshake (any-order sender/receiver start)
//   head_vc      — producer virtual counter (arena occupancy tracking)
//   tail_vc      — consumer virtual counter
//   control ring — fixed-size ControlSlot entries (see below)
//   payload arena — variable-size CBOR envelope bytes, circular
//
// Wire format (per event): canonical CBOR MeshEnvelope (MeshEnvelopeCodec.h).
// Both encode and decode are internal to ShmChannel — callers use the
// high-level send(MeshEnvelope) / drain() API without touching CBOR.
//
// Concurrency model: **SPSC overall** (single producer, single consumer).
//
// The underlying EventQueueBridge (Vyukov) is MPSC-capable by itself, but
// this channel is SPSC because send() reserves arena space via plain
// load/store on head_vc — not CAS. A second concurrent producer would
// race on arena writes even though the control ring push is atomic. The
// debug assertion in send() catches violations.
//
// SPSC is sufficient for Phase 3: each SCE machine is single-threaded
// per W3C RTC, and each ShmChannel is per-target (one sender SM →
// one channel). True MPSC requires either CAS-based arena reservation
// with ticket-ordered control ring entries, or a heap-allocated slab
// pool (iceoryx pattern) — future work when multiple machines on the
// same host share an outbound channel.

#pragma once

#include "mesh/EventQueueBridge.h"
#include "mesh/MeshDispatch.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/ShmSegment.h"

#include <atomic>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <new>
#include <string>
#include <string_view>
#include <thread>

namespace SCE::Mesh {

/// Control ring entry pointing into the payload arena.
///
///   offset   — physical byte offset in the arena where this entry's
///              wire bytes start.
///   length   — wire bytes at that offset (CBOR-encoded MeshEnvelope).
///   advance  — amount by which the consumer must bump tail_vc after
///              processing this entry. Equals `length` when the entry
///              was written contiguously; larger when the producer
///              skipped past an unusable arena tail remainder.
struct ControlSlot {
    uint32_t offset;
    uint32_t length;
    uint32_t advance;
};
static_assert(sizeof(ControlSlot) == 12, "ControlSlot must be 12 bytes");

/// Default arena size in bytes. Configurable via deploy.yaml
/// `shm_arena_bytes` (future work).
inline constexpr std::size_t SHM_DEFAULT_ARENA_BYTES = 65536;

/// Default control ring capacity (slots). Power of 2 required by
/// EventQueueBridge.
inline constexpr std::size_t SHM_DEFAULT_RING_CAPACITY = 256;

/// Shared memory event channel.
///
/// Supports variable-length payloads via a control ring + payload arena
/// layout — a textbook pattern for lock-free shm IPC (used by iceoryx,
/// DDS implementations).
///
/// @tparam ArenaSize    payload arena size in bytes (any value)
/// @tparam RingCapacity control ring slot count (power of 2)
template <std::size_t ArenaSize = SHM_DEFAULT_ARENA_BYTES,
          std::size_t RingCapacity = SHM_DEFAULT_RING_CAPACITY>
class ShmChannel {
    using ControlBridge = EventQueueBridge<ControlSlot, RingCapacity>;

    /// Shared-memory layout. Each hot field is cache-line aligned to
    /// avoid false sharing between producer and consumer.
    struct Layout {
        alignas(64) std::atomic<std::uint64_t> ready{0};
        alignas(64) std::atomic<std::uint64_t> head_vc{0};  // producer counter
        alignas(64) std::atomic<std::uint64_t> tail_vc{0};  // consumer counter
        ControlBridge control;
        alignas(64) char arena[ArenaSize];
    };

    /// "SCE_MSH3" — identifies the control-ring + payload-arena layout
    /// introduced in SCE_MESH.md Section 7.5 (2026-04-13).
    static constexpr std::uint64_t READY_MAGIC = 0x5343455F4D534833ULL;

    ShmSegment segment_;
    Layout* layout_ = nullptr;

#ifndef NDEBUG
    /// SPSC enforcement: the first send() call captures the producer
    /// thread's id; subsequent sends assert they originate from the
    /// same thread. Debug-only (zero cost in release builds).
    mutable std::atomic<std::thread::id> producer_tid_{std::thread::id{}};
#endif

public:
    enum class Mode { Create, Open };

    /// Construct a shared memory channel.
    ///
    /// @param name POSIX shm name (e.g., "/sce_brake_motor")
    /// @param mode Create (sender) or Open (receiver)
    ShmChannel(const char* name, Mode mode)
        : segment_(mode == Mode::Create
                       ? ShmSegment::create(name, sizeof(Layout))
                       : ShmSegment::open(name, sizeof(Layout))) {
        if (!segment_.valid()) return;
        auto* layout = reinterpret_cast<Layout*>(segment_.data());

        if (mode == Mode::Create) {
            ::new (layout) Layout{};
            layout->ready.store(READY_MAGIC, std::memory_order_release);
            layout_ = layout;
        } else {
            if (layout->ready.load(std::memory_order_acquire) == READY_MAGIC) {
                layout_ = layout;
            }
        }
    }

    ~ShmChannel() {
        // See prior comment: do not destroy the in-shm Layout. Peer
        // processes may still be mapping it. ShmSegment handles munmap
        // and (on the creator side) shm_unlink; the kernel reclaims the
        // page when the last mapping is gone.
    }

    ShmChannel(ShmChannel&& o) noexcept
        : segment_(std::move(o.segment_)), layout_(o.layout_) {
        o.layout_ = nullptr;
    }

    ShmChannel& operator=(ShmChannel&& o) noexcept {
        if (this != &o) {
            segment_ = std::move(o.segment_);
            layout_ = o.layout_;
            o.layout_ = nullptr;
        }
        return *this;
    }

    ShmChannel(const ShmChannel&) = delete;
    ShmChannel& operator=(const ShmChannel&) = delete;

    /// Send a MeshEnvelope through the shared memory channel.
    ///
    /// Internally encodes to CBOR before writing to the arena — symmetric
    /// with drain() which decodes CBOR internally. Callers never touch
    /// raw bytes or the CBOR codec directly.
    ///
    /// Returns false on:
    ///   - channel not valid (segment mapping failed or sender not ready)
    ///   - CBOR encoding failure (should not occur for well-formed envelopes)
    ///   - message size exceeds arena capacity
    ///   - arena full (consumer lagging — back-pressure, not truncation)
    ///   - control ring full
    ///
    /// Single-producer: one thread may call send() per channel instance.
    [[nodiscard]] bool send(const MeshEnvelope& env) {
        auto buf = encodeEnvelope(env);
        if (buf.empty()) return false;
        return sendRaw(buf.data(), buf.size());
    }

    /// Check if the channel is valid (shm mapped, sender ready).
    [[nodiscard]] bool valid() const noexcept { return layout_ != nullptr; }

    /// Check if the control ring is empty (receiver side).
    [[nodiscard]] bool empty() const noexcept {
        return layout_ ? layout_->control.empty() : true;
    }

    /// Drain all pending events from the channel into a state machine engine.
    ///
    /// Internally decodes each arena entry from CBOR and dispatches via
    /// dispatchEnvelope() — symmetric with send() which encodes internally.
    /// Callers never touch raw bytes or the CBOR codec directly.
    ///
    /// Does NOT call `engine.step()` — macrostep timing is scheduler/application
    /// responsibility (W3C SCXML 3.12, SCE_MESH.md Section 7.5).
    ///
    /// Unknown event names (not in the receiver's enum) are silently dropped
    /// but still advance the arena tail to reclaim space.
    ///
    /// @tparam Policy Receiver's StatePolicy (generated struct)
    /// @tparam Engine StaticExecutionEngine<Policy>
    /// @return Number of control entries drained
    template <typename Policy, typename Engine>
    std::size_t drain(Engine& engine) {
        if (!layout_) return 0;

        std::size_t count = 0;
        ControlSlot slot;
        while (layout_->control.try_pop(slot)) {
            MeshEnvelope env;
            if (!decodeEnvelope(
                    reinterpret_cast<const std::uint8_t*>(layout_->arena + slot.offset),
                    slot.length, env)) {
                // Malformed CBOR — skip but reclaim arena space.
                layout_->tail_vc.fetch_add(slot.advance, std::memory_order_release);
                continue;
            }

            dispatchEnvelope<Policy>(env, engine);

            // Advance tail unconditionally — arena slot is reclaimed
            // whether or not the event name resolved.
            layout_->tail_vc.fetch_add(slot.advance, std::memory_order_release);
            ++count;
        }
        return count;
    }

private:
    /// Write pre-encoded bytes into the arena. Implementation detail of
    /// send(const MeshEnvelope&) — not part of the public API.
    [[nodiscard]] bool sendRaw(const uint8_t* raw, std::size_t len) noexcept {
        if (!layout_ || len == 0) return false;
        if (len > ArenaSize) return false;

#ifndef NDEBUG
        std::thread::id empty{};
        std::thread::id self = std::this_thread::get_id();
        if (!producer_tid_.compare_exchange_strong(
                empty, self, std::memory_order_relaxed)) {
            assert(empty == self &&
                   "ShmChannel::send() called from multiple threads. "
                   "SCE Mesh shm transport is SPSC (one producer per channel). "
                   "See SCE_MESH.md Section 7.5.");
        }
#endif

        std::uint64_t h = layout_->head_vc.load(std::memory_order_relaxed);
        std::uint64_t t = layout_->tail_vc.load(std::memory_order_acquire);

        std::uint32_t phys = static_cast<std::uint32_t>(h % ArenaSize);
        std::uint32_t remaining = static_cast<std::uint32_t>(ArenaSize - phys);
        std::uint32_t skip = 0;
        if (len > remaining) {
            skip = remaining;
            phys = 0;
        }
        std::uint32_t advance = skip + static_cast<std::uint32_t>(len);

        if ((h + advance) - t > ArenaSize) return false;

        std::memcpy(layout_->arena + phys, raw, len);

        if (!layout_->control.try_push(
                {phys, static_cast<std::uint32_t>(len), advance})) {
            return false;
        }

        layout_->head_vc.store(h + advance, std::memory_order_relaxed);
        return true;
    }
};

}  // namespace SCE::Mesh
