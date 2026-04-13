// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Uuid — RFC 9562 UUID v7 / v4 helpers.
//
// v7 (preferred): 48-bit Unix millisecond timestamp prefix + 74 random bits
// + version/variant. Time-ordered for log/correlation use; sufficient
// uniqueness for distributed peers (MeshEnvelope correlation_id, mesh peer
// identity). See SCE_MESH.md Section 13 Phase 3.5.
//
// v4 (fallback): 122 fully random bits + version/variant. Use when time
// source is unavailable or monotonic ordering is not required.
//
// Returns raw 16-byte arrays (big-endian wire form per RFC 9562 §4). String
// formatting is the caller's concern — kept here as raw bytes to avoid
// allocations on the mesh hot path.

#pragma once

#include <array>
#include <cstdint>

namespace SCE::uuid {

using Bytes = std::array<uint8_t, 16>;

/// Generate a UUID v7 (RFC 9562 §5.7). Big-endian wire bytes.
/// Thread-safe; uses a thread-local PRNG seeded from std::random_device.
[[nodiscard]] Bytes v7();

/// Generate a UUID v4 (RFC 9562 §5.4). Big-endian wire bytes.
/// Thread-safe; uses the same thread-local PRNG as v7().
[[nodiscard]] Bytes v4();

}  // namespace SCE::uuid
