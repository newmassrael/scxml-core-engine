// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh CausalEntry — one entry of a causal route's version vector.
//
// The runtime's spelling of the record sce:std/mesh/causal_entry.scxml
// declares (docs/adr/0085, sections 3 and 3.3): a replica of the group, as the
// two big-endian halves of its 16-byte UUID v7, and a number among that
// replica's changes. A list of these is a vector: a copy's delivered vector,
// the context a change is tagged with, the vector a `causalVector` carries and
// the stable cut. An entry also names one change, its dot.
//
// The envelope carries a vector under key 20 (`causal_vector`), the sender's
// own entry first (SCE_MESH.md §mesh-10.12). MeshEnvelope holds this type, and
// MeshEnvelopeCodec.cpp and CausalRules.cpp convert it to and from the record
// the standard documents generate, as MeshEnvelope stands for the generated
// envelope: no public header names a generated type.

#pragma once

#include <cstdint>

namespace SCE::Mesh {

struct CausalEntry {
    /// The replica's first eight bytes, big-endian.
    std::uint64_t hi = 0;
    /// The replica's last eight bytes, big-endian.
    std::uint64_t lo = 0;
    /// The number among that replica's changes; 0 when it delivered none.
    std::uint64_t seq = 0;

    bool operator==(const CausalEntry &other) const noexcept {
        return hi == other.hi && lo == other.lo && seq == other.seq;
    }

    bool operator!=(const CausalEntry &other) const noexcept {
        return !(*this == other);
    }
};

}  // namespace SCE::Mesh
