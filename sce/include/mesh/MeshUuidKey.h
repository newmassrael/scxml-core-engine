// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh — shared 16-byte UUID v7 key type and FNV-1a hash.
//
// `MeshEnvelope.invoke_id` (§9.5 mesh-rpc) and `MeshEnvelope.id` (§10.10
// envelope id used by the retrying-dispatcher backoff layer) are both
// 16-byte UUID v7 values from disjoint generators. Multiple SCE mesh
// runtime components key per-envelope state by these values:
//   • InvokeCorrelation (§9.5 mesh-rpc correlation table)
//   • MeshDeadlineScheduler (deadline timer registry shared by mesh-rpc,
//     §13 server query timeout, and retry backoff)
//   • RetryingDispatcher (per-envelope retry attempt counter)
//
// Before this header existed each consumer redeclared its own
// `using Key = std::array<uint8_t, 16>` and a private FNV-1a hash
// struct. The redeclarations were typed-equivalent (`std::array<uint8_t,
// 16>` is a structural type) but the duplication itself is an ownership
// inversion (axis 4 — schema completeness): the single concept "16-byte
// UUID key for SCE mesh runtime" lives in two-plus places, and a future
// strong-typing change (e.g. wrapping the array in a tagged type to
// prevent accidental cross-context use) would have to touch every
// redeclaration in lockstep. Centralising here makes the concept a
// single source of truth — each consumer's `using` alias reads from
// this header so the strong-typing change becomes a one-file edit.
//
// `MeshUuidKeyHash` is FNV-1a over 16 bytes. Cheaper than SipHash and
// adequate for the consumer workloads: the maps hold actively in-flight
// entries bounded by the small number of concurrent mesh-rpc invokes /
// pending retries a single peer carries — never adversarially large.
// The key is a random UUID v7 so distribution quality of FNV-1a is not
// load-bearing against a hostile workload.

#pragma once

#include <array>
#include <cstddef>
#include <cstdint>

namespace SCE::Mesh {

using MeshUuidKey = std::array<std::uint8_t, 16>;

struct MeshUuidKeyHash {
    std::size_t operator()(const MeshUuidKey& k) const noexcept {
        std::size_t h = 14695981039346656037ULL;
        for (std::uint8_t b : k) {
            h ^= b;
            h *= 1099511628211ULL;
        }
        return h;
    }
};

}  // namespace SCE::Mesh
