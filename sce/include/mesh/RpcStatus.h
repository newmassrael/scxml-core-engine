// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh RpcStatus — wire-stable RPC reply status code.
//
// Modeled on grpc::StatusCode for cross-ecosystem familiarity. Ok (0) means
// success; all other values are errors. Values are IMMUTABLE once shipped;
// populated on RpcReply envelopes only — omitted on RpcRequest and on
// successful replies (absent == Ok). See SCE_MESH.md Section 13 Phase 3.5.
// Serialized into MeshEnvelope key 10 as CBOR uint8 when present.

#pragma once

#include <cstdint>

namespace SCE::Mesh {

enum class RpcStatus : uint8_t {
    Ok                = 0,
    Cancelled         = 1,
    InvalidArgument   = 3,
    DeadlineExceeded  = 4,
    NotFound          = 5,
    Unimplemented     = 12,
    Internal          = 13,
    Unavailable       = 14,
};

static_assert(static_cast<uint8_t>(RpcStatus::Ok)               == 0,  "RpcStatus wire value changed");
static_assert(static_cast<uint8_t>(RpcStatus::Cancelled)        == 1,  "RpcStatus wire value changed");
static_assert(static_cast<uint8_t>(RpcStatus::InvalidArgument)  == 3,  "RpcStatus wire value changed");
static_assert(static_cast<uint8_t>(RpcStatus::DeadlineExceeded) == 4,  "RpcStatus wire value changed");
static_assert(static_cast<uint8_t>(RpcStatus::NotFound)         == 5,  "RpcStatus wire value changed");
static_assert(static_cast<uint8_t>(RpcStatus::Unimplemented)    == 12, "RpcStatus wire value changed");
static_assert(static_cast<uint8_t>(RpcStatus::Internal)         == 13, "RpcStatus wire value changed");
static_assert(static_cast<uint8_t>(RpcStatus::Unavailable)      == 14, "RpcStatus wire value changed");

}  // namespace SCE::Mesh
