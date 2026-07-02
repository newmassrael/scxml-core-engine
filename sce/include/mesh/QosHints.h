// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh QosHints — delivery quality hints carried on MeshEnvelope.
//
// Hints, not guarantees: each transport may honor a subset (e.g. SOME/IP
// always-reliable for unicast, Zenoh reliable/best_effort selectable, shm
// inherently reliable+ordered). See SCE_MESH.md §mesh-13 "Pattern Realization"
// envelope key 13. Default = best-effort + unordered (lowest cost). The
// field is in-memory only — the optional CBOR key stays omitted until a
// transport starts consuming it.

#pragma once

#include <cstdint>

namespace SCE::Mesh {

struct QosHints {
    enum class Reliability : uint8_t {
        BestEffort = 0,
        Reliable = 1,
    };

    enum class Ordering : uint8_t {
        Unordered = 0,
        Ordered = 1,
    };

    Reliability reliability = Reliability::BestEffort;
    Ordering ordering = Ordering::Unordered;

    bool operator==(const QosHints &other) const noexcept {
        return reliability == other.reliability && ordering == other.ordering;
    }
};

static_assert(static_cast<uint8_t>(QosHints::Reliability::BestEffort) == 0, "QosHints wire value changed");
static_assert(static_cast<uint8_t>(QosHints::Reliability::Reliable) == 1, "QosHints wire value changed");
static_assert(static_cast<uint8_t>(QosHints::Ordering::Unordered) == 0, "QosHints wire value changed");
static_assert(static_cast<uint8_t>(QosHints::Ordering::Ordered) == 1, "QosHints wire value changed");

}  // namespace SCE::Mesh
