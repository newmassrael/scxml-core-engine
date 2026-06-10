// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh MeshEnvelopeCodec — CBOR encode/decode for MeshEnvelope.
//
// Wire form: canonical CBOR (RFC 8949 §4.2.1) integer-keyed map. Required
// keys are always emitted; absent optional keys are omitted from the map.
// Decoders MUST skip unknown integer keys (forward compatibility).
//
// Schema and key map are pinned in SCE_MESH.md §mesh-13 "Pattern Realization"; see
// the kEnvelopeKey* constants below for the wire integers.

#pragma once

#include "mesh/MeshEnvelope.h"

#include <cstddef>
#include <cstdint>
#include <vector>

namespace SCE::Mesh {

/// Wire-stable CBOR map keys. Once shipped, never repurpose; new fields take
/// new integers.
inline constexpr int kEnvelopeKeyId              = 0;
inline constexpr int kEnvelopeKeySource          = 1;
inline constexpr int kEnvelopeKeyType            = 2;
inline constexpr int kEnvelopeKeyPattern         = 3;
inline constexpr int kEnvelopeKeyDatacontenttype = 4;
inline constexpr int kEnvelopeKeyData            = 5;
inline constexpr int kEnvelopeKeySubject         = 6;
inline constexpr int kEnvelopeKeyCorrelationId   = 7;
inline constexpr int kEnvelopeKeyReplyTo         = 8;
inline constexpr int kEnvelopeKeyInvokeId        = 9;
inline constexpr int kEnvelopeKeyRpcStatus       = 10;
inline constexpr int kEnvelopeKeyRpcErrorMessage = 11;
inline constexpr int kEnvelopeKeyDeadlineUnixMs  = 12;
inline constexpr int kEnvelopeKeyQos             = 13;
inline constexpr int kEnvelopeKeySequenceNo      = 14;
inline constexpr int kEnvelopeKeyRoutingId       = 15;
inline constexpr int kEnvelopeKeyParallelId      = 16;  // SCE_MESH.md §mesh-16.5 wire-21 region routing
inline constexpr int kEnvelopeKeyRegionId        = 17;  // SCE_MESH.md §mesh-16.5 wire-21 region routing
inline constexpr int kEnvelopeKeyChildSessionId  = 18;  // SCE_MESH.md §mesh-9.6.2 wire-15 child session URI

/// Single source for the SCE mesh envelope payload size ceiling.
///
/// 16 MiB matches the practical SOME/IP message ceiling and is the
/// hard reject threshold during CBOR decode (`MeshEnvelopeCodec.cpp`
/// data-length validation). Every downstream transport with its own
/// pre-decode size check (custom_tcp's frame reader, future TCP-like
/// wires) must reference this constant directly rather than redeclare
/// a numerically-equivalent literal — the constant is the wire spec,
/// duplicated literals were ownership-inversion via co-declaration.
inline constexpr std::size_t kMaxEnvelopeBytes = 16u * 1024u * 1024u;

/// Encode envelope into canonical CBOR bytes.
///
/// Two-pass: first attempts in-place encoding into a 256-byte stack buffer,
/// resizing on overflow. Always succeeds for any well-formed envelope; the
/// only failure mode (return empty vector) is a buffer-size pathology that
/// should never occur in practice.
[[nodiscard]] std::vector<uint8_t> encodeEnvelope(const MeshEnvelope &env);

/// Decode CBOR bytes into envelope.
///
/// @param raw  CBOR map bytes (typically the payload from a transport drain).
/// @param len  Length of @p raw.
/// @param out  Populated on success.
/// @return true on success, false on malformed CBOR or missing required key.
///         Unknown integer keys are silently skipped (forward compatibility).
[[nodiscard]] bool decodeEnvelope(const uint8_t *raw, std::size_t len, MeshEnvelope &out);

}  // namespace SCE::Mesh
