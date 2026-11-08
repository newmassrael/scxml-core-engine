// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh CausalRules — the causal route's rule, as the C++ core calls it.
//
// The rule of a causal route is written once, as standard Mesh algorithms
// (docs/adr/0085, sections 3 and 3.2; SCE_MESH.md §mesh-10.12), and
// generated for every language: whether a change is dropped, delivered or
// held (sce:std/mesh/causal_admit), the merge of two vectors (causal_raise),
// whether a vector lacks a change (causal_lacks) and the stable cut
// (causal_cut). The C++ tree is generated into sce/src/mesh/generated/ by
// sce/src/mesh/generate_envelope.sh, beside the envelope codec.
//
// These functions are the one door to it. Each converts the runtime's
// CausalEntry to the generated record, calls the generated function and
// converts its answer back; nothing here decides anything the documents do
// not. They are compiled in CausalRules.cpp so that no public header names a
// generated type, as MeshEnvelopeCodec.cpp stands between MeshEnvelope and
// the generated envelope. CausalLayer.h applies them to a copy's state.

#pragma once

#include "mesh/CausalEntry.h"

#include <sce/forge/algorithm.h>
#include <sce/forge/span.h>

#include <cstddef>
#include <cstdint>
#include <vector>

namespace SCE::Mesh {

/// The most entries a vector holds: the bound of a view, of the envelope's
/// `causal_vector` (key 20) and of the rules' lists.
inline constexpr std::size_t kCausalVectorBound = 64;

/// What causal_admit decides for a change that arrives (the values of
/// sce:std/mesh/causal_admission.scxml).
enum class CausalAdmission : std::uint8_t {
    /// Delivered already.
    Drop = 0,
    /// The next change of its author, every change of its context delivered.
    Deliver = 1,
    /// Anything else, until it is deliverable.
    Hold = 2,
};

/// One entry of the latest delivered vector a copy knows of a member of its
/// view (sce:std/mesh/causal_known.scxml): the member's replica
/// (`holderHi`, `holderLo`) and the entry `seq` that member's vector gives the
/// author `hi`, `lo`. A vector per member is spelled as its entries, each
/// with the member it belongs to.
struct CausalKnown {
    std::uint64_t holderHi = 0;
    std::uint64_t holderLo = 0;
    std::uint64_t hi = 0;
    std::uint64_t lo = 0;
    std::uint64_t seq = 0;
};

/// causal_admit: for a copy whose delivered vector is `delivered`, a change
/// tagged with its dot `dot` and its context `context`. Fails `overflow` when
/// the author's delivered entry has no successor.
[[nodiscard]] ::SCE::Forge::AlgorithmResult<CausalAdmission>
causalAdmit(::SCE::Forge::Span<const CausalEntry> delivered, const CausalEntry &dot,
            ::SCE::Forge::Span<const CausalEntry> context);

/// causal_raise: `vector` raised by `other`, each author at the larger of its
/// two numbers, an author only `other` names appended. Fails
/// `capacity-exceeded` past kCausalVectorBound entries.
[[nodiscard]] ::SCE::Forge::AlgorithmResult<std::vector<CausalEntry>>
causalRaise(::SCE::Forge::Span<const CausalEntry> vector, ::SCE::Forge::Span<const CausalEntry> other);

/// causal_lacks: whether `vector` lacks the change `dot`.
[[nodiscard]] bool causalLacks(::SCE::Forge::Span<const CausalEntry> vector, const CausalEntry &dot);

/// causal_cut: the pointwise minimum, over the `members` replicas of the
/// view, of the vectors `known` spells, a member not in `known` at 0. Fails
/// `capacity-exceeded` past kCausalVectorBound entries.
[[nodiscard]] ::SCE::Forge::AlgorithmResult<std::vector<CausalEntry>>
causalCut(std::uint32_t members, ::SCE::Forge::Span<const CausalKnown> known);

}  // namespace SCE::Mesh
